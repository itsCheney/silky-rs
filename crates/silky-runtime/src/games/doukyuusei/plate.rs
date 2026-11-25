//! The status plate of 同級生 Remake (the program's `CStatusPlate`): the
//! day on a calendar leaf, the time and the money along the top of the
//! room, the walking maps and the selections made there.
//!
//! Read from the executable: four pieces of `cal_p_<lang>.akb`, each
//! drawn at its own place.
//!
//! - The leaf (265 x 177 at (6, 0)): gray, red on a Sunday, blue on a
//!   Saturday (from (0, 592), (0, 238), (0, 415)); the two digits of the
//!   day (63 x 70 cells, a row per colour) at (69, 43) and (134, 43); the
//!   weekday (50 x 28 cells from the top) at (108, 135).
//! - The clock (294 x 71 at (241, 13)): `HH:MM` from x 92, y 18 in the
//!   small digits (26 x 33 from (297, 380), the colon 16 wide from (281,
//!   380)), on a 12-hour dial (noon shows 12), and the am or pm diamond (62
//!   x 62 from (559, 238) or (559, 300)) at (228, 1).
//! - The money (294 x 71 at (541, 13)): the digits from the right, the
//!   last one at x 231, 26 apart and 12 more before every fourth; a comma
//!   (16 x 33 from (265, 380)) at x 167 from 1,000 yen.
//! - The game mode diamond (77 x 79 at (841, 9)): C, or E (from (265, 413)
//!   or (342, 413)).
//!
//! The values are the scripts' globals: weekday 4 (0 for Sunday), date 7
//! (month × 100 + day), money 12 (in hundreds of yen) and time 19 (hours
//! × 100 + minutes).

use serde::{Deserialize, Serialize};
use silky_vm::Vm;

use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

const SHEET: &str = "cal_p.akb";

/// The game mode setting (`/Config/GameMode`) as the program keeps it: 0 is
/// the classic mode, 1 the easy mode (the default, and what a reset of the
/// settings gives). It is read each time something is drawn, so a change in
/// the settings shows at once, in a game already begun too.
///
/// What the easy mode adds: the E diamond on the status plate (C in the
/// classic mode), the arrows on the selection bars that raise or lower a
/// girl's regard, the faces on the map jump page, the place labels on the
/// walking maps, and the flag list button of the menu, which the classic
/// mode hides.
pub const fn game_mode(classic: bool) -> i32 {
    if classic {
        0
    } else {
        1
    }
}

/// Globals the plate shows.
pub(crate) const WEEKDAY: usize = 4;
pub(crate) const DATE: usize = 7;
pub(crate) const MONEY: usize = 12;
pub(crate) const TIME: usize = 19;

const LEAF_POS: (i32, i32) = (6, 0);
const LEAF: (u32, u32) = (265, 177);
const CLOCK_POS: (i32, i32) = (241, 13);
const PURSE_POS: (i32, i32) = (541, 13);
const BAR: (u32, u32) = (294, 71);
const MODE_POS: (i32, i32) = (841, 9);

/// Small digits: `,` and `:` 16 wide, then 0 to 9 26 wide.
const SMALL_Y: u32 = 380;
const SMALL_H: u32 = 33;
const SMALL_DIGIT_X: u32 = 297;
const SMALL_W: u32 = 26;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plate {
    /// 0 Sunday to 6 Saturday.
    pub weekday: i32,
    /// Month × 100 + day.
    pub date: i32,
    /// Hours × 100 + minutes.
    pub time: i32,
    /// In hundreds of yen.
    pub money: i32,
    /// The game mode setting (see [`game_mode`]): 1 shows E, others C.
    pub mode: i32,
}

impl Plate {
    pub fn read(vm: &Vm, mode: i32) -> Self {
        let g = |i| vm.var_g(i).int();
        Self {
            weekday: g(WEEKDAY),
            date: g(DATE),
            time: g(TIME),
            money: g(MONEY),
            mode,
        }
    }

    /// `alpha` is how opaque it is (the room's slide fades it).
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        alpha: u8,
    ) -> Result<()> {
        let sheet = assets.localized_image(SHEET, language)?;
        let mut part = |sx: u32, sy: u32, w: u32, h: u32, x: i32, y: i32| {
            canvas.draw_part(&sheet, (sx, sy, w, h), x, y, false, alpha);
        };

        // The leaf.
        let (leaf_y, digit_y) = match self.weekday {
            0 => (238, 98),
            6 => (415, 168),
            _ => (592, 28),
        };
        let (lx, ly) = LEAF_POS;
        part(0, leaf_y, LEAF.0, LEAF.1, lx, ly);
        let day = (self.date % 100).max(0) as u32;
        part(day / 10 * 63, digit_y, 63, 70, lx + 69, ly + 43);
        part(day % 10 * 63, digit_y, 63, 70, lx + 134, ly + 43);
        part(
            self.weekday.clamp(0, 6) as u32 * 50,
            0,
            50,
            28,
            lx + 108,
            ly + 135,
        );

        // The clock.
        let (cx, cy) = CLOCK_POS;
        part(265, 238, BAR.0, BAR.1, cx, cy);
        let hours = self.time / 100;
        let shown = if hours == 12 {
            12
        } else {
            hours.rem_euclid(12)
        };
        let text = format!("{:02}:{:02}", shown, (self.time % 100).max(0));
        let mut x = cx + 92;
        for ch in text.chars() {
            if ch == ':' {
                part(281, SMALL_Y, 16, SMALL_H, x, cy + 18);
                x += 16;
            } else {
                let d = ch.to_digit(10).unwrap_or(0);
                part(
                    SMALL_DIGIT_X + SMALL_W * d,
                    SMALL_Y,
                    SMALL_W,
                    SMALL_H,
                    x,
                    cy + 18,
                );
                x += SMALL_W as i32;
            }
        }
        let am = self.time / 1200 == 0;
        part(559, if am { 238 } else { 300 }, 62, 62, cx + 228, cy + 1);

        // The money.
        let (px, py) = PURSE_POS;
        part(265, 309, BAR.0, BAR.1, px, py);
        let yen = self.money.max(0) as u64 * 100;
        for (x, d) in money_digits(yen) {
            part(
                SMALL_DIGIT_X + SMALL_W * d,
                SMALL_Y,
                SMALL_W,
                SMALL_H,
                px + x,
                py + 18,
            );
        }
        if yen >= 1000 {
            part(265, SMALL_Y, 16, SMALL_H, px + 167, py + 18);
        }

        // The game mode.
        let mx = if self.mode == 1 { 342 } else { 265 };
        part(mx, 413, 77, 79, MODE_POS.0, MODE_POS.1);
        Ok(())
    }
}

/// Where each digit of the money goes, from the last one: 26 apart from
/// x 231, and 12 more before every fourth.
fn money_digits(yen: u64) -> Vec<(i32, u32)> {
    let mut out = Vec::new();
    let (mut x, mut rest) = (231, yen);
    for i in 1.. {
        if i % 4 == 0 {
            x -= 12;
        }
        out.push((x, (rest % 10) as u32));
        rest /= 10;
        x -= 26;
        if rest == 0 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_mode_values() {
        // Easy is 1 (the E diamond), classic 0 (the C diamond).
        assert_eq!(game_mode(false), 1);
        assert_eq!(game_mode(true), 0);
    }

    #[test]
    fn money() {
        // 50,000 yen under Wine.
        let xs: Vec<i32> = money_digits(50_000).iter().map(|d| d.0).collect();
        assert_eq!(xs, [231, 205, 179, 141, 115]);
        assert_eq!(money_digits(0), [(231, 0)]);
    }
}
