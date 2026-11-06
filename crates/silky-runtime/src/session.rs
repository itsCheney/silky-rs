//! A running game for an interactive frontend: interpreter, runtime,
//! renderer and interface state together, driven by pointer events and a
//! clock. The desktop player and a mobile embedding only forward input and
//! present [`Session::frame`].

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use silky_format::akb::Image;

use silky_format::game::Game;
use silky_format::mes::Profile;
use silky_vm::{Stop, Vm};

use crate::audio::Bus;
use crate::menus::{self, extras_ids, ids as set_ids, ExtrasScreen, Menus, Page, SettingsScreen};
use crate::movie::Movie;
use crate::render::{Canvas, Renderer};
use crate::save::{self, Saves};
use crate::screens::{
    dialog_ids, log_ids, save_ids, Action, Backlog, Dialog, Item, LogEntry, Mode, SaveLoad, Screen,
    Screens, LOG_ROWS, SAVE_ROWS,
};
use crate::system::{RightClick, Settings, SystemData};
use crate::text::TextRenderer;
use crate::ui::{bar, title, Button, Layout, Ui};
use crate::{Assets, Config, Error, Extras, Language, Pause, Result, Runtime, SaveState};

/// What the game waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// A message is shown; a click goes on.
    Text,
    /// The title menu is up.
    Title,
    /// A selection is up.
    Choice,
    /// A transition or a timed wait is running; [`Session::tick`] ends it.
    Busy,
    /// An extras screen of the title menu is up.
    Extras,
    /// A movie plays; a click ends it.
    Movie,
    /// The scripts ended, or the player asked to quit.
    Ended,
}

/// Instructions per [`Vm::run`] call, and calls before giving up on a
/// script that never stops.
const STEP_BUDGET: u64 = 50_000_000;
const MAX_BUDGETS: u32 = 20;
/// Length of one step of a character jolt (a guess: the engine's step
/// timing is not read yet).
const JOLT_STEP_MS: u64 = 50;
/// Time between writes of changed system data.
const SYSTEM_STORE_MS: u64 = 3000;
/// Extra time after punctuation (in characters) with the pause setting.
const PUNCTUATION_PAUSE: u64 = 6;
/// Messages kept for the backlog.
const LOG_MAX: usize = 300;

pub struct Session {
    pub vm: Vm,
    pub rt: Runtime,
    pub ui: Ui,
    renderer: Renderer,
    layout: Layout,
    start: String,
    state: State,
    /// Scene without the interface, rendered again only when it changes.
    base: Option<Canvas>,
    frame: Option<Canvas>,
    /// Time the current message has been shown.
    shown_ms: u64,
    saves: Option<Saves>,
    /// Transition or wait in progress.
    busy: Option<Busy>,
    /// Rule masks by number.
    rules: HashMap<u32, Option<Arc<Vec<u8>>>>,
    screens: Screens,
    /// Save/load screen or backlog, drawn instead of the game.
    screen: Option<Screen>,
    /// Yes/no question over everything.
    dialog: Option<Dialog>,
    /// Shown messages, oldest first.
    log: Vec<LogEntry>,
    /// Marked voices still to play from the backlog.
    voice_queue: VecDeque<String>,
    menus: Menus,
    /// Movie playing.
    movie: Option<MoviePlay>,
    /// Messages shown since the start or the last load (the backlog
    /// keeps only the last [`LOG_MAX`]).
    logged: usize,
    /// Places to go back to with "previous choice": [`Session::logged`]
    /// and the state of the message before each selection.
    choice_points: Vec<(usize, SaveState)>,
    /// Skip and auto modes as they were when the last selection came up.
    modes_at_choice: (bool, bool),
    /// Skip runs to the next selection, through unread text too.
    skip_to_choice: bool,
    /// The player clicked while the message was still appearing.
    reveal_all: bool,
    /// Characters of the message drawn; `None` for all.
    shown_chars: Option<usize>,
    /// Clock time of the last system data write.
    system_stored: u64,
    /// Character jolts playing: slot, offsets, start time.
    jolts: Vec<(i32, crate::host::Jolt, u64)>,
    /// Thumbnail of the game as it was when a screen opened.
    thumb: Option<Image>,
    /// The backlog plays the marked voices one after another.
    playing_marked: bool,
}

struct MoviePlay {
    movie: Movie,
    ms: f64,
}

struct Busy {
    /// Frame shown before the change; `None` for plain waits.
    from: Option<Canvas>,
    rule: Option<Arc<Vec<u8>>>,
    ms: u32,
    elapsed: u32,
    /// Input poll: lasts one frame, clicks do not end it.
    frame: bool,
}

/// Length of one frame for input polls.
const FRAME_MS: u32 = 16;

impl Session {
    /// Loads the game and runs its scripts from `start` (`start.mes`) to
    /// the first stop.
    pub fn new(
        game: Game,
        config: Config,
        text: Option<TextRenderer>,
        start: &str,
    ) -> Result<Self> {
        let assets = Assets::new(game);
        let layout = Layout::load(&assets, config.language)?;
        let screens = Screens::load(&assets, config.language);
        let menus = Menus::load(&assets, config.language);
        let mut renderer = Renderer::new(text, config.language);
        let rt = Runtime::new(assets, config);
        let st = &rt.system.settings;
        renderer.window = (st.window_color, st.window_opacity);
        let mut s = Self {
            vm: Vm::new(Profile::default()),
            rt,
            ui: Ui::default(),
            renderer,
            layout,
            start: start.to_string(),
            state: State::Text,
            base: None,
            frame: None,
            shown_ms: 0,
            saves: None,
            busy: None,
            rules: HashMap::new(),
            screens,
            screen: None,
            dialog: None,
            log: Vec::new(),
            voice_queue: VecDeque::new(),
            thumb: None,
            playing_marked: false,
            menus,
            movie: None,
            logged: 0,
            choice_points: Vec::new(),
            modes_at_choice: (false, false),
            skip_to_choice: false,
            reveal_all: false,
            shown_chars: None,
            system_stored: 0,
            jolts: Vec::new(),
        };
        s.restart()?;
        Ok(s)
    }

    /// Where to keep save files and system data (`system.json`: settings
    /// and what was seen). Without one, nothing is kept.
    pub fn set_saves(&mut self, saves: Saves) {
        self.ui.has_saves = !saves.list().is_empty();
        let mut data = SystemData::load(saves.dir());
        let seen = std::mem::take(&mut self.rt.system);
        data.seen.extend(seen.seen);
        data.music.extend(seen.music);
        data.read.extend(seen.read);
        self.rt.system = data;
        self.saves = Some(saves);
        let old = Settings {
            language: Some(self.rt.config.language),
            ..self.rt.system.settings.clone()
        };
        self.settings_changed(old);
        self.rt.system_changed = false;
    }

    /// Writes changed system data now (call before quitting).
    pub fn flush_system(&mut self) {
        if !self.rt.system_changed {
            return;
        }
        if let Some(saves) = &self.saves {
            if let Err(e) = self.rt.system.store(saves.dir()) {
                eprintln!("system data: {e}");
            }
        }
        self.rt.system_changed = false;
        self.system_stored = self.ui.clock;
    }

    /// Applies changed settings: sound, window, language.
    fn settings_changed(&mut self, old: Settings) {
        let st = self.rt.system.settings.clone();
        self.rt.system_changed = true;
        self.rt.apply_volumes();
        self.renderer.window = (st.window_color, st.window_opacity);
        if let Some(language) = st.language.filter(|l| Some(*l) != old.language) {
            self.change_language(language);
        }
        self.base = None;
        self.frame = None;
    }

    fn change_language(&mut self, language: Language) {
        let Ok(layout) = Layout::load(&self.rt.assets, language) else {
            return;
        };
        self.layout = layout;
        self.rt.set_language(language);
        self.screens = Screens::load(&self.rt.assets, language);
        self.menus = Menus::load(&self.rt.assets, language);
        self.renderer.set_language(language);
    }

    /// Saves the shown message to `slot` (see [`save::QUICK`]).
    pub fn save(&mut self, slot: &str) -> Result<()> {
        let Some(saves) = &self.saves else {
            return Ok(());
        };
        if self.state != State::Text {
            return Ok(());
        }
        let saves = saves.clone();
        let thumb = match &self.thumb {
            Some(t) => Some(t.clone()),
            None => self.scene_thumb().ok(),
        };
        saves.write(slot, self.rt.save_state(&self.vm), thumb.as_ref())?;
        self.ui.has_saves = true;
        Ok(())
    }

    /// The game screen without the interface, as a save thumbnail.
    fn scene_thumb(&mut self) -> Result<Image> {
        let canvas = self.renderer.render_scene(
            &self.rt.scene,
            &mut self.rt.assets,
            !self.ui.window_hidden && self.state != State::Title,
        )?;
        Ok(save::thumbnail(&canvas.rgba, canvas.width, canvas.height))
    }

    /// Writes an auto save of the last message (when a selection comes
    /// up).
    fn auto_save(&mut self) {
        let (Some(saves), Some(entry)) = (self.saves.clone(), self.log.last()) else {
            return;
        };
        let state = entry.state.clone();
        let thumb = self.scene_thumb().ok();
        if saves
            .write(&saves.next_auto_slot(), state, thumb.as_ref())
            .is_ok()
        {
            self.ui.has_saves = true;
        }
    }

    /// Loads a save and shows its message.
    pub fn load(&mut self, slot: &str) -> Result<()> {
        let Some(saves) = &self.saves else {
            return Ok(());
        };
        if !saves.list().iter().any(|s| s == slot) {
            return Ok(());
        }
        let file = saves.read(slot)?;
        self.log.clear();
        self.logged = 0;
        self.choice_points.clear();
        self.restore(file.state)
    }

    /// Puts back a game state and shows its message.
    fn restore(&mut self, state: SaveState) -> Result<()> {
        let kept = state.clone();
        self.rt.load_state(&mut self.vm, state)?;
        if self.log.is_empty() {
            if let Some(text) = &self.rt.scene.text {
                self.logged += 1;
                self.log.push(LogEntry {
                    speaker: self.rt.scene.speaker,
                    text: text.clone(),
                    voice: None,
                    state: kept,
                    marked: false,
                });
            }
        }
        self.busy = None;
        self.screen = None;
        self.dialog = None;
        self.voice_queue.clear();
        self.ui = Ui {
            has_saves: true,
            waiting: true,
            ..Ui::default()
        };
        self.state = State::Text;
        self.shown_ms = 0;
        self.reveal_all = true;
        self.shown_chars = None;
        self.skip_to_choice = false;
        self.movie = None;
        self.jolts.clear();
        self.base = None;
        self.frame = None;
        self.update_previous_choice();
        Ok(())
    }

    /// Loads the newest save, if any.
    pub fn load_latest(&mut self) -> Result<()> {
        let latest = self
            .saves
            .as_ref()
            .and_then(|s| s.list().into_iter().next());
        match latest {
            Some(slot) => self.load(&slot),
            None => Ok(()),
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// Starts the game again from its first script.
    pub fn restart(&mut self) -> Result<()> {
        self.vm = Vm::new(Profile::default());
        self.rt.reset();
        self.rt.boot(&mut self.vm, &self.start)?;
        self.busy = None;
        self.screen = None;
        self.dialog = None;
        self.log.clear();
        self.logged = 0;
        self.choice_points.clear();
        self.voice_queue.clear();
        self.movie = None;
        self.jolts.clear();
        self.skip_to_choice = false;
        self.ui = Ui {
            has_saves: self.ui.has_saves,
            ..Ui::default()
        };
        self.run()
    }

    /// Runs the scripts until they wait for the player.
    fn run(&mut self) -> Result<()> {
        let mut budgets = 0;
        loop {
            let stop = self.vm.run(&mut self.rt, STEP_BUDGET).map_err(vm_error)?;
            match stop {
                Stop::Yield => match self.rt.pause.take() {
                    Some(Pause::Text { .. }) => {
                        if let Some(text) = &self.rt.scene.text {
                            if self.log.len() >= LOG_MAX {
                                self.log.remove(0);
                            }
                            self.logged += 1;
                            self.log.push(LogEntry {
                                speaker: self.rt.scene.speaker,
                                text: text.clone(),
                                voice: self.rt.voice.clone(),
                                state: self.rt.save_state(&self.vm),
                                marked: false,
                            });
                        }
                        self.state = State::Text;
                        self.ui.waiting = true;
                        self.shown_ms = 0;
                        self.reveal_all = false;
                        self.shown_chars = self.revealed();
                        break;
                    }
                    Some(Pause::Title) => {
                        self.skip_to_choice = false;
                        self.state = State::Title;
                        self.ui.title = Some(false);
                        self.ui.auto = false;
                        self.ui.skip = false;
                        break;
                    }
                    Some(Pause::Transition(t)) => {
                        let from = self.base.take();
                        let rule = t.rule.and_then(|n| self.rule(n));
                        self.busy = Some(Busy {
                            from,
                            rule,
                            ms: t.ms,
                            elapsed: 0,
                            frame: false,
                        });
                        self.state = State::Busy;
                        break;
                    }
                    Some(Pause::Frame) => {
                        self.busy = Some(Busy {
                            from: None,
                            rule: None,
                            ms: FRAME_MS,
                            elapsed: 0,
                            frame: true,
                        });
                        self.state = State::Busy;
                        break;
                    }
                    Some(Pause::Choice) => {
                        if self.rt.system.settings.auto_save_at_choices {
                            self.auto_save();
                        }
                        let len = self.logged;
                        self.choice_points.retain(|p| p.0 < len);
                        if let Some(e) = self.log.last() {
                            self.choice_points.push((len, e.state.clone()));
                        }
                        self.ui.choices = self.rt.choices.clone();
                        self.ui.waiting = false;
                        self.state = State::Choice;
                        self.modes_at_choice = (self.ui.skip && !self.skip_to_choice, self.ui.auto);
                        self.skip_to_choice = false;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        break;
                    }
                    Some(Pause::Extras(kind)) => {
                        self.ui.title = None;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        self.ui.hover = None;
                        self.screen = Some(Screen::Extras(ExtrasScreen::new(kind)));
                        self.state = State::Extras;
                        break;
                    }
                    Some(Pause::Movie) => match self.rt.open_movie() {
                        Ok(movie) => {
                            self.movie = Some(MoviePlay { movie, ms: 0.0 });
                            self.state = State::Movie;
                            break;
                        }
                        Err(e) => {
                            let name = self.rt.movie.clone().unwrap_or_default();
                            self.rt.note_unhandled(format!("movie {name}: {e}"));
                            self.rt.answer_movie(&mut self.vm);
                        }
                    },
                    None => {}
                },
                Stop::Options => {
                    self.rt.begin_options(self.vm.options().len());
                    self.vm.run_all_options().map_err(vm_error)?;
                }
                Stop::Ended => {
                    self.state = State::Ended;
                    break;
                }
                Stop::Budget => {
                    budgets += 1;
                    if budgets >= MAX_BUDGETS {
                        return Err(Error::Invalid(format!(
                            "scripts ran {} instructions without waiting for the player",
                            STEP_BUDGET * MAX_BUDGETS as u64
                        )));
                    }
                }
            }
        }
        let now = self.ui.clock;
        self.jolts.extend(
            self.rt
                .jolts
                .drain(..)
                .map(|(slot, steps)| (slot, steps, now)),
        );
        self.base = None;
        self.frame = None;
        self.update_previous_choice();
        Ok(())
    }

    /// Moves jolting characters to their offset for the clock time.
    fn play_jolts(&mut self) {
        if self.jolts.is_empty() && self.rt.scene.jolts.is_empty() {
            return;
        }
        let clock = self.ui.clock;
        let mut now = std::collections::BTreeMap::new();
        self.jolts.retain(|(slot, steps, start)| {
            let step = ((clock - start) / JOLT_STEP_MS) as usize;
            if let Some(&o) = steps.get(step) {
                now.insert(*slot, o);
                true
            } else {
                false
            }
        });
        now.retain(|_, o| *o != (0, 0));
        if now != self.rt.scene.jolts {
            self.rt.scene.jolts = now;
            self.base = None;
            self.frame = None;
        }
    }

    fn update_previous_choice(&mut self) {
        let len = self.logged;
        self.ui.previous_choice = self.choice_points.iter().any(|p| p.0 < len)
            && matches!(self.state, State::Text | State::Choice);
    }

    /// Goes back to the message before the last selection (before the one
    /// that is up, at a selection).
    fn previous_choice(&mut self) -> Result<()> {
        let len = self.logged;
        self.choice_points.retain(|p| p.0 < len);
        let Some((n, state)) = self.choice_points.last().cloned() else {
            return Ok(());
        };
        self.truncate_log(self.log.len() - (len - n).min(self.log.len()));
        self.restore(state)
    }

    /// Keeps the first `keep` backlog entries.
    fn truncate_log(&mut self, keep: usize) {
        self.logged -= self.log.len().saturating_sub(keep);
        self.log.truncate(keep);
    }

    /// Characters of the message shown so far, `None` once all are.
    fn revealed(&self) -> Option<usize> {
        let text = self.rt.scene.text.as_deref()?;
        let st = &self.rt.system.settings;
        if !st.typewriter || self.reveal_all || self.ui.skip {
            return None;
        }
        let per = st.char_ms();
        let mut t = 0;
        for (i, c) in text.chars().filter(|&c| c != '\n').enumerate() {
            t += per;
            if st.punctuation_pause && is_pause_punctuation(c) {
                t += per * PUNCTUATION_PAUSE;
            }
            if t > self.shown_ms {
                return Some(i);
            }
        }
        None
    }

    /// Time the whole message takes to appear.
    fn reveal_ms(&self) -> u64 {
        let st = &self.rt.system.settings;
        let Some(text) = self.rt.scene.text.as_deref() else {
            return 0;
        };
        if !st.typewriter || self.reveal_all {
            return 0;
        }
        text.chars()
            .filter(|&c| c != '\n')
            .map(|c| {
                let pause = st.punctuation_pause && is_pause_punctuation(c);
                st.char_ms() * if pause { 1 + PUNCTUATION_PAUSE } else { 1 }
            })
            .sum()
    }

    /// Rule mask `NN.bin`, cached; `None` if the game has none.
    fn rule(&mut self, n: u32) -> Option<Arc<Vec<u8>>> {
        let assets = &self.rt.assets;
        self.rules
            .entry(n)
            .or_insert_with(|| assets.data(&format!("{n:02}.bin")).ok().map(Arc::new))
            .clone()
    }

    /// Ends the running transition or wait and runs the scripts on.
    fn finish_busy(&mut self) -> Result<()> {
        if self.busy.take().is_some() && self.state == State::Busy {
            self.run()?;
        }
        Ok(())
    }

    /// Goes on from the current message.
    fn advance(&mut self) -> Result<()> {
        if self.state == State::Text && self.shown_chars.is_some() {
            self.reveal_all = true;
            self.shown_chars = None;
            self.frame = None;
            return Ok(());
        }
        if self.state == State::Text {
            self.ui.waiting = false;
            self.run()?;
        }
        Ok(())
    }

    /// Pointer moved to (x, y) in screen pixels.
    pub fn pointer_move(&mut self, x: i32, y: i32) {
        if let Some(Screen::Settings(SettingsScreen {
            dragging: Some(id), ..
        })) = &self.screen
        {
            let id = *id;
            self.drag_settings(id, x, y);
            return;
        }
        let hover = self.hit(x, y);
        if hover != self.ui.hover {
            self.ui.hover = hover;
            self.frame = None;
        }
    }

    /// Primary button pressed at (x, y).
    pub fn pointer_down(&mut self, x: i32, y: i32) {
        self.ui.pressed = self.hit(x, y);
        self.frame = None;
        if let (Some(Screen::Settings(s)), Some(Button::Screen(id))) =
            (&mut self.screen, self.ui.pressed)
        {
            if menus::is_slider(s.page, id) || id == set_ids::SCROLL {
                s.dragging = Some(id);
                self.drag_settings(id, x, y);
            }
        }
    }

    /// A slider or the scroll bar of the settings screen follows the
    /// pointer.
    fn drag_settings(&mut self, id: i32, x: i32, y: i32) {
        let Some(Screen::Settings(s)) = &mut self.screen else {
            return;
        };
        self.frame = None;
        if id == set_ids::SCROLL {
            let max = self.menus.max_scroll(&mut self.rt.assets, s.page);
            let (_, ty, _, th) = menus::SETTINGS_TRACK;
            let f = ((y - ty) as f32 / th as f32).clamp(0.0, 1.0);
            s.scroll = (f * max as f32).round() as i32;
            return;
        }
        let Some(a) = self.menus.settings_area(s, id) else {
            return;
        };
        let page = s.page;
        let old = self.rt.system.settings.clone();
        if menus::apply(
            &mut self.rt.system.settings,
            page,
            id,
            menus::slider_value(&a, x),
        ) {
            self.settings_changed(old);
        }
    }

    /// Primary button released at (x, y): a click.
    pub fn pointer_up(&mut self, x: i32, y: i32) -> Result<()> {
        let pressed = self.ui.pressed.take();
        self.frame = None;
        if let Some(Screen::Settings(s)) = &mut self.screen {
            if s.dragging.take().is_some() {
                return Ok(());
            }
        }
        if self.state == State::Movie {
            return self.end_movie();
        }
        if let Some(Screen::Extras(e)) = &mut self.screen {
            if let Some((entry, pos)) = e.view {
                e.view = ExtrasScreen::next_picture(&self.rt.system, entry, Some(pos))
                    .map(|p| (entry, p));
                return Ok(());
            }
        }
        let hit = self.hit(x, y);
        if hit.is_some() && hit != pressed {
            return Ok(());
        }
        if self.dialog.is_some() {
            if let Some(Button::Dialog(id)) = hit {
                self.press_dialog(id)?;
            }
            return Ok(());
        }
        if self.screen.is_some() {
            if let Some(Button::Screen(id)) = hit {
                self.press_screen(id, x, y)?;
            }
            return Ok(());
        }
        if self.state == State::Busy {
            self.rt.clicked = true;
            if let Some(b) = &mut self.busy {
                if !b.frame {
                    b.elapsed = b.ms;
                }
            }
            return Ok(());
        }
        if self.ui.window_hidden {
            self.ui.window_hidden = false;
            self.base = None;
            return Ok(());
        }
        match hit {
            Some(b) if !self.ui.enabled(b) => Ok(()),
            Some(b) => self.press(b),
            None if self.state == State::Text => {
                if self.ui.auto || self.ui.skip {
                    self.ui.auto = false;
                    self.ui.skip = false;
                    self.skip_to_choice = false;
                    Ok(())
                } else {
                    self.advance()
                }
            }
            None => Ok(()),
        }
    }

    /// Secondary click: closes a dialog or screen, else does what the
    /// settings say (hide the message window by default).
    pub fn secondary_click(&mut self) -> Result<()> {
        if self.back()? || self.state != State::Text {
            return Ok(());
        }
        self.frame = None;
        if self.ui.window_hidden {
            self.ui.window_hidden = false;
            self.base = None;
            return Ok(());
        }
        match self.rt.system.settings.right_click {
            RightClick::HideWindow => {
                self.ui.window_hidden = true;
                self.base = None;
            }
            RightClick::Settings => self.open_settings(),
            RightClick::Skip => self.press(Button::Bar(bar::SKIP))?,
            RightClick::Auto => self.press(Button::Bar(bar::AUTO))?,
            RightClick::Load => self.open_save_load(Mode::Load),
            RightClick::Save => self.open_save_load(Mode::Save),
        }
        Ok(())
    }

    /// Closes the dialog or screen that is up, or ends a movie (Escape,
    /// right click). Returns false if there was none.
    pub fn back(&mut self) -> Result<bool> {
        if self.dialog.take().is_some() {
            self.frame = None;
            return Ok(true);
        }
        if self.state == State::Movie {
            self.end_movie()?;
            return Ok(true);
        }
        match &mut self.screen {
            Some(Screen::Extras(e)) if e.view.is_some() => {
                e.view = None;
                self.frame = None;
            }
            Some(Screen::Extras(_)) => self.leave_extras(-1)?,
            Some(_) => self.close_screen(),
            None => return Ok(false),
        }
        Ok(true)
    }

    fn end_movie(&mut self) -> Result<()> {
        if self.movie.take().is_some() {
            self.rt.answer_movie(&mut self.vm);
            self.state = State::Busy;
            self.run()?;
        }
        Ok(())
    }

    /// Leaves an extras screen with `result` for the script: -1 back to
    /// the title, or a scene to replay.
    fn leave_extras(&mut self, result: i32) -> Result<()> {
        self.rt.stop_music();
        self.screen = None;
        self.ui.hover = None;
        self.frame = None;
        if result >= 0 {
            // The scene mode script plays a scene when this is set.
            self.vm.set_g(19, 1);
        }
        self.rt.answer_extras(&mut self.vm, result);
        self.state = State::Busy;
        self.run()
    }

    fn open_settings(&mut self) {
        self.screen = Some(Screen::Settings(SettingsScreen::new()));
        self.ui.skip = false;
        self.ui.hover = None;
        self.frame = None;
    }

    /// Mouse wheel: `up` towards older text. Up on a message opens the
    /// backlog; down at its newest entry closes it.
    pub fn wheel(&mut self, up: bool) {
        if self.dialog.is_some() {
            return;
        }
        let d = if up { -1 } else { 1 };
        match &mut self.screen {
            Some(Screen::SaveLoad(s)) => s.scroll_by(d),
            Some(Screen::Settings(s)) => {
                let max = self.menus.max_scroll(&mut self.rt.assets, s.page);
                s.scroll = (s.scroll + d as i32 * menus::SCROLL_STEP).clamp(0, max);
            }
            Some(Screen::Extras(e)) => match e.kind {
                Extras::Music => {
                    e.scroll = (e.scroll as isize + d).clamp(0, Menus::music_max_scroll() as isize)
                        as usize;
                }
                _ if e.view.is_none() => {
                    e.page = (e.page as isize + d).clamp(0, menus::CG_PAGES as isize - 1) as usize;
                }
                _ => return,
            },
            Some(Screen::Backlog(b)) => {
                if !up && b.top >= Backlog::max_top(self.log.len()) {
                    self.close_screen();
                } else {
                    b.scroll_by(d, self.log.len());
                }
            }
            None if up && self.state == State::Text => self.open_backlog(),
            None => return,
        }
        self.frame = None;
    }

    fn open_save_load(&mut self, mode: Mode) {
        let can_save = self.state == State::Text && self.saves.is_some();
        if mode == Mode::Save && !can_save {
            return;
        }
        self.thumb = if can_save {
            self.scene_thumb().ok()
        } else {
            None
        };
        let mut s = SaveLoad {
            mode,
            tab: save::Kind::Manual,
            scroll: 0,
            rows: Vec::new(),
            can_save,
        };
        if let Some(saves) = &self.saves {
            s.refresh(saves);
        }
        self.screen = Some(Screen::SaveLoad(s));
        self.ui.auto = false;
        self.ui.skip = false;
        self.ui.hover = None;
        self.frame = None;
    }

    fn open_backlog(&mut self) {
        if self.log.is_empty() || self.state != State::Text {
            return;
        }
        for e in &mut self.log {
            e.marked = false;
        }
        self.screen = Some(Screen::Backlog(Backlog {
            top: Backlog::max_top(self.log.len()),
        }));
        self.ui.auto = false;
        self.ui.skip = false;
        self.ui.hover = None;
        self.frame = None;
    }

    fn close_screen(&mut self) {
        if matches!(self.screen, Some(Screen::Settings(_))) {
            self.flush_system();
        }
        self.screen = None;
        self.thumb = None;
        self.voice_queue.clear();
        self.playing_marked = false;
        self.ui.hover = None;
        self.frame = None;
    }

    fn refresh_screen(&mut self) {
        if let (Some(Screen::SaveLoad(s)), Some(saves)) = (&mut self.screen, &self.saves) {
            s.refresh(saves);
        }
        self.ui.has_saves = self.saves.as_ref().is_some_and(|s| !s.list().is_empty());
    }

    /// Asks before doing something, unless the player said not to ask.
    fn ask(&mut self, action: Action) -> Result<()> {
        if !self.rt.system.settings.confirm[action.question()] {
            return self.carry_out(action);
        }
        self.dialog = Some(Dialog {
            action,
            dont_ask: false,
        });
        self.ui.hover = None;
        self.frame = None;
        Ok(())
    }

    fn press_dialog(&mut self, id: i32) -> Result<()> {
        self.frame = None;
        match id {
            dialog_ids::YES => {
                if let Some(d) = self.dialog.take() {
                    if d.dont_ask {
                        self.rt.system.settings.confirm[d.action.question()] = false;
                        self.rt.system_changed = true;
                    }
                    self.carry_out(d.action)?;
                }
            }
            dialog_ids::NO => self.dialog = None,
            dialog_ids::DONT_ASK => {
                if let Some(d) = &mut self.dialog {
                    d.dont_ask = !d.dont_ask;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn carry_out(&mut self, action: Action) -> Result<()> {
        self.frame = None;
        match action {
            Action::Save(slot) => {
                let slot = match (slot, &self.saves) {
                    (Some(slot), _) => slot,
                    (None, Some(saves)) => saves.next_slot(),
                    (None, None) => return Ok(()),
                };
                self.save(&slot)?;
                if let Some(Screen::SaveLoad(s)) = &mut self.screen {
                    s.scroll = 0;
                }
                self.refresh_screen();
            }
            Action::Load(slot) => {
                self.thumb = None;
                self.load(&slot)?;
            }
            Action::Delete(slot) => {
                if let Some(saves) = &self.saves {
                    saves.delete(&slot)?;
                }
                self.refresh_screen();
            }
            Action::DeleteAll => {
                if let Some(saves) = &self.saves {
                    saves.delete_all()?;
                }
                self.refresh_screen();
            }
            Action::Jump(i) => {
                if let Some(e) = self.log.get(i) {
                    let state = e.state.clone();
                    self.truncate_log(i + 1);
                    self.restore(state)?;
                }
            }
            Action::Title => self.restart()?,
            Action::Quit => {
                self.flush_system();
                self.state = State::Ended;
            }
            Action::Scene(i) => self.leave_extras(i as i32)?,
        }
        Ok(())
    }

    fn press_screen(&mut self, id: i32, x: i32, y: i32) -> Result<()> {
        self.frame = None;
        match &self.screen {
            Some(Screen::Settings(_)) => return self.press_settings(id, x, y),
            Some(Screen::Extras(_)) => return self.press_extras(id, y),
            _ => {}
        }
        let mut ask = None;
        let mut close = false;
        match &mut self.screen {
            Some(Screen::SaveLoad(s)) => {
                use save_ids::*;
                if !self.screens.save_load_enabled(s, id, self.ui.has_saves) {
                    return Ok(());
                }
                let mut refresh = false;
                match id {
                    SAVE_MODE | LOAD_MODE => {
                        s.mode = if id == SAVE_MODE {
                            Mode::Save
                        } else {
                            Mode::Load
                        };
                        if s.mode == Mode::Save {
                            s.tab = save::Kind::Manual;
                        }
                        s.scroll = 0;
                        refresh = true;
                    }
                    TAB_MANUAL | TAB_AUTO | TAB_QUICK => {
                        s.tab = match id {
                            TAB_MANUAL => save::Kind::Manual,
                            TAB_AUTO => save::Kind::Auto,
                            _ => save::Kind::Quick,
                        };
                        s.scroll = 0;
                        refresh = true;
                    }
                    BACK => close = true,
                    UP => s.scroll_by(-1),
                    DOWN => s.scroll_by(1),
                    SCROLL => {
                        if let Some(f) = self.screens.scroll_fraction(false, x, y) {
                            s.scroll = (f * s.max_scroll() as f32).round() as usize;
                        }
                    }
                    DELETE_ALL => ask = Some(Action::DeleteAll),
                    _ if (DELETE..DELETE + SAVE_ROWS as i32).contains(&id) => {
                        if let Some(Item::Slot(r)) = s.item((id - DELETE) as usize) {
                            ask = Some(Action::Delete(r.slot.clone()));
                        }
                    }
                    _ if (ROW..ROW + SAVE_ROWS as i32).contains(&id) => {
                        ask = match s.item((id - ROW) as usize) {
                            Some(Item::Create) => Some(Action::Save(None)),
                            Some(Item::Slot(r)) if s.mode == Mode::Save => {
                                Some(Action::Save(Some(r.slot.clone())))
                            }
                            Some(Item::Slot(r)) => Some(Action::Load(r.slot.clone())),
                            None => None,
                        };
                    }
                    _ => {}
                }
                if refresh {
                    if let Some(saves) = &self.saves {
                        s.refresh(saves);
                    }
                }
            }
            Some(Screen::Backlog(b)) => {
                use log_ids::*;
                let len = self.log.len();
                if !Screens::backlog_enabled(b, &self.log, id) {
                    return Ok(());
                }
                let rows = 0..LOG_ROWS as i32;
                match id {
                    _ if rows.contains(&(id - VOICE)) => {
                        let file = self
                            .log
                            .get(b.top + (id - VOICE) as usize)
                            .and_then(|e| e.voice.clone());
                        if let Some(file) = file {
                            self.voice_queue.clear();
                            self.playing_marked = false;
                            self.rt.replay_voice(&file)?;
                        }
                    }
                    _ if rows.contains(&(id - CHECK)) => {
                        if let Some(e) = self.log.get_mut(b.top + (id - CHECK) as usize) {
                            e.marked = !e.marked;
                        }
                    }
                    _ if rows.contains(&(id - JUMP)) => {
                        ask = Some(Action::Jump(b.top + (id - JUMP) as usize));
                    }
                    UP => b.scroll_by(-1, len),
                    DOWN => b.scroll_by(1, len),
                    PAGE_UP => b.scroll_by(-(LOG_ROWS as isize), len),
                    PAGE_DOWN => b.scroll_by(LOG_ROWS as isize, len),
                    SCROLL => {
                        if let Some(f) = self.screens.scroll_fraction(true, x, y) {
                            b.top = (f * Backlog::max_top(len) as f32).round() as usize;
                        }
                    }
                    PLAY_MARKED => {
                        if self.playing_marked {
                            self.playing_marked = false;
                            self.voice_queue.clear();
                            self.rt.stop_sound_voice();
                        } else {
                            self.voice_queue = self
                                .log
                                .iter()
                                .filter(|e| e.marked)
                                .filter_map(|e| e.voice.clone())
                                .collect();
                            self.playing_marked = true;
                        }
                    }
                    BACK => close = true,
                    _ => {}
                }
            }
            Some(Screen::Settings(_) | Screen::Extras(_)) | None => {}
        }
        if close {
            self.close_screen();
        }
        if let Some(action) = ask {
            self.ask(action)?;
        }
        Ok(())
    }

    fn press_settings(&mut self, id: i32, x: i32, y: i32) -> Result<()> {
        let Some(Screen::Settings(s)) = &mut self.screen else {
            return Ok(());
        };
        let max = self.menus.max_scroll(&mut self.rt.assets, s.page);
        match id {
            set_ids::UP => s.scroll = (s.scroll - menus::SCROLL_STEP).max(0),
            set_ids::DOWN => s.scroll = (s.scroll + menus::SCROLL_STEP).min(max),
            set_ids::SCROLL => {
                if let Some(f) = self.menus.settings_track(x, y) {
                    s.scroll = (f * max as f32).round() as i32;
                }
            }
            set_ids::SYSTEM_TAB | set_ids::SOUND_TAB => {
                s.page = if id == set_ids::SYSTEM_TAB {
                    Page::System
                } else {
                    Page::Sound
                };
                s.scroll = 0;
            }
            set_ids::RESET => {
                let page = s.page;
                let old = self.rt.system.settings.clone();
                menus::reset(&mut self.rt.system.settings, page);
                self.settings_changed(old);
            }
            set_ids::BACK => self.close_screen(),
            menus::AUTO_MODE if s.page == Page::System => {
                self.ui.auto = !self.ui.auto;
                self.ui.skip = false;
                self.shown_ms = 0;
            }
            _ => {
                let page = s.page;
                let value = self
                    .menus
                    .settings_area(s, id)
                    .map_or(0.0, |a| menus::slider_value(&a, x));
                let old = self.rt.system.settings.clone();
                if menus::apply(&mut self.rt.system.settings, page, id, value) {
                    self.settings_changed(old);
                }
            }
        }
        Ok(())
    }

    fn press_extras(&mut self, id: i32, y: i32) -> Result<()> {
        use crate::game_tables::MUSIC;
        use extras_ids::*;
        let Some(Screen::Extras(s)) = &mut self.screen else {
            return Ok(());
        };
        match id {
            _ if (MODE..MODE + 3).contains(&id) => {
                let kind = [Extras::Gallery, Extras::Scenes, Extras::Music][(id - MODE) as usize];
                if kind != s.kind {
                    self.rt.stop_music();
                    *s = ExtrasScreen::new(kind);
                }
            }
            BACK => return self.leave_extras(-1),
            PREVIOUS if s.kind == Extras::Music => s.scroll = s.scroll.saturating_sub(1),
            NEXT if s.kind == Extras::Music => {
                s.scroll = (s.scroll + 1).min(Menus::music_max_scroll());
            }
            PREVIOUS => s.page = s.page.saturating_sub(1),
            NEXT => s.page = (s.page + 1).min(menus::CG_PAGES - 1),
            SCROLL if s.kind == Extras::Music => {
                if let Some(a) = crate::screens::area(self.menus.areas(Extras::Music), SCROLL) {
                    let f = ((y - a.top) as f32 / a.height().max(1) as f32).clamp(0.0, 1.0);
                    s.scroll = (f * Menus::music_max_scroll() as f32).round() as usize;
                }
            }
            _ if (PAGE..PAGE + menus::CG_PAGES as i32).contains(&id) => {
                s.page = (id - PAGE) as usize;
            }
            _ if (ITEM..PREVIOUS).contains(&id) => {
                let Some(item) = Menus::item(s, (id - ITEM) as usize) else {
                    return Ok(());
                };
                match s.kind {
                    Extras::Gallery => {
                        s.view = ExtrasScreen::next_picture(&self.rt.system, item, None)
                            .map(|p| (item, p));
                    }
                    Extras::Scenes => return self.ask(Action::Scene(item)),
                    Extras::Music => {
                        if s.playing == Some(item) {
                            s.playing = None;
                            self.rt.stop_music();
                        } else {
                            s.playing = Some(item);
                            self.rt.play_music(MUSIC[item])?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Key that goes on (Enter, Space).
    pub fn proceed(&mut self) -> Result<()> {
        if self.ui.window_hidden {
            self.ui.window_hidden = false;
            self.base = None;
            self.frame = None;
            return Ok(());
        }
        self.advance()
    }

    fn press(&mut self, b: Button) -> Result<()> {
        match b {
            Button::Title(i) => match i {
                title::EXTRAS => self.ui.title = Some(true),
                title::SETTINGS => self.open_settings(),
                title::CONTINUE => self.load_latest()?,
                title::LOAD => self.open_save_load(Mode::Load),
                title::BACK => self.ui.title = Some(false),
                i => {
                    if let Some(result) = title::result(i) {
                        self.ui.title = None;
                        self.rt.answer_title(&mut self.vm, result);
                        self.run()?;
                    } else {
                        self.rt.note_unhandled(format!("title menu entry {i}"));
                    }
                }
            },
            Button::Choice(i) => {
                self.ui.choices.clear();
                let (skip, auto) = self.modes_at_choice;
                let st = &self.rt.system.settings;
                self.ui.skip = skip && st.keep_skip_after_choices;
                self.ui.auto = auto && st.keep_auto_after_choices;
                self.rt.answer_choice(&mut self.vm, i);
                self.run()?;
            }
            Button::Bar(id) => match id {
                bar::AUTO => {
                    self.ui.auto = !self.ui.auto;
                    self.ui.skip = false;
                    self.shown_ms = 0;
                }
                bar::SKIP => {
                    self.ui.skip = !self.ui.skip;
                    self.skip_to_choice = false;
                    self.ui.auto = false;
                }
                bar::NEXT_CHOICE => {
                    self.ui.skip = true;
                    self.skip_to_choice = true;
                    self.ui.auto = false;
                }
                bar::PREVIOUS_CHOICE => self.previous_choice()?,
                bar::CONFIG => self.open_settings(),
                bar::HIDE | bar::CLOSE => {
                    self.ui.window_hidden = true;
                    self.base = None;
                }
                bar::TITLE => self.ask(Action::Title)?,
                bar::QUICK_SAVE => self.save(save::QUICK)?,
                bar::QUICK_LOAD => self.load(save::QUICK)?,
                bar::SAVE => self.open_save_load(Mode::Save),
                bar::LOAD => self.open_save_load(Mode::Load),
                bar::BACKLOG => self.open_backlog(),
                bar::QUIT => self.ask(Action::Quit)?,
                id => self
                    .rt
                    .note_unhandled(format!("message window button {id}")),
            },
            Button::Screen(_) | Button::Dialog(_) => {}
        }
        self.frame = None;
        Ok(())
    }

    fn hit(&self, x: i32, y: i32) -> Option<Button> {
        if self.dialog.is_some() {
            return self.screens.hit_dialog(x, y);
        }
        match &self.screen {
            Some(Screen::SaveLoad(s)) => return self.screens.hit_save_load(s, x, y),
            Some(Screen::Backlog(b)) => return self.screens.hit_backlog(b, &self.log, x, y),
            Some(Screen::Settings(s)) => return self.menus.hit_settings(s, x, y).map(|h| h.0),
            Some(Screen::Extras(e)) => return self.menus.hit_extras(e, &self.rt.system, x, y),
            None => {}
        }
        if matches!(self.state, State::Busy | State::Movie | State::Extras) {
            return None;
        }
        let text_shown =
            matches!(self.state, State::Text | State::Choice) && self.rt.scene.text.is_some();
        self.layout.hit(&self.ui, text_shown, x, y)
    }

    /// Advances the clock by `ms`: animation, auto and skip modes. Returns
    /// whether the frame changed.
    pub fn tick(&mut self, ms: u64) -> Result<bool> {
        let before = self.ui.clock;
        self.ui.clock += ms;
        if self.playing_marked && !self.rt.voice_playing() {
            match self.voice_queue.pop_front() {
                Some(file) => self.rt.replay_voice(&file)?,
                None => {
                    self.playing_marked = false;
                    self.frame = None;
                }
            }
        }
        if self.rt.system_changed && self.ui.clock >= self.system_stored + SYSTEM_STORE_MS {
            self.flush_system();
        }
        if let Some(p) = &mut self.movie {
            match self.rt.movie_clock() {
                Some(Some(t)) => p.ms = t,
                Some(None) => {}
                None => p.ms += ms as f64,
            }
            let changed = p.movie.seek_forward(p.ms);
            if p.movie.ended {
                self.end_movie()?;
                return Ok(true);
            }
            if changed {
                self.frame = None;
            }
            return Ok(self.frame.is_none());
        }
        if let Some(Screen::Extras(e)) = &mut self.screen {
            if e.playing.is_some() && before / 250 != self.ui.clock / 250 {
                if !self.rt.audio.is_playing(Bus::Music, 0) {
                    e.playing = None;
                }
                self.frame = None;
            }
        }
        if self.screen.is_some() || self.dialog.is_some() {
            return Ok(self.frame.is_none());
        }
        self.play_jolts();
        self.shown_ms += ms;
        if self.state == State::Busy {
            // Several short steps (input polls) may fit in one tick.
            let mut left = ms as u32;
            let mut steps = 0;
            while self.state == State::Busy && steps < 64 {
                let Some(b) = &mut self.busy else { break };
                if self.ui.skip {
                    b.elapsed = b.ms;
                }
                let step = left.min(b.ms - b.elapsed.min(b.ms));
                b.elapsed += step;
                left -= step;
                if b.elapsed < b.ms {
                    break;
                }
                self.finish_busy()?;
                steps += 1;
            }
            self.frame = None;
            return Ok(true);
        }
        if self.state == State::Text {
            let shown = self.revealed();
            if shown != self.shown_chars {
                self.shown_chars = shown;
                self.frame = None;
            }
        }
        let st = &self.rt.system.settings;
        if self.state == State::Text
            && self.ui.skip
            && !self.skip_to_choice
            && st.skip_read_only
            && !self.rt.read_before
        {
            // Unread text: skipping stops here.
            self.ui.skip = false;
            self.frame = None;
        }
        if self.state == State::Text && !self.ui.window_hidden {
            let st = &self.rt.system.settings;
            let chars = self
                .rt
                .scene
                .text
                .as_deref()
                .map_or(0, |t| t.chars().count()) as u64;
            let due = if self.ui.skip {
                Some(st.skip_ms())
            } else if self.ui.auto {
                Some(self.reveal_ms() + st.auto_ms(chars))
            } else {
                None
            };
            if due.is_some_and(|d| self.shown_ms >= d) {
                self.advance()?;
                return Ok(true);
            }
        }
        let animating = self.ui.waiting && self.state == State::Text && !self.ui.window_hidden;
        if animating && before / 100 != self.ui.clock / 100 {
            self.frame = None;
            return Ok(true);
        }
        Ok(self.frame.is_none())
    }

    /// The current frame, 1280 x 720 RGBA.
    pub fn frame(&mut self) -> Result<&Canvas> {
        if self.frame.is_none() {
            let mut frame = match &self.screen {
                Some(screen) => {
                    let mut c = Canvas::new(crate::SCREEN_WIDTH, crate::SCREEN_HEIGHT);
                    let text = self.renderer.text();
                    let assets = &mut self.rt.assets;
                    match screen {
                        Screen::SaveLoad(s) => self.screens.draw_save_load(
                            &mut c,
                            assets,
                            &self.ui,
                            s,
                            self.ui.has_saves,
                            text,
                        )?,
                        Screen::Backlog(b) => self.screens.draw_backlog(
                            &mut c,
                            assets,
                            &self.ui,
                            b,
                            &self.log,
                            self.playing_marked,
                            text,
                        )?,
                        Screen::Settings(s) => {
                            let max = self.menus.max_scroll(assets, s.page);
                            self.menus.draw_settings(
                                &mut c,
                                assets,
                                &self.ui,
                                s,
                                &self.rt.system,
                                max,
                                text,
                            )?
                        }
                        Screen::Extras(e) => {
                            let pos = self.rt.audio.position(Bus::Music, 0);
                            self.menus.draw_extras(
                                &mut c,
                                assets,
                                &self.ui,
                                e,
                                &self.rt.system,
                                pos,
                                text,
                            )?
                        }
                    }
                    c
                }
                None => match &self.movie {
                    Some(p) => movie_frame(&p.movie),
                    None => self.game_frame()?,
                },
            };
            if let Some(d) = &self.dialog {
                self.screens
                    .draw_dialog(&mut frame, &mut self.rt.assets, &self.ui, d)?;
            }
            self.frame = Some(frame);
        }
        Ok(self.frame.as_ref().unwrap())
    }

    /// The game with its interface.
    fn game_frame(&mut self) -> Result<Canvas> {
        {
            let window = !self.ui.window_hidden && self.state != State::Title;
            if self.base.is_none() {
                self.base = Some(self.renderer.render_parts(
                    &self.rt.scene,
                    &mut self.rt.assets,
                    window,
                    false,
                )?);
            }
            let mut frame = match &self.busy {
                Some(Busy {
                    from: Some(from),
                    rule,
                    ms,
                    elapsed,
                    ..
                }) => Canvas::blend(
                    from,
                    self.base.as_ref().unwrap(),
                    *elapsed as f32 / (*ms).max(1) as f32,
                    rule.as_deref().map(Vec::as_slice),
                ),
                _ => self.base.clone().unwrap(),
            };
            if window {
                self.renderer
                    .draw_text(&mut frame, &self.rt.scene, self.shown_chars);
            }
            let text_shown =
                matches!(self.state, State::Text | State::Choice) && self.rt.scene.text.is_some();
            self.layout.draw(
                &mut frame,
                &mut self.rt.assets,
                &self.ui,
                text_shown,
                self.renderer.text(),
            )?;
            Ok(frame)
        }
    }
}

/// The movie picture scaled to fit the screen, on black.
fn movie_frame(m: &Movie) -> Canvas {
    let mut c = Canvas::new(crate::SCREEN_WIDTH, crate::SCREEN_HEIGHT);
    c.fill(0, 0, c.width, c.height, [0, 0, 0, 255]);
    let img = Image {
        width: m.width,
        height: m.height,
        rgba: m.rgba.clone(),
    };
    let scale = (c.width as f32 / m.width as f32).min(c.height as f32 / m.height as f32);
    let x = (c.width as f32 - m.width as f32 * scale) / 2.0;
    let y = (c.height as f32 - m.height as f32 * scale) / 2.0;
    if (scale - 1.0).abs() < 1e-3 {
        c.draw(&img, x as i32, y as i32, false);
    } else {
        c.draw_scaled(&img, x, y, scale);
    }
    c
}

/// Punctuation followed by a short pause when text appears letter by
/// letter.
fn is_pause_punctuation(c: char) -> bool {
    matches!(
        c,
        '。' | '、' | '，' | '！' | '？' | '…' | '．' | '.' | ',' | '!' | '?'
    )
}

fn vm_error(e: silky_vm::Error) -> Error {
    Error::Invalid(e.to_string())
}
