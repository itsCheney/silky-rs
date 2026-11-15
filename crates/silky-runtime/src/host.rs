use std::collections::BTreeMap;

use silky_format::game::{Kind, Title};
use silky_format::map::MapFile;
use silky_vm::native::{self, script, GAME, SCRIPT};
use silky_vm::{Flow, Host, Value, Vm};

use std::sync::Arc;

use crate::assets::{Assets, Language};
use crate::audio::{Bus, Mixer, Pcm};
use crate::scene::Scene;

const MUSIC: i32 = 15;
const EFFECT: i32 = 16;
const VOICE: i32 = 18;
const SHAKE: i32 = 3;
const SPRITE: i32 = 19;
const SCREEN: i32 = 20;
const UI: i32 = 26;
const MOVIE: i32 = 23;
const HEFFECT: i32 = 17;
/// Effect bus channel of HEffect channel 0.
const HEFFECT_CHANNELS: i32 = 100;
const MESWND: i32 = 2;
const MOUSE: i32 = 12;
const FLAG: i32 = 14;
const BACKLOG: i32 = 22;

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
pub(crate) fn frames_ms(frames: i32) -> u32 {
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
    /// An extras screen is up (game commands 30, 31, 32); answer with
    /// [`Runtime::answer_extras`].
    Extras(Extras),
    /// A movie plays (`Movie.2`); the frontend shows it, then runs the
    /// scripts again.
    Movie,
    /// きまぐれテンプテーション2's action menu is up ([`Runtime::actions`]);
    /// answer with [`Runtime::answer_action`].
    Actions,
    /// きまぐれテンプテーション2's map is up ([`Runtime::map`]); answer
    /// with [`Runtime::answer_action`].
    Map,
    /// きまぐれテンプテーション2's H mode (`Game.49`, see
    /// [`crate::games::kimaten2::hmode`]); the scripts go on when it ends.
    HMode,
}

/// A button of きまぐれテンプテーション2's action menu (`Game.42`).
///
/// The script puts up to seven buttons in byte variables 0..7 before the
/// call: 0 for none, else the icon (1 着替える .. 11 解除する) plus 50
/// for a grey tick or 100 for a pink one (done). The call returns the
/// variable's number plus one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionButton {
    pub result: i32,
    pub icon: u32,
    /// 0 none, 1 grey tick, 2 pink tick.
    pub tick: u32,
}

/// The three extras screens of the title menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extras {
    Gallery,
    Scenes,
    Music,
}

/// How the last scene change is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub ms: u32,
    /// Rule mask number (`NN.bin`), or `None` for a crossfade.
    pub rule: Option<u32>,
}

/// A character jolt: offsets, one for each step of `step_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jolt {
    pub steps: Vec<(i32, i32)>,
    pub step_ms: u64,
}

/// Mixer channel of movie sound.
const MOVIE_AUDIO: i32 = 99;

pub struct Runtime {
    pub assets: Assets,
    pub scene: Scene,
    pub config: Config,
    pub(crate) text_table: Option<(String, MapFile)>,
    /// Set when a callback yields.
    pub pause: Option<Pause>,
    /// Engine calls not carried out yet, by name, with counts.
    pub unhandled: BTreeMap<String, u64>,
    /// Sound files the scripts asked for that are not in the game.
    pub missing_sounds: std::collections::BTreeSet<String>,
    /// Engine calls carried out.
    pub handled: u64,
    /// Position of the last text line shown in its table.
    pub(crate) last_text: Option<usize>,
    /// Number of options whose bodies ran before the next selection.
    pub(crate) option_count: usize,
    /// Labels of the selection that is up.
    pub choices: Vec<String>,
    /// Set by the frontend when the player clicked; read by input polls.
    pub clicked: bool,
    /// Longest time of the character changes since they were last
    /// applied (game command 13).
    pub(crate) character_time: i32,
    /// Sound output; disabled unless the frontend plays sound.
    pub audio: Mixer,
    /// Voice file of the current message, for the backlog.
    pub voice: Option<String>,
    /// Mouth track of the voice playing (きまぐれテンプテーション2, command
    /// 40 sub 16): `face_talk` values, 60 a second, and the milliseconds
    /// played.
    pub lip: Option<(Vec<f32>, f32)>,
    /// The last mouth track, for the voice loop.
    pub(crate) lip_track: Option<Vec<f32>>,
    /// Settings and what was seen, kept across saves.
    pub system: crate::system::SystemData,
    /// Set when the system data changed and should be stored.
    pub system_changed: bool,
    /// Whether the current message was read before.
    pub read_before: bool,
    /// Movie file set up by `Movie.0`, played by `Movie.2`.
    pub movie: Option<String>,
    /// Character jolts (command 13, sub 7) for the frontend to play: slot
    /// and four offsets.
    pub jolts: Vec<(i32, Jolt)>,
    /// Text of the next message (Japanese builds put it in the script and
    /// pass it to `BackLog.3` before `MesWnd.3` shows it).
    pub(crate) pending_text: Option<String>,
    /// Script of the current message, for its read key (the text is shown
    /// from a library function, so the VM is in `liblary.lib` by then).
    pub(crate) message_script: String,
    /// Buttons of the action menu that is up.
    pub actions: Vec<ActionButton>,
    /// The map that is up.
    pub map: Option<crate::games::kimaten2::map::Map>,
    /// Sprite animations (class 19, subs 16 to 28).
    pub(crate) sprites: crate::sprite::Sprites,
    /// Second step of an effect, after the transition that started it.
    pub(crate) then: Option<FollowUp>,
    /// Pictures of the `.vo` timeline still to come, with their time
    /// from now in milliseconds (ふゆから、くるる。, command 1007), for the
    /// frontend to show.
    pub vo_steps: Vec<(u32, crate::games::fuyukuru::vo::Step)>,
    /// Size of the screen quake to start (command 993, sub 2).
    pub(crate) quake_mode: i32,
    /// Music set up by `Music.5`, by channel, for `Music.4` to start.
    pub(crate) queued_music: BTreeMap<i32, String>,
    /// Set by the game's auto save point (ふゆから、くるる。, command 14);
    /// the frontend saves at the next message.
    pub auto_save: bool,
    /// What 景の海のアペイリア's `Game.12` has put up, for its CG mode.
    pub(crate) cg_track: crate::games::apeiria::omake::CgTrack,
}

/// Second step of a two-step effect.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FollowUp {
    /// Take the flash picture away through a rule mask.
    EndFlash { frames: i32, rule: u32 },
    /// Back to normal size, over `frames`.
    EndZoom { frames: i32 },
    /// "pic_flush": from the enlarged screen to white for a frame.
    FlushWhite,
    /// "pic_flush": from white back to the scene over `frames`.
    FlushEnd { frames: i32 },
}

pub(crate) fn host_error(msg: impl Into<String>) -> silky_vm::Error {
    silky_vm::Error::Host(msg.into())
}

impl Runtime {
    pub fn new(assets: Assets, config: Config) -> Self {
        let audio = config.audio.clone();
        let system = crate::system::SystemData::for_title(assets.game().title);
        Self {
            assets,
            scene: Scene::default(),
            config,
            text_table: None,
            pause: None,
            unhandled: BTreeMap::new(),
            missing_sounds: Default::default(),
            handled: 0,
            last_text: None,
            option_count: 0,
            choices: Vec::new(),
            clicked: false,
            character_time: 0,
            jolts: Vec::new(),
            audio,
            voice: None,
            lip: None,
            lip_track: None,
            system,
            system_changed: false,
            read_before: false,
            movie: None,
            pending_text: None,
            message_script: String::new(),
            actions: Vec::new(),
            map: None,
            sprites: Default::default(),
            then: None,
            vo_steps: Vec::new(),
            quake_mode: 0,
            queued_music: BTreeMap::new(),
            auto_save: false,
            cg_track: Default::default(),
        }
    }

    /// Runs the second step of an effect if one is due: changes the scene
    /// and sets [`Runtime::pause`] to its transition (or leaves it empty
    /// when effects are turned off). False if none is due.
    pub fn follow_up(&mut self) -> bool {
        let Some(step) = self.then.take() else {
            return false;
        };
        let flow = match step {
            FollowUp::EndFlash { frames, rule } => {
                self.scene.flash = None;
                self.transition(frames, Some(rule))
            }
            FollowUp::EndZoom { frames } => {
                self.scene.zoom = None;
                self.transition(frames, None)
            }
            FollowUp::FlushWhite => {
                self.scene.zoom = None;
                self.scene.cover = Some(0xff_ffff);
                self.then = Some(FollowUp::FlushEnd { frames: 16 });
                self.transition(1, None)
            }
            FollowUp::FlushEnd { frames } => {
                self.scene.cover = None;
                self.transition(frames, None)
            }
        };
        if matches!(flow, Flow::Continue) {
            self.pause = None;
        }
        true
    }

    /// Script the current message is in (lowercase).
    pub fn message_script(&self) -> &str {
        &self.message_script
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
        self.lip = None;
        self.jolts.clear();
        self.clicked = false;
    }

    /// Returns the result of an extras screen to the script: -1 to leave,
    /// or the scene to replay.
    pub fn answer_extras(&mut self, vm: &mut Vm, result: i32) {
        vm.push(result);
    }

    /// Returns from a movie: `played` is false if it could not be shown.
    /// The scripts show the movie's still picture (and after the ending,
    /// the end card) when the result is 1.
    pub fn answer_movie(&mut self, vm: &mut Vm, played: bool) {
        self.audio.stop(Bus::Music, MOVIE_AUDIO);
        vm.push(played as i32);
    }

    /// Opens the movie set up with `Movie.0` and starts its sound.
    pub fn open_movie(&mut self) -> crate::Result<crate::movie::Movie> {
        let name = self
            .movie
            .clone()
            .ok_or_else(|| crate::Error::Invalid("no movie set up".into()))?;
        let file = self.assets.read(Kind::Movie, &name)?;
        let stream = Arc::new(crate::movie::mpeg_stream(&file)?);
        let movie = crate::movie::Movie::open(stream.clone())?;
        if self.audio.enabled() {
            self.audio
                .play_later(Bus::Music, MOVIE_AUDIO, false, 1.0, move || {
                    crate::movie::decode_audio(stream).map(Arc::new)
                });
        }
        Ok(movie)
    }

    /// Position of the movie sound in ms, while it plays; `Some(None)`
    /// while it still decodes.
    pub fn movie_clock(&self) -> Option<Option<f64>> {
        match self.audio.position(Bus::Music, MOVIE_AUDIO) {
            Some((pos, _)) => Some(Some(pos * 1000.0)),
            None if self.audio.is_playing(Bus::Music, MOVIE_AUDIO) => Some(None),
            None => None,
        }
    }

    /// Applies the volume settings to the mixer.
    pub fn apply_volumes(&self) {
        let st = &self.system.settings;
        let master = if st.master_on { st.master } else { 0.0 };
        let v = |on: bool, x: f32| if on { x * master } else { 0.0 };
        self.audio.set_volume(Bus::Music, v(st.music_on, st.music));
        self.audio
            .set_volume(Bus::Effect, v(st.effects_on, st.effects));
        self.audio.set_volume(Bus::Voice, v(st.voice_on, st.voice));
    }

    /// Plays a music track from the music room, looping.
    pub fn play_music(&mut self, file: &str) -> crate::Result<()> {
        self.audio.stop_bus(Bus::Music);
        self.play(Bus::Music, Kind::Music, 0, file)
    }

    pub fn stop_music(&mut self) {
        self.audio.stop_bus(Bus::Music);
    }

    /// Re-reads the current message in the configured language.
    pub fn set_language(&mut self, language: Language) {
        self.config.language = language;
        let section = language.map_section();
        if let (Some(index), Some((_, map))) = (self.last_text, &self.text_table) {
            if self.scene.text.is_some() {
                if let Some(t) = map.line(section, index) {
                    self.scene.text = Some(t.to_string());
                }
            }
        }
    }

    /// Plays a voice line again (backlog), cutting the one playing.
    pub fn replay_voice(&mut self, file: &str) -> crate::Result<()> {
        self.audio.stop_bus(Bus::Voice);
        self.play(Bus::Voice, Kind::Voice, 0, file)
    }

    /// Plays the mouth track of the voice again from its start.
    pub fn restart_lip(&mut self) {
        if let Some((_, played)) = &mut self.lip {
            *played = 0.0;
        } else if let Some(track) = self.lip_track.clone() {
            self.lip = Some((track, 0.0));
        }
    }

    /// Plays a sound effect.
    pub fn play_sound(&mut self, file: &str) -> crate::Result<()> {
        self.play(Bus::Effect, Kind::Effect, 0, file)
    }

    /// Stops the voice.
    pub fn stop_sound_voice(&mut self) {
        self.audio.stop_bus(Bus::Voice);
    }

    /// Whether a voice line is playing.
    pub fn voice_playing(&self) -> bool {
        self.audio.is_playing(Bus::Voice, 0)
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

    /// Answers the action menu or the map with a button's result.
    pub fn answer_action(&mut self, vm: &mut Vm, result: i32) {
        self.actions.clear();
        self.map = None;
        vm.push(result);
    }

    /// Sprite calls of ふゆから、くるる。 besides loading the title
    /// picture (see [`crate::sprite`]). `None` for calls it does not know.
    pub(crate) fn sprite(&mut self, vm: &mut Vm, sub: i32, args: &[Value]) -> Option<Flow> {
        use crate::sprite::{Call, Effect};
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        match (sub, args.len()) {
            // Set up at the start: create (0), clear (2), order (13).
            (0, 5) | (2, 1) | (13, 2) => return Some(Flow::Continue),
            // (sprite): hide (10), show (11). Hiding sprite 6 takes the
            // dimming away.
            (10, 1) => {
                if n(0) == 6 {
                    self.scene.dim = None;
                }
                return Some(Flow::Continue);
            }
            (11, 1) => return Some(Flow::Continue),
            // Copy between sprites, at every picture change: the new
            // picture is drawn over sprite 6, which ends a dimming.
            (3, 9) => {
                self.scene.dim = None;
                return Some(Flow::Continue);
            }
            _ => {}
        }
        match self.sprites.call(sub, args)? {
            Call::Done => Some(Flow::Continue),
            Call::Value(v) => {
                vm.push(v);
                Some(Flow::Continue)
            }
            Call::Run(effects) => {
                let mut flow = Flow::Continue;
                for e in effects {
                    flow = match e {
                        Effect::Wipe { frames, rule } => self.transition(frames, Some(rule)),
                        Effect::Dim {
                            frames,
                            color,
                            opacity,
                        } => {
                            self.scene.dim = Some((color, opacity));
                            self.transition(frames, None)
                        }
                    };
                }
                Some(flow)
            }
        }
    }

    pub(crate) fn skip(&mut self, name: String, vm: &mut Vm, class: i32, sub: i32) {
        *self.unhandled.entry(name).or_default() += 1;
        if native::returns_value(
            self.assets.game().title,
            &self.assets.game().engine,
            class,
            sub,
        ) {
            vm.push(0);
        }
    }

    /// Pauses for a transition of `frames`, or goes on at once.
    pub(crate) fn transition(&mut self, frames: i32, rule: Option<u32>) -> Flow {
        let scale = self.system.settings.effect_scale();
        let ms = (frames_ms(frames) as f32 * scale) as u32;
        if ms == 0 {
            return Flow::Continue;
        }
        self.pause = Some(Pause::Transition(Transition { ms, rule }));
        Flow::Yield
    }

    /// Screen, interface and shake calls. `None` for calls it does not
    /// know.
    pub(crate) fn screen(
        &mut self,
        vm: &mut Vm,
        class: i32,
        sub: i32,
        args: &[Value],
    ) -> Option<Flow> {
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
    pub(crate) fn play(&mut self, bus: Bus, kind: Kind, ch: i32, file: &str) -> crate::Result<()> {
        if !self.audio.enabled() {
            return Ok(());
        }
        let key = file.to_ascii_lowercase();
        // Some sounds the scripts play are not in the archives (in
        // きまぐれテンプテーション2 `ses_007_03a.wav` in A01.mes); the
        // original plays nothing then and goes on.
        let data = match self.assets.read(kind, file) {
            Err(crate::Error::Missing(_)) => {
                self.missing_sounds.insert(key);
                return Ok(());
            }
            r => r?,
        };
        let gain = match bus {
            Bus::Voice => {
                let st = &self.system.settings;
                let g = match self.assets.game().title {
                    silky_format::game::Title::Kimaten2 => {
                        crate::system::Settings::voice_group_of_kimaten2(file)
                    }
                    _ => crate::system::Settings::voice_group_of(file),
                };
                if st.voice_group_on[g] {
                    st.voice_group[g]
                } else {
                    0.0
                }
            }
            _ => 1.0,
        };
        self.audio
            .play_later(bus, ch, bus == Bus::Music, gain, move || {
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
    pub(crate) fn sound(
        &mut self,
        vm: &mut Vm,
        class: i32,
        sub: i32,
        args: &[Value],
    ) -> silky_vm::Result<bool> {
        let (bus, kind) = match class {
            MUSIC => (Bus::Music, Kind::Music),
            EFFECT | HEFFECT => (Bus::Effect, Kind::Effect),
            _ => (Bus::Voice, Kind::Voice),
        };
        let mut ch = args.first().map(Value::int).unwrap_or(0);
        // HEffect is a second set of effect channels (景の海のアペイリア
        // plays its sound loops there); they share the effect volume.
        if class == HEFFECT {
            ch += HEFFECT_CHANNELS;
        }
        match (sub, args) {
            (sound::CHANNELS, [_]) => {}
            (sound::VOICE_PLAY, [_, Value::Str(file)]) if bus == Bus::Voice => {
                self.voice = Some(file.clone());
                self.play(bus, kind, ch, file)?;
            }
            (sound::PLAY, [_, Value::Str(file)]) => {
                if bus == Bus::Music {
                    self.scene.music.insert(ch, file.clone());
                    if self.system.music.insert(file.to_ascii_lowercase()) {
                        self.system_changed = true;
                    }
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
            // (channel, file): music to start with the next `Music.4`.
            (5, [_, Value::Str(file)]) if bus == Bus::Music => {
                self.queued_music.insert(ch, file.clone());
            }
            // (channel, time): start the queued music. The original fades
            // it in over the time; it starts at full volume here.
            (4, [_, _]) if bus == Bus::Music => {
                if let Some(file) = self.queued_music.remove(&ch) {
                    self.scene.music.insert(ch, file.clone());
                    if self.system.music.insert(file.to_ascii_lowercase()) {
                        self.system_changed = true;
                    }
                    self.play(bus, kind, ch, &file)?;
                }
            }
            // Scripts poll this once a frame to wait for a sound to end.
            (sound::PLAYING, [_]) => vm.push(self.audio.is_playing(bus, ch) as i32),
            _ => return Ok(false),
        }
        Ok(true)
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
                // Sprite.1 (layer, image): the title screen picture. In
                // the 2024 build it loads the title's button sheet, which
                // the title screen draws in cells itself. The sprite and
                // flag calls below follow the build's profile.
                let title = self.assets.game().title;
                let engine = self.assets.game().engine.profile;
                let base = engine.sprites_and_flags;
                if class == SPRITE && sub == 1 && args.len() == 2 && base {
                    let layer = args[0].int();
                    // The 2017 build creates sprite 5 hidden and 2048
                    // pixels square: a store for button sheets
                    // (title_p.akb), which its screens draw in cells.
                    if engine.sheet_store == Some(layer) {
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
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
                if class == SPRITE && base {
                    if let Some(flow) = self.sprite(vm, sub, &args) {
                        self.handled += 1;
                        return Ok(flow);
                    }
                }
                if matches!(class, SCREEN | UI | SHAKE) {
                    if let Some(flow) = self.screen(vm, class, sub, &args) {
                        self.handled += 1;
                        return Ok(flow);
                    }
                }
                // Japanese builds: the message text arrives as BackLog.3
                // (text), and MesWnd.3 (message, 0) shows it and waits;
                // its result 0 means "go on".
                // A name the script keeps in a string variable comes as a
                // string of its own between the pieces of text.
                if class == BACKLOG && sub == 3 {
                    // Without text: the end of the entry's voice mark.
                    if args.is_empty() && engine.backlog_voice_end {
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
                    if !args.is_empty() && args.iter().all(|a| matches!(a, Value::Str(_))) {
                        let text: String = args.iter().filter_map(Value::as_str).collect();
                        self.pending_text = Some(text);
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
                }
                if class == MESWND && sub == 3 {
                    if let Some(text) = self.pending_text.take() {
                        let index = args.first().map(Value::int).unwrap_or(0).max(0) as usize;
                        self.scene.text = Some(text);
                        if self.message_script.is_empty() {
                            self.message_script =
                                vm.script_name().unwrap_or("").to_ascii_lowercase();
                        }
                        let key = format!("{}:{index}", self.message_script);
                        self.read_before = !self.system.read.insert(key);
                        self.system_changed |= !self.read_before;
                        self.pause = Some(Pause::Text { index });
                        vm.push(0);
                        self.handled += 1;
                        return Ok(Flow::Yield);
                    }
                }
                // MesWnd.2(0) takes the message window away (before scene
                // changes and movies); the next message brings it back.
                // MesWnd.2(1) shows it, which the next message does anyway.
                if class == MESWND && sub == 2 {
                    if let [Value::Int(on)] = args.as_slice() {
                        if *on == 0 {
                            self.scene.text = None;
                            self.scene.speaker = None;
                            self.scene.speaker_name = None;
                        }
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
                }
                // 2017 and 2021 builds: Movie.0 (file) sets up a movie,
                // Movie.3 plays it and returns whether it played, Movie.2
                // closes it and returns nothing (the 2017 scripts also
                // call it after each scene change).
                if class == MOVIE && engine.movie_2_closes {
                    match (sub, args.as_slice()) {
                        (0, [Value::Str(name)]) => self.movie = Some(name.clone()),
                        (2, []) => self.movie = None,
                        (3, []) if self.movie.is_some() => {
                            self.handled += 1;
                            self.pause = Some(Pause::Movie);
                            return Ok(Flow::Yield);
                        }
                        (3, []) => vm.push(0),
                        _ => {}
                    }
                    if matches!((sub, args.len()), (0, 1) | (2, 0) | (3, 0)) {
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
                }
                if class == MOVIE {
                    match (sub, args.as_slice()) {
                        // (0, name): set up a movie.
                        (0, [_, Value::Str(name)]) => {
                            self.movie = Some(name.clone());
                            self.handled += 1;
                            return Ok(Flow::Continue);
                        }
                        // Play it; returns 1 once played (see
                        // [`Runtime::answer_movie`]).
                        (2, []) => {
                            self.handled += 1;
                            if self.movie.is_some() {
                                self.pause = Some(Pause::Movie);
                                return Ok(Flow::Yield);
                            }
                            vm.push(0);
                            return Ok(Flow::Continue);
                        }
                        // Close.
                        (1, []) => {
                            self.movie = None;
                            self.handled += 1;
                            return Ok(Flow::Continue);
                        }
                        _ => {}
                    }
                }
                if matches!(class, MUSIC | EFFECT | VOICE)
                    || (class == HEFFECT && engine.heffect_sounds)
                {
                    if self.sound(vm, class, sub, &args)? {
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
                }
                if base && class == FLAG {
                    if let Some(changed) = crate::flag::call(&mut self.system.flags, vm, sub, &args)
                    {
                        self.system_changed |= changed;
                        self.handled += 1;
                        return Ok(Flow::Continue);
                    }
                }
                let no_op = match title {
                    Title::Fuyukuru => crate::games::fuyukuru::commands::nothing_to_do(class, sub),
                    Title::Apeiria => crate::games::apeiria::commands::nothing_to_do(class, sub),
                    _ => false,
                };
                if no_op {
                    if native::returns_value(title, &self.assets.game().engine, class, sub) {
                        vm.push(0);
                    }
                    self.handled += 1;
                    return Ok(Flow::Continue);
                }
                // (): whether the player clicked, polled once a frame while
                // a voice plays.
                if base && class == MOUSE && sub == 4 && args.is_empty() {
                    vm.push(std::mem::take(&mut self.clicked) as i32);
                    self.pause = Some(Pause::Frame);
                    self.handled += 1;
                    return Ok(Flow::Yield);
                }
                // Select.2(): the engine's own selection, over the options
                // whose bodies left their labels; returns the option
                // picked, from 1.
                if class == native::SELECT && sub == 2 && args.is_empty() {
                    self.choices = std::mem::take(&mut vm.option_labels);
                    self.option_count = 0;
                    self.pause = Some(Pause::Choice);
                    self.handled += 1;
                    return Ok(Flow::Yield);
                }
                let name = native::class_name(class).unwrap_or("?");
                self.skip(format!("{name}.{sub}"), vm, class, sub);
                Ok(Flow::Continue)
            }
        }
    }

    /// Start of a message: the speaker is set again for each line.
    fn message(&mut self, vm: &mut Vm, _number: u32) -> silky_vm::Result<Flow> {
        self.message_script = vm.script_name().unwrap_or("").to_ascii_lowercase();
        self.scene.speaker = None;
        self.scene.speaker_name = None;
        self.voice = None;
        self.lip_track = None;
        // A new message cuts the voice of the last one, unless the player
        // turned that off.
        if self.system.settings.cut_voice {
            self.audio.stop_bus(Bus::Voice);
        }
        Ok(Flow::Continue)
    }
}
