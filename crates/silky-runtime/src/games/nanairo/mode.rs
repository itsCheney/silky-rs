//! What the game's own CG, scene and music screens (`Game.30` to `Game.32`)
//! have in common: the frame with the three tabs that slides in around
//! the page, the completion counter, and the loop that moves between the
//! three screens.
//!
//! The screens draw from their own sheet, and the top and bottom bars lie in
//! the same place on all three sheets. The frame is four pictures:
//!
//! - the tab strip (CG, music, scene) at the top left,
//! - the back and quit buttons at the bottom right,
//! - the banner with the screen's name at the top right,
//! - the bar along the bottom edge.
//!
//! They slide in from the sides when a screen opens and out when it is
//! left. A tab whose byte (0xbba for the CG, 0xbbb for the scenes, 0xbbc for
//! the music) is zero is grey and does nothing.

use std::sync::Arc;

use silky_format::akb::Image;

use crate::render::Canvas;

/// The three screens, numbered as the program's own loop numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Cg = 0,
    Music = 1,
    Scene = 2,
}

impl Mode {
    /// Position of the screen's tab in the strip, left to right.
    pub fn tab(self) -> usize {
        self as usize
    }

    /// The byte variable that opens the tab.
    pub fn tab_byte(self) -> usize {
        match self {
            Mode::Cg => 0xbba,
            Mode::Scene => 0xbbb,
            Mode::Music => 0xbbc,
        }
    }

    /// Area id of the tab.
    pub fn tab_id(self) -> i32 {
        TAB_FIRST + self.tab() as i32
    }

    /// What a screen answers when its tab is pressed: also the numbers
    /// the CG and scene screens answer to go to another screen.
    pub fn code(self) -> i32 {
        TAB_CODE + self.tab() as i32
    }
}

/// Area ids the three screens share: the tabs, back and quit.
pub const TAB_FIRST: i32 = 0xf00;
pub const BACK: i32 = 0xf03;
pub const QUIT: i32 = 0xf04;
/// The first tab code (CG), then music, then scene.
pub const TAB_CODE: i32 = 0x100;
/// What a screen answers to leave to the script with nothing picked.
pub const LEAVE: i32 = -1;
/// The sound of a press, and of a right click that leaves.
pub const OK_SOUND: &str = "ses_001.wav";
pub const CANCEL_SOUND: &str = "ses_002.wav";

/// Where the loop goes after a screen answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Another screen.
    Go(Mode),
    /// Back to the script with this value (-1, or the item picked).
    Leave(i32),
}

/// The mode loop (`Game.30` to `Game.32` share it): what to do with the
/// answer of screen `from`. The music screen answers a tab code, the CG
/// and scene screens answer their own tab codes and the item picked.
pub fn next(from: Mode, answer: i32) -> Next {
    let go = |m: Mode| (answer == m.code()).then_some(Next::Go(m));
    match from {
        Mode::Cg => go(Mode::Music).or_else(|| go(Mode::Scene)),
        Mode::Scene => go(Mode::Cg).or_else(|| go(Mode::Music)),
        Mode::Music => go(Mode::Cg).or_else(|| go(Mode::Scene)),
    }
    .unwrap_or(Next::Leave(if from == Mode::Music {
        LEAVE
    } else {
        answer
    }))
}

/// Frames of the frame's slide in and slide out, 16 ms each.
pub const SLIDE_IN_FRAMES: u64 = 14;
pub const SLIDE_OUT_FRAMES: u64 = 12;
pub const FRAME_MS: u64 = 16;

/// The four pictures of the frame: where they rest, where they slide in
/// from, and the part of the sheet each shows.
struct Piece {
    rest: (i32, i32),
    off: (i32, i32),
}

const TABS: Piece = Piece {
    rest: (25, 6),
    off: (-373, 6),
};
const BUTTONS: Piece = Piece {
    rest: (1077, 605),
    off: (1280, 605),
};
const BANNER: Piece = Piece {
    rest: (422, 13),
    off: (1280, 13),
};
const BAR: Piece = Piece {
    rest: (0, 665),
    off: (-1060, 665),
};

/// Where the frame is in its slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Sliding in, `frames` frames after it began.
    In(u64),
    /// At rest.
    Rest,
    /// Sliding out, `frames` frames after it began.
    Out(u64),
}

impl Phase {
    /// The phase `ms` after the screen opened. `leaving` is the time the
    /// slide out began. A screen that opens without the slide (the CG and
    /// scene screens do when they come back from a picture) passes
    /// `slide_in` false.
    pub fn at(ms: u64, leaving: Option<u64>, slide_in: bool) -> Self {
        if let Some(t) = leaving {
            return Phase::Out(ms.saturating_sub(t) / FRAME_MS);
        }
        let frames = ms / FRAME_MS;
        if slide_in && frames < SLIDE_IN_FRAMES {
            Phase::In(frames)
        } else {
            Phase::Rest
        }
    }

    /// How far out of its place the pictures are, as the program steps a
    /// move: tick `k` of `n` ticks, from the place at rest (`true`) or from
    /// the place off screen (`false`) toward the other.
    fn away(self) -> (bool, i32, i32) {
        match self {
            Phase::In(f) => (false, f.min(SLIDE_IN_FRAMES) as i32, SLIDE_IN_FRAMES as i32),
            Phase::Rest => (true, 0, 1),
            Phase::Out(f) => (
                true,
                f.min(SLIDE_OUT_FRAMES) as i32,
                SLIDE_OUT_FRAMES as i32,
            ),
        }
    }

    /// Opacity of what the screen draws on its page: it fades in over 10
    /// frames and out over 8.
    pub fn page_alpha(self, ms: u64) -> u8 {
        let ramp = |f: u64, n: u64| (f.min(n) * 255 / n) as u8;
        match self {
            Phase::In(_) => ramp(ms / FRAME_MS, 10),
            Phase::Rest => 255,
            Phase::Out(f) => 255 - ramp(f, 8),
        }
    }
}

fn place(p: &Piece, (from_rest, k, n): (bool, i32, i32)) -> (i32, i32) {
    // The program's step: start + (end - start) * k / n in whole numbers.
    let mix = |rest: i32, off: i32| {
        let (a, b) = if from_rest { (rest, off) } else { (off, rest) };
        a + (b - a) * k / n
    };
    (mix(p.rest.0, p.off.0), mix(p.rest.1, p.off.1))
}

/// What the frame is drawn from.
pub struct Frame<'a> {
    /// The screen's own sheet.
    pub sheet: &'a Arc<Image>,
    /// The screen the frame belongs to.
    pub mode: Mode,
    /// Whether each tab (in strip order) is open.
    pub open: [bool; 3],
    /// The area under the pointer and the one pressed.
    pub hover: Option<i32>,
    pub pressed: Option<i32>,
    pub phase: Phase,
}

/// Rows of the button cells: normal, hover, pressed, grey.
const CELL_W: u32 = 93;
const CELL_H: u32 = 111;
const CELL_TOP: u32 = 91;

impl Frame<'_> {
    fn state(&self, id: i32) -> u32 {
        if self.pressed == Some(id) {
            2
        } else if self.hover == Some(id) {
            1
        } else {
            0
        }
    }

    fn cell(&self, canvas: &mut Canvas, i: u32, state: u32, at: (i32, i32)) {
        canvas.draw_part(
            self.sheet,
            (CELL_W * i, CELL_TOP + CELL_H * state, CELL_W, CELL_H),
            at.0,
            at.1,
            false,
            255,
        );
    }

    /// Draws the four pictures.
    pub fn draw(&self, canvas: &mut Canvas) {
        let away = self.phase.away();
        let (x, y) = place(&BANNER, away);
        canvas.draw_part(self.sheet, (0, 0, 858, 60), x, y, false, 255);
        let (x, y) = place(&BAR, away);
        canvas.draw_part(self.sheet, (0, 60, 1060, 30), x, y, false, 255);
        let (x, y) = place(&TABS, away);
        for i in 0..3usize {
            let id = TAB_FIRST + i as i32;
            // The screen's own tab shows pressed; a closed one grey.
            let state = if i == self.mode.tab() {
                2
            } else if !self.open[i] {
                3
            } else {
                self.state(id)
            };
            self.cell(canvas, i as u32, state, (x + 140 * i as i32, y));
        }
        let (x, y) = place(&BUTTONS, away);
        for (k, id) in [BACK, QUIT].into_iter().enumerate() {
            self.cell(
                canvas,
                3 + k as u32,
                self.state(id),
                (x + 101 * k as i32, y),
            );
        }
    }
}

/// The completion in percent as the CG and scene screens show it: the
/// share of the pictures set, cut down, 100 when all are, never under 1.
pub fn percent(set: usize, total: usize) -> u32 {
    if total == 0 || set >= total {
        return 100;
    }
    ((set as f32 / total as f32 * 100.0) as u32).max(1)
}

/// Draws the completion counter on a CG or scene page: the label, the
/// number from the right (at least one digit) and the percent sign. The
/// cells are in the screen's sheet at y = 135.
pub fn draw_percent(canvas: &mut Canvas, sheet: &Arc<Image>, value: u32, alpha: u8) {
    canvas.draw_part(sheet, (733, 135, 93, 32), 1028, 82, false, alpha);
    let mut v = value;
    let mut x = 1191;
    loop {
        canvas.draw_part(
            sheet,
            (826 + 32 * (v % 10), 135, 32, 36),
            x,
            79,
            false,
            alpha,
        );
        v /= 10;
        x -= 32;
        if v == 0 {
            break;
        }
    }
    canvas.draw_part(sheet, (1146, 135, 33, 28), 1223, 87, false, alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_follows_the_table() {
        // The CG screen: -1 leaves, the tab codes go on, anything else is
        // the item picked.
        assert_eq!(next(Mode::Cg, -1), Next::Leave(-1));
        assert_eq!(next(Mode::Cg, 0x101), Next::Go(Mode::Music));
        assert_eq!(next(Mode::Cg, 0x102), Next::Go(Mode::Scene));
        assert_eq!(next(Mode::Cg, 42), Next::Leave(42));
        assert_eq!(next(Mode::Scene, 0x100), Next::Go(Mode::Cg));
        assert_eq!(next(Mode::Scene, 0x101), Next::Go(Mode::Music));
        assert_eq!(next(Mode::Scene, 77), Next::Leave(77));
        assert_eq!(next(Mode::Scene, -1), Next::Leave(-1));
        // The music screen only goes to the others or leaves with -1.
        assert_eq!(next(Mode::Music, 0x100), Next::Go(Mode::Cg));
        assert_eq!(next(Mode::Music, 0x102), Next::Go(Mode::Scene));
        assert_eq!(next(Mode::Music, -1), Next::Leave(-1));
    }

    #[test]
    fn tab_numbers() {
        assert_eq!(Mode::Cg.code(), 0x100);
        assert_eq!(Mode::Music.code(), 0x101);
        assert_eq!(Mode::Scene.code(), 0x102);
        assert_eq!(Mode::Music.tab_id(), 0xf01);
        assert_eq!(Mode::Cg.tab_byte(), 0xbba);
        assert_eq!(Mode::Scene.tab_byte(), 0xbbb);
        assert_eq!(Mode::Music.tab_byte(), 0xbbc);
    }

    #[test]
    fn slide_in_and_out() {
        assert_eq!(Phase::at(0, None, true), Phase::In(0));
        assert_eq!(place(&TABS, Phase::In(0).away()), (-373, 6));
        assert_eq!(place(&BUTTONS, Phase::In(0).away()), (1280, 605));
        assert_eq!(Phase::at(13 * 16, None, true), Phase::In(13));
        assert_eq!(Phase::at(14 * 16, None, true), Phase::Rest);
        assert_eq!(place(&BAR, Phase::Rest.away()), (0, 665));
        assert_eq!(place(&BANNER, Phase::Rest.away()), (422, 13));
        // Without the slide the frame is at rest from the start.
        assert_eq!(Phase::at(0, None, false), Phase::Rest);
        // Out: off screen after 12 frames.
        let out = Phase::at(500 + 12 * 16, Some(500), true);
        assert_eq!(out, Phase::Out(12));
        assert_eq!(place(&TABS, out.away()), (-373, 6));
        assert_eq!(place(&BAR, Phase::Out(6).away()), (-530, 665));
    }

    #[test]
    fn page_fades() {
        assert_eq!(Phase::In(0).page_alpha(0), 0);
        assert_eq!(Phase::In(5).page_alpha(5 * 16), 127);
        assert_eq!(Phase::In(12).page_alpha(12 * 16), 255);
        assert_eq!(Phase::Rest.page_alpha(1000), 255);
        assert_eq!(Phase::Out(8).page_alpha(0), 0);
    }

    #[test]
    fn completion() {
        assert_eq!(percent(0, 644), 1);
        assert_eq!(percent(644, 644), 100);
        assert_eq!(percent(322, 644), 50);
        assert_eq!(percent(1, 78), 1);
        assert_eq!(percent(77, 78), 98);
    }
}
