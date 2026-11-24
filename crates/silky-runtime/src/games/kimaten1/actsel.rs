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

#[derive(Debug, Clone)]
pub struct ActSelect {
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

    fn slot(&self, k: usize) -> Option<&Area> {
        self.areas.iter().find(|a| a.id == k as i32)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
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
            if !self.tutorial_seen {
                self.tutorial_seen = true;
                self.bytes.push((TUTORIAL_SEEN, 1));
            }
            return Click::Result(SKIP_ANSWER);
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
        Ok(())
    }

    /// Headless run: like a player who wants the story to go on. For each
    /// state of the story counters (bytes 100 to 259) the kind of button
    /// pressed least is pressed, lowest kind first, up to a few times each
    /// (a kind may open a list); then the skip button, else the last kind.
    pub fn auto_pick(&self) -> Option<i32> {
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
