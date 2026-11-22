//! The mail forward screen of 景の海のアペイリア (`Game.1016`): a modal
//! list of the mails the player may send back, over the DNA PC
//! background. See `docs/games/apeiria.md`.

use crate::assets::Assets;
use crate::canvas::Canvas;

/// Sheet with the rows, check boxes and forward button.
pub const SHEET: &str = "dnapc_mailparts.akb";
/// Hit areas: ids 0 to 10 are the row slots, 20 the forward button.
pub const AREAS: &str = "mailselect.area";
pub const FORWARD_AREA: i32 = 20;

/// Byte variable that makes candidate `k` available.
pub const CANDIDATES: [usize; 11] = [
    1001, 1002, 1003, 1100, 1100, 1101, 1101, 1102, 1102, 1102, 1102,
];
/// Mode 1 offers only the first candidates.
const MODE1_COUNT: usize = 3;

const ROW_AT: (i32, i32) = (353, 95);
const ROW_SIZE: (u32, u32) = (501, 32);
const ROW_STEP: i32 = 32;
/// The sheet's checked variant of a row sits right of the plain one.
const ROW_CHECKED_X: u32 = 501;
const BOX_AT: (i32, i32) = (860, 100);
const BOX_SRC: (u32, u32) = (1062, 0);
const BOX_SIZE: (u32, u32) = (24, 19);
const BUTTON_AT: (i32, i32) = (929, 389);
const BUTTON_SRC: (u32, u32) = (1002, 0);
const BUTTON_OFF_SRC: (u32, u32) = (1002, 180);
const BUTTON_SIZE: u32 = 60;

/// State of the screen, kept with the scene so saves keep it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MailSelect {
    /// Set up by sub 0: 0 offers every mail, 1 the first three.
    pub mode: i32,
    /// The screen is up.
    pub shown: bool,
    /// Candidate indexes listed, top down.
    pub rows: Vec<usize>,
    /// Candidate checked.
    pub checked: Option<usize>,
    /// The forward button is active.
    pub forward: bool,
    /// Bytes 1 and 2: the mails whose click only checks the row.
    pub pair: [u8; 2],
}

impl MailSelect {
    /// Opens the screen over the byte variables `bytes` (byte `i`).
    pub fn open(&mut self, bytes: impl Fn(usize) -> u8) {
        let n = if self.mode == 1 {
            MODE1_COUNT
        } else {
            CANDIDATES.len()
        };
        self.rows = (0..n).filter(|&k| bytes(CANDIDATES[k]) != 0).collect();
        self.checked = match bytes(0) {
            255 => None,
            b => self.rows.iter().copied().find(|&k| k == b as usize),
        };
        self.forward = self.checked.is_some();
        self.pair = [bytes(1), bytes(2)];
        self.shown = true;
    }

    /// Clicks listed row `v`: the result when that ends the screen.
    pub fn click_row(&mut self, v: usize) -> Option<i32> {
        let k = *self.rows.get(v)?;
        if self.checked == Some(k) {
            self.checked = None;
            self.forward = false;
            return None;
        }
        let n = (k + 1) as u8;
        if self.mode == 1 || self.pair.contains(&n) {
            self.checked = Some(k);
            self.forward = true;
            None
        } else {
            Some(k as i32 + 1)
        }
    }

    /// Presses the forward button: the result when it ends the screen.
    pub fn click_forward(&mut self) -> Option<i32> {
        self.checked.filter(|_| self.forward).map(|k| k as i32 + 1)
    }

    /// Clicks listed row `v`, then forward if it became active; what a
    /// selection answer stands for.
    pub fn answer(&mut self, v: usize) -> i32 {
        self.click_row(v)
            .or_else(|| self.click_forward())
            .unwrap_or(-1)
    }

    /// Labels of the listed rows for a selection.
    pub fn labels(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|&k| CANDIDATES[k].to_string())
            .collect()
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets) -> crate::Result<()> {
        if !self.shown {
            return Ok(());
        }
        let sheet = assets.image(SHEET)?;
        for (v, &k) in self.rows.iter().enumerate() {
            let y = ROW_STEP * v as i32;
            let on = self.checked == Some(k);
            let x = if on { ROW_CHECKED_X } else { 0 };
            let src = (x, ROW_STEP as u32 * k as u32, ROW_SIZE.0, ROW_SIZE.1);
            canvas.draw_part(&sheet, src, ROW_AT.0, ROW_AT.1 + y, false, 255);
            let (bx, by) = BOX_SRC;
            let src = (
                bx,
                by + if on { BOX_SIZE.1 } else { 0 },
                BOX_SIZE.0,
                BOX_SIZE.1,
            );
            canvas.draw_part(&sheet, src, BOX_AT.0, BOX_AT.1 + y, false, 255);
        }
        let (sx, sy) = if self.forward {
            BUTTON_SRC
        } else {
            BUTTON_OFF_SRC
        };
        let src = (sx, sy, BUTTON_SIZE, BUTTON_SIZE);
        canvas.draw_part(&sheet, src, BUTTON_AT.0, BUTTON_AT.1, false, 255);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(set: &[(usize, u8)]) -> impl Fn(usize) -> u8 + '_ {
        move |i| set.iter().find(|s| s.0 == i).map_or(0, |s| s.1)
    }

    #[test]
    fn lists_set_rows_and_forwards() {
        let b = [(0, 255), (1, 3), (1003, 1), (1100, 1)];
        let mut s = MailSelect::default();
        s.open(bytes(&b));
        assert_eq!(s.rows, vec![2, 3, 4]);
        assert_eq!(s.checked, None);
        assert_eq!(s.click_row(0), None);
        assert!(s.forward);
        assert_eq!(s.click_forward(), Some(3));
        assert_eq!(s.click_row(0), None);
        assert!(!s.forward);
        // Not one of the pair: ends at once.
        assert_eq!(s.click_row(1), Some(4));
        assert_eq!(s.answer(0), 3);
    }

    #[test]
    fn mode_one_checks_and_lists_three() {
        let b = [(0, 255), (1002, 1), (1100, 1)];
        let mut s = MailSelect {
            mode: 1,
            ..Default::default()
        };
        s.open(bytes(&b));
        assert_eq!(s.rows, vec![1]);
        assert_eq!(s.click_row(0), None);
        assert_eq!(s.click_forward(), Some(2));
    }
}
