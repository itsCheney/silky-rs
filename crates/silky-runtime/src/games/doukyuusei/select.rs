//! Selections of 同級生 Remake (`Select.2`, the program's
//! `CNormalSelect`), read from the executable and checked under Wine.
//!
//! Each option is a bar built from `sel_p.akb` (rows of 110 pixels by
//! state): the 60-pixel ends, 105 pixels of the body next to each end and
//! 40-pixel body slices between them, one per two cells of the longest
//! label. Up to four options show at a time; with more, arrows above and
//! below the bars turn the page. Options are grayed in replay mode
//! (`h[69]`) unless their mark `b[1900 + N]` is 1.

use std::ops::Range;

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
/// Options per page.
pub const PAGE: usize = 4;
/// Hot spot ids of the arrows (the bars are the options on the page).
pub const PREV: i32 = 4;
pub const NEXT: i32 = 5;
/// Arrows: 397 x 59, the rows of the sheet by state.
const ARROW_W: i32 = 397;
const ARROW_H: i32 = 59;
const PREV_ROW: u32 = 440;
const NEXT_ROW: u32 = 676;
/// Space above the first bar when paging, which the arrow takes.
const ARROW_GAP: i32 = 8;
const PAGED_TOP: i32 = 99;
/// Badges: 105 x 110 cells, 147 pixels in from the bar's left end.
const BADGE_W: u32 = 105;
const BADGE_H: u32 = 110;
const BADGE_IN: i32 = 147;
const BADGE_X: u32 = 397;
const BADGE_UP: u32 = 550;
const BADGE_DOWN: u32 = 660;
/// Arrow x: centred.
const ARROW_X: i32 = (SCREEN_W - ARROW_W) / 2;

/// How an option is shown and answered. `mode` is `h[69]` (a memory scene
/// replay when not 0) and `mark` is `b[1900 + N]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// Can be picked (hover and press show, a click answers).
    pub clickable: bool,
    /// Text drawn gray (0x808080).
    pub gray: bool,
    /// The mark byte: 2 and 3 draw a badge in game mode 1.
    pub mark: u8,
}

/// Status of an option with no replay rule (all options open).
pub const OPEN: Status = Status {
    clickable: true,
    gray: false,
    mark: 0,
};

/// The status of an option from `h[69]` and its mark. In replay mode an
/// option is clickable only when its mark is 1; gray text is drawn only
/// when the mark is 0.
pub fn status(mode: i32, mark: u8) -> Status {
    Status {
        clickable: mode == 0 || mark == 1,
        gray: mode != 0 && mark == 0,
        mark,
    }
}

/// Last page index for `count` options (0 for none).
pub fn last_page(count: usize) -> usize {
    count.saturating_sub(1) / PAGE
}

/// Options shown on `page`, as indices into the list.
pub fn page_range(count: usize, page: usize) -> Range<usize> {
    let start = (page * PAGE).min(count);
    start..(start + PAGE).min(count)
}

/// Whether the arrows are in use (more options than one page).
pub fn paged(count: usize) -> bool {
    count > PAGE
}

/// Whether the previous-page arrow is drawn.
pub fn prev_shown(count: usize, page: usize) -> bool {
    paged(count) && page > 0
}

/// Whether the next-page arrow is drawn.
pub fn next_shown(count: usize, page: usize) -> bool {
    paged(count) && page < last_page(count)
}

/// The page after an arrow (`PREV` or `NEXT`) is clicked; other ids keep it.
pub fn turn(count: usize, page: usize, id: i32) -> usize {
    match id {
        PREV if page > 0 => page - 1,
        NEXT if page < last_page(count) => page + 1,
        _ => page,
    }
}

/// What a point hits in a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// An option, by its index in the list.
    Bar(usize),
    Prev,
    Next,
}

/// Where the bars and arrows of a selection page go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bars {
    pub left: i32,
    pub width: i32,
    /// Options on the page, as indices into the list.
    pub range: Range<usize>,
    /// The page shown (clamped to the last one).
    pub page: usize,
    count: usize,
    /// Top of the page's first bar.
    top: i32,
    slices: i32,
    /// Status of each option of the list (open when empty).
    status: Vec<Status>,
    /// The easy mode is set: the badges show.
    easy: bool,
}

/// The sheet row of the badge a mark draws in the easy mode: 2 the hearts
/// up, 3 the hearts down; other marks draw none.
fn badge_row(mark: u8) -> Option<u32> {
    match mark {
        2 => Some(BADGE_UP),
        3 => Some(BADGE_DOWN),
        _ => None,
    }
}

impl Bars {
    /// The bars of the first page.
    pub fn new(labels: &[String]) -> Self {
        Self::on_page(labels, 0)
    }

    /// The bars of `page` of the options in `labels`. The width covers
    /// every option, so it stays the same on each page.
    pub fn on_page(labels: &[String], page: usize) -> Self {
        let count = labels.len();
        let page = page.min(last_page(count));
        let range = page_range(count, page);
        let slices = labels.iter().map(|l| text_cells(l) / 2).max().unwrap_or(0);
        let width = 2 * (END + NEXT_TO_END) as i32 + SLICE * slices;
        let top = if paged(count) {
            PAGED_TOP + ARROW_H + ARROW_GAP
        } else {
            (AREA_H - STEP * range.len() as i32) / 2
        };
        Self {
            left: (SCREEN_W - width) / 2,
            width,
            range,
            page,
            count,
            top,
            slices,
            status: Vec::new(),
            easy: true,
        }
    }

    /// Sets the status of each option (see [`status`]).
    pub fn with_status(mut self, status: &[Status]) -> Self {
        self.status = status.to_vec();
        self
    }

    /// Sets the game mode: the badges show in the easy mode (the default).
    pub fn with_easy(mut self, easy: bool) -> Self {
        self.easy = easy;
        self
    }

    /// Top of option `i` (on the page).
    pub fn bar_top(&self, i: usize) -> i32 {
        self.top + STEP * i.saturating_sub(self.range.start) as i32
    }

    fn next_top(&self) -> i32 {
        self.bar_top(self.range.end.saturating_sub(1)) + BAR_H + ARROW_GAP
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        let inside =
            |x0: i32, y0: i32, w: i32, h: i32| x >= x0 && x < x0 + w && y >= y0 && y < y0 + h;
        if prev_shown(self.count, self.page) && inside(ARROW_X, PAGED_TOP, ARROW_W, ARROW_H) {
            return Some(Hit::Prev);
        }
        if next_shown(self.count, self.page) && inside(ARROW_X, self.next_top(), ARROW_W, ARROW_H) {
            return Some(Hit::Next);
        }
        self.range
            .clone()
            .find(|&i| inside(self.left, self.bar_top(i), self.width, BAR_H))
            .map(Hit::Bar)
    }

    /// Draws the page. `state` gives the row (0 normal, 1 hover, 2
    /// pressed) of each hit.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        font: Option<&Font>,
        labels: &[String],
        state: impl Fn(Hit) -> u32,
    ) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        let h = BAR_H as u32;
        for i in self.range.clone() {
            let (x, y) = (self.left, self.bar_top(i));
            let row = h * state(Hit::Bar(i)).min(2);
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
            let s = self.status.get(i).copied().unwrap_or(OPEN);
            // Badges show in the easy mode only.
            if self.easy {
                if let Some(by) = badge_row(s.mark) {
                    let bx = x + self.width - BADGE_IN;
                    canvas.draw_part(&sheet, (BADGE_X, by, BADGE_W, BADGE_H), bx, y, false, 255);
                }
            }
            if let (Some(f), Some(label)) = (font, labels.get(i)) {
                let tx = x + (self.width - CELL * text_cells(label)) / 2;
                if s.gray {
                    f.draw_line_gray(canvas, label, tx, y + TEXT_TOP);
                } else {
                    f.draw_line(canvas, label, tx, y + TEXT_TOP);
                }
            }
        }
        if prev_shown(self.count, self.page) {
            let row = ARROW_H as u32 * state(Hit::Prev).min(2);
            let src = (0, PREV_ROW + row, ARROW_W as u32, ARROW_H as u32);
            canvas.draw_part(&sheet, src, ARROW_X, PAGED_TOP, false, 255);
        }
        if next_shown(self.count, self.page) {
            let row = ARROW_H as u32 * state(Hit::Next).min(2);
            let src = (0, NEXT_ROW + row, ARROW_W as u32, ARROW_H as u32);
            canvas.draw_part(&sheet, src, ARROW_X, self.next_top(), false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_start_easy() {
        let l = labels(2);
        assert!(Bars::new(&l).easy);
        assert!(!Bars::new(&l).with_easy(false).easy);
    }

    #[test]
    fn badges_by_mark() {
        assert_eq!(badge_row(0), None);
        assert_eq!(badge_row(1), None);
        assert_eq!(badge_row(2), Some(BADGE_UP));
        assert_eq!(badge_row(3), Some(BADGE_DOWN));
    }

    fn labels(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("Option {i}")).collect()
    }

    #[test]
    fn like_the_original() {
        // "Am I really going to bed?" under Wine: bars from x 695, at y
        // 272 and 390.
        let labels = ["Go to sleep".to_string(), "Stay awake".to_string()];
        let b = Bars::new(&labels);
        assert_eq!((b.left, b.width, b.top), (695, 530, 272));
        assert_eq!(b.bar_top(1), 390);
        assert_eq!(b.hit(700, 300), Some(Hit::Bar(0)));
        assert_eq!(b.hit(700, 385), None);
        assert_eq!(b.hit(700, 300 + 118 * 2), None);
    }

    #[test]
    fn page_count() {
        assert_eq!(last_page(0), 0);
        assert_eq!(last_page(1), 0);
        assert_eq!(last_page(4), 0);
        assert_eq!(last_page(5), 1);
        assert_eq!(last_page(8), 1);
        assert_eq!(last_page(9), 2);
        assert_eq!(last_page(10), 2);
    }

    #[test]
    fn which_options_a_page_shows() {
        assert_eq!(page_range(10, 0), 0..4);
        assert_eq!(page_range(10, 1), 4..8);
        assert_eq!(page_range(10, 2), 8..10);
        assert_eq!(page_range(4, 0), 0..4);
        assert_eq!(page_range(3, 0), 0..3);
        // A page past the end shows nothing.
        assert_eq!(page_range(3, 1), 3..3);
        let l = labels(10);
        assert_eq!(Bars::on_page(&l, 2).range, 8..10);
        // Out-of-range pages clamp to the last one.
        assert_eq!(Bars::on_page(&l, 9).page, 2);
    }

    #[test]
    fn arrow_visibility() {
        // No paging for four or fewer.
        assert!(!prev_shown(4, 0) && !next_shown(4, 0));
        // Five: next only on the first page, prev only after it.
        assert!(!prev_shown(5, 0) && next_shown(5, 0));
        assert!(prev_shown(5, 1) && !next_shown(5, 1));
        // Ten: next on the first two pages, prev on the last two.
        assert!(!prev_shown(10, 0) && next_shown(10, 0));
        assert!(prev_shown(10, 1) && next_shown(10, 1));
        assert!(prev_shown(10, 2) && !next_shown(10, 2));
        assert_eq!(turn(10, 0, PREV), 0);
        assert_eq!(turn(10, 0, NEXT), 1);
        assert_eq!(turn(10, 2, NEXT), 2);
        assert_eq!(turn(10, 2, PREV), 1);
        // A bar id does not turn the page.
        assert_eq!(turn(10, 1, 0), 1);
    }

    #[test]
    fn paged_layout() {
        let l = labels(10);
        let first = Bars::on_page(&l, 0);
        assert_eq!(first.bar_top(0), 166);
        assert_eq!(first.bar_top(3), 166 + 3 * 118);
        // The next arrow sits under the last full bar: 630 + 8.
        assert_eq!(first.hit(ARROW_X + 5, 638 + 5), Some(Hit::Next));
        assert_eq!(first.hit(ARROW_X + 5, 99 + 5), None);
        let last = Bars::on_page(&l, 2);
        assert_eq!(last.bar_top(8), 166);
        assert_eq!(last.hit(ARROW_X + 5, 99 + 5), Some(Hit::Prev));
        assert_eq!(last.hit(ARROW_X + 5, 638 + 5), None);
        // Width is the same on every page: the widest label counts.
        let mut wide = labels(10);
        wide[9] = "x".repeat(40);
        assert_eq!(Bars::on_page(&wide, 0).width, Bars::on_page(&wide, 2).width);
        assert!(Bars::on_page(&wide, 2).width > Bars::on_page(&l, 2).width);
    }

    #[test]
    fn replay_gray_rule() {
        // Normal play: everything open, whatever the mark.
        for mark in 0..4 {
            assert!(status(0, mark).clickable && !status(0, mark).gray);
        }
        // Replay: only mark 1 is clickable; gray only at mark 0.
        assert_eq!(
            status(1, 0),
            Status {
                clickable: false,
                gray: true,
                mark: 0
            }
        );
        assert_eq!(
            status(1, 1),
            Status {
                clickable: true,
                gray: false,
                mark: 1
            }
        );
        assert_eq!(
            status(1, 2),
            Status {
                clickable: false,
                gray: false,
                mark: 2
            }
        );
        assert_eq!(
            status(1, 3),
            Status {
                clickable: false,
                gray: false,
                mark: 3
            }
        );
        // Any non-zero mode counts as replay.
        assert!(!status(2, 0).clickable && status(2, 0).gray);
    }
}
