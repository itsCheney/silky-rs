//! The extras of きまぐれテンプテーション: the CG, scene and music modes
//! (`Game.30` to `Game.32`).
//!
//! The three modes are tabs of one screen: a press on a tab changes the
//! mode without going back to the script. The CG mode shows its entries
//! (a picture and its variants) three pages of twelve, and opens an entry
//! as a grid of its variants, then a variant full screen. The scene mode
//! answers the scene the player picked; the script plays it. The music
//! mode lists the 24 tracks. Each has a button that unlocks everything
//! after a question.
//!
//! Seen pictures are bits from byte 3300, scenes seen are bytes 3101 to
//! 3108, tracks heard bytes 3051 to 3074, and bytes 0xbba to 0xbbc open
//! the tabs of the modes.

use std::cell::Cell;
use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;

use super::cgdata::{ENTRIES, PICTURES};
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// First byte of the pictures seen, one bit each.
pub const CG_SEEN: usize = 3300;
/// Bytes set once a picture is seen, and once all are unlocked.
pub const CG_ANY: usize = 3002;
pub const CG_UNLOCKED: usize = 3001;
/// First byte of the scenes seen, and of the tracks heard.
pub const SCENE_FIRST: usize = 3101;
pub const MUSIC_FIRST: usize = 3051;
/// Bytes that open the tabs of the three modes.
pub const TAB_BYTES: [usize; 3] = [0xbba, 0xbbb, 0xbbc];
pub const SCENES: usize = 8;
pub const TRACKS: usize = 24;
/// What the modes answer to leave to the title.
pub const LEAVE: i32 = -1;

const MODES: usize = 3;
const BACKGROUNDS: [&str; MODES] = ["cgmode_bg.akb", "scenemode_bg.akb", "music_bg.akb"];
const SHEETS: [&str; MODES] = ["cgmode_p.akb", "scenemode_p.akb", "music_p.akb"];
const AREAS: [&str; MODES] = ["CGMODE.AREA", "SCMODE.AREA", "SOUNDMODE.AREA"];
const TAB_AT: [(i32, i32); MODES] = [(38, 121), (8, 300), (38, 479)];
const TAB_SIZE: (u32, u32) = (210, 164);
const PAGES: usize = 3;
const PER_PAGE: usize = 12;
const MUSIC_ROWS: usize = 8;
const SCROLL_MAX: usize = TRACKS - MUSIC_ROWS;
const VOLUME_LEFT: i32 = 817;
const VOLUME_SPAN: f32 = 183.0;
/// Ids of the dialog's buttons and of the picture of the view.
const YES: i32 = 0xd00;
const NO: i32 = 0xd01;
const PICTURE: i32 = 0x900;
/// Ids of the buttons of a view of an entry's variants (the ids of the
/// cells are 0x000 to 0x00b as on the grid).
const BACK: i32 = 0x300;
const UNLOCK: i32 = 0x400;
const PREV: i32 = 0x100;
const NEXT: i32 = 0x101;
const SCROLLBAR: i32 = 0x102;
const VOLUME: i32 = 0x200;

/// The script variable that makes the first press on a picture cell silent.
const QUIET_VAR: usize = 16;

/// The sounds of a press and of leaving by a right click.
const OK_SOUND: &str = "ses_001.wav";
const CANCEL_SOUND: &str = "ses_002.wav";

/// What the music mode asks of the sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Music {
    Play(String),
    Stop,
}

/// What the screen starts from: the bytes of the script.
#[derive(Debug, Clone, Default)]
pub struct Saved {
    pub seen: Vec<u8>,
    pub scenes: [u8; SCENES],
    pub tracks: [u8; TRACKS],
    pub tabs: [u8; MODES],
    pub volume: f32,
    /// Script variable 16 was 1 when the screen opened.
    pub quiet_cell: bool,
}

/// An entry of the CG mode opened as a grid or a picture.
#[derive(Debug, Clone, Copy)]
struct View {
    entry: usize,
    page: usize,
    /// The picture on screen, if one is.
    shown: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Omake {
    mode: usize,
    areas: [Vec<Area>; MODES],
    dialog_areas: Vec<Area>,
    saved: Saved,
    page: usize,
    view: Option<View>,
    dialog: bool,
    scroll: usize,
    volume: f32,
    playing: Option<usize>,
    bytes: Vec<(usize, u8)>,
    music: Option<Music>,
    new_volume: Option<f32>,
    sound: Option<&'static str>,
    /// While script variable 16 is 1 the first press on a picture cell
    /// makes no sound and sets the variable back to 0.
    quiet_cell: bool,
    globals: Vec<(usize, i32)>,
    /// The cell under the pointer on the scene page and since when.
    hover_since: Cell<(Option<i32>, u64)>,
}

/// Pictures of an entry: its first, how many.
fn entry(i: usize) -> (usize, usize, &'static str) {
    ENTRIES[i]
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

/// The number of the picture the file `name` is, if it is one: a picture
/// of the table shown alone (no layers over it).
pub fn picture_of(name: &str) -> Option<usize> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".akb").unwrap_or(&lower);
    PICTURES.iter().position(|p| *p == stem)
}

/// Sets the bit of the picture the engine just showed (`overlays` says
/// whether other layers are over it). Returns the bytes to set: the bit's
/// byte, and the two that say something is seen. Empty if it is not a
/// picture of the table or covered.
pub fn register(bytes: &[u8], background: &str, overlays: bool) -> Vec<(usize, u8)> {
    if overlays {
        return Vec::new();
    }
    let Some(id) = picture_of(background) else {
        return Vec::new();
    };
    let at = CG_SEEN + id / 8;
    let byte = bytes.get(at).copied().unwrap_or(0) | 1 << (id % 8);
    vec![(at, byte), (CG_ANY, 1), (CG_UNLOCKED, 1)]
}

/// The entry's thumbnail: 0 if its first picture is seen, else the first
/// seen one's place in it, `None` if none is.
fn thumb(bits: &[u8], e: usize) -> Option<usize> {
    let (first, count, _) = entry(e);
    if is_seen(bits, first) {
        return Some(0);
    }
    (1..count).find(|&i| is_seen(bits, first + i))
}

fn seen_in(bits: &[u8], e: usize) -> usize {
    let (first, count, _) = entry(e);
    (0..count).filter(|&i| is_seen(bits, first + i)).count()
}

impl Omake {
    pub fn open(mode: usize, areas: [Vec<Area>; MODES], dialog: Vec<Area>, saved: Saved) -> Self {
        let volume = saved.volume;
        let saved_quiet = saved.quiet_cell;
        Self {
            mode: mode.min(MODES - 1),
            areas,
            dialog_areas: dialog,
            saved,
            page: 0,
            view: None,
            dialog: false,
            scroll: 0,
            volume,
            playing: None,
            bytes: Vec::new(),
            music: None,
            new_volume: None,
            sound: None,
            quiet_cell: saved_quiet,
            globals: Vec::new(),
            hover_since: Cell::new((None, 0)),
        }
    }

    /// Script variables (index, value) the screen set (taken once).
    pub fn take_globals(&mut self) -> Vec<(usize, i32)> {
        std::mem::take(&mut self.globals)
    }

    /// The names of the area files, in the order of the modes.
    pub fn area_files() -> [&'static str; MODES] {
        AREAS
    }

    /// Whether the tab of mode `k` can be pressed.
    fn tab_open(&self, k: usize) -> bool {
        k != self.mode && self.saved.tabs[k] != 0
    }

    fn entries_on_page(&self) -> usize {
        ENTRIES
            .len()
            .saturating_sub(self.page * PER_PAGE)
            .min(PER_PAGE)
    }

    fn cell_entry(&self, cell: i32) -> Option<usize> {
        let e = self.page * PER_PAGE + cell as usize;
        (cell >= 0 && (cell as usize) < PER_PAGE && e < ENTRIES.len()).then_some(e)
    }

    fn view_cells(&self, v: &View) -> usize {
        let (_, count, _) = entry(v.entry);
        count.saturating_sub(v.page * PER_PAGE).min(PER_PAGE)
    }

    fn view_pages(&self, v: &View) -> usize {
        entry(v.entry).1.div_ceil(PER_PAGE)
    }

    /// Top left of the grid of a view, and its cell step.
    fn view_origin(&self, v: &View) -> (i32, i32) {
        let n = self.view_cells(v);
        let width = if n < 4 { n as i32 * 239 } else { 956 };
        let height = n.div_ceil(4) as i32 * 139;
        ((1280 - width) / 2 + 4, (690 - height) / 2 + 16)
    }

    fn view_cell_rect(&self, v: &View, i: usize) -> (i32, i32, i32, i32) {
        let (x0, y0) = self.view_origin(v);
        (
            x0 + 239 * (i % 4) as i32,
            y0 + 139 * (i / 4) as i32,
            234,
            134,
        )
    }

    /// The element under (x, y).
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if self.dialog {
            return self
                .dialog_areas
                .iter()
                .find(|a| a.id < 2 && a.contains(x, y))
                .map(|a| YES + a.id);
        }
        if let Some(v) = &self.view {
            if v.shown.is_some() {
                return Some(PICTURE);
            }
            for i in 0..self.view_cells(v) {
                let (l, t, w, h) = self.view_cell_rect(v, i);
                if x >= l && x < l + w && y >= t && y < t + h {
                    return Some(i as i32);
                }
            }
            return [
                (PREV, 535, 671, 589, 711),
                (NEXT, 695, 671, 749, 711),
                (BACK, 1191, 655, 1269, 712),
            ]
            .iter()
            .find(|r| x >= r.1 && x < r.3 && y >= r.2 && y < r.4)
            .map(|r| r.0);
        }
        self.areas[self.mode]
            .iter()
            .find(|a| self.wanted(a.id) && a.contains(x, y))
            .map(|a| a.id)
    }

    /// Whether the area `id` of the mode's file is one this screen has.
    fn wanted(&self, id: i32) -> bool {
        match self.mode {
            0 => {
                matches!(id, 0..=11 | 0x100 | 0x101 | 0x200..=0x202 | 0x300 | 0x400 | 0xa01 | 0xa02)
            }
            1 => matches!(id, 0..=7 | 0x300 | 0x400 | 0xa00 | 0xa02),
            _ => matches!(
                id,
                0..=7 | 0x100 | 0x101 | 0x102 | 0x200 | 0x300 | 0x400 | 0xa00 | 0xa01
            ),
        }
    }

    pub fn enabled(&self, id: i32) -> bool {
        if self.dialog {
            return true;
        }
        if let Some(v) = &self.view {
            return match id {
                PREV => v.page > 0,
                NEXT => v.page + 1 < self.view_pages(v),
                0..=11 => {
                    let (first, count, _) = entry(v.entry);
                    let i = v.page * PER_PAGE + id as usize;
                    i < count && is_seen(&self.saved.seen, first + i)
                }
                _ => true,
            };
        }
        match (self.mode, id) {
            (_, 0xa00..=0xa02) => self.tab_open((id & 0xff) as usize),
            (0, 0..=11) => self
                .cell_entry(id)
                .is_some_and(|e| thumb(&self.saved.seen, e).is_some()),
            (0, 0x100) => self.page > 0,
            (0, 0x101) => self.page + 1 < PAGES,
            (1, 0..=7) => self.saved.scenes[id as usize] != 0,
            (2, 0..=7) => self.saved.tracks[self.scroll + id as usize] == 1,
            (2, 0x100) => self.scroll > 0,
            (2, 0x101) => self.scroll < SCROLL_MAX,
            _ => true,
        }
    }

    fn set_bytes(&mut self, list: impl IntoIterator<Item = (usize, u8)>) {
        for (i, v) in list {
            self.bytes.push((i, v));
            match i {
                CG_SEEN.. if i < CG_SEEN + self.saved.seen.len() => {
                    self.saved.seen[i - CG_SEEN] = v
                }
                SCENE_FIRST..=3108 => self.saved.scenes[i - SCENE_FIRST] = v,
                MUSIC_FIRST..=3074 => self.saved.tracks[i - MUSIC_FIRST] = v,
                _ => {}
            }
        }
    }

    /// What "unlock all" sets in the mode.
    fn unlock_all(&mut self) {
        let list: Vec<(usize, u8)> = match self.mode {
            0 => {
                let mut v: Vec<(usize, u8)> = (0..PICTURES.len().div_ceil(8))
                    .map(|i| {
                        let n = (PICTURES.len() - i * 8).min(8);
                        (CG_SEEN + i, (((1u32 << n) - 1) & 0xff) as u8)
                    })
                    .collect();
                v.push((CG_ANY, 1));
                v.push((CG_UNLOCKED, 1));
                v
            }
            1 => (0..SCENES).map(|i| (SCENE_FIRST + i, 1)).collect(),
            _ => (0..TRACKS).map(|i| (MUSIC_FIRST + i, 1)).collect(),
        };
        self.set_bytes(list);
    }

    pub fn click(&mut self, id: i32) -> Click {
        // The question's own buttons: yes answers like a press, no like
        // a cancel.
        if self.dialog {
            self.dialog = false;
            if id == YES {
                self.sound = Some(OK_SOUND);
                self.unlock_all();
            } else {
                self.sound = Some(CANCEL_SOUND);
            }
            return Click::None;
        }
        if self.view.is_some() {
            return self.click_view(id);
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
            // Leaving by the button sounds like any press.
            BACK => {
                self.sound = Some(OK_SOUND);
                return Click::Result(LEAVE);
            }
            UNLOCK => {
                self.sound = Some(OK_SOUND);
                self.dialog = true;
                return Click::None;
            }
            _ => {}
        }
        match self.mode {
            0 => match id {
                0..=11 => {
                    if let Some(e) = self.cell_entry(id) {
                        if std::mem::take(&mut self.quiet_cell) {
                            self.globals.push((QUIET_VAR, 0));
                        } else {
                            self.sound = Some(OK_SOUND);
                        }
                        self.view = Some(View {
                            entry: e,
                            page: 0,
                            shown: None,
                        });
                    }
                }
                PREV if self.page > 0 => {
                    self.sound = Some(OK_SOUND);
                    self.page -= 1;
                }
                NEXT if self.page + 1 < PAGES => {
                    self.sound = Some(OK_SOUND);
                    self.page += 1;
                }
                0x200..=0x202 if (id & 0xff) as usize != self.page => {
                    self.sound = Some(OK_SOUND);
                    self.page = (id & 0xff) as usize;
                }
                _ => {}
            },
            1 => {
                if (0..SCENES as i32).contains(&id) && self.saved.scenes[id as usize] != 0 {
                    self.sound = Some(OK_SOUND);
                    return Click::Result(id);
                }
            }
            _ => self.click_music(id),
        }
        Click::None
    }

    fn click_view(&mut self, id: i32) -> Click {
        let Some(mut v) = self.view else {
            return Click::None;
        };
        if v.shown.is_some() {
            // The next picture seen, in order; the grid when none is left.
            let (first, count, _) = entry(v.entry);
            let start = first + v.page * PER_PAGE;
            let _ = (start, count);
            let from = v.shown.unwrap_or(0) + 1;
            match (from..PICTURES.len()).find(|&i| is_seen(&self.saved.seen, i)) {
                Some(i) => v.shown = Some(i),
                None => v.shown = None,
            }
            self.view = Some(v);
            return Click::None;
        }
        match id {
            BACK => {
                self.sound = Some(OK_SOUND);
                self.view = None;
            }
            PREV if v.page > 0 => {
                self.sound = Some(OK_SOUND);
                v.page -= 1;
            }
            NEXT if v.page + 1 < self.view_pages(&v) => {
                self.sound = Some(OK_SOUND);
                v.page += 1;
            }
            0..=11 => {
                self.sound = Some(OK_SOUND);
                let (first, _, _) = entry(v.entry);
                v.shown = Some(first + v.page * PER_PAGE + id as usize);
            }
            _ => {}
        }
        if self.view.is_some() {
            self.view = Some(v);
        }
        Click::None
    }

    fn click_music(&mut self, id: i32) {
        match id {
            0..=7 => {
                let track = self.scroll + id as usize;
                if self.saved.tracks[track] != 1 {
                    return;
                }
                self.sound = Some(OK_SOUND);
                if self.playing == Some(track) {
                    self.playing = None;
                    self.music = Some(Music::Stop);
                } else {
                    self.playing = Some(track);
                    self.music = Some(Music::Play(track_file(track)));
                }
            }
            PREV if self.scroll > 0 => {
                self.sound = Some(OK_SOUND);
                self.scroll -= 1;
            }
            NEXT if self.scroll < SCROLL_MAX => {
                self.sound = Some(OK_SOUND);
                self.scroll += 1;
            }
            _ => {}
        }
    }

    /// A press at (x, y) on the scroll bar or the volume bar, which the
    /// original follows while the button is held; here a press sets it.
    pub fn press_at(&mut self, id: i32, x: i32, y: i32) {
        if self.mode != 2 || self.dialog || self.view.is_some() {
            return;
        }
        match id {
            SCROLLBAR => {
                let t = ((y - 120) as f32 / 446.0).clamp(0.0, 1.0);
                self.scroll = (t * SCROLL_MAX as f32).round() as usize;
            }
            VOLUME => {
                let px = (x - VOLUME_LEFT) as f32;
                self.volume = (px / VOLUME_SPAN).clamp(0.0, 1.0);
                self.new_volume = Some(self.volume);
            }
            _ => {}
        }
    }

    /// Right click or Escape. The question and the view of an entry go
    /// without a sound. On the modes themselves it leaves with the cancel
    /// sound; the music mode first stops the track that plays.
    pub fn cancel(&mut self) -> Click {
        if self.dialog {
            self.dialog = false;
        } else if let Some(v) = &mut self.view {
            if v.shown.is_some() {
                v.shown = None;
            } else {
                self.view = None;
            }
        } else if self.mode == 2 && self.playing.is_some() {
            self.playing = None;
            self.music = Some(Music::Stop);
        } else {
            self.sound = Some(CANCEL_SOUND);
            return Click::Result(LEAVE);
        }
        Click::None
    }

    /// Whether a right click goes by the right-click setting: on the music
    /// mode with no question or view up.
    pub fn follows_setting(&self) -> bool {
        self.mode == 2 && !self.dialog && self.view.is_none()
    }

    /// The sound of the last press (taken once).
    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        std::mem::take(&mut self.bytes)
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

    fn help(&self, hover: Option<i32>) -> Option<String> {
        let id = hover?;
        if self.dialog {
            return None;
        }
        if let Some(v) = &self.view {
            let _ = v;
            return None;
        }
        let tab = |k: usize| match k {
            0 => "CGモードに移動します",
            1 => "シーンモードに移動します",
            _ => "音楽鑑賞モードに移動します",
        };
        Some(match id {
            BACK => "タイトル画面に戻ります".to_string(),
            UNLOCK => {
                "未解放のCGも全て解放状態にします".replace("CG", ["CG", "シーン", "曲"][self.mode])
            }
            0xa00..=0xa02 => tab((id & 0xff) as usize).to_string(),
            _ => match (self.mode, id) {
                (0, 0..=11) => format!("CG{:02}を表示します", self.page * PER_PAGE + id as usize),
                (0, 0x100) => "1ページ前を開きます".to_string(),
                (0, 0x101) => "1ページ後ろを開きます".to_string(),
                (0, 0x200..=0x202) => format!("{}ページを開きます", id & 0xff),
                (1, 0..=7) => format!("シーン{:02}を再生します", id),
                _ => return None,
            },
        })
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image(SHEETS[self.mode])?;
        let bg = assets.image(BACKGROUNDS[self.mode])?;
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 255]);
        canvas.draw(&bg, 0, 0, false);
        match self.mode {
            0 => self.draw_cg(canvas, assets, &sheet, look)?,
            1 => self.draw_scene(canvas, assets, &sheet, look)?,
            _ => self.draw_music(canvas, assets, &sheet, look)?,
        }
        self.draw_common(canvas, &sheet, look);
        if let Some(v) = &self.view {
            self.draw_view(canvas, assets, &sheet, v, look)?;
        }
        if self.dialog {
            self.draw_dialog(canvas, assets, look)?;
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

    fn blocked(&self) -> bool {
        self.dialog || self.view.is_some()
    }

    /// Tabs, the back and unlock buttons, the information strip.
    fn draw_common(&self, canvas: &mut Canvas, sheet: &Arc<Image>, look: &Look) {
        let calm = Look {
            hover: None,
            pressed: None,
            ..*look
        };
        let look = if self.blocked() { &calm } else { look };
        for k in 0..MODES {
            let (x, y) = TAB_AT[k];
            if k == self.mode {
                canvas.draw_part(
                    sheet,
                    (210 * k as u32, 328, TAB_SIZE.0, TAB_SIZE.1),
                    x,
                    y,
                    false,
                    255,
                );
            } else {
                let st = if self.tab_open(k) {
                    self.state(look, 0xa00 + k as i32)
                } else {
                    3
                };
                canvas.draw_part(
                    sheet,
                    (210 * k as u32, st * TAB_SIZE.1, TAB_SIZE.0, TAB_SIZE.1),
                    x,
                    y,
                    false,
                    255,
                );
            }
        }
        let back = self.state(look, BACK);
        let unlock = self.state(look, UNLOCK);
        if self.mode == 2 {
            canvas.draw_part(sheet, (820, back * 57 + 60, 78, 57), 1191, 651, false, 255);
            canvas.draw_part(sheet, (1047, unlock * 64, 71, 64), 1203, 9, false, 255);
        } else {
            canvas.draw_part(sheet, (630, back * 57 + 536, 78, 57), 1191, 651, false, 255);
            canvas.draw_part(sheet, (708, 536 + unlock * 64, 71, 64), 1203, 9, false, 255);
        }
        canvas.draw_part(sheet, (0, 656, 513, 30), 664, 690, false, 255);
        if let (Some(text), Some(line)) = (look.text, self.help(look.hover)) {
            text.draw_plain(canvas, &line, 782.0, 713.0, 16.0, [255, 255, 255, 255]);
        }
    }

    fn draw_percent(&self, canvas: &mut Canvas, sheet: &Arc<Image>, value: u32) {
        canvas.draw_part(sheet, (430, 686, 43, 48), 1169, 97, false, 255);
        let mut v = value;
        let mut x = 1126;
        loop {
            canvas.draw_part(sheet, (43 * (v % 10), 686, 43, 48), x, 97, false, 255);
            v /= 10;
            x -= 43;
            if v == 0 {
                break;
            }
        }
    }

    /// Two digits at fixed places: tens (if any) and units.
    fn digits(
        canvas: &mut Canvas,
        sheet: &Arc<Image>,
        n: usize,
        tens_at: (i32, i32),
        units_at: (i32, i32),
        glyph: (u32, u32, u32),
    ) {
        let (w, h, base) = glyph;
        if n >= 10 {
            canvas.draw_part(
                sheet,
                (base + w * (n as u32 / 10 % 10), 734, w, h),
                tens_at.0,
                tens_at.1,
                false,
                255,
            );
        }
        canvas.draw_part(
            sheet,
            (base + w * (n as u32 % 10), 734, w, h),
            units_at.0,
            units_at.1,
            false,
            255,
        );
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
        for cell in 0..self.entries_on_page() {
            let e = self.page * PER_PAGE + cell;
            let (cx, cy) = (276 + 239 * (cell % 4) as i32, 164 + 139 * (cell / 4) as i32);
            let t = thumb(seen, e);
            let id = cell as i32;
            let hot = !self.blocked() && t.is_some();
            match t {
                None => {
                    canvas.draw_part(sheet, (630, 402, 234, 134), cx, cy, false, 255);
                }
                Some(t) => {
                    canvas.draw_part(sheet, (630, 0, 234, 134), cx, cy, false, 255);
                    let tile = if t == 0 {
                        thumbs.clone().map(|img| (img, cell))
                    } else {
                        assets
                            .image(&format!("{}.akb", entry(e).2))
                            .ok()
                            .map(|img| (img, t))
                    };
                    if let Some((img, n)) = tile {
                        canvas.draw_part(
                            &img,
                            ((n % 4) as u32 * 228, (n / 4) as u32 * 128, 228, 128),
                            cx + 3,
                            cy + 3,
                            false,
                            255,
                        );
                    }
                    let (_, count, _) = entry(e);
                    if count != 1 {
                        Self::digits(
                            canvas,
                            sheet,
                            seen_in(seen, e),
                            (cx + 0x8d, cy + 0x5f),
                            (cx + 0x9e, cy + 0x5f),
                            (17, 26, 0),
                        );
                        canvas.draw_part(
                            sheet,
                            (170, 734, 17, 26),
                            cx + 0xad,
                            cy + 0x63,
                            false,
                            255,
                        );
                        Self::digits(
                            canvas,
                            sheet,
                            count,
                            (cx + 0xb8, cy + 0x6b),
                            (cx + 0xc6, cy + 0x6b),
                            (14, 20, 187),
                        );
                    }
                }
            }
            if hot && look.pressed == Some(id) {
                canvas.draw_part(sheet, (630, 268, 234, 134), cx, cy, false, 255);
            } else if hot && look.hover == Some(id) {
                canvas.draw_part(sheet, (630, 134, 234, 134), cx, cy, false, 255);
            }
        }
        let st = |id| {
            if self.blocked() {
                if self.enabled(id) {
                    0
                } else {
                    3
                }
            } else {
                self.state(look, id)
            }
        };
        canvas.draw_part(sheet, (864, st(0x100) * 40, 54, 40), 551, 586, false, 255);
        canvas.draw_part(sheet, (918, st(0x101) * 40, 54, 40), 889, 586, false, 255);
        for k in 0..PAGES {
            let y = if k == self.page {
                80
            } else {
                st(0x200 + k as i32) * 40
            };
            canvas.draw_part(
                sheet,
                (972 + 74 * k as u32, y, 74, 40),
                619 + 90 * k as i32,
                586,
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
        // The percent counts the first six scenes, as the original does.
        let n = self.saved.scenes[..6].iter().filter(|&&b| b != 0).count();
        self.draw_percent(canvas, sheet, percent(n, 6));
        let thumbs = assets.image("scene_thumb.akb").ok();
        let (was, since) = self.hover_since.get();
        let over = if self.blocked() {
            None
        } else {
            look.hover.filter(|&h| (0..SCENES as i32).contains(&h))
        };
        if was != over {
            self.hover_since.set((over, look.ms));
        }
        let since = if was != over { look.ms } else { since };
        for cell in 0..SCENES {
            let (cx, cy) = (324 + 290 * (cell % 3) as i32, 160 + 157 * (cell / 3) as i32);
            let id = cell as i32;
            if self.saved.scenes[cell] == 0 {
                canvas.draw_part(sheet, (630, 450, 262, 150), cx, cy, false, 255);
                continue;
            }
            canvas.draw_part(sheet, (630, 0, 262, 150), cx, cy, false, 255);
            if over == Some(id) {
                let moving = assets.image(&format!("scene_thumb{cell:02}.akb")).ok();
                if let Some(img) = moving {
                    let frames = (img.width / 256).max(1);
                    let dt = look.ms.saturating_sub(since) + 500;
                    let cycle = 2000u64;
                    let step = dt / cycle;
                    let cur = (step % frames as u64) as u32;
                    let inside = dt % cycle;
                    canvas.draw_part(&img, (cur * 256, 0, 256, 144), cx + 3, cy + 3, false, 255);
                    if inside >= 1000 {
                        let next = (cur + 1) % frames;
                        let a = ((inside - 1000) * 255 / 1000) as u8;
                        canvas.draw_part(&img, (next * 256, 0, 256, 144), cx + 3, cy + 3, false, a);
                    }
                }
                if look.pressed == Some(id) {
                    canvas.draw_part(sheet, (630, 300, 262, 150), cx, cy, false, 255);
                } else {
                    canvas.draw_part(sheet, (630, 150, 262, 150), cx, cy, false, 255);
                }
            } else if let Some(img) = &thumbs {
                canvas.draw_part(
                    img,
                    ((cell % 3) as u32 * 256, (cell / 3) as u32 * 144, 256, 144),
                    cx + 3,
                    cy + 3,
                    false,
                    255,
                );
            }
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
        let list = assets.image("musiclist_p.akb")?;
        let look_c = Look {
            hover: None,
            pressed: None,
            ..*look
        };
        let l = if self.blocked() { &look_c } else { look };
        for row in 0..MUSIC_ROWS {
            let track = self.scroll + row;
            let state = if self.saved.tracks[track] != 1 {
                3
            } else if self.playing == Some(track) {
                2
            } else if l.pressed == Some(row as i32) {
                2
            } else if l.hover == Some(row as i32) {
                1
            } else {
                0
            };
            canvas.draw_part(
                &list,
                (
                    (track % 2) as u32 * 768,
                    (state + (track / 2) as u32 * 4) * 64,
                    768,
                    64,
                ),
                291,
                114 + 64 * row as i32,
                false,
                255,
            );
        }
        canvas.draw_part(
            sheet,
            (679, self.state(l, PREV) * 44, 49, 44),
            1063,
            76,
            false,
            255,
        );
        canvas.draw_part(
            sheet,
            (728, self.state(l, NEXT) * 44, 49, 44),
            1063,
            629,
            false,
            255,
        );
        canvas.draw_part(sheet, (1118, 0, 28, 516), 1074, 116, false, 255);
        let y = 120 + (self.scroll as f32 * 446.0 / SCROLL_MAX as f32) as i32;
        let st = if l.pressed == Some(SCROLLBAR) {
            2
        } else if l.hover == Some(SCROLLBAR) {
            1
        } else {
            0
        };
        canvas.draw_part(sheet, (630, st * 63, 49, 63), 1063, y, false, 255);
        let px = (self.volume * VOLUME_SPAN) as i32;
        canvas.draw_part(sheet, (777, 0, 270, 30), 736, 642, false, 255);
        canvas.draw_part(sheet, (777, 30, (px + 81) as u32, 30), 736, 642, false, 255);
        let st = if l.pressed == Some(VOLUME) {
            2
        } else if l.hover == Some(VOLUME) {
            1
        } else {
            0
        };
        canvas.draw_part(
            sheet,
            (777, st * 40 + 60, 43, 40),
            796 + px,
            637,
            false,
            255,
        );
        Ok(())
    }

    fn draw_view(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        v: &View,
        look: &Look,
    ) -> Result<()> {
        if let Some(id) = v.shown {
            canvas.fill(0, 0, 1280, 720, [0, 0, 0, 255]);
            if let Ok(img) = assets.image(&format!("{}.akb", PICTURES[id])) {
                canvas.draw(&img, 0, 0, false);
            }
            return Ok(());
        }
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 203]);
        let (first, count, file) = entry(v.entry);
        let tiles = assets.image(&format!("{file}.akb")).ok();
        for i in 0..self.view_cells(v) {
            let n = v.page * PER_PAGE + i;
            let (x, y, _, _) = self.view_cell_rect(v, i);
            if !is_seen(&self.saved.seen, first + n) {
                canvas.draw_part(sheet, (630, 402, 234, 134), x, y, false, 255);
                continue;
            }
            canvas.draw_part(sheet, (630, 0, 234, 134), x, y, false, 255);
            if let Some(img) = &tiles {
                canvas.draw_part(
                    img,
                    ((n % 4) as u32 * 228, (n / 4) as u32 * 128, 228, 128),
                    x + 3,
                    y + 3,
                    false,
                    255,
                );
            }
            if look.pressed == Some(i as i32) {
                canvas.draw_part(sheet, (630, 268, 234, 134), x, y, false, 255);
            } else if look.hover == Some(i as i32) {
                canvas.draw_part(sheet, (630, 134, 234, 134), x, y, false, 255);
            }
        }
        canvas.draw_part(sheet, (0, 855, 1280, 65), 0, 655, false, 255);
        let st = |id| self.state(look, id);
        canvas.draw_part(sheet, (864, st(PREV) * 40, 54, 40), 535, 671, false, 255);
        canvas.draw_part(sheet, (918, st(NEXT) * 40, 54, 40), 695, 671, false, 255);
        canvas.draw_part(
            sheet,
            (630, st(BACK).min(2) * 57 + 536, 78, 57),
            1191,
            655,
            false,
            255,
        );
        let pages = count.div_ceil(PER_PAGE);
        if pages > 1 {
            Self::digits(
                canvas,
                sheet,
                v.page + 1,
                (609, 667),
                (626, 667),
                (17, 26, 0),
            );
            canvas.draw_part(sheet, (170, 734, 17, 26), 639, 673, false, 255);
            Self::digits(canvas, sheet, pages, (652, 679), (666, 679), (14, 20, 187));
        }
        Ok(())
    }

    fn draw_dialog(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image("dialog_p.akb")?;
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 127]);
        canvas.draw_part(&sheet, (0, 0, 605, 276), 338, 115, false, 255);
        canvas.draw_part(&sheet, (206, 248, 123, 19), 683, 352, false, 255);
        canvas.draw_part(&sheet, (878, 200, 380, 29), 450, 215, false, 255);
        canvas.draw_part(
            &sheet,
            (878, 229 + 51 * self.mode as u32, 380, 51),
            450,
            244,
            false,
            255,
        );
        let st = |id| match (look.pressed == Some(id), look.hover == Some(id)) {
            (true, _) => 2,
            (_, true) => 1,
            _ => 0,
        };
        canvas.draw_part(
            &sheet,
            (605, 200 + 51 * st(YES), 94, 51),
            494,
            295,
            false,
            255,
        );
        canvas.draw_part(
            &sheet,
            (699, 200 + 51 * st(NO), 94, 51),
            694,
            295,
            false,
            255,
        );
        Ok(())
    }
}

/// The file of track `i` (from 0).
pub fn track_file(i: usize) -> String {
    format!("kima_{:02}.wav", i + 1)
}

/// The background music the modes start with when none plays.
pub const BGM: &str = "kima_21.wav";

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
    }

    fn screen(mode: usize) -> Omake {
        let saved = Saved {
            seen: vec![0; PICTURES.len().div_ceil(8)],
            scenes: [1; SCENES],
            tracks: [1; TRACKS],
            tabs: [1; MODES],
            volume: 1.0,
            quiet_cell: false,
        };
        Omake::open(mode, [vec![], vec![], vec![]], vec![], saved)
    }

    #[test]
    fn the_first_cell_press_is_quiet_while_variable_16_is_set() {
        let mut o = screen(0);
        o.quiet_cell = true;
        o.click(0);
        assert_eq!(o.take_sound(), None);
        assert_eq!(o.take_globals(), vec![(16, 0)]);
        // The press after it sounds as usual.
        o.view = None;
        o.click(1);
        assert_eq!(o.take_sound().as_deref(), Some(OK_SOUND));
        assert!(o.take_globals().is_empty());
    }

    #[test]
    fn only_the_music_mode_follows_the_right_click_setting() {
        let follows: Vec<usize> = (0..3).filter(|&m| screen(m).follows_setting()).collect();
        assert_eq!(follows.len(), 1);
        let mut o = screen(follows[0]);
        o.dialog = true;
        assert!(!o.follows_setting());
    }

    #[test]
    fn presses_sound_and_right_click_leaves() {
        let mut o = screen(1);
        assert_eq!(o.click(0), Click::Result(0));
        assert_eq!(o.take_sound().as_deref(), Some(OK_SOUND));
        o.click(0xa02);
        assert_eq!(o.take_sound().as_deref(), Some(OK_SOUND));
        // A track plays, then a right click stops it with no sound.
        o.click(0);
        o.take_sound();
        assert_eq!(o.cancel(), Click::None);
        assert_eq!(o.take_sound(), None);
        assert_eq!(o.music, Some(Music::Stop));
        // With none playing it leaves with the cancel sound.
        assert_eq!(o.cancel(), Click::Result(LEAVE));
        assert_eq!(o.take_sound().as_deref(), Some(CANCEL_SOUND));
        // The question: yes sounds like a press, no like a cancel.
        o.click(UNLOCK);
        o.take_sound();
        o.click(NO);
        assert_eq!(o.take_sound().as_deref(), Some(CANCEL_SOUND));
        o.click(UNLOCK);
        o.click(YES);
        assert_eq!(o.take_sound().as_deref(), Some(OK_SOUND));
        // Leaving by the button sounds as a press.
        assert_eq!(o.click(BACK), Click::Result(LEAVE));
        assert_eq!(o.take_sound().as_deref(), Some(OK_SOUND));
    }

    #[test]
    fn percent_rounds_like_the_original() {
        assert_eq!(percent(0, 261), 0);
        assert_eq!(percent(1, 261), 1);
        assert_eq!(percent(130, 261), 49);
        assert_eq!(percent(260, 261), 99);
        assert_eq!(percent(261, 261), 100);
    }

    #[test]
    fn registers_a_lone_picture() {
        let mut bytes = vec![0u8; 4000];
        let set = register(&bytes, "HG010000_01.AKB", false);
        assert_eq!(set[0], (CG_SEEN, 2));
        for (i, v) in set {
            bytes[i] = v;
        }
        assert!(is_seen(&bytes[CG_SEEN..], 1));
        assert!(register(&bytes, "hg010000_01", true).is_empty());
        assert!(register(&bytes, "bg_room", false).is_empty());
    }

    #[test]
    fn thumbnails_follow_the_first_seen() {
        let mut bits = vec![0u8; 33];
        assert_eq!(thumb(&bits, 2), None);
        // Entry 2 starts at picture 3 and has four.
        bits[0] |= 1 << 5;
        assert_eq!(thumb(&bits, 2), Some(2));
        bits[0] |= 1 << 3;
        assert_eq!(thumb(&bits, 2), Some(0));
        assert_eq!(seen_in(&bits, 2), 2);
    }
}
