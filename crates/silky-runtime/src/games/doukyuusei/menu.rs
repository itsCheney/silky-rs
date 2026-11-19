//! The menu button of 同級生 Remake, at the top right of the room, the
//! walking maps and the selections made there (with the status plate).
//!
//! Read from the executable: seven sprites of `menu_m_p.akb`, the menu
//! button (186 x 188 cells, four rows: normal, hover, pressed, disabled)
//! at (1729, 2) and, while it is open, six buttons (164 x 164 cells from x
//! 186, the same rows) down the right edge: settings, save, load, backlog,
//! schedule and map. `mesmenu.carea` (a byte per screen pixel, 255 for
//! none) gives the button under a point; the six only count while the menu
//! is open. The program slides them out from the menu button and fades
//! them in over twelve frames; here they show at once. Save and load open
//! the system menu's save screen ([`super::system`]), backlog the game's
//! backlog ([`super::log`]), the schedule button the flag list
//! ([`super::flaglist`], gray until the story unlocks it) and the map button
//! the map jump ([`super::jump`]).

use std::sync::Arc;

use crate::assets::Assets;
use crate::render::Canvas;
use crate::ui::bar;
use crate::Result;

const SHEET: &str = "menu_m_p.akb";
const MAP: &str = "mesmenu.carea";
const W: usize = 1920;
const H: usize = 1080;

/// The menu button and the six, as message window button ids, in their
/// `mesmenu.carea` order.
pub const BUTTONS: [i32; 7] = [
    bar::MENU,
    bar::CONFIG,
    bar::SAVE,
    bar::LOAD,
    bar::BACKLOG,
    bar::SCHEDULE,
    bar::MAP,
];
const POS: [(i32, i32); 7] = [
    (1729, 2),
    (1655, 111),
    (1740, 196),
    (1655, 281),
    (1740, 367),
    (1655, 453),
    (1740, 537),
];
const MENU_CELL: (u32, u32) = (186, 188);
/// A state that leaves a button out.
pub const HIDDEN: u32 = 4;
const CELL: u32 = 164;

#[derive(Clone)]
pub struct Menu {
    map: Arc<Vec<u8>>,
}

impl Menu {
    pub fn load(assets: &Assets) -> Result<Self> {
        let mut map = assets.data(MAP)?;
        map.resize(W * H, 255);
        Ok(Self { map: Arc::new(map) })
    }

    /// The button under (x, y), as a message window button id.
    pub fn hit(&self, open: bool, x: i32, y: i32) -> Option<i32> {
        if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 {
            return None;
        }
        let i = self.map[y as usize * W + x as usize] as usize;
        if i >= BUTTONS.len() || (i > 0 && !open) {
            return None;
        }
        Some(BUTTONS[i])
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed, 3 disabled) of
    /// a button id, or [`HIDDEN`]. The program grays save and load when
    /// they cannot be used, leaves the backlog out while the log is empty
    /// (seen under Wine) and shows the map jump only on the walking maps.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        open: bool,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        let count = if open { BUTTONS.len() } else { 1 };
        for (i, (&id, &(x, y))) in BUTTONS.iter().zip(POS.iter()).take(count).enumerate() {
            let row = state(id);
            if row >= HIDDEN {
                continue;
            }
            let part = if i == 0 {
                let (w, h) = MENU_CELL;
                (0, h * row, w, h)
            } else {
                (MENU_CELL.0 + CELL * (i as u32 - 1), CELL * row, CELL, CELL)
            };
            canvas.draw_part(&sheet, part, x, y, false, 255);
        }
        Ok(())
    }
}

impl std::fmt::Debug for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Menu")
    }
}
