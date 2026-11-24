//! The staff roll of きまぐれテンプテーション (`Game.990`).
//!
//! Sub 0 is the roll after the normal endings and sub 1 the one after the
//! true ending; they differ in the strip (`staff01b.akb`, `staff01a.akb`),
//! the cover (black, white) and the track (`KIMA_22`, `KIMA_24`). The whole
//! roll runs on the clock, in milliseconds from its start:
//!
//! - up to 1033 the cover hides a card (`staff00.akb`), over 2000 it fades
//!   away, the card stays up to 8033, and over 5000 the cover comes back;
//! - from 13033 the strip, 3080 pixels tall, rises from the bottom edge
//!   (its top at 680) at an even speed until its bottom has reached the
//!   bottom of the screen at 107000, which ends the roll (the original
//!   ends it at 106999).
//!
//! The call pushes 1 when the roll is over. A press ends it early; what
//! the original does with a press (it checks a setting of the message
//! window first) was not found, so any press ends it here.

use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

const CARD: &str = "staff00.akb";
/// Milliseconds: the cover is gone at `FADE_OUT.1`, comes back from
/// `FADE_IN.0` to `FADE_IN.1`, and the roll starts there.
const FADE_OUT: (u64, u64) = (1033, 3033);
const FADE_IN: (u64, u64) = (8033, 13033);
const END: u64 = 106_999;
const ROLL_END: u64 = 107_000;
/// Top of the strip when it starts, and how far it rises.
const START_Y: i32 = 680;
const RISE: i32 = 3040;
const SCREEN: (u32, u32) = (1280, 720);

#[derive(Debug, Clone)]
pub struct Staff {
    /// After the true ending.
    true_end: bool,
    stop_music: bool,
}

impl Staff {
    pub fn open(true_end: bool) -> Self {
        Self {
            true_end,
            stop_music: false,
        }
    }

    /// The track the roll plays.
    pub fn track(true_end: bool) -> &'static str {
        if true_end {
            "KIMA_24.WAV"
        } else {
            "KIMA_22.WAV"
        }
    }

    fn strip(&self) -> &'static str {
        if self.true_end {
            "staff01a.akb"
        } else {
            "staff01b.akb"
        }
    }

    /// Opacity of the cover at `ms`.
    pub fn cover(ms: u64) -> u8 {
        let ramp = |from: u64, to: u64| ((ms - from) * 255 / (to - from)) as u8;
        match ms {
            0..=1032 => 255,
            m if m < FADE_OUT.1 => 255 - ramp(FADE_OUT.0, FADE_OUT.1),
            m if m < FADE_IN.0 => 0,
            m if m < FADE_IN.1 => ramp(FADE_IN.0, FADE_IN.1),
            _ => 255,
        }
    }

    /// Top of the strip at `ms`, or `None` before the roll starts.
    pub fn strip_top(ms: u64) -> Option<i32> {
        (ms >= FADE_IN.1).then(|| {
            let t = (ms.min(ROLL_END) - FADE_IN.1) as i64;
            START_Y - (RISE as i64 * t / (ROLL_END - FADE_IN.1) as i64) as i32
        })
    }

    pub fn over(&self, ms: u64) -> bool {
        ms >= END
    }

    pub fn hit(&self, _x: i32, _y: i32) -> Option<i32> {
        Some(0)
    }

    pub fn click(&mut self, _id: i32) -> Click {
        self.stop_music = true;
        Click::Result(1)
    }

    pub fn take_stop(&mut self) -> bool {
        std::mem::take(&mut self.stop_music)
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let (w, h) = SCREEN;
        match Self::strip_top(look.ms) {
            None => {
                let card = assets.image(CARD)?;
                canvas.draw_part(&card, (0, 0, w, h), 0, 0, false, 255);
                let c = if self.true_end { 255 } else { 0 };
                canvas.fill(0, 0, w, h, [c, c, c, Self::cover(look.ms)]);
            }
            Some(top) => {
                canvas.fill(0, 0, w, h, [0, 0, 0, 255]);
                let strip = assets.image(self.strip())?;
                let rows = strip.height as i32;
                let from = (-top).max(0);
                let to = (h as i32 - top).min(rows);
                if to > from {
                    canvas.draw_part(
                        &strip,
                        (0, from as u32, w, (to - from) as u32),
                        0,
                        top + from,
                        false,
                        255,
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_and_strip_follow_the_clock() {
        assert_eq!(Staff::cover(0), 255);
        assert_eq!(Staff::cover(2033), 128);
        assert_eq!(Staff::cover(5000), 0);
        assert_eq!(Staff::cover(10533), 127);
        assert_eq!(Staff::strip_top(13000), None);
        assert_eq!(Staff::strip_top(13033), Some(680));
        assert_eq!(Staff::strip_top(107_000), Some(-2360));
    }
}
