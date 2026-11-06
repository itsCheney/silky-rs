use std::collections::HashMap;
use std::sync::Arc;

use silky_format::akb::{self, Image};
use silky_format::game::{Game, Kind};
use silky_format::mes::Script;

use crate::{Error, Result};

/// Text and art language. International builds carry English and Chinese
/// in their `.MAP` tables; Japanese builds carry Japanese in the scripts,
/// and Chinese only through a fan translation folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
    TraditionalChinese,
    Japanese,
}

impl Language {
    /// Section of the `.MAP` text tables.
    pub fn map_section(self) -> usize {
        match self {
            Language::English | Language::Japanese => 0,
            Language::SimplifiedChinese => 1,
            Language::TraditionalChinese => 2,
        }
    }

    /// Suffix of localized file names (`title_bg_00_en.akb`).
    pub fn suffix(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::SimplifiedChinese => "cn",
            Language::TraditionalChinese => "tw",
            Language::Japanese => "jp",
        }
    }

    /// Tag of a fan translation folder (`kimaten2_chs`) in this language.
    pub fn translation_tag(self) -> Option<&'static str> {
        match self {
            Language::SimplifiedChinese => Some("chs"),
            Language::TraditionalChinese => Some("cht"),
            _ => None,
        }
    }

    /// From a short name: `en`, `zh-hans`, `zh-hant`, `ja`, or the table
    /// sections 0, 1, 2.
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "0" | "en" => Language::English,
            "1" | "zh-hans" | "chs" | "zh" => Language::SimplifiedChinese,
            "2" | "zh-hant" | "cht" => Language::TraditionalChinese,
            "ja" | "jp" => Language::Japanese,
            _ => return None,
        })
    }

    pub fn from_section(section: usize) -> Option<Self> {
        Some(match section {
            0 => Language::English,
            1 => Language::SimplifiedChinese,
            2 => Language::TraditionalChinese,
            _ => return None,
        })
    }
}

/// Game files with a cache of decoded images.
pub struct Assets {
    game: Game,
    images: HashMap<String, Arc<Image>>,
}

impl Assets {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            images: HashMap::new(),
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Languages this game can be played in, the default first.
    pub fn languages(&self) -> Vec<Language> {
        languages(&self.game)
    }

    /// Picks the files for a language: a fan translation folder, or the
    /// original files. Cached images are dropped.
    pub fn set_language(&mut self, language: Language) {
        if !self.game.has_text_tables() {
            self.game.use_translation(language.translation_tag());
            self.images.clear();
        }
    }

    pub fn script(&self, name: &str) -> Result<Script> {
        Ok(self.game.script(name)?)
    }

    pub fn data(&self, name: &str) -> Result<Vec<u8>> {
        Ok(self.game.read(Kind::Data, name)?)
    }

    /// A file from the archives of `kind`.
    pub fn read(&self, kind: Kind, name: &str) -> Result<Vec<u8>> {
        self.game
            .read(kind, name)
            .map_err(|_| Error::Missing(name.to_string()))
    }

    /// Whether a file exists in the archives of `kind`.
    pub fn exists(&self, kind: Kind, name: &str) -> bool {
        self.game.exists(kind, name)
    }

    /// Decoded image, with `AKB+` difference images applied to their base.
    pub fn image(&mut self, name: &str) -> Result<Arc<Image>> {
        let name = with_ext(name);
        let key = name.to_ascii_lowercase();
        if let Some(img) = self.images.get(&key) {
            return Ok(img.clone());
        }
        let data = self.read(Kind::Layer, &name)?;
        let header = akb::Header::parse(&data)?;
        let base = match &header.base {
            Some(b) if with_ext(b).eq_ignore_ascii_case(&name) => {
                return Err(Error::Invalid(format!("{name} names itself as base")))
            }
            Some(b) => Some(self.image(b)?),
            None => None,
        };
        let img = Arc::new(akb::decode(&data, base.as_deref())?);
        self.images.insert(key, img.clone());
        Ok(img)
    }

    /// Drops cached images, e.g. between scenes.
    pub fn clear_cache(&mut self) {
        self.images.clear();
    }
}

/// Languages a game can be played in, the default first.
pub fn languages(game: &Game) -> Vec<Language> {
    if game.has_text_tables() {
        return vec![
            Language::English,
            Language::SimplifiedChinese,
            Language::TraditionalChinese,
        ];
    }
    let mut list = vec![Language::Japanese];
    for l in [Language::SimplifiedChinese, Language::TraditionalChinese] {
        let tag = l.translation_tag();
        if game
            .translations
            .iter()
            .any(|t| Some(t.tag.as_str()) == tag)
        {
            list.push(l);
        }
    }
    list
}

/// Base names and script arguments may leave out the extension.
pub fn with_ext(name: &str) -> String {
    if name.contains('.') {
        name.to_string()
    } else {
        format!("{name}.akb")
    }
}

/// `title_bg_00.akb` -> `title_bg_00_en.akb`.
pub fn localized_name(name: &str, lang: Language) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}_{}.{ext}", lang.suffix()),
        None => format!("{name}_{}", lang.suffix()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(with_ext("bg01"), "bg01.akb");
        assert_eq!(
            localized_name("title_bg_00.akb", Language::SimplifiedChinese),
            "title_bg_00_cn.akb"
        );
        assert_eq!(
            Language::from_section(2),
            Some(Language::TraditionalChinese)
        );
        assert_eq!(Language::parse("ja"), Some(Language::Japanese));
        assert_eq!(Language::parse("1"), Some(Language::SimplifiedChinese));
    }
}
