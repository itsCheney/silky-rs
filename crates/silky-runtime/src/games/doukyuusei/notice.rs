//! The modal notice of 同級生 Remake (`Game.30(kind)`): a small dialog
//! with one Close button over the screen as it is, which the script
//! waits on.
//!
//! Read from the executable and compared with a Wine screenshot. The
//! screen is darkened (black at half alpha), then the box and the
//! text line are laid from the sheet. Kinds 0 to 2 use `dialog2_p_<lang>.akb`
//! (a box of 880 x 250 at (520, 415); the line of the kind is the 106
//! rows from row 250 + 106 kind: the name that is not available, the flag
//! list feature, the easy mode); kind 3 uses `dialog3_p_<lang>.akb` (a box
//! of 880 x 310 at (520, 355) and the line from row 310). The Close
//! button (222 x 73 at (849, 560), a row of the sheet's right part per
//! state: normal, hover, pressed) plays the OK sound and closes; Escape
//! and the secondary click close with the cancel sound. The call takes the kind
//! and returns nothing; the hero's room shows kind 1 before it sets the
//! flag list's byte. See `docs/games/doukyuusei.md`.

use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

/// The id of the Close button.
pub const CLOSE: i32 = 0;

/// The kind that uses the second sheet.
const FLAG_KIND: i32 = 3;
const SHEET: &str = "dialog2_p.akb";
const FLAG_SHEET: &str = "dialog3_p.akb";
const DIM: [u8; 4] = [0, 0, 0, 0x7f];
/// The Close button (left, top, width, height) and where its states
/// start in the sheet.
const CLOSE_AT: (i32, i32, u32, u32) = (849, 560, 222, 73);
const CLOSE_SHEET_X: u32 = 880;
/// The text line's width and left, and its height and row 0.
const TEXT_AT_X: i32 = 558;
const TEXT_W: u32 = 805;
const BOX_W: u32 = 880;
const BOX_X: i32 = 520;

/// A kind's picture parts: the sheet, the box (size and place), and the
/// text line (rows in the sheet and place).
struct Layout {
    sheet: &'static str,
    box_h: u32,
    box_y: i32,
    text_row: u32,
    text_h: u32,
    text_y: i32,
}

#[derive(Debug, Clone)]
pub struct Notice {
    kind: i32,
}

impl Notice {
    /// `None` for a kind the original has no picture for.
    pub fn new(kind: i32) -> Option<Self> {
        (0..=FLAG_KIND).contains(&kind).then_some(Self { kind })
    }

    fn layout(&self) -> Layout {
        if self.kind == FLAG_KIND {
            Layout {
                sheet: FLAG_SHEET,
                box_h: 310,
                box_y: 355,
                text_row: 310,
                text_h: 166,
                text_y: 384,
            }
        } else {
            Layout {
                sheet: SHEET,
                box_h: 250,
                box_y: 415,
                text_row: 250 + 106 * self.kind as u32,
                text_h: 106,
                text_y: 444,
            }
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let (l, t, w, h) = CLOSE_AT;
        (x >= l && y >= t && x < l + w as i32 && y < t + h as i32).then_some(CLOSE)
    }

    pub fn click(&mut self, id: i32) -> Click {
        if id == CLOSE {
            Click::Close
        } else {
            Click::None
        }
    }

    /// Draws the dimming, the box, the line and the button; `state` is
    /// the button's row (0 normal, 1 hover, 2 pressed).
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let l = self.layout();
        let sheet = assets.localized_image(l.sheet, language)?;
        canvas.fill(0, 0, canvas.width, canvas.height, DIM);
        canvas.draw_part(&sheet, (0, 0, BOX_W, l.box_h), BOX_X, l.box_y, false, 255);
        canvas.draw_part(
            &sheet,
            (0, l.text_row, TEXT_W, l.text_h),
            TEXT_AT_X,
            l.text_y,
            false,
            255,
        );
        let (x, y, w, h) = CLOSE_AT;
        let row = state(CLOSE).min(2);
        canvas.draw_part(&sheet, (CLOSE_SHEET_X, h * row, w, h), x, y, false, 255);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert!(Notice::new(-1).is_none());
        assert!(Notice::new(4).is_none());
        let rows: Vec<u32> = (0..3)
            .map(|k| Notice::new(k).unwrap().layout().text_row)
            .collect();
        assert_eq!(rows, [250, 356, 462]);
        let flag = Notice::new(3).unwrap().layout();
        assert_eq!(
            (flag.sheet, flag.box_y, flag.text_row),
            (FLAG_SHEET, 355, 310)
        );
        assert_eq!(Notice::new(1).unwrap().layout().sheet, SHEET);
    }

    #[test]
    fn close_button() {
        let mut n = Notice::new(1).unwrap();
        assert_eq!(n.hit(848, 600), None);
        assert_eq!(n.hit(849, 560), Some(CLOSE));
        assert_eq!(n.hit(1070, 632), Some(CLOSE));
        assert_eq!(n.hit(1071, 632), None);
        assert_eq!(n.hit(960, 633), None);
        assert_eq!(n.click(CLOSE), Click::Close);
        assert_eq!(n.click(7), Click::None);
    }
}
