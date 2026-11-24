//! The flag list of 同級生 Remake (the menu's fifth button): an overview
//! of the story events each girl has ahead, and a calendar per girl.
//!
//! Read from the executable and compared with the original under Wine.
//! The list is a grid of the 14 girls (`flag_l_bg_<lang>.akb` with a
//! "Go to Next Flag" plate under each); a card or plate opens the calendar
//! (`flag_s_bg.akb`): four girls a page (scrolled over the 14), a week of
//! seven days a page (days 10 to 30 in three weeks), an icon in the cell
//! of an event's start day (a star waiting, hearts done, a lock out of
//! reach) and a balloon with the place, day and time on hover. Parts come
//! from `flag_p_<lang>.akb`. The first time, a notice of
//! `dialog3_p_<lang>.akb` (byte 8019) explains the list.
//!
//! The events of a girl are records built the way the original builds
//! them, one builder per girl (see `flag_build` and
//! `docs/games/doukyuusei.md`), from the story state when the list opens.
//! The first waiting event is the one to jump to. The jump
//! ([`start_event`]) sets the story state the event expects; the session
//! then drops what is up and starts the event's script.

use std::sync::Arc;

use silky_format::akb::Image;

use super::plate::{DATE, MONEY, TIME};
use super::system::{ids as sys_ids, side, System};
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;
use silky_vm::Vm;

const SHEET: &str = "flag_p.akb";
const LIST_BG: &str = "flag_l_bg.akb";
const CALENDAR_BG: &str = "flag_s_bg.akb";
const NOTICE: &str = "dialog3_p.akb";

/// The byte that is set once the menu's button is unlocked, and the one
/// that is set once the notice has been shown.
pub const UNLOCKED: usize = 8013;
pub const NOTICE_SEEN: usize = 8019;
/// The last day of the story, when the button is gray.
const LAST_DAY: i32 = 831;

pub const GIRLS_COUNT: usize = 14;
const ROWS: usize = 4;
const MAX_SCROLL: usize = GIRLS_COUNT - ROWS;
const WEEKS: usize = 3;
const FIRST_DAY: i32 = 10;

/// Ids of the elements of both pages (the side buttons are
/// [`sys_ids::SIDE`] on).
pub mod ids {
    /// List: the 14 cards, then the 14 plates.
    pub const CARD: i32 = 0;
    pub const PLATE: i32 = 256;
    /// Calendar: day cells are `row * 256 + column`.
    pub const UP: i32 = 0x400;
    pub const DOWN: i32 = 0x401;
    pub const TRACK: i32 = 0x402;
    /// Week tabs 0x500 to 0x502, previous and next week, back.
    pub const TAB: i32 = 0x500;
    pub const PREVIOUS: i32 = 0x600;
    pub const NEXT: i32 = 0x601;
    pub const BACK: i32 = 0x700;
    /// The notice's button.
    pub const CLOSE: i32 = 0x800;
}

/// One event of a girl as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub script: &'static str,
    pub place: u16,
    /// Text line of why it is out of reach, 0 for none.
    pub reason: u16,
    pub d0: i32,
    pub d1: i32,
    pub t0: i32,
    pub t1: i32,
    pub stage: i32,
    pub sub: i32,
    pub cost: i32,
    pub f0: i32,
    /// 0 waiting, 1 done, 2 out of reach.
    pub state: u8,
    /// The girl's first event that is waiting: the one the list can jump to.
    pub jump: bool,
}

/// The story state the builders read.
pub struct Env<'a> {
    pub bytes: &'a [u8],
    pub words: &'a [u16],
    /// Month × 100 + day.
    pub date: i32,
    /// In hundreds of yen.
    pub cash: i32,
    /// Hours × 100 + minutes.
    pub time: i32,
}

/// Weekday and the flag word, among the global variables.
const WEEKDAY: usize = 4;
const FLAG_WORD: usize = 55;
/// What the jump sets: the stage and the sub place of the event, the byte
/// that says the stage is the first (the trip to another town), and the
/// byte that says a jump was made.
const STAGE_AT: usize = 8004;
const SUB_AT: usize = 8005;
const STAGE_ONE_AT: usize = 4069;
const JUMPED_AT: usize = 8001;

/// The records of `girl` (0 to 13) for the story state `env`.
pub fn records(girl: usize, env: &Env) -> Vec<Record> {
    let mut out = super::flag_build::build(girl, env);
    if let Some(r) = out.iter_mut().find(|r| r.state == 0) {
        r.jump = true;
    }
    out
}

/// The jump to `rec`: the story goes to the day and time the event starts
/// (not now), the trip is paid, and the event's stage and place are set,
/// then the caller starts the event's script.
pub fn start_event(rec: &Record, vm: &mut Vm) {
    vm.set_g(DATE, rec.d0);
    vm.set_g(WEEKDAY, (rec.d0 % 100 - 10) % 7);
    vm.set_g(TIME, rec.t0);
    let cash = vm.var_g(MONEY).int();
    vm.set_g(MONEY, cash - rec.cost);
    vm.set_var_b(STAGE_AT, rec.stage as u8);
    vm.set_var_b(SUB_AT, rec.sub as u8);
    // The flag word gets bit 0x80, as the original's jump does.
    let word = vm.var_g(FLAG_WORD).int();
    vm.set_g(FLAG_WORD, word | 0x80);
    vm.set_var_b(STAGE_ONE_AT, (rec.stage == 1) as u8);
    if rec.f0 != -1 {
        vm.set_var_b(rec.f0 as usize, 1);
    }
    vm.set_var_b(JUMPED_AT, 1);
}

/// The record shown in the cell of `day`: of the events that start that
/// day, the first not done (waiting or out of reach alike), else the last
/// done.
pub fn cell_record(records: &[Record], day: i32) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, _) in records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.d0 % 100 == day)
    {
        if best.is_none_or(|b| records[b].state == 1) {
            best = Some(i);
        }
    }
    best
}

/// Whether the button is up: the unlock byte is set and it is not the last
/// day.
pub fn available(bytes: &[u8], date: i32) -> bool {
    bytes.get(UNLOCKED).copied().unwrap_or(0) != 0 && date != LAST_DAY
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    List,
    Calendar,
}

/// What a click did, for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    None,
    /// Something on the screen changed.
    Changed,
    /// A side button, by [`side`] number.
    Side(usize),
    /// The screen closes.
    Leave,
    /// The jump to an event was asked for.
    Jump,
}

#[derive(Debug, Clone)]
pub struct FlagScreen {
    pub page: Page,
    pub girl: usize,
    pub scroll: usize,
    pub week: usize,
    /// The notice is up.
    pub notice: bool,
    /// The scroll thumb is held.
    pub dragging: bool,
    /// Height of the pointer when the thumb last moved with it.
    drag_from: i32,
    records: Vec<Vec<Record>>,
    /// The event a click asked to jump to.
    target: Option<Record>,
    date: i32,
    /// For the side buttons: the game can be saved, the log has entries.
    pub can_save: bool,
    pub log: bool,
}

/// Rectangles (left, top, width, height).
const NOTICE_CLOSE: (i32, i32, i32, i32) = (849, 560, 222, 73);
const TRACK_AT: (i32, i32) = (1878, 174);
const THUMB: (u32, u32) = (24, 636);

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn card_rect(i: usize) -> (i32, i32, i32, i32) {
    (
        326 + 320 * (i % 5) as i32,
        123 + 260 * (i / 5) as i32,
        250,
        175,
    )
}

fn plate_rect(i: usize) -> (i32, i32, i32, i32) {
    (
        331 + 320 * (i % 5) as i32,
        314 + 260 * (i / 5) as i32,
        240,
        50,
    )
}

fn cell_rect(row: usize, col: usize) -> (i32, i32, i32, i32) {
    (522 + 192 * col as i32, 169 + 188 * row as i32, 192, 188)
}

const UP_RECT: (i32, i32, i32, i32) = (1876, 129, 28, 40);
const DOWN_RECT: (i32, i32, i32, i32) = (1876, 915, 28, 40);
const TRACK_RECT: (i32, i32, i32, i32) = (1878, 174, 24, 736);
const PREVIOUS_RECT: (i32, i32, i32, i32) = (366, 1011, 56, 56);
const NEXT_RECT: (i32, i32, i32, i32) = (1063, 1011, 56, 56);
const BACK_RECT: (i32, i32, i32, i32) = (1158, 1002, 74, 74);

fn tab_rect(w: usize) -> (i32, i32, i32, i32) {
    (434 + 210 * w as i32, 1011, 198, 56)
}

impl FlagScreen {
    /// Opens the list from the story state.
    pub fn new(env: &Env, can_save: bool, log: bool, notice: bool) -> Self {
        Self {
            page: Page::List,
            girl: 0,
            scroll: 0,
            week: 0,
            notice,
            dragging: false,
            drag_from: 0,
            records: (0..GIRLS_COUNT).map(|g| records(g, env)).collect(),
            target: None,
            date: env.date,
            can_save,
            log,
        }
    }

    pub fn records(&self, girl: usize) -> &[Record] {
        &self.records[girl]
    }

    /// The day of the month today, for the highlighted column.
    fn today(&self) -> i32 {
        self.date % 100
    }

    /// Opens the calendar at a girl: she is the second row from the top
    /// where possible, and the week is that of her next event.
    pub fn open_calendar(&mut self, girl: usize) {
        self.girl = girl;
        self.page = Page::Calendar;
        self.scroll = girl.saturating_sub(1).min(MAX_SCROLL);
        let day = self.records[girl]
            .iter()
            .find(|r| r.jump)
            .map_or(self.today(), |r| r.d0 % 100);
        self.week = ((day - FIRST_DAY).max(0) as usize / 7).min(WEEKS - 1);
    }

    /// The event the last click asked to jump to.
    pub fn target(&self) -> Option<&Record> {
        self.target.as_ref()
    }

    /// The record in a cell of the calendar (row 0 to 3, column 0 to 6).
    pub fn cell(&self, row: usize, col: usize) -> Option<&Record> {
        let girl = self.scroll + row;
        if girl >= GIRLS_COUNT {
            return None;
        }
        let day = FIRST_DAY + 7 * self.week as i32 + col as i32;
        let recs = &self.records[girl];
        cell_record(recs, day).map(|i| &recs[i])
    }

    /// Whether an element does something now.
    pub fn enabled(&self, id: i32) -> bool {
        match id {
            ids::UP => self.page == Page::Calendar && self.scroll > 0,
            ids::DOWN => self.page == Page::Calendar && self.scroll < MAX_SCROLL,
            ids::PREVIOUS => self.week > 0,
            ids::NEXT => self.week + 1 < WEEKS,
            _ if id >= sys_ids::SIDE => match (id - sys_ids::SIDE) as usize {
                side::LOG => self.log,
                side::SAVE => self.can_save,
                _ => true,
            },
            _ => true,
        }
    }

    /// The element under (x, y).
    pub fn hit(&self, sys: &System, x: i32, y: i32) -> Option<i32> {
        if self.notice {
            return inside(NOTICE_CLOSE, x, y).then_some(ids::CLOSE);
        }
        if let Some(id) = sys.hit_side(x, y) {
            return self.enabled(id).then_some(id);
        }
        let id = match self.page {
            Page::List => (0..GIRLS_COUNT)
                .find(|&i| inside(card_rect(i), x, y))
                .map(|i| ids::CARD + i as i32)
                .or_else(|| {
                    (0..GIRLS_COUNT)
                        .find(|&i| inside(plate_rect(i), x, y))
                        .map(|i| ids::PLATE + i as i32)
                })?,
            Page::Calendar => {
                let cell = (0..ROWS)
                    .flat_map(|r| (0..7).map(move |c| (r, c)))
                    .find(|&(r, c)| inside(cell_rect(r, c), x, y))
                    .map(|(r, c)| 256 * r as i32 + c as i32);
                cell.or_else(|| {
                    [
                        (ids::UP, UP_RECT),
                        (ids::DOWN, DOWN_RECT),
                        (ids::TRACK, TRACK_RECT),
                        (ids::TAB, tab_rect(0)),
                        (ids::TAB + 1, tab_rect(1)),
                        (ids::TAB + 2, tab_rect(2)),
                        (ids::PREVIOUS, PREVIOUS_RECT),
                        (ids::NEXT, NEXT_RECT),
                        (ids::BACK, BACK_RECT),
                    ]
                    .into_iter()
                    .find(|&(_, r)| inside(r, x, y))
                    .map(|(id, _)| id)
                })?
            }
        };
        self.enabled(id).then_some(id)
    }

    /// The girl a card or plate id stands for.
    fn girl_of(id: i32) -> Option<usize> {
        match id {
            0..=13 => Some(id as usize),
            256..=269 => Some((id - ids::PLATE) as usize),
            _ => None,
        }
    }

    /// The row and column of a day cell id.
    fn cell_of(id: i32) -> Option<(usize, usize)> {
        let (row, col) = (id >> 8, id & 255);
        ((0..ROWS as i32).contains(&row) && (0..7).contains(&col))
            .then_some((row as usize, col as usize))
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.notice {
            if id == ids::CLOSE {
                self.notice = false;
                return Click::Changed;
            }
            return Click::None;
        }
        if !self.enabled(id) {
            return Click::None;
        }
        if id >= sys_ids::SIDE {
            let k = (id - sys_ids::SIDE) as usize;
            return if k == side::BACK {
                Click::Leave
            } else {
                Click::Side(k)
            };
        }
        match self.page {
            Page::List => match Self::girl_of(id) {
                Some(g) => {
                    self.open_calendar(g);
                    Click::Changed
                }
                None => Click::None,
            },
            Page::Calendar => match id {
                ids::UP => {
                    self.scroll -= 1;
                    Click::Changed
                }
                ids::DOWN => {
                    self.scroll += 1;
                    Click::Changed
                }
                ids::TRACK => Click::None,
                _ if (ids::TAB..ids::TAB + WEEKS as i32).contains(&id) => {
                    self.week = (id - ids::TAB) as usize;
                    Click::Changed
                }
                ids::PREVIOUS => {
                    self.week -= 1;
                    Click::Changed
                }
                ids::NEXT => {
                    self.week += 1;
                    Click::Changed
                }
                ids::BACK => {
                    self.page = Page::List;
                    Click::Changed
                }
                _ => match Self::cell_of(id).and_then(|(r, c)| self.cell(r, c)) {
                    Some(r) if r.jump => {
                        self.target = Some(r.clone());
                        Click::Jump
                    }
                    _ => Click::None,
                },
            },
        }
    }

    /// Right click: closes the notice, goes back a page or leaves.
    pub fn cancel(&mut self) -> Click {
        if self.notice {
            self.notice = false;
            return Click::Changed;
        }
        match self.page {
            Page::Calendar => {
                self.page = Page::List;
                Click::Changed
            }
            Page::List => Click::Leave,
        }
    }

    /// The wheel on the calendar: down shows the girls below.
    pub fn wheel(&mut self, up: bool) -> bool {
        if self.page != Page::Calendar || self.notice {
            return false;
        }
        let to = if up {
            self.scroll.saturating_sub(1)
        } else {
            (self.scroll + 1).min(MAX_SCROLL)
        };
        std::mem::replace(&mut self.scroll, to) != to
    }

    /// The track is pressed with the pointer at height `y`.
    pub fn press_track(&mut self, y: i32) {
        self.dragging = true;
        self.drag_from = y;
        self.drag(y);
    }

    /// One pass of the held track with the pointer at height `y`: above or
    /// below the thumb it steps four rows; on the thumb it follows the
    /// pointer by whole steps of ten pixels from where it last moved.
    /// Whether the rows moved.
    pub fn drag(&mut self, y: i32) -> bool {
        let top = TRACK_AT.1 + 10 * self.scroll as i32;
        let to = if y < top {
            self.scroll as i32 - 4
        } else if y > top + THUMB.1 as i32 {
            self.scroll as i32 + 4
        } else {
            let delta = (y - self.drag_from) / 10;
            if delta == 0 {
                return false;
            }
            self.drag_from = y;
            self.scroll as i32 + delta
        };
        let to = to.clamp(0, MAX_SCROLL as i32) as usize;
        std::mem::replace(&mut self.scroll, to) != to
    }

    /// The line of `SYSTEM_<lang>.MAP` for the hint bar.
    fn hint_line(&self, id: i32) -> Option<u32> {
        if id >= sys_ids::SIDE {
            return Some(178 + (id - sys_ids::SIDE) as u32);
        }
        match self.page {
            Page::List => Self::girl_of(id).map(|g| 186 + g as u32),
            Page::Calendar => match id {
                ids::UP => Some(169),
                ids::DOWN => Some(170),
                ids::TRACK => Some(171),
                ids::PREVIOUS => Some(175),
                ids::NEXT => Some(176),
                ids::BACK => Some(177),
                _ if (ids::TAB..ids::TAB + WEEKS as i32).contains(&id) => {
                    Some(172 + (id - ids::TAB) as u32)
                }
                _ => {
                    let (row, col) = Self::cell_of(id)?;
                    self.cell(row, col)
                        .filter(|r| r.jump)
                        .map(|_| 155 + (self.scroll + row) as u32)
                }
            },
        }
    }

    /// The pointer-time hint text of an element.
    pub fn hint(&self, sys: &System, id: i32) -> Option<String> {
        let n = self.hint_line(id)?;
        Some(sys.system.as_ref()?.get(0, n)?.to_string())
    }

    /// Draws the page; `state` gives the row (0 normal, 1 hover, 2 pressed)
    /// of an id and `hover` is the id under the pointer.
    pub fn draw(
        &self,
        sys: &System,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
    ) -> Result<()> {
        let sheet = assets.localized_image(SHEET, language)?;
        let bg = match self.page {
            Page::List => assets.localized_image(LIST_BG, language)?,
            Page::Calendar => assets.image(CALENDAR_BG)?,
        };
        canvas.draw(&bg, 0, 0, false);
        let hint = if self.notice {
            None
        } else {
            hover.and_then(|id| self.hint(sys, id))
        };
        sys.draw_base(canvas, assets, hint.as_deref())?;
        match self.page {
            Page::List => self.draw_list(canvas, &sheet, &state),
            Page::Calendar => {
                self.draw_calendar(canvas, &sheet, &state);
                if let (Some(id), false) = (hover, self.notice) {
                    self.draw_balloon(sys, canvas, assets, &sheet, language, id)?;
                }
            }
        }
        sys.draw_side(canvas, assets, |i| {
            let id = sys_ids::SIDE + i as i32;
            if !self.enabled(id) {
                3
            } else {
                state(id).min(1)
            }
        })?;
        if self.notice {
            self.draw_notice(canvas, assets, language, state(ids::CLOSE))?;
        }
        Ok(())
    }

    fn draw_list(&self, canvas: &mut Canvas, sheet: &Arc<Image>, state: &impl Fn(i32) -> u32) {
        for i in 0..GIRLS_COUNT {
            let (x, y, w, h) = plate_rect(i);
            let row = state(ids::CARD + i as i32)
                .max(state(ids::PLATE + i as i32))
                .min(2);
            canvas.draw_part(
                sheet,
                (1000, 404 + 50 * row, w as u32, h as u32),
                x,
                y,
                false,
                255,
            );
        }
    }

    fn draw_calendar(&self, canvas: &mut Canvas, sheet: &Arc<Image>, state: &impl Fn(i32) -> u32) {
        let today = self.today();
        let first = FIRST_DAY + 7 * self.week as i32;
        // The columns: today's band, weekday and the two digits of the day.
        for c in 0..7 {
            let day = first + c;
            let x = 522 + 192 * c;
            if day == today {
                canvas.draw_part(sheet, (1490, 0, 192, 866), x, 54, false, 255);
            }
            canvas.draw_part(
                sheet,
                (1000 + 56 * c as u32, 0, 56, 30),
                590 + 192 * c,
                139,
                false,
                255,
            );
            let row = match c {
                0 => 91,
                6 => 152,
                _ => 30,
            };
            for (k, digit) in [day / 10, day % 10].into_iter().enumerate() {
                canvas.draw_part(
                    sheet,
                    (1000 + 49 * digit as u32, row, 49, 61),
                    568 + 192 * c + 51 * k as i32,
                    69,
                    false,
                    255,
                );
            }
        }
        for r in 0..ROWS {
            let girl = self.scroll + r;
            let y = 186 * r as i32;
            canvas.draw_part(
                sheet,
                ((girl as u32 & 3) * 250, (girl as u32 >> 2) * 175, 250, 175),
                258,
                176 + y,
                false,
                255,
            );
            for (i, rec) in self.records[girl].iter().enumerate() {
                canvas.draw_part(
                    sheet,
                    (750 + 30 * (rec.state == 1) as u32, 525, 30, 30),
                    263 + (i as i32 & 7) * 30,
                    316 + y - (i as i32 >> 3) * 30,
                    false,
                    255,
                );
            }
            for c in 0..7 {
                let Some(rec) = self.cell(r, c) else { continue };
                // Only the next event reacts to the pointer.
                let id = 256 * r as i32 + c as i32;
                let row = if rec.jump { state(id).min(2) } else { 0 };
                canvas.draw_part(
                    sheet,
                    (1682 + 144 * rec.state as u32, 144 * row, 144, 144),
                    546 + 192 * c as i32,
                    191 + 188 * r as i32,
                    false,
                    255,
                );
            }
        }
        // The scroll bar.
        let arrow = |id: i32, off: u32, rect: (i32, i32, i32, i32), canvas: &mut Canvas| {
            let row = if self.enabled(id) {
                state(id).min(2)
            } else {
                3
            };
            canvas.draw_part(
                sheet,
                (off, 608 + 40 * row, 28, 40),
                rect.0,
                rect.1,
                false,
                255,
            );
        };
        arrow(ids::UP, 1682, UP_RECT, canvas);
        arrow(ids::DOWN, 1710, DOWN_RECT, canvas);
        let col = 2114 + 24 * state(ids::TRACK).min(2);
        let top = TRACK_AT.1 + 10 * self.scroll as i32;
        let mut y = top;
        for (sy, h) in [
            (0, 36),
            (20, 145),
            (20, 145),
            (20, 145),
            (20, 145),
            (230, 20),
        ] {
            canvas.draw_part(sheet, (col, sy, 24, h), TRACK_AT.0, y, false, 255);
            y += h as i32;
        }
        // The week tabs, the arrows beside them and back.
        for w in 0..WEEKS {
            let id = ids::TAB + w as i32;
            let row = if w == self.week { 2 } else { state(id).min(1) };
            let r = tab_rect(w);
            canvas.draw_part(
                sheet,
                (320 + 198 * w as u32, 700 + 56 * row, 198, 56),
                r.0,
                r.1,
                false,
                255,
            );
        }
        for (id, sx, r) in [
            (ids::PREVIOUS, 264, PREVIOUS_RECT),
            (ids::NEXT, 914, NEXT_RECT),
        ] {
            let row = if self.enabled(id) {
                state(id).min(2)
            } else {
                3
            };
            canvas.draw_part(sheet, (sx, 700 + 56 * row, 56, 56), r.0, r.1, false, 255);
        }
        canvas.draw_part(
            sheet,
            (1286, 213 + 74 * state(ids::BACK).min(2), 74, 74),
            BACK_RECT.0,
            BACK_RECT.1,
            false,
            255,
        );
    }

    /// The balloon over a cell with the place, day and time of its event.
    fn draw_balloon(
        &self,
        sys: &System,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        language: Language,
        id: i32,
    ) -> Result<()> {
        let Some((row, col)) = Self::cell_of(id) else {
            return Ok(());
        };
        let Some(rec) = self.cell(row, col) else {
            return Ok(());
        };
        let lines = match rec.state {
            2 if rec.reason == 0 => return Ok(()),
            2 => vec![sys.line(rec.reason as u32)],
            _ => vec![
                labelled(language, sys.line(262), &sys.line(rec.place as u32)),
                labelled(language, sys.line(263), &date_text(language, rec.d0 % 100)),
                labelled(language, sys.line(264), &time_text(language, rec.t0)),
            ],
        };
        let x = if col < 6 {
            410 + 192 * col as i32
        } else {
            1504
        };
        let y = 273 + 188 * row as i32;
        canvas.draw(&Arc::new(balloon(sheet, col == 6)), x, y, false);
        if let Some(font) = sys.font(assets) {
            for (i, line) in lines.iter().enumerate() {
                font.draw_small(canvas, line, x + TEXT_AT.0, y + TEXT_AT.1 + 20 * i as i32);
            }
        }
        Ok(())
    }

    fn draw_notice(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        button: u32,
    ) -> Result<()> {
        let sheet = assets.localized_image(NOTICE, language)?;
        canvas.fill(0, 0, canvas.width, canvas.height, [0, 0, 0, 127]);
        canvas.draw_part(&sheet, (0, 0, 880, 310), 520, 355, false, 255);
        canvas.draw_part(&sheet, (0, 310, 805, 166), 558, 384, false, 255);
        canvas.draw_part(
            &sheet,
            (880, 74 * button.min(2), 222, 73),
            NOTICE_CLOSE.0,
            NOTICE_CLOSE.1,
            false,
            255,
        );
        Ok(())
    }
}

/// The balloon's layer: three rows of the sheet (top, middle, bottom) cut
/// into a left edge, four middle tiles and a right edge, then the spike
/// over the top row: the one pointing left, or for the last column (whose
/// balloon sits further left) the one pointing right.
const BALLOON_SIZE: (u32, u32) = (376, 176);
/// Each row: where it is on the sheet, its height, where it goes, and the
/// sheet column of its middle tiles.
const BALLOON_ROWS: [(u32, u32, u32, u32); 3] = [
    (432, 58, 0, 1953),
    (490, 55, 58, 1953),
    (545, 63, 113, 1870),
];
const BALLOON_LEFT: u32 = 1682;
const BALLOON_RIGHT: u32 = 2058;
const BALLOON_EDGE: u32 = 22;
const BALLOON_TILE: u32 = 83;
const SPIKE: (u32, u32, u32, u32) = (1787, 432, 83, 58);
const SPIKE_LAST_X: u32 = 1870;
const SPIKE_AT: u32 = 188;
const TEXT_AT: (i32, i32) = (28, 55);

fn balloon(sheet: &Image, last: bool) -> Image {
    let (w, h) = BALLOON_SIZE;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut copy = |sx: u32, sy: u32, cw: u32, ch: u32, dx: u32, dy: u32| {
        for j in 0..ch {
            let from = (((sy + j) * sheet.width + sx) * 4) as usize;
            let to = (((dy + j) * w + dx) * 4) as usize;
            let n = (cw * 4) as usize;
            if let (Some(src), Some(dst)) =
                (sheet.rgba.get(from..from + n), rgba.get_mut(to..to + n))
            {
                dst.copy_from_slice(src);
            }
        }
    };
    for (sy, rh, dy, tile) in BALLOON_ROWS {
        copy(BALLOON_LEFT, sy, BALLOON_EDGE, rh, 0, dy);
        for k in 0..4 {
            copy(
                tile,
                sy,
                BALLOON_TILE,
                rh,
                BALLOON_EDGE + k * BALLOON_TILE,
                dy,
            );
        }
        copy(BALLOON_RIGHT, sy, BALLOON_EDGE, rh, w - BALLOON_EDGE, dy);
    }
    let (sx, sy, sw, sh) = SPIKE;
    copy(
        if last { SPIKE_LAST_X } else { sx },
        sy,
        sw,
        sh,
        SPIKE_AT,
        0,
    );
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// A label and its value; the Chinese tables' labels end in a colon and
/// take no space.
fn labelled(language: Language, label: String, value: &str) -> String {
    match language {
        Language::English => format!("{label} {value}"),
        _ => format!("{label}{value}"),
    }
}

/// The day of August as the balloon words it.
pub fn date_text(language: Language, day: i32) -> String {
    match language {
        Language::SimplifiedChinese | Language::TraditionalChinese => format!("８月{day:02}日"),
        _ => format!("August {day:02}"),
    }
}

/// A time (hours × 100 + minutes): a 12-hour dial in English, 24 hours in
/// Chinese.
pub fn time_text(language: Language, time: i32) -> String {
    let (h, m) = (time.max(0) / 100, time.max(0) % 100);
    match language {
        Language::SimplifiedChinese => format!("{h:02}点{m:02}分"),
        Language::TraditionalChinese => format!("{h:02}點{m:02}分"),
        _ => {
            let (h12, half) = if h >= 12 {
                (if h >= 13 { h - 12 } else { h }, "p.m.")
            } else {
                (h, "a.m.")
            };
            format!("{h12:02}:{m:02} {half}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(bytes: &'a [u8], words: &'a [u16], date: i32) -> Env<'a> {
        Env {
            bytes,
            words,
            date,
            cash: 500,
            time: 800,
        }
    }

    fn states(girl: usize, set: &[usize], words: &[(usize, u16)]) -> Vec<Record> {
        let mut bytes = vec![0u8; 9000];
        bytes[UNLOCKED] = 1;
        for &i in set {
            bytes[i] = 1;
        }
        let mut w = vec![0u16; 700];
        for &(i, v) in words {
            w[i] = v;
        }
        records(girl, &env(&bytes, &w, 810))
    }

    #[test]
    fn cells() {
        let r = states(0, &[], &[]);
        // Two events start on day 10: the one to jump to is shown, as the
        // original lets that cell be clicked (Mai's pool event under Wine).
        assert_eq!(cell_record(&r, 10), Some(0));
        assert_eq!(cell_record(&r, 12), Some(2));
        assert_eq!(cell_record(&r, 14), None);
        // Satomi's two locked events on day 20: the first.
        let r = states(2, &[], &[]);
        assert_eq!(cell_record(&r, 20), Some(3));
        // Four events on day 14, the last out of reach: the first.
        let r = states(3, &[], &[]);
        assert_eq!(cell_record(&r, 14), Some(1));
        // A done event gives way to a later one not done.
        let mut r = r;
        r[1].state = 1;
        assert_eq!(cell_record(&r, 14), Some(2));
    }

    #[test]
    fn texts() {
        assert_eq!(time_text(Language::English, 900), "09:00 a.m.");
        assert_eq!(time_text(Language::English, 1255), "12:55 p.m.");
        assert_eq!(time_text(Language::English, 2105), "09:05 p.m.");
        assert_eq!(date_text(Language::English, 12), "August 12");
        assert_eq!(time_text(Language::SimplifiedChinese, 905), "09点05分");
    }

    #[test]
    fn pages() {
        let mut bytes = vec![0u8; 9000];
        bytes[UNLOCKED] = 1;
        let w = vec![0u16; 10];
        let mut s = FlagScreen::new(&env(&bytes, &w, 810), true, true, false);
        assert_eq!(s.click(3), Click::Changed);
        // Girl 3: second row of the page, week of her first event (day 10).
        assert_eq!((s.page, s.scroll, s.week), (Page::Calendar, 2, 0));
        s.open_calendar(0);
        assert_eq!(s.scroll, 0);
        s.open_calendar(13);
        assert_eq!(s.scroll, 10);
        assert!(!s.wheel(false) && s.wheel(true) && s.scroll == 9);
        // Below the thumb: four rows a pass.
        s.scroll = 3;
        s.press_track(174 + 30 + 636 + 50);
        assert_eq!(s.scroll, 7);
        assert!(s.drag(900) && s.scroll == 10);
        // On the thumb: whole steps of ten pixels from where it moved.
        s.scroll = 3;
        s.press_track(300);
        assert_eq!(s.scroll, 3);
        assert!(!s.drag(309));
        assert!(s.drag(321) && s.scroll == 5);
        assert!(!s.drag(325));
        assert!(s.drag(305) && s.scroll == 4);
        // Girl 1 is in the second row; her first event (day 10) is the one
        // to jump to, and so is girl 0's on the same day.
        s.open_calendar(1);
        assert_eq!(s.click(256), Click::Jump);
        assert_eq!(s.click(0), Click::Jump);
        assert_eq!(s.click(1), Click::None);
        assert_eq!(s.cancel(), Click::Changed);
        assert_eq!(s.cancel(), Click::Leave);
    }

    #[test]
    fn jump_sets_the_story_state() {
        let mut bytes = vec![0u8; 9000];
        bytes[UNLOCKED] = 1;
        let w = vec![0u16; 10];
        let mut s = FlagScreen::new(&env(&bytes, &w, 810), true, true, false);
        // Girl 1's first event on day 10 is the one to jump to.
        s.open_calendar(1);
        assert_eq!(s.click(256), Click::Jump);
        let rec = s.target().cloned().unwrap();
        assert_eq!(rec.script, records(1, &env(&bytes, &w, 810))[0].script);
        let mut vm = Vm::new(silky_format::mes::Profile::default());
        vm.set_g(MONEY, 10);
        start_event(&rec, &mut vm);
        assert_eq!(vm.var_g(DATE).int(), rec.d0);
        assert_eq!(vm.var_g(TIME).int(), rec.t0);
        assert_eq!(vm.var_g(WEEKDAY).int(), (rec.d0 % 100 - 10) % 7);
        assert_eq!(vm.var_g(MONEY).int(), 10);
        assert_eq!(vm.var_b(JUMPED_AT), 1);
        assert_eq!(vm.var_b(STAGE_ONE_AT), 0);

        // A trip: stage 1 costs cash, sets the stage byte and its own flag.
        let trip = records(0, &env(&bytes, &w, 810))[6].clone();
        assert_eq!(
            (trip.script, trip.stage, trip.cost, trip.f0),
            ("yabuki.mes", 1, 3, 8002)
        );
        start_event(&trip, &mut vm);
        assert_eq!(vm.var_g(DATE).int(), 824);
        assert_eq!(vm.var_g(WEEKDAY).int(), (24 - 10) % 7);
        assert_eq!(vm.var_g(MONEY).int(), 7);
        assert_eq!(vm.var_b(STAGE_AT), 1);
        assert_eq!(vm.var_b(SUB_AT), 2);
        assert_eq!(vm.var_b(STAGE_ONE_AT), 1);
        assert_eq!(vm.var_b(8002), 1);
        assert_eq!(vm.var_g(FLAG_WORD).int() & 0x80, 0x80);
    }
}
