//! 同級生 Remake (Steam release, English and Chinese).
//!
//! The scripts carry no text. Each script `X.MES` has a text table per
//! language, `X_EN.MAP`, `X_CN.MAP` and `X_TW.MAP`, and `Game.24` shows a
//! line of it by its line id; speaker names are in `NAMES_<lang>.MAP`.
//! See `docs/games/doukyuusei.md`.

pub mod album;
pub mod calendar;
pub(crate) mod commands;
pub mod comment;
pub mod config;
pub mod cursor;
mod flag_build;
mod flag_clock;
pub mod flaglist;
pub mod gdi;
pub mod hours;
pub mod intro;
mod intro_data;
pub mod jump;
pub mod keys;
pub mod log;
pub mod menu;
pub mod music;
pub mod name;
pub mod notice;
pub mod pan;
pub mod picker;
pub mod plate;
pub mod reel;
pub mod room;
pub mod savenote;
pub mod select;
pub mod sound;
pub mod system;
pub mod town;
pub mod train;
pub mod unlocked;
pub mod who;
mod who_data;
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
    /// The memory reel (`Game.51`).
    Reel(reel::Reel),
    /// The music mode (`Game.52`).
    Music(music::Music),
    /// The comments page (`Game.53`).
    Comment(comment::Comment),
    /// The name entry (`Game.18`).
    Name(name::Name),
    /// The modal notice (`Game.30`).
    Notice(notice::Notice),
    /// The "comment unlocked" pop-up (`Game.37`).
    Unlocked(unlocked::Unlocked),
    /// The map jump page, over the walk.
    Jump(jump::Jump),
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
            Panel::Reel(r) => r.hit(x, y),
            Panel::Music(m) => m.hit(x, y),
            Panel::Comment(c) => c.hit(x, y),
            Panel::Name(n) => n.hit(x, y),
            Panel::Notice(n) => n.hit(x, y),
            Panel::Unlocked(u) => u.hit(x, y),
            Panel::Jump(j) => j.hit(x, y),
            Panel::Town => None,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        match self {
            Panel::Room(r) => r.click(id),
            Panel::Hours(h) => Click::Result(h.click(id)),
            Panel::Picker(p) => p.click(id),
            Panel::Album(a) => a.click(id),
            Panel::Reel(r) => r.click(id),
            Panel::Music(m) => m.click(id),
            Panel::Comment(c) => c.click(id),
            Panel::Name(n) => n.click(id),
            Panel::Notice(n) => n.click(id),
            Panel::Unlocked(u) => u.click(id),
            Panel::Jump(j) => j.click(id),
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
    /// `hover` is the id under the pointer; `fx` the room's slide.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
        font: Option<&window::Font>,
        fx: Option<room::Pass>,
        dodge: bool,
    ) -> crate::Result<()> {
        match self {
            Panel::Room(r) => r.draw(canvas, assets, state, hover, font, fx, dodge),
            Panel::Hours(h) => h.draw(canvas, assets, language, state),
            Panel::Picker(p) => p.draw(canvas, assets, language, state),
            Panel::Album(a) => a.draw(canvas, assets, language, state),
            Panel::Reel(r) => r.draw(canvas, assets, language, state),
            Panel::Music(m) => m.draw(canvas, assets, language, state),
            Panel::Comment(c) => c.draw(canvas, assets, language, state),
            Panel::Name(n) => n.draw(canvas, assets, language, state, font),
            Panel::Notice(n) => n.draw(canvas, assets, language, state),
            Panel::Unlocked(u) => u.draw(canvas, assets, language, state),
            Panel::Jump(j) => j.draw(canvas, assets, language, state),
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

/// The name a table file was loaded under: `misa_z_en.map` -> `misa_z`.
pub fn table_stem_of(file: &str) -> Option<String> {
    ["_en.map", "_cn.map", "_tw.map"]
        .iter()
        .find_map(|suffix| file.strip_suffix(suffix))
        .map(str::to_string)
}

impl Runtime {
    /// The name of the table lines are read from: the one the script gave
    /// (`Game.24`, sub 2), whatever script runs now; the running script's
    /// own name before any. The update archive's variants of a script
    /// (`misa_z_r18.mes`) name the table of the plain one (`misa_z`), which
    /// has the lines of both.
    pub(crate) fn table_stem(&self, script: &str) -> String {
        self.table_stem
            .clone()
            .unwrap_or_else(|| script.to_string())
    }

    /// The hero's name: the raw area's name field, or the default one
    /// when nothing was entered.
    pub(crate) fn hero_name(&self, vm: &Vm) -> String {
        let field = vm.raw.get(name::NAME_AT..).unwrap_or(&[]);
        let field = &field[..field.len().min(name::NAME_BYTES)];
        let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
        match String::from_utf8_lossy(&field[..end]).into_owned() {
            s if s.is_empty() => default_hero_name(self.config.language).to_string(),
            s => s,
        }
    }

    /// Keeps the name entered in the raw area.
    pub(crate) fn store_hero_name(vm: &mut Vm, name: &str) {
        if vm.raw.len() < name::NAME_AT + name::NAME_BYTES {
            vm.raw.resize(name::NAME_AT + name::NAME_BYTES, 0);
        }
        let bytes = &name.as_bytes()[..name.len().min(name::NAME_BYTES - 1)];
        vm.raw[name::NAME_AT..name::NAME_AT + bytes.len()].copy_from_slice(bytes);
        vm.raw[name::NAME_AT + bytes.len()] = 0;
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
    /// its line id of the script's text table in `h[40 + N]`, N being the
    /// option's number.
    pub(crate) fn doukyuusei_options(&mut self, vm: &Vm) -> Vec<String> {
        let script = self.table_stem(&self.message_script);
        let hero = self.hero_name(vm);
        // The line of option N is h[40 + N].
        let numbers = self.option_numbers.clone();
        numbers
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let id = vm.var_h((40 + n).max(0) as usize).int();
                self.doukyuusei_table(&script)
                    .ok()
                    .and_then(|t| t.get(0, id.max(0) as u32))
                    .map(|l| plain_text(&shown_text(l, &hero)))
                    .unwrap_or_else(|| format!("{}", i + 1))
            })
            .collect()
    }

    /// Shown status of each option: replay mode is `h[69]`, the mark of
    /// option N is `b[1900 + N]`.
    pub(crate) fn doukyuusei_status(&self, vm: &Vm) -> Vec<select::Status> {
        let mode = vm.var_h(69).int();
        self.option_numbers
            .iter()
            .map(|&n| select::status(mode, vm.var_b((1900 + n).max(0) as usize)))
            .collect()
    }

    /// After a language change: the message shown, again in the new
    /// language.
    pub(crate) fn doukyuusei_relanguage(&mut self, vm: &Vm) {
        let (Some(id), true) = (self.last_text, self.scene.text.is_some()) else {
            return;
        };
        let script = self.table_stem(&self.message_script);
        let hero = self.hero_name(vm);
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

    #[test]
    fn table_stems() {
        // A variant script names the plain script's table; the stem is
        // what the file name had before its language part.
        let file = table_name("misa_z", Language::English);
        assert_eq!(file, "misa_z_en.map");
        assert_eq!(table_stem_of(&file).as_deref(), Some("misa_z"));
        assert_eq!(table_stem_of("names_cn.map").as_deref(), Some("names"));
        assert_eq!(table_stem_of("misa_z.map"), None);
    }
}
