//! The train of Misa's scenes (`Game.39`): a picture object, the side of
//! a train 8000 pixels wide (`y04btr.akb`, its windows see-through), that
//! slides across the screen over the background.
//!
//! Read from the executable. Sub 0 makes the object (shown at the top
//! left until something moves it), sub 1 takes it away. Sub 2 brings the
//! train in from the right edge and stops it with its right end on the
//! screen (494 frames, the script waits); sub 3 sends it on to the left
//! until it is gone (228 frames, the script waits); sub 4 puts it at the
//! stopping place at once. The speed follows a table of steps: 32 pixels
//! a frame at first, then 16, 8, 4, 2 and 1 as it comes to a halt, and
//! the way out is the same table run the other way (2, 4, 8, then 16). Sub
//! 5 draws `y04bb.akb` into the background at once. The scripts never
//! call sub 4. See `docs/games/doukyuusei.md`.
//!
//! A frame is one sixtieth of a second here, as for the other waits; the
//! original's pace was not measured under Wine.

use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// The train's picture and the one sub 5 draws into the background.
pub const FILE: &str = "y04btr.akb";
pub const BACKGROUND: &str = "y04bb.akb";

/// Where the train's left end is when it has stopped, and when it is gone.
pub const STOP_X: i32 = -6244;
pub const GONE_X: i32 = -8000;
/// Where the left end starts when it comes in: the screen's right edge.
const ENTER_X: i32 = 1920;

pub const ARRIVE_FRAMES: i32 = 494;
pub const LEAVE_FRAMES: i32 = 228;

/// The two moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Move {
    Arrive,
    Leave,
}

impl Move {
    /// How many frames the move takes.
    pub fn frames(self) -> i32 {
        match self {
            Move::Arrive => ARRIVE_FRAMES,
            Move::Leave => LEAVE_FRAMES,
        }
    }

    /// Where the left end is at frame `f` of the move.
    pub fn x_at(self, f: i32) -> i32 {
        let f = f.clamp(0, self.frames());
        match self {
            Move::Arrive => {
                let shift = match f {
                    0..=170 => f * 32,
                    171..=270 => (f + 170) * 16,
                    271..=370 => f * 8 + 4880,
                    371..=420 => f * 4 + 6360,
                    421..=470 => f * 2 + 7200,
                    _ => f + 7670,
                };
                ENTER_X - shift
            }
            Move::Leave => {
                let shift = match f {
                    0..=28 => f * 2,
                    29..=78 => f * 4 - 56,
                    79..=178 => f * 8 - 368,
                    _ => (f - 112) * 16,
                };
                STOP_X - shift
            }
        }
    }
}

/// The object: made by sub 0, kept in the scene until sub 1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Train {
    /// Left end, and whether it is shown, once the last move is over.
    pub x: i32,
    pub visible: bool,
    /// The last move, to be shown while it runs.
    pub moved: Option<Move>,
    /// Frame of that move while the script waits for it.
    #[serde(skip)]
    pub frame: Option<i32>,
}

impl Default for Train {
    fn default() -> Self {
        Self::new()
    }
}

impl Train {
    /// As sub 0 leaves it.
    pub fn new() -> Self {
        Self {
            x: 0,
            visible: true,
            moved: None,
            frame: None,
        }
    }

    /// Sub 2: in from the right and stops.
    pub fn arrive(&mut self) {
        *self = Self {
            x: STOP_X,
            visible: true,
            moved: Some(Move::Arrive),
            frame: None,
        };
    }

    /// Sub 3: on to the left until gone.
    pub fn leave(&mut self) {
        *self = Self {
            x: GONE_X,
            visible: false,
            moved: Some(Move::Leave),
            frame: None,
        };
    }

    /// Sub 4: at the stopping place at once.
    pub fn park(&mut self) {
        *self = Self {
            x: STOP_X,
            visible: true,
            moved: None,
            frame: None,
        };
    }

    /// The wait for the last move has gone `elapsed` of `total`.
    pub fn progress(&mut self, elapsed: u32, total: u32) {
        if let Some(m) = self.moved {
            let f = elapsed as u64 * m.frames() as u64 / total.max(1) as u64;
            self.frame = Some(f.min(m.frames() as u64) as i32);
        }
    }

    /// The left end to draw, `None` while nothing of it is shown.
    pub fn shown(&self) -> Option<i32> {
        match (self.frame, self.moved) {
            (Some(f), Some(m)) => Some(m.x_at(f)),
            _ => self.visible.then_some(self.x),
        }
    }

    /// Draws the part of the picture that is on the screen.
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets) -> Result<()> {
        let Some(x) = self.shown() else {
            return Ok(());
        };
        let img = assets.image(FILE)?;
        let left = x.max(0);
        let right = (x + img.width as i32).min(canvas.width as i32);
        if right <= left {
            return Ok(());
        }
        let part = ((left - x) as u32, 0, (right - left) as u32, img.height);
        canvas.draw_part(&img, part, left, 0, false, 255);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrival_runs_from_the_edge_to_the_stop() {
        assert_eq!(Move::Arrive.x_at(0), 1920);
        assert_eq!(Move::Arrive.x_at(ARRIVE_FRAMES), STOP_X);
        // Never backwards.
        let xs: Vec<i32> = (0..=ARRIVE_FRAMES).map(|f| Move::Arrive.x_at(f)).collect();
        assert!(xs.windows(2).all(|w| w[0] >= w[1]));
        assert_eq!(xs[1] - xs[2], 32);
        assert_eq!(xs[493] - xs[494], 1);
    }

    #[test]
    fn leaving_goes_on_from_the_stop() {
        assert_eq!(Move::Leave.x_at(0), STOP_X);
        let xs: Vec<i32> = (0..=LEAVE_FRAMES).map(|f| Move::Leave.x_at(f)).collect();
        assert!(xs.windows(2).all(|w| w[0] > w[1]));
        // The whole picture is off the left edge by the end.
        assert!(xs[LEAVE_FRAMES as usize] + 8000 <= 0);
        assert_eq!(xs[1] - xs[2], 2);
        assert_eq!(xs[228] - xs[227], -16);
    }

    #[test]
    fn states() {
        let mut t = Train::new();
        assert_eq!(t.shown(), Some(0));
        t.arrive();
        assert_eq!(t.shown(), Some(STOP_X));
        t.progress(0, 8000);
        assert_eq!(t.shown(), Some(1920));
        t.progress(4000, 8000);
        assert_eq!(t.frame, Some(247));
        t.frame = None;
        t.leave();
        assert_eq!(t.shown(), None);
        t.progress(1, 8000);
        assert_eq!(t.shown(), Some(STOP_X));
        t.frame = None;
        t.park();
        assert_eq!(t.shown(), Some(STOP_X));
    }
}
