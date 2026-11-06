//! What is on screen, as names and numbers. Images are decoded only when
//! the scene is rendered.

use std::collections::BTreeMap;

/// A standing character (game command 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    /// Body image; a 2048 x 2048 canvas in this game.
    pub body: String,
    /// Expression image drawn over the body, on the same canvas.
    pub face: Option<String>,
    /// Size in percent of the canvas (40, 50, 75, 80 in this game).
    pub scale: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scene {
    /// Background or event picture (game command 12, sub 0).
    pub background: Option<String>,
    /// Difference pictures drawn over the background, by slot (command 12,
    /// sub 1). Cleared when a new background is loaded.
    pub overlays: BTreeMap<i32, String>,
    /// Standing characters by slot 0..3 (command 13).
    pub characters: BTreeMap<i32, Character>,
    /// Images loaded into sprite layers (class Sprite, sub 1); the title
    /// screen uses these.
    pub layers: BTreeMap<i32, String>,
    /// Message window portrait: body and face (command 10). Tracked, not
    /// drawn yet.
    pub portrait: Option<(String, Option<String>)>,
    pub portrait_visible: bool,
    /// Current message text.
    pub text: Option<String>,
}
