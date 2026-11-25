//! The action menu (`Game.42`): up to four large buttons (look around,
//! talk, move, present evidence), a skip button at the lower right once the
//! tutorial is unlocked, and the system buttons along the bottom.
//!
//! The script fills bytes 0 to 3 before the call: a non-zero byte turns that
//! kind of button on. The buttons on show are packed into the first
//! positions of `actselect.area` in kind order, and a press answers the
//! kind plus one (1 look, 2 talk, 3 move, 4 present). The skip button
//! answers 100.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;

use silky_format::area::Area;

use super::chrome::{self, BAR_LEN};
use super::fade::Fade;
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// Id of the skip button.
pub const SKIP: i32 = 0x100;
/// What the skip button answers.
const SKIP_ANSWER: i32 = 100;
/// Byte set once the tutorial has been offered.
const TUTORIAL_SEEN: usize = 0xbc0;
/// Ids of the skip tutorial's OK button and of the confirmation's buttons.
const TUTORIAL_OK: i32 = 0x200;
const YES: i32 = 0xd00;
const NO: i32 = 0xd01;
/// Row of the confirmation's text in `dialog_p.akb` (the skip question).
const CONFIRM_ROW: u32 = 5;
/// Size of a button picture.
const CELL: (u32, u32) = (380, 134);
/// Row of the first button picture in the sheet.
const CELL_TOP: u32 = 152;
const SKIP_HELP: &str = "行動選択をスキップします";
/// Click sound of the interface, and the one of the question's no button.
const CLICK: &str = "ses_001.wav";
const CANCEL: &str = "ses_002.wav";
/// Played when the skip tutorial opens. Its OK button makes no sound of
/// its own.
const TUTORIAL_SOUND: &str = "ses_009.wav";
/// The tutorial fades in and out over 8 frames of 1/60 s, the question
/// fades in over 12 (it goes away at once). Both are waited for inside
/// the original's own calls, so neither takes a press meanwhile.
const FADE_FRAMES: u64 = 8;
const FADE_MS: u64 = FADE_FRAMES * 1000 / 60;
const CONFIRM_FRAMES: u64 = 12;
const CONFIRM_MS: u64 = CONFIRM_FRAMES * 1000 / 60;

/// The story counters the scripts keep from byte 100 on (and the flags of
/// the scene from 0 to 3); the headless run
/// reads them to tell whether something it pressed moved the story.
pub const PROGRESS_FIRST: usize = 100;
pub const PROGRESS_LEN: usize = 160;

/// How often the headless run pressed a kind: (counters then, kind). A
/// kind can open a list of several entries, so it is pressed up to this
/// many times for one state of the story.
type Tried = HashMap<([u8; PROGRESS_LEN], u8), u8>;
static TRIED: Mutex<Option<Tried>> = Mutex::new(None);
const PRESSES: u8 = 5;

/// What is on top of the menu: the skip tutorial the first time the skip
/// button is used, then the question whether to skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Popup {
    None,
    Tutorial,
    Confirm,
}

/// Where the skip tutorial is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    In,
    Shown,
    Out,
}

/// Row of the tutorial's OK button picture. The original shows the pressed
/// row while the button is held and draws the resting row again before
/// the page fades out, whatever the pointer does then.
fn ok_row(phase: Phase, look: &Look) -> u32 {
    if phase == Phase::Out {
        0
    } else {
        chrome::state(look, TUTORIAL_OK)
    }
}

/// Draws with `draw` and mixes the result over what was there at `alpha`.
fn faded(
    canvas: &mut Canvas,
    alpha: u8,
    draw: impl FnOnce(&mut Canvas) -> Result<()>,
) -> Result<()> {
    if alpha == 255 {
        return draw(canvas);
    }
    let under = canvas.clone();
    draw(canvas)?;
    *canvas = Canvas::blend(&under, canvas, alpha as f32 / 255.0, None);
    Ok(())
}

/// Opacity of a fade of `total` frames, `age` ms in: it gains (or loses)
/// an equal step each frame.
fn fade_alpha(age: u64, out: bool, total: u64) -> u8 {
    let frames = (age * 60 / 1000 + 1).min(total);
    let a = (255 * frames / total) as u8;
    if out {
        255 - a
    } else {
        a
    }
}

#[derive(Debug, Clone)]
pub struct ActSelect {
    popup: Popup,
    phase: Phase,
    /// The screen's clock when the phase was first seen.
    phase_at: Cell<Option<u64>>,
    tutorial_areas: Vec<Area>,
    dialog_areas: Vec<Area>,
    areas: Vec<Area>,
    /// Kinds of the buttons on show, in order.
    kinds: Vec<u8>,
    /// Whether the skip button is on.
    skip: bool,
    tutorial_seen: bool,
    /// The system buttons that are grey.
    pub(super) off: [bool; BAR_LEN],
    bytes: Vec<(usize, u8)>,
    sound: Option<&'static str>,
    pub(super) fade: Fade,
    progress: [u8; PROGRESS_LEN],
}

impl ActSelect {
    /// `flags` are bytes 0 to 3, `skip` is the call's argument (1 shows the
    /// skip button, once `unlocked`), `notebook` whether the notebook has
    /// been unlocked.
    pub fn open(
        areas: Vec<Area>,
        flags: [u8; 4],
        skip: bool,
        unlocked: bool,
        tutorial_seen: bool,
        notebook: bool,
        progress: [u8; PROGRESS_LEN],
    ) -> Self {
        let mut off = [false; BAR_LEN];
        // The notebook needs its flag.
        off[7] = !notebook;
        Self {
            popup: Popup::None,
            phase: Phase::In,
            phase_at: Cell::new(None),
            tutorial_areas: Vec::new(),
            dialog_areas: Vec::new(),
            areas,
            kinds: (0..4u8).filter(|&k| flags[k as usize] != 0).collect(),
            skip: skip && unlocked,
            tutorial_seen,
            off,
            bytes: Vec::new(),
            sound: None,
            fade: Fade::default(),
            progress,
        }
    }

    /// The areas of the skip tutorial and of the confirmation dialog.
    pub fn with_popups(mut self, tutorial: Vec<Area>, dialog: Vec<Area>) -> Self {
        self.tutorial_areas = tutorial;
        self.dialog_areas = dialog;
        self
    }

    fn slot(&self, k: usize) -> Option<&Area> {
        self.areas.iter().find(|a| a.id == k as i32)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        match self.popup {
            Popup::Tutorial => {
                if self.phase != Phase::Shown {
                    return None;
                }
                return self
                    .tutorial_areas
                    .iter()
                    .find(|a| a.id == 0 && a.contains(x, y))
                    .map(|_| TUTORIAL_OK);
            }
            Popup::Confirm => {
                if self.phase != Phase::Shown {
                    return None;
                }
                return self
                    .dialog_areas
                    .iter()
                    .find(|a| a.id < 2 && a.contains(x, y))
                    .map(|a| YES + a.id);
            }
            Popup::None => {}
        }
        chrome::hit_area(&self.areas, x, y, |id| {
            id == SKIP && self.skip
                || chrome::bar_index(id).is_some()
                || (0..self.kinds.len() as i32).contains(&id)
        })
        .map(|a| a.id)
    }

    pub fn enabled(&self, id: i32) -> bool {
        chrome::bar_index(id).map_or(true, |i| !self.off[i])
    }

    pub fn click(&mut self, id: i32) -> Click {
        match self.popup {
            Popup::Tutorial => {
                // The press closes the page: it fades out, then the
                // question comes.
                if id == TUTORIAL_OK && self.phase == Phase::Shown {
                    self.phase = Phase::Out;
                    self.phase_at.set(None);
                }
                return Click::None;
            }
            Popup::Confirm => {
                self.sound = Some(if id == YES { CLICK } else { CANCEL });
                self.popup = Popup::None;
                // Yes answers the skip; no goes back to the menu.
                return if id == YES {
                    Click::Result(SKIP_ANSWER)
                } else {
                    Click::None
                };
            }
            Popup::None => {}
        }
        if let Some(c) = chrome::bar_click(id) {
            return c;
        }
        if let Some(&kind) = usize::try_from(id).ok().and_then(|k| self.kinds.get(k)) {
            *TRIED
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .entry((self.progress, kind))
                .or_default() += 1;
            self.sound = Some(CLICK);
            return Click::Result(kind as i32 + 1);
        }
        if id == SKIP && self.skip {
            self.sound = Some(CLICK);
            // The first use shows the tutorial; the question follows it.
            self.phase = Phase::In;
            self.phase_at.set(None);
            self.popup = if self.tutorial_seen {
                Popup::Confirm
            } else {
                self.tutorial_seen = true;
                self.bytes.push((TUTORIAL_SEEN, 1));
                // The page's own sound takes the place of the click.
                self.sound = Some(TUTORIAL_SOUND);
                Popup::Tutorial
            };
            return Click::None;
        }
        Click::None
    }

    /// Milliseconds since the tutorial's phase began.
    fn age(&self, ms: u64) -> u64 {
        let at = self.phase_at.get().unwrap_or_else(|| {
            self.phase_at.set(Some(ms));
            ms
        });
        ms.saturating_sub(at)
    }

    /// Whether the tutorial is fading in or out at `ms`.
    pub fn animating(&self, ms: u64) -> bool {
        let _ = ms;
        self.popup != Popup::None && self.phase != Phase::Shown
    }

    /// How long the popup's fade in or out lasts.
    fn fade_ms(&self) -> u64 {
        if self.popup == Popup::Confirm {
            CONFIRM_MS
        } else {
            FADE_MS
        }
    }

    /// Runs the tutorial's fades: once the fade out is over the question
    /// follows.
    pub fn tick(&mut self, ms: u64) -> Option<Click> {
        if self.popup == Popup::None || self.phase == Phase::Shown {
            return None;
        }
        if self.age(ms) < self.fade_ms() {
            return None;
        }
        self.advance()
    }

    /// For a run without time: ends the fade under way at once.
    pub fn settle(&mut self) -> Option<Click> {
        if self.popup != Popup::None && self.phase != Phase::Shown {
            return self.advance();
        }
        None
    }

    fn advance(&mut self) -> Option<Click> {
        self.phase_at.set(None);
        match self.phase {
            Phase::In => self.phase = Phase::Shown,
            Phase::Out => {
                self.popup = Popup::Confirm;
                self.phase = Phase::In;
            }
            Phase::Shown => return None,
        }
        Some(Click::None)
    }

    /// A right click: answers no to the question.
    pub fn cancel(&mut self) -> Click {
        if self.popup == Popup::Confirm && self.phase == Phase::Shown {
            self.popup = Popup::None;
        } else if self.popup == Popup::None && !self.fade.leaving() {
            // Mode 0 of the right-click setting: the buttons go away with
            // the cancel sound, and come back with the next click.
            if !self.fade.hidden() {
                self.sound = Some(CANCEL);
            }
            self.fade.toggle_hidden();
        }
        Click::None
    }

    /// Whether a right click goes by the right-click setting (no question
    /// or page is up and the screen is not leaving).
    pub fn follows_setting(&self) -> bool {
        self.popup == Popup::None && !self.fade.leaving()
    }

    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        std::mem::take(&mut self.bytes)
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = chrome::draw_common(canvas, assets, look, &self.off)?;
        // Title at the upper right.
        canvas.draw_part(&sheet, (1143, 754, 419, 100), 860, 29, false, 255);
        if self.skip {
            let row = chrome::state(look, SKIP);
            if let Some(a) = self.areas.iter().find(|a| a.id == SKIP) {
                canvas.draw_part(&sheet, (1093, 38 * row, 52, 38), a.left, a.top, false, 255);
            }
        }
        // Pointing at a button of the menu leaves the information line empty;
        // only the skip button and the system buttons write to it.
        let mut help = chrome::bar_help(look);
        for (k, &kind) in self.kinds.iter().enumerate() {
            let Some(a) = self.slot(k) else {
                continue;
            };
            let row = chrome::state(look, k as i32);
            canvas.draw_part(
                &sheet,
                (
                    CELL.0 * kind as u32,
                    CELL_TOP + CELL.1 * row,
                    CELL.0,
                    CELL.1,
                ),
                a.left,
                a.top,
                false,
                255,
            );
        }
        if look.hover == Some(SKIP) {
            help = Some(SKIP_HELP);
        }
        chrome::draw_help(canvas, look, help);
        match self.popup {
            Popup::Tutorial => self.draw_tutorial(canvas, assets, look)?,
            Popup::Confirm => self.draw_confirm(canvas, assets, look)?,
            Popup::None => {}
        }
        Ok(())
    }

    /// The skip tutorial: its picture over the whole screen and an OK
    /// button at the lower right.
    fn draw_tutorial(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let page = assets.image("moveskip_tutorial.akb")?;
        let ok = assets.image("OKbotton.akb")?;
        let alpha = match self.phase {
            Phase::In => fade_alpha(self.age(look.ms), false, FADE_FRAMES),
            Phase::Shown => 255,
            Phase::Out => fade_alpha(self.age(look.ms), true, FADE_FRAMES),
        };
        // The button is part of the page, so both fade together.
        faded(canvas, alpha, |canvas| {
            canvas.draw_part(&page, (0, 0, 1280, 720), 0, 0, false, 255);
            if let Some(a) = self.tutorial_areas.iter().find(|a| a.id == 0) {
                let row = ok_row(self.phase, look);
                canvas.draw_part(&ok, (0, 44 * row, 102, 44), a.left, a.top, false, 255);
            }
            Ok(())
        })
    }

    /// The question whether to skip the action selection, with yes and no.
    fn draw_confirm(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image("dialog_p.akb")?;
        let alpha = match self.phase {
            Phase::In => fade_alpha(self.age(look.ms), false, CONFIRM_FRAMES),
            _ => 255,
        };
        // The dim layer and the dialog fade in together.
        faded(canvas, alpha, |canvas| {
            self.draw_confirm_now(canvas, &sheet, look)
        })
    }

    fn draw_confirm_now(
        &self,
        canvas: &mut Canvas,
        sheet: &std::sync::Arc<silky_format::akb::Image>,
        look: &Look,
    ) -> Result<()> {
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 127]);
        canvas.draw_part(&sheet, (0, 0, 605, 276), 338, 115, false, 255);
        canvas.draw_part(&sheet, (206, 248, 123, 19), 683, 352, false, 255);
        canvas.draw_part(&sheet, (878, 200, 380, 29), 450, 215, false, 255);
        canvas.draw_part(
            &sheet,
            (878, 229 + 51 * CONFIRM_ROW, 380, 51),
            450,
            244,
            false,
            255,
        );
        let yes = 200 + 51 * chrome::state(look, YES);
        let no = 200 + 51 * chrome::state(look, NO);
        canvas.draw_part(&sheet, (605, yes, 94, 51), 494, 295, false, 255);
        canvas.draw_part(&sheet, (699, no, 94, 51), 694, 295, false, 255);
        Ok(())
    }

    /// Headless run: like a player who wants the story to go on. For each
    /// state of the story counters (bytes 100 to 259) the kind of button
    /// pressed least is pressed, lowest kind first, up to a few times each
    /// (a kind may open a list); then the skip button, else the last kind.
    pub fn auto_pick(&self) -> Option<i32> {
        match self.popup {
            Popup::Tutorial => return Some(TUTORIAL_OK),
            Popup::Confirm => return Some(YES),
            Popup::None => {}
        }
        let tried = TRIED.lock().unwrap();
        let count = |k: usize| {
            tried
                .as_ref()
                .and_then(|t| t.get(&(self.progress, self.kinds[k])))
                .copied()
                .unwrap_or(0)
        };
        let least = (0..self.kinds.len())
            .filter(|&k| count(k) < PRESSES)
            .min_by_key(|&k| (count(k), k));
        match least {
            Some(k) => Some(k as i32),
            None if self.skip => Some(SKIP),
            None => self.kinds.len().checked_sub(1).map(|k| k as i32),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(id: i32, l: i32, t: i32, r: i32, b: i32) -> Area {
        Area {
            id,
            left: l,
            top: t,
            right: r,
            bottom: b,
        }
    }

    fn menu(seen: bool) -> ActSelect {
        ActSelect::open(
            vec![area(SKIP, 10, 10, 60, 50)],
            [1, 0, 0, 0],
            true,
            true,
            seen,
            true,
            [0; PROGRESS_LEN],
        )
        .with_popups(
            vec![area(0, 1028, 566, 1130, 610)],
            vec![area(0, 494, 295, 588, 346), area(1, 694, 295, 788, 346)],
        )
    }

    #[test]
    fn right_click_hides_the_buttons_and_brings_them_back() {
        let mut m = menu(true);
        assert!(m.follows_setting());
        m.cancel();
        assert!(m.fade.hidden());
        assert_eq!(m.take_sound().as_deref(), Some(CANCEL));
        m.cancel();
        assert!(!m.fade.hidden());
        assert_eq!(m.take_sound(), None);
    }

    #[test]
    fn right_click_during_a_question_only_closes_it() {
        let mut m = menu(true);
        m.click(SKIP);
        m.phase = Phase::Shown;
        assert!(!m.follows_setting());
        m.cancel();
        assert!(!m.fade.hidden());
    }

    #[test]
    fn the_first_skip_shows_the_tutorial_then_asks() {
        let mut m = menu(false);
        assert_eq!(m.click(SKIP), Click::None);
        assert_eq!(m.take_bytes(), vec![(TUTORIAL_SEEN, 1)]);
        assert_eq!(m.take_sound().as_deref(), Some(TUTORIAL_SOUND));
        // It fades in first and takes no press meanwhile.
        assert_eq!(m.hit(1050, 580), None);
        assert!(m.animating(0));
        assert_eq!(m.tick(0), None);
        assert_eq!(m.tick(FADE_MS), Some(Click::None));
        assert_eq!(m.hit(1050, 580), Some(TUTORIAL_OK));
        assert_eq!(m.hit(30, 30), None);
        assert_eq!(m.click(TUTORIAL_OK), Click::None);
        // No click sound; the page fades out, then the question comes.
        assert_eq!(m.take_sound(), None);
        assert_eq!(m.hit(500, 300), None);
        assert_eq!(m.tick(FADE_MS + 10), None);
        assert_eq!(m.tick(2 * FADE_MS + 10), Some(Click::None));
        // The question fades in over 12 frames and takes no press meanwhile.
        assert!(m.animating(0));
        assert_eq!(m.hit(500, 300), None);
        let t = 2 * FADE_MS + 20;
        assert_eq!(m.tick(t), None);
        assert_eq!(m.tick(t + CONFIRM_MS - 1), None);
        assert_eq!(m.tick(t + CONFIRM_MS), Some(Click::None));
        assert!(!m.animating(0));
        assert_eq!(m.hit(500, 300), Some(YES));
        assert_eq!(m.click(YES), Click::Result(SKIP_ANSWER));
    }

    #[test]
    fn later_skips_only_ask_and_no_goes_back() {
        let mut m = menu(true);
        assert_eq!(m.click(SKIP), Click::None);
        assert!(m.take_bytes().is_empty());
        assert_eq!(m.hit(700, 300), None);
        // Escape does not answer while it fades in either.
        m.cancel();
        assert!(m.animating(0));
        m.settle();
        assert_eq!(m.hit(700, 300), Some(NO));
        assert_eq!(m.click(NO), Click::None);
        assert_eq!(m.hit(30, 30), Some(SKIP));
        m.click(SKIP);
        m.settle();
        m.cancel();
        assert_eq!(m.hit(30, 30), Some(SKIP));
    }

    #[test]
    fn the_ok_button_rests_while_the_page_fades_out() {
        let look = |hover, pressed| Look {
            hover,
            pressed,
            ms: 0,
            text: None,
        };
        assert_eq!(ok_row(Phase::Shown, &look(None, None)), 0);
        assert_eq!(ok_row(Phase::Shown, &look(Some(TUTORIAL_OK), None)), 1);
        assert_eq!(
            ok_row(Phase::Shown, &look(Some(TUTORIAL_OK), Some(TUTORIAL_OK))),
            2
        );
        assert_eq!(ok_row(Phase::Out, &look(Some(TUTORIAL_OK), None)), 0);
    }

    #[test]
    fn fades_gain_an_equal_step_a_frame() {
        assert_eq!(fade_alpha(0, false, 12), 21);
        assert_eq!(fade_alpha(1000, false, 12), 255);
        assert_eq!(fade_alpha(1000, true, 12), 0);
    }
}
