//! A running game for an interactive frontend: interpreter, runtime,
//! renderer and interface state together, driven by pointer events and a
//! clock. The desktop player and a mobile embedding only forward input and
//! present [`Session::frame`].

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use silky_format::akb::Image;

use silky_format::game::Game;
use silky_format::mes::Profile;
use silky_vm::{Stop, Vm};

use crate::render::{Canvas, Renderer};
use crate::save::{self, Saves};
use crate::screens::{
    dialog_ids, log_ids, save_ids, Action, Backlog, Dialog, Item, LogEntry, Mode, SaveLoad, Screen,
    Screens, LOG_ROWS, SAVE_ROWS,
};
use crate::text::TextRenderer;
use crate::ui::{bar, title, Button, Layout, Ui};
use crate::{Assets, Config, Error, Pause, Result, Runtime, SaveState};

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
    /// The scripts ended, or the player asked to quit.
    Ended,
}

/// Instructions per [`Vm::run`] call, and calls before giving up on a
/// script that never stops.
const STEP_BUDGET: u64 = 50_000_000;
const MAX_BUDGETS: u32 = 20;
/// Skip mode: time per message.
const SKIP_MS: u64 = 60;
/// Auto mode: time per message, plus per character.
const AUTO_BASE_MS: u64 = 1500;
const AUTO_CHAR_MS: u64 = 60;
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
    /// Questions the player asked not to see again.
    dont_ask: HashSet<usize>,
    /// Thumbnail of the game as it was when a screen opened.
    thumb: Option<Image>,
    /// The backlog plays the marked voices one after another.
    playing_marked: bool,
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
        let renderer = Renderer::new(text, config.language);
        let rt = Runtime::new(assets, config);
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
            dont_ask: HashSet::new(),
            thumb: None,
            playing_marked: false,
        };
        s.restart()?;
        Ok(s)
    }

    /// Where to keep save files. Without one, saving does nothing.
    pub fn set_saves(&mut self, saves: Saves) {
        self.ui.has_saves = !saves.list().is_empty();
        self.saves = Some(saves);
        self.frame = None;
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
        self.restore(file.state)
    }

    /// Puts back a game state and shows its message.
    fn restore(&mut self, state: SaveState) -> Result<()> {
        let kept = state.clone();
        self.rt.load_state(&mut self.vm, state)?;
        if self.log.is_empty() {
            if let Some(text) = &self.rt.scene.text {
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
        self.base = None;
        self.frame = None;
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
        self.voice_queue.clear();
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
                        break;
                    }
                    Some(Pause::Title) => {
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
                        self.auto_save();
                        self.ui.choices = self.rt.choices.clone();
                        self.ui.waiting = false;
                        self.state = State::Choice;
                        self.ui.skip = false;
                        self.ui.auto = false;
                        break;
                    }
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
        self.base = None;
        self.frame = None;
        Ok(())
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
        if self.state == State::Text {
            self.ui.waiting = false;
            self.run()?;
        }
        Ok(())
    }

    /// Pointer moved to (x, y) in screen pixels.
    pub fn pointer_move(&mut self, x: i32, y: i32) {
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
    }

    /// Primary button released at (x, y): a click.
    pub fn pointer_up(&mut self, x: i32, y: i32) -> Result<()> {
        let pressed = self.ui.pressed.take();
        self.frame = None;
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
                    Ok(())
                } else {
                    self.advance()
                }
            }
            None => Ok(()),
        }
    }

    /// Secondary click: closes a dialog or screen, else hides or shows
    /// the message window.
    pub fn secondary_click(&mut self) {
        if self.back() {
            return;
        }
        if self.state == State::Text {
            self.ui.window_hidden = !self.ui.window_hidden;
            self.base = None;
            self.frame = None;
        }
    }

    /// Closes the dialog or screen that is up (Escape, right click).
    /// Returns false if there was none.
    pub fn back(&mut self) -> bool {
        if self.dialog.take().is_some() {
            self.frame = None;
            return true;
        }
        if self.screen.is_some() {
            self.close_screen();
            return true;
        }
        false
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
        if self.dont_ask.contains(&action.question()) {
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
                        self.dont_ask.insert(d.action.question());
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
                    self.log.truncate(i + 1);
                    self.restore(state)?;
                }
            }
            Action::Title => self.restart()?,
            Action::Quit => self.state = State::Ended,
        }
        Ok(())
    }

    fn press_screen(&mut self, id: i32, x: i32, y: i32) -> Result<()> {
        self.frame = None;
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
            None => {}
        }
        if close {
            self.close_screen();
        }
        if let Some(action) = ask {
            self.ask(action)?;
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
                    self.ui.auto = false;
                }
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
            None => {}
        }
        if self.state == State::Busy {
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
        if self.screen.is_some() || self.dialog.is_some() {
            return Ok(self.frame.is_none());
        }
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
        if self.state == State::Text && !self.ui.window_hidden {
            let chars = self
                .rt
                .scene
                .text
                .as_deref()
                .map_or(0, |t| t.chars().count()) as u64;
            let due = if self.ui.skip {
                Some(SKIP_MS)
            } else if self.ui.auto {
                Some(AUTO_BASE_MS + AUTO_CHAR_MS * chars)
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
                    }
                    c
                }
                None => self.game_frame()?,
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
            if self.base.is_none() {
                let window = !self.ui.window_hidden;
                self.base = Some(self.renderer.render_scene(
                    &self.rt.scene,
                    &mut self.rt.assets,
                    window && self.state != State::Title,
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

fn vm_error(e: silky_vm::Error) -> Error {
    Error::Invalid(e.to_string())
}
