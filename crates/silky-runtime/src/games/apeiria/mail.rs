//! The αMAIL screen of 景の海のアペイリア (`Game.1012`), read from the
//! executable: a glass panel (`SLD.akb`) with the mail list
//! (`mail01_p.akb`), a "new mail" notice and an open mail
//! (`mail02_p.akb`), or the Apeiria system window. The panel wipes in
//! from its middle with rising light specks and closes by shrinking to
//! a tenth of its height.

use std::sync::Arc;

use silky_format::akb::Image;

use crate::assets::Assets;
use crate::canvas::Canvas;
use crate::text::TextRenderer;

/// State of the screen, kept with the scene so saves keep it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Mail {
    /// 0 inbox, 1 sent tray.
    pub tray: i32,
    /// Number of the newest mail, from 1 (byte 1200 or 1400).
    pub newest: u8,
    /// The panel is up (sub 1 to sub 2).
    pub shown: bool,
    /// "n new mails received" notice (sub 5 to sub 6).
    pub notice: Option<i32>,
    /// Row picked in the list (sub 8 to sub 9).
    pub selected: Option<i32>,
    /// Text of the open mail, lines split (sub 3 to sub 4).
    pub body: Option<String>,
    /// Page of the open mail's text (sub 10): 381 pixels a page.
    pub body_page: i32,
    /// The panel holds the Apeiria system window instead of the list
    /// (sub 7, with the mode its animation takes).
    pub system: Option<i32>,
    /// The panel opening or closing.
    pub anim: Option<MailAnim>,
    /// Light specks rising into it as it opens.
    pub lights: Option<super::holo::Lights>,
}

/// The panel opening (sub 1): it wipes in through rule picture 46 (the
/// panel's size, lighter outwards) over 64 frames (the same in
/// Casablanca, 0x479720) while light specks
/// rise into it; or closing (sub 2): over 16 frames its height goes to a
/// tenth about its middle (479, 259), and it is gone. The script waits
/// for either.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MailAnim {
    pub opening: bool,
    /// Time since the start.
    pub ms: u64,
}

/// Rule picture of the panel wiping in.
pub const WIPE_RULE: u32 = 46;
const CLOSE_FRAMES: i32 = 16;

impl MailAnim {
    pub fn frames(&self) -> i32 {
        if self.opening {
            super::holo::MAIL_FRAMES
        } else {
            CLOSE_FRAMES
        }
    }

    pub fn length_ms(&self) -> u64 {
        self.frames() as u64 * super::anim::TICK_MS
    }

    pub fn done(&self) -> bool {
        self.ms >= self.length_ms()
    }

    /// Frames stepped so far.
    fn stepped(&self) -> u64 {
        (self.ms / super::anim::TICK_MS).min(self.frames() as u64)
    }

    /// How far it is, 0 to 1. Opening, the wipe is stepped `frames - 1`
    /// times by an even amount (Casablanca 0x4c3190: 512 / (frames - 1)
    /// a step), so the picture is whole at the last frame the script
    /// waits for, one before the animation is over; closing, it is
    /// `frames` steps.
    pub fn progress(&self) -> f32 {
        let n = self.frames();
        let steps = if self.opening { n - 1 } else { n };
        (self.stepped() as f32 / steps as f32).min(1.0)
    }

    /// Casablanca's wipe level, 0 to 512 in whole units: a pixel of rule
    /// value `r` shows at opacity `level - r` up to its own (0x4c3880).
    pub fn level(&self) -> f32 {
        (self.stepped() as f32 * (512.0 / (self.frames() - 1) as f32)).floor()
    }
}

impl Mail {
    /// Starts the panel opening or closing; the time the script waits.
    pub fn animate(&mut self, opening: bool, seed: u32) -> u64 {
        let a = MailAnim { opening, ms: 0 };
        self.anim = Some(a);
        self.lights = opening.then(|| super::holo::Lights::mail(seed));
        a.length_ms()
    }

    /// Steps the animation: whether it moved, and whether the panel is
    /// gone.
    pub fn advance(&mut self, ms: u64) -> (bool, bool) {
        let Some(a) = &mut self.anim else {
            return (false, false);
        };
        a.ms += ms;
        let done = a.done();
        let closed = done && !a.opening;
        if let Some(l) = &mut self.lights {
            l.advance(ms);
            if l.done() {
                self.lights = None;
            }
        }
        if done {
            self.anim = None;
            self.lights = None;
        }
        (true, closed)
    }

    /// Ends the animation at once; whether the panel is gone.
    pub fn finish(&mut self) -> bool {
        let closed = self.anim.is_some_and(|a| !a.opening);
        self.anim = None;
        self.lights = None;
        closed
    }

    /// The panel's top left and size.
    pub fn panel() -> ((i32, i32), (u32, u32)) {
        (PANEL, PANEL_SIZE)
    }
}

/// The system window in the panel (sub 7): the pieces of the DNA PC's
/// window ([`super::panels::SYSTEM_PARTS`]) from (309, 36).
const SYSTEM_AT: (i32, i32) = (PANEL.0 + 154, PANEL.1 + 36);
/// The open mail's text scrolls in pages of 381 pixels, shown from 84
/// pixels below the top of the open mail; the last page ends 365 pixels
/// below the end of the text.
const BODY_PAGE: i32 = 381;
const BODY_VIEW_TOP: i32 = 84;
const BODY_LAST: i32 = 365;

/// Glass panel: the top 958 x 518 of `SLD.akb`.
const PANEL: (i32, i32) = (155, 0);
const PANEL_SIZE: (u32, u32) = (958, 518);
/// The list: a 375 x 468 window of `mail01_p.akb` from (0, 142).
const LIST: (i32, i32) = (PANEL.0 + 299, PANEL.1 + 50);
const LIST_SHEET: (u32, u32, u32, u32) = (0, 142, 375, 468);
/// The list's body is seen down to row 447 of its window; its last 42
/// rows fade out linearly (measured against Wine), so the white list
/// ends in the glass instead of in an edge.
const LIST_FADE: (u32, u32) = (405, 447);
/// Tray name, 120 x 24, from (0, 662) (inbox) or (0, 686) (sent).
const TRAY: (i32, i32) = (113, 48);
/// Newest mail row: 346 x 70 cells, 13 a column, in 692-pixel columns
/// from x = 1067 (the picked look 346 pixels to the right).
const ROW: (i32, i32) = (6, 83);
const ROW_SIZE: (u32, u32) = (346, 70);
/// Older rows: 346 x 210 from (375, 30), then its first 48 rows again
/// below.
const OLDER: (i32, i32) = (6, 153);
const OLDER_SHEET: (u32, u32, u32, u32) = (375, 30, 346, 210);
const OLDER_MORE: (i32, i32) = (6, 363);
const OLDER_MORE_SHEET: (u32, u32, u32, u32) = (375, 30, 346, 48);
/// Scroll bar, 12 x 179 from (375, 382).
const SCROLL: (i32, i32) = (361, 114);
const SCROLL_SHEET: (u32, u32, u32, u32) = (375, 382, 12, 179);
/// Notice: 309 x 142 from (0, 0); the count is a 33 x 41 digit from
/// (375 + 33 n, 341), the wing a 63 x 52 picture from (309, 0).
const NOTICE: (i32, i32) = (491, 184);
const NOTICE_SHEET: (u32, u32, u32, u32) = (0, 0, 309, 142);
const DIGIT: (i32, i32) = (139, 27);
const WING: (i32, i32, u32, u32, u32, u32) = (203, 29, 309, 0, 63, 52);
/// Open mail: 461 x 471 from the top left of `mail02_p.akb`; the text,
/// 12-pixel characters on a 16-pixel line, was placed as measured under
/// Wine.
const BODY: (i32, i32) = (578, 31);
const BODY_SHEET: (u32, u32, u32, u32) = (0, 0, 461, 471);
const BODY_TEXT: (f32, f32) = (598.0, 127.0);
const BODY_SIZE: f32 = 12.0;
const BODY_LINE: f32 = 16.0;
/// Lines longer than this are wrapped (the panel's inner width; not
/// measured yet).
const BODY_WIDTH: f32 = 420.0;

impl Mail {
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        text: Option<&TextRenderer>,
    ) -> crate::Result<()> {
        if !self.shown {
            return Ok(());
        }
        let list = assets.image("mail01_p.akb")?;
        let glass = assets.image("sld.akb")?;
        let (pw, ph) = PANEL_SIZE;
        let sq = self.anim.filter(|a| !a.opening).map(|a| {
            let mid = (PANEL.1 + PANEL_SIZE.1 as i32 / 2) as f32;
            (mid, 1.0 - 0.9 * a.progress())
        });
        part(canvas, sq, &glass, (0, 0, pw, ph), PANEL);
        if self.system.is_some() {
            use super::panels::{SYSTEM_PARTS, SYSTEM_SHEET};
            let sheet = assets.image(SYSTEM_SHEET)?;
            for (src, (dx, dy)) in SYSTEM_PARTS {
                part(
                    canvas,
                    sq,
                    &sheet,
                    src,
                    (SYSTEM_AT.0 + dx, SYSTEM_AT.1 + dy),
                );
            }
            return Ok(());
        }
        let at = |p: (i32, i32)| (LIST.0 + p.0, LIST.1 + p.1);
        let (fade_from, fade_to) = LIST_FADE;
        let (sx, sy, sw, _) = LIST_SHEET;
        part(canvas, sq, &list, (sx, sy, sw, fade_from), LIST);
        for row in fade_from..fade_to {
            let a = list_fade_alpha(row);
            part_a(
                canvas,
                sq,
                &list,
                (sx, sy + row, sw, 1),
                (LIST.0, LIST.1 + row as i32),
                a,
            );
        }
        let tray_row = if self.tray == 0 { 662 } else { 686 };
        part(canvas, sq, &list, (0, tray_row, 120, 24), at(TRAY));
        if self.newest > 0 {
            let i = self.newest as u32 - 1;
            let picked = if self.selected == Some(0) {
                ROW_SIZE.0
            } else {
                0
            };
            let x = 1067 + 692 * (i / 13) + picked;
            let y = ROW_SIZE.1 * (i % 13);
            part(canvas, sq, &list, (x, y, ROW_SIZE.0, ROW_SIZE.1), at(ROW));
        }
        part(canvas, sq, &list, OLDER_SHEET, at(OLDER));
        part(canvas, sq, &list, OLDER_MORE_SHEET, at(OLDER_MORE));
        part(canvas, sq, &list, SCROLL_SHEET, at(SCROLL));
        if let Some(n) = self.notice {
            part(canvas, sq, &list, NOTICE_SHEET, NOTICE);
            let n = n.clamp(0, 9) as u32;
            let d = (NOTICE.0 + DIGIT.0, NOTICE.1 + DIGIT.1);
            part(canvas, sq, &list, (375 + 33 * n, 341, 33, 41), d);
            let (wx, wy, sx, sy, w, h) = WING;
            part(
                canvas,
                sq,
                &list,
                (sx, sy, w, h),
                (NOTICE.0 + wx, NOTICE.1 + wy),
            );
        }
        if let Some(body) = &self.body {
            let sheet = assets.image("mail02_p.akb")?;
            part(canvas, sq, &sheet, BODY_SHEET, BODY);
            if let Some(tr) = text {
                let page = self.body_page;
                tr.with_wide_advance(1.0, |tr| draw_body(canvas, tr, body, page));
            }
        }
        Ok(())
    }
}

/// The open mail's text, its lines wrapped to the panel, from page
/// `page`.
fn draw_body(canvas: &mut Canvas, tr: &TextRenderer, body: &str, page: i32) {
    let (x, y) = BODY_TEXT;
    let lines: Vec<String> = body
        .lines()
        .flat_map(|l| {
            let wrapped = tr.wrap(l, BODY_SIZE, BODY_WIDTH);
            if wrapped.is_empty() {
                vec![String::new()]
            } else {
                wrapped
            }
        })
        .collect();
    let height = (BODY_LINE * lines.len() as f32) as i32;
    let mut scroll = page.max(0) * BODY_PAGE;
    if height < scroll + BODY_PAGE {
        scroll = (height - BODY_LAST).max(0).min(scroll);
    }
    let top = (BODY.1 + BODY_VIEW_TOP) as f32;
    let bottom = top + BODY_PAGE as f32;
    for (k, line) in lines.iter().enumerate() {
        let y = y + BODY_LINE * k as f32 - scroll as f32;
        if y - BODY_SIZE < top || y > bottom {
            continue;
        }
        tr.draw_plain(canvas, line, x, y, BODY_SIZE, [40, 40, 40, 255]);
    }
}

/// Opacity of row `row` of the list's window (255 down to the fade's
/// start, 0 at its end).
fn list_fade_alpha(row: u32) -> u8 {
    let (from, to) = LIST_FADE;
    if row <= from {
        255
    } else if row >= to {
        0
    } else {
        (255 * (to - row) / (to - from)) as u8
    }
}

/// [`part`] at opacity `alpha`.
fn part_a(
    canvas: &mut Canvas,
    squash: Squash,
    img: &Arc<Image>,
    part: (u32, u32, u32, u32),
    at: (i32, i32),
    alpha: u8,
) {
    match squash {
        None => canvas.draw_part(img, part, at.0, at.1, false, alpha),
        Some((mid, k)) => {
            let (x, w, h) = (at.0 as f32, part.2 as f32, part.3 as f32);
            let y = mid + (at.1 as f32 - mid) * k;
            let corners = [[x, y], [x + w, y], [x, y + h * k]];
            canvas.draw_sprite(Some(img), part, corners, [255, 255, 255, alpha], false);
        }
    }
}

/// Draws `part` of `img` at `at`; `squash` (middle, factor) scales it
/// vertically about a line while the panel closes.
fn part(
    canvas: &mut Canvas,
    squash: Squash,
    img: &Arc<Image>,
    part: (u32, u32, u32, u32),
    at: (i32, i32),
) {
    match squash {
        None => canvas.draw_part(img, part, at.0, at.1, false, 255),
        Some((mid, k)) => {
            let (x, w, h) = (at.0 as f32, part.2 as f32, part.3 as f32);
            let y = mid + (at.1 as f32 - mid) * k;
            let corners = [[x, y], [x + w, y], [x, y + h * k]];
            canvas.draw_sprite(Some(img), part, corners, [255; 4], false);
        }
    }
}

type Squash = Option<(f32, f32)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_wipe_is_whole_on_the_last_frame_waited_for() {
        let t = super::super::anim::TICK_MS;
        let at = |k: u64| MailAnim {
            opening: true,
            ms: k * t,
        };
        assert_eq!(at(0).progress(), 0.0);
        assert_eq!(at(63).progress(), 1.0);
        assert_eq!(at(0).level(), 0.0);
        assert_eq!(at(63).level(), 512.0);
        assert!(at(62).level() < 512.0 && at(62).level() > 500.0);
        assert!(!at(63).done() && at(64).done());
    }

    #[test]
    fn list_fades_out_at_its_bottom() {
        assert_eq!(list_fade_alpha(0), 255);
        assert_eq!(list_fade_alpha(LIST_FADE.0), 255);
        assert_eq!(list_fade_alpha(LIST_FADE.1), 0);
        let mid = list_fade_alpha((LIST_FADE.0 + LIST_FADE.1) / 2);
        assert!((120..=135).contains(&mid));
    }
}
