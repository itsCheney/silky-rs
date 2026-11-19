//! What one game adds to the engine.
//!
//! Silky's engine is shared by its games, but each game's executable also
//! carries code of its own: the commands of native class 29 (`Game`),
//! tables (gallery, music, scenes) and some screens. The shared engine
//! lives in the rest of this crate; each game's own part lives here, one
//! module per game. Each command is described in `docs/game-commands.md`,
//! with what it does and how sure that is.
//!
//! A `Game` call goes first to the module of the running game. What that
//! module leaves goes to [`Runtime::common_game`]: the commands of the
//! first game, ふゆから、くるる。, that are general enough to try on a game
//! not known yet. きまぐれテンプテーション2 takes only some of them
//! ([`kimaten2::SHARED_COMMANDS`]). What nobody handles is reported as not
//! carried out.

pub mod apeiria;
pub mod doukyuusei;
pub mod fuyukuru;
pub mod kimaten2;

use silky_format::game::{Kind, Title};
use silky_format::map::MapFile;
use silky_vm::native::GAME;
use silky_vm::{Flow, Value, Vm};

use crate::assets::localized_name;
use crate::host::{host_error, Extras, Jolt, Pause, Runtime};
use crate::scene::Character;

/// Game commands (class 29) of the common set.
pub(crate) mod cmd {
    /// Message window portrait: 6 body, 7 face, 9 show, 11 hide, 14 erase
    /// the message window.
    pub const PORTRAIT: i32 = 10;
    /// Speaker name plate: `Game.11(row)` (a row of the name plate
    /// sheet), or `Game.11(name)` in Japanese builds.
    pub const SPEAKER: i32 = 11;
    /// Background: 0 load, 1 difference picture, 2 and 3 transitions,
    /// 4 and 5 close-up, 6 back from a close-up, 11 close.
    pub const PICTURE: i32 = 12;
    /// Standing characters: 0 load, 14 load shifted, 1 face, 2 apply, 3
    /// remove, 4 remove all, 7 jolt, 10 move to another slot.
    pub const CHARACTER: i32 = 13;
    /// Selection: `Game.20(question line, 9)` shows the options whose
    /// labels the option bodies put in `H[40 + i]`, or `Game.20(style,
    /// question)` with the labels the option bodies left; returns the
    /// option picked, from 1.
    pub const SELECT: i32 = 20;
    /// Story text: 2 load a `.map` table, 0 show a line.
    pub const TEXT: i32 = 23;
    /// Title menu; returns the item picked.
    pub const TITLE: i32 = 110;
    /// Localized file name: `x.akb` -> `x_en.akb` when that exists.
    pub const LOCALIZE: i32 = 223;
}

/// A `Game` call: the command, its sub command (`-1` without one) and the
/// values before it.
pub(crate) struct Call {
    pub command: i32,
    pub sub: i32,
    pub args: Vec<Value>,
}

impl Call {
    pub fn int(&self, i: usize) -> i32 {
        self.args.get(i).map(Value::int).unwrap_or(0)
    }

    pub fn str(&self, i: usize) -> Option<String> {
        self.args.get(i).and_then(Value::as_str).map(str::to_string)
    }
}

impl Runtime {
    /// Carries out a `Game` call.
    pub(crate) fn game(&mut self, vm: &mut Vm) -> silky_vm::Result<Flow> {
        let mut args = vm.pop_statement();
        let Some(Value::Int(command)) = args.pop() else {
            return Err(host_error("game call without a command"));
        };
        let sub = match args.last() {
            Some(Value::Int(s)) => *s,
            _ => -1,
        };
        let title = self.assets.game().title;
        if title == Title::Kimaten2 {
            if let Some(flow) = self.kimaten2_game(vm, command, sub, &args) {
                return flow;
            }
            if !kimaten2::SHARED_COMMANDS.contains(&command) {
                self.skip(format!("Game.{command}"), vm, GAME, command);
                return Ok(Flow::Continue);
            }
        }
        let plain = [cmd::TITLE, cmd::LOCALIZE, cmd::SPEAKER];
        // A selection's last value is its window style (9, or -1 for the
        // default), not a sub command.
        if (sub >= 0 || command == cmd::SELECT) && !plain.contains(&command) {
            args.pop();
        }
        let call = Call { command, sub, args };
        if title == Title::Doukyuusei {
            if let Some(flow) = self.doukyuusei_game(vm, &call) {
                return flow;
            }
            // Its commands are numbered its own way. Backgrounds (12) take
            // the common calls; the rest are not read from its executable
            // yet.
            if command == cmd::PICTURE {
                if let Some(flow) = self.common_game(vm, &call) {
                    return flow;
                }
            }
            self.skip(format!("Game.{command}.{sub}"), vm, GAME, command);
            return Ok(Flow::Continue);
        }
        if title == Title::Fuyukuru {
            if let Some(flow) = self.fuyukuru_game(vm, &call) {
                return flow;
            }
        }
        if title == Title::Apeiria {
            if let Some(flow) = self.apeiria_game(vm, &call) {
                return flow;
            }
        }
        if let Some(flow) = self.common_game(vm, &call) {
            return flow;
        }
        self.skip(format!("Game.{command}.{sub}"), vm, GAME, command);
        Ok(Flow::Continue)
    }

    /// Shows line `index` of the loaded text table and waits for the
    /// player.
    fn show_text(&mut self, index: usize) -> silky_vm::Result<Flow> {
        let section = self.config.language.map_section();
        let text = self
            .text_table
            .as_ref()
            .and_then(|(_, m)| m.line(section, index))
            .ok_or_else(|| host_error(format!("no text line {index}")))?;
        self.scene.text = Some(text.to_string());
        self.last_text = Some(index);
        let key = format!(
            "{}:{index}",
            self.text_table
                .as_ref()
                .map_or("", |t| t.0.as_str())
                .to_ascii_lowercase()
        );
        self.read_before = !self.system.read.insert(key);
        self.system_changed |= !self.read_before;
        self.pause = Some(Pause::Text { index });
        self.handled += 1;
        Ok(Flow::Yield)
    }

    /// Counts a call as carried out and goes on.
    pub(crate) fn done(&mut self) -> Option<silky_vm::Result<Flow>> {
        self.handled += 1;
        Some(Ok(Flow::Continue))
    }

    /// Counts a call as carried out and waits for the frontend.
    pub(crate) fn wait_for(&mut self, pause: Pause) -> Option<silky_vm::Result<Flow>> {
        self.pause = Some(pause);
        self.handled += 1;
        Some(Ok(Flow::Yield))
    }

    /// The common set of `Game` commands. `None` for the others.
    fn common_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let (command, sub) = (c.command, c.sub);
        // Character changes are applied together by CHARACTER.2, over the
        // longest time given.
        if command == cmd::CHARACTER && matches!(sub, 0 | 1 | 3 | 4 | 10) {
            self.character_time = self.character_time.max(c.int(0));
        }
        if command == cmd::CHARACTER && sub == 14 {
            self.character_time = self.character_time.max(c.int(2));
        }
        let scene = &mut self.scene;
        match (command, sub, c.args.len()) {
            (cmd::PORTRAIT, 6, 1) => scene.portrait = c.str(0).map(|b| (b, None)),
            (cmd::PORTRAIT, 7, 1) => {
                if let Some(p) = &mut scene.portrait {
                    p.1 = c.str(0);
                }
            }
            (cmd::PORTRAIT, 9, 0) => scene.portrait_visible = true,
            (cmd::PORTRAIT, 14, 0) => {
                scene.text = None;
                scene.speaker = None;
            }
            (cmd::PORTRAIT, 11, 0) => scene.portrait_visible = false,
            (cmd::SPEAKER, row, 1) if row >= 0 => scene.speaker = Some(row),
            (cmd::SPEAKER, _, 1) if c.str(0).is_some() => scene.speaker_name = c.str(0),
            // Japanese builds: (style, question); the labels are the
            // strings the option bodies left.
            (cmd::SELECT, _, 2) if c.str(1).is_some() => {
                scene.text = c.str(1).filter(|q| !q.is_empty());
                self.choices = std::mem::take(&mut vm.option_labels);
                self.option_count = 0;
                return self.wait_for(Pause::Choice);
            }
            // (question line): the question and option labels are lines of
            // the loaded text table.
            (cmd::SELECT, _, 1) => {
                let section = self.config.language.map_section();
                let map = self.text_table.as_ref().map(|t| &t.1);
                let line = |pos: i32| {
                    map.and_then(|m| m.line(section, pos.max(0) as usize))
                        .map(str::to_string)
                };
                if let Some(q) = line(c.int(0)) {
                    scene.text = Some(q);
                }
                self.choices = (0..self.option_count)
                    .map(|i| line(vm.var_h(40 + i).int()).unwrap_or_else(|| format!("{}", i + 1)))
                    .collect();
                self.option_count = 0;
                return self.wait_for(Pause::Choice);
            }
            // Back to the whole picture after a close-up.
            (cmd::PICTURE, 6, 0) => {
                scene.background_offset = (0, 0);
                scene.background_zoom = 0;
            }
            // 10 (file): a background too. きまぐれテンプテーション2 puts
            // its event pictures up so; ふゆから、くるる。 asks for its
            // running scenes so when movies are on, and the picture
            // stands still here, as with movies off.
            (cmd::PICTURE, 0 | 10, 1) => {
                scene.background = c.str(0);
                scene.town = None;
                scene.effects.background_changed();
                if let Some(name) = &scene.background {
                    self.system
                        .seen
                        .insert(crate::assets::with_ext(&name.to_ascii_lowercase()));
                    self.system_changed = true;
                }
                scene.overlays.clear();
                scene.anime.clear_shown();
                // Provisional: the title picture sits in a sprite layer;
                // the first background of the game replaces it.
                scene.layers.clear();
            }
            // (slot, parameter, name)
            (cmd::PICTURE, 1, 3) => match c.str(2).filter(|v| !v.is_empty()) {
                Some(name) => {
                    self.system
                        .seen
                        .insert(crate::assets::with_ext(&name.to_ascii_lowercase()));
                    self.system_changed = true;
                    scene.overlays.insert(c.int(0), name);
                }
                None => {
                    scene.overlays.remove(&c.int(0));
                }
            },
            // Close-ups: the scripts give y before x (as in Sprite.0,
            // height before width); the engine stores them as x, y and
            // places the picture's top left corner there.
            (cmd::PICTURE, 4, 2) => scene.background_offset = (c.int(1), c.int(0)),
            // Size in percent; the picture grows from its top left corner.
            (cmd::PICTURE, 5, 1) => scene.background_zoom = c.int(0),
            // 2 sets up the change, 11 closes it; nothing to draw.
            (cmd::PICTURE, 2, 0) | (cmd::PICTURE, 11, 0) => {
                if sub == 2 {
                    self.register_cg(vm);
                }
            }
            // (time): crossfade to the new picture.
            (cmd::PICTURE, 3, 1) => {
                self.register_cg(vm);
                self.handled += 1;
                return Some(Ok(self.transition(c.int(0), None)));
            }
            // (time, size, name, slot)
            (cmd::CHARACTER, 0, 4) => {
                let Some(body) = c.str(2) else {
                    return Some(Err(host_error("character load without a name")));
                };
                let character = Character {
                    body,
                    face: None,
                    scale: c.int(1),
                    offset: (0, 0),
                    cheeks: None,
                };
                scene.characters.insert(c.int(3), character);
            }
            // (x, y, time, size, name, slot): a character shifted from its
            // slot position.
            (cmd::CHARACTER, 14, 6) => {
                let Some(body) = c.str(4) else {
                    return Some(Err(host_error("character load without a name")));
                };
                let character = Character {
                    body,
                    face: None,
                    scale: c.int(3),
                    offset: (c.int(0), c.int(1)),
                    cheeks: None,
                };
                scene.characters.insert(c.int(5), character);
            }
            // (pixels, kind, slot): a short jolt, four steps of an offset
            // table; kind 0 hops up (0, -p/2, -p, -p/2), else it shakes
            // sideways (-p, +p, 0, 0).
            (cmd::CHARACTER, 7, 3) => {
                let p = c.int(0);
                let steps = if c.int(1) == 0 {
                    vec![(0, -p / 2), (0, -p), (0, -p / 2), (0, 0)]
                } else {
                    vec![(-p, 0), (p, 0), (0, 0), (0, 0)]
                };
                // A step of the engine's sprite animation task, which runs
                // once more than 15 ms have passed: 16 ms.
                self.jolts.push((c.int(2), Jolt { steps, step_ms: 16 }));
            }
            // (time, new slot, slot): a character moves to another slot.
            (cmd::CHARACTER, 10, 3) => {
                if let Some(ch) = scene.characters.remove(&c.int(2)) {
                    scene.characters.insert(c.int(1), ch);
                }
            }
            // (time, slot, name)
            (cmd::CHARACTER, 1, 3) => {
                if let Some(ch) = scene.characters.get_mut(&c.int(1)) {
                    ch.face = c.str(2);
                }
            }
            // Apply: crossfade over the longest time given.
            (cmd::CHARACTER, 2, 0) => {
                let t = std::mem::take(&mut self.character_time);
                self.handled += 1;
                return Some(Ok(self.transition(t, None)));
            }
            // (time, slot)
            (cmd::CHARACTER, 3, 2) => {
                scene.characters.remove(&c.int(1));
            }
            (cmd::CHARACTER, 4, 1) => scene.characters.clear(),
            (cmd::TEXT, 2, 1) => {
                let Some(name) = c.str(0) else {
                    return Some(Err(host_error("text table without a name")));
                };
                let map = match self
                    .assets
                    .data(&name)
                    .and_then(|b| Ok(MapFile::parse(&b)?))
                {
                    Ok(m) => m,
                    Err(e) => return Some(Err(e.into())),
                };
                self.text_table = Some((name, map));
                vm.push(0);
            }
            (cmd::TEXT, 0, 1) => return Some(self.show_text(c.int(0).max(0) as usize)),
            // Extras screens, CG (30), scenes (31) and music (32); the
            // result is pushed by
            // [`Runtime::answer_extras`].
            (30..=32, _, _) => {
                return self.wait_for(Pause::Extras(match command {
                    30 => Extras::Gallery,
                    31 => Extras::Scenes,
                    _ => Extras::Music,
                }));
            }
            // The result is pushed by [`Runtime::answer_title`] once the
            // player picks an entry.
            (cmd::TITLE, _, _) => return self.wait_for(Pause::Title),
            // Also used inside the arguments of another call, as in
            // Game.12(0, 1, Game.223("x.akb"), 1): the values before the
            // name belong to the outer call and go back on the stack.
            (cmd::LOCALIZE, _, len) if len >= 1 && matches!(c.args.last(), Some(Value::Str(_))) => {
                let name = match c.str(len - 1) {
                    Some(n) => n,
                    None => return Some(Err(host_error("localize without a name"))),
                };
                for v in &c.args[..len - 1] {
                    vm.push(v.clone());
                }
                let local = localized_name(&name, self.config.language);
                let found = [Kind::Layer, Kind::Movie]
                    .iter()
                    .any(|&k| self.assets.exists(k, &local));
                vm.push(if found { local } else { name }.as_str());
            }
            _ => return None,
        }
        self.done()
    }
}
