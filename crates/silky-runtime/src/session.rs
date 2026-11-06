//! A running game for an interactive frontend: interpreter, runtime,
//! renderer and interface state together, driven by pointer events and a
//! clock. The desktop player and a mobile embedding only forward input and
//! present [`Session::frame`].

use silky_format::game::Game;
use silky_format::mes::Profile;
use silky_vm::{Stop, Vm};

use crate::render::{Canvas, Renderer};
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
        };
        s.restart()?;
        Ok(s)
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// Starts the game again from its first script.
    pub fn restart(&mut self) -> Result<()> {
        self.vm = Vm::new(Profile::default());
        self.rt.reset();
        self.rt.boot(&mut self.vm, &self.start)?;
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
        let text_shown = self.state != State::Title && self.rt.scene.text.is_some();
        self.layout.hit(&self.ui, text_shown, x, y)
    }

    /// Advances the clock by `ms`: animation, auto and skip modes. Returns
    /// whether the frame changed.
    pub fn tick(&mut self, ms: u64) -> Result<bool> {
        let before = self.ui.clock;
        self.ui.clock += ms;
        self.shown_ms += ms;
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
            let mut frame = self.base.clone().unwrap();
            let text_shown = self.state != State::Title && self.rt.scene.text.is_some();
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
