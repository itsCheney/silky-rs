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
    /// A second picture drawn over the expression (景の海のアペイリア:
    /// cheeks, command 13 sub 6).
    #[serde(default)]
    pub cheeks: Option<String>,
    /// Opacity, 0 to 255 (command 13, sub 12 gives one).
    #[serde(default = "opaque")]
    pub alpha: i32,
}

fn opaque() -> i32 {
    255
}

/// 同級生 Remake: a picture object of `Game.13`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Picture {
    pub name: String,
    /// Picture laid onto it (`Game.13` sub 1), the same size; a new one
    /// covers the last.
    pub face: Option<String>,
    /// Opacity in percent.
    pub alpha: i32,
}

/// Where an E-mote model stands: its size in percent and the position the
/// game hands the E-mote driver (read from the executable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub scale: i32,
    pub x: i32,
    pub y: i32,
    /// An H scene model (`?g...`): the engine hands its position to the
    /// model rather than to the picture it draws, which puts it a little
    /// higher than a standing model at the same numbers.
    #[serde(default)]
    pub scene: bool,
}

impl Place {
    /// Anneliese at the bottom left while she walks around with the
    /// protagonist (command 10; the engine places her at 500, -930 at size
    /// 0.35).
    pub const COMPANION: Place = Place {
        scale: 35,
        x: 500,
        y: -930,
        scene: false,
    };

    /// A standing model in `slot` at `scale` percent (command 40, sub 0):
    /// x by slot, y keeps the feet at the same line whatever
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
                scene: true,
            };
        }
        let y = ((1.0 - s) * 800.0 - 1200.0) as i32;
        Place {
            scale,
            x,
            y,
            scene: false,
        }
    }

    /// Where the model's origin lands on the 1280 x 720 screen. The driver
    /// takes the position as a camera offset, hence the signs; the
    /// constants were measured on screenshots of the game (0.35 at the
    /// bottom left and 0.6 on the right agree). An H scene model sits 26
    /// pixels higher, which brings its 1280 x 720 frame onto the screen.
    pub fn origin(&self) -> (f32, f32) {
        let top = if self.scene { -284.0 } else { -258.0 };
        (640.0 - self.x as f32, top - self.y as f32)
    }

    /// Top left corner of the 2000 x 2000 picture the game draws the model
    /// into, from the model's origin; the censor mosaic's cells line up
    /// with it. An H scene model's picture is 4 pixels lower than its
    /// origin would put it (measured on the game's mosaic under Wine).
    pub fn picture(&self) -> (f32, f32) {
        let half = crate::emote::HALF_PICTURE;
        (-half, if self.scene { 4.0 - half } else { -half })
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
    /// Main timeline to play once the current one has ended (command 40,
    /// sub 13).
    #[serde(default)]
    pub next_pose: Option<i32>,
    pub visible: bool,
    /// The second outfit (byte 708) when the model was loaded; an H scene
    /// model then starts without its clothes.
    #[serde(default)]
    pub outfit: bool,
    /// None for the companion picture of command 10.
    pub place: Option<Place>,
    /// きまぐれテンプテーション: a CG model (command 48), one file whose
    /// timelines are numbered from 0 for faces and for the main timeline.
    #[serde(default)]
    pub cg: bool,
}

impl Scene {
    /// The corner of the picture in sprite layer `layer` that is drawn at
    /// the screen's top left.
    pub fn layer_offset(&self, layer: i32) -> (i32, i32) {
        match &self.pan {
            Some(p) if layer == p.layer => p.shown(),
            _ => self.layer_view.get(&layer).copied().unwrap_or((0, 0)),
        }
    }
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
    /// Cells the `Anime` class drew into the background, and its programs
    /// (see [`crate::anime`]).
    #[serde(default)]
    pub anime: crate::anime::Anime,
    /// Standing characters by slot 0..3 (command 13).
    pub characters: BTreeMap<i32, Character>,
    /// Images loaded into sprite layers (class Sprite, sub 1); the title
    /// screen uses these.
    pub layers: BTreeMap<i32, String>,
    /// Message window portrait: body and face (command 10), drawn at the
    /// bottom left over the window while `portrait_visible` is set.
    pub portrait: Option<(String, Option<String>)>,
    pub portrait_visible: bool,
    /// 先輩が私の妄想にドージンする: the second portrait window (command
    /// 16), on the right.
    #[serde(default)]
    pub portrait2: Option<(String, Option<String>)>,
    #[serde(default)]
    pub portrait2_visible: bool,
    /// 景の海のアペイリア: cheeks drawn over the portrait (command 10,
    /// sub 8).
    #[serde(default)]
    pub portrait_cheeks: Option<String>,
    /// E-mote picture of Anneliese walking along (command 10).
    pub emote: Option<Emote>,
    /// E-mote standing models by slot (command 40).
    pub emotes: BTreeMap<i32, Emote>,
    /// Speaker of the current line: row of the nameplate sheet (command
    /// 11). Cleared at every message.
    pub speaker: Option<i32>,
    /// 同級生 Remake: the `.area` file of the room screen (`Game.17`).
    #[serde(default)]
    pub room_area: Option<String>,
    /// 同級生 Remake: the speaker's icon in the message window, a cell of
    /// `message_icon.akb` (`Game.10` sub 15).
    #[serde(default)]
    pub speaker_icon: Option<u32>,
    /// Speaker given by name (Japanese builds: `Game.11("name")`).
    #[serde(default)]
    pub speaker_name: Option<String>,
    /// 同級生 Remake: the speaker is the hero (his name is the player's).
    #[serde(default)]
    pub speaker_hero: bool,
    /// Music playing, by channel, so a loaded save can start it again.
    pub music: BTreeMap<i32, String>,
    /// Screen covered by a colour (0xRRGGBB) after a fade out.
    pub cover: Option<u32>,
    /// Colour (0xRRGGBB) and opacity laid over the pictures by sprite 6,
    /// under the message window.
    #[serde(default)]
    pub dim: Option<(u32, u8)>,
    /// The αMAIL screen (景の海のアペイリア, command 1012).
    #[serde(default)]
    pub mail: Option<crate::games::apeiria::mail::Mail>,
    /// The mail forward screen (景の海のアペイリア, command 1016).
    #[serde(default)]
    pub mail_select: Option<crate::games::apeiria::mail_select::MailSelect>,
    /// Apeiria on the hologram panel (景の海のアペイリア, command 1013).
    #[serde(default)]
    pub holo: Option<crate::games::apeiria::holo::Holo>,
    /// Pictures 景の海のアペイリア lays over the scene (commands 994 to
    /// 997).
    #[serde(default)]
    pub panels: crate::games::apeiria::panels::Panels,
    /// 同級生 Remake: the walking map drawn as the background (`Game.20`
    /// sub 10), shown while no background picture is loaded.
    #[serde(default)]
    pub town: Option<crate::games::doukyuusei::town::View>,
    /// 同級生 Remake: the status plate and menu button are up (see
    /// [`crate::games::doukyuusei::plate`]).
    #[serde(default)]
    pub plate: bool,
    /// 同級生 Remake: picture objects over the background by id
    /// (`Game.13`): a standing character the size of the screen and the
    /// face laid onto it.
    #[serde(default)]
    pub pictures: BTreeMap<i32, Picture>,
    /// 同級生 Remake: the train of Misa's scenes (`Game.39`), over the
    /// background.
    #[serde(default)]
    pub train: Option<crate::games::doukyuusei::train::Train>,
    /// 同級生 Remake: where the window of a tall sprite layer's picture is
    /// (`Game.33`, `Sprite.3`), by layer; a layer not here is drawn from
    /// its top left corner.
    #[serde(default)]
    pub layer_view: BTreeMap<i32, (i32, i32)>,
    /// 同級生 Remake: the pan over a layer's picture while the script waits
    /// for it (`Game.33`). Not saved.
    #[serde(skip)]
    pub pan: Option<crate::games::doukyuusei::pan::Pan>,
    /// 同級生 Remake: the intro of the first start (`Game.31`), while the
    /// script waits for it.
    #[serde(default)]
    pub intro: Option<crate::games::doukyuusei::intro::Intro>,
    /// なないろリンカネーション: the CG picture named so far (`Game.14`).
    #[serde(default)]
    pub cg: crate::games::nanairo::cg::Combo,
    /// Snow falls (ふゆから、くるる。, command 115).
    #[serde(default)]
    pub snow: bool,
    /// The engine's store of variables (class `Save`, see [`crate::store`]).
    pub store: crate::store::Store,
    /// 景の海のアペイリア's attack and spell effects (commands 998, 1000,
    /// 1001, 1002). Not saved.
    #[serde(skip)]
    pub effects: crate::games::apeiria::anim::Stage,
    /// きまぐれテンプテーション's E-mote fades (see
    /// [`crate::games::kimaten1::effects`]). Not saved.
    #[serde(skip)]
    pub k1fx: crate::games::kimaten1::effects::Fx,
    /// Screen shake offset in pixels.
    pub shake: (i32, i32),
    /// Current message text.
    pub text: Option<String>,
    /// Characters jumping or shaking now: pixels by slot (command 13,
    /// sub 7). Not saved.
    #[serde(skip)]
    pub jolts: BTreeMap<i32, (i32, i32)>,
    /// Whole screen enlarged from its centre, in percent (ふゆから、
    /// くるる。, command 113). Not saved.
    #[serde(skip)]
    pub zoom: Option<u32>,
    /// Picture shown over a black screen, above everything (ふゆから、
    /// くるる。, command 114). Not saved.
    #[serde(skip)]
    pub flash: Option<String>,
    /// Screen shaking up and down by this many pixels until stopped
    /// (ふゆから、くるる。, command 993).
    #[serde(default)]
    pub quake: Option<i32>,
    /// Screen shaking through these offsets, a step a frame, until stopped
    /// (景の海のアペイリア, command 993).
    #[serde(default)]
    pub quake_steps: Option<Vec<(i32, i32)>>,
}
