use std::collections::BTreeMap;

use silky_format::game::Kind;
use silky_format::map::MapFile;
use silky_vm::native::{self, script, GAME, SCRIPT};
use silky_vm::{Flow, Host, Value, Vm};

use crate::assets::{localized_name, Assets, Language};
use crate::scene::{Character, Scene};

const SPRITE: i32 = 19;

/// Game commands (class 29) this runtime knows.
mod cmd {
    /// Message window portrait: 6 body, 7 face, 9 show, 11 hide.
    pub const PORTRAIT: i32 = 10;
    /// Background: 0 load, 1 difference picture, 2 and 3 transitions.
    pub const PICTURE: i32 = 12;
    /// Standing characters: 0 load, 1 face, 2 apply, 3 remove, 4 remove all.
    pub const CHARACTER: i32 = 13;
    /// Story text: 2 load a `.map` table, 0 show a line.
    pub const TEXT: i32 = 23;
    /// Title menu; returns the item picked.
    pub const TITLE: i32 = 110;
    /// Localized file name: `x.akb` -> `x_en.akb` when that exists.
    pub const LOCALIZE: i32 = 223;
}

#[derive(Debug, Clone)]
pub struct Config {
    pub language: Language,
    /// Result of the title menu: 1 start, 257 CG mode, 258 scene mode,
    /// 259 music mode.
    pub title_choice: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::English,
            title_choice: 1,
        }
    }
}

/// Why the runtime asked the interpreter to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pause {
    /// A message line is shown; the original waits for a click here.
    Text { index: usize },
    /// The title screen is up.
    Title,
}

pub struct Runtime {
    pub assets: Assets,
    pub scene: Scene,
    pub config: Config,
    text_table: Option<(String, MapFile)>,
    /// Set when a callback yields.
    pub pause: Option<Pause>,
    /// Engine calls not carried out yet, by name, with counts.
    pub unhandled: BTreeMap<String, u64>,
    /// Engine calls carried out.
    pub handled: u64,
}

fn host_error(msg: impl Into<String>) -> silky_vm::Error {
    silky_vm::Error::Host(msg.into())
}

impl Runtime {
    pub fn new(assets: Assets, config: Config) -> Self {
        Self {
            assets,
            scene: Scene::default(),
            config,
            text_table: None,
            pause: None,
            unhandled: BTreeMap::new(),
            handled: 0,
        }
    }

    /// Starts the game at its first script (`start.mes`).
    pub fn boot(&self, vm: &mut Vm, start: &str) -> crate::Result<()> {
        vm.jump_script(start, self.assets.script(start)?);
        Ok(())
    }

    fn skip(&mut self, name: String, vm: &mut Vm, class: i32, sub: i32) {
        *self.unhandled.entry(name).or_default() += 1;
        if native::returns_value(class, sub) {
            vm.push(0);
        }
    }

    fn game(&mut self, vm: &mut Vm) -> silky_vm::Result<Flow> {
        let mut args = vm.pop_statement();
        let Some(Value::Int(command)) = args.pop() else {
            return Err(host_error("game call without a command"));
        };
        let sub = match args.last() {
            Some(Value::Int(s)) => *s,
            _ => -1,
        };
        if sub >= 0 && command != cmd::TITLE && command != cmd::LOCALIZE {
            args.pop();
        }
        let s = |i: usize| args.get(i).and_then(Value::as_str).map(str::to_string);
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        let scene = &mut self.scene;
        match (command, sub, args.len()) {
            (cmd::PORTRAIT, 6, 1) => scene.portrait = s(0).map(|b| (b, None)),
            (cmd::PORTRAIT, 7, 1) => {
                if let Some(p) = &mut scene.portrait {
                    p.1 = s(0);
                }
            }
            (cmd::PORTRAIT, 9, 0) => scene.portrait_visible = true,
            (cmd::PORTRAIT, 11, 0) => scene.portrait_visible = false,
            (cmd::PICTURE, 0, 1) => {
                scene.background = s(0);
                scene.overlays.clear();
                // Provisional: the title picture sits in a sprite layer;
                // the first background of the game replaces it.
                scene.layers.clear();
            }
            // (slot, parameter, name)
            (cmd::PICTURE, 1, 3) => match s(2).filter(|v| !v.is_empty()) {
                Some(name) => {
                    scene.overlays.insert(n(0), name);
                }
                None => {
                    scene.overlays.remove(&n(0));
                }
            },
            // Transitions: the state is already applied.
            (cmd::PICTURE, 2, 0) | (cmd::PICTURE, 3, 1) => {}
            // (time, size, name, slot)
            (cmd::CHARACTER, 0, 4) => {
                let Some(body) = s(2) else {
                    return Err(host_error("character load without a name"));
                };
                scene.characters.insert(
                    n(3),
                    Character {
                        body,
                        face: None,
                        scale: n(1),
                    },
                );
            }
            // (time, slot, name)
            (cmd::CHARACTER, 1, 3) => {
                if let Some(c) = scene.characters.get_mut(&n(1)) {
                    c.face = s(2);
                }
            }
            (cmd::CHARACTER, 2, 0) => {}
            // (time, slot)
            (cmd::CHARACTER, 3, 2) => {
                scene.characters.remove(&n(1));
            }
            (cmd::CHARACTER, 4, 1) => scene.characters.clear(),
            (cmd::TEXT, 2, 1) => {
                let name = s(0).ok_or_else(|| host_error("text table without a name"))?;
                let map = MapFile::parse(&self.assets.data(&name)?)?;
                self.text_table = Some((name, map));
                vm.push(0);
            }
            (cmd::TEXT, 0, 1) => {
                let index = n(0).max(0) as usize;
                let section = self.config.language.map_section();
                let text = self
                    .text_table
                    .as_ref()
                    .and_then(|(_, m)| m.line(section, index))
                    .ok_or_else(|| host_error(format!("no text line {index}")))?;
                scene.text = Some(text.to_string());
                self.pause = Some(Pause::Text { index });
                self.handled += 1;
                return Ok(Flow::Yield);
            }
            (cmd::TITLE, _, _) => {
                vm.push(self.config.title_choice);
                self.pause = Some(Pause::Title);
                self.handled += 1;
                return Ok(Flow::Yield);
            }
            (cmd::LOCALIZE, _, 1) => {
                let name = s(0).ok_or_else(|| host_error("localize without a name"))?;
                let local = localized_name(&name, self.config.language);
                let found = [Kind::Layer, Kind::Movie]
                    .iter()
                    .any(|&k| self.assets.exists(k, &local));
                vm.push(if found { local } else { name }.as_str());
            }
            _ => {
                self.skip(format!("Game.{command}.{sub}"), vm, GAME, command);
                return Ok(Flow::Continue);
            }
        }
        self.handled += 1;
        Ok(Flow::Continue)
    }
}

impl Host for Runtime {
    fn native(&mut self, vm: &mut Vm, class: i32) -> silky_vm::Result<Flow> {
        match class {
            SCRIPT => {
                let sub = vm.pop_int()?;
                let file = vm.pop_str()?;
                let s = self.assets.script(&file)?;
                match sub {
                    script::JUMP => vm.jump_script(&file, s),
                    script::CALL | script::LIBRARY => vm.call_script(&file, s),
                    _ => return Err(host_error(format!("Script.{sub}"))),
                }
                self.handled += 1;
                Ok(Flow::Continue)
            }
            GAME => self.game(vm),
            _ => {
                let mut args = vm.pop_statement();
                let sub = match args.pop() {
                    Some(Value::Int(s)) => s,
                    _ => -1,
                };
                // Sprite.1 (layer, image): the title screen picture.
                if class == SPRITE && sub == 1 && args.len() == 2 {
                    let layer = args[0].int();
                    match args[1].as_str() {
                        Some(name) if !name.is_empty() => {
                            self.scene.layers.insert(layer, name.to_string());
                        }
                        _ => {
                            self.scene.layers.remove(&layer);
                        }
                    }
                    self.handled += 1;
                    return Ok(Flow::Continue);
                }
                let name = native::class_name(class).unwrap_or("?");
                self.skip(format!("{name}.{sub}"), vm, class, sub);
                Ok(Flow::Continue)
            }
        }
    }
}
