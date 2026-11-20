//! The character picker of 同級生 Remake (`Game.36`): the fortune teller
//! and the confession ask which girl.
//!
//! Read from the executable (not compared under Wine yet): the screen is
//! dimmed with black at half opacity, then `charselect.area` places the
//! 14 cards (ids 0..13, five to a row from (18, 80), 337 x 313 apart),
//! the side buttons (256..259: settings, backlog, save, load; 260, 261:
//! quick save and load) and the back button (512, only for sub 0). The
//! cards come from `chara_list_<lang>.akb`: 316 x 305 cells, seven to a row,
//! a row per state (normal, hover, pressed, gray) and the second seven
//! 1220 lower. The buttons come from `chara_menu.akb`. Byte variable `i`
//! (`b[i]`) is 1 when card `i` can be picked. The call returns the card
//! from 1, or 15 for back (also the secondary click, for sub 0).
//! See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::area::{self, Area};

use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

pub const CARDS: usize = 14;
pub const SETTINGS: i32 = 256;
pub const BACKLOG: i32 = 257;
pub const SAVE: i32 = 258;
pub const LOAD: i32 = 259;
pub const QUICK_SAVE: i32 = 260;
pub const QUICK_LOAD: i32 = 261;
pub const BACK: i32 = 512;
/// What back returns.
pub const NONE: i32 = 15;

const LIST: &str = "chara_list.akb";
const MENU: &str = "chara_menu.akb";
const CARD: (u32, u32) = (316, 305);
const SIDE: u32 = 164;
/// Quick buttons: width, height, top of the first row.
const QUICK: (u32, u32, u32) = (91, 54, 656);
/// Back button: left, width, height.
const BACK_CELL: (u32, u32, u32) = (951, 295, 134);
const DIM: [u8; 4] = [0, 0, 0, 0x7f];
/// The click sound of a card, back and the side buttons. The executable
/// plays its system sound 4 for them (`ses_001.wav`, as the notice's).
pub const CLICK_SOUND: &str = "ses_001.wav";
pub const QUICK_SAVE_SOUND: &str = "ses_003.wav";
pub const QUICK_LOAD_SOUND: &str = "ses_004.wav";

#[derive(Debug, Clone)]
pub struct Picker {
    /// Sub 0 has the back button; sub 1 must pick.
    pub sub: i32,
    pub cards: [bool; CARDS],
    areas: Arc<Vec<Area>>,
}

impl Picker {
    pub fn load(assets: &Assets, sub: i32, bytes: &[u8]) -> Result<Self> {
        let areas = area::parse(&assets.data("charselect.area")?)?;
        let cards = std::array::from_fn(|i| bytes.get(i) == Some(&1));
        Ok(Self {
            sub,
            cards,
            areas: Arc::new(areas),
        })
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        self.areas
            .iter()
            .filter(|a| a.id != BACK || self.sub == 0)
            .find(|a| a.contains(x, y))
            .map(|a| a.id)
    }

    /// Whether a card can be picked; the buttons are left to the caller.
    pub fn enabled(&self, id: i32) -> bool {
        match usize::try_from(id) {
            Ok(i) if i < CARDS => self.cards[i],
            _ => true,
        }
    }

    pub fn click(&self, id: i32) -> Click {
        match id {
            i if (0..CARDS as i32).contains(&i) && self.enabled(i) => Click::Result(i + 1),
            BACK if self.sub == 0 => Click::Result(NONE),
            SETTINGS => Click::Settings,
            BACKLOG => Click::Backlog,
            SAVE => Click::Save,
            LOAD => Click::Load,
            QUICK_SAVE => Click::QuickSave,
            QUICK_LOAD => Click::QuickLoad,
            _ => Click::None,
        }
    }

    /// The sound a click on `id` plays, if any: none for a click that does
    /// nothing (a gray card, back on sub 1).
    pub fn sound(&self, id: i32) -> Option<&'static str> {
        match id {
            QUICK_SAVE => Some(QUICK_SAVE_SOUND),
            QUICK_LOAD => Some(QUICK_LOAD_SOUND),
            _ if matches!(self.click(id), Click::None) => None,
            _ => Some(CLICK_SOUND),
        }
    }

    /// The secondary click's result, if it leaves.
    pub fn cancel(&self) -> Option<i32> {
        (self.sub == 0).then_some(NONE)
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed, 3 gray) of an
    /// id.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        canvas.fill(0, 0, 1920, 1080, DIM);
        let list = assets.localized_image(LIST, language)?;
        let menu = assets.image(MENU)?;
        for a in self.areas.iter() {
            let s = state(a.id).min(3);
            let (sheet, part) = match a.id {
                i @ 0..=13 => {
                    let (cw, ch) = CARD;
                    let i = i as u32;
                    (&list, (i % 7 * cw, s * ch + i / 7 * 4 * ch, cw, ch))
                }
                i @ SETTINGS..=LOAD => {
                    let i = (i - SETTINGS) as u32;
                    (&menu, (i * SIDE, s * SIDE, SIDE, SIDE))
                }
                i @ (QUICK_SAVE | QUICK_LOAD) => {
                    let (qw, qh, top) = QUICK;
                    let x = (i - QUICK_SAVE) as u32 * qw;
                    (&menu, (x, top + s * qh, qw, qh))
                }
                BACK if self.sub == 0 => {
                    let (x, bw, bh) = BACK_CELL;
                    (&menu, (x, s * bh, bw, bh))
                }
                _ => continue,
            };
            canvas.draw_part(sheet, part, a.left, a.top, false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results() {
        let mut cards = [false; CARDS];
        cards[3] = true;
        let p = Picker {
            sub: 0,
            cards,
            areas: Arc::new(Vec::new()),
        };
        assert!(matches!(p.click(3), Click::Result(4)));
        assert!(matches!(p.click(2), Click::None));
        assert!(matches!(p.click(BACK), Click::Result(NONE)));
        assert_eq!(p.cancel(), Some(NONE));
        let p = Picker { sub: 1, ..p };
        assert!(matches!(p.click(BACK), Click::None));
        assert_eq!(p.cancel(), None);
    }

    #[test]
    fn sounds() {
        let mut cards = [false; CARDS];
        cards[3] = true;
        let p = Picker {
            sub: 0,
            cards,
            areas: Arc::new(Vec::new()),
        };
        assert_eq!(p.sound(3), Some(CLICK_SOUND));
        assert_eq!(p.sound(2), None);
        assert_eq!(p.sound(BACK), Some(CLICK_SOUND));
        assert_eq!(p.sound(SAVE), Some(CLICK_SOUND));
        assert_eq!(p.sound(QUICK_SAVE), Some("ses_003.wav"));
        assert_eq!(p.sound(QUICK_LOAD), Some("ses_004.wav"));
        assert_eq!(p.sound(99), None);
        let p = Picker { sub: 1, ..p };
        assert_eq!(p.sound(BACK), None);
    }
}
