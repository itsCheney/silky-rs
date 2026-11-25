//! The message window of なないろリンカネーション, from the executable.
//!
//! The window is the alpha mask in the top 199 rows of `meswaku.akb`,
//! filled with the window colour, on a layer at the top given by
//! `/Config/MesWnd/Pos` (521 in a new install). The buttons are cells of
//! the same sheet laid out over the click areas of `mes.area`, which are
//! relative to the window.

use std::cell::Cell;

use silky_format::area::{self, Area};

use crate::assets::Assets;
use crate::render::Canvas;
use crate::ui::{bar, Button, Ui};
use crate::Result;

/// Sheet of the window and its buttons.
pub const SHEET: &str = "meswaku.akb";
/// Top of the window on screen.
pub const TOP: i32 = 521;
/// Alpha mask of the window on the sheet.
pub const BAND: (u32, u32, u32, u32) = (0, 0, 1280, 199);

/// Window colour (red, green, blue of 268) and transparency (of 268) of a
/// new install; the executable scales each by 255 / 268.
pub const COLOUR: [u32; 3] = [19, 70, 133];
pub const ALPHA: u32 = 50;
pub const SCALE: u32 = 268;

/// Left of the text and the first line's baseline (measured under Wine:
/// glyph rows 618..637), the pen advance of a full-width character, the
/// line pitch and the line length (not measured yet).
pub const TEXT_LEFT: f32 = 258.0;
pub const TEXT_BASELINE: f32 = 637.0;
pub const TEXT_CELL: f32 = 28.0;
pub const TEXT_LINE: f32 = 32.0;
pub const TEXT_WIDTH: f32 = 28.0 * 28.0;

/// The click-wait mark: a 29 by 28 layer at x = 1050, 154 below the window
/// top. Its frames lie in two rows of the sheet from x = 691, 29 apart: a
/// triangle (11 frames, row y = 461) and a speaker (14 frames, row y = 433,
/// shown while a voice plays).
pub const WAIT_AT: (i32, i32) = (1050, TOP + 154);
const WAIT_X: u32 = 691;
const WAIT_W: u32 = 29;
const WAIT_H: u32 = 28;
const TRIANGLE: (u32, i32) = (461, 11);
const SPEAKER: (u32, i32) = (433, 14);

/// The animation of the click-wait mark. A timer steps it: the frames 0 to
/// 4 play in, 5 and 6 (triangle) or 5 to 8 (speaker) repeat, and when the
/// voice starts or stops the current picture plays out (triangle frames 7
/// to 10, speaker frames 9 to 13) before the other one plays in. Steps
/// are 100 ms, and 400 ms while repeating.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    frame: i32,
    phase: i32,
    speaker: i32,
    /// Sheet row and frame shown now.
    shown: (u32, i32),
    /// Clock time of the next step.
    due: u64,
}

impl Mark {
    /// A new message waits for a click at `clock`.
    pub fn new(clock: u64) -> Self {
        Self {
            frame: 0,
            phase: 0,
            speaker: -1,
            shown: (TRIANGLE.0, 0),
            due: clock,
        }
    }

    /// One step of the timer; returns the time to the next one.
    fn step(&mut self, voice: bool) -> u64 {
        let ((row, count), speaker) = if voice {
            if self.speaker == 0 && self.phase == 1 {
                self.phase = 2;
                self.frame = 7;
                (TRIANGLE, 1)
            } else if self.speaker == 0 || self.phase != 2 {
                (SPEAKER, 1)
            } else {
                (TRIANGLE, 1)
            }
        } else if self.speaker == 1 {
            if self.phase == 1 {
                self.phase = 2;
                self.frame = 9;
                (SPEAKER, 0)
            } else {
                (TRIANGLE, 0)
            }
        } else if self.phase == 2 {
            (SPEAKER, 0)
        } else {
            (TRIANGLE, 0)
        };
        self.speaker = speaker;
        self.shown = (row, self.frame);
        self.frame = (self.frame + 1) % count;
        match self.phase {
            0 => {
                if self.frame == 5 {
                    self.phase = 1;
                }
                100
            }
            1 => {
                let end = if voice { 9 } else { 7 };
                if self.frame == end {
                    self.frame = 5;
                }
                400
            }
            _ => {
                if self.frame == 0 {
                    self.phase = 0;
                    self.speaker = 1;
                }
                100
            }
        }
    }

    /// Brings the animation up to `clock` and gives the sheet cell to draw.
    pub fn at(&mut self, clock: u64, voice: bool) -> (u32, u32, u32, u32) {
        for _ in 0..64 {
            if clock < self.due {
                break;
            }
            self.due += self.step(voice);
        }
        if clock >= self.due {
            self.due = clock + 1;
        }
        let (row, f) = self.shown;
        (WAIT_X + WAIT_W * f as u32, row, WAIT_W, WAIT_H)
    }
}

/// The window buttons: (bar id, click area on screen, sheet cell, cell
/// size, state row step). State rows are normal, hover, pressed and
/// disabled.
pub struct Bar {
    buttons: Vec<(i32, Area, (u32, u32), (u32, u32), u32)>,
    /// The wait mark's animation while a message waits.
    mark: Cell<Option<Mark>>,
}

impl Bar {
    pub fn load(assets: &Assets) -> Result<Self> {
        let mut buttons = Vec::new();
        for a in area::parse(&assets.data("mes.area")?)? {
            let (id, cell, size, step) = match a.id {
                0 => (bar::SAVE, (0, 433), (77, 22), 22),
                1 => (bar::LOAD, (77, 433), (77, 22), 22),
                2 => (bar::QUICK_SAVE, (154, 433), (77, 22), 22),
                3 => (bar::QUICK_LOAD, (231, 433), (77, 22), 22),
                4 => (bar::LOOP_VOICE, (308, 433), (77, 22), 22),
                5 => (bar::HIDE, (646, 433), (45, 44), 44),
                6 => (bar::AUTO, (1000, 199), (45, 44), 44),
                7 => (bar::SKIP, (1045, 199), (45, 44), 44),
                8 => (bar::NEXT_CHOICE, (1135, 199), (45, 44), 44),
                9 => (bar::PREVIOUS_CHOICE, (1090, 199), (45, 44), 44),
                10 => (bar::TITLE, (511, 433), (45, 44), 44),
                11 => (bar::BACKLOG, (556, 433), (45, 44), 44),
                12 => (bar::CONFIG, (601, 433), (45, 44), 44),
                _ => continue,
            };
            let mut a = a;
            a.top += TOP;
            a.bottom += TOP;
            buttons.push((id, a, cell, size, step));
        }
        Ok(Self {
            buttons,
            mark: Cell::new(None),
        })
    }

    pub fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(id, a, ..)| a.contains(x, y) && ui.enabled(Button::Bar(*id)))
            .map(|&(id, ..)| Button::Bar(id))
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        for &(id, a, (sx, sy), (w, h), step) in &self.buttons {
            let b = Button::Bar(id);
            // Auto, skip and voice loop show pressed while on.
            let active = match id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip,
                bar::LOOP_VOICE => ui.loop_voice,
                _ => false,
            };
            let state = ui.state(b, ui.enabled(b), active) as u32;
            canvas.draw_part(
                &sheet,
                (sx, sy + step * state, w, h),
                a.left,
                a.top,
                false,
                255,
            );
        }
        // The click-wait mark, from the start of each wait.
        if ui.waiting {
            let mut mark = self.mark.get().unwrap_or_else(|| Mark::new(ui.clock));
            let cell = mark.at(ui.clock, ui.voice);
            self.mark.set(Some(mark));
            canvas.draw_part(&sheet, cell, WAIT_AT.0, WAIT_AT.1, false, 255);
        } else {
            self.mark.set(None);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(voice: impl Fn(u64) -> bool, until: u64) -> Vec<(u32, u32)> {
        let mut m = Mark::new(0);
        let mut out = Vec::new();
        let mut t = 0;
        while t <= until {
            let c = m.at(t, voice(t));
            let f = ((c.0 - WAIT_X) / WAIT_W, c.1);
            if out.last() != Some(&f) {
                out.push(f);
            }
            t += 20;
        }
        out
    }

    /// Without a voice: the triangle plays in, then frames 5 and 6 repeat.
    #[test]
    fn triangle() {
        let f = frames(|_| false, 2000);
        let rows: Vec<u32> = f.iter().map(|x| x.0).collect();
        assert_eq!(&rows[..9], &[0, 1, 2, 3, 4, 5, 6, 5, 6]);
        assert!(f.iter().all(|x| x.1 == 461));
    }

    /// A voice that ends switches the speaker for the triangle through the
    /// speaker's play-out.
    #[test]
    fn speaker_ends() {
        let f = frames(|t| t < 1500, 4000);
        assert!(f.iter().any(|x| x.1 == 433));
        assert!(f.contains(&(13, 433)));
        assert_eq!(f.last().map(|x| x.1), Some(461));
    }
}
