//! 同級生 Remake (Steam release, English and Chinese).
//!
//! The scripts carry no text. Each script `X.MES` has a text table per
//! language, `X_EN.MAP`, `X_CN.MAP` and `X_TW.MAP`, and `Game.24` shows a
//! line of it by its line id; speaker names are in `NAMES_<lang>.MAP`.
//! See `docs/games/doukyuusei.md`.

pub(crate) mod commands;
pub mod window;

use silky_format::map::MapFile;

use crate::assets::Language;
use crate::host::Runtime;

/// The hero's name when none was entered, by language, from the program.
pub fn default_hero_name(language: Language) -> &'static str {
    match language {
        Language::English => "Takurou",
        Language::Japanese => "たくろう",
        _ => "拓郎",
    }
}

/// A text table line as shown: `\n` breaks the line, `\i` turns italics
/// on and off (kept as [`window::ITALIC`]), `:myname:` is the hero's name.
pub fn shown_text(raw: &str, hero: &str) -> String {
    raw.replace("\\n", "\n")
        .replace("\\i", &window::ITALIC.to_string())
        .replace(":myname:", hero)
}

/// Text without its italic switches, for places that show it plainly.
pub fn plain_text(shown: &str) -> String {
    shown.replace(window::ITALIC, "")
}

/// Language part of picture names: `en`, `ch` (Simplified Chinese) or
/// `tw`; the text tables say `cn` instead of `ch`.
pub fn picture_suffix(language: Language) -> &'static str {
    match language {
        Language::SimplifiedChinese => "ch",
        Language::TraditionalChinese => "tw",
        _ => "en",
    }
}

/// `cafe.mes` -> `cafe_en.map`.
pub fn table_name(script: &str, language: Language) -> String {
    let stem = script.rsplit_once('.').map_or(script, |(s, _)| s);
    format!("{stem}_{}.map", language.suffix())
}

impl Runtime {
    pub(crate) fn hero_name(&self) -> String {
        self.hero_name
            .clone()
            .unwrap_or_else(|| default_hero_name(self.config.language).to_string())
    }

    /// The table of `script` in the configured language, kept loaded.
    pub(crate) fn doukyuusei_table(&mut self, script: &str) -> crate::Result<&MapFile> {
        let name = table_name(script, self.config.language);
        if self.text_table.as_ref().map(|t| t.0.as_str()) != Some(name.as_str()) {
            let map = MapFile::parse(&self.assets.data(&name)?)?;
            self.text_table = Some((name, map));
        }
        Ok(&self.text_table.as_ref().unwrap().1)
    }

    /// After a language change: the message shown, again in the new
    /// language.
    pub(crate) fn doukyuusei_relanguage(&mut self) {
        let (Some(id), true) = (self.last_text, self.scene.text.is_some()) else {
            return;
        };
        let script = self.message_script.clone();
        let hero = self.hero_name();
        if let Ok(t) = self.doukyuusei_table(&script) {
            if let Some(line) = t.get(0, id as u32) {
                let text = shown_text(line, &hero);
                self.scene.text = Some(text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text() {
        assert_eq!(
            plain_text(&shown_text(
                "Uh... :myname:?\\nYou can \\itoo\\i.",
                "Takurou"
            )),
            "Uh... Takurou?\nYou can too."
        );
        assert_eq!(
            table_name("CAFE.MES", Language::SimplifiedChinese),
            "CAFE_cn.map"
        );
    }
}
