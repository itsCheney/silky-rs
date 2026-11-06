use std::collections::HashMap;
use std::sync::Arc;

use silky_format::akb::{self, Image};
use silky_format::game::{Game, Kind};
use silky_format::mes::Script;

use crate::{Error, Result};

/// Text and art language of the international build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
    TraditionalChinese,
}

impl Language {
    /// Section of the `.MAP` text tables.
    pub fn map_section(self) -> usize {
        match self {
            Language::English => 0,
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
        }
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

    pub fn script(&self, name: &str) -> Result<Script> {
        Ok(Script::parse(&self.game.set(Kind::Script).read(name)?)?)
    }

    pub fn data(&self, name: &str) -> Result<Vec<u8>> {
        Ok(self.game.set(Kind::Data).read(name)?)
    }

    /// A file from the archives of `kind`.
    pub fn read(&self, kind: Kind, name: &str) -> Result<Vec<u8>> {
        self.game
            .set(kind)
            .read(name)
            .map_err(|_| Error::Missing(name.to_string()))
    }

    /// Whether a file exists in the archives of `kind`.
    pub fn exists(&self, kind: Kind, name: &str) -> bool {
        self.game.set(kind).find(name).is_some()
    }

    /// Decoded image, with `AKB+` difference images applied to their base.
    pub fn image(&mut self, name: &str) -> Result<Arc<Image>> {
        let name = with_ext(name);
        let key = name.to_ascii_lowercase();
        if let Some(img) = self.images.get(&key) {
            return Ok(img.clone());
        }
        let data = self
            .game
            .set(Kind::Layer)
            .read(&name)
            .map_err(|_| Error::Missing(name.clone()))?;
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
    }
}
