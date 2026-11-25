//! The fade of the game's own selection screens (action menu, investigation,
//! evidence choice, option list).
//!
//! The original fades such a screen in over 12 frames when it opens and out
//! over 12 once it has its answer, each inside a call that waits for the fade
//! to end. The screen's own input handling does not run meanwhile, so
//! nothing the player does in that time reaches it. The answer goes to the
//! script after the fade out, and the pressed button is drawn at rest again
//! before the fade begins.

use super::map::{fade_alpha, faded, FADE_MS};
use super::screen::{Click, Look};
use crate::render::Canvas;
use crate::Result;

/// Where the fade out is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Leave {
    #[default]
    No,
    /// Answered; the fade begins when the clock is next seen.
    Pending(Click),
    At(Click, u64),
}

#[derive(Debug, Clone, Default)]
pub struct Fade {
    leave: Leave,
    /// The right click put the screen's buttons away (the picture behind
    /// stays); the next one brings them back.
    hidden: bool,
}

impl Fade {
    /// Takes a press's answer: one that leaves the screen starts the fade
    /// out and is given on when it is over.
    pub fn begin(&mut self, click: Click) -> Click {
        match click {
            Click::Result(_) | Click::Close => {
                if self.leave != Leave::No {
                    return Click::None;
                }
                self.leave = Leave::Pending(click);
                Click::None
            }
            c => c,
        }
    }

    /// Whether the buttons are put away.
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    /// Puts the buttons away, or brings them back.
    pub fn toggle_hidden(&mut self) {
        self.hidden = !self.hidden;
    }

    /// Whether the fade out has begun (the screen takes no more input).
    pub fn leaving(&self) -> bool {
        self.leave != Leave::No
    }

    /// Whether a fade is running `ms` after the screen opened.
    pub fn animating(&self, ms: u64) -> bool {
        self.leaving() || ms < FADE_MS
    }

    /// Runs the fade out; the answer comes once it is over.
    pub fn tick(&mut self, ms: u64) -> Option<Click> {
        match self.leave {
            Leave::No => None,
            Leave::Pending(c) => {
                self.leave = Leave::At(c, ms);
                None
            }
            Leave::At(c, t0) => (ms >= t0 + FADE_MS).then_some(c),
        }
    }

    /// A run without time ends the fade out at once.
    pub fn settle(&mut self) -> Option<Click> {
        match self.leave {
            Leave::No => None,
            Leave::Pending(c) | Leave::At(c, _) => Some(c),
        }
    }

    /// Opacity of the screen `ms` after it opened.
    pub fn alpha(&self, ms: u64) -> u8 {
        match self.leave {
            Leave::No => fade_alpha(ms, false),
            Leave::Pending(_) => 255,
            Leave::At(_, t0) => fade_alpha(ms.saturating_sub(t0), true),
        }
    }

    /// Draws a screen with `draw`, faded. Once the fade out has begun the
    /// pointer's button is drawn at rest.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        look: &Look,
        draw: impl FnOnce(&mut Canvas, &Look) -> Result<()>,
    ) -> Result<()> {
        if self.hidden {
            return Ok(());
        }
        let resting = Look {
            hover: None,
            pressed: None,
            ..*look
        };
        let look = if self.leaving() { &resting } else { look };
        faded(canvas, self.alpha(look.ms), |canvas| draw(canvas, look))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_waits_for_the_fade_out() {
        let mut f = Fade::default();
        assert_eq!(f.begin(Click::Bar(3)), Click::Bar(3));
        assert!(!f.leaving());
        assert_eq!(f.begin(Click::Result(2)), Click::None);
        assert!(f.leaving());
        // A second press while leaving changes nothing.
        assert_eq!(f.begin(Click::Result(5)), Click::None);
        assert_eq!(f.tick(5000), None);
        assert_eq!(f.tick(5000 + FADE_MS - 1), None);
        assert_eq!(f.tick(5000 + FADE_MS), Some(Click::Result(2)));
    }

    #[test]
    fn a_run_without_time_answers_at_once() {
        let mut f = Fade::default();
        assert_eq!(f.settle(), None);
        f.begin(Click::Close);
        assert_eq!(f.settle(), Some(Click::Close));
    }

    #[test]
    fn hidden_buttons_draw_nothing() {
        let mut f = Fade::default();
        assert!(!f.hidden());
        f.toggle_hidden();
        assert!(f.hidden());
        let mut canvas = Canvas::new(4, 4);
        let look = Look {
            hover: None,
            pressed: None,
            ms: 1000,
            text: None,
        };
        let mut drew = false;
        f.draw(&mut canvas, &look, |_, _| {
            drew = true;
            Ok(())
        })
        .unwrap();
        assert!(!drew);
        f.toggle_hidden();
        f.draw(&mut canvas, &look, |_, _| {
            drew = true;
            Ok(())
        })
        .unwrap();
        assert!(drew);
    }

    #[test]
    fn it_fades_in_for_twelve_frames() {
        let f = Fade::default();
        assert!(f.animating(0));
        assert!(f.animating(FADE_MS - 1));
        assert!(!f.animating(FADE_MS));
        assert_eq!(f.alpha(0), 21);
        assert_eq!(f.alpha(FADE_MS), 255);
    }
}
