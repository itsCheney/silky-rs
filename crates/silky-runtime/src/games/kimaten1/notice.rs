//! The notices of きまぐれテンプテーション (`Game.44`): small pink strips
//! such as 「捜査手帳」が解放されました that slide in from the right edge
//! while the script goes on. Nothing waits on them unless the script polls
//! for the end (sub 4).
//!
//! The script queues strings (sub 0), shows the queue (sub 1), and clears
//! the queue (sub 3) or the strips that are up (sub 2). Each strip slides
//! in, stays four seconds, and slides out; the next one of a batch starts
//! when the one before has arrived, and the older ones step down to make
//! room.

use crate::assets::Assets;
use crate::render::Canvas;
use crate::text::TextRenderer;
use crate::Result;

/// Screen width: strips start and end here.
const EDGE: i32 = 1280;
/// Top of the newest strip, and the step down for the older ones.
const TOP: i32 = 32;
const STACK_STEP: i32 = 50;
/// Height of a strip; the cap and the repeated middle piece in
/// `message_p.akb`, left to right.
const HEIGHT: u32 = 48;
const CAP: (u32, u32, u32, u32) = (90, 680, 44, HEIGHT);
const MIDDLE: (u32, u32, u32, u32) = (134, 680, 24, HEIGHT);
/// A full-width character is 20 pixels, and the strip is 68 wider than its
/// text cells.
const CELL: i32 = 20;
const PADDING: i32 = 68;
const TEXT_LEFT: i32 = 44;
const TEXT_TOP: i32 = 14;
/// How long a strip stays after it starts, in ms.
const STAY_MS: u64 = 4000;
/// The strips move once per frame.
const FRAMES_PER_SEC: u64 = 60;
/// Steps a strip pushed down takes per frame.
const DROP_STEP: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Queued behind the one before.
    Waiting,
    /// Sliding in.
    In,
    /// Resting.
    Rest,
    /// Sliding out.
    Out,
    /// Stepping down under a newer strip.
    Drop,
    /// Gone off the edge.
    Gone,
}

#[derive(Debug, Clone)]
struct Toast {
    text: String,
    /// Half the width of the longest string of the batch, in cells.
    cells: i32,
    /// Cells of this string.
    own: i32,
    x: i32,
    y: i32,
    target_y: i32,
    phase: Phase,
    /// 1 when it has just arrived, 2 once a newer one pushed it down.
    arrived: u8,
    started: u64,
    pushed: bool,
}

impl Toast {
    fn width(&self) -> i32 {
        self.cells * CELL + PADDING
    }

    /// One frame of the strip's own movement.
    fn step(&mut self, now: u64) {
        match self.phase {
            Phase::In => {
                self.x -= self.width() / 12;
                if self.x <= EDGE - self.width() {
                    self.x = EDGE - self.width();
                    self.phase = Phase::Rest;
                    self.arrived = 1;
                }
            }
            Phase::Rest => {
                if self.pushed {
                    self.pushed = false;
                    self.phase = Phase::Drop;
                } else if self.started + STAY_MS <= now {
                    self.phase = Phase::Out;
                }
            }
            Phase::Out => {
                self.x += self.width() / 12;
                if self.x > EDGE - 1 {
                    self.x = EDGE;
                    self.phase = Phase::Gone;
                }
            }
            Phase::Drop => {
                self.y += DROP_STEP;
                if self.target_y <= self.y {
                    self.y = self.target_y;
                    self.phase = Phase::Rest;
                }
            }
            Phase::Waiting | Phase::Gone => {}
        }
    }
}

/// Size of a string in Shift_JIS bytes, which is how the original counts
/// its cells (two bytes per full-width character).
fn sjis_len(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if (c as u32) < 0x80 || ('\u{ff61}'..='\u{ff9f}').contains(&c) {
                1
            } else {
                2
            }
        })
        .sum()
}

/// The queue of notices and the strips that are up.
#[derive(Debug, Clone, Default)]
pub struct Notices {
    queue: Vec<String>,
    toasts: Vec<Toast>,
    active: bool,
    /// Animation clock in ms at the last [`Notices::advance`].
    clock: u64,
}

impl Notices {
    /// Sub 0: queues a string.
    pub fn push(&mut self, text: &str) {
        self.queue.push(text.to_string());
    }

    /// Sub 3: forgets the queued strings (the strips that are up stay).
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Sub 2: takes the strips down at once and clears the queue.
    pub fn clear(&mut self) {
        self.toasts.clear();
        self.active = false;
        self.queue.clear();
    }

    /// Sub 4: whether strips are still up.
    pub fn busy(&self) -> bool {
        self.active
    }

    /// Whether there is anything to draw.
    pub fn visible(&self) -> bool {
        !self.toasts.is_empty()
    }

    /// Sub 1: makes a strip of each queued string, all as wide as the
    /// longest. The first starts if none was up.
    pub fn show(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let longest = self.queue.iter().map(|s| sjis_len(s)).max().unwrap_or(0) as i32;
        let cells = longest / 2;
        for text in &self.queue {
            self.toasts.push(Toast {
                own: sjis_len(text) as i32 / 2,
                text: text.clone(),
                cells,
                x: EDGE,
                y: TOP,
                target_y: TOP,
                phase: Phase::Waiting,
                arrived: 0,
                started: 0,
                pushed: false,
            });
        }
        if !self.active {
            let t = &mut self.toasts[0];
            t.phase = Phase::In;
            t.arrived = 0;
            t.started = self.clock;
        }
        self.active = true;
    }

    /// Moves on to the animation clock `clock` (ms), a step a frame.
    /// Returns whether anything moved.
    pub fn advance(&mut self, clock: u64) -> bool {
        let frames =
            (clock * FRAMES_PER_SEC / 1000).saturating_sub(self.clock * FRAMES_PER_SEC / 1000);
        self.clock = self.clock.max(clock);
        if self.toasts.is_empty() || frames == 0 {
            return false;
        }
        for _ in 0..frames {
            self.frame();
        }
        true
    }

    fn frame(&mut self) {
        let now = self.clock;
        let mut before = 0u8;
        for k in 0..self.toasts.len() {
            if before == 1 {
                // The one before has arrived: this one starts, and the
                // older ones step down.
                let t = &mut self.toasts[k];
                t.phase = Phase::In;
                t.arrived = 0;
                t.started = now;
                for j in 0..k {
                    let o = &mut self.toasts[j];
                    o.target_y += STACK_STEP;
                    o.pushed = true;
                    o.arrived = 2;
                }
            }
            self.toasts[k].step(now);
            before = self.toasts[k].arrived;
        }
        self.toasts.retain(|t| t.phase != Phase::Gone);
        if self.toasts.is_empty() {
            self.active = false;
        }
    }

    /// Draws the strips that are up.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        if self.toasts.is_empty() {
            return Ok(());
        }
        let sheet = assets.image("message_p.akb")?;
        for t in &self.toasts {
            if t.phase == Phase::Waiting {
                continue;
            }
            canvas.draw_part(&sheet, CAP, t.x, t.y, false, 255);
            for i in 0..=t.cells {
                canvas.draw_part(&sheet, MIDDLE, t.x + TEXT_LEFT + i * CELL, t.y, false, 255);
            }
            if let Some(tr) = text {
                // Shorter strings sit toward the right edge of the strip.
                let left = t.x + TEXT_LEFT + (t.cells - t.own) * CELL;
                let top = t.y + TEXT_TOP;
                tr.draw_plain(
                    canvas,
                    &t.text,
                    left as f32,
                    top as f32 + 16.0,
                    CELL as f32,
                    [255, 255, 255, 255],
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_shift_jis_cells() {
        assert_eq!(sjis_len("「捜査手帳」が解放されました") / 2, 14);
        assert_eq!(sjis_len("ab"), 2);
    }

    #[test]
    fn strip_slides_in_waits_and_leaves() {
        let mut n = Notices::default();
        n.push("「捜査手帳」が解放されました");
        n.show();
        assert!(n.busy() && n.visible());
        // 14 cells: 348 wide, 29 a frame; in after 12 frames.
        let mut clock = 0;
        for _ in 0..13 {
            clock += 17;
            n.advance(clock);
        }
        assert_eq!(n.toasts[0].x, EDGE - 348);
        n.advance(4500);
        n.advance(5500);
        assert!(!n.busy() && !n.visible());
    }

    #[test]
    fn second_strip_pushes_the_first_down() {
        let mut n = Notices::default();
        n.push("あいう");
        n.push("えお");
        n.show();
        for f in 1..=80 {
            n.advance(f * 1000 / 60);
        }
        assert_eq!(n.toasts.len(), 2);
        assert_eq!(n.toasts[0].y, TOP + STACK_STEP);
        assert_eq!(n.toasts[1].y, TOP);
    }
}
