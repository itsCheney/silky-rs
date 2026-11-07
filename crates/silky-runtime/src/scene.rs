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

/// Where an E-mote model stands: its size in percent and the position the
/// game hands the E-mote driver (read from the executable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub scale: i32,
    pub x: i32,
    pub y: i32,
}

impl Place {
    /// Anneliese at the bottom left while she walks around with the
    /// protagonist (command 10; 0x45c170 is given 500, -930 and sets the
    /// size to 0.35).
    pub const COMPANION: Place = Place {
        scale: 35,
        x: 500,
        y: -930,
    };

    /// A standing model in `slot` at `scale` percent (command 40, sub 0;
    /// 0x4577a0): x by slot, y keeps the feet at the same line whatever
    /// the size. `name` is the first file: H scene models (`?g...`) have
    /// their own place.
    pub fn standing(slot: i32, scale: i32, name: &str) -> Place {
        let s = scale as f32 / 100.0;
        let x = match slot {
            1 => 360,
            2 => -360,
            3 => 400,
            _ => 0,
        };
        if name.as_bytes().get(1) == Some(&b'g') {
            return Place {
                scale,
                x: 1,
                y: -683,
            };
        }
        let y = ((1.0 - s) * 800.0 - 1200.0) as i32;
        Place { scale, x, y }
    }

    /// Where the model's origin lands on the 1280 x 720 screen. The driver
    /// takes the position as a camera offset, hence the signs; the
    /// constants were measured on screenshots of the game (0.35 at the
    /// bottom left and 0.6 on the right agree).
    pub fn origin(&self) -> (f32, f32) {
        (640.0 - self.x as f32, -258.0 - self.y as f32)
    }
}

/// An E-mote standing picture (きまぐれテンプテーション2, commands 10 and
/// 40): the timelines the script chose, by the game's numbers (see
/// [`crate::emote::Character`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Emote {
    /// The `_h0007` and `_h0813` files of one costume.
    pub files: [String; 2],
    /// Face 0..=13 and the extra timeline (sub 7).
    pub face: Option<(i32, i32)>,
    /// Main timeline (sub 13).
    pub pose: Option<i32>,
    /// `a` timeline (sub 16).
    pub sub: Option<i32>,
    pub visible: bool,
    /// None for the companion picture of command 10.
    pub place: Option<Place>,
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
    /// E-mote picture of Anneliese walking along (command 10).
    pub emote: Option<Emote>,
    /// E-mote standing models by slot (command 40).
    pub emotes: BTreeMap<i32, Emote>,
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
    /// Colour (0xRRGGBB) and opacity laid over the pictures by sprite 6,
    /// under the message window.
    #[serde(default)]
    pub dim: Option<(u32, u8)>,
    /// Snow falls (ふゆから、くるる。, command 115).
    #[serde(default)]
    pub snow: bool,
    /// Screen shake offset in pixels.
    pub shake: (i32, i32),
    /// Current message text.
    pub text: Option<String>,
    /// Characters jumping or shaking now: pixels by slot (command 13,
    /// sub 7). Not saved.
    #[serde(skip)]
    pub jolts: BTreeMap<i32, (i32, i32)>,
}
