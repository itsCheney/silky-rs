//! What is on screen, as names and numbers. Images are decoded only when
//! the scene is rendered.

use std::collections::BTreeMap;

/// A standing character (game command 13).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Character {
    /// Body image; a 2048 x 2048 canvas in this game.
    pub body: String,
    /// Expression image drawn over the body, on the same canvas.
    pub face: Option<String>,
    /// Size in percent of the canvas (40, 50, 75, 80 in this game).
    pub scale: i32,
    /// Shift from the slot position in pixels (command 13, sub 14).
    #[serde(default)]
    pub offset: (i32, i32),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Scene {
    /// Background or event picture (game command 12, sub 0).
    pub background: Option<String>,
    /// Position of the background's top left corner (command 12, sub 4),
    /// for close-ups of event pictures.
    pub background_offset: (i32, i32),
    /// Size of the background in percent (command 12, sub 5); 0 is 100.
    pub background_zoom: i32,
    /// Difference pictures drawn over the background, by slot (command 12,
    /// sub 1). Cleared when a new background is loaded.
    pub overlays: BTreeMap<i32, String>,
    /// Standing characters by slot 0..3 (command 13).
    pub characters: BTreeMap<i32, Character>,
    /// Images loaded into sprite layers (class Sprite, sub 1); the title
    /// screen uses these.
    pub layers: BTreeMap<i32, String>,
    /// Message window portrait: body and face (command 10), drawn at the
    /// bottom left over the window while `portrait_visible` is set.
    pub portrait: Option<(String, Option<String>)>,
    pub portrait_visible: bool,
    /// Speaker of the current line: row of the nameplate sheet (command
    /// 11). Cleared at every message.
    pub speaker: Option<i32>,
    /// Speaker given by name (Japanese builds: `Game.11("name")`).
    #[serde(default)]
    pub speaker_name: Option<String>,
    /// Music playing, by channel, so a loaded save can start it again.
    pub music: BTreeMap<i32, String>,
    /// Screen covered by a colour (0xRRGGBB) after a fade out.
    pub cover: Option<u32>,
    /// Screen shake offset in pixels.
    pub shake: (i32, i32),
    /// Current message text.
    pub text: Option<String>,
    /// Characters jumping or shaking now: pixels by slot (command 13,
    /// sub 7). Not saved.
    #[serde(skip)]
    pub jolts: BTreeMap<i32, (i32, i32)>,
}
