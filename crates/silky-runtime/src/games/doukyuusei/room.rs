//! The room screen of 同級生 Remake (`Game.16`, the program's
//! `CAreaSelect`): the hero looks at the things of a place or uses the
//! buttons at the bottom.
//!
//! Read from the executable and checked under Wine: the buttons are
//! cells of `search_p.akb`, three large ones (168 x 171) from x = 1752
//! leftwards at y = 909 and two small ones (150 x 153) at x = 15 and 165,
//! y = 927; `areasel_0N.carea` (a byte per screen pixel, 255 for none)
//! says which button a pixel belongs to for each button set. In set 0
//! the buttons are go out, look and sleep; the other sets have a back
//! arrow in place of go out and talk in place of sleep. The look button
//! hides the others and shows the back arrow; a click on a thing then
//! returns 1 × 256 + its id in the place's `.area` file. A button returns
//! 255 × 256 + its id. See `docs/games/doukyuusei.md`.
//!
//! `Game.17(1)` (the program's plain area selection) is the same without
//! buttons: the hot spots of the file `Game.17(file, 0)` named and the
//! hint bar, until a thing is clicked.

use std::sync::Arc;

use silky_format::area::{self, Area};
use silky_format::map::MapFile;

use super::window::Font;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

/// Ids of the buttons in the `.carea` maps: go out (back in the other
/// sets), look, sleep (talk), quick save and quick load (gray without a
/// quick save).
pub const EXIT: u8 = 0;
pub const LOOK: u8 = 1;
pub const SLEEP: u8 = 2;
pub const QUICK_SAVE: u8 = 3;
pub const QUICK_LOAD: u8 = 4;
/// Result high byte of a button and of a look.
pub const BUTTON: i32 = 255;
pub const VERB_LOOK: i32 = 1;

/// What a click on the room screen does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// Return this to the script.
    Result(i32),
    /// Switch look mode on or off.
    Look(bool),
    /// Quick save and quick load. The save holds the message before the
    /// room; loading it shows that message, then the room again.
    QuickSave,
    QuickLoad,
    /// Open a system screen over it.
    Settings,
    Backlog,
    Save,
    Load,
    /// Ask before opening all of a girl's pictures (the album).
    Unlock,
    /// Nothing happens.
    None,
}

/// Where a pointer is, as [`crate::ui::Button::Room`] ids: buttons by
/// their `.carea` id, things as 256 + area id, the back arrow 512.
pub const THING: i32 = 256;
pub const BACK: i32 = 512;

#[derive(Clone)]
pub struct Room {
    /// Button map of this button set (1920 x 1080 bytes).
    buttons: Arc<Vec<u8>>,
    /// Back arrow map (look mode).
    back: Arc<Vec<u8>>,
    /// The place's hot spots; many rectangles can share an id.
    spots: Vec<Area>,
    /// The button set: 0 has go out and sleep.
    set: i32,
    /// `SYSTEM_<lang>.MAP`, for the hint bar.
    system: Option<Arc<MapFile>>,
    pub look: bool,
    /// The plain area selection: things only, no buttons or back arrow.
    plain: bool,
}

impl std::fmt::Debug for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Room")
            .field("spots", &self.spots.len())
            .field("look", &self.look)
            .finish()
    }
}

const W: usize = 1920;
const H: usize = 1080;

/// The `.area` file of a background: `42_x.akb` -> `42.area`. Only for
/// a room the scripts gave no file for (`Game.17`).
pub fn area_name(background: &str) -> String {
    let stem = background.rsplit_once('.').map_or(background, |(s, _)| s);
    let place = stem.split('_').next().unwrap_or(stem);
    format!("{place}.area")
}

/// The back arrow: 138 x 140 cells from (672, 0), at the go-out button's
/// place.
const BACK_CELL: (u32, u32, u32) = (672, 138, 140);
const BACK_POS: (i32, i32) = (1782, 940);
/// Top left of the hint bar's text.
const HINT_TEXT: (i32, i32) = (725, 1029);

/// Large buttons: sheet column (sleep, look, talk, go out) by id.
fn big_column(id: u8, set: i32) -> u32 {
    match id {
        EXIT => 3,
        LOOK => 1,
        _ if set != 0 => 2,
        _ => 0,
    }
}

impl Room {
    /// `set` picks `areasel_0N.carea`: 0 for exit, look and sleep.
    /// `area` is the hot spots' file, else the background's.
    pub fn load(
        assets: &Assets,
        area: Option<&str>,
        background: Option<&str>,
        set: i32,
        language: Language,
    ) -> Result<Self> {
        let map = |n: i32| -> Result<Arc<Vec<u8>>> {
            let mut d = assets.data(&format!("areasel_{n:02}.carea"))?;
            d.resize(W * H, 255);
            Ok(Arc::new(d))
        };
        let name = area
            .map(str::to_string)
            .or_else(|| background.map(area_name));
        let spots = match name {
            Some(name) => assets
                .data(&name)
                .ok()
                .and_then(|d| area::parse(&d).ok())
                .unwrap_or_default(),
            None => Vec::new(),
        };
        Ok(Self {
            buttons: map(set.clamp(0, 3))?,
            back: map(3)?,
            spots,
            set: set.clamp(0, 3),
            system: assets
                .data(&format!("system_{}.map", language.suffix()))
                .ok()
                .and_then(|d| MapFile::parse(&d).ok())
                .map(Arc::new),
            look: false,
            plain: false,
        })
    }

    /// The plain area selection of `Game.17(1)` over the hot spots of
    /// `area`.
    pub fn plain(assets: &Assets, area: Option<&str>, language: Language) -> Result<Self> {
        let mut room = Self::load(assets, area, None, 0, language)?;
        room.look = true;
        room.plain = true;
        Ok(room)
    }

    /// Whether this is the plain area selection of `Game.17(1)`.
    pub fn is_plain(&self) -> bool {
        self.plain
    }

    /// The button set (`areasel_0N.carea`).
    pub fn set(&self) -> i32 {
        self.set
    }

    /// Ids of the things that can be looked at, in order.
    pub fn things(&self) -> Vec<i32> {
        let mut ids: Vec<i32> = self.spots.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn at(map: &[u8], x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 {
            return None;
        }
        Some(map[y as usize * W + x as usize]).filter(|&v| v != 255)
    }

    /// The element under (x, y), as an id for [`crate::ui::Button::Room`].
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if self.look {
            if !self.plain && Self::at(&self.back, x, y).is_some() {
                return Some(BACK);
            }
            return self
                .spots
                .iter()
                .find(|a| a.contains(x, y))
                .map(|a| THING + a.id);
        }
        Self::at(&self.buttons, x, y).map(i32::from)
    }

    /// The hint bar's line of `SYSTEM_<lang>.MAP` for the element under
    /// the pointer: what a button does, the verb of a thing (239 + the
    /// high byte of its id: look, talk, hit, push, open and so on), back.
    /// Nothing while the pointer is on none.
    pub fn hint(&self, hover: i32) -> Option<u32> {
        Some(match hover {
            BACK => 252,
            i if i >= THING => 239 + ((i - THING) >> 8) as u32,
            i => match i as u8 {
                EXIT if self.set == 0 => 232,
                EXIT => 233,
                LOOK => 234,
                SLEEP if self.set == 0 => 235,
                SLEEP => 236,
                QUICK_SAVE => 237,
                QUICK_LOAD => 238,
                _ => return None,
            },
        })
    }

    pub fn click(&self, id: i32) -> Click {
        match id {
            BACK => Click::Look(false),
            i if i >= THING => Click::Result(VERB_LOOK * 256 + (i - THING)),
            i if i == LOOK as i32 => Click::Look(true),
            i if i == QUICK_SAVE as i32 => Click::QuickSave,
            i if i == QUICK_LOAD as i32 => Click::QuickLoad,
            i => Click::Result(BUTTON * 256 + i),
        }
    }

    /// Draws the buttons; `state` gives the row (0 normal, 1 hover, 2
    /// pressed, 3 gray) of an element id.
    /// `hover` is the element under the pointer, for the hint bar.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
        font: Option<&Font>,
    ) -> Result<()> {
        let sheet = assets.image("search_p.akb")?;
        // The hint bar, and its line in the half-size font from (725,
        // 1029).
        canvas.draw_part(&sheet, (0, 684, 584, 68), 658, 1005, false, 255);
        let line = hover
            .and_then(|h| self.hint(h))
            .and_then(|id| self.system.as_ref()?.get(0, id));
        if let (Some(line), Some(font)) = (line, font) {
            font.draw_small(canvas, line, HINT_TEXT.0, HINT_TEXT.1);
        }
        let back = |canvas: &mut Canvas, s: u32| {
            let (x, w, h) = BACK_CELL;
            canvas.draw_part(&sheet, (x, h * s, w, h), BACK_POS.0, BACK_POS.1, false, 255);
        };
        if self.plain {
            return Ok(());
        }
        if self.look {
            back(canvas, state(BACK));
            return Ok(());
        }
        let present = |id: u8| self.buttons.iter().any(|&v| v == id);
        for id in [EXIT, LOOK, SLEEP] {
            if !present(id) {
                continue;
            }
            let s = state(id as i32);
            if id == EXIT && self.set != 0 {
                back(canvas, s);
                continue;
            }
            let x = 1752 - 168 * id as i32;
            let part = (168 * big_column(id, self.set), 171 * s, 168, 171);
            canvas.draw_part(&sheet, part, x, 909, false, 255);
        }
        // Quick save and quick load (gray, row 3, without a save).
        for (x, sx, id) in [(15, 960, QUICK_SAVE), (165, 810, QUICK_LOAD)] {
            if !present(id) {
                continue;
            }
            let s = state(id as i32);
            canvas.draw_part(&sheet, (sx, 153 * s, 150, 153), x, 927, false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(area_name("42_X.akb"), "42.area");
        assert_eq!(area_name("cafe.akb"), "cafe.area");
    }
}
