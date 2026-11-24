//! The map of the apartment (`Game.43`): the six rooms as cards on the
//! first and second floor, with the engine's menu buttons under them.
//!
//! The script sets bytes 0..5 first (a room that is 0 shows as locked) and
//! calls with 1 when the player may go back to the action menu and 0 when
//! the choice is forced. The call returns the room's number 1..6, or 100
//! for the back button. A room's byte 810 + n tells whether it was looked
//! at already; a room not looked at shows a plain card on hover.

use silky_format::area::{self, Area};

use super::notebook::{Notebook, BOOK_FIRST, BOOK_VARS};
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::text::TextRenderer;
use crate::Result;

/// Rooms: 101, 102, 103, 201, 202, 203.
pub const PLACES: usize = 6;
/// Byte variables of the rooms' state and the first of the "looked at"
/// bytes.
pub const SHOWN_VAR: usize = 0;
pub const VISITED_VAR: usize = 810;

/// Area ids of `mapselect.area`: rooms 0..5, back, then the menu buttons.
pub const BACK: i32 = 256;
pub const MENU: i32 = 3840;
const MENU_BUTTONS: i32 = 8;
/// The menu button that opens the notebook.
const MENU_NOTEBOOK: i32 = MENU + 7;
/// The answer of the back button.
pub const BACK_RESULT: i32 = 100;

const CARD: (u32, u32) = (166, 298);
/// Where the cards are drawn (top left of each room's area).
const CARD_POS: [(i32, i32); PLACES] = [
    (131, 259),
    (301, 259),
    (471, 259),
    (689, 259),
    (859, 259),
    (1029, 259),
];
/// Row of the locked card in `move01_p.akb`, in pixels.
const LOCKED_Y: u32 = 894;

/// The floors' frames in `move_p.akb`: (source rectangle, place).
const FLOORS: [((u32, u32, u32, u32), (i32, i32)); 2] = [
    ((0, 554, 604, 421), (50, 156)),
    ((604, 554, 539, 421), (672, 156)),
];
/// The two bars of the information line and the title picture.
const BAR_LEFT: ((u32, u32, u32, u32), (i32, i32)) = ((548, 30, 545, 30), (0, 687));
const BAR_RIGHT: ((u32, u32, u32, u32), (i32, i32)) = ((548, 0, 513, 30), (602, 687));
const TITLE: ((u32, u32, u32, u32), (i32, i32)) = ((1143, 554, 419, 100), (860, 29));
/// The back button: size, source x and first source row, row step.
const BACK_SIZE: (u32, u32) = (78, 57);
const BACK_SRC: (u32, u32) = (1143, 854);
const BACK_POS: (i32, i32) = (1172, 660);
/// Menu buttons: size, step on the screen, first source x, step in the
/// sheet, top, row height.
const MENU_SIZE: (u32, u32) = (52, 38);
const MENU_POS: (i32, i32, i32) = (40, 60, 679);
const MENU_SRC_X: u32 = 80;
/// The information line's text: left, baseline.
const INFO_POS: (f32, f32) = (716.0, 708.0);
const INFO_SIZE: f32 = 16.0;

const ROOM_INFO: [&str; PLACES] = [
    "１０１号室へ移動する",
    "１０２号室へ移動する",
    "１０３号室へ移動する",
    "２０１号室へ移動する",
    "２０２号室へ移動する",
    "２０３号室へ移動する",
];
const BACK_INFO: &str = "行動選択に戻る";
pub(super) const MENU_INFO: [&str; 8] = [
    "システム設定を開きます",
    "サウンド設定を開きます",
    "履歴画面を開きます",
    "セーブ画面を開きます",
    "ロード画面を開きます",
    "クイックセーブします",
    "クイックロードします",
    "捜査手帳を開きます",
];
/// The sound of a press.
pub(super) const OK_SOUND: &str = "ses_001.wav";

#[derive(Debug, Clone)]
pub struct Map {
    /// Whether the back button is there.
    back: bool,
    shown: [bool; PLACES],
    visited: [bool; PLACES],
    areas: Vec<Area>,
    /// The right click hides the buttons to show the picture under them.
    hidden: bool,
    sound: Option<String>,
    /// The notebook, once its menu button is pressed.
    book: Option<Box<Notebook>>,
    book_vars: Vec<u8>,
    book_areas: Vec<Area>,
}

impl Map {
    /// Opens the map. `back` is the call's argument (1: back button);
    /// `var` reads a byte variable.
    pub fn new(assets: &Assets, back: bool, var: impl Fn(usize) -> u8) -> Result<Self> {
        let areas = area::parse(&assets.data("mapselect.area")?)?;
        let book_areas = area::parse(&assets.data("notebook.area")?)?;
        let mut shown = [false; PLACES];
        let mut visited = [false; PLACES];
        for i in 0..PLACES {
            shown[i] = var(SHOWN_VAR + i) != 0;
            visited[i] = var(VISITED_VAR + i) != 0;
        }
        let book_vars = (0..BOOK_VARS).map(|i| var(BOOK_FIRST + i)).collect();
        Ok(Self {
            back,
            shown,
            visited,
            areas,
            hidden: false,
            sound: None,
            book: None,
            book_vars,
            book_areas,
        })
    }

    fn room(id: i32) -> Option<usize> {
        (0..PLACES as i32).contains(&id).then_some(id as usize)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if let Some(b) = &self.book {
            return b.hit(x, y);
        }
        if self.hidden {
            return None;
        }
        self.areas.iter().find(|a| a.contains(x, y)).map(|a| a.id)
    }

    pub fn enabled(&self, id: i32) -> bool {
        if let Some(b) = &self.book {
            return b.enabled(id);
        }
        match id {
            BACK => self.back,
            _ => Self::room(id).map_or(true, |i| self.shown[i]),
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if let Some(b) = &mut self.book {
            if b.click(id) == Click::Close {
                self.book = None;
            }
            return Click::None;
        }
        if !self.enabled(id) {
            return Click::None;
        }
        if let Some(i) = Self::room(id) {
            self.sound = Some(OK_SOUND.into());
            return Click::Result(i as i32 + 1);
        }
        match id {
            BACK => {
                self.sound = Some(OK_SOUND.into());
                Click::Result(BACK_RESULT)
            }
            MENU_NOTEBOOK => {
                self.sound = Some(OK_SOUND.into());
                self.book = Some(Box::new(Notebook::with_areas(
                    self.book_areas.clone(),
                    self.book_vars.clone(),
                    false,
                )));
                Click::None
            }
            // The other menu buttons open the engine's own screens, which
            // the frontend's bar handles.
            _ => Click::None,
        }
    }

    pub fn cancel(&mut self) -> Click {
        if let Some(b) = &mut self.book {
            if b.cancel() == Click::Close {
                self.book = None;
            }
            return Click::None;
        }
        if self.back {
            return Click::Result(BACK_RESULT);
        }
        // With no way back the right click only hides the buttons.
        self.hidden = !self.hidden;
        Click::None
    }

    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        self.book
            .as_mut()
            .map(|b| b.take_bytes())
            .unwrap_or_default()
    }

    pub fn take_sound(&mut self) -> Option<String> {
        match &mut self.book {
            Some(b) => b.take_sound().or_else(|| self.sound.take()),
            None => self.sound.take(),
        }
    }

    /// The first room not looked at yet, else the first that can be
    /// entered, else the back button.
    pub fn auto_pick(&self) -> (Option<i32>, &'static str) {
        if let Some(b) = &self.book {
            return b.auto_pick();
        }
        let open = (0..PLACES).filter(|&i| self.shown[i]);
        let pick = open
            .clone()
            .find(|&i| !self.visited[i])
            .or_else(|| open.clone().next());
        match pick {
            Some(i) => (Some(i as i32), "map"),
            None if self.back => (Some(BACK), "map"),
            None => (None, "map"),
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        if let Some(b) = &self.book {
            return b.draw(canvas, assets, look);
        }
        if self.hidden {
            return Ok(());
        }
        let sheet = assets.image("move_p.akb")?;
        let cards = assets.image("move01_p.akb")?;
        for (src, (x, y)) in FLOORS {
            canvas.draw_part(&sheet, src, x, y, false, 255);
        }
        let state = |id: i32| match (look.pressed == Some(id), look.hover == Some(id)) {
            (true, _) => 2,
            (_, true) => 1,
            _ => 0,
        };
        for i in 0..PLACES {
            let (x, y) = CARD_POS[i];
            let sx = CARD.0 * i as u32;
            let sy = if self.shown[i] {
                let mut s = state(i as i32);
                // Rooms not looked at yet have their own hover and press.
                if !self.visited[i] && s > 0 {
                    s += 3;
                }
                CARD.1 * s
            } else {
                LOCKED_Y
            };
            canvas.draw_part(&cards, (sx, sy, CARD.0, CARD.1), x, y, false, 255);
        }
        for (src, (x, y)) in [BAR_LEFT, BAR_RIGHT, TITLE] {
            canvas.draw_part(&sheet, src, x, y, false, 255);
        }
        if self.back {
            let s = state(BACK);
            canvas.draw_part(
                &sheet,
                (
                    BACK_SRC.0,
                    BACK_SRC.1 + BACK_SIZE.1 * s as u32,
                    BACK_SIZE.0,
                    BACK_SIZE.1,
                ),
                BACK_POS.0,
                BACK_POS.1,
                false,
                255,
            );
        }
        for n in 0..MENU_BUTTONS {
            let s = state(MENU + n) as u32;
            canvas.draw_part(
                &sheet,
                (
                    MENU_SRC_X + MENU_SIZE.0 * n as u32,
                    MENU_SIZE.1 * s,
                    MENU_SIZE.0,
                    MENU_SIZE.1,
                ),
                MENU_POS.0 + MENU_POS.1 * n,
                MENU_POS.2,
                false,
                255,
            );
        }
        if let (Some(tr), Some(info)) = (look.text, self.info(look.hover)) {
            draw_info(tr, canvas, info);
        }
        Ok(())
    }

    /// The information line for the element under the pointer.
    fn info(&self, hover: Option<i32>) -> Option<&'static str> {
        let id = hover?;
        if let Some(i) = Self::room(id) {
            return self.shown[i].then_some(ROOM_INFO[i]);
        }
        match id {
            BACK => self.back.then_some(BACK_INFO),
            _ if (MENU..MENU + MENU_BUTTONS).contains(&id) => Some(MENU_INFO[(id - MENU) as usize]),
            _ => None,
        }
    }
}

/// Draws a line of the information bar (white, 16 pixels).
pub(super) fn draw_info(tr: &TextRenderer, canvas: &mut Canvas, line: &str) {
    tr.draw_plain(
        canvas,
        line,
        INFO_POS.0,
        INFO_POS.1,
        INFO_SIZE,
        [255, 255, 255, 255],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(shown: [bool; PLACES], visited: [bool; PLACES], back: bool) -> Map {
        let areas = (0..PLACES as i32)
            .map(|i| Area {
                id: i,
                left: 131 + 170 * i,
                top: 259,
                right: 297 + 170 * i,
                bottom: 557,
            })
            .collect();
        Map {
            back,
            shown,
            visited,
            areas,
            hidden: false,
            sound: None,
            book: None,
            book_vars: vec![0; BOOK_VARS],
            book_areas: Vec::new(),
        }
    }

    #[test]
    fn places_answer_their_number() {
        let mut m = map([true; PLACES], [false; PLACES], true);
        assert_eq!(m.click(2), Click::Result(3));
        assert_eq!(m.take_sound().as_deref(), Some(OK_SOUND));
        assert_eq!(m.click(BACK), Click::Result(BACK_RESULT));
    }

    #[test]
    fn locked_rooms_and_missing_back_do_nothing() {
        let mut shown = [true; PLACES];
        shown[5] = false;
        let mut m = map(shown, [false; PLACES], false);
        assert_eq!(m.click(5), Click::None);
        assert_eq!(m.click(BACK), Click::None);
        assert!(!m.enabled(5) && !m.enabled(BACK));
    }

    #[test]
    fn auto_pick_takes_the_first_room_not_looked_at() {
        let mut visited = [false; PLACES];
        visited[0] = true;
        let m = map([true; PLACES], visited, true);
        assert_eq!(m.auto_pick().0, Some(1));
        let m = map([true; PLACES], [true; PLACES], true);
        assert_eq!(m.auto_pick().0, Some(0));
    }

    #[test]
    fn right_click_goes_back_or_hides() {
        let mut m = map([true; PLACES], [false; PLACES], true);
        assert_eq!(m.cancel(), Click::Result(BACK_RESULT));
        let mut m = map([true; PLACES], [false; PLACES], false);
        assert_eq!(m.cancel(), Click::None);
        assert_eq!(m.hit(140, 300), None);
        m.cancel();
        assert_eq!(m.hit(140, 300), Some(0));
    }
}
