//! The hours dial of 同級生 Remake (`Game.29`, the program's
//! `CTimeSelect`): how many hours the hero sleeps, or how far the clock
//! goes back or forward.
//!
//! Read from the executable: `time_p_<lang>.akb` holds the dial (750 x
//! 756 from (0, 592), drawn at (586, 42)), the question of each kind (403
//! x 76 from (1152, 76 × kind), drawn at (760, 456)), the eight hour
//! buttons (144 x 148 cells, a row per state) and the back button (151 x
//! 153 cells from (750, 592)). `TimeSelect.area` places the buttons:
//! ids 0..8 for one to eight hours, 256 for back. The call returns the
//! hours, 0 for back. See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::area::{self, Area};

use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

/// Area id of the back button.
pub const BACK: i32 = 256;

const SHEET: &str = "time_p.akb";
const DIAL: (u32, u32, u32, u32) = (0, 592, 750, 756);
const DIAL_POS: (i32, i32) = (586, 42);
const QUESTION: (u32, u32, u32) = (1152, 403, 76);
const QUESTION_POS: (i32, i32) = (760, 456);
const HOUR_CELL: (u32, u32) = (144, 148);
const BACK_CELL: (u32, u32, u32, u32) = (750, 592, 151, 153);

#[derive(Debug, Clone)]
pub struct Hours {
    /// 0 sleep, 1 wind back, 2 skip forward.
    pub kind: i32,
    areas: Arc<Vec<Area>>,
}

impl Hours {
    pub fn load(assets: &Assets, kind: i32) -> Result<Self> {
        let areas = area::parse(&assets.data("TimeSelect.area")?)?;
        Ok(Self {
            kind: kind.clamp(0, 2),
            areas: Arc::new(areas),
        })
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        self.areas.iter().find(|a| a.contains(x, y)).map(|a| a.id)
    }

    /// The hours for a button, 0 for back.
    pub fn click(&self, id: i32) -> i32 {
        if id == BACK {
            0
        } else {
            (id & 0xff) + 1
        }
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed) of an id.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let sheet = assets.localized_image(SHEET, language)?;
        canvas.draw_part(&sheet, DIAL, DIAL_POS.0, DIAL_POS.1, false, 255);
        let (qx, qw, qh) = QUESTION;
        let part = (qx, qh * self.kind as u32, qw, qh);
        canvas.draw_part(&sheet, part, QUESTION_POS.0, QUESTION_POS.1, false, 255);
        for a in self.areas.iter() {
            let s = state(a.id);
            let part = if a.id == BACK {
                let (x, y, w, h) = BACK_CELL;
                (x, y + h * s, w, h)
            } else {
                let (w, h) = HOUR_CELL;
                (w * (a.id & 0xff) as u32, h * s, w, h)
            };
            canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results() {
        let h = Hours {
            kind: 0,
            areas: Arc::new(Vec::new()),
        };
        assert_eq!(h.click(BACK), 0);
        assert_eq!(h.click(0), 1);
        assert_eq!(h.click(7), 8);
    }
}
