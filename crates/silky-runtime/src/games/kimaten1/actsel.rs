//! The action menu (`Game.42`): up to four large buttons (look around,
//! talk, move, present evidence), a skip button at the lower right once the
//! tutorial is unlocked, and the system buttons along the bottom.
//!
//! The script fills bytes 0 to 3 before the call: a non-zero byte turns that
//! kind of button on. The buttons on show are packed into the first
//! positions of `actselect.area` in kind order, and a press answers the
//! kind plus one (1 look, 2 talk, 3 move, 4 present). The skip button
//! answers 100.

use std::collections::HashMap;
use std::sync::Mutex;

use silky_format::area::Area;

use super::chrome::{self, BAR_LEN};
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
/// Help line of each kind of button.
const HELP: [&str; 4] = [
    "部屋の中を調べる",
    "人物と会話する",
    "他の場所へ移動する",
    "証拠品をつきつける",
];
const SKIP_HELP: &str = "行動選択をスキップします";
/// Click sound of the interface.
const CLICK: &str = "ses_001.wav";

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

#[derive(Debug, Clone)]
pub struct ActSelect {
    popup: Popup,
    tutorial_areas: Vec<Area>,
    dialog_areas: Vec<Area>,
    areas: Vec<Area>,
    /// Kinds of the buttons on show, in order.
    kinds: Vec<u8>,
    /// Whether the skip button is on.
    skip: bool,
    tutorial_seen: bool,
    /// The system buttons that are grey.
    off: [bool; BAR_LEN],
    bytes: Vec<(usize, u8)>,
    sound: Option<&'static str>,
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
            tutorial_areas: Vec::new(),
            dialog_areas: Vec::new(),
            areas,
            kinds: (0..4u8).filter(|&k| flags[k as usize] != 0).collect(),
            skip: skip && unlocked,
            tutorial_seen,
            off,
            bytes: Vec::new(),
            sound: None,
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
                return self
                    .tutorial_areas
                    .iter()
                    .find(|a| a.id == 0 && a.contains(x, y))
                    .map(|_| TUTORIAL_OK);
            }
            Popup::Confirm => {
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
                if id == TUTORIAL_OK {
                    self.sound = Some(CLICK);
                    self.popup = Popup::Confirm;
                }
                return Click::None;
            }
            Popup::Confirm => {
                self.sound = Some(CLICK);
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
            self.popup = if self.tutorial_seen {
                Popup::Confirm
            } else {
                self.tutorial_seen = true;
                self.bytes.push((TUTORIAL_SEEN, 1));
                Popup::Tutorial
            };
            return Click::None;
        }
        Click::None
    }

    /// A right click: answers no to the question.
    pub fn cancel(&mut self) -> Click {
        if self.popup == Popup::Confirm {
            self.popup = Popup::None;
        }
        Click::None
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
            if look.hover == Some(k as i32) {
                help = Some(HELP[kind as usize]);
            }
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
        canvas.draw_part(&page, (0, 0, 1280, 720), 0, 0, false, 255);
        let ok = assets.image("OKbotton.akb")?;
        if let Some(a) = self.tutorial_areas.iter().find(|a| a.id == 0) {
            let row = chrome::state(look, TUTORIAL_OK);
            canvas.draw_part(&ok, (0, 44 * row, 102, 44), a.left, a.top, false, 255);
        }
        Ok(())
    }

    /// The question whether to skip the action selection, with yes and no.
    fn draw_confirm(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image("dialog_p.akb")?;
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
    fn the_first_skip_shows_the_tutorial_then_asks() {
        let mut m = menu(false);
        assert_eq!(m.click(SKIP), Click::None);
        assert_eq!(m.take_bytes(), vec![(TUTORIAL_SEEN, 1)]);
        assert_eq!(m.hit(1050, 580), Some(TUTORIAL_OK));
        assert_eq!(m.hit(30, 30), None);
        assert_eq!(m.click(TUTORIAL_OK), Click::None);
        assert_eq!(m.hit(500, 300), Some(YES));
        assert_eq!(m.click(YES), Click::Result(SKIP_ANSWER));
    }

    #[test]
    fn later_skips_only_ask_and_no_goes_back() {
        let mut m = menu(true);
        assert_eq!(m.click(SKIP), Click::None);
        assert!(m.take_bytes().is_empty());
        assert_eq!(m.hit(700, 300), Some(NO));
        assert_eq!(m.click(NO), Click::None);
        assert_eq!(m.hit(30, 30), Some(SKIP));
        m.click(SKIP);
        m.cancel();
        assert_eq!(m.hit(30, 30), Some(SKIP));
    }
}
