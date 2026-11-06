//! Data kept across saves: the player's settings and what has been seen
//! (pictures for the gallery and scene replays, music for the music room,
//! read lines for "skip read text only"). Stored as `system.json` in the
//! save folder.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::assets::Language;
use crate::{Error, Result};

/// What a right click does (system settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum RightClick {
    #[default]
    HideWindow,
    Settings,
    Skip,
    Auto,
    Load,
    Save,
}

/// Number of character voice groups in the sound settings: nine named
/// characters (voice files `va*`..`vi*`) and everyone else.
pub const VOICE_GROUPS: usize = 10;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Text appears letter by letter (else all at once).
    pub typewriter: bool,
    /// Text speed, 0..1 (1 fastest).
    pub text_speed: f32,
    /// Short pause after punctuation.
    pub punctuation_pause: bool,
    pub auto_save_at_choices: bool,
    pub tooltips: bool,
    /// Skip only text read before.
    pub skip_read_only: bool,
    pub skip_speed: f32,
    pub keep_skip_after_choices: bool,
    pub auto_speed: f32,
    pub keep_auto_after_choices: bool,
    /// Message window tint and opacity, 0..1.
    pub window_color: [f32; 3],
    pub window_opacity: f32,
    /// Kept for the settings screen; the window never moves here.
    pub fixed_window: bool,
    /// Screen effect speed: 0 normal, 1 fast, 2 off.
    pub effect_speed: u8,
    /// Ask before: title, quit, scene select, backlog jump, save, load,
    /// overwrite, delete, delete all (in the order of the dialog
    /// questions, see [`crate::screens::question`]).
    pub confirm: [bool; 9],
    pub right_click: RightClick,
    pub language: Option<Language>,
    pub master_on: bool,
    pub master: f32,
    pub music_on: bool,
    pub music: f32,
    pub effects_on: bool,
    pub effects: f32,
    pub voice_on: bool,
    pub voice: f32,
    pub voice_group_on: [bool; VOICE_GROUPS],
    pub voice_group: [f32; VOICE_GROUPS],
    /// A click cuts the voice of the message.
    pub cut_voice: bool,
    /// Animated E-mote characters (きまぐれテンプテーション2); kept for the
    /// settings screen, E-mote is not drawn yet.
    pub emote_animation: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            typewriter: true,
            text_speed: 0.7,
            punctuation_pause: false,
            auto_save_at_choices: true,
            tooltips: true,
            skip_read_only: true,
            skip_speed: 0.8,
            keep_skip_after_choices: false,
            auto_speed: 0.5,
            keep_auto_after_choices: false,
            window_color: [0.0, 0.0, 0.0],
            window_opacity: 0.5,
            fixed_window: false,
            effect_speed: 0,
            confirm: [true; 9],
            right_click: RightClick::HideWindow,
            language: None,
            master_on: true,
            master: 1.0,
            music_on: true,
            music: 0.7,
            effects_on: true,
            effects: 0.8,
            voice_on: true,
            voice: 1.0,
            voice_group_on: [true; VOICE_GROUPS],
            voice_group: [1.0; VOICE_GROUPS],
            cut_voice: true,
            emote_animation: true,
        }
    }
}

impl Settings {
    /// Milliseconds per character of text.
    pub fn char_ms(&self) -> u64 {
        (5.0 + (1.0 - self.text_speed.clamp(0.0, 1.0)) * 75.0) as u64
    }

    /// Skip mode: time per message.
    pub fn skip_ms(&self) -> u64 {
        (20.0 + (1.0 - self.skip_speed.clamp(0.0, 1.0)) * 200.0) as u64
    }

    /// Auto mode: time per message and per character.
    pub fn auto_ms(&self, chars: u64) -> u64 {
        let slow = 1.0 - self.auto_speed.clamp(0.0, 1.0);
        (600.0 + slow * 2400.0) as u64 + chars * (20.0 + slow * 80.0) as u64
    }

    /// Factor on transition times.
    pub fn effect_scale(&self) -> f32 {
        match self.effect_speed {
            0 => 1.0,
            1 => 0.5,
            _ => 0.0,
        }
    }

    /// Voice group of a voice file: `va0001.ogg` is group 0, `vi..` 8,
    /// anything else 9.
    pub fn voice_group_of(file: &str) -> usize {
        let b = file.as_bytes();
        match (
            b.first().map(u8::to_ascii_lowercase),
            b.get(1).map(u8::to_ascii_lowercase),
        ) {
            (Some(b'v'), Some(c @ b'a'..=b'i')) => (c - b'a') as usize,
            _ => VOICE_GROUPS - 1,
        }
    }

    /// Voice group in きまぐれテンプテーション2 (`k2a0001.ogg`): the third
    /// letter names the speaker. Groups follow the sound page: アンネリーゼ,
    /// コハル, 美代, 千代, みりあ, 花音, 晶, キルト, others. Letters matched to
    /// names by the speaker shown after each voice in a trace.
    pub fn voice_group_of_kimaten2(file: &str) -> usize {
        match file.as_bytes().get(2).map(u8::to_ascii_lowercase) {
            Some(b'a') => 0,
            Some(b'b') => 1,
            Some(b'c') => 2,
            Some(b'd') => 3,
            Some(b'f') => 4,
            Some(b'g') => 5,
            Some(b'h') => 6,
            Some(b'e') => 7,
            _ => 8,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SystemData {
    pub settings: Settings,
    /// Pictures shown in the story (lower case, with extension).
    pub seen: BTreeSet<String>,
    /// Music played in the story (lower case).
    pub music: BTreeSet<String>,
    /// Lines read, as `table:index`.
    pub read: BTreeSet<String>,
}

impl SystemData {
    fn path(dir: &Path) -> PathBuf {
        dir.join("system.json")
    }

    /// Reads `system.json` from a save folder; defaults if there is none.
    pub fn load(dir: &Path) -> Self {
        std::fs::read(Self::path(dir))
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_default()
    }

    pub fn store(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)
            .map_err(|e| Error::Invalid(format!("{}: {e}", dir.display())))?;
        let path = Self::path(dir);
        let tmp = path.with_extension("tmp");
        let json = serde_json::to_vec(self).map_err(|e| Error::Invalid(e.to_string()))?;
        std::fs::write(&tmp, json)
            .map_err(|e| Error::Invalid(format!("{}: {e}", tmp.display())))?;
        std::fs::rename(&tmp, &path).map_err(|e| Error::Invalid(format!("{}: {e}", path.display())))
    }

    /// Whether every layer of a gallery picture was seen.
    pub fn seen_all(&self, layers: &[&str]) -> bool {
        layers.iter().all(|l| {
            self.seen
                .contains(&format!("{}.akb", l.to_ascii_lowercase()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_groups() {
        assert_eq!(Settings::voice_group_of("va0001.ogg"), 0);
        assert_eq!(Settings::voice_group_of("VI0040.ogg"), 8);
        assert_eq!(Settings::voice_group_of("vk0145a.ogg"), 9);
        assert_eq!(Settings::voice_group_of_kimaten2("K2E0001.OGG"), 7);
        assert_eq!(Settings::voice_group_of_kimaten2("k2x0001.ogg"), 8);
    }

    #[test]
    fn settings_round_trip() {
        let s = SystemData::default();
        let j = serde_json::to_string(&s).unwrap();
        let back: SystemData = serde_json::from_str(&j).unwrap();
        assert_eq!(back.settings, s.settings);
    }
}
