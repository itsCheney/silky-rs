//! The screens of きまぐれテンプテーション that the script opens and waits
//! on (action menu, map, investigation, ...). Each answers the script
//! with a number, or closes without one.

use super::actsel::ActSelect;
use super::areasel::AreaSelect;
use super::itemsel::ItemSelect;
use super::map::Map;
use super::notebook::Notebook;
use super::omake::{Music, Omake};
use super::viewer::Viewer;
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
    /// The map of the apartment (`Game.43`).
    Map(Map),
    /// The 捜査手帳.
    Notebook(Notebook),
    /// The CG, scene and music modes (`Game.30` to `Game.32`).
    Omake(Box<Omake>),
    /// The アンネViewer (`Game.33`).
    Viewer(Box<Viewer>),
}

impl Screen {
    /// The element under (x, y), as an id for
    /// [`crate::ui::Button::Kimaten1`].
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        match self {
            Screen::Action(s) => s.hit(x, y),
            Screen::Area(s) => s.hit(x, y),
            Screen::Item(s) => s.hit(x, y),
            Screen::Map(m) => m.hit(x, y),
            Screen::Notebook(n) => n.hit(x, y),
            Screen::Omake(o) => o.hit(x, y),
            Screen::Viewer(v) => v.hit(x, y),
        }
    }

    /// Whether the element can be pressed (a disabled one shows grey).
    pub fn enabled(&self, id: i32) -> bool {
        match self {
            Screen::Action(s) => s.enabled(id),
            Screen::Area(s) => s.enabled(id),
            Screen::Item(s) => s.enabled(id),
            Screen::Map(m) => m.enabled(id),
            Screen::Notebook(n) => n.enabled(id),
            Screen::Omake(o) => o.enabled(id),
            Screen::Viewer(v) => v.enabled(id),
        }
    }

    /// A press on element `id`.
    pub fn click(&mut self, id: i32) -> Click {
        match self {
            Screen::Action(s) => s.click(id),
            Screen::Area(s) => s.click(id),
            Screen::Item(s) => s.click(id),
            Screen::Map(m) => m.click(id),
            Screen::Notebook(n) => n.click(id),
            Screen::Omake(o) => o.click(id),
            Screen::Viewer(v) => v.click(id),
        }
    }

    /// A right click or Escape: what the original does then (most screens
    /// ignore it).
    pub fn cancel(&mut self) -> Click {
        match self {
            Screen::Area(s) => s.cancel(),
            Screen::Map(m) => m.cancel(),
            Screen::Notebook(n) => n.cancel(),
            Screen::Omake(o) => o.cancel(),
            Screen::Viewer(v) => v.cancel(),
            _ => Click::None,
        }
    }

    /// Byte variables the screen changed, to write back to the script
    /// (taken once).
    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        match self {
            Screen::Action(s) => s.take_bytes(),
            Screen::Map(m) => m.take_bytes(),
            Screen::Notebook(n) => n.take_bytes(),
            Screen::Omake(o) => o.take_bytes(),
            Screen::Viewer(v) => v.take_bytes(),
            _ => Vec::new(),
        }
    }

    /// A sound effect to play for the last press or change (taken once).
    pub fn take_sound(&mut self) -> Option<String> {
        match self {
            Screen::Action(s) => s.take_sound(),
            Screen::Area(s) => s.take_sound(),
            Screen::Item(s) => s.take_sound(),
            Screen::Map(m) => m.take_sound(),
            Screen::Notebook(n) => n.take_sound(),
            Screen::Omake(_) => None,
            Screen::Viewer(v) => v.take_sound(),
        }
    }

    /// Draws the screen over the scene.
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        match self {
            Screen::Action(s) => s.draw(canvas, assets, look),
            Screen::Area(s) => s.draw(canvas, assets, look),
            Screen::Item(s) => s.draw(canvas, assets, look),
            Screen::Map(m) => m.draw(canvas, assets, look),
            Screen::Notebook(n) => n.draw(canvas, assets, look),
            Screen::Omake(o) => o.draw(canvas, assets, look),
            Screen::Viewer(v) => v.draw(canvas, assets, look),
        }
    }

    /// The press a headless run makes (`silky-run snap`), and a short name
    /// for the frame it saves.
    pub fn auto_pick(&self) -> (Option<i32>, &'static str) {
        match self {
            Screen::Action(s) => (s.auto_pick(), "actmenu"),
            Screen::Area(s) => (s.auto_pick(), "area"),
            Screen::Item(s) => (s.auto_pick(), "item"),
            Screen::Map(m) => m.auto_pick(),
            Screen::Notebook(n) => n.auto_pick(),
            Screen::Omake(o) => (o.auto_pick(), "omake"),
            Screen::Viewer(v) => (v.auto_pick(), "viewer"),
        }
    }

    /// A press or drag at (x, y) on element `id` (the bars of the music
    /// mode).
    pub fn press_at(&mut self, id: i32, x: i32, y: i32) {
        match self {
            Screen::Omake(o) => o.press_at(id, x, y),
            Screen::Viewer(v) => v.press_at(id, x, y),
            _ => {}
        }
    }

    /// The mouse wheel (`up` away from the player); true when the screen
    /// has something to scroll.
    pub fn wheel(&mut self, up: bool) -> bool {
        match self {
            Screen::Viewer(v) => v.wheel(up),
            _ => false,
        }
    }

    /// Puts the model the screen shows into the scene; true when the
    /// scene changed.
    pub fn sync_scene(&mut self, scene: &mut crate::scene::Scene) -> bool {
        match self {
            Screen::Viewer(v) => v.sync_scene(scene),
            _ => false,
        }
    }

    /// Gives the scene back when the screen closes.
    pub fn leave_scene(&mut self, scene: &mut crate::scene::Scene) {
        if let Screen::Viewer(v) = self {
            v.leave_scene(scene);
        }
    }

    /// The music the screen asks for (taken once).
    pub fn take_music(&mut self) -> Option<Music> {
        match self {
            Screen::Omake(o) => o.take_music(),
            _ => None,
        }
    }

    /// The music volume the screen set (taken once).
    pub fn take_volume(&mut self) -> Option<f32> {
        match self {
            Screen::Omake(o) => o.take_volume(),
            _ => None,
        }
    }
}
