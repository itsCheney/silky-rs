//! The game's own mouse pointer of 同級生 Remake, read from the
//! executable.
//!
//! The pointer is a 48 x 48 sprite on top of the screen that follows the
//! system pointer; while it is up the system's pointer is hidden. It is
//! made of `curo_p.akb` (96 x 720: two frames side by side, a row per
//! picture): row 1 is the magnifier of look, then a face for talk, a fist
//! for hit, hands for push, open and take, lips for kiss, hands for touch
//! and feel, a tongue, a mouth for bite, the "XXX" face (row 12) and the
//! "?" (row 13). Row 0, the arrow, is never drawn: picture 0 means the
//! system's pointer.
//!
//! The room's and the plain area selection's hover picks it: a hot spot
//! whose verb (the high byte of its id) is 0 to 12 sets picture verb + 1,
//! anything else (a button, nothing) sets 0, which takes the sprite away
//! and brings the system's pointer back. Nothing else sets a picture.
//! `Game.27(1)` takes it away until the hover changes; `Game.27(0)` puts
//! up nothing by itself. The two frames swap every 300 ms. The point of
//! the picture is its middle (24, 24), for the magnifier 20 in from the
//! top left. Outside the screen the sprite is away and the system's
//! pointer is back.

use std::sync::Arc;

use silky_format::akb::Image;

use crate::canvas::Canvas;

pub const SHEET: &str = "curo_p.akb";
/// Size of a picture.
pub const CELL: u32 = 48;
/// Time of one frame.
pub const FRAME_MS: u64 = 300;
/// Highest verb that has a picture (12, the "?").
pub const LAST_VERB: i32 = 12;

/// The picture a hot spot id (verb in the high byte) sets: verb + 1, or 0
/// for none (the system's pointer).
pub fn picture_for_id(id: i32) -> u8 {
    let verb = id >> 8;
    if (0..=LAST_VERB).contains(&verb) {
        verb as u8 + 1
    } else {
        0
    }
}

/// How far the picture's point is in from its top left.
pub fn hot_spot(picture: u8) -> i32 {
    match picture {
        1 => 20,
        _ => 24,
    }
}

/// Whether a screen point is where the sprite shows (the program tests
/// x up to 1979 and y below 1080).
pub fn inside(x: i32, y: i32) -> bool {
    (0..=1979).contains(&x) && (0..1080).contains(&y)
}

/// The state of the pointer: which picture is up (0 for none) and since
/// when, on the session clock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cursor {
    /// The picture the hover asks for, last time it changed.
    want: u8,
    picture: u8,
    since: u64,
}

impl Cursor {
    /// The hover now asks for `want`. Does something only when it differs
    /// from last time, as the program compares; returns whether it did.
    pub fn sync(&mut self, want: u8, now: u64) -> bool {
        if want == self.want {
            return false;
        }
        self.want = want;
        self.picture = want;
        self.since = now;
        true
    }

    /// `Game.27(1)`: away until the hover changes.
    pub fn hide(&mut self) {
        self.picture = 0;
    }

    /// The picture up, 0 for none.
    pub fn picture(&self) -> u8 {
        self.picture
    }

    /// Whether the sprite is up, and the system's pointer hidden.
    pub fn shown(&self) -> bool {
        self.picture != 0
    }

    /// The frame (0 or 1) at `now`: the first when set, then one every
    /// 300 ms.
    pub fn frame(&self, now: u64) -> u32 {
        ((now.saturating_sub(self.since) / FRAME_MS) % 2) as u32
    }

    /// Draws the sprite at the pointer (screen pixels) on `canvas`.
    pub fn draw(&self, canvas: &mut Canvas, sheet: &Arc<Image>, pointer: (i32, i32), now: u64) {
        if !self.shown() || !inside(pointer.0, pointer.1) {
            return;
        }
        let hot = hot_spot(self.picture);
        canvas.draw_part(
            sheet,
            (
                self.frame(now) * CELL,
                self.picture as u32 * CELL,
                CELL,
                CELL,
            ),
            pointer.0 - hot,
            pointer.1 - hot,
            false,
            255,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbs_pick_pictures() {
        // Look is the magnifier (row 1), "?" row 13.
        assert_eq!(picture_for_id(0x0001), 1);
        assert_eq!(picture_for_id(0x0105), 2);
        assert_eq!(picture_for_id(0x0c03), 13);
        // A button (verb 255) and anything past 12 take the system's.
        assert_eq!(picture_for_id(0xff00), 0);
        assert_eq!(picture_for_id(0x0d00), 0);
    }

    #[test]
    fn hover_sets_and_hide_holds_until_it_changes() {
        let mut c = Cursor::default();
        assert!(!c.shown());
        assert!(!c.sync(0, 10));
        assert!(c.sync(3, 100));
        assert!(c.shown());
        assert_eq!(c.picture(), 3);
        // Same hover again: nothing.
        assert!(!c.sync(3, 200));
        c.hide();
        assert!(!c.shown());
        assert!(!c.sync(3, 300));
        assert!(!c.shown());
        // A new hover brings it back.
        assert!(c.sync(2, 400));
        assert_eq!(c.picture(), 2);
        assert!(c.sync(0, 500));
        assert!(!c.shown());
    }

    #[test]
    fn frames_swap_every_300_ms_from_the_set() {
        let mut c = Cursor::default();
        c.sync(2, 1000);
        assert_eq!(c.frame(1000), 0);
        assert_eq!(c.frame(1299), 0);
        assert_eq!(c.frame(1300), 1);
        assert_eq!(c.frame(1599), 1);
        assert_eq!(c.frame(1600), 0);
    }

    #[test]
    fn points_and_screen() {
        assert_eq!(hot_spot(1), 20);
        assert_eq!(hot_spot(2), 24);
        assert_eq!(hot_spot(13), 24);
        assert!(inside(0, 0));
        assert!(inside(1979, 1079));
        assert!(!inside(1980, 10));
        assert!(!inside(-1, 10));
        assert!(!inside(10, 1080));
    }
}
