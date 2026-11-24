//! The message window of なないろリンカネーション, from the executable.
//!
//! The window is the alpha mask in the top 199 rows of `meswaku.akb`,
//! filled with the window colour, on a layer at the top given by
//! `/Config/MesWnd/Pos` (521 in a new install). The buttons are cells of
//! the same sheet laid out over the click areas of `mes.area`, which are
//! relative to the window.

use silky_format::area::{self, Area};

use crate::assets::Assets;
use crate::render::Canvas;
use crate::ui::{bar, Button, Ui};
use crate::Result;

/// Sheet of the window and its buttons.
pub const SHEET: &str = "meswaku.akb";
/// Top of the window on screen.
pub const TOP: i32 = 521;
/// Alpha mask of the window on the sheet.
pub const BAND: (u32, u32, u32, u32) = (0, 0, 1280, 199);

/// Window colour (red, green, blue of 268) and transparency (of 268) of a
/// new install; the executable scales each by 255 / 268.
pub const COLOUR: [u32; 3] = [19, 70, 133];
pub const ALPHA: u32 = 50;
pub const SCALE: u32 = 268;

/// The window buttons: (bar id, click area on screen, sheet cell, cell
/// size, state row step). State rows are normal, hover, pressed and
/// disabled.
pub struct Bar {
    buttons: Vec<(i32, Area, (u32, u32), (u32, u32), u32)>,
}

impl Bar {
    pub fn load(assets: &Assets) -> Result<Self> {
        let mut buttons = Vec::new();
        for a in area::parse(&assets.data("mes.area")?)? {
            let (id, cell, size, step) = match a.id {
                0 => (bar::SAVE, (0, 433), (77, 22), 22),
                1 => (bar::LOAD, (77, 433), (77, 22), 22),
                2 => (bar::QUICK_SAVE, (154, 433), (77, 22), 22),
                3 => (bar::QUICK_LOAD, (231, 433), (77, 22), 22),
                4 => (bar::LOOP_VOICE, (308, 433), (77, 22), 22),
                5 => (bar::HIDE, (646, 433), (45, 44), 44),
                6 => (bar::AUTO, (1000, 199), (45, 44), 44),
                7 => (bar::SKIP, (1045, 199), (45, 44), 44),
                8 => (bar::NEXT_CHOICE, (1135, 199), (45, 44), 44),
                9 => (bar::PREVIOUS_CHOICE, (1090, 199), (45, 44), 44),
                10 => (bar::TITLE, (511, 433), (45, 44), 44),
                11 => (bar::BACKLOG, (556, 433), (45, 44), 44),
                12 => (bar::CONFIG, (601, 433), (45, 44), 44),
                _ => continue,
            };
            let mut a = a;
            a.top += TOP;
            a.bottom += TOP;
            buttons.push((id, a, cell, size, step));
        }
        Ok(Self { buttons })
    }

    pub fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(id, a, ..)| a.contains(x, y) && ui.enabled(Button::Bar(*id)))
            .map(|&(id, ..)| Button::Bar(id))
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        for &(id, a, (sx, sy), (w, h), step) in &self.buttons {
            let b = Button::Bar(id);
            // Auto, skip and voice loop show pressed while on.
            let active = match id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip,
                bar::LOOP_VOICE => ui.loop_voice,
                _ => false,
            };
            let state = ui.state(b, ui.enabled(b), active) as u32;
            canvas.draw_part(
                &sheet,
                (sx, sy + step * state, w, h),
                a.left,
                a.top,
                false,
                255,
            );
        }
        Ok(())
    }
}
