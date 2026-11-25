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
use crate::games::apeiria::REPLAY_VAR;
use crate::games::doukyuusei::room;
use crate::games::doukyuusei::town::CANCEL_VAR;
use crate::games::doukyuusei::Panel as DoukPanel;
use crate::games::kimaten2::archive::{
    self, Archive, ArchiveAction, ArchiveScreen, Book, CostumeScreen,
};
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
    veil: Option<Veil>,
    /// A script asked to quit (`Pause::Quit`) and its box is up; the
    /// script goes on when the box is gone and the game is not over.
    script_quit: bool,
    /// Scene without the interface, rendered again only when it changes.
    base: Option<Canvas>,
    /// Whether the screen was covered when the last transition began or
    /// the last frame was drawn (see `Busy::ui_fade`).
    was_covered: bool,
    /// 同級生 Remake: the plate as it was drawn before the flag list's
    /// jump (the original keeps that picture until the script draws the
    /// plate again), and whether it has faded out since.
    plate_hold: Option<crate::games::doukyuusei::plate::Plate>,
    plate_gone: bool,
    /// The part of `base` below the E-mote pictures, and whether those
    /// moved since `base` was drawn: an E-mote frame redraws only what
    /// lies above them.
    under: Option<Canvas>,
    emote_moved: bool,
    /// Whether the cached scene picture was made with the message window.
    base_window: bool,
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
    /// きまぐれテンプテーション2's notebook, collection and costume screens.
    archive: Archive,
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
    /// `.vo` pictures still to come: clock time and picture.
    vo_steps: Vec<(u64, crate::games::fuyukuru::vo::Step)>,
    /// Thumbnail of the game as it was when a screen opened.
    thumb: Option<Image>,
    /// The backlog plays the marked voices one after another.
    playing_marked: bool,
    /// きまぐれテンプテーション2's H mode, over the game.
    hmode: Option<crate::games::kimaten2::hmode::HMode>,
    /// 同級生 Remake's walking map: the input held and the time towards
    /// the next step.
    town_input: crate::games::doukyuusei::town::Input,
    town_pointer_down: bool,
    /// The press just handled acted already (a walk cancel or a map jump
    /// choice), so its release does nothing.
    press_done: bool,
    town_ms: u64,
    /// The screen element being clicked (for a dialog it opens).
    clicked: Option<Button>,
    /// The comment typed for the save about to be made.
    save_comment: String,
    /// 同級生 Remake's system menu screens.
    douk_system: Option<crate::games::doukyuusei::system::System>,
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
    /// The picture turns and scales meanwhile.
    spin: Option<crate::host::Spin>,
    /// 同級生 Remake: the status plate and the menu button go with a
    /// fade out to a colour (`Some(true)`) and come back with the fade
    /// in (`Some(false)`).
    ui_fade: Option<bool>,
    /// A wait that Enter ends too (きまぐれテンプテーション's text strip).
    key_wait: bool,
}

/// Length of one frame for input polls.
const FRAME_MS: u32 = 16;

/// 景の海のアペイリア's extras fade in from black when they open and out to
/// black when they close: 12 steps of the 20 ms tick.
#[derive(Debug, Clone, Copy)]
struct Veil {
    start: u64,
    out: bool,
    /// Closing: the result to leave the extras with once black.
    then: Option<i32>,
}

impl Veil {
    const STEPS: u64 = 12;
    const STEP_MS: u64 = 20;

    fn done(&self, clock: u64) -> bool {
        clock >= self.start + Self::STEPS * Self::STEP_MS
    }

    /// Opacity of the black over the screen.
    fn alpha(&self, clock: u64) -> u8 {
        let step = ((clock - self.start) / Self::STEP_MS).min(Self::STEPS);
        let shown = if self.out { step } else { Self::STEPS - step };
        (shown * 255 / Self::STEPS) as u8
    }
}

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
        let archive = Archive::load(&assets);
        let douk_system = (assets.game().title == silky_format::game::Title::Doukyuusei)
            .then(|| crate::games::doukyuusei::system::System::load(&assets, config.language).ok())
            .flatten();
        let mut renderer = Renderer::new(text, config.language);
        renderer.set_title(assets.game().title);
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
            veil: None,
            script_quit: false,
            base: None,
            was_covered: false,
            plate_hold: None,
            plate_gone: false,
            under: None,
            emote_moved: false,
            base_window: false,
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
            archive,
            movie: None,
            logged: 0,
            choice_points: Vec::new(),
            modes_at_choice: (false, false),
            skip_to_choice: false,
            reveal_all: false,
            shown_chars: None,
            system_stored: 0,
            jolts: Vec::new(),
            vo_steps: Vec::new(),
            hmode: None,
            town_input: Default::default(),
            town_pointer_down: false,
            press_done: false,
            town_ms: 0,
            clicked: None,
            save_comment: String::new(),
            douk_system,
        };
        s.restart()?;
        Ok(s)
    }

    /// Where to keep save files and system data (`system.json`: settings
    /// and what was seen). Without one, nothing is kept.
    pub fn set_saves(&mut self, saves: Saves) {
        self.ui.has_saves = !saves.list().is_empty();
        let mut data = SystemData::load(saves.dir(), self.rt.assets.game().title);
        let seen = std::mem::take(&mut self.rt.system);
        data.seen.extend(seen.seen);
        data.music.extend(seen.music);
        data.read.extend(seen.read);
        if data.flags.is_empty() {
            data.flags = seen.flags;
        }
        self.rt.system = data;
        self.saves = Some(saves);
        self.apply_global_bytes();
        let old = Settings {
            language: Some(self.rt.config.language),
            ..self.rt.system.settings.clone()
        };
        self.settings_changed(old);
        self.rt.system_changed = false;
    }

    /// Records frames for a GPU compositor ([`crate::canvas::Op`]) instead
    /// of drawing them: [`Session::frame`] then returns a recording.
    pub fn set_recording(&mut self, record: bool) {
        self.renderer.record = record;
        self.base = None;
        self.under = None;
        self.frame = None;
    }

    /// Draws E-mote characters on `gpu` instead of in software.
    #[cfg(feature = "emote")]
    pub fn set_emote_gpu(&mut self, gpu: crate::emote::Gpu) {
        self.renderer.set_emote_gpu(gpu);
        self.frame = None;
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
        self.rt.set_language(&self.vm, language);
        self.screens = Screens::load(&self.rt.assets, language);
        self.menus = Menus::load(&self.rt.assets, language);
        self.renderer.set_language(language);
        if self.douk_system.is_some() {
            self.douk_system =
                crate::games::doukyuusei::system::System::load(&self.rt.assets, language).ok();
        }
    }

    /// Saves the shown message to `slot` (see [`save::QUICK`]).
    pub fn save(&mut self, slot: &str) -> Result<()> {
        self.save_with_comment(slot, String::new())
    }

    /// Saves to `slot`, keeping the comment typed for it (同級生 Remake).
    fn save_with_comment(&mut self, slot: &str, comment: String) -> Result<()> {
        let Some(saves) = &self.saves else {
            return Ok(());
        };
        // At a selection or an action menu, the save holds the message
        // before it (loading it shows that message, then the menu again).
        let state = match self.state {
            State::Text => self.rt.save_state(&self.vm),
            // The walking map: the VM waits in the panel call, which is
            // the state the original's load jumps back to.
            State::Choice
                if self.douk_system.is_some()
                    && matches!(self.rt.panel, Some(crate::games::doukyuusei::Panel::Town)) =>
            {
                self.rt.save_state(&self.vm)
            }
            State::Choice => match self.log.last() {
                Some(e) => e.state.clone(),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let saves = saves.clone();
        let thumb = match &self.thumb {
            Some(t) => Some(t.clone()),
            None => self.scene_thumb().ok(),
        };
        saves.write_with_comment(slot, state, thumb.as_ref(), comment)?;
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
        let canvas = canvas.rasterize();
        let size = match self.douk_system {
            Some(_) => crate::games::doukyuusei::system::THUMB,
            None => save::THUMB_SIZE,
        };
        Ok(save::thumbnail_sized(
            &canvas.rgba,
            canvas.width,
            canvas.height,
            size,
        ))
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
        let kept_at_panel = state.at_panel;
        self.rt.load_state(&mut self.vm, state)?;
        self.apply_global_bytes();
        if self.log.is_empty() {
            if let Some(text) = self.rt.scene.text.as_ref().filter(|t| !t.is_empty()) {
                self.logged += 1;
                self.log.push(LogEntry {
                    speaker: self.rt.scene.speaker,
                    speaker_name: self.rt.scene.speaker_name.clone(),
                    speaker_hero: self.rt.scene.speaker_hero,
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
        self.vo_steps.clear();
        self.base = None;
        self.frame = None;
        self.update_previous_choice();
        if kept_at_panel {
            // Back on the walking map: the walk input waits again.
            self.ui.panel = self.rt.panel.clone();
            self.ui.waiting = false;
            self.state = State::Choice;
            self.town_ms = 0;
            self.town_input = Default::default();
            self.town_pointer_down = false;
        }
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
        let mut vm = Vm::new(Profile::default());
        vm.trace_continue(&mut self.vm);
        self.vm = vm;
        self.rt.reset();
        self.rt.boot(&mut self.vm, &self.start)?;
        self.apply_global_bytes();
        self.clear_play()
    }

    /// Back to the title menu, as the game menu's "title" button does: the
    /// executable starts `main.mes` again (the title loop of both games)
    /// with the variables kept, so the logos do not play again.
    pub fn to_title(&mut self) -> Result<()> {
        const TITLE: &str = "main.mes";
        let Ok(script) = self.rt.assets.script(TITLE) else {
            return self.restart();
        };
        self.rt.reset();
        self.vm.restart_at(TITLE, script);
        self.apply_global_bytes();
        self.clear_play()
    }

    /// Forgets the game in play (log, screens, queued effects) and runs
    /// the scripts from where they now stand.
    fn clear_play(&mut self) -> Result<()> {
        self.busy = None;
        self.screen = None;
        self.dialog = None;
        self.log.clear();
        self.logged = 0;
        self.choice_points.clear();
        self.voice_queue.clear();
        self.movie = None;
        self.jolts.clear();
        self.vo_steps.clear();
        self.skip_to_choice = false;
        self.ui = Ui {
            has_saves: self.ui.has_saves,
            ..Ui::default()
        };
        self.run()
    }

    /// 同級生 Remake: the flag list's jump to an event. The story state is
    /// set for the event, what is up (the list, the walk, the menu, the
    /// message) goes, and the event's script starts from its beginning with
    /// the variables kept, as when the title menu is left for a script.
    fn jump_to_event(&mut self) -> Result<()> {
        use crate::games::doukyuusei::flaglist;
        let Some(Screen::DoukFlag(s)) = &self.screen else {
            return Ok(());
        };
        let Some(rec) = s.target().cloned() else {
            return Ok(());
        };
        let Ok(script) = self.rt.assets.script(rec.script) else {
            self.rt
                .note_unhandled(format!("event script {}", rec.script));
            return Ok(());
        };
        self.plate_hold = self.rt.plate(&self.vm);
        self.plate_gone = false;
        flaglist::start_event(&rec, &mut self.vm);
        self.store_global_bytes();
        self.rt.leave_for_event();
        self.vm.restart_at(rec.script, script);
        self.busy = None;
        self.screen = None;
        self.thumb = None;
        self.dialog = None;
        self.voice_queue.clear();
        self.movie = None;
        self.jolts.clear();
        self.vo_steps.clear();
        self.skip_to_choice = false;
        self.town_ms = 0;
        self.town_input = Default::default();
        self.town_pointer_down = false;
        self.ui = Ui {
            has_saves: self.ui.has_saves,
            ..Ui::default()
        };
        self.state = State::Busy;
        // The picture that was up stays (the walking map) for the script's
        // own fade to start from.
        self.ui.plate = self.plate_hold;
        self.was_covered = self.rt.scene.cover.is_some();
        let under = self
            .renderer
            .render_under(&self.rt.scene, &mut self.rt.assets)?;
        self.base = Some(self.renderer.render_over(
            under,
            &self.rt.scene,
            &mut self.rt.assets,
            false,
            false,
        )?);
        self.frame = None;
        self.run()
    }

    /// Runs the scripts until they wait for the player.
    fn run(&mut self) -> Result<()> {
        // 景の海のアペイリア makes a voice line's picture change at once
        // when the message it came with goes.
        if self.state == State::Text && self.menus.is_apeiria() {
            for (_, step) in std::mem::take(&mut self.vo_steps) {
                step.apply(&mut self.rt.scene);
            }
        }
        let mut budgets = 0;
        loop {
            let stop = if self.rt.follow_up(&mut self.vm) {
                if self.rt.pause.is_none() {
                    continue;
                }
                Stop::Yield
            } else {
                self.vm.run(&mut self.rt, STEP_BUDGET).map_err(vm_error)?
            };
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
                                speaker_name: self.rt.scene.speaker_name.clone(),
                                speaker_hero: self.rt.scene.speaker_hero,
                                text: text.clone(),
                                voice: self.rt.voice.clone(),
                                state: self.rt.save_state(&self.vm),
                                marked: false,
                            });
                        }
                        if std::mem::take(&mut self.rt.auto_save)
                            && self.rt.system.settings.auto_save_at_choices
                        {
                            self.auto_save();
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
                        self.ui.title = Some(self.rt.k1_title_extras);
                        self.ui.title_shown = self.ui.clock;
                        self.ui.title_opened = self.ui.clock;
                        self.ui.locked_titles = self
                            .layout
                            .title_unlocks()
                            .iter()
                            .filter(|&&(_, var)| self.vm.var_b(var) == 0)
                            .map(|&(entry, _)| entry)
                            .collect();
                        self.ui.auto = false;
                        self.ui.skip = false;
                        break;
                    }
                    Some(Pause::Transition(t)) => {
                        if std::mem::take(&mut self.rt.k1_blink) {
                            self.ui.k1_blink = Some(self.ui.clock);
                        }
                        // A live wait shows the train on its way.
                        if let (true, Some(train)) = (t.live, &mut self.rt.scene.train) {
                            train.progress(0, t.ms);
                        }
                        if let (true, Some(intro)) = (t.live, &mut self.rt.scene.intro) {
                            intro.progress(0, t.ms);
                        }
                        if let (true, Some(pan)) = (t.live, &mut self.rt.scene.pan) {
                            pan.progress(0, t.ms);
                        }
                        let from = self.base.take().filter(|_| !t.live);
                        let rule = t.rule.and_then(|n| self.rule(n));
                        let covered = self.rt.scene.cover.is_some();
                        let ui_fade = (covered != self.was_covered).then_some(covered);
                        self.was_covered = covered;
                        if ui_fade == Some(false) {
                            self.plate_hold = None;
                            self.plate_gone = false;
                        }
                        self.busy = Some(Busy {
                            from,
                            rule,
                            ms: t.ms,
                            elapsed: 0,
                            frame: false,
                            spin: t.spin,
                            ui_fade,
                            key_wait: std::mem::take(&mut self.rt.k1_keywait),
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
                            spin: None,
                            ui_fade: None,
                            key_wait: false,
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
                        // The mail forward screen has no bars: it takes
                        // clicks on its own rows.
                        let mail = self.rt.scene.mail_select.as_ref().is_some_and(|m| m.shown);
                        self.ui.choices = if mail {
                            Vec::new()
                        } else {
                            self.rt.choices.clone()
                        };
                        self.ui.choice_status = self.rt.choice_status.clone();
                        self.ui.choice_page = 0;
                        self.ui.waiting = false;
                        self.state = State::Choice;
                        self.modes_at_choice = (self.ui.skip && !self.skip_to_choice, self.ui.auto);
                        self.skip_to_choice = false;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        break;
                    }
                    Some(Pause::Actions) => {
                        if self.rt.system.settings.auto_save_at_choices {
                            self.auto_save();
                        }
                        self.ui.actions = self.rt.actions.clone();
                        self.ui.waiting = false;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        self.skip_to_choice = false;
                        self.state = State::Choice;
                        self.base = None;
                        break;
                    }
                    Some(Pause::Map) => {
                        self.ui.map = self.rt.map.clone();
                        self.ui.waiting = false;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        self.skip_to_choice = false;
                        self.state = State::Choice;
                        self.base = None;
                        break;
                    }
                    Some(Pause::Panel) => {
                        let had_plate = self.ui.plate.is_some();
                        self.ui.panel = self.rt.panel.clone();
                        self.room_opens(had_plate);
                        self.ui.waiting = false;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        self.skip_to_choice = false;
                        self.state = State::Choice;
                        self.base = None;
                        break;
                    }
                    Some(Pause::Kimaten1) => {
                        self.show_k1();
                        break;
                    }
                    Some(Pause::HMode) => {
                        self.open_hmode();
                        break;
                    }
                    Some(Pause::Quit) => {
                        self.ui.skip = false;
                        self.ui.auto = false;
                        self.ui.hover = None;
                        self.skip_to_choice = false;
                        self.state = State::Choice;
                        self.script_quit = true;
                        self.frame = None;
                        self.ask(Action::Quit)?;
                        break;
                    }
                    Some(Pause::Costume(mode)) => {
                        let vm = &self.vm;
                        let mut c = CostumeScreen::with_mode(|i| vm.var_b(i), mode);
                        c.script = true;
                        self.screen = Some(Screen::Costume(c));
                        self.ui.title = None;
                        self.ui.waiting = false;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        self.skip_to_choice = false;
                        self.ui.hover = None;
                        self.state = State::Choice;
                        self.base = None;
                        self.frame = None;
                        break;
                    }
                    Some(Pause::Extras(kind)) => {
                        self.open_extras(kind);
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
                            self.rt.answer_movie(&mut self.vm, false);
                        }
                    },
                    None => {}
                },
                Stop::Options => {
                    let numbers: Vec<i32> =
                        self.vm.options().iter().map(|(_, v)| v.int()).collect();
                    // A further run of options for the same selection
                    // keeps the labels of the runs before it.
                    let kept = if self.rt.option_count > 0 {
                        std::mem::take(&mut self.vm.option_labels)
                    } else {
                        Vec::new()
                    };
                    self.rt.begin_options(&numbers);
                    self.vm.run_all_options().map_err(vm_error)?;
                    self.vm.option_labels = kept;
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
        // A new message ends the pictures of the last voice line.
        if self.state == State::Text || !self.rt.vo_steps.is_empty() {
            self.vo_steps = self
                .rt
                .vo_steps
                .drain(..)
                .map(|(at, step)| (now + at as u64, step))
                .collect();
        }
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

    /// Shows the `.vo` pictures that are due and moves a quaking screen
    /// (8 frames a cycle: down, up, then six still).
    fn play_effects(&mut self) {
        let clock = self.ui.clock;
        if self.vo_steps.first().is_some_and(|(at, _)| *at <= clock) {
            let n = self
                .vo_steps
                .iter()
                .take_while(|(at, _)| *at <= clock)
                .count();
            let due: Vec<_> = self.vo_steps.drain(..n).collect();
            for (_, step) in due {
                step.apply(&mut self.rt.scene);
            }
            self.base = None;
            self.frame = None;
        }
        if let Some(b) = self.rt.scene.panels.battle.as_mut() {
            let drained = !b.drains.is_empty() && b.advance(clock);
            if b.step(clock) || drained {
                self.base = None;
                self.frame = None;
            }
        }
        let step = (clock * 60 / 1000) as usize;
        let shake = if let Some(a) = self.rt.scene.quake {
            Some((
                0,
                match step % 8 {
                    0 => a,
                    1 => -a,
                    _ => 0,
                },
            ))
        } else {
            self.rt
                .scene
                .quake_steps
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| s[step % s.len()])
        };
        if let Some(o) = shake {
            if self.rt.scene.shake != o {
                self.rt.scene.shake = o;
                self.frame = None;
            }
        }
    }

    /// Moves jolting characters to their offset for the clock time.
    fn play_jolts(&mut self) {
        if self.jolts.is_empty() && self.rt.scene.jolts.is_empty() {
            return;
        }
        let clock = self.ui.clock;
        let mut now = std::collections::BTreeMap::new();
        self.jolts.retain(|(slot, steps, start)| {
            let step = ((clock - start) / steps.step_ms.max(1)) as usize;
            if let Some(&o) = steps.steps.get(step) {
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
        self.ui.jump_off = self.layout.is_kimaten2() && !self.rt.read_before;
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
        let mut t = 0;
        for (i, c) in text.chars().filter(|&c| c != '\n').enumerate() {
            let wait = self.char_wait(c);
            t += wait;
            // Doujin's window shows a character, then waits; the others
            // wait first.
            let at = if self.own_typing() { t - wait } else { t };
            if at > self.shown_ms {
                return Some(i);
            }
        }
        None
    }

    /// Whether the message window of this game types at its own pace
    /// (`games::doujin::window::char_wait`).
    fn own_typing(&self) -> bool {
        self.rt.assets.game().title == silky_format::game::Title::Doujin
    }

    /// Time that passes after the character `c` of a typed message.
    fn char_wait(&self, c: char) -> u64 {
        let st = &self.rt.system.settings;
        if self.own_typing() {
            return crate::games::doujin::window::char_wait(st, c);
        }
        let pause = st.punctuation_pause && is_pause_punctuation(c);
        st.char_ms() * if pause { 1 + PUNCTUATION_PAUSE } else { 1 }
    }

    /// Time the whole message takes to appear.
    fn reveal_ms(&self) -> u64 {
        if let Some(ms) = crate::games::doujin::comic::typing_ms(&self.rt) {
            return ms;
        }
        let st = &self.rt.system.settings;
        let Some(text) = self.rt.scene.text.as_deref() else {
            return 0;
        };
        if !st.typewriter || self.reveal_all {
            return 0;
        }
        let waits: Vec<u64> = text
            .chars()
            .filter(|&c| c != '\n')
            .map(|c| self.char_wait(c))
            .collect();
        let total: u64 = waits.iter().sum();
        if self.own_typing() {
            // The last character comes up before its own wait.
            total - waits.last().copied().unwrap_or(0)
        } else {
            total
        }
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
        let busy = self.busy.take();
        if let Some(train) = &mut self.rt.scene.train {
            if train.frame.take().is_some() {
                self.base = None;
            }
        }
        // The pan stops at its end position as the script runs on.
        if self.rt.scene.pan.take().is_some() {
            self.base = None;
        }
        // The intro's picture becomes the background as the script runs on.
        if self.rt.scene.intro.is_some() {
            self.base = None;
        }
        if busy.as_ref().is_some_and(|b| b.ui_fade == Some(true)) {
            self.plate_gone = true;
        }
        if busy.is_some() && self.state == State::Busy {
            // Effects the script waited for end with the wait.
            self.rt.scene.effects.settle();
            self.take_painted();
            self.run()?;
        }
        Ok(())
    }

    /// Moves what the effects drew into the background to the scene.
    fn take_painted(&mut self) {
        for paint in self.rt.scene.effects.take_painted() {
            self.rt.scene.panels.paint(paint);
            self.base = None;
            self.frame = None;
        }
    }

    /// Goes on from the current message.
    fn advance(&mut self) -> Result<()> {
        // The doujinshi screen puts the rest of a bubble's words up first.
        if self.state == State::Text && crate::games::doujin::comic::finish(&mut self.rt) {
            self.frame = None;
            return Ok(());
        }
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

    /// Whether the game draws its own pointer, so the system's is hidden.
    pub fn own_cursor(&self) -> bool {
        self.hmode.as_ref().is_some_and(|h| h.own_cursor()) || self.game_cursor_shown()
    }

    /// 同級生 Remake: whether the game's own pointer (the picture of the
    /// verb of the hot spot under the pointer) is up, so the system's is
    /// hidden.
    pub fn game_cursor_shown(&self) -> bool {
        let (x, y) = self.ui.pointer;
        self.rt.cursor.shown() && crate::games::doukyuusei::cursor::inside(x, y)
    }

    /// Sets the game's own pointer from the hover, as the program does
    /// when the focus changes; away while a dialog, a screen or a fade is
    /// up. Returns whether the picture changed.
    fn sync_cursor(&mut self) -> bool {
        use crate::games::doukyuusei::room::Room;
        let want = match &self.ui.panel {
            Some(DoukPanel::Room(_))
                if self.dialog.is_none() && self.screen.is_none() && self.veil.is_none() =>
            {
                match self.ui.hover {
                    Some(Button::Panel(id)) => Room::cursor_picture(Some(id)),
                    _ => 0,
                }
            }
            _ => 0,
        };
        self.rt.cursor.sync(want, self.ui.clock)
    }

    /// 同級生 Remake: the walk on the map is up.
    fn walking(&self) -> bool {
        use crate::games::doukyuusei::Panel;
        self.screen.is_none() && self.dialog.is_none() && matches!(self.ui.panel, Some(Panel::Town))
    }

    /// Arrow keys held ([`crate::games::doukyuusei::town::dir`] bits) and
    /// Shift or Ctrl, for the walking map of 同級生 Remake.
    pub fn set_held_keys(&mut self, keys: u8, fast: bool) {
        self.town_input.keys = keys;
        self.town_input.fast = fast;
    }

    /// A click, Escape or Delete during the automatic walk cancels it at
    /// its next step if [`CANCEL_VAR`] is set. Returns whether it did.
    pub fn cancel_walk(&mut self) -> bool {
        if !self.walking() || !self.rt.town_walking() || self.vm.var_b(CANCEL_VAR) == 0 {
            return false;
        }
        self.rt.cancel_town_walk();
        true
    }

    /// Steps of the walk on the map, one per step time; a place reached
    /// ends the walk with its id.
    fn tick_town(&mut self, ms: u64) -> Result<()> {
        use crate::games::doukyuusei::town::step_ms;
        self.town_ms += ms;
        let step = step_ms(self.rt.system.settings.map_speed);
        self.town_input.pointer = self.town_pointer_down.then_some(self.ui.pointer);
        self.town_input.hold = self.vm.var_b(CANCEL_VAR) != 0;
        while self.town_ms >= step {
            self.town_ms -= step;
            let input = self.town_input;
            let found = self.rt.town_tick(&mut self.vm, &input);
            self.base = None;
            self.frame = None;
            if let Some(id) = found {
                self.town_ms = 0;
                self.town_pointer_down = false;
                self.ui.panel = None;
                self.ui.side_menu = false;
                self.rt.answer_action(&mut self.vm, id);
                self.run()?;
                break;
            }
        }
        Ok(())
    }

    /// Whether the screen takes typed text (the name entry).
    pub fn wants_text(&self) -> bool {
        matches!(&self.ui.panel, Some(DoukPanel::Name(n)) if n.typing())
            || self.dialog.as_ref().is_some_and(|d| d.commenting)
    }

    /// Whether the screen takes input method text: the same screens, in
    /// a language with an input method (Chinese). English types on the
    /// keyboard and has none.
    pub fn wants_ime(&self) -> bool {
        self.wants_text() && self.rt.config.language != Language::English
    }

    /// Typed or committed text (an input method's result); Backspace
    /// comes as `\u{8}`. In the save dialog English takes digits, `@` and
    /// letters to 20 columns, Chinese anything to 40.
    pub fn text_input(&mut self, text: &str) {
        use crate::games::doukyuusei::savenote;
        let english = self.rt.config.language == Language::English;
        if let Some(d) = self.dialog.as_mut().filter(|d| d.commenting) {
            d.preedit.clear();
            if english {
                savenote::type_ascii(&mut d.comment, text);
            } else {
                savenote::commit(&mut d.comment, text);
            }
            self.frame = None;
            return;
        }
        if self.screen.is_some() || self.dialog.is_some() || self.menu_sliding() {
            return;
        }
        if let Some(DoukPanel::Name(n)) = &mut self.ui.panel {
            n.preedit.clear();
            if n.type_text(text) {
                self.frame = None;
            }
        }
    }

    /// The input method's composition changed (empty when it ended): it
    /// is drawn after the text.
    pub fn preedit(&mut self, text: &str) {
        if let Some(d) = self.dialog.as_mut().filter(|d| d.commenting) {
            if d.preedit != text {
                d.preedit = text.to_string();
                self.frame = None;
            }
            return;
        }
        if self.screen.is_some() || self.dialog.is_some() {
            return;
        }
        if let Some(DoukPanel::Name(n)) = &mut self.ui.panel {
            if n.preedit != text {
                n.preedit = text.to_string();
                self.frame = None;
            }
        }
    }

    /// Ctrl+V with the clipboard's text (the frontend reads it). In the
    /// save dialog it replaces the whole comment; on the name entry it
    /// goes in as the program does.
    pub fn paste(&mut self, text: &str) {
        use crate::games::doukyuusei::savenote;
        if let Some(d) = self.dialog.as_mut().filter(|d| d.commenting) {
            savenote::paste(&mut d.comment, text);
            self.frame = None;
            return;
        }
        if self.screen.is_some() || self.dialog.is_some() || self.menu_sliding() {
            return;
        }
        if let Some(DoukPanel::Name(n)) = &mut self.ui.panel {
            if n.paste(text) {
                self.frame = None;
            }
        }
    }

    /// Pointer moved to (x, y) in screen pixels.
    pub fn pointer_move(&mut self, x: i32, y: i32) {
        self.ui.pointer = (x, y);
        // A bar of the music mode follows the pointer while held.
        if let (Some(Button::Kimaten1(id)), Some(k)) = (
            self.ui.pressed,
            self.ui.k1_menu.as_mut().or(self.ui.k1.as_mut()),
        ) {
            k.press_at(id, x, y);
            let _ = self.k1_answer(crate::games::kimaten1::Click::None);
        }
        if let Some(h) = &mut self.hmode {
            h.pointer_move(x, y);
            return;
        }
        if let Some(Screen::Settings(SettingsScreen {
            dragging: Some(id), ..
        })) = &self.screen
        {
            let id = *id;
            self.drag_settings(id, x, y);
            return;
        }
        if let Some(Screen::DoukConfig(s)) = &self.screen {
            if let Some(id) = s.dragging {
                self.drag_douk_config(id, x);
                return;
            }
        }
        if let Some(DoukPanel::Music(m)) = &self.ui.panel {
            if m.dragging {
                self.drag_music_volume(x);
                return;
            }
        }
        if let Some(Screen::DoukFlag(s)) = &mut self.screen {
            if s.dragging {
                if s.drag(y) {
                    self.frame = None;
                }
                return;
            }
        }
        if let Some(Screen::Extras(e)) = &self.screen {
            if e.k2.dragging_volume || e.ap.dragging_volume {
                self.drag_omake_volume(x);
                return;
            }
            if e.ap.dragging_scroll {
                self.drag_apeiria_scroll(y);
                return;
            }
        }
        if self.state == State::Choice
            && self.dialog.is_none()
            && self.screen.is_none()
            && self.rt.mail_select_hover(x, y)
        {
            self.frame = None;
        }
        if crate::games::doujin::comic::hover(&mut self.rt, x, y) {
            self.frame = None;
        }
        let hover = self.hit(x, y);
        if hover != self.ui.hover {
            self.ui.hover = hover;
            self.frame = None;
            self.apeiria_preview();
            if let Some(DoukPanel::Name(n)) = &mut self.ui.panel {
                n.set_focus(match hover {
                    Some(Button::Panel(id)) => Some(id),
                    _ => None,
                });
            }
        }
        // The hint bar steps aside for the pointer.
        let up = matches!(&self.ui.panel, Some(DoukPanel::Room(r)) if r.hint_dodges(x, y));
        if up != self.ui.hint_up {
            self.ui.hint_up = up;
            self.frame = None;
        }
        // The game's own pointer follows the system's.
        if self.sync_cursor() || self.rt.cursor.shown() {
            self.frame = None;
        }
    }

    /// Primary button pressed at (x, y). On the map jump a press on a
    /// place chooses it at once; its release does nothing.
    pub fn pointer_down(&mut self, x: i32, y: i32) -> Result<()> {
        if self.menu_sliding() {
            self.press_done = true;
            return Ok(());
        }
        // The mail forward screen takes its own clicks, not the choice bars.
        if self.state == State::Choice
            && self.dialog.is_none()
            && self.screen.is_none()
            && self.rt.scene.mail_select.as_ref().is_some_and(|m| m.shown)
        {
            self.press_done = true;
            self.base = None;
            // A click while the message window is hidden only brings it back.
            if self.ui.window_hidden {
                self.ui.window_hidden = false;
                self.frame = None;
                return Ok(());
            }
            if self.rt.mail_select_click(&mut self.vm, x, y)
                && !self.rt.scene.mail_select.as_ref().is_some_and(|m| m.shown)
            {
                self.ui.choices.clear();
                self.ui.choice_status.clear();
                let (skip, auto) = self.modes_at_choice;
                let st = &self.rt.system.settings;
                self.ui.skip = skip && st.keep_skip_after_choices;
                self.ui.auto = auto && st.keep_auto_after_choices;
                self.run()?;
            }
            return Ok(());
        }
        if self.veil.is_none() && self.hmode.is_none() {
            if let (Some(DoukPanel::Jump(_)), Some(Button::Panel(id))) =
                (&self.ui.panel, self.hit(x, y))
            {
                if id != crate::games::doukyuusei::jump::BACK {
                    self.press_done = true;
                    return self.press(Button::Panel(id));
                }
            }
        }
        self.press_down(x, y);
        Ok(())
    }

    fn press_down(&mut self, x: i32, y: i32) {
        // Input waits for the extras' fade.
        if self.veil.is_some() {
            return;
        }
        if let Some(h) = &mut self.hmode {
            let mut ops = Vec::new();
            h.pointer_down(x, y, &mut ops);
            self.apply_hmode(ops);
            return;
        }
        self.ui.pressed = self.hit(x, y);
        self.frame = None;
        // The notebook's thumb jumps under the pointer as it is pressed.
        if let Some(Button::Kimaten1(id)) = self.ui.pressed {
            use crate::games::kimaten1::Screen as K1;
            if let Some(s @ (K1::Notebook(_) | K1::Map(_))) =
                self.ui.k1_menu.as_mut().or(self.ui.k1.as_mut())
            {
                s.press_at(id, x, y);
            }
        }
        // A click that cancels the walk is no click on the map.
        let cancelled = self.cancel_walk();
        if self.walking() && self.ui.pressed.is_none() {
            if cancelled {
                self.press_done = true;
            } else {
                self.ui.pointer = (x, y);
                self.town_pointer_down = true;
            }
            return;
        }
        if let (Some(Screen::Extras(e)), Some(Button::Screen(id))) =
            (&mut self.screen, self.ui.pressed)
        {
            if self.menus.is_kimaten2()
                && e.kind == Extras::Music
                && e.k2.view.is_none()
                && id == crate::games::kimaten2::omake::ids::VOLUME
            {
                e.k2.dragging_volume = true;
                self.drag_omake_volume(x);
                return;
            }
            // ふゆから、くるる。: the volume bar.
            if !self.menus.is_apeiria()
                && !self.menus.is_kimaten2()
                && e.kind == Extras::Music
                && id == extras_ids::PAGE
            {
                e.ap.dragging_volume = true;
                self.drag_omake_volume(x);
                return;
            }
            if self.menus.is_apeiria() && e.kind == Extras::Music && e.ap.view.is_none() {
                use crate::games::apeiria::omake::ids;
                if id == ids::VOLUME {
                    e.ap.dragging_volume = true;
                    self.drag_omake_volume(x);
                    return;
                }
                if id == ids::SCROLL {
                    e.ap.dragging_scroll = true;
                    self.drag_apeiria_scroll(y);
                    return;
                }
                // The arrows scroll once as they go down, and not again
                // while held (as under Wine).
                if id == ids::PREVIOUS || id == ids::NEXT {
                    let d = if id == ids::NEXT { 1 } else { -1 };
                    let max = e.ap.t.max_scroll() as isize;
                    e.scroll = (e.scroll as isize + d).clamp(0, max) as usize;
                    self.ui.pressed = None;
                    return;
                }
            }
        }
        if let (Some(DoukPanel::Music(m)), Some(Button::Panel(id))) =
            (&mut self.ui.panel, self.ui.pressed)
        {
            if id == crate::games::doukyuusei::music::VOLUME && m.on {
                m.dragging = true;
                self.drag_music_volume(x);
                return;
            }
        }
        if let (Some(Screen::Settings(s)), Some(Button::Screen(id))) =
            (&mut self.screen, self.ui.pressed)
        {
            if menus::is_slider(s.page, id) || id == set_ids::SCROLL {
                s.dragging = Some(id);
                self.drag_settings(id, x, y);
            }
        }
        if let (Some(Screen::DoukFlag(s)), Some(Button::Screen(id))) =
            (&mut self.screen, self.ui.pressed)
        {
            use crate::games::doukyuusei::flaglist::{ids, Page};
            if id == ids::TRACK && s.page == Page::Calendar && !s.notice {
                s.press_track(y);
            }
        }
        if let (Some(Screen::DoukConfig(s)), Some(Button::Screen(id))) =
            (&mut self.screen, self.ui.pressed)
        {
            if crate::games::doukyuusei::config::is_slider(s.page, id) {
                s.dragging = Some(id);
                self.drag_douk_config(id, x);
            }
        }
    }

    /// The volume slider of 同級生 Remake's music mode follows the pointer.
    fn drag_music_volume(&mut self, x: i32) {
        use crate::games::doukyuusei::music::Music;
        let old = self.rt.system.settings.clone();
        let v = Music::volume_at(x);
        self.rt.system.settings.music = v;
        if let Some(DoukPanel::Music(m)) = &mut self.ui.panel {
            m.volume = v;
        }
        self.settings_changed(old);
        self.frame = None;
    }

    /// A slider of 同級生 Remake's settings follows the pointer.
    fn drag_douk_config(&mut self, id: i32, x: i32) {
        use crate::games::doukyuusei::{config, sound};
        let (Some(Screen::DoukConfig(s)), Some(sys)) = (&self.screen, &self.douk_system) else {
            return;
        };
        let Some(a) = sys.config_area(s.page, id) else {
            return;
        };
        let old = self.rt.system.settings.clone();
        let changed = match s.page {
            config::Page::Sound => {
                let value = sound::slider_value(a, x);
                sound::apply(&mut self.rt.system.settings, s.voices, id, value)
            }
            _ => config::apply(
                &mut self.rt.system.settings,
                id,
                config::slider_value(a, id, x),
            ),
        };
        if changed {
            self.settings_changed(old);
        }
        self.frame = None;
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
            let (_, ty, _, th) = self.menus.track();
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
        if std::mem::take(&mut self.press_done) {
            return Ok(());
        }
        // Input waits for the extras' fade.
        if self.veil.is_some() || self.menu_sliding() {
            return Ok(());
        }
        if let Some(h) = &mut self.hmode {
            let mut ops = Vec::new();
            h.pointer_up(x, y, &mut ops);
            self.apply_hmode(ops);
            return Ok(());
        }
        if std::mem::take(&mut self.town_pointer_down) {
            return Ok(());
        }
        let pressed = self.ui.pressed.take();
        self.frame = None;
        if let Some(DoukPanel::Music(m)) = &mut self.ui.panel {
            if std::mem::take(&mut m.dragging) {
                return Ok(());
            }
        }
        if let Some(Screen::Settings(s)) = &mut self.screen {
            if s.dragging.take().is_some() {
                return Ok(());
            }
        }
        if let Some(Screen::DoukFlag(s)) = &mut self.screen {
            if std::mem::take(&mut s.dragging) {
                return Ok(());
            }
        }
        if let Some(Screen::DoukConfig(s)) = &mut self.screen {
            if let Some(id) = s.dragging.take() {
                if s.page == crate::games::doukyuusei::config::Page::Sound {
                    let voices = s.voices;
                    self.douk_sound_sample(voices, id);
                }
                return Ok(());
            }
        }
        if self.state == State::Movie {
            return self.end_movie();
        }
        if let Some(Screen::Extras(e)) = &mut self.screen {
            if e.k2.dragging_volume || e.ap.dragging_volume || e.ap.dragging_scroll {
                e.k2.dragging_volume = false;
                e.ap.dragging_volume = false;
                e.ap.dragging_scroll = false;
                return Ok(());
            }
            if let Some(v) = &mut e.ap.view {
                if let Some(p) = v.picture {
                    // The next seen picture, through the later entries;
                    // after the last, the grid of the entry shown.
                    use crate::games::apeiria::omake;
                    match omake::next_seen(e.ap.t, &self.vm.bytes, v.entry, p) {
                        Some((entry, p)) => {
                            v.entry = entry;
                            v.picture = Some(p);
                        }
                        None => {
                            v.page = p / 12;
                            v.picture = None;
                        }
                    }
                    self.ui.hover = None;
                    return Ok(());
                }
            }
            if let Some(v) = &mut e.k2.view {
                if let Some(p) = v.picture {
                    // The next picture of the entry, then back to its grid
                    // (or to the screen when the entry has one picture).
                    v.picture =
                        crate::games::kimaten2::omake::next_seen(&self.vm.bytes, v.entry, p);
                    if v.picture.is_none()
                        && crate::games::kimaten2::omake::CG_ENTRIES[v.entry].1 == 1
                    {
                        e.k2.view = None;
                    }
                    return Ok(());
                }
            }
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
        // A system screen over the message takes every click.
        if self.ui.k1_menu.is_some() && !matches!(hit, Some(Button::Kimaten1(_))) {
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
                self.clicked = hit;
                let result = self.press_screen(id, x, y);
                self.clicked = None;
                result?;
            }
            return Ok(());
        }
        if self.state == State::Busy {
            self.rt.clicked = true;
            // The intro of 同級生 Remake takes no click.
            if let Some(b) = &mut self.busy {
                // The staff roll takes no primary click (only a secondary
                // click, Escape, Delete or keypad 0; see `roll_cancel`).
                if !b.frame
                    && self.rt.scene.intro.is_none()
                    && !self.rt.roll_locked
                    && !self.rt.rolling()
                {
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
        // A click beside the open side menu closes it (同級生 Remake's
        // menu stays open beside the room and the maps).
        if self.ui.side_menu && !matches!(hit, Some(Button::Bar(_))) && self.ui.plate.is_none() {
            self.ui.side_menu = false;
            self.frame = None;
            return Ok(());
        }
        match hit {
            Some(b) if !self.ui.enabled(b) => Ok(()),
            Some(b) => self.press(b),
            None if self.state == State::Text => {
                use crate::games::doujin::comic::{click, Click};
                match click(&mut self.rt, x, y) {
                    Click::Used => return Ok(()),
                    Click::Pass => {}
                }
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

    /// The staff roll's cancel (0x4a8780 sends ids 1, 0x52, 0xd3 and the
    /// secondary button to 0x480ff0): Escape, keypad 0, Delete or the
    /// secondary click end it, and the call answers 1. Only while bit 11
    /// of global 55 is set. Returns whether a roll was running.
    fn roll_cancel(&mut self) -> bool {
        if self.state != State::Busy || !self.rt.rolling() {
            return false;
        }
        if !self.rt.roll_locked {
            if let Some(b) = &mut self.busy {
                b.elapsed = b.ms;
                self.rt.cut_roll();
            }
        }
        true
    }

    /// Secondary click: closes a dialog or screen, else does what the
    /// settings say (hide the message window by default).
    pub fn secondary_click(&mut self) -> Result<()> {
        if self.roll_cancel() {
            return Ok(());
        }
        if self.back()? {
            return Ok(());
        }
        if self.replaying() {
            return self.ask(self.replay_action());
        }
        // The mail forward screen of 景の海のアペイリア does what the
        // settings say too.
        if self.mail_select_up() {
            return self.mail_select_secondary();
        }
        if self.poll_cancel() {
            return Ok(());
        }
        if self.state != State::Text {
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
            RightClick::Settings => self.room_system(room::Sys::Settings),
            RightClick::Skip => self.press(Button::Bar(bar::SKIP))?,
            RightClick::Auto => self.press(Button::Bar(bar::AUTO))?,
            RightClick::Load => self.room_system(room::Sys::Load),
            RightClick::Save => self.room_system(room::Sys::Save),
        }
        Ok(())
    }

    /// Escape: ends the walk on the map, closes what is up, or on the
    /// mail forward screen does what a secondary click does.
    pub fn escape(&mut self) -> Result<()> {
        if self.roll_cancel() {
            return Ok(());
        }
        if self.douk_cancel_key()? {
            return Ok(());
        }
        if self.cancel_walk() || self.back()? {
            return Ok(());
        }
        if self.replaying() {
            return self.ask(self.replay_action());
        }
        if self.mail_select_up() {
            return self.mail_select_secondary();
        }
        self.poll_cancel();
        Ok(())
    }

    /// なないろリンカネーション: a secondary click or Escape while the
    /// script polls the input (the CG viewer does) is what the poll for
    /// 257 answers. Returns whether it was taken.
    fn poll_cancel(&mut self) -> bool {
        let polling = self.state == State::Busy && self.busy.as_ref().is_some_and(|b| b.frame);
        if polling && self.rt.assets.game().title == silky_format::game::Title::Nanairo {
            self.rt.cancelled = true;
            return true;
        }
        false
    }

    /// 同級生 Remake: Escape, Delete and keypad 0 close a dialog or a
    /// notice as a cancel, with the cancel sound (the save dialog's
    /// result is a cancel). Returns whether one was up.
    fn douk_cancel_key(&mut self) -> Result<bool> {
        if self.douk_system.is_none() || self.veil.is_some() || self.menu_sliding() {
            return Ok(false);
        }
        if self.dialog.take().is_some() {
            let _ = self
                .rt
                .play_sound(crate::games::doukyuusei::name::CANCEL_SOUND);
            self.frame = None;
            return Ok(true);
        }
        let notice = match &self.ui.panel {
            Some(DoukPanel::Notice(_) | DoukPanel::Unlocked(_)) => true,
            Some(DoukPanel::Name(n)) => n.notice,
            _ => false,
        };
        if notice && self.screen.is_none() {
            return self.back();
        }
        self.room_cancel()
    }

    /// The cancel of the room and the plain area selection (Escape, Delete,
    /// keypad 0, the secondary click): the cancel sound, then what
    /// [`crate::games::doukyuusei::room::Room::cancel`] says. Returns
    /// whether the room was up.
    fn room_cancel(&mut self) -> Result<bool> {
        use crate::games::doukyuusei::Click;
        if self.screen.is_some() || self.dialog.is_some() {
            return Ok(false);
        }
        let Some(DoukPanel::Room(r)) = &self.ui.panel else {
            return Ok(false);
        };
        let click = r.cancel();
        let _ = self
            .rt
            .play_sound(crate::games::doukyuusei::name::CANCEL_SOUND);
        self.frame = None;
        match click {
            Click::Look(on) => {
                self.room_look(on);
                self.sync_cursor();
            }
            Click::Result(result) => self.room_result(result)?,
            _ => {}
        }
        Ok(true)
    }

    /// Keypad 0: a cancel on a dialog or notice of 同級生 Remake, as
    /// Escape and Delete are; nothing else.
    pub fn numpad0(&mut self) -> Result<()> {
        if self.roll_cancel() {
            return Ok(());
        }
        self.douk_cancel_key().map(|_| ())
    }

    /// Enter and keypad Enter. On 同級生 Remake's dialogs and name entry
    /// they activate the focused button (the one the arrow keys or the
    /// pointer are on) and nothing else; otherwise they go on, as
    /// [`Session::proceed`].
    pub fn enter(&mut self) -> Result<()> {
        if self.douk_system.is_some() && self.veil.is_none() && !self.menu_sliding() {
            if self.dialog.is_some() {
                if let Some(Button::Dialog(id)) = self.ui.hover {
                    self.press_dialog(id)?;
                }
                return Ok(());
            }
            if self.screen.is_none() && matches!(self.ui.panel, Some(DoukPanel::Room(_))) {
                // Enter clicks what the focus is on; nothing else.
                match self.ui.hover {
                    Some(b @ (Button::Panel(_) | Button::Bar(_))) if self.ui.enabled(b) => {
                        self.press(b)?
                    }
                    _ => {}
                }
                return Ok(());
            }
            if self.screen.is_none()
                && matches!(
                    self.ui.panel,
                    Some(DoukPanel::Name(_) | DoukPanel::Notice(_))
                )
            {
                if let Some(Button::Panel(id)) = self.ui.hover {
                    self.press(Button::Panel(id))?;
                }
                return Ok(());
            }
        }
        self.proceed()
    }

    /// Arrow keys on 同級生 Remake's dialogs, name entry and notices: the
    /// focus (the hovered item) moves as the program's tables say, with
    /// no sound and no wrapping. Returns whether one of them is up.
    fn focus_arrow(&mut self, key: crate::games::apeiria::keys::Arrow) -> bool {
        use crate::games::doukyuusei::{keys, savenote};
        if self.douk_system.is_none()
            || self.veil.is_some()
            || self.menu_sliding()
            || self.hmode.is_some()
        {
            return false;
        }
        let from = match self.ui.hover {
            Some(Button::Dialog(id) | Button::Panel(id)) => id,
            _ => -1,
        };
        if let Some(d) = &self.dialog {
            let areas = self.screens.dialog_areas(d);
            let toggle = !matches!(d.action, Action::Unlock(_));
            let to = if d.commenting {
                savenote::arrow(from, key)
            } else {
                keys::dialog_arrow(from, key, toggle)
            };
            // A target the area list lacks does nothing.
            let to = to.filter(|t| {
                areas.iter().any(|a| a.id == *t) && (toggle || *t != dialog_ids::DONT_ASK)
            });
            if let Some(t) = to {
                self.ui.hover = Some(Button::Dialog(t));
                self.frame = None;
            }
            return true;
        }
        if self.screen.is_some() {
            return false;
        }
        match &mut self.ui.panel {
            Some(DoukPanel::Name(n)) => {
                if let Some(t) = n.arrow(from, key) {
                    n.set_focus(Some(t));
                    self.ui.hover = Some(Button::Panel(t));
                    self.frame = None;
                }
                true
            }
            Some(DoukPanel::Notice(_)) => {
                if let Some(t) = keys::single_arrow(from, key) {
                    self.ui.hover = Some(Button::Panel(t));
                    self.frame = None;
                }
                true
            }
            _ => false,
        }
    }

    /// Arrow keys on the room and the plain area selection. With the
    /// buttons up they move the focus along the buttons and the menu as
    /// [`crate::games::doukyuusei::room::arrow`] says, setting the hover
    /// where the program warps the pointer, which stays where it is. In
    /// look mode and on the plain selection they move the pointer itself,
    /// 15 pixels (45 with Shift), as the program does.
    fn room_arrow(&mut self, key: crate::games::apeiria::keys::Arrow) -> Option<(i32, i32)> {
        use crate::games::apeiria::keys::Arrow;
        use crate::games::doukyuusei::keys::Target;
        use crate::games::doukyuusei::menu::BUTTONS;
        use crate::games::doukyuusei::room::{self, Focus, Go};
        let Some(DoukPanel::Room(r)) = &self.ui.panel else {
            return None;
        };
        if r.look {
            let step = if self.town_input.fast {
                room::STEP_FAST
            } else {
                room::STEP
            };
            let (dx, dy) = match key {
                Arrow::Up => (0, -step),
                Arrow::Down => (0, step),
                Arrow::Left => (-step, 0),
                Arrow::Right => (step, 0),
            };
            let (w, h) = self.screen_size();
            let x = (self.ui.pointer.0 + dx).clamp(0, w as i32 - 1);
            let y = (self.ui.pointer.1 + dy).clamp(0, h as i32 - 1);
            self.pointer_move(x, y);
            return Some((x, y));
        }
        let set = r.set();
        let focus = match self.ui.hover {
            Some(Button::Panel(id)) if (0..256).contains(&id) => Focus::Button(id as u8),
            Some(Button::Bar(id)) => match BUTTONS.iter().position(|&b| b == id) {
                Some(k) => Focus::Menu(k),
                None => Focus::None,
            },
            _ => Focus::None,
        };
        let menu_point =
            |this: &Self, k: usize| this.layout.douk_key_point(&this.ui, Target::Menu(k as i32));
        let hidden5 = menu_point(self, 5).is_none();
        let to = room::arrow(focus, key, self.ui.side_menu, hidden5);
        let point = match to {
            Some(Go::Button(k)) => room::button_point(set, k),
            Some(Go::Menu(k)) => menu_point(self, k),
            None => None,
        };
        if let Some((x, y)) = point {
            self.ui.hover = self.hit(x, y);
            self.sync_cursor();
            self.frame = None;
        }
        None
    }

    /// Delete: ends the walk on the map, or on the mail forward screen
    /// does what a secondary click does.
    pub fn delete_key(&mut self) -> Result<()> {
        if self.roll_cancel() {
            return Ok(());
        }
        if self.douk_cancel_key()? || self.cancel_walk() {
            return Ok(());
        }
        if self.replaying() {
            return self.ask(self.replay_action());
        }
        if self.mail_select_up() {
            return self.mail_select_secondary();
        }
        Ok(())
    }

    /// Backspace: on 同級生 Remake's room and place screens with their
    /// buttons up, opens the backlog as the menu's backlog button does.
    pub fn backspace_key(&mut self) -> Result<()> {
        let room_up = self.douk_system.is_some()
            && self.screen.is_none()
            && self.dialog.is_none()
            && !self.menu_sliding()
            && matches!(self.ui.panel, Some(DoukPanel::Room(ref r)) if !r.look);
        if room_up {
            self.room_system(room::Sys::Backlog);
            return Ok(());
        }
        // The message window: Backspace brings it back when the player hid
        // it, else opens the backlog (the window fades away first).
        let message = self.douk_system.is_some()
            && self.state == State::Text
            && self.screen.is_none()
            && self.dialog.is_none()
            && !self.menu_sliding()
            && self.ui.panel.is_none();
        if message && self.ui.window_hidden {
            self.ui.window_hidden = false;
            self.base = None;
            self.frame = None;
        } else if message {
            self.room_system(room::Sys::Backlog);
        }
        Ok(())
    }

    /// 景の海のアペイリア and 先輩が私の妄想にドージンする: a scene of the
    /// scene list is being replayed
    /// (variable H 69, set by the scene list) and its text waits, so a
    /// secondary click, Escape or Delete asks about going back to the list.
    fn replaying(&self) -> bool {
        let title = self.rt.assets.game().title;
        (title.is_apeiria() || title == silky_format::game::Title::Doujin)
            && self.state == State::Text
            && self.dialog.is_none()
            && self.screen.is_none()
            && self.vm.var_h(REPLAY_VAR).int() == 1
    }

    /// The question asked before leaving a replay: the doujinshi screen
    /// asks to go back to the title.
    fn replay_action(&self) -> Action {
        if self.rt.assets.game().title == silky_format::game::Title::Doujin {
            Action::ComicReturn
        } else {
            Action::ReplayReturn
        }
    }

    /// Ends the waiting text with result 1, which the scene scripts take
    /// (in variable G 16) as the sign to return to the scene list.
    fn leave_replay(&mut self) -> Result<()> {
        if self.state != State::Text {
            return Ok(());
        }
        if self.vm.pop().is_ok() {
            self.vm.push(1);
        }
        self.shown_chars = None;
        self.ui.window_hidden = false;
        self.ui.waiting = false;
        self.run()
    }

    fn mail_select_up(&self) -> bool {
        self.state == State::Choice
            && self.dialog.is_none()
            && self.screen.is_none()
            && self.rt.scene.mail_select.as_ref().is_some_and(|m| m.shown)
    }

    /// Secondary click on the mail forward screen: hide the message window
    /// (the rows stay), the settings, load, or skip or auto mode for after
    /// the selection. Saving waits for the next message.
    fn mail_select_secondary(&mut self) -> Result<()> {
        self.frame = None;
        if self.ui.window_hidden {
            self.ui.window_hidden = false;
            return Ok(());
        }
        match self.rt.system.settings.right_click {
            RightClick::HideWindow => self.ui.window_hidden = true,
            RightClick::Settings => self.open_settings(),
            RightClick::Skip => self.modes_at_choice.0 ^= true,
            RightClick::Auto => self.modes_at_choice.1 ^= true,
            RightClick::Load => self.open_save_load(Mode::Load),
            RightClick::Save => {}
        }
        Ok(())
    }

    /// Opens the notebook as a system screen over what is up (the message,
    /// or the screen the script waits on). Closing it returns there.
    fn open_k1_notebook(&mut self) {
        use crate::games::kimaten1::notebook::{Notebook, BOOK_FIRST, BOOK_VARS};
        if self.ui.k1_menu.is_some() {
            return;
        }
        let vars = (BOOK_FIRST..BOOK_FIRST + BOOK_VARS)
            .map(|i| self.vm.var_b(i))
            .collect();
        let book = Notebook::new(&self.rt.assets, vars)
            .map(|n| n.from_bar(self.ui.notebook, !self.ui.has_log, !self.ui.has_saves));
        if let Ok(n) = book {
            self.ui.k1_menu = Some(crate::games::kimaten1::Screen::Notebook(n));
            self.ui.k1_under_since = self.ui.k1_since;
            self.ui.k1_since = self.ui.clock;
            self.ui.hover = None;
            self.base = None;
            self.frame = None;
        }
    }

    /// Puts up the screen of きまぐれテンプテーション (or one of the game
    /// modes of なないろリンカネーション) the runtime has ready.
    fn show_k1(&mut self) {
        self.ui.k1 = self.rt.k1.take();
        if let Some(k) = &mut self.ui.k1 {
            k.set_context(self.ui.has_saves, !self.log.is_empty());
            if self.ui.k1_replay {
                k.set_replay();
            }
        }
        self.ui.k1_menu = None;
        self.ui.k1_since = self.ui.clock;
        self.ui.hover = None;
        self.ui.waiting = false;
        self.ui.skip = false;
        self.ui.auto = false;
        self.skip_to_choice = false;
        self.state = State::Choice;
        self.base = None;
    }

    /// Opens one of the shared extras screens (the gallery, the scenes
    /// or the music room).
    fn open_extras(&mut self, kind: crate::host::Extras) {
        self.ui.title = None;
        self.ui.skip = false;
        self.ui.auto = false;
        self.ui.hover = None;
        let mut screen = ExtrasScreen::new(kind);
        screen.ap = crate::games::apeiria::omake::Omake::new(crate::games::apeiria::omake::tables(
            self.rt.assets.game().title,
        ));
        self.screen = Some(Screen::Extras(screen));
        self.state = State::Extras;
        if self.menus.is_apeiria() {
            self.veil = Some(Veil {
                start: self.ui.clock,
                out: false,
                then: None,
            });
            self.apeiria_extras_music(kind);
        }
    }

    /// Carries out what a press on a screen of きまぐれテンプテーション did:
    /// its sound, the bytes it changed, and its answer to the script.
    fn k1_answer(&mut self, click: crate::games::kimaten1::Click) -> Result<()> {
        use crate::games::kimaten1::Click;
        // A system screen over the script's own takes the press; what it
        // closes with is never an answer to the script.
        if let Some(s) = &mut self.ui.k1_menu {
            let is_book = matches!(s, crate::games::kimaten1::Screen::Notebook(_));
            let sound = s.take_sound();
            let voice = s.take_voice();
            for (i, v) in s.take_bytes() {
                self.vm.set_var_b(i, v);
            }
            if let Some(file) = sound {
                let _ = self.rt.play_sound(&file);
            }
            self.k1_voice(voice);
            self.frame = None;
            match click {
                Click::Close | Click::Result(_) => {
                    self.ui.k1_menu = None;
                    self.ui.k1_since = self.ui.k1_under_since;
                    self.ui.hover = None;
                    self.base = None;
                }
                Click::Bar(id) => {
                    // The notebook's own menu buttons leave it first.
                    if is_book {
                        self.ui.k1_menu = None;
                        self.ui.k1_since = self.ui.k1_under_since;
                        self.ui.hover = None;
                        self.base = None;
                    }
                    self.press(crate::ui::Button::Bar(id))?
                }
                Click::None => {}
            }
            return Ok(());
        }
        let Some(s) = &mut self.ui.k1 else {
            return Ok(());
        };
        let sound = s.take_sound();
        let voice = s.take_voice();
        for (i, v) in s.take_bytes() {
            self.vm.set_var_b(i, v);
        }
        for (i, v) in s.take_globals() {
            self.vm.set_g(i, v);
        }
        // The viewer keeps its model in the scene, and gives it back.
        if matches!(click, Click::Result(_) | Click::Close) {
            s.leave_scene(&mut self.rt.scene);
            self.base = None;
        } else if s.sync_scene(&mut self.rt.scene) {
            self.base = None;
        }
        let (music, volume) = (s.take_music(), s.take_volume());
        match music {
            Some(crate::games::kimaten1::omake::Music::Play(file)) => {
                let _ = self.rt.play_music_once(&file);
            }
            Some(crate::games::kimaten1::omake::Music::Stop) => self.rt.stop_music(),
            None => {}
        }
        if let Some(v) = volume {
            let old = self.rt.system.settings.clone();
            self.rt.system.settings.music = v;
            self.settings_changed(old);
        }
        if let Some(file) = sound {
            let _ = self.rt.play_sound(&file);
        }
        self.k1_voice(voice);
        self.frame = None;
        match click {
            Click::None => {}
            Click::Bar(id) => self.press(crate::ui::Button::Bar(id))?,
            Click::Result(r) => {
                // The game's own mode screens hand over to each other inside
                // one call; the script sees only the last answer.
                use crate::games::kimaten1::Screen as K;
                use crate::games::nanairo::mode::{self, Mode, Next};
                let from = match &self.ui.k1 {
                    Some(K::NanairoMusic(_)) => Some(Mode::Music),
                    Some(K::NanairoCg(_)) => Some(Mode::Cg),
                    Some(K::NanairoScene(_)) => Some(Mode::Scene),
                    _ => None,
                };
                let page = match &self.ui.k1 {
                    Some(K::NanairoScene(s)) => s.page(),
                    _ => 0,
                };
                let next = from.map(|m| mode::next(m, r));
                if let (Some(m), Some(n)) = (from, next) {
                    self.rt.nanairo_mode_left(&mut self.vm, m, page, n);
                }
                self.ui.k1 = None;
                self.ui.hover = None;
                self.base = None;
                let mut r = r;
                match next {
                    Some(Next::Go(m)) => {
                        self.rt.nanairo_open_mode(&mut self.vm, m);
                        self.show_k1();
                        return Ok(());
                    }
                    Some(Next::Leave(v)) => r = v,
                    None => {}
                }
                self.rt.answer_action(&mut self.vm, r);
                self.run()?;
            }
            Click::Close => {
                self.ui.k1 = None;
                self.ui.hover = None;
                self.base = None;
                self.rt.close_k1();
                self.run()?;
            }
        }
        Ok(())
    }

    /// Plays or stops the voice line a screen asked for (the notebook's
    /// recording).
    fn k1_voice(&mut self, voice: Option<crate::games::kimaten1::notebook::Voice>) {
        use crate::games::kimaten1::notebook::Voice;
        match voice {
            Some(Voice::Play(file)) => {
                let _ = self.rt.replay_voice(file);
            }
            Some(Voice::Stop) => self.rt.stop_sound_voice(),
            None => {}
        }
    }

    pub fn back(&mut self) -> Result<bool> {
        // H mode ends only with its own finish button.
        if self.hmode.is_some() || self.veil.is_some() || self.menu_sliding() {
            return Ok(true);
        }
        if self.screen.is_none() && self.dialog.is_none() {
            let ms = self.ui.clock.saturating_sub(self.ui.k1_since);
            if let Some(s) = self.ui.k1_menu.as_mut().or(self.ui.k1.as_mut()) {
                // A fade in or out takes no input.
                if s.blocks_input(ms) {
                    return Ok(true);
                }
                let click = s.cancel();
                self.k1_answer(click)?;
                return Ok(true);
            }
        }
        if self.dialog.take().is_some() {
            self.frame = None;
            return Ok(true);
        }
        if let Some(Screen::DoukFlag(s)) = &mut self.screen {
            use crate::games::doukyuusei::flaglist::Click as FlagClick;
            if s.cancel() == FlagClick::Leave {
                self.close_screen();
            }
            self.ui.hover = None;
            self.frame = None;
            return Ok(true);
        }
        // The notice closes on Escape and the secondary click with the
        // cancel sound.
        if matches!(self.ui.panel, Some(DoukPanel::Notice(_))) {
            let _ = self
                .rt
                .play_sound(crate::games::doukyuusei::name::CANCEL_SOUND);
            self.close_notice()?;
            return Ok(true);
        }
        // The "comment unlocked" pop-up closes with the cancel sound.
        if matches!(self.ui.panel, Some(DoukPanel::Unlocked(_))) {
            let _ = self
                .rt
                .play_sound(crate::games::doukyuusei::name::CANCEL_SOUND);
            self.close_notice()?;
            return Ok(true);
        }
        // The map jump goes back to the walk, the menu as it was, with the
        // cancel sound.
        if matches!(self.ui.panel, Some(DoukPanel::Jump(_))) {
            let _ = self
                .rt
                .play_sound(crate::games::doukyuusei::name::CANCEL_SOUND);
            self.ui.panel = Some(DoukPanel::Town);
            self.ui.hover = None;
            self.frame = None;
            return Ok(true);
        }
        // The room's menu stays open beside it; the cancel is the room's.
        if self.room_cancel()? {
            return Ok(true);
        }
        if std::mem::take(&mut self.ui.side_menu) {
            self.frame = None;
            return Ok(true);
        }
        if self.state == State::Movie {
            self.end_movie()?;
            return Ok(true);
        }
        // 同級生 Remake's character picker leaves on a secondary click; the
        // album goes from a girl's pictures back to the girls, else leaves.
        if self.screen.is_none() && self.dialog.is_none() {
            use crate::games::doukyuusei::Panel;
            let cancel = match &mut self.ui.panel {
                Some(Panel::Picker(p)) => p.cancel(),
                Some(Panel::Album(a)) => {
                    let leave = a.cancel();
                    if leave.is_none() {
                        self.ui.hover = None;
                        self.frame = None;
                        return Ok(true);
                    }
                    leave
                }
                Some(Panel::Reel(r)) => {
                    let leave = r.cancel();
                    if leave.is_none() {
                        self.ui.hover = None;
                        self.frame = None;
                        return Ok(true);
                    }
                    leave
                }
                Some(Panel::Comment(c)) => c.cancel(),
                // The name entry has no way out; only its notice closes.
                Some(Panel::Name(n)) => {
                    if n.secondary() {
                        let _ = self
                            .rt
                            .play_sound(crate::games::doukyuusei::name::CANCEL_SOUND);
                    }
                    self.frame = None;
                    return Ok(true);
                }
                Some(Panel::Music(m)) => {
                    let leave = m.cancel();
                    self.rt.stop_music();
                    if leave.is_none() {
                        self.ui.hover = None;
                        self.frame = None;
                        return Ok(true);
                    }
                    leave
                }
                _ => None,
            };
            if let Some(result) = cancel {
                self.ui.panel = None;
                self.base = None;
                self.rt.answer_action(&mut self.vm, result);
                self.run()?;
                return Ok(true);
            }
        }
        match &mut self.screen {
            Some(Screen::Extras(e)) if e.view.is_some() => {
                e.view = None;
                self.frame = None;
            }
            Some(Screen::Extras(e)) if e.k2.view.is_some() => {
                let v = e.k2.view.as_mut().expect("checked");
                if v.picture.take().is_none()
                    || crate::games::kimaten2::omake::CG_ENTRIES[v.entry].1 == 1
                {
                    e.k2.view = None;
                }
                self.ui.hover = None;
                self.frame = None;
            }
            Some(Screen::Extras(e)) if e.ap.view.is_some() => {
                let v = e.ap.view.as_mut().expect("checked");
                match v.picture.take() {
                    Some(p) => v.page = p / 12,
                    None => {
                        e.page = v.entry / 12;
                        e.ap.view = None;
                    }
                }
                self.ui.hover = None;
                self.frame = None;
            }
            Some(Screen::Extras(e)) if self.menus.is_apeiria() && e.playing.is_some() => {
                e.playing = None;
                self.rt.stop_music();
                self.frame = None;
            }
            Some(Screen::Extras(_)) => self.leave_extras(-1)?,
            // From アンネの部屋 a secondary click is もどる.
            Some(Screen::Costume(c)) if c.script => {
                self.press_archive(archive::costume_ids::BACK, 0, 0)?
            }
            Some(_) => self.close_screen(),
            None => return Ok(false),
        }
        Ok(true)
    }

    fn end_movie(&mut self) -> Result<()> {
        if self.movie.take().is_some() {
            self.rt.answer_movie(&mut self.vm, true);
            self.state = State::Busy;
            self.run()?;
        }
        Ok(())
    }

    /// Leaves an extras screen with `result` for the script: -1 back to
    /// the title, or a scene to replay.
    fn leave_extras(&mut self, result: i32) -> Result<()> {
        if self.menus.is_apeiria() {
            match self.veil {
                Some(Veil { out: true, .. }) => return Ok(()),
                _ => {
                    self.veil = Some(Veil {
                        start: self.ui.clock,
                        out: true,
                        then: Some(result),
                    });
                    self.frame = None;
                    return Ok(());
                }
            }
        }
        self.close_extras(result)
    }

    fn close_extras(&mut self, result: i32) -> Result<()> {
        self.veil = None;
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
        if self.douk_system.is_some() {
            self.open_douk_settings(crate::games::doukyuusei::config::Page::System1);
            return;
        }
        self.screen = Some(Screen::Settings(SettingsScreen {
            opened: self.ui.clock,
            ..SettingsScreen::new()
        }));
        self.ui.skip = false;
        self.ui.hover = None;
        self.frame = None;
    }

    /// Mouse wheel: `up` towards older text. Up on a message opens the
    /// backlog; down at its newest entry closes it, and on a message of
    /// きまぐれテンプテーション2 goes on like a click.
    /// An arrow key. On 景の海のアペイリア's extras it moves the pointer to
    /// another button, as the original does; the new place is returned for
    /// the host to move its own pointer there.
    pub fn arrow(&mut self, key: crate::games::apeiria::keys::Arrow) -> Result<Option<(i32, i32)>> {
        use crate::games::apeiria::keys::{self, Move};
        use crate::games::apeiria::omake;
        if let Some(to) = self.k1_arrow(key) {
            return Ok(to);
        }
        if matches!(self.ui.panel, Some(DoukPanel::Room(_)))
            && self.dialog.is_none()
            && self.screen.is_none()
            && self.veil.is_none()
            && !self.menu_sliding()
            && self.hmode.is_none()
        {
            return Ok(self.room_arrow(key));
        }
        if self.focus_arrow(key) {
            return Ok(None);
        }
        if let Some(r) = self.douk_arrow(key) {
            return Ok(r);
        }
        if !self.menus.is_apeiria() || self.dialog.is_some() || self.veil.is_some() {
            return Ok(None);
        }
        let Some(Screen::Extras(e)) = &self.screen else {
            return Ok(None);
        };
        let at = match self.ui.hover {
            Some(Button::Screen(id)) if e.ap.view.is_some() => keys::to_view(id),
            Some(Button::Screen(id)) => id,
            _ => -1,
        };
        let bytes = &self.vm.bytes;
        let on = |i: i32| {
            i >= 0
                && match &e.ap.view {
                    Some(v) => omake::view_on(e.ap.t, v, bytes, i as usize),
                    None => omake::item(e.ap.t, e.kind, e.page, e.scroll, i as usize)
                        .is_some_and(|it| omake::open(e.ap.t, e.kind, bytes, it)),
                }
        };
        match keys::arrow(e.kind, &e.ap, at, key, &on) {
            Move::To(id) => {
                let areas = self.menus.extras_areas(e.kind);
                if let Some((x, y)) = keys::centre(areas, &e.ap, id) {
                    self.pointer_move(x, y);
                    return Ok(Some((x, y)));
                }
            }
            Move::ScrollDown => self.wheel(false)?,
            Move::Picture { next } => self.wheel(!next)?,
            Move::Stay => {}
        }
        Ok(None)
    }

    /// Arrow keys on the notebook of きまぐれテンプテーション: `Some` when
    /// one is up (the pointer moves to the element the key leads to, if
    /// there is one).
    fn k1_arrow(&mut self, key: crate::games::apeiria::keys::Arrow) -> Option<Option<(i32, i32)>> {
        if self.dialog.is_some() || self.screen.is_some() || self.veil.is_some() {
            return None;
        }
        let screen = self.ui.k1_menu.as_ref().or(self.ui.k1.as_ref())?;
        let book = screen.notebook()?;
        // A fade in or out takes no key either.
        if screen.blocks_input(self.ui.clock.saturating_sub(self.ui.k1_since)) {
            return Some(None);
        }
        let at = match self.ui.hover {
            Some(Button::Kimaten1(id)) => Some(id),
            _ => None,
        };
        let to = book.arrow(at, key).and_then(|id| book.centre(id));
        if let Some((x, y)) = to {
            self.pointer_move(x, y);
        }
        Some(to)
    }

    /// Arrow keys on a 同級生 Remake selection: `Some` when one is up (the
    /// pointer moves to the target, if there is one).
    fn douk_arrow(
        &mut self,
        key: crate::games::apeiria::keys::Arrow,
    ) -> Option<Option<(i32, i32)>> {
        use crate::games::doukyuusei::keys;
        if self.dialog.is_some()
            || self.screen.is_some()
            || self.veil.is_some()
            || self.menu_sliding()
            || self.hmode.is_some()
        {
            return None;
        }
        let state = self.layout.douk_key_state(&self.ui)?;
        let to = keys::arrow(&state, key).and_then(|t| self.layout.douk_key_point(&self.ui, t));
        if let Some((x, y)) = to {
            self.pointer_move(x, y);
        }
        Some(to)
    }

    pub fn wheel(&mut self, up: bool) -> Result<()> {
        // Input waits for the extras' fade.
        if self.veil.is_some() || self.menu_sliding() {
            return Ok(());
        }
        if self.dialog.is_some() {
            return Ok(());
        }
        let ms = self.ui.clock.saturating_sub(self.ui.k1_since);
        if let Some(s) = self.ui.k1_menu.as_mut().or(self.ui.k1.as_mut()) {
            if s.blocks_input(ms) {
                return Ok(());
            }
            if s.wheel(up) {
                return self.k1_answer(crate::games::kimaten1::Click::None);
            }
            // The notebook takes the wheel whether or not it scrolls: the
            // message behind it gets none.
            if s.notebook().is_some() || self.ui.k1_menu.is_some() {
                return Ok(());
            }
        }
        let d = if up { -1 } else { 1 };
        match &mut self.screen {
            Some(Screen::SaveLoad(s)) => s.scroll_by(d),
            Some(Screen::Settings(s)) => {
                let max = self.menus.max_scroll(&mut self.rt.assets, s.page);
                s.scroll = (s.scroll + d as i32 * menus::SCROLL_STEP).clamp(0, max);
            }
            Some(Screen::Archive(s)) => {
                let max = self.archive.max_scroll(&mut self.rt.assets, s);
                Archive::wheel(s, up, max);
            }
            Some(Screen::Costume(_) | Screen::DoukSave(_) | Screen::DoukConfig(_)) => return Ok(()),
            Some(Screen::DoukFlag(s)) => {
                s.wheel(up);
            }
            Some(Screen::Extras(e)) if self.menus.is_apeiria() => {
                use crate::games::apeiria::omake;
                let t = e.ap.t;
                match (&mut e.ap.view, e.kind) {
                    (Some(v), _) => match v.picture {
                        Some(p) => {
                            let to = if up {
                                omake::previous_seen(t, &self.vm.bytes, v.entry, p)
                            } else {
                                omake::next_seen(t, &self.vm.bytes, v.entry, p)
                            };
                            if let Some((entry, p)) = to {
                                v.entry = entry;
                                v.picture = Some(p);
                            }
                        }
                        None => {
                            let max = omake::view_pages(t, v.entry) as isize - 1;
                            v.page = (v.page as isize + d).clamp(0, max) as usize;
                        }
                    },
                    (None, Extras::Music) => {
                        e.scroll =
                            (e.scroll as isize + d).clamp(0, t.max_scroll() as isize) as usize;
                    }
                    (None, kind) => {
                        let pages = if kind == Extras::Gallery {
                            t.cg.pages
                        } else {
                            t.scene.pages
                        };
                        e.page = (e.page as isize + d).clamp(0, pages as isize - 1) as usize;
                        e.ap.preview = None;
                    }
                }
            }
            Some(Screen::Extras(e)) if self.menus.is_kimaten2() => {
                use crate::games::kimaten2::omake;
                match (&mut e.k2.view, e.kind) {
                    (Some(v), _) if v.picture.is_none() => {
                        let max = omake::view_pages(v.entry) as isize - 1;
                        v.page = (v.page as isize + d).clamp(0, max) as usize;
                    }
                    (Some(_), _) | (None, Extras::Scenes) => return Ok(()),
                    (None, Extras::Music) => {
                        e.scroll =
                            (e.scroll as isize + d).clamp(0, omake::MAX_SCROLL as isize) as usize;
                    }
                    (None, Extras::Gallery) => {
                        e.page =
                            (e.page as isize + d).clamp(0, omake::CG_PAGES as isize - 1) as usize;
                    }
                }
            }
            Some(Screen::Extras(e)) => match e.kind {
                Extras::Music => {
                    e.scroll = (e.scroll as isize + d).clamp(0, Menus::music_max_scroll() as isize)
                        as usize;
                }
                _ if e.view.is_none() => {
                    e.page = (e.page as isize + d).clamp(0, menus::CG_PAGES as isize - 1) as usize;
                }
                _ => return Ok(()),
            },
            Some(Screen::Backlog(b)) => {
                if !up && b.top >= Backlog::max_top(self.log.len()) {
                    self.close_screen();
                } else {
                    b.scroll_by(d, self.log.len());
                }
            }
            None if up && self.state == State::Text => {
                self.room_system(room::Sys::Backlog);
                return Ok(());
            }
            // きまぐれテンプテーション2: down goes on like a click.
            None if self.layout.is_kimaten2() && self.state == State::Text => {
                return self.proceed();
            }
            None => return Ok(()),
        }
        self.frame = None;
        Ok(())
    }

    /// 同級生 Remake: whether the game can be saved now, and whether the
    /// backlog has entries.
    fn douk_can(&self) -> (bool, bool) {
        let at_panel = self.douk_system.is_some() && self.state == State::Choice;
        let can_save = (self.state == State::Text || at_panel) && self.saves.is_some();
        (can_save, !self.log.is_empty())
    }

    /// Opens 同級生 Remake's settings at `page`.
    fn open_douk_settings(&mut self, page: crate::games::doukyuusei::config::Page) {
        use crate::games::doukyuusei::config::{ConfigScreen, Page};
        use crate::games::doukyuusei::menu::Next;
        if self.open_after_fade(Next::Settings {
            sound: page == Page::Sound,
        }) {
            return;
        }
        self.screen = Some(Screen::DoukConfig(ConfigScreen::new(page, self.ui.clock)));
        self.fade_screen_in();
        self.ui.skip = false;
        self.sync_douk_speeds();
        self.ui.hover = None;
        self.frame = None;
    }

    /// A click on 同級生 Remake's settings.
    fn press_douk_config(&mut self, id: i32, x: i32) -> Result<()> {
        use crate::games::doukyuusei::config::{self, ids, Page};
        use crate::games::doukyuusei::sound;
        use crate::games::doukyuusei::system::{side, System};
        let (can_save, has_log) = self.douk_can();
        let (Some(Screen::DoukConfig(s)), Some(sys)) = (&mut self.screen, &self.douk_system) else {
            return Ok(());
        };
        if !System::config_enabled(s, &self.rt.system.settings, id, can_save, has_log) {
            return Ok(());
        }
        if s.page == Page::Sound && id < ids::SIDE {
            let voices = s.voices;
            match id {
                sound::ids::PREVIOUS => s.voices = voices.saturating_sub(1),
                sound::ids::NEXT => s.voices = (voices + 1).min(sound::PAGES - 1),
                _ if (sound::ids::TAB..sound::ids::TAB + sound::PAGES as i32).contains(&id) => {
                    s.voices = (id - sound::ids::TAB) as usize
                }
                sound::ids::RESET => {
                    let old = self.rt.system.settings.clone();
                    let defaults = Settings::for_title(self.rt.assets.game().title);
                    sound::reset(&mut self.rt.system.settings, &defaults);
                    self.settings_changed(old);
                }
                _ => {
                    let value = sys
                        .config_area(s.page, id)
                        .map_or(0.0, |a| sound::slider_value(a, x));
                    let old = self.rt.system.settings.clone();
                    if sound::apply(&mut self.rt.system.settings, voices, id, value) {
                        self.settings_changed(old);
                    }
                }
            }
            self.frame = None;
            return Ok(());
        }
        match id {
            _ if id >= ids::SIDE => match (id - ids::SIDE) as usize {
                side::SETTINGS => s.page = Page::System1,
                side::SOUND => s.page = Page::Sound,
                side::SAVE | side::LOAD => {
                    let mode = if id - ids::SIDE == side::SAVE as i32 {
                        Mode::Save
                    } else {
                        Mode::Load
                    };
                    self.close_screen();
                    self.open_save_load(mode);
                }
                side::LOG => {
                    self.close_screen();
                    self.open_backlog();
                }
                side::TITLE => self.ask(Action::Title)?,
                side::QUIT => self.ask(Action::Quit)?,
                side::BACK => self.close_screen(),
                _ => {}
            },
            ids::PREVIOUS | ids::TAB => s.page = Page::System1,
            ids::NEXT => s.page = Page::System2,
            _ if id == ids::TAB + 1 => s.page = Page::System2,
            ids::RESET => {
                let old = self.rt.system.settings.clone();
                let defaults = Settings::for_title(self.rt.assets.game().title);
                config::reset(&mut self.rt.system.settings, &defaults);
                self.settings_changed(old);
            }
            ids::SKIP => {
                self.ui.skip = !self.ui.skip;
                self.ui.auto = false;
            }
            ids::AUTO => {
                self.ui.auto = !self.ui.auto;
                self.ui.skip = false;
                self.shown_ms = 0;
            }
            _ => {
                let value = sys
                    .config_area(s.page, id)
                    .map_or(0.0, |a| config::slider_value(a, id, x));
                let old = self.rt.system.settings.clone();
                if config::apply(&mut self.rt.system.settings, id, value) {
                    self.settings_changed(old);
                }
            }
        }
        self.sync_douk_speeds();
        self.frame = None;
        Ok(())
    }

    /// Plays the sample of a slider of 同級生 Remake's sound page once it
    /// is let go.
    fn douk_sound_sample(&mut self, voices: usize, id: i32) {
        use crate::games::doukyuusei::sound;
        let Some(file) = sound::sample(voices, id) else {
            return;
        };
        let played = match id {
            sound::ids::EFFECTS => self.rt.play_sound(file),
            _ => self.rt.replay_voice(file),
        };
        if let Err(e) = played {
            eprintln!("sound sample {file}: {e}");
        }
    }

    /// The speed sliders of 同級生 Remake's settings follow their switches.
    fn sync_douk_speeds(&mut self) {
        let speeds = [
            self.rt.system.settings.typewriter,
            self.ui.skip,
            self.ui.auto,
        ];
        if let Some(Screen::DoukConfig(s)) = &mut self.screen {
            s.speeds = speeds;
        }
    }

    fn open_save_load(&mut self, mode: Mode) {
        if self.douk_system.is_some()
            && self.open_after_fade(crate::games::doukyuusei::menu::Next::Save {
                load: mode == Mode::Load,
            })
        {
            return;
        }
        // 同級生 Remake saves at its room and map screens too.
        let at_panel = self.douk_system.is_some() && self.state == State::Choice;
        let can_save = (self.state == State::Text || at_panel) && self.saves.is_some();
        if mode == Mode::Save && !can_save {
            return;
        }
        self.thumb = if can_save {
            self.scene_thumb().ok()
        } else {
            None
        };
        if self.douk_system.is_some() {
            use crate::games::doukyuusei::system::SaveScreen;
            let mut s = SaveScreen::new(mode == Mode::Load, can_save, self.saves.as_ref());
            s.log = !self.log.is_empty();
            self.screen = Some(Screen::DoukSave(s));
            self.fade_screen_in();
            self.ui.auto = false;
            self.ui.skip = false;
            self.ui.hover = None;
            self.frame = None;
            return;
        }
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

    /// A click on 同級生 Remake's save screen.
    fn press_douk_save(&mut self, id: i32) -> Result<()> {
        use crate::games::doukyuusei::system::{ids, side, slot_name, SaveScreen};
        let Some(Screen::DoukSave(s)) = &mut self.screen else {
            return Ok(());
        };
        if !s.enabled(id) {
            return Ok(());
        }
        let mut ask = None;
        let mut refresh = false;
        match id {
            _ if id >= ids::SIDE => match (id - ids::SIDE) as usize {
                side::SAVE | side::LOAD => {
                    let load = id - ids::SIDE == side::LOAD as i32;
                    if load || s.can_save {
                        s.load = load;
                        s.page = s
                            .page
                            .min(crate::games::doukyuusei::system::MANUAL_PAGES - 1);
                        s.deleting = false;
                        refresh = true;
                    }
                }
                side::LOG => {
                    self.open_backlog();
                    return Ok(());
                }
                side::SETTINGS | side::SOUND => {
                    use crate::games::doukyuusei::config::Page;
                    let page = match (id - ids::SIDE) as usize {
                        side::SOUND => Page::Sound,
                        _ => Page::System1,
                    };
                    self.close_screen();
                    self.open_douk_settings(page);
                    return Ok(());
                }
                side::TITLE => ask = Some(Action::Title),
                side::QUIT => ask = Some(Action::Quit),
                side::BACK => {
                    self.close_screen();
                    return Ok(());
                }
                _ => {}
            },
            _ if (ids::CARD..ids::CARD + 10).contains(&id) => {
                let i = (id - ids::CARD) as usize;
                let Some(slot) = slot_name(s.page, i) else {
                    return Ok(());
                };
                ask = if s.deleting {
                    Some(Action::Delete(slot))
                } else if s.load {
                    Some(Action::Load(slot))
                } else {
                    Some(Action::Save(Some(slot)))
                };
            }
            ids::PREVIOUS => {
                s.turn(-1);
                refresh = true;
            }
            ids::NEXT => {
                s.turn(1);
                refresh = true;
            }
            ids::QUICK if s.deleting => ask = Some(Action::Delete(save::QUICK.to_string())),
            ids::QUICK if s.load => ask = Some(Action::Load(save::QUICK.to_string())),
            ids::QUICK => {
                self.save(save::QUICK)?;
                self.refresh_screen();
                return Ok(());
            }
            ids::AUTO => {
                if let Some(c) = &s.auto {
                    ask = Some(Action::Load(c.slot.clone()));
                }
            }
            ids::DELETE => s.deleting = !s.deleting,
            ids::DELETE_ALL => ask = Some(Action::DeleteAll),
            _ => {
                if let Some(p) = SaveScreen::page_of(id) {
                    if s.page_open(p) {
                        s.page = p;
                        refresh = true;
                    }
                }
            }
        }
        if refresh {
            self.refresh_screen();
        }
        if let Some(action) = ask {
            self.ask(action)?;
        }
        Ok(())
    }

    fn open_backlog(&mut self) {
        if self.douk_system.is_some()
            && self.open_after_fade(crate::games::doukyuusei::menu::Next::Backlog)
        {
            return;
        }
        // 同級生 Remake opens it at its room and map screens too.
        let at_panel = self.douk_system.is_some() && self.state == State::Choice;
        if self.log.is_empty() || !(self.state == State::Text || at_panel) {
            return;
        }
        for e in &mut self.log {
            e.marked = false;
        }
        let pad = match self.douk_system {
            Some(_) => LOG_ROWS.saturating_sub(self.log.len()),
            None => 0,
        };
        self.screen = Some(Screen::Backlog(Backlog {
            top: Backlog::max_top(self.log.len()),
            pad,
        }));
        self.ui.auto = false;
        self.ui.skip = false;
        self.ui.hover = None;
        self.frame = None;
    }

    /// きまぐれテンプテーション2's costume screen (from the button bar).
    fn open_costume(&mut self) {
        if self.state != State::Text {
            return;
        }
        let vm = &self.vm;
        self.screen = Some(Screen::Costume(CostumeScreen::new(|i| vm.var_b(i))));
        self.ui.hover = None;
        self.frame = None;
    }

    /// The notebook or collection; from the title menu it shows what is
    /// kept outside a game.
    /// きまぐれテンプテーション: the 捜査手帳 button of the extras page.
    /// HOOK for the notebook screen, which is built apart from the title
    /// menu: open it here.
    fn kimaten1_notebook_hook(&mut self) {
        use crate::games::kimaten1::notebook::{Notebook, BOOK_FIRST, EXTRAS_BASE, EXTRAS_VARS};
        // The title menu's copy keeps its bytes 1000 higher; closing it
        // leaves the extras page as it was.
        let vars = (BOOK_FIRST..BOOK_FIRST + EXTRAS_VARS)
            .map(|i| self.vm.var_b(EXTRAS_BASE + i))
            .collect();
        if let Ok(n) = Notebook::new_extras(&self.rt.assets, vars) {
            self.ui.k1_menu = Some(crate::games::kimaten1::Screen::Notebook(n));
            self.ui.k1_since = self.ui.clock;
            self.ui.hover = None;
            self.base = None;
            self.frame = None;
        }
    }

    fn open_archive(&mut self, book: Book) {
        let global = self.state == State::Title;
        let vm = &self.vm;
        self.screen = Some(Screen::Archive(ArchiveScreen::new(
            book,
            |i| vm.var_b(i),
            global,
        )));
        self.ui.hover = None;
        self.frame = None;
    }

    fn press_archive(&mut self, id: i32, x: i32, y: i32) -> Result<()> {
        let action = match &mut self.screen {
            Some(Screen::Archive(s)) => {
                let max = self.archive.max_scroll(&mut self.rt.assets, s);
                if id == archive::ids::TEXT_SCROLL {
                    if let Some(f) = self.archive.track_fraction(s, x, y) {
                        s.scroll = (f * max as f32).round() as i32;
                    }
                    ArchiveAction::None
                } else {
                    self.archive.press(s, id, max)
                }
            }
            Some(Screen::Costume(c)) => Archive::press_costume(c, id),
            _ => ArchiveAction::None,
        };
        match action {
            ArchiveAction::Close => {
                let script = match &self.screen {
                    Some(Screen::Costume(c)) if c.script => Some(c.script_result()),
                    _ => None,
                };
                self.close_screen();
                if let Some(result) = script {
                    self.rt.answer_action(&mut self.vm, result);
                    self.state = State::Busy;
                    self.run()?;
                }
            }
            ArchiveAction::Seen(i) => self.vm.set_var_b(i, 1),
            ArchiveAction::Recording => {
                let Some(Screen::Archive(s)) = &mut self.screen else {
                    return Ok(());
                };
                if s.recording.take().is_some() {
                    self.rt.stop_sound_voice();
                } else {
                    s.recording = Some(0);
                    self.play_recording(0);
                }
                self.frame = None;
            }
            ArchiveAction::None => {}
        }
        Ok(())
    }

    /// Plays a part of the voice recording. The executable names
    /// `kt_*.ogg`, which the game data seen so far does not have: a missing
    /// part is reported and skipped.
    fn play_recording(&mut self, part: usize) {
        let file = archive::RECORDING[part];
        if let Err(e) = self.rt.replay_voice(file) {
            self.rt.note_unhandled(format!("recording voice: {e}"));
        }
    }

    /// Plays the voice recording on: the next part when one ends.
    /// Starts H mode (`Game.49`) over the scene.
    fn open_hmode(&mut self) {
        let assets = &self.rt.assets;
        let areas = crate::screens::area_file(assets, "hmode.area");
        let carea = assets.data("anne_h.carea").unwrap_or_default();
        let vm = &self.vm;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        self.hmode = Some(crate::games::kimaten2::hmode::HMode::new(
            areas,
            carea,
            |i| vm.var_b(i),
            seed,
        ));
        self.ui.waiting = false;
        self.ui.skip = false;
        self.ui.auto = false;
        self.skip_to_choice = false;
        self.state = State::Choice;
        self.base = None;
        self.frame = None;
    }

    fn tick_hmode(&mut self, ms: u64) -> Result<()> {
        let voice = self.rt.voice_playing();
        #[cfg(feature = "emote")]
        // Without a model yet (not drawn), nothing is started on it.
        let main = match self.hmode_slot() {
            Some(s) => self
                .renderer
                .emote_stage(s)
                .is_none_or(|st| st.character().main_playing()),
            None => true,
        };
        #[cfg(not(feature = "emote"))]
        let main = true;
        let mut ops = Vec::new();
        let Some(h) = &mut self.hmode else {
            return Ok(());
        };
        h.tick(ms, voice, main, &mut ops);
        let done = h.done;
        self.apply_hmode(ops);
        self.frame = None;
        if done {
            self.hmode = None;
            self.base = None;
            self.state = State::Busy;
            self.run()?;
        }
        Ok(())
    }

    /// The standing slot that holds the H mode model (the first one).
    fn hmode_slot(&self) -> Option<i32> {
        self.rt.scene.emotes.keys().next().copied()
    }

    fn apply_hmode(&mut self, ops: Vec<crate::games::kimaten2::hmode::Op>) {
        use crate::games::kimaten2::hmode::Op;
        for op in ops {
            match op {
                Op::Voice(name) => {
                    if let Err(e) = self.rt.replay_voice(&format!("{name}.ogg")) {
                        self.rt.note_unhandled(format!("H mode voice {name}: {e}"));
                    }
                }
                Op::Sound(name) => {
                    if let Err(e) = self.rt.play_sound(name) {
                        self.rt.note_unhandled(format!("H mode sound {name}: {e}"));
                    }
                }
                #[cfg(feature = "emote")]
                op => {
                    let Some(slot) = self.hmode_slot() else {
                        continue;
                    };
                    let Some(stage) = self.renderer.emote_stage(slot) else {
                        continue;
                    };
                    let c = stage.character();
                    match op {
                        Op::Main { index, file, cut } => c.h_main(index, file, cut),
                        Op::StopMain => c.h_stop_main(),
                        Op::Diff { index, file } => c.h_diff(index, file),
                        Op::StopDiff(i) => c.h_stop_diff(i),
                        Op::Pass => c.h_pass(),
                        Op::Face(f) => c.h_face(f),
                        Op::Voice(_) | Op::Sound(_) => {}
                    }
                    self.base = None;
                }
                #[cfg(not(feature = "emote"))]
                _ => {}
            }
        }
        self.frame = None;
    }

    fn advance_recording(&mut self) {
        let Some(Screen::Archive(s)) = &mut self.screen else {
            return;
        };
        let Some(part) = s.recording else {
            return;
        };
        if !s.recording_shown() {
            s.recording = None;
            self.rt.stop_sound_voice();
            self.frame = None;
        } else if !self.rt.voice_playing() {
            if part + 1 < archive::RECORDING.len() {
                s.recording = Some(part + 1);
                self.play_recording(part + 1);
            } else {
                s.recording = None;
                self.frame = None;
            }
        }
    }

    /// Copies changed global byte variables into the system data (the
    /// layout's ranges one after another).
    fn store_global_bytes(&mut self) {
        let ranges = self.layout.global_bytes();
        if ranges.is_empty() {
            return;
        }
        let len = self.vm.bytes.len();
        let mut all: Vec<u8> = Vec::new();
        for r in ranges {
            // An open range ends with the VM's bytes; a closed one keeps
            // its length so the next one keeps its place.
            let end = if r.end == usize::MAX { len } else { r.end };
            all.extend((r.start..end).map(|i| self.vm.bytes.get(i).copied().unwrap_or(0)));
        }
        while all.last() == Some(&0) {
            all.pop();
        }
        if all != self.rt.system.global_bytes {
            self.rt.system.global_bytes = all;
            self.rt.system_changed = true;
        }
    }

    /// Puts the kept global byte variables into the VM (after a boot or a
    /// load, which bring their own copies).
    fn apply_global_bytes(&mut self) {
        let ranges = self.layout.global_bytes();
        let kept = self.rt.system.global_bytes.clone();
        let mut at = 0;
        for r in ranges {
            let end = if r.end == usize::MAX {
                self.vm
                    .bytes
                    .len()
                    .max(r.start + kept.len().saturating_sub(at))
            } else {
                r.end
            };
            for i in r.start..end {
                let v = kept.get(at).copied().unwrap_or(0);
                at += 1;
                if self.vm.var_b(i) != v {
                    self.vm.set_var_b(i, v);
                }
            }
        }
    }

    /// Whether some notebook entry and some collection item are open, for
    /// the buttons that lead there.
    fn refresh_archive_flags(&mut self) {
        let any = |base: usize, n: usize| (base..base + n).any(|i| self.vm.var_b(i) != 0);
        let notebook = any(archive::NOTEBOOK_VAR, archive::NOTEBOOK_ENTRIES);
        let collection = any(archive::ITEM_VAR, archive::ITEMS);
        // The costume button is grey until the scripts set byte 700.
        let costume = self.vm.var_b(archive::COSTUME_ON) != 0;
        if (notebook, collection, costume)
            != (self.ui.notebook, self.ui.collection, self.ui.costume)
        {
            self.ui.notebook = notebook;
            self.ui.collection = collection;
            self.ui.costume = costume;
            self.frame = None;
        }
    }

    /// きまぐれテンプテーション2's bar: up while pinned or while the
    /// pointer is low enough, else it slides away (see [`crate::ui::BAR_DROP`]).
    fn slide_bar(&mut self, before: u64) {
        use crate::ui::{BAR_DROP, BAR_SHOW_Y, BAR_STEP_MS};
        self.ui.menu_lock = self.rt.system.settings.menu_lock;
        let target = if self.ui.menu_lock || self.ui.pointer.1 >= BAR_SHOW_Y {
            0
        } else {
            BAR_DROP
        };
        let steps = (self.ui.clock / BAR_STEP_MS - before / BAR_STEP_MS) as i32;
        let d = self.ui.bar_drop;
        let to = if target > d {
            (d + 2 * steps).min(target)
        } else {
            (d - 2 * steps).max(target)
        };
        if to != d {
            self.ui.bar_drop = to;
            self.frame = None;
        }
    }

    /// The menu button: 同級生 Remake slides the six buttons out or back.
    fn toggle_side_menu(&mut self) {
        use crate::games::doukyuusei::menu::{slide_passes, MenuAnim};
        let opening = !self.ui.side_menu;
        if self.rt.assets.game().title != silky_format::game::Title::Doukyuusei {
            self.ui.side_menu = opening;
            return;
        }
        let n = slide_passes(self.rt.system.settings.effect_scale());
        self.ui.side_menu = true;
        self.ui.menu_anim = Some(MenuAnim {
            opening,
            start: self.ui.clock,
            n,
        });
        self.frame = None;
    }

    /// Whether the side buttons or the room's buttons are sliding (the
    /// program takes no input).
    fn menu_sliding(&self) -> bool {
        self.ui.menu_anim.is_some()
            || self.ui.room_fx.is_some()
            || self.ui.screen_fx.is_some()
            || self.ui.win_fx.is_some()
    }

    /// Starts a slide of the room (see [`crate::games::doukyuusei::room::Fx`]).
    /// The hint bar keeps the line it shows while it slides away; `then` is
    /// what the slide hands the script when it ends the room.
    fn room_slide(&mut self, step: room::Step, then: Option<i32>) {
        let mut fx = room::Fx::new(step, self.ui.clock, self.rt.system.settings.effect_scale());
        fx.then = then;
        self.ui.room_away = false;
        if let Some(DoukPanel::Room(r)) = &self.ui.panel {
            let hover = match self.ui.hover {
                Some(Button::Panel(id)) => Some(id),
                _ => None,
            };
            fx.hint = r.hint(hover);
        }
        self.ui.room_fx = Some(fx);
        self.ui.hover = None;
        self.frame = None;
    }

    /// Opens a system screen from a menu button or Backspace. Over the
    /// room with its buttons up the room slides away first (the same
    /// twelve passes as look mode's), and slides back in when the screen
    /// is closed; elsewhere the screen opens at once.
    fn room_system(&mut self, sys: room::Sys) {
        if self.window_fades(sys) {
            self.ui.win_fx = Some(crate::games::doukyuusei::menu::WindowFx::new(
                true,
                self.ui.clock,
                self.rt.system.settings.effect_scale(),
                Some(sys),
            ));
            self.ui.hover = None;
            self.frame = None;
            return;
        }
        if !self.room_slides(sys) {
            self.open_system(sys);
            return;
        }
        self.room_slide(room::Step::HideRoom, None);
        if let Some(fx) = &mut self.ui.room_fx {
            fx.open = Some(sys);
        }
    }

    /// Whether the message window fades away before `sys` opens over a
    /// message (same twelve passes; the screen opens when it is gone).
    fn window_fades(&self, sys: room::Sys) -> bool {
        self.douk_system.is_some()
            && self.state == State::Text
            && self.rt.scene.text.is_some()
            && !self.ui.window_hidden
            && self.screen.is_none()
            && self.dialog.is_none()
            && self.ui.panel.is_none()
            && self.hmode.is_none()
            && self.ui.win_fx.is_none()
            && !self.menu_sliding()
            && self.can_open(sys)
    }

    /// Ends the message window's fade when its last pass is shown: after
    /// the fade out the screen opens.
    fn tick_window_fx(&mut self) {
        let Some(fx) = self.ui.win_fx else {
            return;
        };
        self.frame = None;
        if !fx.done(self.ui.clock) {
            return;
        }
        self.ui.win_fx = None;
        if !fx.out {
            let (x, y) = self.ui.pointer;
            self.ui.hover = self.hit(x, y);
            self.sync_cursor();
            return;
        }
        if let Some(sys) = fx.open {
            self.ui.win_off = true;
            self.open_system(sys);
        }
    }

    /// The screen that took the message window away is closed: the window
    /// fades back in (unless the game has gone on to something else).
    fn tick_window_return(&mut self) {
        if !self.ui.win_off
            || self.screen.is_some()
            || self.ui.screen_fx.is_some()
            || self.ui.win_fx.is_some()
        {
            return;
        }
        self.ui.win_off = false;
        self.frame = None;
        if self.state == State::Text && self.rt.scene.text.is_some() && !self.ui.window_hidden {
            self.ui.win_fx = Some(crate::games::doukyuusei::menu::WindowFx::new(
                false,
                self.ui.clock,
                self.rt.system.settings.effect_scale(),
                None,
            ));
            self.ui.hover = None;
        }
    }

    /// Whether the room slides away before `sys` opens (see
    /// [`Session::room_system`]).
    fn room_slides(&self, sys: room::Sys) -> bool {
        let on_room = matches!(&self.ui.panel, Some(DoukPanel::Room(r)) if !r.look);
        on_room && self.ui.room_fx.is_none() && self.can_open(sys)
    }

    /// Whether `open_system` would open its screen now.
    fn can_open(&self, sys: room::Sys) -> bool {
        let at_panel = self.douk_system.is_some() && self.state == State::Choice;
        let ready = self.state == State::Text || at_panel;
        match sys {
            room::Sys::Settings | room::Sys::Load => true,
            room::Sys::Save => ready && self.saves.is_some(),
            room::Sys::Backlog => ready && !self.log.is_empty(),
            room::Sys::Schedule => {
                self.douk_system.is_some() && self.ui.flag_list && !self.ui.classic
            }
        }
    }

    fn open_system(&mut self, sys: room::Sys) {
        match sys {
            room::Sys::Settings => self.open_settings(),
            room::Sys::Save => self.open_save_load(Mode::Save),
            room::Sys::Load => self.open_save_load(Mode::Load),
            room::Sys::Backlog => self.open_backlog(),
            room::Sys::Schedule => self.open_flag(),
        }
    }

    /// The system screen opened from the room is closed: the room slides
    /// back in.
    fn room_return(&mut self) {
        if !self.ui.room_away || self.screen.is_some() || self.ui.room_fx.is_some() {
            return;
        }
        self.ui.room_away = false;
        if matches!(&self.ui.panel, Some(DoukPanel::Room(r)) if !r.look && !r.is_plain()) {
            self.room_slide(room::Step::ShowRoom, None);
            if let Some(fx) = &mut self.ui.room_fx {
                fx.back = true;
            }
        }
    }

    /// The look button or the back arrow: look mode starts with the buttons
    /// sliding out, ends with the arrow sliding out.
    fn room_look(&mut self, on: bool) {
        if !on {
            self.vm.set_var_b(room::LOOK_BYTE, 0);
        }
        let step = if on {
            room::Step::HideRoom
        } else {
            room::Step::HideBack
        };
        self.room_slide(step, None);
    }

    /// The room hands `result` to the script: the buttons (or in look mode
    /// the back arrow) slide out first, and a look at a thing makes the
    /// place's next room start in look mode.
    fn room_result(&mut self, result: i32) -> Result<()> {
        let looking = matches!(&self.ui.panel, Some(DoukPanel::Room(r)) if r.look && !r.is_plain());
        let plain = matches!(&self.ui.panel, Some(DoukPanel::Room(r)) if r.is_plain());
        if plain {
            return self.room_answer(result);
        }
        if looking {
            self.vm.set_var_b(room::LOOK_BYTE, 1);
        }
        let step = if looking {
            room::Step::HideBack
        } else {
            room::Step::HideRoom
        };
        self.room_slide(step, Some(result));
        Ok(())
    }

    /// The room is gone; the script goes on with `result`.
    fn room_answer(&mut self, result: i32) -> Result<()> {
        self.ui.panel = None;
        self.ui.hover = None;
        self.base = None;
        self.rt.answer_action(&mut self.vm, result);
        self.run()
    }

    /// A room has come up: it slides in, in look mode with the back arrow
    /// when the last look at a thing left byte 8000 set.
    fn room_opens(&mut self, had_plate: bool) {
        let Some(DoukPanel::Room(r)) = &mut self.ui.panel else {
            return;
        };
        if r.is_plain() {
            return;
        }
        let looking = self.vm.var_b(room::LOOK_BYTE) != 0;
        r.look = looking;
        let step = if looking {
            room::Step::ShowBack
        } else {
            room::Step::ShowRoom
        };
        self.room_slide(step, None);
        if let Some(fx) = &mut self.ui.room_fx {
            fx.chrome = !had_plate;
        }
    }

    /// Ends a slide of the room when its last pass is shown: the next one
    /// starts, or the room's answer goes to the script.
    fn tick_room_fx(&mut self) -> Result<()> {
        use crate::games::doukyuusei::room::Step;
        let Some(fx) = self.ui.room_fx else {
            return Ok(());
        };
        self.frame = None;
        if !fx.done(self.ui.clock) {
            return Ok(());
        }
        self.ui.room_fx = None;
        if let Some(result) = fx.then {
            return self.room_answer(result);
        }
        if let Some(sys) = fx.open {
            self.ui.room_away = true;
            self.open_system(sys);
            return Ok(());
        }
        if let Some(step) = fx.next() {
            // Look mode begins as the buttons are gone, and ends as the
            // arrow is.
            self.ui
                .panel
                .as_mut()
                .map(|p| p.set_look(step == Step::ShowBack));
            self.room_slide(step, None);
            return Ok(());
        }
        // The pointer may rest on something now (not on a button of the
        // room that comes back: its states are reset until the pointer moves).
        if !fx.back {
            let (x, y) = self.ui.pointer;
            self.ui.hover = self.hit(x, y);
            self.sync_cursor();
        }
        Ok(())
    }

    /// Whether the open screen is one the original fades in and out
    /// (settings, save, load and the schedule list).
    fn screen_fades(&self) -> bool {
        self.douk_system.is_some()
            && matches!(
                self.screen,
                Some(Screen::DoukConfig(_) | Screen::DoukSave(_) | Screen::DoukFlag(_))
            )
    }

    /// A screen has just opened: it fades in over the scene.
    fn fade_screen_in(&mut self) {
        if self.screen_fades() {
            let scale = self.rt.system.settings.effect_scale();
            self.ui.screen_fx = Some(crate::games::doukyuusei::menu::ScreenFx::new(
                false,
                self.ui.clock,
                scale,
            ));
        }
    }

    /// A screen is being opened while the one before it fades out: it
    /// opens when that is over. Returns whether it was put off.
    fn open_after_fade(&mut self, next: crate::games::doukyuusei::menu::Next) -> bool {
        if self.screen.is_none() {
            return false;
        }
        match &mut self.ui.screen_fx {
            Some(fx) if fx.out => {
                fx.next = Some(next);
                true
            }
            _ => false,
        }
    }

    /// Ends a fade of a system screen when its last pass is shown.
    fn tick_screen_fx(&mut self) {
        use crate::games::doukyuusei::menu::Next;
        let Some(fx) = self.ui.screen_fx else {
            return;
        };
        if self.screen.is_none() {
            self.ui.screen_fx = None;
            return;
        }
        self.frame = None;
        if !fx.done(self.ui.clock) {
            return;
        }
        self.ui.screen_fx = None;
        if !fx.out {
            let (x, y) = self.ui.pointer;
            self.ui.hover = self.hit(x, y);
            self.sync_cursor();
            return;
        }
        self.finish_close();
        match fx.next {
            Some(Next::Settings { sound }) => {
                use crate::games::doukyuusei::config::Page;
                self.open_douk_settings(if sound { Page::Sound } else { Page::System1 })
            }
            Some(Next::Save { load }) => {
                self.open_save_load(if load { Mode::Load } else { Mode::Save })
            }
            Some(Next::Backlog) => self.open_backlog(),
            None => {}
        }
    }

    /// Redraws the dialog while it fades in and when its caret flips.
    fn tick_dialog(&mut self) {
        let clock = self.ui.clock;
        let Some(d) = &mut self.dialog else { return };
        if d.fading(clock) {
            self.frame = None;
        }
        if d.commenting {
            let on = crate::games::doukyuusei::savenote::caret_on(clock - d.opened.min(clock));
            if on != d.caret {
                d.caret = on;
                self.frame = None;
            }
        }
    }

    /// Ends the slide when its last pass is shown.
    fn tick_menu_anim(&mut self) {
        if let Some(a) = self.ui.menu_anim {
            self.frame = None;
            if a.done(self.ui.clock) {
                self.ui.menu_anim = None;
                if !a.opening {
                    self.ui.side_menu = false;
                }
            }
        }
    }

    fn close_screen(&mut self) {
        match &self.screen {
            Some(Screen::Settings(_) | Screen::DoukConfig(_)) => self.flush_system(),
            Some(Screen::Archive(s)) if s.recording.is_some() => self.rt.stop_sound_voice(),
            // The costume screen writes its choice back when it closes.
            Some(Screen::Costume(c)) => {
                for (i, v) in c.result() {
                    self.vm.set_var_b(i, v);
                }
                self.base = None;
            }
            _ => {}
        }
        // The original's screens fade out before they are gone.
        if self.screen_fades() {
            if !self.ui.screen_fx.is_some_and(|f| f.out) {
                let scale = self.rt.system.settings.effect_scale();
                self.ui.screen_fx = Some(crate::games::doukyuusei::menu::ScreenFx::new(
                    true,
                    self.ui.clock,
                    scale,
                ));
            }
            self.ui.hover = None;
            self.frame = None;
            return;
        }
        self.finish_close();
    }

    fn finish_close(&mut self) {
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
        if let (Some(Screen::DoukSave(s)), Some(saves)) = (&mut self.screen, &self.saves) {
            s.refresh(saves);
        }
        self.ui.has_saves = self.saves.as_ref().is_some_and(|s| !s.list().is_empty());
    }

    /// Asks before doing something, unless the player said not to ask.
    fn ask(&mut self, action: Action) -> Result<()> {
        let always = matches!(action, Action::Unlock(_) | Action::ComicReturn);
        // The jump to an event has its own setting.
        let confirm = match action {
            Action::FlagJump => self.rt.system.settings.confirm_event,
            _ => always || self.rt.system.settings.confirm[action.question()],
        };
        if !confirm {
            return self.carry_out(action);
        }
        let fresh = match (&action, &self.saves) {
            (Action::Save(Some(slot)), Some(saves)) => !saves.list().contains(slot),
            _ => false,
        };
        let douk = self.douk_system.is_some();
        let commenting = douk && matches!(action, Action::Save(_));
        // Overwriting starts with the old comment.
        let comment = match (&action, &self.saves) {
            (Action::Save(Some(slot)), Some(saves)) if commenting && !fresh => {
                saves.read(slot).map(|f| f.comment).unwrap_or_default()
            }
            _ => String::new(),
        };
        let fade = if douk {
            crate::games::doukyuusei::menu::slide_passes(self.rt.system.settings.effect_scale())
        } else {
            0
        };
        self.dialog = Some(Dialog {
            fresh,
            action,
            dont_ask: false,
            opener: self.clicked.take().or(self.ui.hover),
            commenting,
            comment,
            preedit: String::new(),
            opened: self.ui.clock,
            fade,
            caret: true,
        });
        self.ui.hover = None;
        self.frame = None;
        Ok(())
    }

    fn press_dialog(&mut self, id: i32) -> Result<()> {
        self.frame = None;
        // 同級生 Remake's buttons play the click sound; the no button the
        // cancel sound.
        if self.douk_system.is_some() {
            use crate::games::doukyuusei::name::{CANCEL_SOUND, OK_SOUND};
            let sound = match id {
                dialog_ids::YES | dialog_ids::DONT_ASK | dialog_ids::CLEAR => Some(OK_SOUND),
                dialog_ids::NO => Some(CANCEL_SOUND),
                _ => None,
            };
            if let Some(file) = sound {
                let _ = self.rt.play_sound(file);
            }
        }
        match id {
            dialog_ids::YES => {
                if let Some(d) = self.dialog.take() {
                    if d.dont_ask && d.action == Action::FlagJump {
                        self.rt.system.settings.confirm_event = false;
                        self.rt.system_changed = true;
                    } else if d.dont_ask
                        && d.action.question() < self.rt.system.settings.confirm.len()
                    {
                        self.rt.system.settings.confirm[d.action.question()] = false;
                        self.rt.system_changed = true;
                    }
                    self.save_comment = d.comment;
                    self.carry_out(d.action)?;
                }
            }
            dialog_ids::NO => self.dialog = None,
            dialog_ids::CLEAR => {
                if let Some(d) = &mut self.dialog {
                    d.comment.clear();
                }
            }
            dialog_ids::DONT_ASK => {
                if let Some(d) = &mut self.dialog {
                    d.dont_ask = !d.dont_ask;
                    // 同級生 Remake writes the setting when the box is
                    // ticked, whatever the answer.
                    if matches!(
                        self.rt.assets.game().title,
                        silky_format::game::Title::Doukyuusei | silky_format::game::Title::Nanairo
                    ) {
                        let s = &mut self.rt.system.settings;
                        if d.action == Action::FlagJump {
                            s.confirm_event = !d.dont_ask;
                        } else if let Some(c) = s.confirm.get_mut(d.action.question()) {
                            *c = !d.dont_ask;
                        }
                        self.rt.system_changed = true;
                    }
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
                let comment = std::mem::take(&mut self.save_comment);
                self.save_with_comment(&slot, comment)?;
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
            Action::FlagJump => self.jump_to_event()?,
            Action::Title => self.to_title()?,
            Action::Quit => {
                self.flush_system();
                self.state = State::Ended;
            }
            Action::Scene(i) => self.leave_extras(i as i32)?,
            Action::ReplayReturn | Action::ComicReturn => self.leave_replay()?,
            Action::Unlock(_) if matches!(self.ui.panel, Some(DoukPanel::Reel(_))) => {
                if let Some(DoukPanel::Reel(r)) = &mut self.ui.panel {
                    let girl = r.girl.unwrap_or(0);
                    for &i in r.girl_scenes(girl) {
                        self.vm.set_var_b(i, 1);
                    }
                    r.refresh(&self.vm.bytes);
                }
                self.store_global_bytes();
            }
            Action::Unlock(_) if matches!(self.ui.panel, Some(DoukPanel::Album(_))) => {
                if let Some(DoukPanel::Album(a)) = &mut self.ui.panel {
                    let girl = a.girl.unwrap_or(0);
                    for &i in a.girl_pictures(girl) {
                        self.vm.set_var_b(i, 1);
                    }
                    a.refresh(&self.vm.bytes);
                }
                self.store_global_bytes();
            }
            Action::Unlock(kind) => {
                for (i, v) in crate::games::kimaten2::omake::unlock_all(kind) {
                    self.vm.set_var_b(i, v);
                }
                self.store_global_bytes();
            }
        }
        Ok(())
    }

    fn press_screen(&mut self, id: i32, x: i32, y: i32) -> Result<()> {
        self.frame = None;
        match &self.screen {
            Some(Screen::Settings(_)) => return self.press_settings(id, x, y),
            Some(Screen::Extras(_)) => return self.press_extras(id, x, y),
            Some(Screen::Archive(_) | Screen::Costume(_)) => return self.press_archive(id, x, y),
            Some(Screen::DoukSave(_)) => return self.press_douk_save(id),
            Some(Screen::DoukConfig(_)) => return self.press_douk_config(id, x),
            Some(Screen::DoukFlag(_)) => return self.press_douk_flag(id),
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
                            .get(b.entry((id - VOICE) as usize).unwrap_or(usize::MAX))
                            .and_then(|e| e.voice.clone());
                        if let Some(file) = file {
                            self.voice_queue.clear();
                            self.playing_marked = false;
                            self.rt.replay_voice(&file)?;
                        }
                    }
                    _ if rows.contains(&(id - CHECK)) => {
                        if let Some(e) = self
                            .log
                            .get_mut(b.entry((id - CHECK) as usize).unwrap_or(usize::MAX))
                        {
                            e.marked = !e.marked;
                        }
                    }
                    _ if rows.contains(&(id - JUMP)) => {
                        ask = b.entry((id - JUMP) as usize).map(Action::Jump);
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
            Some(
                Screen::Archive(_)
                | Screen::Costume(_)
                | Screen::DoukSave(_)
                | Screen::DoukConfig(_)
                | Screen::DoukFlag(_),
            ) => {}
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

    fn press_extras(&mut self, id: i32, x: i32, y: i32) -> Result<()> {
        if self.menus.is_kimaten2() {
            return self.press_omake(id, y);
        }
        if self.menus.is_apeiria() {
            return self.press_apeiria(id, x, y);
        }
        use crate::games::fuyukuru::tables::MUSIC;
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

    /// A button of きまぐれテンプテーション2's extras.
    fn press_omake(&mut self, id: i32, y: i32) -> Result<()> {
        use crate::games::kimaten2::omake::{self, ids::*, View};
        let Some(Screen::Extras(s)) = &mut self.screen else {
            return Ok(());
        };
        self.frame = None;
        if let Some(v) = &mut s.k2.view {
            if id == omake::VIEW_CLOSE {
                s.k2.view = None;
                self.ui.hover = None;
            } else if id >= omake::VIEW_CELL {
                v.picture = Some(v.page * 12 + (id - omake::VIEW_CELL) as usize);
                self.ui.hover = None;
            }
            return Ok(());
        }
        match id {
            _ if (MODE..MODE + 3).contains(&id) => {
                let kind = [Extras::Gallery, Extras::Scenes, Extras::Music][(id - MODE) as usize];
                if kind != s.kind {
                    self.rt.stop_music();
                    *s = ExtrasScreen::new(kind);
                }
            }
            BACK => return self.leave_extras(-1),
            UNLOCK => {
                let kind = s.kind;
                return self.ask(Action::Unlock(kind));
            }
            _ if s.kind == Extras::Music => match id {
                PREVIOUS => s.scroll = s.scroll.saturating_sub(1),
                NEXT => s.scroll = (s.scroll + 1).min(omake::MAX_SCROLL),
                SCROLL => s.scroll = omake::scroll_at(y),
                VOLUME => s.k2.dragging_volume = true,
                _ if (0..7).contains(&id) => {
                    let Some(t) = omake::item(Extras::Music, 0, s.scroll, id as usize) else {
                        return Ok(());
                    };
                    if s.playing == Some(t) {
                        s.playing = None;
                        self.rt.stop_music();
                    } else {
                        s.playing = Some(t);
                        self.rt.play_music(&omake::track_file(t))?;
                    }
                }
                _ => {}
            },
            PREVIOUS => s.page = s.page.saturating_sub(1),
            NEXT => s.page = (s.page + 1).min(omake::CG_PAGES - 1),
            _ if (PAGE..PAGE + omake::CG_PAGES as i32).contains(&id) => {
                s.page = (id - PAGE) as usize;
            }
            _ if (0..12).contains(&id) => {
                let Some(item) = omake::item(s.kind, s.page, 0, id as usize) else {
                    return Ok(());
                };
                match s.kind {
                    // The engine plays the scene without asking.
                    Extras::Scenes => return self.leave_extras(item as i32),
                    _ => {
                        let single = omake::CG_ENTRIES[item].1 == 1;
                        s.k2.view = Some(View {
                            entry: item,
                            page: 0,
                            picture: single.then_some(0),
                        });
                        self.ui.hover = None;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// A button of 景の海のアペイリア's extras.
    fn press_apeiria(&mut self, id: i32, _x: i32, _y: i32) -> Result<()> {
        use crate::games::apeiria::omake::{self, ids::*, View};
        let Some(Screen::Extras(s)) = &mut self.screen else {
            return Ok(());
        };
        self.frame = None;
        let t = s.ap.t;
        if let Some(v) = &mut s.ap.view {
            match id {
                VIEW_PREVIOUS => v.page = v.page.saturating_sub(1),
                VIEW_NEXT => v.page = (v.page + 1).min(omake::view_pages(t, v.entry) - 1),
                VIEW_BACK => {
                    s.page = v.entry / t.cg.cells;
                    s.ap.view = None;
                }
                _ if (VIEW_CELL..VIEW_CELL + 12).contains(&id) => {
                    v.picture = Some(v.page * 12 + (id - VIEW_CELL) as usize);
                }
                _ => {}
            }
            self.ui.hover = None;
            return Ok(());
        }
        let pages = match s.kind {
            Extras::Gallery => t.cg.pages,
            _ => t.scene.pages,
        };
        match id {
            _ if (MODE..MODE + 3).contains(&id) => {
                let kind = [Extras::Gallery, Extras::Scenes, Extras::Music][(id - MODE) as usize];
                if kind != s.kind {
                    *s = ExtrasScreen::new(kind);
                    s.ap = omake::Omake::new(t);
                    self.ui.hover = None;
                    self.apeiria_extras_music(kind);
                }
                return Ok(());
            }
            BACK => return self.leave_extras(-1),
            _ if s.kind == Extras::Music => match id {
                PREVIOUS => s.scroll = s.scroll.saturating_sub(1),
                NEXT => s.scroll = (s.scroll + 1).min(t.max_scroll()),
                _ if (0..8).contains(&id) => {
                    let Some(track) = omake::item(t, Extras::Music, 0, s.scroll, id as usize)
                    else {
                        return Ok(());
                    };
                    if s.playing == Some(track) {
                        s.playing = None;
                        self.rt.stop_music();
                    } else {
                        s.playing = Some(track);
                        self.rt.play_music(t.track_files[track])?;
                    }
                }
                _ => {}
            },
            PREVIOUS => s.page = s.page.saturating_sub(1),
            NEXT => s.page = (s.page + 1).min(pages - 1),
            _ if (PAGE..PAGE + pages as i32).contains(&id) => s.page = (id - PAGE) as usize,
            _ if (0..12).contains(&id) => {
                let Some(item) = omake::item(t, s.kind, s.page, 0, id as usize) else {
                    return Ok(());
                };
                match s.kind {
                    // The engine plays the scene without asking.
                    Extras::Scenes => return self.leave_extras(item as i32),
                    _ => {
                        s.ap.view = Some(View {
                            entry: item,
                            page: 0,
                            picture: None,
                        });
                        self.ui.hover = None;
                    }
                }
            }
            _ => {}
        }
        if matches!(id, PREVIOUS | NEXT) || (PAGE..PAGE + pages as i32).contains(&id) {
            s.ap.preview = None;
        }
        Ok(())
    }

    /// 景の海のアペイリア: the music of its extras. The music mode stops what
    /// plays; the others play their own track when nothing does.
    fn apeiria_extras_music(&mut self, kind: Extras) {
        let extras_music = match &self.screen {
            Some(Screen::Extras(e)) => e.ap.t.extras_music,
            _ => return,
        };
        if kind == Extras::Music {
            self.rt.stop_music();
        } else if !self.rt.audio.is_playing(Bus::Music, 0) {
            // A missing track only leaves the screens silent.
            let _ = self.rt.play_music(extras_music);
        }
    }

    /// The scroll bar of 景の海のアペイリア's music mode follows the pointer.
    fn drag_apeiria_scroll(&mut self, y: i32) {
        if let Some(Screen::Extras(e)) = &mut self.screen {
            e.scroll = crate::games::apeiria::omake::scroll_at(e.ap.t, y);
            self.frame = None;
        }
    }

    /// The scene under the pointer in 景の海のアペイリア's scene mode starts
    /// its slide show.
    fn apeiria_preview(&mut self) {
        use crate::games::apeiria::omake;
        if !self.menus.is_apeiria() {
            return;
        }
        let Some(Screen::Extras(e)) = &mut self.screen else {
            return;
        };
        let scene = match (e.kind, self.ui.hover) {
            (Extras::Scenes, Some(Button::Screen(id))) if (0..12).contains(&id) => {
                omake::item(e.ap.t, Extras::Scenes, e.page, 0, id as usize)
            }
            _ => None,
        };
        if e.ap.preview.map(|p| p.0) != scene {
            e.ap.preview = scene.map(|s| (s, self.ui.clock));
        }
    }

    /// The music volume bar of きまぐれテンプテーション2 follows the pointer.
    fn drag_omake_volume(&mut self, x: i32) {
        let old = self.rt.system.settings.clone();
        self.rt.system.settings.music = if self.menus.is_apeiria() {
            crate::games::apeiria::omake::volume_at(x)
        } else if self.menus.is_kimaten2() {
            crate::games::kimaten2::omake::volume_at(x)
        } else {
            menus::music_volume_at(x)
        };
        self.settings_changed(old);
        self.frame = None;
    }

    /// Key that goes on (Enter, Space).
    pub fn proceed(&mut self) -> Result<()> {
        // Space does nothing on 同級生 Remake's dialogs and name entry.
        if self.douk_system.is_some()
            && (self.dialog.is_some() || matches!(self.ui.panel, Some(DoukPanel::Name(_))))
        {
            return Ok(());
        }
        // The notebook of きまぐれテンプテーション: Enter clicks what the
        // pointer is on (where the arrow keys put it).
        if self.dialog.is_none()
            && self.screen.is_none()
            && self
                .ui
                .k1_menu
                .as_ref()
                .or(self.ui.k1.as_ref())
                .is_some_and(|s| s.notebook().is_some())
        {
            let (x, y) = self.ui.pointer;
            self.pointer_down(x, y)?;
            return self.pointer_up(x, y);
        }
        // 景の海のアペイリア's extras: Enter clicks where the arrow keys
        // put the pointer.
        if self.menus.is_apeiria() && matches!(self.screen, Some(Screen::Extras(_))) {
            let (x, y) = self.ui.pointer;
            self.pointer_down(x, y)?;
            return self.pointer_up(x, y);
        }
        // A selection of 同級生 Remake: Enter clicks where the arrow keys
        // put the pointer.
        if !self.ui.choices.is_empty()
            && self.dialog.is_none()
            && self.screen.is_none()
            && self.layout.douk_key_state(&self.ui).is_some()
        {
            let (x, y) = self.ui.pointer;
            self.pointer_down(x, y)?;
            return self.pointer_up(x, y);
        }
        // Enter ends a text strip's wait (the original's key wait takes
        // Enter, Ctrl and the left button alike).
        if self.state == State::Busy {
            if let Some(b) = self.busy.as_mut().filter(|b| b.key_wait) {
                b.elapsed = b.ms;
                return Ok(());
            }
        }
        if self.ui.window_hidden {
            self.ui.window_hidden = false;
            self.base = None;
            self.frame = None;
            return Ok(());
        }
        self.advance()
    }

    fn press(&mut self, b: Button) -> Result<()> {
        // Any button but the side menu's own closes the side menu; the
        // room's menu stays open through the screen a button opens (the
        // room only slides away and back).
        let keeps = match b {
            Button::Bar(bar::CONFIG) => self.room_slides(room::Sys::Settings),
            Button::Bar(bar::SAVE) => self.room_slides(room::Sys::Save),
            Button::Bar(bar::LOAD) => self.room_slides(room::Sys::Load),
            Button::Bar(bar::BACKLOG) => self.room_slides(room::Sys::Backlog),
            Button::Bar(bar::SCHEDULE) => self.room_slides(room::Sys::Schedule),
            _ => false,
        };
        if b != Button::Bar(bar::MENU) && !keeps {
            self.ui.side_menu = false;
        }
        match b {
            Button::Title(i) => match i {
                title::EXTRAS => {
                    self.ui.title = Some(true);
                    self.ui.title_shown = self.ui.clock;
                }
                title::SETTINGS => self.open_settings(),
                title::CONTINUE => self.load_latest()?,
                title::LOAD => self.open_save_load(Mode::Load),
                title::BACK => {
                    self.ui.title = Some(false);
                    self.ui.title_shown = self.ui.clock;
                }
                // The engine's own screen; the script does nothing with 261.
                title::NOTEBOOK if self.layout.is_kimaten1() => self.kimaten1_notebook_hook(),
                title::NOTEBOOK => self.open_archive(Book::Notebook),
                // 同級生 Remake quits from the title on its own.
                title::QUIT if self.layout.title_result(i).is_none() => self.ask(Action::Quit)?,
                i => {
                    if let Some(result) = self.layout.title_result(i) {
                        self.ui.title = None;
                        // The script's fade starts from the title as shown,
                        // menu included.
                        if let Some(f) = self.frame.take() {
                            self.base = Some(f);
                        }
                        self.rt.answer_title(&mut self.vm, result);
                        self.run()?;
                    } else {
                        self.rt.note_unhandled(format!("title menu entry {i}"));
                    }
                }
            },
            Button::Action(i) => {
                let result = self.ui.actions.get(i).map_or(-1, |a| a.result);
                self.ui.actions.clear();
                self.base = None;
                self.rt.answer_action(&mut self.vm, result);
                self.run()?;
            }
            Button::Map(id) => {
                use crate::games::kimaten2::map::{Click, BACK};
                let Some(m) = &mut self.ui.map else {
                    return Ok(());
                };
                let click = match id {
                    512 => Some(Click::Result(BACK)),
                    256.. => m.click_panel((id - 256) as usize),
                    _ => m.click_place(id as usize),
                };
                match click {
                    Some(Click::Result(r)) => {
                        self.ui.map = None;
                        self.base = None;
                        self.rt.answer_action(&mut self.vm, r);
                        self.run()?;
                    }
                    Some(Click::OpenPanel(p)) => m.panel = Some(p),
                    Some(Click::ClosePanel) => m.panel = None,
                    None => {}
                }
            }
            Button::Kimaten1(id) => {
                let (px, py) = self.ui.pointer;
                let click = match self.ui.k1_menu.as_mut().or(self.ui.k1.as_mut()) {
                    Some(s) => {
                        s.press_at(id, px, py);
                        s.click(id)
                    }
                    None => return Ok(()),
                };
                self.k1_answer(click)?;
            }
            Button::Panel(id) => {
                use crate::games::doukyuusei::Click;
                let Some(p) = &mut self.ui.panel else {
                    return Ok(());
                };
                let click = p.click(id);
                let set = match p {
                    crate::games::doukyuusei::Panel::Picker(k) => {
                        if let Some(file) = k.sound(id) {
                            let _ = self.rt.play_sound(file);
                        }
                        Vec::new()
                    }
                    crate::games::doukyuusei::Panel::Album(a) => a.take_globals(),
                    crate::games::doukyuusei::Panel::Reel(r) => r.take_globals(),
                    crate::games::doukyuusei::Panel::Music(m) => m.take_globals(),
                    crate::games::doukyuusei::Panel::Comment(c) => c.take_globals(),
                    crate::games::doukyuusei::Panel::Name(n) => {
                        if n.take_sound() {
                            let _ = self.rt.play_sound(crate::games::doukyuusei::name::OK_SOUND);
                        }
                        Vec::new()
                    }
                    crate::games::doukyuusei::Panel::Room(r) => {
                        if r.click_sound(id) {
                            let _ = self.rt.play_sound(crate::games::doukyuusei::name::OK_SOUND);
                        }
                        Vec::new()
                    }
                    crate::games::doukyuusei::Panel::Jump(_)
                    | crate::games::doukyuusei::Panel::Unlocked(_)
                    | crate::games::doukyuusei::Panel::Notice(_) => {
                        if !matches!(click, Click::None) {
                            let _ = self.rt.play_sound(crate::games::doukyuusei::name::OK_SOUND);
                        }
                        Vec::new()
                    }
                    _ => Vec::new(),
                };
                for (i, v) in set {
                    self.vm.set_g(i, v);
                }
                match click {
                    Click::Result(result) => {
                        if matches!(self.ui.panel, Some(DoukPanel::Music(_))) {
                            self.rt.stop_music();
                        }
                        // A place chosen on the map jump ends the walk.
                        self.ui.side_menu &= !matches!(self.ui.panel, Some(DoukPanel::Jump(_)));
                        if matches!(self.ui.panel, Some(DoukPanel::Room(_))) {
                            self.room_result(result)?;
                        } else {
                            self.room_answer(result)?;
                        }
                    }
                    Click::Resume => {
                        self.ui.panel = Some(DoukPanel::Town);
                        self.ui.hover = None;
                        self.base = None;
                    }
                    Click::Close => self.close_notice()?,
                    Click::Name => {
                        if let Some(DoukPanel::Name(n)) = self.ui.panel.take() {
                            self.rt.finish_name(&mut self.vm, &n.text());
                        }
                        self.base = None;
                        self.run()?;
                    }
                    Click::Track(Some(t)) => {
                        // A track missing from the archive stays silent.
                        let _ = self
                            .rt
                            .play_music_once(&crate::games::doukyuusei::music::file(t));
                    }
                    Click::Track(None) => self.rt.stop_music(),
                    Click::Look(on) => self.room_look(on),
                    Click::QuickSave => self.save(save::QUICK)?,
                    Click::QuickLoad => self.load(save::QUICK)?,
                    Click::Settings => {
                        self.open_douk_settings(crate::games::doukyuusei::config::Page::System1)
                    }
                    Click::Backlog if self.ui.has_log => self.open_backlog(),
                    Click::Backlog => {}
                    Click::Save => self.open_save_load(Mode::Save),
                    Click::Load => self.open_save_load(Mode::Load),
                    Click::Unlock => {
                        let kind = match &self.ui.panel {
                            Some(DoukPanel::Reel(_)) => crate::Extras::Scenes,
                            _ => crate::Extras::Gallery,
                        };
                        self.ask(Action::Unlock(kind))?
                    }
                    Click::None => {}
                }
            }
            Button::ChoicePage(id) => {
                // The arrows play the OK sound and do not answer.
                let _ = self.rt.play_sound(crate::games::doukyuusei::name::OK_SOUND);
                let count = self.ui.choices.len();
                self.ui.choice_page =
                    crate::games::doukyuusei::select::turn(count, self.ui.choice_page, id);
                // The bar now under the pointer shows its hover.
                let (x, y) = self.ui.pointer;
                self.ui.hover = self.hit(x, y);
                self.frame = None;
            }
            Button::Choice(i) => {
                self.ui.choices.clear();
                self.ui.choice_status.clear();
                let (skip, auto) = self.modes_at_choice;
                let st = &self.rt.system.settings;
                self.ui.skip = skip && st.keep_skip_after_choices;
                self.ui.auto = auto && st.keep_auto_after_choices;
                self.rt.answer_choice(&mut self.vm, i);
                self.run()?;
            }
            Button::Bar(bar::MENU) => self.toggle_side_menu(),
            Button::Bar(bar::RING) => {
                self.ui.ring_closed = !self.ui.ring_closed;
                if self.layout.is_kimaten1() {
                    self.ui.k1_fold = Some((!self.ui.ring_closed, self.ui.clock));
                }
            }
            Button::Bar(id) => match id {
                bar::AUTO => {
                    self.ui.auto = !self.ui.auto;
                    self.ui.skip = false;
                    self.shown_ms = 0;
                }
                bar::SKIP => {
                    self.ui.skip = !self.ui.skip || self.ui.skip_all;
                    self.ui.skip_all = false;
                    self.skip_to_choice = false;
                    self.ui.auto = false;
                }
                bar::SKIP_ALL => {
                    self.ui.skip = !(self.ui.skip && self.ui.skip_all);
                    self.ui.skip_all = self.ui.skip;
                    self.skip_to_choice = false;
                    self.ui.auto = false;
                }
                bar::LOOP_VOICE => self.ui.loop_voice = !self.ui.loop_voice,
                bar::NEXT_CHOICE => {
                    self.ui.skip = true;
                    self.skip_to_choice = true;
                    self.ui.auto = false;
                }
                bar::PREVIOUS_CHOICE => self.previous_choice()?,
                bar::CONFIG => self.room_system(room::Sys::Settings),
                bar::COSTUME => self.open_costume(),
                bar::PIN => {
                    let st = &mut self.rt.system.settings;
                    st.menu_lock = !st.menu_lock;
                    self.ui.menu_lock = st.menu_lock;
                    self.flush_system();
                }
                bar::NOTEBOOK => self.open_archive(Book::Notebook),
                bar::K1_NOTEBOOK => self.open_k1_notebook(),
                bar::COLLECTION => self.open_archive(Book::Items),
                bar::HIDE | bar::CLOSE => {
                    self.ui.window_hidden = true;
                    self.base = None;
                }
                bar::TITLE => self.ask(Action::Title)?,
                bar::QUICK_SAVE => self.save(save::QUICK)?,
                bar::QUICK_LOAD => self.load(save::QUICK)?,
                bar::SAVE => self.room_system(room::Sys::Save),
                bar::LOAD => self.room_system(room::Sys::Load),
                bar::BACKLOG => self.room_system(room::Sys::Backlog),
                bar::MAP => self.open_jump(),
                bar::SCHEDULE => self.room_system(room::Sys::Schedule),
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

    /// 同級生 Remake: the flag list over the room or the walk.
    fn open_flag(&mut self) {
        use crate::games::doukyuusei::flaglist::{self, Env, FlagScreen};
        if self.douk_system.is_none() || !self.ui.flag_list || self.ui.classic {
            return;
        }
        let (can_save, has_log) = self.douk_can();
        let g = |i| self.vm.var_g(i).int();
        let env = Env {
            bytes: &self.vm.bytes,
            words: &self.vm.words,
            date: g(crate::games::doukyuusei::plate::DATE),
            cash: g(crate::games::doukyuusei::plate::MONEY),
            time: g(crate::games::doukyuusei::plate::TIME),
        };
        // The notice comes once, the first time.
        let notice = self.vm.var_b(flaglist::NOTICE_SEEN) == 0;
        let s = FlagScreen::new(&env, can_save, has_log, notice);
        if notice {
            self.vm.set_var_b(flaglist::NOTICE_SEEN, 1);
        }
        self.screen = Some(Screen::DoukFlag(s));
        self.fade_screen_in();
        self.ui.auto = false;
        self.ui.skip = false;
        self.ui.hover = None;
        self.frame = None;
    }

    /// The modal notice or pop-up is done: the script goes on.
    fn close_notice(&mut self) -> Result<()> {
        self.ui.panel = None;
        self.rt.panel = None;
        self.ui.hover = None;
        self.frame = None;
        self.base = None;
        self.run()
    }

    /// A click on 同級生 Remake's flag list.
    fn press_douk_flag(&mut self, id: i32) -> Result<()> {
        use crate::games::doukyuusei::flaglist::Click as FlagClick;
        use crate::games::doukyuusei::system::side;
        let Some(Screen::DoukFlag(s)) = &mut self.screen else {
            return Ok(());
        };
        let click = s.click(id);
        if click != FlagClick::None {
            let _ = self.rt.play_sound(crate::games::doukyuusei::name::OK_SOUND);
        }
        self.frame = None;
        match click {
            FlagClick::Side(k) => match k {
                side::SETTINGS | side::SOUND => {
                    use crate::games::doukyuusei::config::Page;
                    let page = if k == side::SOUND {
                        Page::Sound
                    } else {
                        Page::System1
                    };
                    self.close_screen();
                    self.open_douk_settings(page);
                }
                side::SAVE | side::LOAD => {
                    self.close_screen();
                    self.open_save_load(if k == side::LOAD {
                        Mode::Load
                    } else {
                        Mode::Save
                    });
                }
                side::LOG => {
                    self.close_screen();
                    self.open_backlog();
                }
                side::TITLE => self.ask(Action::Title)?,
                side::QUIT => self.ask(Action::Quit)?,
                _ => {}
            },
            FlagClick::Leave => self.close_screen(),
            FlagClick::Jump => self.ask(Action::FlagJump)?,
            FlagClick::Changed | FlagClick::None => {}
        }
        Ok(())
    }

    /// 同級生 Remake: the map jump page over the walk.
    fn open_jump(&mut self) {
        use crate::games::doukyuusei::{jump::Jump, Panel};
        if !matches!(self.ui.panel, Some(Panel::Town)) {
            return;
        }
        if let Ok(j) = Jump::load(&self.rt.assets, &self.vm, self.rt.easy_mode()) {
            self.ui.panel = Some(Panel::Jump(j));
            self.ui.side_menu = true;
            self.ui.hover = None;
            self.base = None;
        }
    }

    fn hit(&self, x: i32, y: i32) -> Option<Button> {
        if let Some(d) = &self.dialog {
            return self.screens.hit_dialog(d, x, y);
        }
        match &self.screen {
            Some(Screen::SaveLoad(s)) => return self.screens.hit_save_load(s, x, y),
            Some(Screen::Backlog(b)) => return self.screens.hit_backlog(b, &self.log, x, y),
            Some(Screen::Settings(s)) => return self.menus.hit_settings(s, x, y).map(|h| h.0),
            Some(Screen::Extras(e)) => {
                return self
                    .menus
                    .hit_extras(e, &self.rt.system, &self.vm.bytes, x, y)
            }
            Some(Screen::Archive(s)) => return self.archive.hit(s, x, y),
            Some(Screen::Costume(c)) => return self.archive.hit_costume(c, x, y),
            Some(Screen::DoukSave(s)) => {
                return self
                    .douk_system
                    .as_ref()?
                    .hit_save(s, x, y)
                    .map(Button::Screen)
            }
            Some(Screen::DoukConfig(s)) => {
                use crate::games::doukyuusei::system::System;
                let id = self.douk_system.as_ref()?.hit_config(s, x, y)?;
                let (can_save, has_log) = self.douk_can();
                let st = &self.rt.system.settings;
                return System::config_enabled(s, st, id, can_save, has_log)
                    .then_some(Button::Screen(id));
            }
            Some(Screen::DoukFlag(s)) => {
                return s.hit(self.douk_system.as_ref()?, x, y).map(Button::Screen)
            }
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
        self.ui.voice = self.rt.voice_playing();
        // Calls recorded for a trace carry the session clock.
        self.vm.set_trace_tick(self.ui.clock);
        if self.script_quit && self.dialog.is_none() {
            // The box was answered no (or the question was not asked).
            self.script_quit = false;
            self.frame = None;
            if self.state != State::Ended {
                self.run()?;
            }
        }
        self.ui.tooltips = self.rt.system.settings.tooltips;
        if self.state != State::Busy && self.rt.scene.cover.is_none() && self.plate_gone {
            self.plate_hold = None;
            self.plate_gone = false;
        }
        // 同級生 Remake: the game mode is read as it is now; a change from
        // the settings shows on the map's labels and the jump page's faces
        // (the plate and the flag list button follow from [`Ui`]).
        let classic = !self.rt.easy_mode();
        if self.ui.classic != classic {
            self.ui.classic = classic;
            self.rt.refresh_town(&self.vm.bytes);
            if let Some(DoukPanel::Jump(j)) = &mut self.ui.panel {
                j.set_easy(!classic);
            }
            self.frame = None;
        }
        self.ui.plate_keep = self
            .vm
            .var_b(crate::games::doukyuusei::commands::PLATE_KEEP)
            != 0;
        let plate = self.plate_hold.or_else(|| self.rt.plate(&self.vm));
        if plate != self.ui.plate {
            self.ui.plate = plate;
            self.frame = None;
        }
        // The held track of the flag list steps on while the pointer
        // stays off the thumb.
        if let Some(Screen::DoukFlag(s)) = &mut self.screen {
            if s.dragging && s.drag(self.ui.pointer.1) {
                self.frame = None;
            }
        }
        if let Some(DoukPanel::Name(n)) = &mut self.ui.panel {
            if n.tick(ms) {
                self.frame = None;
            }
        }
        // The music mode's track ended (without audio it plays on).
        if let Some(DoukPanel::Music(m)) = &mut self.ui.panel {
            if m.playing.is_some()
                && self.rt.audio.enabled()
                && !self.rt.audio.is_playing(Bus::Music, 0)
            {
                m.playing = None;
                self.frame = None;
            }
        }
        // The extras screen's picked track ended (without audio it plays on).
        if let Some(k) = &mut self.ui.k1 {
            if k.track_playing()
                && self.rt.audio.enabled()
                && !self.rt.audio.is_playing(Bus::Music, 0)
                && k.track_ended()
            {
                self.frame = None;
            }
        }
        let flag = crate::games::doukyuusei::flaglist::available(
            &self.vm.bytes,
            self.vm.var_g(crate::games::doukyuusei::plate::DATE).int(),
        );
        if self.ui.flag_list != flag {
            self.ui.flag_list = flag;
            self.frame = None;
        }
        let jump = crate::games::doukyuusei::jump::Jump::available(&self.vm.bytes);
        if self.ui.map_jump != jump {
            self.ui.map_jump = jump;
            self.frame = None;
        }
        if self.ui.has_log == self.log.is_empty() {
            self.ui.has_log = !self.log.is_empty();
            self.frame = None;
        }
        if self.playing_marked && !self.rt.voice_playing() {
            match self.voice_queue.pop_front() {
                Some(file) => self.rt.replay_voice(&file)?,
                None => {
                    self.playing_marked = false;
                    self.frame = None;
                }
            }
        }
        if self.ui.loop_voice
            && self.state == State::Text
            && !self.ui.skip
            && !self.playing_marked
            && !self.rt.voice_playing()
        {
            if let Some(file) = self.rt.voice.clone() {
                self.rt.replay_voice(&file)?;
                self.rt.restart_lip();
            }
        }
        if self.layout.is_kimaten1() {
            let on = self.vm.var_b(899) != 0;
            if on != self.ui.notebook {
                self.ui.notebook = on;
                self.frame = None;
            }
            let replay = self.vm.var_h(crate::games::apeiria::REPLAY_VAR).int() != 0;
            if replay != self.ui.k1_replay {
                self.ui.k1_replay = replay;
                self.frame = None;
            }
        }
        if self.layout.is_kimaten2() {
            self.refresh_archive_flags();
            self.advance_recording();
        }
        if self.layout.keeps_global_bytes() {
            self.store_global_bytes();
        }
        if self.layout.is_kimaten2() || self.layout.is_doujin() {
            self.slide_bar(before);
        }
        self.tick_menu_anim();
        self.tick_screen_fx();
        self.tick_window_fx();
        self.tick_window_return();
        self.tick_room_fx()?;
        self.room_return();
        self.tick_dialog();
        // The game's own pointer swaps its two frames every 300 ms.
        self.sync_cursor();
        if self.rt.cursor.shown()
            && self.rt.cursor.frame(before) != self.rt.cursor.frame(self.ui.clock)
        {
            self.frame = None;
        }
        // きまぐれテンプテーション's notices slide over the scene.
        let moved = self.rt.k1_notices.advance(self.ui.clock);
        if moved || self.ui.k1_notices.is_some() != self.rt.k1_notices.visible() {
            self.ui.k1_notices = self
                .rt
                .k1_notices
                .visible()
                .then(|| self.rt.k1_notices.clone());
            self.frame = None;
        }
        // The map and the notebook fade in and out; a screen that is
        // leaving answers when its fade is over.
        let k1_ms = self.ui.clock.saturating_sub(self.ui.k1_since);
        let mut k1_click = None;
        if let Some(s) = self.ui.k1_menu.as_mut().or(self.ui.k1.as_mut()) {
            if s.animating(k1_ms) {
                self.frame = None;
            }
            k1_click = s.tick(k1_ms);
            // The notebook's recording goes on to its next line when the
            // one playing ends (without audio it plays on).
            if k1_click.is_none()
                && s.voice_wait()
                && self.rt.audio.enabled()
                && !self.rt.voice_playing()
                && s.voice_ended()
            {
                k1_click = Some(crate::games::kimaten1::Click::None);
            }
        }
        if let Some(click) = k1_click {
            self.frame = None;
            self.k1_answer(click)?;
        }
        // The viewer's fades of the buttons and of the model.
        if let Some(s) = self.ui.k1.as_mut() {
            let ms = self.ui.clock.saturating_sub(self.ui.k1_since);
            let (scene, redraw) = s.tick_scene(&mut self.rt.scene, ms);
            if scene {
                self.base = None;
            }
            if scene || redraw {
                self.frame = None;
            }
        }
        // The staff roll rises by itself and ends when its time is up.
        if let Some(s) = self.ui.k1.as_ref().filter(|s| s.timed()) {
            self.frame = None;
            if s.over(self.ui.clock.saturating_sub(self.ui.k1_since)) {
                self.k1_answer(crate::games::kimaten1::Click::Result(1))?;
            }
        }
        if let Some(v) = self.veil {
            self.frame = None;
            if v.done(self.ui.clock) {
                match v.then {
                    Some(result) => self.close_extras(result)?,
                    None => self.veil = None,
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
        if self.hmode.is_some() {
            self.tick_hmode(ms)?;
        }
        if self.walking() {
            self.tick_town(ms)?;
        }
        // The Anime class takes a step a frame (60 a second).
        let frames = (self.ui.clock * 60 / 1000 - before * 60 / 1000) as u32;
        if self.screen.is_none() && self.rt.tick_anime(frames) {
            self.frame = None;
        }
        if self.screen.is_none() && self.renderer.progress_snow(&self.rt.scene, ms) {
            self.frame = None;
        }
        if self.screen.is_none() && self.rt.system.settings.emote_animation {
            // Mouth of the E-mote pictures: the voice's track (command 40,
            // sub 16), 60 values a second, while the voice plays. The
            // script starts the voice before it hands over the track.
            let playing = self.rt.voice_playing();
            let talking = match &mut self.rt.lip {
                Some((track, played)) if playing => {
                    *played += ms as f32;
                    let value = track.get((*played * 0.06) as usize).copied();
                    if value.is_none() {
                        self.rt.lip = None;
                    }
                    value.map(|v| v.max(0.0))
                }
                // Ended, or not started yet.
                Some((_, played)) => {
                    if *played > 0.0 {
                        self.rt.lip = None;
                    }
                    None
                }
                None => None,
            };
            if self.renderer.progress_emote(&self.rt.scene, ms, talking) {
                self.emote_moved = true;
                self.frame = None;
            }
        }
        if let Some(Screen::DoukConfig(s)) = &self.screen {
            let per = self.rt.system.settings.char_ms().max(1);
            if s.page == crate::games::doukyuusei::config::Page::System1
                && (before - s.opened) / per != (self.ui.clock - s.opened) / per
            {
                self.frame = None;
            }
        }
        if let Some(Screen::Settings(s)) = &self.screen {
            // The window sample types out.
            let per = self.rt.system.settings.char_ms().max(1);
            if s.page == Page::System
                && (before - s.opened) / per != (self.ui.clock - s.opened) / per
            {
                self.frame = None;
            }
        }
        if let Some(Screen::Extras(e)) = &mut self.screen {
            if let Some((scene, _)) = e.ap.preview {
                if crate::games::apeiria::omake::animates(e.ap.t, scene)
                    && before / 40 != self.ui.clock / 40
                {
                    self.frame = None;
                }
            }
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
        self.play_effects();
        if self.rt.scene.effects.advance(ms) {
            self.base = None;
            self.frame = None;
        }
        self.take_painted();
        if let Some(holo) = &mut self.rt.scene.holo {
            if holo.advance(ms) {
                self.base = None;
                self.frame = None;
            }
        }
        if let Some(m) = &mut self.rt.scene.mail {
            let (moved, closed) = m.advance(ms);
            if closed {
                self.rt.scene.mail = None;
            }
            if moved {
                self.base = None;
                self.frame = None;
            }
        }
        if self.rt.scene.panels.advance(ms) {
            self.base = None;
            self.frame = None;
        }
        self.shown_ms += ms;
        if self.state == State::Busy {
            // Several short steps (input polls) may fit in one tick.
            let mut left = ms as u32;
            let mut steps = 0;
            while self.state == State::Busy && steps < 64 {
                let Some(b) = &mut self.busy else { break };
                // The roll does not look at the skip mode either.
                if self.ui.skip
                    && self.rt.scene.intro.is_none()
                    && !self.rt.roll_locked
                    && !self.rt.rolling()
                {
                    b.elapsed = b.ms;
                }
                let step = left.min(b.ms - b.elapsed.min(b.ms));
                b.elapsed += step;
                left -= step;
                if let Some(train) = &mut self.rt.scene.train {
                    if train.frame.is_some() {
                        train.progress(b.elapsed, b.ms);
                        self.base = None;
                    }
                }
                if let Some(intro) = &mut self.rt.scene.intro {
                    intro.progress(b.elapsed, b.ms);
                    self.base = None;
                }
                if let Some(pan) = &mut self.rt.scene.pan {
                    pan.progress(b.elapsed, b.ms);
                    self.base = None;
                }
                if b.elapsed < b.ms {
                    break;
                }
                self.finish_busy()?;
                steps += 1;
            }
            self.frame = None;
            return Ok(true);
        }
        if self.state == State::Text
            && crate::games::doujin::comic::tick(&mut self.rt, ms, self.ui.skip)
        {
            self.frame = None;
        }
        if self.state == State::Text
            && crate::games::doujin::comic::answer_due(&mut self.rt, &mut self.vm)
        {
            self.advance()?;
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
        // Where unread text stops skipping: always for the jump button of
        // きまぐれテンプテーション2 (to the next unread text or selection),
        // never for skipping to the next selection of ふゆから、くるる。,
        // else as the settings say.
        let stops_at_unread = if self.skip_to_choice {
            self.layout.is_kimaten2()
        } else {
            !self.ui.skip_all && st.skip_read_only
        };
        if self.state == State::Text && self.ui.skip && stops_at_unread && !self.rt.read_before {
            self.ui.skip = false;
            self.skip_to_choice = false;
            self.frame = None;
        }
        if self.state == State::Text && !self.ui.window_hidden && self.ui.win_fx.is_none() {
            let st = &self.rt.system.settings;
            let text = self.rt.scene.text.as_deref().unwrap_or("");
            let due = if self.ui.skip {
                Some(st.skip_ms())
            } else if self.ui.auto && !self.rt.voice_playing() {
                // Auto mode waits for the whole text and for the voice.
                Some(
                    crate::games::doujin::comic::auto_ms(&self.rt)
                        .unwrap_or_else(|| self.reveal_ms().max(st.auto_ms(text))),
                )
            } else {
                None
            };
            if due.is_some_and(|d| self.shown_ms >= d) {
                self.advance()?;
                return Ok(true);
            }
        }
        if self.ui.title.is_some()
            && self.layout.is_kimaten1()
            && crate::games::kimaten1::title::animating(&self.ui, before)
        {
            self.frame = None;
            return Ok(true);
        }
        if self.ui.title.is_some()
            && self.layout.is_kimaten2()
            && before < self.ui.title_shown + crate::ui::TITLE_JUMP_MS
        {
            self.frame = None;
            return Ok(true);
        }
        if self.ui.title.is_some()
            && self.ui.title_shown > self.ui.title_opened
            && self.layout.is_doujin()
            && before < self.ui.title_shown + crate::games::doujin::title::FADE_MS
        {
            self.frame = None;
            return Ok(true);
        }
        let animating = self.ui.waiting && self.state == State::Text && !self.ui.window_hidden;
        if animating && before / 100 != self.ui.clock / 100 {
            self.frame = None;
            return Ok(true);
        }
        Ok(self.frame.is_none())
    }

    /// The size the game lays its screen out in (1280 x 720 for most).
    pub fn screen_size(&self) -> (u32, u32) {
        self.renderer.size()
    }

    /// Sets the height of the frames in output pixels (the game is laid
    /// out in [`Session::screen_size`]; pictures are scaled up, text is
    /// drawn at this size). Below the game's height the frames stay at it.
    pub fn set_output_height(&mut self, height: u32) {
        let scale = (height as f32 / self.renderer.size().1 as f32).max(1.0);
        if (scale - self.renderer.scale).abs() > 1e-3 {
            self.renderer.scale = scale;
            self.base = None;
            self.frame = None;
        }
    }

    /// The current frame at output size (see
    /// [`Session::set_output_height`]), RGBA.
    pub fn frame(&mut self) -> Result<&Canvas> {
        if self.frame.is_none() {
            let mut frame = match &self.screen {
                Some(screen) => {
                    // The settings screen's window sample shows the game
                    // screen through it.
                    let under = match screen {
                        Screen::Settings(_) | Screen::DoukConfig(_)
                            if !self.layout.is_kimaten2() =>
                        {
                            let c = self
                                .renderer
                                .render_scene(&self.rt.scene, &mut self.rt.assets, false)?
                                .rasterize();
                            Some(Arc::new(Image {
                                width: c.width,
                                height: c.height,
                                rgba: c.rgba,
                            }))
                        }
                        _ => None,
                    };
                    let douk_can = self.douk_can();
                    let dialog = self.dialog.as_ref().map(|d| d.opener);
                    let mut c = self.renderer.canvas();
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
                        Screen::Backlog(b) if self.douk_system.is_some() => {
                            let sys = self.douk_system.as_ref().unwrap();
                            let ui = &self.ui;
                            let log = &self.log;
                            let playing = self.playing_marked;
                            let hover = match ui.hover {
                                Some(Button::Screen(id)) => Some(id),
                                _ => None,
                            };
                            sys.draw_backlog(
                                &mut c,
                                assets,
                                self.screens.backlog_areas(),
                                b,
                                log,
                                |id| {
                                    let on = Screens::backlog_enabled(b, log, id);
                                    let active = playing && id == log_ids::PLAY_MARKED;
                                    ui.state(Button::Screen(id), on, active) as u32
                                },
                                hover,
                            )?;
                        }
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
                                under.as_ref(),
                            )?
                        }
                        Screen::Archive(s) => self.archive.draw(&mut c, assets, &self.ui, s)?,
                        Screen::Costume(k) => {
                            self.archive.draw_costume(&mut c, assets, &self.ui, k)?
                        }
                        Screen::DoukConfig(s) => {
                            if let Some(sys) = &self.douk_system {
                                let ui = &self.ui;
                                let (hover, state) = under_dialog(ui, dialog);
                                let (can_save, has_log) = douk_can;
                                sys.draw_config(
                                    &mut c,
                                    assets,
                                    s,
                                    &self.rt.system.settings,
                                    state,
                                    hover,
                                    (ui.skip, ui.auto, can_save, has_log),
                                    (under.as_ref(), ui.clock),
                                )?;
                            }
                        }
                        Screen::DoukSave(s) => {
                            if let Some(sys) = &self.douk_system {
                                let ui = &self.ui;
                                let (hover, state) = under_dialog(ui, dialog);
                                sys.draw_save(&mut c, assets, s, state, hover)?;
                            }
                        }
                        Screen::DoukFlag(s) => {
                            if let Some(sys) = &self.douk_system {
                                let ui = &self.ui;
                                let (hover, state) = under_dialog(ui, dialog);
                                s.draw(sys, &mut c, assets, self.rt.config.language, state, hover)?;
                            }
                        }
                        Screen::Extras(e) => self.menus.draw_extras(
                            &mut c,
                            assets,
                            &self.ui,
                            e,
                            &self.rt.system,
                            &self.vm.bytes,
                            text,
                        )?,
                    }
                    c
                }
                None => match &self.movie {
                    Some(p) => movie_frame(&p.movie, self.renderer.canvas()),
                    None => match self.ui.win_fx {
                        Some(fx) => self.window_fade_frame(fx)?,
                        None => self.game_frame()?,
                    },
                },
            };
            // A system screen fades over the scene it opened on.
            if let Some(fx) = self.ui.screen_fx.filter(|_| self.screen.is_some()) {
                let base = self.game_frame()?;
                frame = Canvas::blend(&base, &frame, fx.alpha(self.ui.clock), None);
            }
            if let Some(d) = &self.dialog {
                let font = self
                    .douk_system
                    .as_ref()
                    .and_then(|s| s.font(&mut self.rt.assets));
                self.screens
                    .draw_dialog(&mut frame, &mut self.rt.assets, &self.ui, d, font)?;
            }
            // The game's own pointer is above everything.
            self.sync_cursor();
            if self.rt.cursor.shown() {
                if let Ok(sheet) = self
                    .rt
                    .assets
                    .image(crate::games::doukyuusei::cursor::SHEET)
                {
                    self.rt
                        .cursor
                        .draw(&mut frame, &sheet, self.ui.pointer, self.ui.clock);
                }
            }
            if let Some(v) = &self.veil {
                let (w, h) = self.renderer.size();
                frame.fill(0, 0, w, h, [0, 0, 0, v.alpha(self.ui.clock)]);
            }
            self.frame = Some(if frame.ops().is_some() {
                frame
            } else {
                frame.present()
            });
        }
        Ok(self.frame.as_ref().unwrap())
    }

    /// The game while its message window fades away or back in.
    fn window_fade_frame(
        &mut self,
        fx: crate::games::doukyuusei::menu::WindowFx,
    ) -> Result<Canvas> {
        let was = self.ui.win_off;
        self.ui.win_off = true;
        let bare = self.game_frame();
        self.ui.win_off = false;
        let full = self.game_frame();
        self.ui.win_off = was;
        Ok(Canvas::blend(&bare?, &full?, fx.alpha(self.ui.clock), None))
    }

    /// The game with its interface.
    fn game_frame(&mut self) -> Result<Canvas> {
        {
            // The action menu takes the message window away.
            let window = !self.ui.window_hidden
                && self.hmode.is_none()
                && self.state != State::Title
                && self.ui.actions.is_empty()
                && self.ui.map.is_none()
                && self.ui.k1_menu.is_none()
                // The "comment unlocked" pop-up goes over the window.
                && matches!(self.ui.panel, None | Some(DoukPanel::Unlocked(_)))
                // 同級生 Remake puts its selections up without it.
                && !(!self.ui.choices.is_empty()
                    && self.rt.assets.game().title == silky_format::game::Title::Doukyuusei);
            // A system screen opened from a message has taken the window away.
            let window = window && !self.ui.win_off;
            let moved = std::mem::take(&mut self.emote_moved) || self.base_window != window;
            self.base_window = window;
            if self.base.is_none() || (moved && self.under.is_none()) {
                let under = self
                    .renderer
                    .render_under(&self.rt.scene, &mut self.rt.assets)?;
                self.base = Some(self.renderer.render_over(
                    under.clone(),
                    &self.rt.scene,
                    &mut self.rt.assets,
                    window,
                    false,
                )?);
                self.under = Some(under);
            } else if moved {
                let under = self.under.clone().expect("checked");
                self.base = Some(self.renderer.render_over(
                    under,
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
                    spin,
                    ..
                }) => {
                    let t = *elapsed as f32 / (*ms).max(1) as f32;
                    let base = self.base.as_ref().unwrap();
                    match spin {
                        // The picture leaving shrinks; the one coming grows.
                        Some(spin) => {
                            let f = (t * spin.frames as f32) as i32;
                            let (scale, turn) = spin.at(f);
                            let (mut a, mut b) = (from.clone(), base.clone());
                            if spin.grow {
                                b.spin_whole(scale, turn);
                            } else {
                                a.spin_whole(scale, turn);
                            }
                            Canvas::blend_shared(&a, &b, t, rule.clone())
                        }
                        None => Canvas::blend_shared(from, base, t, rule.clone()),
                    }
                }
                _ => self.base.clone().unwrap(),
            };
            if window {
                self.renderer
                    .draw_text(&mut frame, &self.rt.scene, self.shown_chars);
            }
            if let Some(h) = &self.hmode {
                let assets = &mut self.rt.assets;
                let img = |assets: &mut crate::Assets, n: &str| assets.image(n).ok();
                let images = [
                    img(assets, "action_p.akb"),
                    img(assets, "hg030100_hand.akb"),
                    img(assets, "hg030100_tongue.akb"),
                    img(assets, "yume.akb"),
                    img(assets, "white.akb"),
                    img(assets, "cursor.akb"),
                ];
                h.draw(&mut frame, images.each_ref().map(|i| i.as_ref()));
                return Ok(frame);
            }
            let text_shown = matches!(self.state, State::Text | State::Choice)
                && self.rt.scene.text.is_some()
                && !self.ui.win_off;
            // 同級生 Remake: the status plate is part of the covered screen,
            // so it goes dark and comes back with the fade.
            let ui_alpha = match (&self.busy, self.ui.plate.is_some()) {
                (_, false) => 1.0,
                (
                    Some(Busy {
                        ui_fade: Some(out),
                        elapsed,
                        ms,
                        ..
                    }),
                    _,
                ) => {
                    let t = (*elapsed as f32 / (*ms).max(1) as f32).clamp(0.0, 1.0);
                    match (*out, self.plate_gone) {
                        (true, true) => 0.0,
                        (true, false) => 1.0 - t,
                        (false, _) => t,
                    }
                }
                (_, true) if self.rt.scene.cover.is_some() || self.plate_gone => 0.0,
                _ => 1.0,
            };
            if self.busy.is_none() {
                self.was_covered = self.rt.scene.cover.is_some();
            }
            self.ui.jumping = self.skip_to_choice;
            if ui_alpha > 0.0 {
                let bare = (ui_alpha < 1.0).then(|| frame.clone());
                self.layout.draw(
                    &mut frame,
                    &mut self.rt.assets,
                    &self.ui,
                    text_shown,
                    self.renderer.text(),
                )?;
                if let Some(bare) = bare {
                    frame = Canvas::blend_shared(&bare, &frame, ui_alpha, None);
                }
            }
            Ok(frame)
        }
    }
}

/// The movie picture scaled to fit the screen, on black.
fn movie_frame(m: &Movie, mut c: Canvas) -> Canvas {
    c.fill(0, 0, c.width, c.height, [0, 0, 0, 255]);
    let img = Arc::new(Image {
        width: m.width,
        height: m.height,
        rgba: m.rgba.clone(),
    });
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
/// How a 同級生 Remake screen draws its elements: the hint's element and
/// the row of each. While a dialog is up (`dialog` holds its opener) the
/// opener shows pressed with its hint and the rest sit normal, no matter
/// where the pointer is.
fn under_dialog(
    ui: &Ui,
    dialog: Option<Option<Button>>,
) -> (Option<i32>, impl Fn(i32) -> u32 + '_) {
    let id_of = |b: Option<Button>| match b {
        Some(Button::Screen(id)) => Some(id),
        _ => None,
    };
    let hover = match dialog {
        Some(opener) => id_of(opener),
        None => id_of(ui.hover),
    };
    let state = move |id: i32| match dialog {
        Some(opener) => 2 * (id_of(opener) == Some(id)) as u32,
        None => ui.state(Button::Screen(id), true, false) as u32,
    };
    (hover, state)
}
