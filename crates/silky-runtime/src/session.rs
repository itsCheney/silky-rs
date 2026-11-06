//! A running game for an interactive frontend: interpreter, runtime,
//! renderer and interface state together, driven by pointer events and a
//! clock. The desktop player and a mobile embedding only forward input and
//! present [`Session::frame`].

use std::collections::HashMap;
use std::sync::Arc;

use silky_format::game::Game;
use silky_format::mes::Profile;
use silky_vm::{Stop, Vm};

use crate::render::{Canvas, Renderer};
use crate::save::{self, Saves};
use crate::text::TextRenderer;
use crate::ui::{bar, title, Button, Layout, Ui};
use crate::{Assets, Config, Error, Pause, Result, Runtime};

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
        saves.write(slot, self.rt.save_state(&self.vm))?;
        self.ui.has_saves = true;
        Ok(())
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
        self.rt.load_state(&mut self.vm, file.state)?;
        self.busy = None;
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

    /// Secondary click: hides or shows the message window.
    pub fn secondary_click(&mut self) {
        if self.state == State::Text {
            self.ui.window_hidden = !self.ui.window_hidden;
            self.base = None;
            self.frame = None;
        }
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
                // No save screen yet: both load the newest save.
                title::CONTINUE | title::LOAD => self.load_latest()?,
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
                bar::TITLE => self.restart()?,
                bar::QUICK_SAVE => self.save(save::QUICK)?,
                bar::QUICK_LOAD => self.load(save::QUICK)?,
                // No save screen yet: Save takes a new slot, Load the
                // newest save.
                bar::SAVE => {
                    if let Some(slot) = self.saves.as_ref().map(Saves::next_slot) {
                        self.save(&slot)?;
                    }
                }
                bar::LOAD => self.load_latest()?,
                bar::QUIT => self.state = State::Ended,
                id => self
                    .rt
                    .note_unhandled(format!("message window button {id}")),
            },
        }
        self.frame = None;
        Ok(())
    }

    fn hit(&self, x: i32, y: i32) -> Option<Button> {
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
            self.frame = Some(frame);
        }
        Ok(self.frame.as_ref().unwrap())
    }
}

fn vm_error(e: silky_vm::Error) -> Error {
    Error::Invalid(e.to_string())
}
