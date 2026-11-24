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
//! them in over twelve passes (and back in on closing, see [`MenuAnim`]),
//! taking no input meanwhile. Save and load open
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

/// Passes of the slide at normal speed.
const SLIDE_PASSES: u32 = 12;
/// Length of a pass in milliseconds' denominator: 60 passes a second.
const PASSES_PER_SECOND: u64 = 60;

/// Passes of the slide for an effect scale (1, a half, or none at all),
/// at least one.
pub fn slide_passes(scale: f32) -> u32 {
    ((SLIDE_PASSES as f32 * scale) as u32).max(1)
}

/// Passes from `start` to `clock` (milliseconds), 60 a second.
pub fn passes(start: u64, clock: u64) -> u32 {
    (clock.saturating_sub(start) * PASSES_PER_SECOND / 1000).min(u32::MAX as u64) as u32
}

/// A slide of the six buttons that is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuAnim {
    pub opening: bool,
    /// The session clock (ms) when it started.
    pub start: u64,
    /// Passes it takes.
    pub n: u32,
}

impl MenuAnim {
    /// Passes since the start; the first frame is pass 0.
    pub fn pass(&self, clock: u64) -> u32 {
        passes(self.start, clock)
    }

    /// Whether the last pass has been shown.
    pub fn done(&self, clock: u64) -> bool {
        self.pass(clock) >= self.n
    }

    pub fn slide(&self, clock: u64) -> Slide {
        Slide {
            opening: self.opening,
            t: self.pass(clock),
            n: self.n,
        }
    }
}

/// One frame of the slide: pass `t` of `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slide {
    pub opening: bool,
    pub t: u32,
    pub n: u32,
}

impl Slide {
    /// Where and how opaque the side button `i` (1 to 6) is, or `None`
    /// when it is not shown. Linear from the menu button's place (opening)
    /// or to it (closing), truncating like integer division.
    pub fn place(&self, i: usize) -> Option<(i32, i32, u8)> {
        let own = POS[i];
        if self.t >= self.n {
            return self.opening.then_some((own.0, own.1, 255));
        }
        let (from, to, a0, a1) = if self.opening {
            (POS[0], own, 0, 255)
        } else {
            (own, POS[0], 255, 0)
        };
        let (t, n) = (self.t as i32, self.n as i32);
        let at = |a: i32, b: i32| a + (b - a) * t / n;
        Some((at(from.0, to.0), at(from.1, to.1), at(a0, a1) as u8))
    }
}

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
        slide: Option<Slide>,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        let count = if open { BUTTONS.len() } else { 1 };
        // The six first, the menu button over them.
        for i in (1..count).chain([0]) {
            let id = BUTTONS[i];
            let row = state(id);
            if row >= HIDDEN {
                continue;
            }
            let (x, y, alpha) = match slide.filter(|_| i > 0) {
                Some(s) => match s.place(i) {
                    Some(p) => p,
                    None => continue,
                },
                None => (POS[i].0, POS[i].1, 255),
            };
            let part = if i == 0 {
                let (w, h) = MENU_CELL;
                (0, h * row, w, h)
            } else {
                (MENU_CELL.0 + CELL * (i as u32 - 1), CELL * row, CELL, CELL)
            };
            canvas.draw_part(&sheet, part, x, y, false, alpha);
        }
        Ok(())
    }
}

impl std::fmt::Debug for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Menu")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_follow_the_effect_scale() {
        assert_eq!(slide_passes(1.0), 12);
        assert_eq!(slide_passes(0.5), 6);
        assert_eq!(slide_passes(0.0), 1);
    }

    #[test]
    fn opening_starts_at_the_menu_button() {
        let s = Slide {
            opening: true,
            t: 0,
            n: 12,
        };
        assert_eq!(s.place(1), Some((POS[0].0, POS[0].1, 0)));
        let s = Slide {
            opening: true,
            t: 6,
            n: 12,
        };
        let (x, y, a) = s.place(2).unwrap();
        assert_eq!(x, 1729 + (1740 - 1729) * 6 / 12);
        assert_eq!(y, 2 + (196 - 2) * 6 / 12);
        assert_eq!(a, 127);
        let s = Slide {
            opening: true,
            t: 12,
            n: 12,
        };
        assert_eq!(s.place(6), Some((1740, 537, 255)));
    }

    #[test]
    fn closing_truncates_toward_zero_and_ends_hidden() {
        // x goes from 1655 to 1729: positive step; 1740 to 1729: negative.
        let s = Slide {
            opening: false,
            t: 5,
            n: 12,
        };
        let (x, _, a) = s.place(2).unwrap();
        assert_eq!(x, 1740 + (1729 - 1740) * 5 / 12);
        assert_eq!(x, 1740 - 4);
        assert_eq!(a as i32, 255 + (0 - 255) * 5 / 12);
        let s = Slide {
            opening: false,
            t: 12,
            n: 12,
        };
        assert_eq!(s.place(2), None);
    }

    #[test]
    fn anim_counts_sixty_passes_a_second() {
        let a = MenuAnim {
            opening: true,
            start: 1000,
            n: 12,
        };
        assert_eq!(a.pass(1000), 0);
        assert_eq!(a.pass(1016), 0);
        assert_eq!(a.pass(1017), 1);
        assert!(!a.done(1199));
        assert!(a.done(1200));
    }
}
