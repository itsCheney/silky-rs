//! `Game` commands of 同級生 Remake, read from its executable. The game
//! numbers its commands its own way; see `docs/games/doukyuusei.md`.

use silky_format::map::MapFile;
use silky_vm::{Flow, Vm};

use super::shown_text;
use crate::games::Call;
use crate::host::{host_error, Runtime};

/// `Game.24`: sub 0 (line id) takes a line of the script's text table for
/// the next `MesWnd.3`, sub 1 (name id) names the speaker from
/// `NAMES_<lang>.MAP`, sub 2 loads the script's table.
pub const TEXT: i32 = 24;
/// `Game.22(name)`: the hero speaks; the plate shows the name entered at
/// the start (the executable reads its own copy, not the argument).
pub const HERO_SPEAKS: i32 = 22;

impl Runtime {
    pub(crate) fn doukyuusei_game(
        &mut self,
        vm: &mut Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        match (c.command, c.sub) {
            (TEXT, 0) => {
                let id = c.int(0);
                let script = vm.script_name().unwrap_or("").to_ascii_lowercase();
                self.message_script = script.clone();
                let hero = self.hero_name();
                let line = match self.doukyuusei_table(&script) {
                    Ok(t) => t.get(0, id as u32).map(|l| shown_text(l, &hero)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                };
                let Some(text) = line else {
                    return Some(Err(host_error(format!("{script}: no text line {id}"))));
                };
                self.pending_text = Some(text);
                self.last_text = Some(id.max(0) as usize);
                self.done()
            }
            (TEXT, 1) => {
                let id = c.int(0);
                let name = format!("names_{}.map", self.config.language.suffix());
                self.scene.speaker_name = self
                    .assets
                    .data(&name)
                    .ok()
                    .and_then(|b| MapFile::parse(&b).ok())
                    .and_then(|m| m.get(0, id as u32).map(str::to_string));
                self.done()
            }
            (HERO_SPEAKS, _) => {
                self.scene.speaker_name = Some(self.hero_name());
                self.done()
            }
            (TEXT, 2) => {
                let script = vm.script_name().unwrap_or("").to_ascii_lowercase();
                if let Err(e) = self.doukyuusei_table(&script) {
                    return Some(Err(host_error(e.to_string())));
                }
                self.done()
            }
            _ => None,
        }
    }
}
