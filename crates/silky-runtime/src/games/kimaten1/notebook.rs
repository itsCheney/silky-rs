//! The 捜査手帳 (investigation notebook) of きまぐれテンプテーション.
//!
//! A list of 36 entries on three pages of 14 rows; an entry that was
//! unlocked opens its page (`archive_w01.akb` ...) on the right, under a
//! title picture, and a tall page scrolls ten pixels at a time. Each entry
//! has a byte variable: 0 is locked, 1 is read, 2 or 3 is new. Opening a
//! new entry reads it.
//!
//! No `Game` command opens this screen: the script only announces it
//! (`Game.44`, see [`super::notice`]). The player opens it from the message
//! window's menu and the map's menu; the map here opens it on its own
//! button.
//!
//! The whole screen fades in over 12 frames when it opens and out over 12
//! when it is left; the page and the background are the two layers that
//! fade, so what is under shows through. The thumb of the scroll bar
//! follows the pointer while it is held, and the wheel scrolls ten pixels
//! a notch. The recording on the phone's page (entry 28) is five lines
//! played one after the other.

use std::cell::Cell;

use silky_format::area::{self, Area};

use super::map::{draw_info, fade_alpha, faded, FADE_MS, MENU_INFO, OK_SOUND};
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// Entries, rows a page and pages.
pub const ENTRIES: usize = 36;
const ROWS: usize = 14;
const PAGES: usize = 3;
/// Byte variable of each entry, in list order.
pub const ENTRY_VARS: [usize; ENTRIES] = [
    900, 901, 902, 903, 904, 905, 906, 907, 908, 909, 910, 911, 912, 913, 914, 915, 916, 917, 918,
    919, 920, 921, 922, 924, 925, 929, 926, 936, 930, 927, 935, 934, 931, 933, 932, 928,
];
/// First byte variable the notebook reads, and how many from there (the
/// entries and the one at 940 that remembers entry 30 was opened).
pub const BOOK_FIRST: usize = 900;
pub const BOOK_VARS: usize = 41;
const OPENED_30: usize = 940;

/// Area ids of `notebook.area`: rows 0..13, then these.
pub const UP: i32 = 0x100;
pub const DOWN: i32 = 0x101;
pub const TRACK: i32 = 0x102;
pub const BACK: i32 = 0x200;
pub const PREV: i32 = 0x300;
pub const NEXT: i32 = 0x301;
pub const VOICE: i32 = 0x400;
pub const UNLOCK: i32 = 0x500;
pub const MENU: i32 = 0xf00;
/// The title menu's copy keeps the same bytes 1000 higher, and reads on to
/// byte 1000 (the unlock-all button writes up to there).
pub const EXTRAS_BASE: usize = 1000;
pub const EXTRAS_VARS: usize = 101;

/// The menu button of the notebook itself (shown grey here).
const MENU_NOTEBOOK: i32 = MENU + 5;

/// The entry whose page has a recording.
const VOICE_ENTRY: usize = 28;
/// The five lines of the recording, in the order they play; each starts
/// when the one before it has ended.
const VOICES: [&str; 5] = [
    "kt_b0155.ogg",
    "kt_c0201.ogg",
    "kt_b0156.ogg",
    "kt_c0202.ogg",
    "kt_c0203.ogg",
];
/// The entry that, when first opened, marks the first entry as new.
const MARKER_ENTRY: usize = 30;
/// The entry with the other background.
const LAST_ENTRY: usize = 35;

const ENTRY_POS: (i32, i32, i32) = (16, 137, 32);
const ENTRY_SIZE: (u32, u32) = (302, 30);
/// Rows a group of four entries takes in `archive_p.akb`.
const GROUP_ROWS: u32 = 7;
const LOCKED_ROW: u32 = 3;
const NEW_ROWS: u32 = 4;

const VIEW: (i32, i32, u32, u32) = (385, 130, 808, 523);
const TITLE_POS: (i32, i32) = (492, 35);
const TITLE_SIZE: (u32, u32) = (707, 77);
const TITLE_ROWS: usize = 12;
const INFO_BAR: ((u32, u32, u32, u32), (i32, i32)) = ((364, 0, 513, 30), (540, 687));
/// Scroll arrows: the shape in `archive00_p.akb` and where they go.
const ARROW_SIZE: (u32, u32) = (49, 44);
const ARROW_Y: u32 = 152;
const UP_SRC_X: u32 = 604;
const DOWN_SRC_X: u32 = 653;
const UP_POS: (i32, i32) = (1211, 75);
const DOWN_POS: (i32, i32) = (1211, 627);
const THUMB_SIZE: (u32, u32) = (49, 51);
const THUMB_SRC_X: u32 = 702;
const TRACK_TOP: i32 = 119;
const TRACK_RANGE: i32 = 457;
/// A pointer at y holds the thumb centred on it: its top is 26 above.
const THUMB_GRAB: i32 = 145;
const SCROLL_STEP: i32 = 10;
const PAGE_BUTTON_Y: [i32; 2] = [95, 595];
const MENU_SIZE: (u32, u32) = (52, 38);
const MENU_POS: (i32, i32, i32) = (40, 60, 679);
const BACK_POS: (i32, i32) = (1198, 679);
const BACK_SRC_X: u32 = 312;
const VOICE_FRAME: (u32, u32, i32, i32) = (156, 156, 959, 132);
const VOICE_FRAME_Y: u32 = 272;
/// The unlock-all button of the title's copy: size, place, and its cells
/// on `archive00_p.akb` (three rows: normal, hover, pressed).
const UNLOCK_SIZE: (u32, u32) = (71, 64);
const UNLOCK_POS: (i32, i32) = (1200, 11);
const UNLOCK_SRC: (u32, u32) = (156, 272);

/// Titles of the entries, as the information line names them.
pub const NAMES: [&str; ENTRIES] = [
    "【アンネリーゼ】",
    "【加賀見家】",
    "【刑事十三課】",
    "【集団自殺（？）事件】",
    "【大家】",
    "【呪いのマンション】",
    "【１０２号室の住人】",
    "【１０３号室の住人】",
    "【２０１号室の住人】",
    "【２０２号室の住人】",
    "【教典】",
    "【神の胎】",
    "【神】",
    "【天使/家族】",
    "【マザー】",
    "【刻印】",
    "【裏世界】",
    "【２０１号室の植物】",
    "【注射器】",
    "【儀式】",
    "【神の祝福】",
    "【漫画：セカンダリィオペレーション７巻】",
    "【大家の日記】",
    "【取引現場の写真】",
    "【霧生晶の下着について】",
    "【楠木みりあのスケッチブック】",
    "【ぼくの反抗期】",
    "【使用済み注射器】",
    "【野々宮花音のスマホ】",
    "【楠木みりあの原稿】",
    "【約束】",
    "【解決】",
    "【終幕】",
    "【後悔】",
    "【---------------------】",
    "【　　　　　　　】",
];
const LOCKED_INFO: &str = "【？？？？？？？？】";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Locked,
    New,
    Read,
}

/// What the screen asks of the voice channel (taken once).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    Play(&'static str),
    Stop,
}

/// Leaving: the press is made; the fade out begins when the screen's
/// clock is next seen and runs to its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    No,
    Pending,
    At(u64),
}

#[derive(Debug, Clone)]
pub struct Notebook {
    areas: Vec<Area>,
    /// Bytes 900.. as they are now.
    vars: Vec<u8>,
    page: usize,
    selected: Option<usize>,
    offset: i32,
    /// Height of the selected page's picture, known once it is drawn.
    height: Cell<i32>,
    playing: bool,
    writes: Vec<(usize, u8)>,
    sound: Option<String>,
    /// The title menu's copy: bytes 1000 higher, with the unlock-all button.
    extras: bool,
    /// The line of the recording now playing (valid while `playing`).
    line: usize,
    voice: Option<Voice>,
    leave: Leave,
    /// The screen's clock when it was first seen, for the fade in.
    since: Cell<Option<u64>>,
}

impl Notebook {
    /// Opens the notebook with the bytes 900..=940 as they are.
    pub fn new(assets: &Assets, vars: Vec<u8>) -> Result<Self> {
        let areas = area::parse(&assets.data("notebook.area")?)?;
        Ok(Self::with_areas(areas, vars, false).from_start())
    }

    /// Opens the title menu's copy: `vars` are the bytes 1900.. (900 on
    /// from [`BOOK_FIRST`]), and what it writes goes 1000 higher.
    pub fn new_extras(assets: &Assets, vars: Vec<u8>) -> Result<Self> {
        let areas = area::parse(&assets.data("notebook.area")?)?;
        Ok(Self::with_areas(areas, vars, true).from_start())
    }

    /// Takes the screen's clock to start when the screen opened (a notebook
    /// inside another screen starts at the first time it is seen).
    fn from_start(self) -> Self {
        self.since.set(Some(0));
        self
    }

    /// Opens it with the areas read already. `extras` is the title menu's
    /// copy of the notebook, whose bytes are 1000 higher.
    pub fn with_areas(areas: Vec<Area>, mut vars: Vec<u8>, extras: bool) -> Self {
        vars.resize(if extras { EXTRAS_VARS } else { BOOK_VARS }, 0);
        Self {
            areas,
            vars,
            page: 0,
            selected: None,
            offset: 0,
            height: Cell::new(-1),
            playing: false,
            writes: Vec::new(),
            sound: None,
            extras,
            line: 0,
            voice: None,
            leave: Leave::No,
            since: Cell::new(None),
        }
    }

    fn var(&self, v: usize) -> u8 {
        self.vars.get(v - BOOK_FIRST).copied().unwrap_or(0)
    }

    fn entry(&self, idx: usize) -> Entry {
        if idx >= ENTRIES {
            return Entry::Locked;
        }
        match self.var(ENTRY_VARS[idx]) {
            0 => Entry::Locked,
            2 | 3 => Entry::New,
            _ => Entry::Read,
        }
    }

    fn row_entry(&self, row: usize) -> usize {
        self.page * ROWS + row
    }

    /// The most the page can scroll, 0 while it is not known.
    fn max_scroll(&self) -> i32 {
        (self.height.get() - VIEW.3 as i32).max(0)
    }

    fn set_var(&mut self, v: usize, value: u8) {
        if let Some(b) = self.vars.get_mut(v - BOOK_FIRST) {
            *b = value;
        }
        let base = if self.extras { EXTRAS_BASE } else { 0 };
        self.writes.push((base + v, value));
    }

    /// The unlock-all button: every entry opens (new where it was locked),
    /// and the story's own entries are put in the state the end of the
    /// story leaves them.
    fn unlock_all(&mut self) {
        for v in ENTRY_VARS {
            let now = if self.var(v) == 0 { 3 } else { 1 };
            self.set_var(v, now);
            self.set_var(v + 0x78, 1);
        }
        // (watched byte, value it must have, entry made new otherwise).
        const WATCH: [(usize, u8, usize); 18] = [
            (940, 2, 900),
            (941, 1, 901),
            (943, 1, 903),
            (944, 1, 904),
            (945, 3, 905),
            (946, 5, 906),
            (947, 6, 907),
            (948, 5, 908),
            (949, 5, 909),
            (950, 1, 910),
            (952, 1, 912),
            (954, 2, 914),
            (957, 1, 917),
            (960, 2, 920),
            (985, 1, 905),
            (986, 1, 906),
            (988, 1, 908),
            (1000, 1, 920),
        ];
        for (watched, want, entry) in WATCH {
            if self.var(watched) != want {
                self.set_var(entry, 3);
            }
        }
        for (watched, want, _) in WATCH {
            self.set_var(watched, want);
        }
    }

    fn select(&mut self, idx: usize) {
        self.selected = Some(idx);
        self.offset = 0;
        self.height.set(-1);
        self.set_var(ENTRY_VARS[idx], 1);
        // The last story entry makes the first one new again, once.
        if idx == MARKER_ENTRY && self.var(OPENED_30) != 2 {
            self.set_var(ENTRY_VARS[0], 3);
            self.set_var(OPENED_30, 2);
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        self.areas.iter().find(|a| a.contains(x, y)).map(|a| a.id)
    }

    pub fn enabled(&self, id: i32) -> bool {
        let scroll = self.max_scroll() > 0;
        match id {
            0..=13 => self.entry(self.row_entry(id as usize)) != Entry::Locked,
            UP => scroll && self.offset > 0,
            DOWN => scroll && self.offset < self.max_scroll(),
            TRACK => scroll,
            PREV => self.page > 0,
            NEXT => self.page + 1 < PAGES,
            VOICE => self.selected == Some(VOICE_ENTRY),
            // Only the title menu's copy has the button that unlocks all.
            UNLOCK => self.extras,
            MENU_NOTEBOOK => false,
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if !self.enabled(id) || self.leave != Leave::No {
            return Click::None;
        }
        match id {
            0..=13 => {
                let idx = self.row_entry(id as usize);
                if self.selected != Some(idx) {
                    self.sound = Some(OK_SOUND.into());
                    self.select(idx);
                }
            }
            UP | DOWN => {
                self.sound = Some(OK_SOUND.into());
                let d = if id == UP { -SCROLL_STEP } else { SCROLL_STEP };
                self.offset = (self.offset + d).clamp(0, self.max_scroll());
            }
            PREV | NEXT => {
                self.sound = Some(OK_SOUND.into());
                self.page = if id == PREV {
                    self.page - 1
                } else {
                    self.page + 1
                };
            }
            VOICE => {
                // Starts the recording from its first line, or stops it.
                self.sound = Some(OK_SOUND.into());
                self.line = 0;
                self.playing = !self.playing;
                self.voice = Some(if self.playing {
                    Voice::Play(VOICES[0])
                } else {
                    Voice::Stop
                });
            }
            BACK => {
                self.sound = Some(OK_SOUND.into());
                self.begin_leave();
            }
            UNLOCK => {
                self.sound = Some(OK_SOUND.into());
                self.unlock_all();
            }
            _ => {}
        }
        Click::None
    }

    pub fn cancel(&mut self) -> Click {
        if self.leave == Leave::No {
            self.begin_leave();
        }
        Click::None
    }

    /// Leaving cuts the voice, then the screen fades out.
    fn begin_leave(&mut self) {
        self.playing = false;
        self.voice = Some(Voice::Stop);
        self.leave = Leave::Pending;
    }

    /// Whether the screen is fading in or out at `ms` on the clock.
    pub fn animating(&self, ms: u64) -> bool {
        self.leave != Leave::No || self.age(ms) < FADE_MS
    }

    /// Milliseconds since the screen was first seen.
    fn age(&self, ms: u64) -> u64 {
        let since = self.since.get().unwrap_or_else(|| {
            self.since.set(Some(ms));
            ms
        });
        ms.saturating_sub(since)
    }

    /// Opacity of the whole screen at `ms`.
    fn alpha(&self, ms: u64) -> u8 {
        match self.leave {
            Leave::At(t0) => fade_alpha(ms.saturating_sub(t0), true),
            Leave::Pending => 255,
            Leave::No => fade_alpha(self.age(ms), false),
        }
    }

    /// Runs the fade out; closes the screen once it is over.
    pub fn tick(&mut self, ms: u64) -> Option<Click> {
        self.age(ms);
        match self.leave {
            Leave::Pending => {
                self.leave = Leave::At(ms);
                None
            }
            Leave::At(t0) => (ms >= t0 + FADE_MS).then_some(Click::Close),
            Leave::No => None,
        }
    }

    /// A headless run has no time: ends the fade out at once.
    pub fn settle(&mut self) -> Option<Click> {
        (self.leave != Leave::No).then_some(Click::Close)
    }

    /// Whether the screen is fading in or out and so lets what is under
    /// it show.
    pub fn fading(&self, ms: u64) -> bool {
        self.alpha(ms) < 255
    }

    /// The mouse wheel: ten pixels a notch, when the page scrolls.
    pub fn wheel(&mut self, up: bool) -> bool {
        if self.leave != Leave::No || self.max_scroll() == 0 {
            return false;
        }
        let d = if up { -SCROLL_STEP } else { SCROLL_STEP };
        self.offset = (self.offset + d).clamp(0, self.max_scroll());
        true
    }

    /// A press or drag at y on the thumb's track: the thumb is centred on
    /// the pointer and the page follows in proportion, rounded.
    pub fn press_at(&mut self, id: i32, _x: i32, y: i32) {
        let max = self.max_scroll();
        if id != TRACK || max == 0 || self.leave != Leave::No {
            return;
        }
        let pos = (y - THUMB_GRAB).clamp(0, TRACK_RANGE);
        self.offset = ((pos * max * 10 / TRACK_RANGE + 5) / 10).clamp(0, max);
    }

    pub fn take_voice(&mut self) -> Option<Voice> {
        self.voice.take()
    }

    /// Whether the screen waits for a line of the recording to end.
    pub fn voice_wait(&self) -> bool {
        self.playing
    }

    /// The line that was playing has ended: the next one starts, or the
    /// recording is over after the fifth.
    pub fn voice_ended(&mut self) -> bool {
        if !self.playing {
            return false;
        }
        if self.line + 1 >= VOICES.len() {
            self.playing = false;
        } else {
            self.line += 1;
            self.voice = Some(Voice::Play(VOICES[self.line]));
        }
        true
    }

    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        std::mem::take(&mut self.writes)
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take()
    }

    pub fn auto_pick(&self) -> (Option<i32>, &'static str) {
        (Some(BACK), "notebook")
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let alpha = self.alpha(look.ms);
        faded(canvas, alpha, |canvas| self.draw_all(canvas, assets, look))
    }

    fn draw_all(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let bg = if self.selected == Some(LAST_ENTRY) {
            "archive_bg01.akb"
        } else {
            "archive_bg.akb"
        };
        canvas.draw(&assets.image(bg)?, 0, 0, false);
        let sheet = assets.image("archive00_p.akb")?;
        let labels = assets.image("archive_p.akb")?;
        let state = |id: i32| match (look.pressed == Some(id), look.hover == Some(id)) {
            (true, _) => 2,
            (_, true) => 1,
            _ => 0u32,
        };
        let (src, (x, y)) = INFO_BAR;
        canvas.draw_part(&sheet, src, x, y, false, 255);

        // The list.
        for row in 0..ROWS {
            let idx = self.row_entry(row);
            if idx >= ENTRIES {
                continue;
            }
            let (w, h) = ENTRY_SIZE;
            let sx = (idx as u32 % 4) * w;
            let group = (idx as u32 / 4) * GROUP_ROWS;
            let st = if self.selected == Some(idx) {
                2
            } else {
                state(row as i32)
            };
            let line = match self.entry(idx) {
                Entry::Locked => group + LOCKED_ROW,
                Entry::New => group + NEW_ROWS + st,
                Entry::Read => group + st,
            };
            canvas.draw_part(
                &labels,
                (sx, line * h, w, h),
                ENTRY_POS.0,
                ENTRY_POS.1 + ENTRY_POS.2 * row as i32,
                false,
                255,
            );
        }
        for (id, x, sx) in [(PREV, 0u32, 0u32), (NEXT, 1, ENTRY_SIZE.0)] {
            let st = if self.enabled(id) { state(id) } else { 3 };
            canvas.draw_part(
                &sheet,
                (sx, ARROW_Y + st * ENTRY_SIZE.1, ENTRY_SIZE.0, ENTRY_SIZE.1),
                ENTRY_POS.0,
                PAGE_BUTTON_Y[x as usize],
                false,
                255,
            );
        }

        // The page and its title.
        if let Some(idx) = self.selected {
            let col = (idx / TITLE_ROWS) as u32;
            let row = (idx % TITLE_ROWS) as u32;
            let titles = assets.image("archive_t.akb")?;
            canvas.draw_part(
                &titles,
                (
                    col * TITLE_SIZE.0,
                    row * TITLE_SIZE.1,
                    TITLE_SIZE.0,
                    TITLE_SIZE.1,
                ),
                TITLE_POS.0,
                TITLE_POS.1,
                false,
                255,
            );
            let page = assets.image(&format!("archive_w{:02}.akb", idx + 1))?;
            self.height.set(page.height as i32);
            let off = self.offset.clamp(0, self.max_scroll()) as u32;
            let h = VIEW.3.min(page.height.saturating_sub(off));
            canvas.draw_part(&page, (0, off, VIEW.2, h), VIEW.0, VIEW.1, false, 255);
            if idx == VOICE_ENTRY {
                // A frame round the picture of the phone: dark, white on
                // hover or press, yellow while the recording plays.
                let st = match (state(VOICE), self.playing) {
                    (0, true) => 2,
                    (s, _) => s,
                };
                let (w, h, x, y) = VOICE_FRAME;
                canvas.draw_part(&sheet, (0, VOICE_FRAME_Y + h * st, w, h), x, y, false, 255);
            }
        }

        // Scroll bar.
        let max = self.max_scroll();
        for (id, sx, (x, y)) in [(UP, UP_SRC_X, UP_POS), (DOWN, DOWN_SRC_X, DOWN_POS)] {
            let st = if self.enabled(id) { state(id) } else { 3 };
            canvas.draw_part(
                &sheet,
                (sx, ARROW_Y + st * ARROW_SIZE.1, ARROW_SIZE.0, ARROW_SIZE.1),
                x,
                y,
                false,
                255,
            );
        }
        if max > 0 {
            let st = state(TRACK);
            let y = self.offset * TRACK_RANGE / max + TRACK_TOP;
            canvas.draw_part(
                &sheet,
                (
                    THUMB_SRC_X,
                    ARROW_Y + st * THUMB_SIZE.1,
                    THUMB_SIZE.0,
                    THUMB_SIZE.1,
                ),
                UP_POS.0,
                y,
                false,
                255,
            );
        }

        if self.extras {
            let st = state(UNLOCK);
            canvas.draw_part(
                &sheet,
                (
                    UNLOCK_SRC.0,
                    UNLOCK_SRC.1 + UNLOCK_SIZE.1 * st,
                    UNLOCK_SIZE.0,
                    UNLOCK_SIZE.1,
                ),
                UNLOCK_POS.0,
                UNLOCK_POS.1,
                false,
                255,
            );
        }

        // Bottom row: menu buttons and the way out.
        for n in 0..6 {
            let id = MENU + n;
            let st = if id == MENU_NOTEBOOK { 3 } else { state(id) };
            canvas.draw_part(
                &sheet,
                (
                    MENU_SIZE.0 * n as u32,
                    MENU_SIZE.1 * st,
                    MENU_SIZE.0,
                    MENU_SIZE.1,
                ),
                MENU_POS.0 + MENU_POS.1 * n,
                MENU_POS.2,
                false,
                255,
            );
        }
        canvas.draw_part(
            &sheet,
            (
                BACK_SRC_X,
                MENU_SIZE.1 * state(BACK),
                MENU_SIZE.0,
                MENU_SIZE.1,
            ),
            BACK_POS.0,
            BACK_POS.1,
            false,
            255,
        );
        if let (Some(tr), Some(info)) = (look.text, self.info(look.hover)) {
            draw_info(tr, canvas, &info);
        }
        Ok(())
    }

    /// The information line for the element under the pointer.
    fn info(&self, hover: Option<i32>) -> Option<String> {
        let id = hover?;
        let text = match id {
            0..=13 => {
                let idx = self.row_entry(id as usize);
                return Some(match self.entry(idx) {
                    Entry::Locked => LOCKED_INFO.to_string(),
                    _ => format!("{}を閲覧します", NAMES[idx]),
                });
            }
            UP => "上へスクロールします",
            DOWN => "下へスクロールします",
            TRACK => "スクロールします",
            BACK => "捜査手帳画面から抜けます",
            PREV => "捜査手帳項目リストを１ページ戻します",
            NEXT => "捜査手帳項目リストを１ページ進めます",
            VOICE if self.selected == Some(VOICE_ENTRY) => "録音音声を再生します",
            0xf00..=0xf05 => MENU_INFO[(id - MENU) as usize],
            _ => return None,
        };
        Some(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(vars: &[(usize, u8)]) -> Notebook {
        let mut v = vec![0; BOOK_VARS];
        for &(i, b) in vars {
            v[i - BOOK_FIRST] = b;
        }
        let areas = (0..14)
            .map(|i| Area {
                id: i,
                left: 16,
                top: 137 + 32 * i,
                right: 318,
                bottom: 167 + 32 * i,
            })
            .collect();
        Notebook::with_areas(areas, v, false)
    }

    #[test]
    fn locked_entries_cannot_be_opened() {
        let mut b = book(&[(900, 2)]);
        assert!(b.enabled(0) && !b.enabled(1));
        assert_eq!(b.click(1), Click::None);
        assert!(b.take_bytes().is_empty());
    }

    #[test]
    fn opening_a_new_entry_reads_it() {
        let mut b = book(&[(900, 2), (901, 3)]);
        assert_eq!(b.entry(1), Entry::New);
        b.click(1);
        assert_eq!(b.entry(1), Entry::Read);
        assert_eq!(b.take_bytes(), vec![(901, 1)]);
        assert_eq!(b.take_sound().as_deref(), Some(OK_SOUND));
        // The same entry again does nothing.
        b.click(1);
        assert!(b.take_bytes().is_empty());
    }

    #[test]
    fn entry_thirty_marks_the_first_new_once() {
        let mut b = book(&[(900, 1), (ENTRY_VARS[30], 1)]);
        b.page = 2;
        b.click(2);
        assert_eq!(
            b.take_bytes(),
            vec![(ENTRY_VARS[30], 1), (900, 3), (940, 2)]
        );
        b.click(2);
        b.selected = None;
        b.click(2);
        assert_eq!(b.take_bytes(), vec![(ENTRY_VARS[30], 1)]);
    }

    #[test]
    fn pages_and_back() {
        let mut b = book(&[]);
        assert!(!b.enabled(PREV) && b.enabled(NEXT));
        b.click(NEXT);
        b.click(NEXT);
        assert_eq!(b.page, 2);
        assert!(!b.enabled(NEXT));
        // Leaving fades out first, then closes.
        assert_eq!(b.click(BACK), Click::None);
        assert_eq!(b.tick(500), None);
        assert_eq!(b.tick(500 + FADE_MS), Some(Click::Close));
        let mut b = book(&[]);
        b.cancel();
        assert_eq!(b.settle(), Some(Click::Close));
    }

    #[test]
    fn the_recording_plays_five_lines_in_turn() {
        let mut b = book(&[(ENTRY_VARS[VOICE_ENTRY], 1)]);
        b.page = 2;
        b.click(0);
        assert!(b.enabled(VOICE));
        b.click(VOICE);
        assert_eq!(b.take_voice(), Some(Voice::Play("kt_b0155.ogg")));
        let mut heard = Vec::new();
        while b.voice_wait() {
            assert!(b.voice_ended());
            if let Some(Voice::Play(f)) = b.take_voice() {
                heard.push(f);
            }
        }
        assert_eq!(
            heard,
            [
                "kt_c0201.ogg",
                "kt_b0156.ogg",
                "kt_c0202.ogg",
                "kt_c0203.ogg"
            ]
        );
        // A second press while it plays stops it; leaving stops it too.
        b.click(VOICE);
        b.take_voice();
        b.click(VOICE);
        assert_eq!(b.take_voice(), Some(Voice::Stop));
        b.click(VOICE);
        b.click(BACK);
        assert_eq!(b.take_voice(), Some(Voice::Stop));
        assert!(!b.voice_wait());
    }

    #[test]
    fn wheel_and_thumb_scroll_the_page() {
        let mut b = book(&[(900, 1)]);
        assert!(!b.wheel(false));
        b.height.set(600);
        assert!(b.wheel(false) && b.offset == 10);
        assert!(b.wheel(true) && b.wheel(true) && b.offset == 0);
        // The thumb follows the pointer, centred on it.
        b.press_at(TRACK, 1230, 119 + 26);
        assert_eq!(b.offset, 0);
        b.press_at(TRACK, 1230, 119 + 26 + TRACK_RANGE);
        assert_eq!(b.offset, 77);
        b.press_at(TRACK, 1230, 145 + 229);
        assert_eq!(b.offset, 39);
        b.press_at(DOWN, 0, 0);
        assert_eq!(b.offset, 39);
    }

    #[test]
    fn scroll_stays_inside_the_page() {
        let mut b = book(&[(900, 1)]);
        b.height.set(600);
        assert_eq!(b.max_scroll(), 77);
        for _ in 0..12 {
            b.click(DOWN);
        }
        assert_eq!(b.offset, 77);
        assert!(!b.enabled(DOWN) && b.enabled(UP));
    }
}
