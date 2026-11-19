//! 同級生 Remake (Steam release, English and Chinese).
//!
//! The scripts carry no text. Each script `X.MES` has a text table per
//! language, `X_EN.MAP`, `X_CN.MAP` and `X_TW.MAP`, and `Game.24` shows a
//! line of it by its line id; speaker names are in `NAMES_<lang>.MAP`.
//! See `docs/games/doukyuusei.md`.

pub mod album;
pub mod calendar;
pub(crate) mod commands;
pub mod config;
pub mod hours;
pub mod log;
pub mod menu;
pub mod picker;
pub mod plate;
pub mod room;
pub mod select;
pub mod sound;
pub mod system;
pub mod town;
pub mod window;

use silky_format::map::MapFile;
use silky_vm::Vm;

use crate::assets::{Assets, Language};
use crate::host::Runtime;
use crate::render::Canvas;

pub use room::Click;

/// A screen of the game that answers the script with a number: the room
/// (`Game.16`) or the hours dial (`Game.29`).
#[derive(Debug, Clone)]
pub enum Panel {
    Room(room::Room),
    Hours(hours::Hours),
    /// The character picker (`Game.36`).
    Picker(picker::Picker),
    /// The album (`Game.50`).
    Album(album::Album),
    /// A walk on the map (`Game.20(30)`); the map itself is
    /// [`Runtime::town`], drawn as the background.
    Town,
}

impl Panel {
    /// The element under (x, y), as an id for [`crate::ui::Button::Panel`].
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        match self {
            Panel::Room(r) => r.hit(x, y),
            Panel::Hours(h) => h.hit(x, y),
            Panel::Picker(p) => p.hit(x, y),
            Panel::Album(a) => a.hit(x, y),
            Panel::Town => None,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        match self {
            Panel::Room(r) => r.click(id),
            Panel::Hours(h) => Click::Result(h.click(id)),
            Panel::Picker(p) => p.click(id),
            Panel::Album(a) => a.click(id),
            Panel::Town => Click::None,
        }
    }

    /// Room look mode on or off.
    pub fn set_look(&mut self, on: bool) {
        if let Panel::Room(r) = self {
            r.look = on;
        }
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed) of an id;
    /// `hover` is the id under the pointer.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
        font: Option<&window::Font>,
    ) -> crate::Result<()> {
        match self {
            Panel::Room(r) => r.draw(canvas, assets, state, hover, font),
            Panel::Hours(h) => h.draw(canvas, assets, language, state),
            Panel::Picker(p) => p.draw(canvas, assets, language, state),
            Panel::Album(a) => a.draw(canvas, assets, language, state),
            Panel::Town => Ok(()),
        }
    }
}

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

    /// Labels of the options before `Select.2`: each option body leaves
    /// its line id of the script's text table in `h[40 + i]`.
    pub(crate) fn doukyuusei_options(&mut self, vm: &Vm) -> Vec<String> {
        let script = self.message_script.clone();
        let hero = self.hero_name();
        let count = self.option_count;
        (0..count)
            .map(|i| {
                let id = vm.var_h(40 + i).int();
                self.doukyuusei_table(&script)
                    .ok()
                    .and_then(|t| t.get(0, id.max(0) as u32))
                    .map(|l| plain_text(&shown_text(l, &hero)))
                    .unwrap_or_else(|| format!("{}", i + 1))
            })
            .collect()
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
