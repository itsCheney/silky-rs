//! Selections of 同級生 Remake (`Select.2`, the program's
//! `CNormalSelect`), read from the executable and checked under Wine.
//!
//! Each option is a bar built from `sel_p.akb` (rows of 110 pixels by
//! state): the 60-pixel ends, 105 pixels of the body next to each end and
//! 40-pixel body slices between them, one per two cells of the longest
//! label. The bars are centred across the screen and, 118 pixels apart,
//! in the 780 pixels above the bottom buttons; a label is centred in its
//! bar, 35 pixels from its top, in the message font. Up to four options
//! show at a time.

use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

use super::window::{text_cells, Font, CELL};

const SHEET: &str = "sel_p.akb";
const BAR_H: i32 = 110;
const STEP: i32 = 118;
const END: u32 = 60;
const NEXT_TO_END: u32 = 105;
const SLICE: i32 = 40;
/// Right end of the bar on the sheet.
const RIGHT_END: u32 = 730;
const TEXT_TOP: i32 = 35;
/// Height the bars are centred in.
const AREA_H: i32 = 1080 - 300;
const SCREEN_W: i32 = 1920;
pub const PAGE: usize = 4;

/// Where the bars of a selection go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bars {
    pub left: i32,
    pub width: i32,
    pub top: i32,
    pub count: usize,
    slices: i32,
}

impl Bars {
    pub fn new(labels: &[String]) -> Self {
        let count = labels.len().min(PAGE);
        let slices = labels
            .iter()
            .take(count)
            .map(|l| text_cells(l) / 2)
            .max()
            .unwrap_or(0);
        let width = 2 * (END + NEXT_TO_END) as i32 + SLICE * slices;
        Self {
            left: (SCREEN_W - width) / 2,
            width,
            top: (AREA_H - STEP * count as i32) / 2,
            count,
            slices,
        }
    }

    pub fn bar_top(&self, i: usize) -> i32 {
        self.top + STEP * i as i32
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.count).find(|&i| {
            let t = self.bar_top(i);
            x >= self.left && x < self.left + self.width && y >= t && y < t + BAR_H
        })
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed) of option `i`.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        font: Option<&Font>,
        labels: &[String],
        state: impl Fn(usize) -> u32,
    ) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        let h = BAR_H as u32;
        for (i, label) in labels.iter().take(self.count).enumerate() {
            let (x, y) = (self.left, self.bar_top(i));
            let row = h * state(i).min(2);
            let mut part = |sx: u32, w: u32, dx: i32| {
                canvas.draw_part(&sheet, (sx, row, w, h), x + dx, y, false, 255);
            };
            part(0, END, 0);
            part(END, NEXT_TO_END, END as i32);
            for k in 0..self.slices {
                part(END, SLICE as u32, (END + NEXT_TO_END) as i32 + SLICE * k);
            }
            part(END, NEXT_TO_END, self.width - (END + NEXT_TO_END) as i32);
            part(RIGHT_END, END, self.width - END as i32);
            if let Some(f) = font {
                let tx = x + (self.width - CELL * text_cells(label)) / 2;
                f.draw_line(canvas, label, tx, y + TEXT_TOP);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_the_original() {
        // "Am I really going to bed?" under Wine: bars from x 695, at y
        // 272 and 390.
        let labels = ["Go to sleep".to_string(), "Stay awake".to_string()];
        let b = Bars::new(&labels);
        assert_eq!((b.left, b.width, b.top), (695, 530, 272));
        assert_eq!(b.bar_top(1), 390);
        assert_eq!(b.hit(700, 300), Some(0));
        assert_eq!(b.hit(700, 385), None);
    }
}
