//! The message window of 先輩が私の妄想にドージンする: its button bar,
//! click-wait mark, tooltips, speaker name and the two portrait windows.
//!
//! The window is the 220 pixel band at the bottom of the screen (top at
//! 500, `/Config/MesWnd/Pos`). `mes_o.area` gives the click areas of the
//! fifteen buttons in screen pixels: thirteen 36 x 38 cells of
//! `message_p.akb` in a row from (0, 660), four state rows 41 apart
//! (normal, hover, pressed, grey), then the window hide cross and the
//! padlock that keeps the bar up.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::{self, Area};

use crate::assets::{Assets, HelpLines, Language};
use crate::render::Canvas;
use crate::text::TextRenderer;
use crate::ui::{bar, Button, Ui};
use crate::Result;

/// Sheet of the window and its buttons.
pub const SHEET: &str = "message_p.akb";
/// Top of the window on screen.
pub const TOP: i32 = 500;

/// First cell of the button row, its size, and the distance between
/// state rows.
const CELL_ORIGIN: (u32, u32) = (0, 660);
const CELL: (u32, u32) = (36, 38);
const STEP: u32 = 41;
/// Cell of each button by area id: the title button (11) shows the home
/// cell and the quit button (12) the power cell, the sheet's order
/// swapped; hide takes the cross.
const CELLS: [u32; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 11, 13, 0];
/// The padlock: cells 37 x 39 at x = 423, rows 39 apart, from y = 812
/// with the bar kept up and from y = 929 when it is not.
const LOCK_X: u32 = 423;
const LOCK_SIZE: (u32, u32) = (37, 39);
const LOCK_ON: u32 = 812;
const LOCK_OFF: u32 = 929;

/// With the pin off the thirteen buttons slide 40 pixels down out of
/// sight (the hide cross and the pin stay), as the bar of きまぐれ
/// テンプテーション2 does.
const DROP: i32 = 40;

/// The full-screen picture shown while jumping to the next unread text or
/// selection.
pub const SKIP_PICTURE: &str = "skip.akb";

/// The click-wait mark: ten 44 pixel frames down the sheet from (504,
/// 660), shown 174 below the window top at x = 1021.
const WAIT_ORIGIN: (u32, u32) = (504, 660);
const WAIT_CELL: u32 = 44;
const WAIT_FRAMES: u64 = 10;
const WAIT_AT: (i32, i32) = (1021, TOP + 174);
const WAIT_FRAME_MS: u64 = 150;

/// Left of the text, and the width of a line (33 cells of 28 pixels and a
/// little over).
pub const TEXT_LEFT: f32 = 180.0;
pub const TEXT_WIDTH: f32 = 930.0;

/// The speaker's name is set in the text's own size, without the text's
/// cells, on a layer 178 across and 27 down from the window's corner.
pub const NAME_LEFT: f32 = 178.0;
pub const NAME_BASELINE: f32 = TOP as f32 + 51.0;
pub const NAME_SIZE: f32 = 24.0;

/// Portrait windows: a 1024 pixel square layer shown at 40 %, its top at
/// y = 320. The body picture is not shrunk: a 1024 wide piece of it, taken
/// from x = 512 of the 2048 pixel picture, is copied into the layer pixel
/// for pixel. The first window (`Game.10`) sits at x = -80 and copies the
/// whole 1024 rows from the picture's top; the second (`Game.16`) sits at
/// x = 950 and copies 880 rows into the layer from row 144 down, so its
/// picture starts 57.6 pixels lower.
pub const LAYER_SCALE: f32 = 0.4;
pub const LAYER_TOP: f32 = 320.0;
pub const PORTRAIT_LEFT: [f32; 2] = [-80.0, 950.0];
const PIECE_LEFT: u32 = 512;
const PIECE_WIDTH: u32 = 1024;
const PIECE_ROWS: [u32; 2] = [1024, 880];
const LAYER_ROW: [u32; 2] = [0, 144];

/// Tooltip lines of `MesButtonHelp.hlg` by button id.
const HELP: [usize; 15] = [7, 9, 8, 10, 6, 3, 5, 4, 1, 0, 2, 13, 14, 11, 12];
const HELP_SIZE: f32 = 16.0;

/// The window buttons: (bar id, click area on screen, sheet cell).
pub struct Bar {
    buttons: Vec<(i32, Area, (u32, u32))>,
    help: Option<HelpLines>,
}

impl Bar {
    pub fn load(assets: &Assets) -> Result<Self> {
        // Buttons by area id: save, load, quick save, quick load, voice
        // loop, jump to the next unread text or selection, backlog, auto,
        // skip read text, skip all, settings, title, quit, hide, pin.
        const IDS: [i32; 15] = [
            bar::SAVE,
            bar::LOAD,
            bar::QUICK_SAVE,
            bar::QUICK_LOAD,
            bar::LOOP_VOICE,
            bar::NEXT_CHOICE,
            bar::BACKLOG,
            bar::AUTO,
            bar::SKIP,
            bar::SKIP_ALL,
            bar::CONFIG,
            bar::TITLE,
            bar::QUIT,
            bar::HIDE,
            bar::PIN,
        ];
        let mut buttons = Vec::new();
        for a in area::parse(&assets.data("mes_o.area")?)? {
            let Some(&id) = usize::try_from(a.id).ok().and_then(|i| IDS.get(i)) else {
                continue;
            };
            let cell = (CELL_ORIGIN.0 + CELL.0 * CELLS[a.id as usize], CELL_ORIGIN.1);
            buttons.push((id, a, cell));
        }
        Ok(Self {
            buttons,
            help: HelpLines::load(assets, "MesButtonHelp"),
        })
    }

    /// How far a button sits below its place.
    fn drop(id: i32, ui: &Ui) -> i32 {
        let _ = id;
        ui.bar_drop.min(DROP)
    }

    pub fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(id, a, _)| {
                let drop = Self::drop(*id, ui);
                drop < DROP - 2 && a.contains(x, y - drop) && ui.enabled(Button::Bar(*id))
            })
            .map(|&(id, ..)| Button::Bar(id))
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        help: Option<(&TextRenderer, Language)>,
    ) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        for &(id, a, (sx, sy)) in &self.buttons {
            let b = Button::Bar(id);
            let active = match id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip && !ui.skip_all,
                bar::SKIP_ALL => ui.skip && ui.skip_all,
                bar::LOOP_VOICE => ui.loop_voice,
                _ => false,
            };
            let state = ui.state(b, ui.enabled(b), active) as u32;
            let part = if id == bar::PIN {
                let top = if ui.menu_lock { LOCK_ON } else { LOCK_OFF };
                let row = state.min(2);
                (LOCK_X, top + LOCK_SIZE.1 * row, LOCK_SIZE.0, LOCK_SIZE.1)
            } else {
                (sx, sy + STEP * state, CELL.0, CELL.1)
            };
            let drop = Self::drop(id, ui);
            if drop < DROP {
                canvas.draw_part(&sheet, part, a.left, a.top + drop, false, 255);
            }
        }
        if ui.waiting {
            let frame = (ui.clock / WAIT_FRAME_MS % WAIT_FRAMES) as u32;
            let (x, y) = WAIT_ORIGIN;
            canvas.draw_part(
                &sheet,
                (x, y + WAIT_CELL * frame, WAIT_CELL, WAIT_CELL),
                WAIT_AT.0,
                WAIT_AT.1,
                false,
                255,
            );
        }
        if let Some((tr, language)) = help {
            self.draw_help(canvas, ui, tr, language);
        }
        Ok(())
    }

    /// The tooltip of the button under the pointer, above it.
    fn draw_help(&self, canvas: &mut Canvas, ui: &Ui, tr: &TextRenderer, language: Language) {
        let Some(Button::Bar(id)) = ui.hover else {
            return;
        };
        let Some(&(_, a, _)) = self.buttons.iter().find(|(i, ..)| *i == id) else {
            return;
        };
        // While a mode is on, its button offers to stop it.
        let stop = match id {
            bar::SKIP_ALL if ui.skip && ui.skip_all => Some("既読・未読両方スキップを停止します"),
            bar::SKIP if ui.skip && !ui.skip_all => Some("既読文章スキップを停止します"),
            bar::AUTO if ui.auto => Some("文章の自動再生を停止します"),
            bar::LOOP_VOICE if ui.loop_voice => Some("音声のループ再生を停止します"),
            _ => None,
        };
        let line = |k: usize| self.help.as_ref()?.line(language, HELP[k]);
        let index = match id {
            bar::SAVE => 0,
            bar::LOAD => 1,
            bar::QUICK_SAVE => 2,
            bar::QUICK_LOAD => 3,
            bar::LOOP_VOICE => 4,
            bar::NEXT_CHOICE => 5,
            bar::BACKLOG => 6,
            bar::AUTO => 7,
            bar::SKIP => 8,
            bar::SKIP_ALL => 9,
            bar::CONFIG => 10,
            bar::TITLE => 11,
            bar::QUIT => 12,
            bar::HIDE => 13,
            bar::PIN => 14,
            _ => return,
        };
        let Some(text) = stop.or_else(|| line(index)) else {
            return;
        };
        // `\` breaks the line.
        let lines: Vec<&str> = text.split('\\').collect();
        let width = lines
            .iter()
            .map(|l| tr.width(l, HELP_SIZE))
            .fold(0.0, f32::max);
        let (pad, pitch) = (10, 22);
        let w = width.ceil() as i32 + 2 * pad;
        let h = pitch * lines.len() as i32 + 2 * pad - 4;
        let x = (a.left + a.width() as i32 / 2 - w / 2).clamp(0, 1280 - w);
        let y = a.top - h - 6;
        canvas.fill(x, y, w as u32, h as u32, [0x30, 0x30, 0x30, 0xe6]);
        for (i, l) in lines.iter().enumerate() {
            let base = (y + pad + pitch * (i as i32 + 1) - 5) as f32;
            tr.draw_plain(canvas, l, (x + pad) as f32, base, HELP_SIZE, [255; 4]);
        }
    }
}

/// A body picture of a portrait window with its face drawn on, put at its
/// place: `window` is 0 for `Game.10` and 1 for `Game.16`.
pub fn draw_portrait(canvas: &mut Canvas, img: &Arc<Image>, window: usize) {
    let rows = PIECE_ROWS[window].min(img.height);
    let width = PIECE_WIDTH.min(img.width.saturating_sub(PIECE_LEFT));
    if width == 0 || rows == 0 {
        return;
    }
    let mut rgba = Vec::with_capacity((width * rows * 4) as usize);
    for y in 0..rows {
        let start = ((y * img.width + PIECE_LEFT) * 4) as usize;
        rgba.extend_from_slice(&img.rgba[start..start + (width * 4) as usize]);
    }
    let piece = Arc::new(Image {
        width,
        height: rows,
        rgba,
    });
    let top = LAYER_TOP + LAYER_ROW[window] as f32 * LAYER_SCALE;
    canvas.draw_scaled(&piece, PORTRAIT_LEFT[window], top, LAYER_SCALE);
}

/// Lays `top` over `canvas` (same width) the way the second portrait's
/// over-picture is laid: a pixel with no alpha leaves the pixel below, a
/// full one replaces its colour, and in between the colours mix by the
/// top's alpha, each part rounded down. The alpha of the pixel below
/// stays as it was, so the picture shows only where the portrait does.
pub fn overlay_keep_alpha(canvas: &mut Canvas, top: &Image) {
    if top.width != canvas.width {
        return;
    }
    let n = (canvas.width.min(top.width) * canvas.height.min(top.height)) as usize;
    for i in 0..n {
        let s = &top.rgba[i * 4..i * 4 + 4];
        let a = s[3] as u32;
        if a == 0 {
            continue;
        }
        let d = &mut canvas.rgba[i * 4..i * 4 + 4];
        for k in 0..3 {
            d[k] = if a == 255 {
                s[k]
            } else {
                ((s[k] as u32 * a / 255) + (d[k] as u32 * (255 - a) / 255)) as u8
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_picture_keeps_alpha_below() {
        let mut c = Canvas::from_rgba(
            3,
            1,
            vec![100, 100, 100, 0, 100, 100, 100, 255, 10, 20, 30, 128],
        );
        let top = Image {
            width: 3,
            height: 1,
            rgba: vec![200, 0, 0, 255, 200, 0, 0, 128, 0, 0, 0, 0],
        };
        overlay_keep_alpha(&mut c, &top);
        assert_eq!(&c.rgba[0..4], &[200, 0, 0, 0]);
        assert_eq!(&c.rgba[4..8], &[149, 49, 49, 255]);
        assert_eq!(&c.rgba[8..12], &[10, 20, 30, 128]);
    }
}
