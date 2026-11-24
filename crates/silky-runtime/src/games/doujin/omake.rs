//! The extras of 先輩が私の妄想にドージンする: the CG, scene and music modes
//! (`Game.30` to `Game.32`).
//!
//! The three modes are tabs of one screen. A press on an open tab changes
//! the mode without going back to the script; the back button and a right
//! click answer -1 and the script returns to the title. Only the scene mode
//! answers something else: the number of the scene the player picked
//! (0 to 18), which the script plays.
//!
//! The CG mode shows its entries (a picture and its variants) nine to a
//! page on four pages. A press on an entry opens a grid of its variants
//! over the page; a press on a variant shows it full screen, and each
//! press after that shows the next picture seen, up to the end of the
//! list, when the grid of the first one comes back. The scene mode shows
//! the 19 scenes nine to a page on three pages; the scene under the
//! pointer plays two frames of its own thumbnail. The music mode lists the
//! 13 tracks eight at a time with a scroll bar and a volume bar.
//!
//! Pictures seen are bits from byte 3300, scenes seen are bytes 3101 to
//! 3119, tracks heard are bytes 3051 to 3063, and bytes 0xbba to 0xbbc
//! open the tabs of the modes. The original shows no help lines on these
//! screens.

use std::cell::Cell;
use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;

use super::cgdata::{ENTRIES, PICTURES};
use crate::assets::Assets;
use crate::games::kimaten1::omake::Music;
use crate::games::kimaten1::{Click, Look};
use crate::render::Canvas;
use crate::Result;

/// First byte of the pictures seen, one bit each.
pub const CG_SEEN: usize = 3300;
/// Bytes set once a picture is seen.
pub const CG_ANY: usize = 0xbba;
pub const CG_UNLOCKED: usize = 0xbb9;
/// First byte of the scenes seen, and of the tracks heard.
pub const SCENE_FIRST: usize = 3101;
pub const MUSIC_FIRST: usize = 3051;
/// Bytes that open the tabs of the three modes.
pub const TAB_BYTES: [usize; 3] = [0xbba, 0xbbb, 0xbbc];
pub const SCENES: usize = 19;
pub const TRACKS: usize = 13;
/// What the modes answer to leave to the title.
pub const LEAVE: i32 = -1;
/// The background music the CG and scene modes start with when none plays.
pub const BGM: &str = "do-jon_OP_inst.wav";

const MODES: usize = 3;
const BACKGROUNDS: [&str; MODES] = ["cgmode_bg.akb", "scenemode_bg.akb", "musicmode_bg.akb"];
const SHEETS: [&str; MODES] = ["cgscenemode_p.akb", "cgscenemode_p.akb", "musicmode_p.akb"];
const AREAS: [&str; MODES] = ["CGMODE.AREA", "SCMODE.AREA", "SOUNDMODE.AREA"];
const TAB_AT: [(i32, i32); MODES] = [(34, 187), (34, 325), (34, 461)];
const TAB_SIZE: (u32, u32) = (200, 123);
const CELLS: usize = 9;
const CG_PAGES: usize = 4;
const SCENE_PAGES: usize = 3;
const MUSIC_ROWS: usize = 8;
const SCROLL_MAX: usize = TRACKS - MUSIC_ROWS;
const VOLUME_LEFT: i32 = 407;
const VOLUME_SPAN: f32 = 460.0;
const BACK: i32 = 0x300;
/// The sounds of the buttons: the confirm sound for every press that
/// does something, the cancel sound for the right click that leaves.
const OK_SOUND: &str = "ses_001.wav";
const CANCEL_SOUND: &str = "ses_002.wav";
const PREV: i32 = 0x100;
const NEXT: i32 = 0x101;
const SCROLLBAR: i32 = 0x102;
const VOLUME: i32 = 0x200;
/// The picture shown full screen.
const PICTURE: i32 = 0x900;

/// The files of the tracks.
const TRACK_FILES: [&str; TRACKS] = [
    "degi_09.wav",
    "degi_02.wav",
    "degi_03.wav",
    "degi_15.wav",
    "degi_06.wav",
    "degi_07.wav",
    "degi_12.wav",
    "degi_04.wav",
    "degi_17.wav",
    "degi_26.wav",
    "degi_14.wav",
    "do-jon_OP.wav",
    "do-jon_OP_inst.wav",
];

/// The file of track `i` (from 0).
pub fn track_file(i: usize) -> String {
    TRACK_FILES[i].to_string()
}

/// What the screen starts from: the bytes of the script.
#[derive(Debug, Clone, Default)]
pub struct Saved {
    pub seen: Vec<u8>,
    pub scenes: [u8; SCENES],
    pub tracks: [u8; TRACKS],
    pub tabs: [u8; MODES],
    pub volume: f32,
}

/// The grid of the variants of an entry, and the picture on screen.
#[derive(Debug, Clone, Copy)]
struct Picker {
    entry: usize,
    page: usize,
    /// The picture shown full screen, and where it was picked from.
    shown: Option<usize>,
    from: usize,
}

#[derive(Debug, Clone)]
pub struct Omake {
    mode: usize,
    areas: [Vec<Area>; MODES],
    saved: Saved,
    page: usize,
    scene_page: usize,
    picker: Option<Picker>,
    scroll: usize,
    volume: f32,
    playing: Option<usize>,
    music: Option<Music>,
    new_volume: Option<f32>,
    /// The sound to play for the last press (taken once).
    sound: Option<&'static str>,
    /// The scene under the pointer and since when.
    hover_since: Cell<(Option<i32>, u64)>,
}

/// Whether picture `id` is seen in `bits`.
pub fn is_seen(bits: &[u8], id: usize) -> bool {
    bits.get(id / 8).is_some_and(|b| b & (1 << (id % 8)) != 0)
}

/// Pictures seen.
pub fn seen_count(bits: &[u8]) -> usize {
    (0..PICTURES.len()).filter(|&i| is_seen(bits, i)).count()
}

/// The completion in percent as the modes show it: 0 with nothing seen,
/// 100 with all, else at least 1.
pub fn percent(seen: usize, total: usize) -> u32 {
    if seen == 0 || total == 0 {
        0
    } else if seen >= total {
        100
    } else {
        ((seen as f32 / total as f32 * 100.0) as u32).max(1)
    }
}

/// Marks the picture on screen as seen when it is one of the table: the
/// background and the layers over it in slot order must be the ones of a
/// picture, compared without case and extension. Returns the bytes to set
/// (the bit's byte and the two that open the CG mode), empty if the
/// picture is not one of the table.
pub fn register<'a>(
    bytes: &[u8],
    background: &str,
    layers: impl Iterator<Item = &'a str>,
) -> Vec<(usize, u8)> {
    let norm = |s: &str| {
        let s = s.to_ascii_lowercase();
        s.strip_suffix(".akb").map(str::to_string).unwrap_or(s)
    };
    let bg = norm(background);
    let layers: Vec<String> = layers.map(norm).collect();
    let Some(id) = PICTURES.iter().position(|(b, l)| {
        *b == bg && l.len() == layers.len() && l.iter().zip(&layers).all(|(a, b)| a == b)
    }) else {
        return Vec::new();
    };
    let at = CG_SEEN + id / 8;
    let byte = bytes.get(at).copied().unwrap_or(0) | 1 << (id % 8);
    vec![(at, byte), (CG_ANY, 1), (CG_UNLOCKED, 1)]
}

/// The variant of entry `e` its cell shows: 0 if its first picture is
/// seen, else the first one seen, `None` if none is.
fn thumb(bits: &[u8], e: usize) -> Option<usize> {
    let (first, count, _) = ENTRIES[e];
    (0..count).find(|&i| is_seen(bits, first + i))
}

fn seen_in(bits: &[u8], e: usize) -> usize {
    let (first, count, _) = ENTRIES[e];
    (0..count).filter(|&i| is_seen(bits, first + i)).count()
}

/// The entry of picture `id`.
fn entry_of(id: usize) -> usize {
    ENTRIES
        .iter()
        .position(|&(first, count, _)| id >= first && id < first + count)
        .unwrap_or(0)
}

/// Rectangle of cell `i` of the grid of variants when it holds `n`.
fn picker_cell(n: usize, i: usize) -> (i32, i32, i32, i32) {
    let across = n.min(3) as i32;
    let rows = n.div_ceil(3) as i32;
    let x0 = (1280 - 330 * across) / 2 + 4;
    let y0 = (620 - 190 * rows) / 2 + 16;
    (
        x0 + 330 * (i % 3) as i32,
        y0 + 190 * (i / 3) as i32,
        320,
        180,
    )
}

impl Omake {
    pub fn open(mode: usize, areas: [Vec<Area>; MODES], saved: Saved) -> Self {
        let volume = saved.volume;
        Self {
            mode: mode.min(MODES - 1),
            areas,
            saved,
            page: 0,
            scene_page: 0,
            picker: None,
            scroll: 0,
            volume,
            playing: None,
            music: None,
            new_volume: None,
            sound: None,
            hover_since: Cell::new((None, 0)),
        }
    }

    /// The names of the area files, in the order of the modes.
    pub fn area_files() -> [&'static str; MODES] {
        AREAS
    }

    /// Whether the tab of mode `k` can be pressed.
    fn tab_open(&self, k: usize) -> bool {
        k != self.mode && self.saved.tabs[k] != 0
    }

    fn cell_entry(&self, cell: i32) -> Option<usize> {
        let e = self.page * CELLS + usize::try_from(cell).ok()?;
        (cell < CELLS as i32 && e < ENTRIES.len()).then_some(e)
    }

    fn cell_scene(&self, cell: i32) -> Option<usize> {
        let s = self.scene_page * CELLS + usize::try_from(cell).ok()?;
        (cell < CELLS as i32 && s < SCENES).then_some(s)
    }

    fn picker_cells(&self, p: &Picker) -> usize {
        ENTRIES[p.entry].1.saturating_sub(p.page * CELLS).min(CELLS)
    }

    fn picker_pages(&self, p: &Picker) -> usize {
        ENTRIES[p.entry].1.div_ceil(CELLS)
    }

    /// The element under (x, y).
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if let Some(p) = &self.picker {
            if p.shown.is_some() {
                return Some(PICTURE);
            }
            let n = self.picker_cells(p);
            for i in 0..n {
                let (l, t, w, h) = picker_cell(n, i);
                if x >= l && x < l + w && y >= t && y < t + h {
                    return Some(i as i32);
                }
            }
            let mut buttons = vec![(BACK, 1069, 636, 1269, 707)];
            if self.picker_pages(p) > 1 {
                buttons.push((PREV, 437, 645, 491, 699));
                buttons.push((NEXT, 741, 645, 795, 699));
            }
            return buttons
                .iter()
                .find(|r| x >= r.1 && x < r.3 && y >= r.2 && y < r.4)
                .map(|r| r.0);
        }
        self.areas[self.mode]
            .iter()
            .find(|a| a.contains(x, y))
            .map(|a| a.id)
    }

    pub fn enabled(&self, id: i32) -> bool {
        if let Some(p) = &self.picker {
            return match id {
                PREV => p.page > 0,
                NEXT => p.page + 1 < self.picker_pages(p),
                0..=8 => {
                    let (first, count, _) = ENTRIES[p.entry];
                    let i = p.page * CELLS + id as usize;
                    i < count && is_seen(&self.saved.seen, first + i)
                }
                _ => true,
            };
        }
        match (self.mode, id) {
            (_, 0xa00..=0xa02) => self.tab_open((id & 0xff) as usize),
            (0, 0..=8) => self
                .cell_entry(id)
                .is_some_and(|e| thumb(&self.saved.seen, e).is_some()),
            (0, PREV) => self.page > 0,
            (0, NEXT) => self.page + 1 < CG_PAGES,
            (0, 0x200..=0x203) => true,
            (1, 0..=8) => self
                .cell_scene(id)
                .is_some_and(|s| self.saved.scenes[s] != 0),
            (1, PREV) => self.scene_page > 0,
            (1, NEXT) => self.scene_page + 1 < SCENE_PAGES,
            (2, 0..=7) => self
                .saved
                .tracks
                .get(self.scroll + id as usize)
                .is_some_and(|&t| t == 1),
            (2, PREV) => self.scroll > 0,
            (2, NEXT) => self.scroll < SCROLL_MAX,
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.picker.is_some() {
            return self.click_picker(id);
        }
        match id {
            0xa00..=0xa02 => {
                let k = (id & 0xff) as usize;
                if self.tab_open(k) {
                    self.sound = Some(OK_SOUND);
                    self.mode = k;
                    self.hover_since.set((None, 0));
                }
                return Click::None;
            }
            BACK => {
                self.sound = Some(OK_SOUND);
                return Click::Result(LEAVE);
            }
            _ => {}
        }
        // Every press that does something plays the confirm sound.
        let works = self.enabled(id);
        if works && !(self.mode == 2 && matches!(id, SCROLLBAR | VOLUME)) {
            self.sound = Some(OK_SOUND);
        }
        match self.mode {
            0 => match id {
                0..=8 => {
                    if let Some(e) = self.cell_entry(id).filter(|_| self.enabled(id)) {
                        self.picker = Some(Picker {
                            entry: e,
                            page: 0,
                            shown: None,
                            from: 0,
                        });
                    }
                }
                PREV if self.page > 0 => self.page -= 1,
                NEXT if self.page + 1 < CG_PAGES => self.page += 1,
                0x200..=0x203 => {
                    if self.page == (id & 0xff) as usize {
                        self.sound = None;
                    }
                    self.page = (id & 0xff) as usize;
                }
                _ => {}
            },
            1 => match id {
                0..=8 => {
                    if let Some(s) = self.cell_scene(id).filter(|_| self.enabled(id)) {
                        return Click::Result(s as i32);
                    }
                }
                PREV if self.scene_page > 0 => self.scene_page -= 1,
                NEXT if self.scene_page + 1 < SCENE_PAGES => self.scene_page += 1,
                0x200..=0x202 => {
                    if self.scene_page == (id & 0xff) as usize {
                        self.sound = None;
                    }
                    self.scene_page = (id & 0xff) as usize;
                }
                _ => {}
            },
            _ => self.click_music(id),
        }
        Click::None
    }

    fn click_picker(&mut self, id: i32) -> Click {
        let Some(mut p) = self.picker else {
            return Click::None;
        };
        if let Some(shown) = p.shown {
            // The next picture seen, in order; the grid when none is left.
            p.shown = (shown + 1..PICTURES.len()).find(|&i| is_seen(&self.saved.seen, i));
            if p.shown.is_none() {
                p.entry = entry_of(p.from);
                p.page = (p.from - ENTRIES[p.entry].0) / CELLS;
            }
            self.picker = Some(p);
            return Click::None;
        }
        if self.enabled(id) {
            self.sound = Some(OK_SOUND);
        }
        match id {
            BACK => {
                self.picker = None;
                return Click::None;
            }
            PREV if p.page > 0 => p.page -= 1,
            NEXT if p.page + 1 < self.picker_pages(&p) => p.page += 1,
            0..=8 if self.enabled(id) => {
                let first = ENTRIES[p.entry].0 + p.page * CELLS + id as usize;
                p.shown = Some(first);
                p.from = first;
            }
            _ => {}
        }
        self.picker = Some(p);
        Click::None
    }

    fn click_music(&mut self, id: i32) {
        match id {
            0..=7 => {
                let track = self.scroll + id as usize;
                if self.saved.tracks.get(track) != Some(&1) {
                    return;
                }
                if self.playing == Some(track) {
                    self.playing = None;
                    self.music = Some(Music::Stop);
                } else {
                    self.playing = Some(track);
                    self.music = Some(Music::Play(track_file(track)));
                }
            }
            PREV if self.scroll > 0 => self.scroll -= 1,
            NEXT if self.scroll < SCROLL_MAX => self.scroll += 1,
            _ => {}
        }
    }

    /// A press at (x, y) on the scroll bar or the volume bar, which the
    /// original follows while the button is held.
    pub fn press_at(&mut self, id: i32, x: i32, y: i32) {
        if self.mode != 2 || self.picker.is_some() {
            return;
        }
        match id {
            SCROLLBAR => {
                let t = (y as f32 - 95.0 - 41.0) / 76.0;
                self.scroll = (t.round().max(0.0) as usize).min(SCROLL_MAX);
            }
            VOLUME => {
                let px = (x - VOLUME_LEFT) as f32;
                self.volume = (px / VOLUME_SPAN).clamp(0.0, 1.0);
                self.new_volume = Some(self.volume);
            }
            _ => {}
        }
    }

    /// Right click or Escape: back one step, and out of the modes.
    pub fn cancel(&mut self) -> Click {
        match &mut self.picker {
            Some(p) if p.shown.is_some() => {
                p.shown = None;
                Click::None
            }
            Some(_) => {
                self.picker = None;
                Click::None
            }
            // A track playing stops first; the screen is left only when
            // none plays, with the cancel sound.
            None if self.mode == 2 && self.playing.is_some() => {
                self.playing = None;
                self.music = Some(Music::Stop);
                Click::None
            }
            None => {
                self.sound = Some(CANCEL_SOUND);
                Click::Result(LEAVE)
            }
        }
    }

    /// The sound to play for the last press (taken once).
    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    /// Clears the playing row once the picked track has ended; true when
    /// the screen changed.
    pub fn track_ended(&mut self) -> bool {
        self.playing.take().is_some()
    }

    /// Whether a picked track is meant to be playing.
    pub fn track_playing(&self) -> bool {
        self.playing.is_some()
    }

    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        Vec::new()
    }

    pub fn take_music(&mut self) -> Option<Music> {
        self.music.take()
    }

    pub fn take_volume(&mut self) -> Option<f32> {
        self.new_volume.take()
    }

    /// Headless run: the first thing to press.
    pub fn auto_pick(&self) -> Option<i32> {
        Some(BACK)
    }

    // ---- drawing ----

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image(SHEETS[self.mode])?;
        let bg = assets.image(BACKGROUNDS[self.mode])?;
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 255]);
        canvas.draw(&bg, 0, 0, false);
        let calm = Look {
            hover: None,
            pressed: None,
            ..*look
        };
        let l = if self.picker.is_some() { &calm } else { look };
        match self.mode {
            0 => self.draw_cg(canvas, assets, &sheet, l)?,
            1 => self.draw_scene(canvas, assets, &sheet, l)?,
            _ => self.draw_music(canvas, assets, &sheet, l)?,
        }
        self.draw_common(canvas, &sheet, l);
        if let Some(p) = &self.picker {
            self.draw_picker(canvas, assets, &sheet, p, look)?;
        }
        Ok(())
    }

    fn state(&self, look: &Look, id: i32) -> u32 {
        if !self.enabled(id) {
            3
        } else if look.pressed == Some(id) {
            2
        } else if look.hover == Some(id) {
            1
        } else {
            0
        }
    }

    /// Tabs and the back button.
    fn draw_common(&self, canvas: &mut Canvas, sheet: &Arc<Image>, look: &Look) {
        for k in 0..MODES {
            let (x, y) = TAB_AT[k];
            let st = if k == self.mode {
                2
            } else if self.tab_open(k) {
                self.state(look, 0xa00 + k as i32).min(2)
            } else {
                3
            };
            canvas.draw_part(
                sheet,
                (200 * k as u32, st * TAB_SIZE.1, TAB_SIZE.0, TAB_SIZE.1),
                x,
                y,
                false,
                255,
            );
        }
        // The back button is green in the scene mode.
        let rows = if self.mode == 1 { 4 } else { 0 };
        let st = self.state(look, BACK).min(2) + rows;
        canvas.draw_part(sheet, (600, st * 71, 200, 71), 30, 623, false, 255);
    }

    /// The completion: digits from the right, then the percent sign.
    fn draw_percent(&self, canvas: &mut Canvas, sheet: &Arc<Image>, value: u32) {
        canvas.draw_part(sheet, (340, 492, 34, 53), 1217, 639, false, 255);
        let mut v = value;
        let mut x = 1171;
        loop {
            canvas.draw_part(sheet, (34 * (v % 10), 492, 34, 53), x, 639, false, 255);
            v /= 10;
            x -= 34;
            if v == 0 {
                break;
            }
        }
    }

    fn digit(canvas: &mut Canvas, sheet: &Arc<Image>, src: (u32, u32, u32, u32), at: (i32, i32)) {
        canvas.draw_part(sheet, src, at.0, at.1, false, 255);
    }

    fn draw_cg(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        look: &Look,
    ) -> Result<()> {
        let seen = &self.saved.seen;
        self.draw_percent(canvas, sheet, percent(seen_count(seen), PICTURES.len()));
        let thumbs = assets
            .image(&format!("cgmode_thm{:02}.akb", self.page))
            .ok();
        for cell in 0..CELLS {
            let Some(e) = self.cell_entry(cell as i32) else {
                continue;
            };
            let (cx, cy) = (271 + 330 * (cell % 3) as i32, 51 + 190 * (cell / 3) as i32);
            let id = cell as i32;
            let Some(t) = thumb(seen, e) else {
                canvas.draw_part(sheet, (800, 540, 320, 180), cx, cy, false, 255);
                continue;
            };
            canvas.draw_part(sheet, (800, 0, 320, 180), cx, cy, false, 255);
            let tile = if t == 0 {
                thumbs.clone().map(|img| (img, cell))
            } else {
                assets
                    .image(&format!("{}.akb", ENTRIES[e].2))
                    .ok()
                    .map(|img| (img, t))
            };
            if let Some((img, n)) = tile {
                canvas.draw_part(
                    &img,
                    ((n % 3) as u32 * 316, (n / 3) as u32 * 176, 316, 176),
                    cx + 2,
                    cy + 2,
                    false,
                    255,
                );
            }
            let count = ENTRIES[e].1;
            if count != 1 {
                let n = seen_in(seen, e);
                if n >= 10 {
                    Self::digit(
                        canvas,
                        sheet,
                        (28 * (n as u32 / 10 % 10), 545, 28, 44),
                        (cx + 167, cy + 126),
                    );
                }
                Self::digit(
                    canvas,
                    sheet,
                    (28 * (n as u32 % 10), 545, 28, 44),
                    (cx + 195, cy + 126),
                );
                Self::digit(canvas, sheet, (280, 545, 28, 44), (cx + 223, cy + 126));
                let mut x = cx + 251;
                if count >= 10 {
                    Self::digit(
                        canvas,
                        sheet,
                        (20 * (count as u32 / 10 % 10), 589, 20, 29),
                        (x, cy + 141),
                    );
                    x += 20;
                }
                Self::digit(
                    canvas,
                    sheet,
                    (20 * (count as u32 % 10), 589, 20, 29),
                    (x, cy + 141),
                );
            }
            if look.pressed == Some(id) {
                canvas.draw_part(sheet, (800, 360, 320, 180), cx, cy, false, 255);
            } else if look.hover == Some(id) {
                canvas.draw_part(sheet, (800, 180, 320, 180), cx, cy, false, 255);
            }
        }
        canvas.draw_part(
            sheet,
            (1440, self.state(look, PREV) * 54, 54, 54),
            292,
            640,
            false,
            255,
        );
        canvas.draw_part(
            sheet,
            (1494, self.state(look, NEXT) * 54, 54, 54),
            549,
            640,
            false,
            255,
        );
        for k in 0..CG_PAGES {
            let y = if k == self.page {
                314
            } else {
                self.state(look, 0x200 + k as i32) * 49 + 216
            };
            canvas.draw_part(
                sheet,
                (1486 + 46 * k as u32, y, 46, 49),
                349 + 50 * k as i32,
                643,
                false,
                255,
            );
        }
        Ok(())
    }

    fn draw_scene(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        look: &Look,
    ) -> Result<()> {
        let n = self.saved.scenes.iter().filter(|&&b| b != 0).count();
        self.draw_percent(canvas, sheet, percent(n, SCENES));
        let thumbs = assets.image("scene_thumb.akb").ok();
        let (was, since) = self.hover_since.get();
        let over = look.hover.filter(|&h| (0..CELLS as i32).contains(&h));
        let since = if was != over {
            self.hover_since.set((over, look.ms));
            look.ms
        } else {
            since
        };
        for cell in 0..CELLS {
            let Some(s) = self.cell_scene(cell as i32) else {
                continue;
            };
            let (cx, cy) = (271 + 330 * (cell % 3) as i32, 51 + 190 * (cell / 3) as i32);
            let id = cell as i32;
            if self.saved.scenes[s] == 0 {
                canvas.draw_part(sheet, (1120, 540, 320, 180), cx, cy, false, 255);
                continue;
            }
            canvas.draw_part(sheet, (1120, 0, 320, 180), cx, cy, false, 255);
            if let Some(img) = &thumbs {
                canvas.draw_part(
                    img,
                    ((s % 3) as u32 * 316, (s / 3) as u32 * 176, 316, 176),
                    cx + 2,
                    cy + 2,
                    false,
                    255,
                );
            }
            if over != Some(id) {
                continue;
            }
            // Two frames of the scene's own thumbnail, one second still,
            // one second fading into the other.
            if let Ok(moving) = assets.image(&format!("scene_thumb{s:02}.akb")) {
                let frames = (moving.width / 316).max(1);
                let dt = look.ms.saturating_sub(since) + 500;
                let cur = ((dt / 2000) % frames as u64) as u32;
                let inside = dt % 2000;
                canvas.draw_part(
                    &moving,
                    (cur * 316, 0, 316, 176),
                    cx + 2,
                    cy + 2,
                    false,
                    255,
                );
                if inside >= 1000 {
                    let next = (cur + 1) % frames;
                    let a = ((inside - 1000) * 255 / 1000) as u8;
                    canvas.draw_part(&moving, (next * 316, 0, 316, 176), cx + 2, cy + 2, false, a);
                }
            }
            if look.pressed == Some(id) {
                canvas.draw_part(sheet, (1120, 360, 320, 180), cx, cy, false, 255);
            } else {
                canvas.draw_part(sheet, (1120, 180, 320, 180), cx, cy, false, 255);
            }
        }
        canvas.draw_part(
            sheet,
            (1547, self.state(look, PREV) * 54, 54, 54),
            292,
            640,
            false,
            255,
        );
        canvas.draw_part(
            sheet,
            (1601, self.state(look, NEXT) * 54, 54, 54),
            516,
            640,
            false,
            255,
        );
        for k in 0..SCENE_PAGES {
            let y = if k == self.scene_page {
                510
            } else {
                self.state(look, 0x200 + k as i32) * 49 + 412
            };
            canvas.draw_part(
                sheet,
                (1486 + 46 * k as u32, y, 46, 49),
                354 + 54 * k as i32,
                644,
                false,
                255,
            );
        }
        Ok(())
    }

    fn draw_music(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        look: &Look,
    ) -> Result<()> {
        let heard = self.saved.tracks.iter().filter(|&&t| t == 1).count();
        self.draw_percent(canvas, sheet, percent(heard, TRACKS));
        let list = assets.image("musiclist.akb")?;
        for row in 0..MUSIC_ROWS {
            let track = self.scroll + row;
            let state = if self.saved.tracks[track] != 1 {
                3
            } else if self.playing == Some(track) {
                2
            } else if look.hover == Some(row as i32) {
                1
            } else {
                0
            };
            canvas.draw_part(
                &list,
                (
                    (track % 2) as u32 * 894,
                    ((track / 2) as u32 * 4 + state) * 60,
                    894,
                    60,
                ),
                280,
                53 + 68 * row as i32,
                false,
                255,
            );
        }
        canvas.draw_part(
            sheet,
            (800, 168 + self.state(look, PREV) * 54, 54, 54),
            1195,
            42,
            false,
            255,
        );
        canvas.draw_part(
            sheet,
            (854, 168 + self.state(look, NEXT) * 54, 54, 54),
            1195,
            559,
            false,
            255,
        );
        let st = if look.pressed == Some(SCROLLBAR) {
            2
        } else if look.hover == Some(SCROLLBAR) {
            1
        } else {
            0
        };
        canvas.draw_part(
            sheet,
            (908, 168 + st * 83, 40, 83),
            1202,
            95 + 76 * self.scroll as i32,
            false,
            255,
        );
        let fill = (self.volume * VOLUME_SPAN).round() as i32;
        canvas.draw_part(sheet, (800, 0, 466, 56), 407, 641, false, 255);
        canvas.draw_part(
            sheet,
            (800, 56, fill.max(0) as u32, 56),
            407,
            641,
            false,
            255,
        );
        let st = if look.pressed == Some(VOLUME) {
            2
        } else if look.hover == Some(VOLUME) {
            1
        } else {
            0
        };
        canvas.draw_part(
            sheet,
            (948, 168 + st * 64, 46, 64),
            fill + 384,
            639,
            false,
            255,
        );
        Ok(())
    }

    /// The grid of the variants of an entry over the page, or the picture
    /// itself.
    fn draw_picker(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        p: &Picker,
        look: &Look,
    ) -> Result<()> {
        if let Some(id) = p.shown {
            canvas.fill(0, 0, 1280, 720, [0, 0, 0, 255]);
            let (bg, layers) = PICTURES[id];
            if let Ok(img) = assets.image(&format!("{bg}.akb")) {
                canvas.draw(&img, 0, 0, false);
            }
            for name in layers {
                if let Ok(img) = assets.image(&format!("{name}.akb")) {
                    canvas.draw(&img, 0, 0, true);
                }
            }
            return Ok(());
        }
        // The page stays visible behind a dark veil.
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 203]);
        let (first, _, file) = ENTRIES[p.entry];
        let tiles = assets.image(&format!("{file}.akb")).ok();
        let n = self.picker_cells(p);
        for i in 0..n {
            let v = p.page * CELLS + i;
            let (x, y, _, _) = picker_cell(n, i);
            if !is_seen(&self.saved.seen, first + v) {
                canvas.draw_part(sheet, (800, 540, 320, 180), x, y, false, 255);
                continue;
            }
            canvas.draw_part(sheet, (800, 0, 320, 180), x, y, false, 255);
            if let Some(img) = &tiles {
                canvas.draw_part(
                    img,
                    ((v % 3) as u32 * 316, (v / 3) as u32 * 176, 316, 176),
                    x + 2,
                    y + 2,
                    false,
                    255,
                );
            }
            if look.pressed == Some(i as i32) {
                canvas.draw_part(sheet, (800, 360, 320, 180), x, y, false, 255);
            } else if look.hover == Some(i as i32) {
                canvas.draw_part(sheet, (800, 180, 320, 180), x, y, false, 255);
            }
        }
        let st = |id| self.state(look, id);
        canvas.draw_part(
            sheet,
            (600, st(BACK).min(2) * 71, 200, 71),
            1069,
            636,
            false,
            255,
        );
        let pages = self.picker_pages(p);
        if pages > 1 {
            canvas.draw_part(sheet, (1440, st(PREV) * 54, 54, 54), 437, 645, false, 255);
            canvas.draw_part(sheet, (1494, st(NEXT) * 54, 54, 54), 741, 645, false, 255);
            let num = |canvas: &mut Canvas, v: usize, tens_x: i32, units_x: i32, y: i32| {
                if v >= 10 {
                    canvas.draw_part(
                        sheet,
                        (28 * (v as u32 / 10 % 10), 545, 28, 44),
                        tens_x,
                        y,
                        false,
                        255,
                    );
                }
                canvas.draw_part(
                    sheet,
                    (28 * (v as u32 % 10), 545, 28, 44),
                    units_x,
                    y,
                    false,
                    255,
                );
            };
            num(canvas, p.page + 1, 547, 575, 651);
            canvas.draw_part(sheet, (280, 545, 28, 44), 603, 652, false, 255);
            num(canvas, pages, 631, 659, 652);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_agree() {
        // Entries cover the pictures once, in order.
        let mut next = 0;
        for (first, count, _) in ENTRIES {
            assert_eq!(first, next);
            next += count;
        }
        assert_eq!(next, PICTURES.len());
        assert!(ENTRIES.len() <= CELLS * CG_PAGES);
    }

    #[test]
    fn percent_rounds_like_the_original() {
        assert_eq!(percent(0, 279), 0);
        assert_eq!(percent(1, 279), 1);
        assert_eq!(percent(3, 13), 23);
        assert_eq!(percent(278, 279), 99);
        assert_eq!(percent(279, 279), 100);
    }

    #[test]
    fn registers_a_picture_with_its_layers() {
        let mut bytes = vec![0u8; 4000];
        // Picture 1: a background and one layer.
        let set = register(&bytes, "EV01_02.AKB", ["ev01_01_h01.akb"].into_iter());
        assert_eq!(set[0], (CG_SEEN, 2));
        for (i, v) in set {
            bytes[i] = v;
        }
        assert!(is_seen(&bytes[CG_SEEN..], 1));
        assert_eq!(bytes[CG_ANY], 1);
        // The same background with no layer, or another one, is not it.
        assert!(register(&bytes, "ev01_02", std::iter::empty()).is_empty());
        assert!(register(&bytes, "bg01", std::iter::empty()).is_empty());
    }

    #[test]
    fn thumbnails_follow_the_first_seen() {
        let mut bits = vec![0u8; 36];
        assert_eq!(thumb(&bits, 1), None);
        // Entry 1 starts at picture 5 and has eight.
        bits[0] |= 1 << 7;
        assert_eq!(thumb(&bits, 1), Some(2));
        bits[0] |= 1 << 5;
        assert_eq!(thumb(&bits, 1), Some(0));
        assert_eq!(seen_in(&bits, 1), 2);
        assert_eq!(entry_of(7), 1);
    }

    fn music_screen() -> Omake {
        let mut saved = Saved::default();
        saved.tracks = [1; TRACKS];
        saved.tabs = [1; MODES];
        Omake::open(2, [Vec::new(), Vec::new(), Vec::new()], saved)
    }

    #[test]
    fn buttons_play_the_confirm_sound() {
        let mut o = music_screen();
        o.click(0);
        assert_eq!(o.take_sound().as_deref(), Some("ses_001.wav"));
        assert!(o.take_sound().is_none());
        // The bars make no sound.
        o.click(VOLUME);
        o.click(SCROLLBAR);
        assert!(o.take_sound().is_none());
        // A button that cannot work makes none either.
        o.click(PREV);
        assert!(o.take_sound().is_none());
        o.click(NEXT);
        assert!(o.take_sound().is_some());
    }

    #[test]
    fn right_click_stops_a_track_before_it_leaves() {
        let mut o = music_screen();
        o.click(0);
        o.take_sound();
        assert!(matches!(o.cancel(), Click::None));
        assert!(matches!(o.take_music(), Some(Music::Stop)));
        assert!(o.take_sound().is_none());
        assert!(matches!(o.cancel(), Click::Result(LEAVE)));
        assert_eq!(o.take_sound().as_deref(), Some("ses_002.wav"));
    }

    #[test]
    fn a_track_that_ended_clears_the_row() {
        let mut o = music_screen();
        o.click(1);
        assert!(o.track_playing());
        assert!(o.track_ended());
        assert!(!o.track_playing());
    }

    #[test]
    fn variant_grids_are_centred() {
        assert_eq!(picker_cell(9, 0), (149, 16 + 25, 320, 180));
        let (x, _, _, _) = picker_cell(1, 0);
        assert_eq!(x, 479);
    }
}
