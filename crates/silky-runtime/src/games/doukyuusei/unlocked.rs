//! The "comment unlocked" pop-up of 同級生 Remake (`Game.37(n)`): the
//! picture of one message of the comments page (see [`super::comment`])
//! put up over the screen as it is, which the script waits on.
//!
//! Read from the executable; not compared with a Wine screenshot yet. The
//! value is the number of the picture, `comment0n.akb` (the same number
//! as in the comments page's file table, not the thumbnail's place), and
//! the scripts that call it (the three places that open a comment: SECRET
//! with 1, GENBA with 2, BABA with 4) set the comment's byte variable
//! themselves, before or after the call: the pop-up writes no variable.
//!
//! The screen is darkened (black at half alpha) and the 1599 x 1068
//! picture is laid at (154, 0), then the return button from the right
//! part of `comment_p.akb` (138 x 140 cells at x 848, a row per state:
//! normal, hover, pressed, gray) at (1642, 928). Only that button is a hot
//! spot (`comment.area`). It plays the OK sound and closes, as does Enter
//! while the pointer is on it; the secondary click and Escape (also
//! numpad 0 and Delete in the original) play the cancel sound and close.
//! See `docs/games/doukyuusei.md`.

use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

/// The id of the return button.
pub const BACK: i32 = 0;

/// How many pictures there are (`comment00.akb` to `comment07.akb`).
pub const PICTURES: i32 = 8;

const DIM: [u8; 4] = [0, 0, 0, 0x7f];
const PICTURE_AT: (i32, i32) = (154, 0);
const PICTURE_SIZE: (u32, u32) = (1599, 1068);
/// The return button (left, top, width, height) and where its states
/// start in the sheet.
const BUTTON_AT: (i32, i32, u32, u32) = (1642, 928, 138, 140);
const BUTTON_SHEET_X: u32 = 848;

#[derive(Debug, Clone)]
pub struct Unlocked {
    picture: i32,
}

impl Unlocked {
    /// `None` for a number the program has no picture for.
    pub fn new(picture: i32) -> Option<Self> {
        (0..PICTURES).contains(&picture).then_some(Self { picture })
    }

    /// The picture's file, before the language is added.
    pub fn file(&self) -> String {
        format!("comment{:02}.akb", self.picture)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let (l, t, w, h) = BUTTON_AT;
        (x >= l && y >= t && x < l + w as i32 && y < t + h as i32).then_some(BACK)
    }

    pub fn click(&mut self, id: i32) -> Click {
        if id == BACK {
            Click::Close
        } else {
            Click::None
        }
    }

    /// `state` gives the button's row (0 normal, 1 hover, 2 pressed).
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let picture = assets.localized_image(&self.file(), language)?;
        let sheet = assets.localized_image("comment_p.akb", language)?;
        canvas.fill(0, 0, canvas.width, canvas.height, DIM);
        let (w, h) = PICTURE_SIZE;
        canvas.draw_part(
            &picture,
            (0, 0, w, h),
            PICTURE_AT.0,
            PICTURE_AT.1,
            false,
            255,
        );
        let (x, y, w, h) = BUTTON_AT;
        let row = state(BACK).min(2);
        canvas.draw_part(&sheet, (BUTTON_SHEET_X, h * row, w, h), x, y, false, 255);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures() {
        assert!(Unlocked::new(-1).is_none());
        assert!(Unlocked::new(8).is_none());
        assert_eq!(Unlocked::new(4).unwrap().file(), "comment04.akb");
    }

    #[test]
    fn button() {
        let mut u = Unlocked::new(1).unwrap();
        assert_eq!(u.hit(1641, 1000), None);
        assert_eq!(u.hit(1642, 928), Some(BACK));
        assert_eq!(u.hit(1779, 1067), Some(BACK));
        assert_eq!(u.hit(1780, 1000), None);
        assert_eq!(u.hit(1700, 1068), None);
        assert_eq!(u.click(BACK), Click::Close);
        assert_eq!(u.click(3), Click::None);
    }
}
