//! The option list of the talk and look menus (`Game.45`).
//!
//! The options the script registered come as a list of bars on the
//! 682 x 80 bar picture of `message_p.akb` (three rows: rest, pointer over,
//! pressed), four to a page, with the label centred in each. A page after
//! the first has a bar with a up triangle above its bars and a page before
//! the last has one with a down triangle below them. A press on a bar
//! answers the number the script gave that option, plus one; the back button
//! and right click answer 100. The bottom strips and system buttons are
//! the ones of the other investigation screens, and the help line stays
//! empty over the bars.
//!
//! The labels are the strings the option bodies leave; a conditional option
//! list comes as several runs of options before the selection, and the
//! numbers are those of the options as written in the script.
//!
//! An option is on while the byte of the raw area at 7500 plus its number
//! is 1 (the script's option wrapper sets all of them before it opens the
//! list, and the options that are not available clear theirs). One with another
//! value is drawn in grey text and does nothing: it takes no hover, plays no
//! click sound and answers nothing. The text of a bar is made once, so a
//! pressed bar shows the same text in the same place as a bar at rest.

use std::collections::HashMap;
use std::sync::Mutex;

use silky_format::area::Area;

use super::chrome::{self, BAR_LEN};
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// Id of the back button.
pub const BACK: i32 = 0x100;
/// Ids of the page bars.
const UP: i32 = 4;
const DOWN: i32 = 5;
const BACK_ANSWER: i32 = 100;
const SHEET: &str = "message_p.akb";
/// Row of the first bar picture in the sheet, and its size.
const CELL_TOP: u32 = 440;
const CELL: (i32, i32) = (682, 80);
/// Distance between the tops of two bars.
const PITCH: i32 = 88;
const PER_PAGE: usize = 4;
const TEXT_SIZE: f32 = 24.0;
/// Where the text sits in a bar: top of the glyph row, and the baseline
/// under it (the same offset the selection bars of the message window use,
/// checked under Wine there).
const TEXT_TOP: i32 = 28;
const TEXT_BASELINE: f32 = (TEXT_TOP + 20) as f32;
const CLICK: &str = "ses_001.wav";
/// First of the raw-area bytes that switch the options on, one for each
/// number.
pub const FLAG_FIRST: usize = 7500;
/// Text colour of an option that is off.
const OFF_TEXT: [u8; 4] = [128, 128, 128, 255];
/// The system buttons this screen can grey besides the notebook's.
const BACKLOG_BUTTON: usize = 2;
const LOAD_BUTTON: usize = 4;
/// The notebook's byte greys this button on this screen (the original
/// numbers it so; the notebook's own button is the last one).
const NOTEBOOK_BUTTON: usize = 5;

/// How often the headless run picked an option: (the list, the option's
/// number).
static TRIED: Mutex<Option<HashMap<(u64, i32), u32>>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct ExSelect {
    areas: Vec<Area>,
    /// Label and number of each option.
    entries: Vec<(String, i32)>,
    /// Whether each option is on.
    on: Vec<bool>,
    page: usize,
    off: [bool; BAR_LEN],
    sound: Option<&'static str>,
    /// Identifies the list, for the headless run.
    list: u64,
}

impl ExSelect {
    /// `areas` are those of `areaselect.area` (back button and system
    /// buttons), `entries` the labels with the numbers of their options.
    pub fn open(areas: Vec<Area>, entries: Vec<(String, i32)>, notebook: bool) -> Self {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        entries.hash(&mut h);
        let mut off = [false; BAR_LEN];
        // This screen greys the fifth button, not the last, when the
        // notebook is still locked.
        off[NOTEBOOK_BUTTON] = !notebook;
        let on = vec![true; entries.len()];
        Self {
            areas,
            entries,
            on,
            page: 0,
            off,
            sound: None,
            list: h.finish(),
        }
    }

    /// Sets which options are on, in list order (see [`FLAG_FIRST`]).
    pub fn with_flags(mut self, on: Vec<bool>) -> Self {
        if on.len() == self.entries.len() {
            self.on = on;
        }
        self
    }

    /// What the window knows: the load button is grey while there is
    /// nothing to load, and the backlog button while the log is empty.
    pub fn set_context(&mut self, has_saves: bool, has_log: bool) {
        self.off[LOAD_BUTTON] = !has_saves;
        self.off[BACKLOG_BUTTON] = !has_log;
    }

    /// Whether the option on bar `k` of the page is on.
    fn option_on(&self, k: i32) -> bool {
        usize::try_from(k)
            .ok()
            .and_then(|k| self.on.get(self.page * PER_PAGE + k))
            .copied()
            .unwrap_or(false)
    }

    fn last_page(&self) -> usize {
        self.entries.len().saturating_sub(1) / PER_PAGE
    }

    /// Options on the page that is up.
    fn on_page(&self) -> usize {
        (self.entries.len() - self.page * PER_PAGE).min(PER_PAGE)
    }

    /// Top of the first bar of an option and of the up bar.
    fn tops(&self) -> (i32, i32) {
        let top = if self.last_page() == 0 {
            (720 - 225 - self.on_page() as i32 * PITCH) / 2
        } else {
            (720 - 225 - 432) / 2
        };
        (top, top + if self.last_page() == 0 { 0 } else { PITCH })
    }

    /// The bars on show: (id, left, top).
    fn bars(&self) -> Vec<(i32, i32, i32)> {
        let left = (1280 - CELL.0) / 2;
        let (top, first) = self.tops();
        let n = self.on_page() as i32;
        let mut bars: Vec<_> = (0..n).map(|k| (k, left, first + PITCH * k)).collect();
        if self.page > 0 {
            bars.push((UP, left, top));
        }
        if self.page < self.last_page() {
            bars.push((DOWN, left, first + PITCH * n));
        }
        bars
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if let Some(a) = chrome::hit_area(&self.areas, x, y, |id| {
            id == BACK || chrome::bar_index(id).is_some()
        }) {
            return Some(a.id);
        }
        self.bars()
            .into_iter()
            .find(|&(_, l, t)| x >= l && x < l + CELL.0 && y >= t && y < t + CELL.1)
            .map(|b| b.0)
    }

    pub fn enabled(&self, id: i32) -> bool {
        if let Some(i) = chrome::bar_index(id) {
            return !self.off[i];
        }
        match id {
            k if (0..self.on_page() as i32).contains(&k) => self.option_on(k),
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if let Some(c) = chrome::bar_click(id) {
            return c;
        }
        match id {
            BACK => {
                self.sound = Some(CLICK);
                Click::Result(BACK_ANSWER)
            }
            UP if self.page > 0 => {
                self.sound = Some(CLICK);
                self.page -= 1;
                Click::None
            }
            DOWN if self.page < self.last_page() => {
                self.sound = Some(CLICK);
                self.page += 1;
                Click::None
            }
            k if (0..self.on_page() as i32).contains(&k) && !self.option_on(k) => Click::None,
            k if (0..self.on_page() as i32).contains(&k) => {
                let number = self.entries[self.page * PER_PAGE + k as usize].1;
                *TRIED
                    .lock()
                    .unwrap()
                    .get_or_insert_with(HashMap::new)
                    .entry((self.list, number))
                    .or_default() += 1;
                self.sound = Some(CLICK);
                Click::Result(number + 1)
            }
            _ => Click::None,
        }
    }

    pub fn cancel(&mut self) -> Click {
        self.sound = Some(CLICK);
        Click::Result(BACK_ANSWER)
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    /// Width the game centres a label by: 24 pixels for each two bytes of
    /// its Shift-JIS form.
    fn label_width(label: &str) -> i32 {
        let bytes: i32 = label
            .chars()
            .map(|c| {
                if (c as u32) < 0x80 || (0xff61..=0xff9f).contains(&(c as u32)) {
                    1
                } else {
                    2
                }
            })
            .sum();
        (bytes / 2) * 24
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let bars = assets.image(SHEET)?;
        let sheet = chrome::draw_common(canvas, assets, look, &self.off)?;
        if let Some(a) = self.areas.iter().find(|a| a.id == BACK) {
            let row = chrome::state(look, BACK);
            canvas.draw_part(
                &sheet,
                (1143, 854 + 57 * row, 78, 57),
                a.left,
                a.top,
                false,
                255,
            );
        }
        for (id, left, top) in self.bars() {
            let row = chrome::state(look, id);
            canvas.draw_part(
                &bars,
                (
                    0,
                    CELL_TOP + CELL.1 as u32 * row,
                    CELL.0 as u32,
                    CELL.1 as u32,
                ),
                left,
                top,
                false,
                255,
            );
            let label = match id {
                UP => "\u{25b2}",
                DOWN => "\u{25bc}",
                k => &self.entries[self.page * PER_PAGE + k as usize].0,
            };
            let shade = match id {
                k if k < UP && !self.option_on(k) => OFF_TEXT,
                _ => [255, 255, 255, 255],
            };
            let x = if id == UP || id == DOWN {
                (CELL.0 - 24) / 2
            } else {
                (CELL.0 - Self::label_width(label)) / 2
            };
            if let Some(text) = look.text {
                text.draw_dropped(
                    canvas,
                    label,
                    (left + x + 2) as f32,
                    top as f32 + TEXT_BASELINE,
                    TEXT_SIZE,
                    shade,
                );
            }
        }
        let help = match look.hover {
            Some(BACK) => Some("行動選択に戻る"),
            _ => chrome::bar_help(look),
        };
        chrome::draw_help(canvas, look, help);
        Ok(())
    }

    /// Headless run: the option of the list picked least so far, the first
    /// of them (turning the pages to it). A list can be asked again and
    /// again until every entry has done what it does, so each is picked in
    /// turn.
    pub fn auto_pick(&self) -> Option<i32> {
        let tried = TRIED.lock().unwrap();
        let count = |n: i32| {
            tried
                .as_ref()
                .and_then(|t| t.get(&(self.list, n)))
                .copied()
                .unwrap_or(0)
        };
        let Some(want) = (0..self.entries.len())
            .filter(|&i| self.on[i])
            .min_by_key(|&i| (count(self.entries[i].1), i))
        else {
            return Some(BACK);
        };
        let page = want / PER_PAGE;
        Some(match page.cmp(&self.page) {
            std::cmp::Ordering::Less => UP,
            std::cmp::Ordering::Greater => DOWN,
            std::cmp::Ordering::Equal => (want % PER_PAGE) as i32,
        })
    }
}
