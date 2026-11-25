//! きまぐれテンプテーション's message window and button bar.
//!
//! The window is a band of `message_p.akb` (rows 0..220, an alpha mask the
//! engine fills with the window colour; rows 220..440, a diamond texture)
//! whose top is the `/Config/MesWnd/Pos` setting, 505 unless the player
//! drags the window. The text sits in a box from (280, 66) to (1120, 202)
//! below that top, and the speaker name, when the script gives one, is one
//! line above it at (280, 30).
//!
//! The bar is thirteen 44 x 36 buttons along the bottom edge, drawn from
//! the same sheet: button `n` is column `n` from x = 682, state rows from
//! y = 440 (normal, hover, pressed or on, off). `mes_o.area` holds their
//! click areas, in screen pixels, and the hide cross (id 13) relative to
//! the window top. The cross is a 33 x 33 cell from (682, 584).

use std::sync::Arc;

use silky_format::area::{self, Area};

use crate::assets::Assets;
use crate::render::Canvas;
use crate::ui::{bar, Button, State, Ui};
use crate::Result;

/// Top of the message window (`/Config/MesWnd/Pos` default).
pub const TOP: i32 = 505;
/// The alpha mask and the texture over it on `message_p.akb`.
pub const BAND: (u32, u32, u32, u32) = (0, 0, 1280, 220);
pub const TEXTURE: (u32, u32, u32, u32) = (0, 220, 1280, 220);

/// Left of the text box, and where the first line's baseline falls below
/// the window top (measured under Wine: kana rows 582..602 at the top).
pub const TEXT_LEFT: f32 = 280.0;
pub const TEXT_BASELINE: f32 = 602.0;
/// Pen advance of a full-width character, the line pitch and the box width.
pub const TEXT_CELL: f32 = 28.0;
pub const TEXT_LINE: f32 = 36.0;
pub const TEXT_WIDTH: f32 = 840.0;

/// The button cells.
const SHEET: (u32, u32) = (682, 440);
const CELL: (u32, u32) = (44, 36);
const CROSS_SHEET: (u32, u32) = (682, 584);
const CROSS_CELL: u32 = 33;
/// The bar ids: the hide cross, and the last button, which folds the
/// other buttons away and back.
const CROSS: i32 = 13;
const FOLD: i32 = 12;
/// The notebook button, which blinks when the notebook is unlocked: five
/// rounds of 400 ms, lit on the first, third and fifth.
const NOTEBOOK: i32 = 11;
const BLINK_ROUND_MS: u64 = 400;
const BLINK_ROUNDS: u64 = 5;
/// Folding the bar moves the twelve buttons down off the screen, and
/// unfolding moves them back, over twelve passes of the engine's sprite
/// animation (60 a second), in a straight line.
const SLIDE_PASSES: i64 = 12;
const SCREEN_H: i32 = 720;

/// Click-wait bat: four 84 x 84 frames down the sheet at (1281, 0) (a
/// second set at x = 1365 is used while a voice plays), at (1126, window
/// top + 90). The engine steps it every tenth tick, 200 ms as measured
/// for 景の海のアペイリア, which runs the same animation code.
const WAIT_SHEET: (u32, u32) = (1281, 0);
const WAIT_CELL: u32 = 84;
const WAIT_FRAMES: u64 = 4;
const WAIT_FRAME_MS: u64 = 200;
const WAIT_POS: (i32, i32) = (1126, TOP + 90);

/// What a button of the bar does, by its `mes_o.area` id, as a [`bar`]
/// action: save, load, quick save, quick load, voice loop, jump to the
/// next selection (the executable's scene skip), backlog, auto, skip read
/// text, skip all text, settings, notebook, fold the bar.
pub fn action(id: i32) -> i32 {
    match id {
        0 => bar::SAVE,
        1 => bar::LOAD,
        2 => bar::QUICK_SAVE,
        3 => bar::QUICK_LOAD,
        4 => bar::LOOP_VOICE,
        5 => bar::NEXT_CHOICE,
        6 => bar::BACKLOG,
        7 => bar::AUTO,
        8 => bar::SKIP,
        9 => bar::SKIP_ALL,
        10 => bar::CONFIG,
        11 => bar::K1_NOTEBOOK,
        FOLD => bar::RING,
        _ => bar::HIDE,
    }
}

pub struct Bar {
    /// (id, click area in screen pixels).
    areas: Vec<(i32, Area)>,
}

impl Bar {
    pub fn load(assets: &Assets) -> Result<Self> {
        let mut areas = Vec::new();
        for mut a in area::parse(&assets.data("mes_o.area")?)? {
            if !(0..=CROSS).contains(&a.id) {
                continue;
            }
            if a.id == CROSS {
                a.top += TOP;
                a.bottom += TOP;
            }
            areas.push((a.id, a));
        }
        Ok(Self { areas })
    }

    /// Buttons that show: with the bar folded, the last button and the
    /// cross.
    fn shown<'a>(&'a self, ui: &'a Ui) -> impl Iterator<Item = &'a (i32, Area)> {
        self.areas
            .iter()
            .filter(move |(id, _)| !ui.ring_closed || *id >= FOLD)
    }

    /// How far button `id` is below its place while the bar slides.
    fn slide_dy(ui: &Ui, id: i32, top: i32) -> Option<i32> {
        let (opening, start) = ui.k1_fold?;
        if id >= FOLD {
            return None;
        }
        let t = i64::from(crate::games::doukyuusei::menu::passes(start, ui.clock));
        if t >= SLIDE_PASSES {
            return None;
        }
        let way = i64::from(SCREEN_H - top);
        let t = if opening { SLIDE_PASSES - t } else { t };
        Some((way * t / SLIDE_PASSES) as i32)
    }

    pub fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        self.shown(ui)
            .find(|(_, a)| a.contains(x, y))
            .map(|(id, _)| Button::Bar(action(*id)))
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui) -> Result<()> {
        let sheet = assets.image("message_p.akb")?;
        for (id, a) in &self.areas {
            let dy = Self::slide_dy(ui, *id, a.top);
            if dy.is_none() && ui.ring_closed && *id < FOLD {
                continue;
            }
            let dy = dy.unwrap_or(0);
            let b = Button::Bar(action(*id));
            let active = match *id {
                7 => ui.auto,
                8 => ui.skip && !ui.skip_all,
                9 => ui.skip && ui.skip_all,
                4 => ui.loop_voice,
                _ => false,
            };
            let mut state = ui.state(b, ui.enabled(b), active);
            if *id == NOTEBOOK {
                if let Some(start) = ui.k1_blink {
                    let round = ui.clock.saturating_sub(start) / BLINK_ROUND_MS;
                    if round < BLINK_ROUNDS {
                        state = if round % 2 == 0 {
                            State::Pressed
                        } else {
                            State::Normal
                        };
                    }
                }
            }
            if *id == CROSS {
                // The cross has no off row.
                let row = (state as u32).min(2);
                let part = (
                    CROSS_SHEET.0,
                    CROSS_SHEET.1 + CROSS_CELL * row,
                    CROSS_CELL,
                    CROSS_CELL,
                );
                canvas.draw_part(&sheet, part, a.left, a.top + dy, false, 255);
            } else {
                let part = (
                    SHEET.0 + CELL.0 * *id as u32,
                    SHEET.1 + CELL.1 * state as u32,
                    CELL.0,
                    CELL.1,
                );
                canvas.draw_part(&sheet, part, a.left, a.top + dy, false, 255);
            }
        }
        if ui.waiting {
            let frame = (ui.clock / WAIT_FRAME_MS) % WAIT_FRAMES;
            let (x, y) = WAIT_SHEET;
            let part = (x, y + WAIT_CELL * frame as u32, WAIT_CELL, WAIT_CELL);
            canvas.draw_part(&sheet, part, WAIT_POS.0, WAIT_POS.1, false, 255);
        }
        Ok(())
    }
}

/// Draws the band and its texture at the window position; `rgb` is the
/// window colour and `alpha` its opacity.
pub fn draw_band(
    canvas: &mut Canvas,
    parts: &Arc<silky_format::akb::Image>,
    rgb: [u8; 3],
    alpha: u8,
) {
    canvas.draw_part(parts, BAND, 0, TOP, false, alpha);
    canvas.draw_part(parts, TEXTURE, 0, TOP, false, 255);
    if rgb != [0; 3] {
        let (_, _, w, h) = BAND;
        canvas.fill(0, TOP, w, h, [rgb[0], rgb[1], rgb[2], alpha / 2]);
    }
}
