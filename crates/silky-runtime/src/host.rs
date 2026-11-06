use std::collections::BTreeMap;

use silky_format::game::Kind;
use silky_format::map::MapFile;
use silky_vm::native::{self, script, GAME, SCRIPT};
use silky_vm::{Flow, Host, Value, Vm};

use std::sync::Arc;

use crate::assets::{localized_name, Assets, Language};
use crate::audio::{Bus, Mixer, Pcm};
use crate::scene::{Character, Scene};

const MUSIC: i32 = 15;
const EFFECT: i32 = 16;
const VOICE: i32 = 18;
const SHAKE: i32 = 3;
const SPRITE: i32 = 19;
const SCREEN: i32 = 20;
const UI: i32 = 26;

/// Screen calls (class 20). Times are in frames of 1/60 s (provisional).
mod screen {
    /// `(time, colour)`: fade out to a colour.
    pub const FADE_OUT: i32 = 0;
    /// `(time)`: fade in from the colour.
    pub const FADE_IN: i32 = 1;
    /// `(time, rule, colour)`: fade out through a rule mask.
    pub const RULE_OUT: i32 = 2;
    /// `(time, rule)`: fade in through a rule mask.
    pub const RULE_IN: i32 = 3;
    /// `()`: whether the screen is faded out; returns 1 or 0.
    pub const COVERED: i32 = 4;
}

/// Interface calls (class 26).
mod ui_call {
    /// `(milliseconds)`: wait; a click ends it early.
    pub const WAIT: i32 = 0;
    /// `(input)`: one frame passes; returns 1 if the input happened
    /// (256 is a click; the others are not known yet and return 0).
    pub const POLL: i32 = 1;
}

/// Frames of 1/60 s to milliseconds.
fn frames_ms(frames: i32) -> u32 {
    frames.max(0) as u32 * 1000 / 60
}

/// Sound calls, the same for the music, effect and voice classes.
mod sound {
    /// `(count)`: number of channels.
    pub const CHANNELS: i32 = 0;
    /// `(channel, file)`: play; music loops.
    pub const PLAY: i32 = 1;
    /// `(channel)`: stop.
    pub const STOP: i32 = 2;
    /// `(channel, file)` on the voice class: play a voice line.
    pub const VOICE_PLAY: i32 = 5;
    /// `(channel)`: whether the channel still plays; returns 1 or 0.
    pub const PLAYING: i32 = 7;
    /// `(channel, time)`: fade out and stop. Time is taken as 1/60 s
    /// units (provisional: `music_fade` passes 60).
    pub const FADE_OUT: i32 = 3;
}

/// Game commands (class 29) this runtime knows.
mod cmd {
    /// Speaker nameplate: `Game.11(row)`, row of `nameplate_<lang>.akb`.
    pub const SPEAKER: i32 = 11;
    /// Message window portrait: 6 body, 7 face, 9 show, 11 hide.
    pub const PORTRAIT: i32 = 10;
    /// Background: 0 load, 1 difference picture, 2 and 3 transitions.
    pub const PICTURE: i32 = 12;
    /// Standing characters: 0 load, 1 face, 2 apply, 3 remove, 4 remove all.
    pub const CHARACTER: i32 = 13;
    /// Selection: `Game.20(question line, 9)` shows the options whose
    /// labels the option bodies put in `H[40 + i]`; returns the option
    /// picked, from 1.
    pub const SELECT: i32 = 20;
    /// Story text: 2 load a `.map` table, 0 show a line.
    pub const TEXT: i32 = 23;
    /// Title menu; returns the item picked.
    pub const TITLE: i32 = 110;
    /// Localized file name: `x.akb` -> `x_en.akb` when that exists.
    pub const LOCALIZE: i32 = 223;
}

/// What a save file holds besides its header.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveState {
    pub vm: silky_vm::Snapshot,
    pub scene: Scene,
    /// Name of the `.map` table in use.
    pub text_table: Option<String>,
    pub last_text: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub language: Language,
    /// Title menu result for frontends that do not show the menu (the
    /// headless runner): 1 start, 5 quit, 257 CG mode, 258 scene mode,
    /// 259 music mode.
    pub title_choice: i32,
    /// Sound output ([`Mixer::new`] in a frontend that plays sound).
    pub audio: Mixer,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::English,
            title_choice: 1,
            audio: Mixer::disabled(),
        }
    }
}

/// Why the runtime asked the interpreter to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pause {
    /// A message line is shown; the original waits for a click here.
    Text { index: usize },
    /// The title menu is up; answer with [`Runtime::answer_title`].
    Title,
    /// A selection is up ([`Runtime::choices`]); answer with
    /// [`Runtime::answer_choice`].
    Choice,
    /// The scene changed with a transition, or the script waits; the
    /// frontend shows it for this long, then runs the scripts again.
    Transition(Transition),
    /// The script polls input once per frame ([`Runtime::clicked`]).
    Frame,
}

/// How the last scene change is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub ms: u32,
    /// Rule mask number (`NN.bin`), or `None` for a crossfade.
    pub rule: Option<u32>,
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
    /// Position of the last text line shown in its table.
    last_text: Option<usize>,
    /// Number of options whose bodies ran before the next selection.
    option_count: usize,
    /// Labels of the selection that is up.
    pub choices: Vec<String>,
    /// Set by the frontend when the player clicked; read by input polls.
    pub clicked: bool,
    /// Longest time of the character changes since they were last
    /// applied (game command 13).
    character_time: i32,
    /// Sound output; disabled unless the frontend plays sound.
    pub audio: Mixer,
}

fn host_error(msg: impl Into<String>) -> silky_vm::Error {
    silky_vm::Error::Host(msg.into())
}

impl Runtime {
    pub fn new(assets: Assets, config: Config) -> Self {
        let audio = config.audio.clone();
        Self {
            assets,
            scene: Scene::default(),
            config,
            text_table: None,
            pause: None,
            unhandled: BTreeMap::new(),
            handled: 0,
            last_text: None,
            option_count: 0,
            choices: Vec::new(),
            clicked: false,
            character_time: 0,
            audio,
        }
    }

    /// Starts the game at its first script (`start.mes`).
    pub fn boot(&self, vm: &mut Vm, start: &str) -> crate::Result<()> {
        vm.jump_script(start, self.assets.script(start)?);
        Ok(())
    }

    /// Notes that the option bodies of a list are about to run (see
    /// [`Vm::run_all_options`]); the selection call that follows shows
    /// that many options.
    pub fn begin_options(&mut self, count: usize) {
        self.option_count = count;
    }

    /// Returns the option picked (from 0) to the script; call after
    /// [`Pause::Choice`].
    pub fn answer_choice(&mut self, vm: &mut Vm, index: usize) {
        self.choices.clear();
        vm.push(index as i32 + 1);
    }

    /// Game state for a save file: interpreter, scene and text table. Take
    /// it while a message is shown ([`Pause::Text`]).
    pub fn save_state(&self, vm: &Vm) -> SaveState {
        SaveState {
            vm: vm.snapshot(),
            scene: self.scene.clone(),
            text_table: self.text_table.as_ref().map(|t| t.0.clone()),
            last_text: self.last_text,
        }
    }

    /// Puts back a state from [`Runtime::save_state`]. The game goes on
    /// from the message that was shown.
    pub fn load_state(&mut self, vm: &mut Vm, state: SaveState) -> crate::Result<()> {
        let assets = &self.assets;
        vm.restore_snapshot(state.vm, &mut |name| {
            assets
                .script(name)
                .map_err(|e| silky_vm::Error::Host(e.to_string()))
        })
        .map_err(|e| crate::Error::Invalid(e.to_string()))?;
        self.text_table = match state.text_table {
            Some(name) => {
                let map = MapFile::parse(&self.assets.data(&name)?)?;
                Some((name, map))
            }
            None => None,
        };
        self.scene = state.scene;
        self.last_text = state.last_text;
        self.stop_sound();
        for (ch, file) in self.scene.music.clone() {
            self.play(Bus::Music, Kind::Music, ch, &file)?;
        }
        self.pause = None;
        self.option_count = 0;
        self.choices.clear();
        Ok(())
    }

    /// Stops all sound.
    pub fn stop_sound(&mut self) {
        for bus in [Bus::Music, Bus::Effect, Bus::Voice] {
            self.audio.stop_bus(bus);
        }
    }

    /// Clears the scene and script state for a fresh start.
    pub fn reset(&mut self) {
        self.stop_sound();
        self.scene = Scene::default();
        self.text_table = None;
        self.pause = None;
        self.last_text = None;
        self.option_count = 0;
        self.choices.clear();
        self.character_time = 0;
        self.clicked = false;
    }

    /// Counts a player action the runtime does not carry out yet.
    pub fn note_unhandled(&mut self, what: String) {
        *self.unhandled.entry(what).or_default() += 1;
    }

    /// Returns the title menu result to the script; call after
    /// [`Pause::Title`], before running the interpreter again.
    pub fn answer_title(&mut self, vm: &mut Vm, result: i32) {
        vm.push(result);
    }

    fn skip(&mut self, name: String, vm: &mut Vm, class: i32, sub: i32) {
        *self.unhandled.entry(name).or_default() += 1;
        if native::returns_value(class, sub) {
            vm.push(0);
        }
    }

    /// Pauses for a transition of `frames`, or goes on at once.
    fn transition(&mut self, frames: i32, rule: Option<u32>) -> Flow {
        let ms = frames_ms(frames);
        if ms == 0 {
            return Flow::Continue;
        }
        self.pause = Some(Pause::Transition(Transition { ms, rule }));
        Flow::Yield
    }

    /// Screen, interface and shake calls. `None` for calls it does not
    /// know.
    fn screen(&mut self, vm: &mut Vm, class: i32, sub: i32, args: &[Value]) -> Option<Flow> {
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        let rule = |i: usize| Some(n(i).max(0) as u32);
        let colour = |v: i32| v as u32 & 0xff_ffff;
        Some(match (class, sub, args.len()) {
            (SCREEN, screen::FADE_OUT, 2) => {
                self.scene.cover = Some(colour(n(1)));
                self.transition(n(0), None)
            }
            (SCREEN, screen::FADE_IN, 1) => {
                self.scene.cover = None;
                self.transition(n(0), None)
            }
            (SCREEN, screen::RULE_OUT, 3) => {
                self.scene.cover = Some(colour(n(2)));
                self.transition(n(0), rule(1))
            }
            (SCREEN, screen::RULE_IN, 2) => {
                if self.scene.cover.take().is_none() {
                    return Some(Flow::Continue);
                }
                self.transition(n(0), rule(1))
            }
            (SCREEN, screen::COVERED, 0) => {
                vm.push(self.scene.cover.is_some() as i32);
                Flow::Continue
            }
            (UI, ui_call::WAIT, 1) => {
                let ms = n(0).max(0) as u32;
                if ms > 0 {
                    self.pause = Some(Pause::Transition(Transition { ms, rule: None }));
                    Flow::Yield
                } else {
                    Flow::Continue
                }
            }
            (UI, ui_call::POLL, 1) => {
                let hit = n(0) == 256 && std::mem::take(&mut self.clicked);
                vm.push(hit as i32);
                self.pause = Some(Pause::Frame);
                Flow::Yield
            }
            // (dx, dy): shift the screen; 0 starts and 2 ends a shake.
            (SHAKE, 1, 2) => {
                self.scene.shake = (n(0), n(1));
                Flow::Continue
            }
            (SHAKE, 0 | 2, 0) => {
                self.scene.shake = (0, 0);
                Flow::Continue
            }
            _ => return None,
        })
    }

    /// Starts a sound, decoded on another thread. Music loops.
    fn play(&mut self, bus: Bus, kind: Kind, ch: i32, file: &str) -> crate::Result<()> {
        if !self.audio.enabled() {
            return Ok(());
        }
        let key = file.to_ascii_lowercase();
        let data = self.assets.read(kind, file)?;
        self.audio.play_later(bus, ch, bus == Bus::Music, move || {
            Ok(Arc::new(if key.ends_with(".ogg") {
                Pcm::from_ogg(&data)?
            } else {
                Pcm::from_wav(&data)?
            }))
        });
        Ok(())
    }

    /// Music, effect and voice calls. Returns false for calls it does not
    /// know.
    fn sound(
        &mut self,
        vm: &mut Vm,
        class: i32,
        sub: i32,
        args: &[Value],
    ) -> silky_vm::Result<bool> {
        let (bus, kind) = match class {
            MUSIC => (Bus::Music, Kind::Music),
            EFFECT => (Bus::Effect, Kind::Effect),
            _ => (Bus::Voice, Kind::Voice),
        };
        let ch = args.first().map(Value::int).unwrap_or(0);
        match (sub, args) {
            (sound::CHANNELS, [_]) => {}
            (sound::VOICE_PLAY, [_, Value::Str(file)]) if bus == Bus::Voice => {
                self.play(bus, kind, ch, file)?;
            }
            (sound::PLAY, [_, Value::Str(file)]) => {
                if bus == Bus::Music {
                    self.scene.music.insert(ch, file.clone());
                }
                self.play(bus, kind, ch, file)?;
            }
            (sound::STOP, [_]) => {
                self.audio.stop(bus, ch);
                if bus == Bus::Music {
                    self.scene.music.remove(&ch);
                }
            }
            (sound::FADE_OUT, [_, t]) => {
                self.audio
                    .fade_out(bus, ch, (t.int().max(0) as u32) * 1000 / 60);
                if bus == Bus::Music {
                    self.scene.music.remove(&ch);
                }
            }
            // Scripts poll this once a frame to wait for a sound to end.
            (sound::PLAYING, [_]) => vm.push(self.audio.is_playing(bus, ch) as i32),
            _ => return Ok(false),
        }
        Ok(true)
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
        let plain = [cmd::TITLE, cmd::LOCALIZE, cmd::SPEAKER];
        if sub >= 0 && !plain.contains(&command) {
            args.pop();
        }
        let s = |i: usize| args.get(i).and_then(Value::as_str).map(str::to_string);
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        if command == cmd::CHARACTER && matches!(sub, 0 | 1 | 3 | 4) {
            self.character_time = self.character_time.max(n(0));
        }
        let scene = &mut self.scene;
        match (command, sub, args.len()) {
            (cmd::SPEAKER, row, 1) if row >= 0 => scene.speaker = Some(row),
            (cmd::PORTRAIT, 6, 1) => scene.portrait = s(0).map(|b| (b, None)),
            (cmd::PORTRAIT, 7, 1) => {
                if let Some(p) = &mut scene.portrait {
                    p.1 = s(0);
                }
            }
            (cmd::PORTRAIT, 9, 0) => scene.portrait_visible = true,
            // Erase the message window.
            (cmd::PORTRAIT, 14, 0) => {
                scene.text = None;
                scene.speaker = None;
            }
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
            // 2 sets up the change, 11 closes it; nothing to draw.
            (cmd::PICTURE, 2, 0) | (cmd::PICTURE, 11, 0) => {}
            // (time): crossfade to the new picture.
            (cmd::PICTURE, 3, 1) => {
                self.handled += 1;
                return Ok(self.transition(n(0), None));
            }
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
            // Apply: crossfade over the longest time given.
            (cmd::CHARACTER, 2, 0) => {
                let t = std::mem::take(&mut self.character_time);
                self.handled += 1;
                return Ok(self.transition(t, None));
            }
            // (time, slot)
            (cmd::CHARACTER, 3, 2) => {
                scene.characters.remove(&n(1));
            }
            (cmd::CHARACTER, 4, 1) => scene.characters.clear(),
            (cmd::SELECT, _, 1) => {
                let section = self.config.language.map_section();
                let map = self.text_table.as_ref().map(|t| &t.1);
                let line = |pos: i32| {
                    map.and_then(|m| m.line(section, pos.max(0) as usize))
                        .map(str::to_string)
                };
                if let Some(q) = line(n(0)) {
                    scene.text = Some(q);
                }
                self.choices = (0..self.option_count)
                    .map(|i| line(vm.var_h(40 + i).int()).unwrap_or_else(|| format!("{}", i + 1)))
                    .collect();
                self.option_count = 0;
                self.pause = Some(Pause::Choice);
                self.handled += 1;
                return Ok(Flow::Yield);
            }
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
                self.last_text = Some(index);
                self.pause = Some(Pause::Text { index });
                self.handled += 1;
                return Ok(Flow::Yield);
            }
            // The result is pushed by [`Runtime::answer_title`] once the
            // player picks an entry.
            (cmd::TITLE, _, _) => {
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
                if matches!(class, SCREEN | UI | SHAKE) {
                    if let Some(flow) = self.screen(vm, class, sub, &args) {
                        self.handled += 1;
                        return Ok(flow);
                    }
                }
                if matches!(class, MUSIC | EFFECT | VOICE) && self.sound(vm, class, sub, &args)? {
                    self.handled += 1;
                    return Ok(Flow::Continue);
                }
                let name = native::class_name(class).unwrap_or("?");
                self.skip(format!("{name}.{sub}"), vm, class, sub);
                Ok(Flow::Continue)
            }
        }
    }

    /// Start of a message: the speaker is set again for each line.
    fn message(&mut self, _vm: &mut Vm, _number: u32) -> silky_vm::Result<Flow> {
        self.scene.speaker = None;
        // A new message cuts the voice of the last one.
        self.audio.stop_bus(Bus::Voice);
        Ok(Flow::Continue)
    }
}
