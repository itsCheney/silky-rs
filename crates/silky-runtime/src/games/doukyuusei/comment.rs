//! The comments page of 同級生 Remake (`Game.53`), the fourth page of the
//! extras: messages from the artists.
//!
//! Read from the executable. Eight thumbnails (`comment_p.akb`, four
//! states each) pick the picture shown (`comment00..07.akb`); the first is
//! always open, the others when their byte variable is 1. Clicking an open
//! thumbnail shows its picture, a locked one does nothing.
//!
//! The side tabs answer 100 with global 12 naming the next page of the
//! extras (globals 16 and 17 cleared); back answers -1, as does a
//! secondary click. See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::area::{self, Area};

use super::album;
use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

pub use super::album::{BACK, LEAVE, TAB, TO_PAGE};

/// Globals the comments page writes.
pub mod globals {
    pub const PAGE_OF_EXTRAS: usize = 12;
    pub const RETURNING: usize = 16;
    pub const PAGE: usize = 17;
}

pub const COMMENTS: usize = 8;

/// Byte variable of comments 1..7 (the first is always open).
const FLAGS: [usize; COMMENTS - 1] = [3479, 3481, 3482, 3480, 3477, 3475, 3476];

/// File number of each comment's picture.
const FILES: [usize; COMMENTS] = [0, 1, 4, 2, 3, 6, 5, 7];

/// Left edge of a thumbnail and the space between two.
const THUMB_X: i32 = 991;
const THUMB_STEP: i32 = 110;

#[derive(Debug, Clone)]
pub struct Comment {
    /// With the update archive's pages (the scene tab).
    pub r18: bool,
    /// The comment shown.
    pub selected: usize,
    open: [bool; COMMENTS],
    areas: Arc<Vec<Area>>,
    set: Vec<(usize, i32)>,
}

impl Comment {
    /// `bytes` are the byte variables.
    pub fn load(assets: &Assets, bytes: &[u8]) -> Result<Self> {
        let areas = area::parse(&assets.data("commentmode.area")?)?;
        let mut c = Self {
            r18: album::r18(assets),
            selected: 0,
            open: [false; COMMENTS],
            areas: Arc::new(areas),
            set: Vec::new(),
        };
        c.refresh(bytes);
        Ok(c)
    }

    /// Reads which comments are open.
    pub fn refresh(&mut self, bytes: &[u8]) {
        self.open =
            std::array::from_fn(|i| i == 0 || bytes.get(FLAGS[i - 1]).copied().unwrap_or(0) == 1);
    }

    pub fn take_globals(&mut self) -> Vec<(usize, i32)> {
        std::mem::take(&mut self.set)
    }

    /// The tabs there are: the fourth only with the update archive.
    fn tabs(&self) -> i32 {
        if self.r18 {
            4
        } else {
            3
        }
    }

    /// The tab of this page.
    fn current_tab(&self) -> i32 {
        if self.r18 {
            3
        } else {
            2
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        self.areas
            .iter()
            .filter(|a| match a.id >> 8 {
                0 => (a.id as usize) < COMMENTS,
                1 => (a.id & 0xff) < self.tabs(),
                _ => true,
            })
            .find(|a| a.contains(x, y))
            .map(|a| a.id)
    }

    pub fn click(&mut self, id: i32) -> Click {
        let (kind, i) = (id >> 8, (id & 0xff) as usize);
        match kind {
            0 if i < COMMENTS => {
                if self.open[i] {
                    self.selected = i;
                }
                Click::None
            }
            1 if (i as i32) < self.tabs() && i as i32 != self.current_tab() => {
                let page = match (i, self.r18) {
                    (0, _) => 0,
                    (1, true) => 1,
                    _ => 2,
                };
                self.set = vec![
                    (globals::RETURNING, 0),
                    (globals::PAGE, 0),
                    (globals::PAGE_OF_EXTRAS, page),
                ];
                Click::Result(TO_PAGE)
            }
            2 => Click::Result(LEAVE),
            _ => Click::None,
        }
    }

    /// The secondary click leaves.
    pub fn cancel(&mut self) -> Option<i32> {
        Some(LEAVE)
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed) of an id.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let parts = assets.localized_image("omake_p.akb", language)?;
        let message =
            assets.localized_image(&format!("comment{:02}.akb", FILES[self.selected]), language)?;
        canvas.draw_part(&message, (0, 0, 1599, 1069), 286, 0, false, 255);
        canvas.draw_part(&parts, (0, 204, 1920, 68), 0, 0, false, 255);

        let thumbs = assets.localized_image("comment_p.akb", language)?;
        for i in 0..COMMENTS {
            let row = if !self.open[i] {
                3
            } else if i == self.selected {
                0
            } else {
                state(i as i32).min(2)
            };
            canvas.draw_part(
                &thumbs,
                (i as u32 * 106, row * 110, 106, 110),
                THUMB_X + i as i32 * THUMB_STEP,
                968,
                false,
                255,
            );
        }
        for i in 0..self.tabs() {
            let s = if i == self.current_tab() {
                2
            } else {
                state(TAB | i).min(1)
            };
            let sx = if self.r18 {
                i as u32 * 237
            } else {
                [0, 474, 711][i as usize]
            };
            canvas.draw_part(
                &parts,
                (sx, s * 163 + 272, 237, 163),
                0,
                i * 170 + 132,
                false,
                255,
            );
        }
        let s = state(BACK).min(2);
        canvas.draw_part(&parts, (948, s * 123 + 272, 218, 123), 10, 953, false, 255);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(r18: bool) -> Comment {
        let mut c = Comment {
            r18,
            selected: 0,
            open: [false; COMMENTS],
            areas: Arc::new(Vec::new()),
            set: Vec::new(),
        };
        c.refresh(&[]);
        c
    }

    #[test]
    fn select() {
        let mut c = comment(true);
        assert!(matches!(c.click(1), Click::None));
        assert_eq!(c.selected, 0);
        let mut bytes = vec![0u8; 5000];
        bytes[FLAGS[1]] = 1;
        c.refresh(&bytes);
        c.click(2);
        assert_eq!(c.selected, 2);
        c.click(3);
        assert_eq!(c.selected, 2);
        c.click(0);
        assert_eq!(c.selected, 0);
    }

    #[test]
    fn tabs() {
        let mut c = comment(true);
        assert!(matches!(c.click(TAB | 3), Click::None));
        assert!(matches!(c.click(TAB | 1), Click::Result(TO_PAGE)));
        let set = c.take_globals();
        assert!(set.contains(&(globals::PAGE_OF_EXTRAS, 1)) && set.contains(&(globals::PAGE, 0)));
        assert!(matches!(c.click(TAB | 2), Click::Result(TO_PAGE)));
        assert!(c.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 2)));
        assert!(matches!(c.click(BACK), Click::Result(LEAVE)));
        let mut c = comment(false);
        assert!(matches!(c.click(TAB | 3), Click::None));
        assert!(matches!(c.click(TAB | 1), Click::Result(TO_PAGE)));
        assert!(c.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 2)));
        assert_eq!(c.cancel(), Some(LEAVE));
    }
}
