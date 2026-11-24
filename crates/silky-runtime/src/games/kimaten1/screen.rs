//! The screens of きまぐれテンプテーション that the script opens and waits
//! on (action menu, map, investigation, ...). Each answers the script
//! with a number, or closes without one.

use crate::assets::Assets;
use crate::render::Canvas;
use crate::text::TextRenderer;
use crate::Result;

/// What a press on a screen did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// Nothing to answer yet (the screen may have changed).
    None,
    /// Close and push this value for the script.
    Result(i32),
    /// Close without a value (the call returns nothing).
    Close,
}

/// What a screen draws with: the pointer's element, the pressed one, the
/// time in ms since the screen opened, and the text renderer if any.
pub struct Look<'a> {
    pub hover: Option<i32>,
    pub pressed: Option<i32>,
    pub ms: u64,
    pub text: Option<&'a TextRenderer>,
}

/// A screen that is up.
#[derive(Debug, Clone)]
pub enum Screen {
    /// Placeholder until the first screen is added.
    #[doc(hidden)]
    Empty,
}

impl Screen {
    /// The element under (x, y), as an id for
    /// [`crate::ui::Button::Kimaten1`].
    pub fn hit(&self, _x: i32, _y: i32) -> Option<i32> {
        match self {
            Screen::Empty => None,
        }
    }

    /// Whether the element can be pressed (a disabled one shows grey).
    pub fn enabled(&self, _id: i32) -> bool {
        true
    }

    /// A press on element `id`.
    pub fn click(&mut self, _id: i32) -> Click {
        match self {
            Screen::Empty => Click::Result(0),
        }
    }

    /// A right click or Escape: what the original does then (most screens
    /// ignore it).
    pub fn cancel(&mut self) -> Click {
        Click::None
    }

    /// Byte variables the screen changed, to write back to the script
    /// (taken once).
    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        Vec::new()
    }

    /// A sound effect to play for the last press or change (taken once).
    pub fn take_sound(&mut self) -> Option<String> {
        None
    }

    /// Draws the screen over the scene.
    pub fn draw(&self, _canvas: &mut Canvas, _assets: &mut Assets, _look: &Look) -> Result<()> {
        Ok(())
    }

    /// The press a headless run makes (`silky-run snap`), and a short name
    /// for the frame it saves.
    pub fn auto_pick(&self) -> (Option<i32>, &'static str) {
        match self {
            Screen::Empty => (None, "screen"),
        }
    }
}
