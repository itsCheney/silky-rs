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
//!
//! Keys, read from the executable. With the buttons up the arrow keys
//! move the focus (the program warps the pointer to a button's middle and
//! the hover follows, see [`arrow`]); the hot spots take no key focus. In
//! look mode and on the plain selection they move the pointer itself, 15
//! pixels a press (45 with Shift). Enter clicks what the pointer is on.
//! The secondary click, Escape, Delete and keypad 0 play the cancel
//! sound and, with the buttons up, return 0xffff to the script (the
//! hero's room asks there whether to leave); in look mode they leave it,
//! on the plain selection they return 0.

use std::sync::Arc;

use silky_format::area::{self, Area};
use silky_format::map::MapFile;

use super::cursor;
use super::gdi;
use super::keys::Arrow;
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
    /// Play a track of the music mode, or stop the music (`None`).
    Track(Option<usize>),
    /// The name entry is done; its name is read from the panel.
    Name,
    /// The map jump page closes and the walk goes on.
    Resume,
    /// A modal notice closes; the script goes on with no value.
    Close,
    /// Nothing happens.
    None,
}

/// What the cancel keys return from the room with the buttons up.
pub const CANCELLED: i32 = 0xffff;
/// Pixels an arrow key moves the pointer in look mode and on the plain
/// selection; with Shift held.
pub const STEP: i32 = 15;
pub const STEP_FAST: i32 = 45;

/// Where the focus is with the buttons up: nothing, a button by its
/// `.carea` id, or one of the menu's buttons (0 the menu button, 1..=6
/// its six).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    None,
    Button(u8),
    Menu(usize),
}

/// Where an arrow key puts the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    Button(u8),
    Menu(usize),
}

/// The target of an arrow key with the buttons up, if it goes anywhere.
/// From nothing every key goes to the menu button. From a button, up goes
/// to the lowest menu button (the fifth, the fourth when that one is
/// left out; the menu button while the menu is shut), left steps along
/// go out or back, look, sleep or talk (not on to the quick buttons) and
/// right back, down does nothing. From the menu, up and down step along
/// it and down from the last of them goes to go out or back; with the
/// menu shut, down goes there too and the other keys stay on the menu
/// button.
pub fn arrow(focus: Focus, key: Arrow, menu_open: bool, hidden5: bool) -> Option<Go> {
    match focus {
        Focus::None => Some(Go::Menu(0)),
        Focus::Button(k) => match key {
            Arrow::Up => Some(Go::Menu(match (menu_open, hidden5) {
                (false, _) => 0,
                (true, true) => 4,
                (true, false) => 5,
            })),
            Arrow::Down => None,
            Arrow::Left => (k != SLEEP).then_some(Go::Button(k + 1)),
            Arrow::Right => (k != EXIT).then(|| Go::Button(k - 1)),
        },
        Focus::Menu(k) => match key {
            Arrow::Up if !menu_open => Some(Go::Menu(0)),
            Arrow::Up => match k {
                0 => None,
                5 if hidden5 => Some(Go::Menu(0)),
                k => Some(Go::Menu(k - 1)),
            },
            Arrow::Down if !menu_open => Some(Go::Button(EXIT)),
            Arrow::Down => match k {
                0..=3 => Some(Go::Menu(k + 1)),
                4 if !hidden5 => Some(Go::Menu(5)),
                4 => Some(Go::Button(EXIT)),
                5 if hidden5 => Some(Go::Menu(0)),
                5 => Some(Go::Button(EXIT)),
                _ => None,
            },
            Arrow::Left | Arrow::Right => (!menu_open).then_some(Go::Menu(0)),
        },
    }
}

/// The middle of the button a key goes to, by set: where the program
/// warps the pointer (go out at (1836, 994) in set 0, the back arrow at
/// (1851, 1010) in the others). `None` for a button the set lacks.
pub fn button_point(set: i32, id: u8) -> Option<(i32, i32)> {
    match (set, id) {
        (0, EXIT) => Some((1836, 994)),
        (1..=3, EXIT) => Some((1851, 1010)),
        (0..=2, LOOK) => Some((1668, 994)),
        (0 | 2, SLEEP) => Some((1500, 994)),
        (_, QUICK_SAVE) => Some((88, 1000)),
        (_, QUICK_LOAD) => Some((240, 1000)),
        _ => None,
    }
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
/// Width of the hint drawer's rectangle; longer lines wrap.
const HINT_WIDTH: i32 = 500;

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

    /// The picture of the game's own pointer for the element under the
    /// pointer (see [`cursor`]): by the verb of a thing, none otherwise.
    pub fn cursor_picture(hover: Option<i32>) -> u8 {
        match hover {
            Some(h) if h != BACK && h >= THING => cursor::picture_for_id(h - THING),
            _ => 0,
        }
    }

    /// Whether a click on this element plays the OK sound: go out or back,
    /// look and sleep or talk, in the button mode (quick save and quick
    /// load, the back arrow of look mode and the things play none).
    pub fn click_sound(&self, id: i32) -> bool {
        !self.look && (0..THING).contains(&id) && matches!(id as u8, EXIT | LOOK | SLEEP)
    }

    /// The cancel keys and the secondary click: the click they make.
    pub fn cancel(&self) -> Click {
        if self.plain {
            Click::Result(0)
        } else if self.look {
            Click::Look(false)
        } else {
            Click::Result(CANCELLED)
        }
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
        _font: Option<&Font>,
    ) -> Result<()> {
        let sheet = assets.image("search_p.akb")?;
        // The hint bar, and its line in the GDI font (size 20) from (725,
        // 1029).
        canvas.draw_part(&sheet, (0, 684, 584, 68), 658, 1005, false, 255);
        let line = hover
            .and_then(|h| self.hint(h))
            .and_then(|id| self.system.as_ref()?.get(0, id));
        if let Some(line) = line {
            gdi::draw_hint(canvas, line, HINT_TEXT.0, HINT_TEXT.1, 20, HINT_WIDTH);
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
    use Arrow::*;

    fn room(set: i32) -> Room {
        Room {
            buttons: Arc::new(Vec::new()),
            back: Arc::new(Vec::new()),
            spots: Vec::new(),
            set,
            system: None,
            look: false,
            plain: false,
        }
    }

    #[test]
    fn keys_from_nothing_go_to_the_menu_button() {
        for k in [Up, Down, Left, Right] {
            assert_eq!(arrow(Focus::None, k, true, false), Some(Go::Menu(0)));
            assert_eq!(arrow(Focus::None, k, false, false), Some(Go::Menu(0)));
        }
    }

    #[test]
    fn keys_on_the_buttons() {
        let f = |k, key| arrow(Focus::Button(k), key, true, false);
        assert_eq!(f(EXIT, Left), Some(Go::Button(LOOK)));
        assert_eq!(f(LOOK, Left), Some(Go::Button(SLEEP)));
        assert_eq!(f(SLEEP, Left), None);
        assert_eq!(f(QUICK_SAVE, Left), Some(Go::Button(QUICK_LOAD)));
        assert_eq!(f(EXIT, Right), None);
        assert_eq!(f(LOOK, Right), Some(Go::Button(EXIT)));
        assert_eq!(f(QUICK_SAVE, Right), Some(Go::Button(SLEEP)));
        assert_eq!(f(QUICK_LOAD, Right), Some(Go::Button(QUICK_SAVE)));
        assert_eq!(f(LOOK, Down), None);
        assert_eq!(f(LOOK, Up), Some(Go::Menu(5)));
        assert_eq!(
            arrow(Focus::Button(LOOK), Up, true, true),
            Some(Go::Menu(4))
        );
        assert_eq!(
            arrow(Focus::Button(LOOK), Up, false, false),
            Some(Go::Menu(0))
        );
    }

    #[test]
    fn keys_on_the_menu() {
        let f = |k, key| arrow(Focus::Menu(k), key, true, false);
        assert_eq!(f(0, Up), None);
        assert_eq!(f(3, Up), Some(Go::Menu(2)));
        assert_eq!(f(0, Down), Some(Go::Menu(1)));
        assert_eq!(f(3, Down), Some(Go::Menu(4)));
        assert_eq!(f(4, Down), Some(Go::Menu(5)));
        assert_eq!(f(5, Down), Some(Go::Button(EXIT)));
        assert_eq!(f(6, Down), None);
        assert_eq!(f(6, Up), Some(Go::Menu(5)));
        assert_eq!(f(2, Left), None);
        assert_eq!(f(2, Right), None);
        assert_eq!(
            arrow(Focus::Menu(4), Down, true, true),
            Some(Go::Button(EXIT))
        );
        let g = |key| arrow(Focus::Menu(0), key, false, false);
        assert_eq!(g(Down), Some(Go::Button(EXIT)));
        assert_eq!(g(Up), Some(Go::Menu(0)));
        assert_eq!(g(Left), Some(Go::Menu(0)));
        assert_eq!(g(Right), Some(Go::Menu(0)));
    }

    #[test]
    fn points_by_set() {
        assert_eq!(button_point(0, EXIT), Some((1836, 994)));
        assert_eq!(button_point(1, EXIT), Some((1851, 1010)));
        assert_eq!(button_point(2, LOOK), Some((1668, 994)));
        assert_eq!(button_point(0, SLEEP), Some((1500, 994)));
        assert_eq!(button_point(2, SLEEP), Some((1500, 994)));
        assert_eq!(button_point(1, SLEEP), None);
        assert_eq!(button_point(3, LOOK), None);
        assert_eq!(button_point(1, QUICK_SAVE), Some((88, 1000)));
        assert_eq!(button_point(1, QUICK_LOAD), Some((240, 1000)));
        assert_eq!(button_point(0, 5), None);
    }

    #[test]
    fn cancel_by_mode() {
        let mut r = room(2);
        assert_eq!(r.cancel(), Click::Result(0xffff));
        r.look = true;
        assert_eq!(r.cancel(), Click::Look(false));
        r.plain = true;
        assert_eq!(r.cancel(), Click::Result(0));
    }

    #[test]
    fn sounds_and_pointer_pictures() {
        let mut r = room(0);
        for id in [EXIT, LOOK, SLEEP] {
            assert!(r.click_sound(id as i32));
        }
        assert!(!r.click_sound(QUICK_SAVE as i32));
        assert!(!r.click_sound(QUICK_LOAD as i32));
        r.look = true;
        assert!(!r.click_sound(BACK));
        assert!(!r.click_sound(THING + 5));
        assert_eq!(Room::cursor_picture(Some(THING + 3)), 1);
        assert_eq!(Room::cursor_picture(Some(THING + 0x405)), 5);
        assert_eq!(Room::cursor_picture(Some(BACK)), 0);
        assert_eq!(Room::cursor_picture(Some(LOOK as i32)), 0);
        assert_eq!(Room::cursor_picture(None), 0);
    }

    #[test]
    fn names() {
        assert_eq!(area_name("42_X.akb"), "42.area");
        assert_eq!(area_name("cafe.akb"), "cafe.area");
    }
}
