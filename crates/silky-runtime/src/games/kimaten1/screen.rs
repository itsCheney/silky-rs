//! The screens of きまぐれテンプテーション that the script opens and waits
//! on (action menu, map, investigation, ...). Each answers the script
//! with a number, or closes without one.

use super::actsel::ActSelect;
use super::areasel::AreaSelect;
use super::itemsel::ItemSelect;
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
    /// Stay up and do what this message window button does (save, load,
    /// settings, backlog, ...).
    Bar(i32),
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
    /// The action menu (`Game.42`).
    Action(ActSelect),
    /// Investigation over the background (`Game.41`).
    Area(AreaSelect),
    /// Choosing a piece of evidence (`Game.46`).
    Item(ItemSelect),
}

impl Screen {
    /// The element under (x, y), as an id for
    /// [`crate::ui::Button::Kimaten1`].
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        match self {
            Screen::Action(s) => s.hit(x, y),
            Screen::Area(s) => s.hit(x, y),
            Screen::Item(s) => s.hit(x, y),
        }
    }

    /// Whether the element can be pressed (a disabled one shows grey).
    pub fn enabled(&self, id: i32) -> bool {
        match self {
            Screen::Action(s) => s.enabled(id),
            Screen::Area(s) => s.enabled(id),
            Screen::Item(s) => s.enabled(id),
        }
    }

    /// A press on element `id`.
    pub fn click(&mut self, id: i32) -> Click {
        match self {
            Screen::Action(s) => s.click(id),
            Screen::Area(s) => s.click(id),
            Screen::Item(s) => s.click(id),
        }
    }

    /// A right click or Escape: what the original does then (most screens
    /// ignore it).
    pub fn cancel(&mut self) -> Click {
        match self {
            Screen::Area(s) => s.cancel(),
            _ => Click::None,
        }
    }

    /// Byte variables the screen changed, to write back to the script
    /// (taken once).
    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        match self {
            Screen::Action(s) => s.take_bytes(),
            _ => Vec::new(),
        }
    }

    /// A sound effect to play for the last press or change (taken once).
    pub fn take_sound(&mut self) -> Option<String> {
        match self {
            Screen::Action(s) => s.take_sound(),
            Screen::Area(s) => s.take_sound(),
            Screen::Item(s) => s.take_sound(),
        }
    }

    /// Draws the screen over the scene.
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        match self {
            Screen::Action(s) => s.draw(canvas, assets, look),
            Screen::Area(s) => s.draw(canvas, assets, look),
            Screen::Item(s) => s.draw(canvas, assets, look),
        }
    }

    /// The press a headless run makes (`silky-run snap`), and a short name
    /// for the frame it saves.
    pub fn auto_pick(&self) -> (Option<i32>, &'static str) {
        match self {
            Screen::Action(s) => (s.auto_pick(), "actmenu"),
            Screen::Area(s) => (s.auto_pick(), "area"),
            Screen::Item(s) => (s.auto_pick(), "item"),
        }
    }
}
