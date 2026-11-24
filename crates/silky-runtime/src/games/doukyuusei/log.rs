//! The backlog of 同級生 Remake.
//!
//! Read from the executable and compared under Wine. `log_bg.akb` holds
//! the title, the three entry panels and the hint bar; the buttons are
//! parts of `log_p.akb` (rows normal, hover, pressed, gray) at the places
//! `backlog00.area` gives. An entry is 290 high: a star and a rule with
//! the speaker's name, the text in the message window's bitmap font below
//! it, a J button to go back to it and, when it has a voice, a play
//! button and a check box for "play the marked voices". A log of fewer
//! than three entries sits in the bottom rows. The scroll bar's thumb is
//! built from end caps and a middle of up to 210 pixels; it loses two
//! pixels per entry past the first three, moves two pixels per entry and
//! is left out while everything fits.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;

use super::gdi;
use super::system::System;
use super::window;
use crate::assets::Assets;
use crate::render::Canvas;
use crate::screens::{log_ids, Backlog, LogEntry, LOG_ROWS};
use crate::Result;

/// Pitch of the entries.
const PITCH: i32 = 290;
/// The star and rule (`log_p.akb`) and where they go in the first entry.
const RULE: (u32, u32, u32, u32) = (0, 816, 1219, 54);
const RULE_POS: (i32, i32) = (97, 106);
/// The speaker's name and the text in the first entry.
const NAME_POS: (i32, i32) = (146, 112);
const TEXT_POS: (i32, i32) = (97, 162);
/// Lines of text an entry has room for.
const TEXT_LINES: i32 = 3;

/// Button columns of `log_p.akb`: (x, width, height); rows go down by the
/// height.
const PAGE_UP: (u32, u32, u32) = (0, 167, 72);
const PAGE_DOWN: (u32, u32, u32) = (167, 167, 72);
const PLAY_MARKED: (u32, u32, u32) = (334, 202, 204);
const JUMP: (u32, u32, u32) = (536, 90, 90);
const BACK: (u32, u32, u32) = (626, 138, 140);
const VOICE: (u32, u32, u32) = (764, 90, 90);
/// Scroll arrows: up in rows 0..4, down in rows 4..8.
const ARROW: (u32, u32, u32) = (928, 28, 40);
/// Check boxes: rows normal, hover, pressed, ticked.
const CHECK: (u32, u32, u32) = (1028, 48, 48);
const TICKED: u32 = 3;

/// The scroll bar: the track (x, y, width, height) and the thumb's
/// columns of `log_p.akb` (normal, hover, pressed).
pub const TRACK: (i32, i32, i32, i32) = (1662, 85, 24, 863);
const THUMB_COLUMNS: [u32; 3] = [956, 980, 1004];
const THUMB_CAP: u32 = 20;
const THUMB_MIDDLE: u32 = 210;
const THUMB_MIN: i32 = 2 * THUMB_CAP as i32;

/// The hint bar's first letter cell; a hint of two lines starts higher
/// by [`HINT_RAISE`] and puts its lines [`HINT_LINE`] apart (measured
/// under Wine).
const HINT_TEXT: (i32, i32) = (977, 996);
const HINT_RAISE: i32 = 15;
const HINT_LINE: i32 = 25;
const HINT_SIZE: u32 = 24;
/// Pen move of the hint (wide, narrow): 26 and 13 at size 24.
const HINT_ADVANCE: (i32, i32) = (26, 13);

/// The thumb: top and length.
pub fn thumb(top: usize, len: usize) -> (i32, i32) {
    let past = len.saturating_sub(LOG_ROWS) as i32;
    let length = (TRACK.3 - 2 * past).max(THUMB_MIN);
    let y = TRACK.1 + 2 * top as i32;
    (y.min(TRACK.1 + TRACK.3 - length), length)
}

fn draw_cell(canvas: &mut Canvas, sheet: &Arc<Image>, cell: (u32, u32, u32), row: u32, a: &Area) {
    let (x, w, h) = cell;
    canvas.draw_part(sheet, (x, row * h, w, h), a.left, a.top, false, 255);
}

impl System {
    /// The hint for a backlog element (`SYSTEM_<lang>.MAP` 214..224).
    fn log_hint(&self, id: i32) -> Option<String> {
        use log_ids::*;
        let n = match id {
            UP => 214,
            DOWN => 215,
            SCROLL => 216,
            _ if (log_ids::CHECK..log_ids::CHECK + LOG_ROWS as i32).contains(&id) => 217,
            _ if (log_ids::JUMP..log_ids::JUMP + LOG_ROWS as i32).contains(&id) => 218,
            log_ids::PLAY_MARKED => 219,
            log_ids::PAGE_UP => 220,
            log_ids::PAGE_DOWN => 221,
            _ if (log_ids::VOICE..log_ids::VOICE + LOG_ROWS as i32).contains(&id) => 222,
            log_ids::BACK => 223,
            _ => return None,
        };
        Some(self.system.as_ref()?.get(0, n)?.to_string())
    }

    /// Draws the backlog. `state` gives the row (0 normal, 1 hover, 2
    /// pressed, 3 gray) of an element; `hover` is the element under the
    /// pointer.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_backlog(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        areas: &[Area],
        b: &Backlog,
        log: &[LogEntry],
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
    ) -> Result<()> {
        canvas.draw(&assets.image("log_bg.akb")?, 0, 0, false);
        let sheet = assets.image("log_p.akb")?;
        let area = |id: i32| areas.iter().find(|a| a.id == id);
        for (id, cell, row) in [
            (log_ids::UP, ARROW, 0),
            (log_ids::DOWN, ARROW, 4),
            (log_ids::PAGE_UP, PAGE_UP, 0),
            (log_ids::PAGE_DOWN, PAGE_DOWN, 0),
            (log_ids::PLAY_MARKED, PLAY_MARKED, 0),
            (log_ids::BACK, BACK, 0),
        ] {
            if let Some(a) = area(id) {
                draw_cell(canvas, &sheet, cell, row + state(id), a);
            }
        }
        let font = self.font(assets);
        for i in 0..LOG_ROWS as i32 {
            let Some(e) = b.entry(i as usize).and_then(|n| log.get(n)) else {
                continue;
            };
            let dy = PITCH * i;
            canvas.draw_part(&sheet, RULE, RULE_POS.0, RULE_POS.1 + dy, false, 255);
            if let Some(font) = font {
                if let Some(name) = &e.speaker_name {
                    font.draw_name(canvas, name, NAME_POS.0, NAME_POS.1 + dy, e.speaker_hero);
                }
                for (_, p) in window::layout(&e.text) {
                    if p.y >= TEXT_LINES * window::LINE {
                        break;
                    }
                    let (x, y) = (TEXT_POS.0 + p.x, TEXT_POS.1 + dy + p.y);
                    font.draw_char(canvas, p.ch, x, y, p.italic, 255);
                }
            }
            if e.voice.is_some() {
                if let Some(a) = area(log_ids::VOICE + i) {
                    draw_cell(canvas, &sheet, VOICE, state(log_ids::VOICE + i), a);
                }
                if let Some(a) = area(log_ids::CHECK + i) {
                    let row = match e.marked {
                        true => TICKED,
                        false => state(log_ids::CHECK + i).min(2),
                    };
                    draw_cell(canvas, &sheet, CHECK, row, a);
                }
            }
            if let Some(a) = area(log_ids::JUMP + i) {
                draw_cell(canvas, &sheet, JUMP, state(log_ids::JUMP + i), a);
            }
        }
        // No thumb while everything fits.
        if log.len() > LOG_ROWS {
            let column = THUMB_COLUMNS[state(log_ids::SCROLL).min(2) as usize];
            let (y, length) = thumb(b.top, log.len());
            draw_thumb(canvas, &sheet, column, y, length);
        }
        if let Some(text) = hover.and_then(|id| self.log_hint(id)) {
            let n = text.lines().count() as i32;
            for (k, line) in text.lines().enumerate() {
                let y = HINT_TEXT.1 - HINT_RAISE * (n - 1) + k as i32 * HINT_LINE;
                let style = gdi::Style::OUTLINED;
                gdi::draw_line(
                    canvas,
                    line,
                    HINT_TEXT.0,
                    y,
                    HINT_SIZE,
                    HINT_ADVANCE,
                    style,
                    None,
                );
            }
        }
        Ok(())
    }
}

/// The thumb: a top cap, middle pieces of up to [`THUMB_MIDDLE`], and a
/// bottom cap.
fn draw_thumb(canvas: &mut Canvas, sheet: &Arc<Image>, column: u32, y: i32, length: i32) {
    let x = TRACK.0;
    let cap = THUMB_CAP as i32;
    canvas.draw_part(sheet, (column, 0, 24, THUMB_CAP), x, y, false, 255);
    let mut at = y + cap;
    let end = y + length - cap;
    while at < end {
        let h = (end - at).min(THUMB_MIDDLE as i32);
        canvas.draw_part(sheet, (column, THUMB_CAP, 24, h as u32), x, at, false, 255);
        at += h;
    }
    let bottom = THUMB_CAP + THUMB_MIDDLE;
    canvas.draw_part(sheet, (column, bottom, 24, THUMB_CAP), x, end, false, 255);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_shrinks_and_moves_two_pixels_an_entry() {
        assert_eq!(thumb(0, 2), (85, 863));
        // Seen under Wine: 63 entries at the bottom.
        assert_eq!(thumb(60, 63), (205, 743));
    }
}
