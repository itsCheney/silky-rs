//! Movies of なないろリンカネーション laid over the picture: scene movies,
//! effect movies and the moving background.
//!
//! The scripts name a descriptor (`HG070502.mov`) in the movie archive; it
//! holds the name of the stream (`HG070502.vsd`), an MPEG-1 picture of
//! 1280 by 720 pixels with MP2 sound. A movie plays on the stage as a
//! sprite whose picture follows the stage's clock.

use std::sync::Arc;

use crate::assets::Assets;
use crate::games::apeiria::anim::{Look, Sprite, Stage, Step};
use crate::host::Runtime;
use silky_format::game::Kind;

use super::effects::{clear, Fx};

/// Owner on the stage of the moving background of `Movie.1`.
pub(super) const BACKGROUND: u32 = 2301;
/// Drawing order of the moving background: under the standing figures.
const Z_BACKGROUND: i32 = -1;

/// How a movie's picture reaches the screen.
pub(super) const OPAQUE: u8 = 0;
pub(super) const GREEN_AS_OPACITY: u8 = 1;
pub(super) const BRIGHTNESS_AS_OPACITY: u8 = 2;

/// The name of the stream a descriptor points to: after a 4 byte
/// version, a zero padded name. Without a descriptor (a stream named
/// directly) the script's name with the extension of a stream.
fn stream_name(descriptor: &[u8], script_name: &str) -> String {
    let named = descriptor
        .get(4..)
        .and_then(|rest| rest.split(|&b| b == 0).next())
        .filter(|n| n.len() > 4 && n.is_ascii())
        .and_then(|n| String::from_utf8(n.to_vec()).ok());
    named.unwrap_or_else(|| {
        let stem = script_name
            .rsplit_once('.')
            .map_or(script_name, |(stem, _)| stem);
        format!("{stem}.vsd")
    })
}

/// The MPEG stream of the movie a script names.
pub fn stream(assets: &Assets, script_name: &str) -> Option<Vec<u8>> {
    let descriptor = assets.read(Kind::Movie, script_name).unwrap_or_default();
    let file = assets
        .read(Kind::Movie, &stream_name(&descriptor, script_name))
        .ok()?;
    crate::movie::mpeg_stream(&file).ok()
}

/// Whether a sprite is the moving background (drawn under the figures).
pub fn is_background(s: &Sprite) -> bool {
    s.owner == BACKGROUND
}

/// Puts the moving background up: the movie plays over the whole screen
/// for ever, at once, until the background changes.
pub fn background_begin(st: &mut Stage, name: &str) {
    clear(st, BACKGROUND);
    let mut fx = Fx::new(st, BACKGROUND);
    let id = fx.sprite(0, video(name, OPAQUE, true), Z_BACKGROUND);
    if let Some(Step::Add(s)) = fx.steps.last_mut() {
        s.shown = true;
    }
    fx.run();
    st.until_background.push(id);
}

/// A movie sprite's look.
pub(super) fn video(name: &str, mode: u8, looped: bool) -> Look {
    Look::Video {
        name: name.to_string(),
        mode,
        looped,
    }
}

impl Runtime {
    /// Whether the configuration has the movies on (`/Config/Movie/Play`,
    /// on unless a script or the settings turned it off).
    pub(crate) fn movies_on(&self) -> bool {
        self.xml.get("/Config/Movie/Play").copied().unwrap_or(1) != 0
    }

    /// Whether the movie a script names can be played.
    pub(crate) fn movie_playable(&self, name: &str) -> bool {
        self.movies_on() && stream(&self.assets, name).is_some()
    }

    /// The moving background of `Movie.1`; false when the movie is not
    /// played (movies off or not in the archive) and the picture stands.
    pub(crate) fn nanairo_background_movie(&mut self, name: &str) -> bool {
        if !self.movie_playable(name) {
            return false;
        }
        background_begin(&mut self.scene.effects, name);
        true
    }

    /// Starts the sound of a scene movie.
    pub(crate) fn movie_sound(&mut self, name: &str) {
        if let Some(stream) = stream(&self.assets, name) {
            self.start_movie_sound(Arc::new(stream));
        }
    }

    /// Stops the sound of a scene movie.
    pub(crate) fn movie_sound_stop(&mut self) {
        self.audio
            .stop(crate::audio::Bus::Music, crate::host::MOVIE_AUDIO);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_descriptor_names_its_stream() {
        let mut d = vec![2, 0, 0, 0];
        d.extend_from_slice(b"HG030603.vsd");
        d.resize(64, 0);
        assert_eq!(stream_name(&d, "HG030603.mov"), "HG030603.vsd");
    }

    #[test]
    fn without_a_descriptor_the_stream_has_the_scripts_name() {
        assert_eq!(stream_name(&[], "BG020002.mov"), "BG020002.vsd");
        assert_eq!(stream_name(&[2, 0, 0, 0, 0, 0], "A.mov"), "A.vsd");
    }

    #[test]
    fn the_background_movie_goes_with_the_background() {
        let mut st = Stage::default();
        background_begin(&mut st, "BG020002.mov");
        let s = st.sprites.iter().find(|s| is_background(s)).unwrap();
        assert!(s.shown);
        assert!(matches!(&s.look, Look::Video { looped: true, .. }));
        st.background_changed();
        assert!(!st.sprites.iter().any(is_background));
    }

    fn clip() -> crate::movie::Movie {
        let bytes = include_bytes!("../../../testdata/green32.mpg").to_vec();
        crate::movie::Movie::open(Arc::new(bytes)).unwrap()
    }

    #[test]
    fn modes_turn_a_picture_into_opacity() {
        use crate::games::apeiria::effects::video_picture;
        let m = clip();
        let opaque = video_picture(&m, OPAQUE);
        assert_eq!(opaque.rgba[3], 255);
        let green = video_picture(&m, GREEN_AS_OPACITY);
        assert_eq!(green.rgba[..3], [255, 255, 255]);
        assert_eq!(green.rgba[3], m.rgba[1]);
        let luma = video_picture(&m, BRIGHTNESS_AS_OPACITY);
        assert_eq!(luma.rgba[..3], [0xb0, 0xc8, 0xff]);
        assert!(luma.rgba[3] > 0 && luma.rgba[3] < 255);
    }

    #[test]
    fn a_movie_on_the_stage_keeps_the_screen_changing() {
        let mut st = Stage::default();
        assert!(!st.advance(16));
        background_begin(&mut st, "BG020002.mov");
        assert!(st.advance(16));
        assert_eq!(st.video_ms, 16);
        st.clear(BACKGROUND);
        assert!(!st.advance(16));
    }
}
