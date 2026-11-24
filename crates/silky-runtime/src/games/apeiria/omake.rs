//! The extras of 景の海のアペイリア and of its fan disc (CG, scene and
//! music modes, `Game.30` to `Game.32`), read from their executables. They use the area files
//! `cgmode.area`, `scmode.area` and `soundmode.area`, one sheet for each
//! mode, and keep what was seen in the byte variables from 3000 (kept
//! across games):
//!
//! - CG: a bit set from byte 3300, one bit per picture of [`CG_IMAGES`],
//!   set when the background and layers of `Game.12` match a row once
//!   shown (see [`CgTrack`]); byte 3002 is set once an event picture was
//!   shown.
//! - Scenes: bytes 3101..=3166, set by the scripts; 3003 opens the mode.
//! - Music: bytes 3050..=3090 (one each per track, the last two swapped),
//!   set by the scripts; 3004 opens the mode.
//!
//! What differs between the two games (cells, pages, tables) is in
//! [`Tables`]; [`APEIRIA`] and [`CASABLANCA`] hold it.
//!
//! CG entries open a grid of their pictures over the dimmed page; a seen
//! picture there shows over the whole screen, and clicks walk on through
//! every seen picture to the last, then back to the grid. Scene cells play
//! a slide show of the scene while the pointer is over them.

use silky_format::akb::Image;
use silky_format::area::Area;
use silky_format::game::Title;
use silky_vm::Vm;
use std::sync::Arc;

use super::casablanca;
use crate::assets::Assets;
use crate::render::Canvas;
use crate::ui::{Button, Ui};
use crate::{Extras, Result};

pub const CG_SEEN: usize = 3300;
/// Set once an event picture was shown; opens the CG mode.
pub const CG_ANY: usize = 3002;
/// Set when a CG bit is newly set.
pub const CG_CHANGED: usize = 3001;
pub const SCENE_ANY: usize = 3003;
pub const MUSIC_ANY: usize = 3004;
pub const SCENE_FIRST: usize = 3101;
pub const MUSIC_FIRST: usize = 3050;

/// Cells of the grid of a CG entry, three rows of four.
const GRID_CELLS: usize = 12;
const CARD: (u32, u32) = (276, 158);
const THUMB: (u32, u32) = (270, 152);
/// Music rows shown at once, and the last first row.
const ROWS: usize = 8;
/// Black over the page under a CG entry's grid.
const VIEW_DIM: [u8; 4] = [0, 0, 0, 204];

/// Keys of a page of cells: what the arrow keys reach.
#[derive(Debug, Clone, Copy)]
pub struct PageKeys {
    /// Cells before this one may go down to the cell below.
    pub down_cells: i32,
    /// Last page button.
    pub last_page: i32,
    /// Page button that up from the next arrow goes to, if any.
    pub up_from_next: Option<i32>,
}

/// Where a page of cells is and its page arrows and buttons.
#[derive(Debug, Clone, Copy)]
pub struct PageLayout {
    pub cells: usize,
    pub pages: usize,
    /// First cell: x, y, and the steps between cells.
    pub cell: (i32, i32, i32, i32),
    /// Previous arrow y, next arrow y and y of the first page button.
    pub arrows: (i32, i32, i32),
    pub keys: PageKeys,
}

/// Everything that differs between the games: the layout of the pages and
/// the tables of pictures, scenes and tracks.
#[derive(Debug)]
pub struct Tables {
    pub cg: PageLayout,
    pub scene: PageLayout,
    pub cg_entries: &'static [(usize, usize, &'static str)],
    pub cg_images: &'static [&'static [&'static str]],
    /// Seen bit of each row of `cg_images`; none when it is the row.
    pub cg_bits: Option<&'static [u16]>,
    /// Backgrounds with these names (lower case) are kept by the tracker.
    pub cg_prefixes: &'static [&'static str],
    pub scenes: usize,
    pub scene_frames: &'static [u8],
    pub tracks: usize,
    pub track_files: &'static [&'static str],
    /// Byte variable of each track.
    pub track_bytes: &'static [usize],
    pub extras_music: &'static str,
    /// The scroll bar maps its height to this many tenths of a row.
    pub scroll_tenths: usize,
}

impl Tables {
    pub fn max_scroll(&self) -> usize {
        self.tracks - ROWS
    }

    /// The seen bit of row `row` of the CG pictures.
    pub fn cg_bit(&self, row: usize) -> usize {
        self.cg_bits.map_or(row, |b| b[row] as usize)
    }

    pub fn row_seen(&self, bytes: &[u8], row: usize) -> bool {
        cg_seen(bytes, self.cg_bit(row))
    }
}

pub static APEIRIA: Tables = Tables {
    cg: PageLayout {
        cells: 12,
        pages: 8,
        cell: (315, 18, 280, 170),
        arrows: (66, 527, 117),
        keys: PageKeys {
            down_cells: 12,
            last_page: 7,
            up_from_next: Some(7),
        },
    },
    scene: PageLayout {
        cells: 12,
        pages: 6,
        cell: (315, 18, 280, 170),
        arrows: (66, 429, 117),
        keys: PageKeys {
            down_cells: 12,
            last_page: 5,
            up_from_next: Some(5),
        },
    },
    cg_entries: &CG_ENTRIES,
    cg_images: &CG_IMAGES,
    cg_bits: None,
    cg_prefixes: &["ev"],
    scenes: 66,
    scene_frames: &SCENE_FRAMES,
    tracks: 41,
    track_files: &TRACK_FILES,
    track_bytes: &TRACK_BYTES,
    extras_music: "ape_27.ogg",
    scroll_tenths: 330,
};

pub static CASABLANCA: Tables = casablanca::TABLES;

/// The tables of a game's extras.
pub fn tables(title: Title) -> &'static Tables {
    match title {
        Title::Casablanca => &CASABLANCA,
        _ => &APEIRIA,
    }
}

/// Ids of the area files, as in [`crate::menus::extras_ids`].
pub mod ids {
    pub const PREVIOUS: i32 = 0x100;
    pub const NEXT: i32 = 0x101;
    pub const SCROLL: i32 = 0x102;
    pub const PAGE: i32 = 0x200;
    pub const VOLUME: i32 = 0x200;
    pub const BACK: i32 = 0x300;
    pub const MODE: i32 = 0xa00;
    /// The grid of a CG entry, which has no area file: its cells from
    /// here, then its arrows and back.
    pub const VIEW_CELL: i32 = 0x1000;
    pub const VIEW_PREVIOUS: i32 = 0x1100;
    pub const VIEW_NEXT: i32 = 0x1101;
    pub const VIEW_BACK: i32 = 0x1300;
    /// Anywhere on a picture over the whole screen.
    pub const PICTURE: i32 = 0x1400;
}

/// The grid of one CG entry, and one of its pictures over the whole
/// screen. Walking through pictures may move on to later entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub entry: usize,
    pub page: usize,
    /// Index into the entry's pictures, when one is shown.
    pub picture: Option<usize>,
}

/// The screens as the session keeps them.
#[derive(Debug, Clone)]
pub struct Omake {
    pub t: &'static Tables,
    pub view: Option<View>,
    pub dragging_volume: bool,
    pub dragging_scroll: bool,
    /// Scene under the pointer and the clock when it got there.
    pub preview: Option<(usize, u64)>,
}

impl Default for Omake {
    fn default() -> Self {
        Self::new(&APEIRIA)
    }
}

impl Omake {
    pub fn new(t: &'static Tables) -> Self {
        Self {
            t,
            view: None,
            dragging_volume: false,
            dragging_scroll: false,
            preview: None,
        }
    }
}

/// Byte variables as the VM holds them.
pub fn byte(bytes: &[u8], i: usize) -> u8 {
    bytes.get(i).copied().unwrap_or(0)
}

pub fn cg_seen(bytes: &[u8], picture: usize) -> bool {
    byte(bytes, CG_SEEN + picture / 8) & (1 << (picture % 8)) != 0
}

pub fn scene_seen(t: &Tables, bytes: &[u8], scene: usize) -> bool {
    scene < t.scenes && byte(bytes, SCENE_FIRST + scene) != 0
}

/// Bytes of the tracks: in order, the last two swapped.
const fn apeiria_track_bytes() -> [usize; 41] {
    let mut a = [0; 41];
    let mut t = 0;
    while t < 41 {
        a[t] = MUSIC_FIRST + t;
        t += 1;
    }
    a[39] = MUSIC_FIRST + 40;
    a[40] = MUSIC_FIRST + 39;
    a
}

const TRACK_BYTES: [usize; 41] = apeiria_track_bytes();

pub fn track_heard(t: &Tables, bytes: &[u8], track: usize) -> bool {
    track < t.tracks && byte(bytes, t.track_bytes[track]) == 1
}

/// Whether the button of a mode opens it.
pub fn mode_open(bytes: &[u8], kind: Extras) -> bool {
    let at = match kind {
        Extras::Gallery => CG_ANY,
        Extras::Scenes => SCENE_ANY,
        Extras::Music => MUSIC_ANY,
    };
    byte(bytes, at) != 0
}

/// Pictures of a CG entry, as rows of the pictures table.
fn entry_pictures(t: &Tables, entry: usize) -> std::ops::Range<usize> {
    let (first, count, _) = t.cg_entries[entry];
    first..first + count
}

/// First picture of an entry that was seen, by its place in the entry.
fn first_seen(t: &Tables, bytes: &[u8], entry: usize) -> Option<usize> {
    entry_pictures(t, entry).position(|p| t.row_seen(bytes, p))
}

/// The seen picture after (entry, picture), through the later entries in
/// table order.
pub fn next_seen(t: &Tables, bytes: &[u8], entry: usize, picture: usize) -> Option<(usize, usize)> {
    (entry..t.cg_entries.len()).find_map(|e| {
        let from = if e == entry { picture + 1 } else { 0 };
        let rows = entry_pictures(t, e);
        let k = (from..rows.len()).find(|&k| t.row_seen(bytes, rows.start + k))?;
        Some((e, k))
    })
}

/// The seen picture before (entry, picture), through the earlier entries.
pub fn previous_seen(
    t: &Tables,
    bytes: &[u8],
    entry: usize,
    picture: usize,
) -> Option<(usize, usize)> {
    (0..=entry).rev().find_map(|e| {
        let rows = entry_pictures(t, e);
        let to = if e == entry { picture } else { rows.len() };
        let k = (0..to).rev().find(|&k| t.row_seen(bytes, rows.start + k))?;
        Some((e, k))
    })
}

/// Completion in per cent, at least 1 once anything was seen.
fn percent(seen: usize, total: usize) -> u32 {
    match seen {
        0 => 0,
        _ => ((seen as f32 / total as f32 * 100.0) as u32).max(1),
    }
}

fn cg_percent(t: &Tables, bytes: &[u8]) -> u32 {
    let n = t.cg_images.len();
    percent((0..n).filter(|&p| t.row_seen(bytes, p)).count(), n)
}

fn scene_percent(t: &Tables, bytes: &[u8]) -> u32 {
    percent(
        (0..t.scenes).filter(|&s| scene_seen(t, bytes, s)).count(),
        t.scenes,
    )
}

fn music_percent(t: &Tables, bytes: &[u8]) -> u32 {
    percent(
        (0..t.tracks).filter(|&k| track_heard(t, bytes, k)).count(),
        t.tracks,
    )
}

/// What `Game.12` has shown, matched against [`CG_IMAGES`] whenever it is
/// put on screen. Only backgrounds with the game's names count.
#[derive(Debug, Clone)]
pub struct CgTrack {
    t: &'static Tables,
    background: String,
    layers: [String; 7],
}

fn norm(name: &str) -> String {
    let s = name.to_ascii_lowercase();
    s.strip_suffix(".akb").map(str::to_string).unwrap_or(s)
}

impl Default for CgTrack {
    fn default() -> Self {
        Self::new(&APEIRIA)
    }
}

impl CgTrack {
    pub fn new(t: &'static Tables) -> Self {
        Self {
            t,
            background: String::new(),
            layers: Default::default(),
        }
    }

    /// A new background; its layers go.
    pub fn set_background(&mut self, name: &str) {
        let n = norm(name);
        self.background = if self.t.cg_prefixes.iter().any(|p| n.starts_with(p)) {
            n
        } else {
            String::new()
        };
        self.layers = Default::default();
    }

    pub fn set_layer(&mut self, slot: i32, name: &str) {
        if let Some(l) = usize::try_from(slot)
            .ok()
            .and_then(|s| self.layers.get_mut(s))
        {
            *l = norm(name);
        }
    }

    /// The screen is shown: marks its picture seen if it is a CG.
    pub fn shown(&self, vm: &mut Vm) {
        if self.background.is_empty() {
            return;
        }
        vm.set_var_b(CG_ANY, 1);
        let layers: Vec<&str> = self
            .layers
            .iter()
            .take_while(|l| !l.is_empty())
            .map(String::as_str)
            .collect();
        if self.layers.iter().skip(layers.len()).any(|l| !l.is_empty()) {
            return;
        }
        let Some(i) = self.t.cg_images.iter().position(|row| {
            row[0] == self.background
                && row.len() == layers.len() + 1
                && row[1..].iter().zip(&layers).all(|(a, b)| a == b)
        }) else {
            return;
        };
        mark(vm, self.t.cg_bit(i));
    }
}

/// Sets the seen bit `i`.
pub fn mark(vm: &mut Vm, i: usize) {
    let at = CG_SEEN + i / 8;
    vm.set_var_b(at, vm.var_b(at) | 1 << (i % 8));
    vm.set_var_b(CG_ANY, 1);
    vm.set_var_b(CG_CHANGED, 1);
}

/// Cell `i` of a page on screen.
fn cell(l: &PageLayout, i: usize) -> (i32, i32) {
    let (x, y, dx, dy) = l.cell;
    (x + dx * (i % 3) as i32, y + dy * (i / 3) as i32)
}

/// Pictures of an entry's grid page.
fn view_count(t: &Tables, v: &View) -> usize {
    (t.cg_entries[v.entry].1 - v.page * GRID_CELLS).min(GRID_CELLS)
}

pub fn view_pages(t: &Tables, entry: usize) -> usize {
    t.cg_entries[entry].1.div_ceil(GRID_CELLS)
}

/// Grid of an entry: where cell `i` of `n` goes.
fn view_cell(i: usize, n: usize) -> (i32, i32) {
    let w = if n < 4 { n as i32 * 286 } else { 1144 };
    let h = if n < 5 {
        170
    } else {
        n.div_ceil(4) as i32 * 170
    };
    let x0 = (1280 - w) / 2 + 4;
    let y0 = (690 - h) / 2 + 16;
    (x0 + 286 * (i % 4) as i32, y0 + 170 * (i / 4) as i32)
}

const VIEW_PREVIOUS_AT: (i32, i32, i32, i32) = (536, 647, 566, 703);
const VIEW_NEXT_AT: (i32, i32, i32, i32) = (716, 647, 746, 703);
const VIEW_BACK_AT: (i32, i32, i32, i32) = (1055, 640, 1270, 713);

/// Place of a button on a CG entry's grid (left, top, right, bottom).
pub fn view_rect(t: &Tables, v: &View, id: i32) -> Option<(i32, i32, i32, i32)> {
    match id {
        ids::VIEW_PREVIOUS => Some(VIEW_PREVIOUS_AT),
        ids::VIEW_NEXT => Some(VIEW_NEXT_AT),
        ids::VIEW_BACK => Some(VIEW_BACK_AT),
        i if (ids::VIEW_CELL..ids::VIEW_CELL + GRID_CELLS as i32).contains(&i) => {
            let n = view_count(t, v);
            let i = (i - ids::VIEW_CELL) as usize;
            let (x, y) = view_cell(i, n);
            (i < n).then_some((x, y, x + CARD.0 as i32, y + CARD.1 as i32))
        }
        _ => None,
    }
}

/// Whether cell `i` of the grid of `v` holds a seen picture.
pub fn view_on(t: &Tables, v: &View, bytes: &[u8], i: usize) -> bool {
    i < view_count(t, v) && t.row_seen(bytes, t.cg_entries[v.entry].0 + v.page * GRID_CELLS + i)
}

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && x < r.2 && y >= r.1 && y < r.3
}

/// The item behind cell or row `i`: CG entry, scene, track.
pub fn item(t: &Tables, kind: Extras, page: usize, scroll: usize, i: usize) -> Option<usize> {
    let (k, n, cells) = match kind {
        Extras::Gallery => (page * t.cg.cells + i, t.cg_entries.len(), t.cg.cells),
        Extras::Scenes => (page * t.scene.cells + i, t.scenes, t.scene.cells),
        Extras::Music => (scroll + i, t.tracks, ROWS),
    };
    (i < cells && k < n).then_some(k)
}

pub fn open(t: &Tables, kind: Extras, bytes: &[u8], item: usize) -> bool {
    match kind {
        Extras::Gallery => first_seen(t, bytes, item).is_some(),
        Extras::Scenes => scene_seen(t, bytes, item),
        Extras::Music => track_heard(t, bytes, item),
    }
}

/// Button under the pointer.
#[allow(clippy::too_many_arguments)]
pub fn hit(
    areas: &[Area],
    kind: Extras,
    page: usize,
    scroll: usize,
    o: &Omake,
    bytes: &[u8],
    x: i32,
    y: i32,
) -> Option<Button> {
    let t = o.t;
    if let Some(v) = &o.view {
        if v.picture.is_some() {
            return Some(Button::Screen(ids::PICTURE));
        }
        let n = view_count(t, v);
        if let Some(i) = (0..n).find(|&i| {
            let (cx, cy) = view_cell(i, n);
            x >= cx && x < cx + CARD.0 as i32 && y >= cy && y < cy + CARD.1 as i32
        }) {
            let p = t.cg_entries[v.entry].0 + v.page * GRID_CELLS + i;
            return t
                .row_seen(bytes, p)
                .then_some(Button::Screen(ids::VIEW_CELL + i as i32));
        }
        let pages = view_pages(t, v.entry);
        let id = if pages > 1 && inside(VIEW_PREVIOUS_AT, x, y) {
            ids::VIEW_PREVIOUS
        } else if pages > 1 && inside(VIEW_NEXT_AT, x, y) {
            ids::VIEW_NEXT
        } else if inside(VIEW_BACK_AT, x, y) {
            ids::VIEW_BACK
        } else {
            return None;
        };
        return Some(Button::Screen(id));
    }
    let a = areas.iter().find(|a| a.contains(x, y))?;
    let id = a.id;
    if (0..0x100).contains(&id) {
        let it = item(t, kind, page, scroll, id as usize)?;
        if !open(t, kind, bytes, it) {
            return None;
        }
    }
    if (ids::MODE..ids::MODE + 3).contains(&id) {
        let k = [Extras::Gallery, Extras::Scenes, Extras::Music][(id - ids::MODE) as usize];
        if k == kind || !mode_open(bytes, k) {
            return None;
        }
    }
    Some(Button::Screen(id))
}

/// What the screens draw beside the bytes.
pub struct Draw<'a> {
    pub kind: Extras,
    pub page: usize,
    pub scroll: usize,
    pub playing: Option<usize>,
    pub omake: &'a Omake,
    pub bytes: &'a [u8],
    /// Music volume, and whether music is on.
    pub volume: (f32, bool),
    pub clock: u64,
}

fn part(c: &mut Canvas, img: &Arc<Image>, src: (u32, u32), dst: (i32, i32), size: (u32, u32)) {
    c.draw_part(
        img,
        (src.0, src.1, size.0, size.1),
        dst.0,
        dst.1,
        false,
        255,
    );
}

/// Draws the screen.
pub fn draw(canvas: &mut Canvas, assets: &mut Assets, ui: &Ui, d: &Draw) -> Result<()> {
    if let Some(View {
        entry,
        picture: Some(p),
        ..
    }) = d.omake.view
    {
        return draw_picture(canvas, assets, d.omake.t, d.omake.t.cg_entries[entry].0 + p);
    }
    let (bg, sheet) = match d.kind {
        Extras::Gallery => ("cgmode_bg.akb", "cgmode_p.akb"),
        Extras::Scenes => ("scenemode_bg.akb", "scenemode_p.akb"),
        Extras::Music => ("musicmode_bg.akb", "musicmode_p.akb"),
    };
    canvas.draw(&assets.image(bg)?, 0, 0, false);
    let sheet = assets.image(sheet)?;
    match d.kind {
        Extras::Gallery => draw_cg(canvas, assets, &sheet, ui, d)?,
        Extras::Scenes => draw_scenes(canvas, assets, &sheet, ui, d)?,
        Extras::Music => draw_music(canvas, assets, &sheet, ui, d)?,
    }
    if let Some(v) = &d.omake.view {
        draw_view(canvas, assets, &sheet, ui, d.omake.t, v, d.bytes)?;
    }
    Ok(())
}

fn st(ui: &Ui, id: i32, enabled: bool) -> u32 {
    ui.state(Button::Screen(id), enabled, false) as u32
}

/// Mode buttons, back and the three completions, as the CG and scene
/// sheets lay them out (the music sheet has its own columns).
fn draw_frame(canvas: &mut Canvas, sheet: &Arc<Image>, ui: &Ui, d: &Draw) {
    let digits = match d.kind {
        Extras::Music => (444, 292),
        _ => (935, 184),
    };
    let places = [(54, 82), (114, 262), (54, 442)];
    for (k, kind) in [Extras::Gallery, Extras::Scenes, Extras::Music]
        .into_iter()
        .enumerate()
    {
        let row = if kind == d.kind {
            2
        } else {
            st(ui, ids::MODE + k as i32, mode_open(d.bytes, kind))
        };
        part(
            canvas,
            sheet,
            (148 * k as u32, 148 * row),
            places[k],
            (148, 148),
        );
    }
    part(
        canvas,
        sheet,
        (444, 73 * st(ui, ids::BACK, true)),
        (28, 617),
        (215, 73),
    );
    for (x, value) in [
        (55, cg_percent(d.omake.t, d.bytes)),
        (157, scene_percent(d.omake.t, d.bytes)),
        (265, music_percent(d.omake.t, d.bytes)),
    ] {
        draw_percent(canvas, sheet, digits, x, value);
    }
}

/// Small digits of a completion, right aligned before the per cent sign.
fn draw_percent(canvas: &mut Canvas, sheet: &Arc<Image>, at: (u32, u32), x: i32, mut v: u32) {
    part(canvas, sheet, (at.0 + 120, at.1), (x + 38, 699), (18, 17));
    let mut dx = x + 26;
    loop {
        part(
            canvas,
            sheet,
            (at.0 + 12 * (v % 10), at.1),
            (dx, 699),
            (12, 17),
        );
        v /= 10;
        dx -= 13;
        if v == 0 {
            break;
        }
    }
}

/// Big digit `n` (26 x 36) of the CG and scene sheets.
fn digit(canvas: &mut Canvas, sheet: &Arc<Image>, n: usize, x: i32, y: i32) {
    part(canvas, sheet, (935 + 26 * n as u32, 201), (x, y), (26, 36));
}

/// A number of up to two big digits, the tens left out under ten.
fn number(canvas: &mut Canvas, sheet: &Arc<Image>, n: usize, tens: i32, ones: i32, y: i32) {
    if n > 9 {
        digit(canvas, sheet, n / 10 % 10, tens, y);
    }
    digit(canvas, sheet, n % 10, ones, y);
}

/// Hover and pressed frames over a cell.
fn draw_cell_state(canvas: &mut Canvas, sheet: &Arc<Image>, ui: &Ui, id: i32, at: (i32, i32)) {
    let y = match st(ui, id, true) {
        1 => 158,
        2 => 316,
        _ => return,
    };
    part(canvas, sheet, (659, y), at, CARD);
}

/// Page arrows and page buttons on the right.
fn draw_pages(canvas: &mut Canvas, sheet: &Arc<Image>, ui: &Ui, page: usize, l: &PageLayout) {
    let (pages, (prev_y, next_y, first_y)) = (l.pages, l.arrows);
    let prev = if page == 0 {
        3
    } else {
        st(ui, ids::PREVIOUS, true)
    };
    part(
        canvas,
        sheet,
        (1445, 97 + 30 * prev),
        (1185, prev_y),
        (56, 30),
    );
    let next = if page + 1 == pages {
        3
    } else {
        st(ui, ids::NEXT, true)
    };
    part(
        canvas,
        sheet,
        (1501, 97 + 30 * next),
        (1185, next_y),
        (56, 30),
    );
    for p in 0..pages {
        let id = ids::PAGE + p as i32;
        let y = if p == page { 92 } else { 46 * st(ui, id, true) };
        part(
            canvas,
            sheet,
            (935 + 51 * p as u32, y),
            (1187, first_y + 49 * p as i32),
            (51, 46),
        );
    }
}

fn draw_cg(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Arc<Image>,
    ui: &Ui,
    d: &Draw,
) -> Result<()> {
    draw_frame(canvas, sheet, ui, d);
    let t = d.omake.t;
    let thumbs = assets.image(&format!("cgmode_thm{:02}.akb", d.page)).ok();
    for i in 0..t.cg.cells {
        let Some(entry) = item(t, Extras::Gallery, d.page, 0, i) else {
            continue;
        };
        let (x, y) = cell(&t.cg, i);
        let Some(first) = first_seen(t, d.bytes, entry) else {
            part(canvas, sheet, (659, 474), (x, y), CARD);
            continue;
        };
        part(canvas, sheet, (659, 0), (x, y), CARD);
        let (tw, th) = THUMB;
        if first == 0 {
            if let Some(t) = &thumbs {
                let src = ((i % 3) as u32 * tw, (i / 3) as u32 * th);
                part(canvas, t, src, (x + 3, y + 3), THUMB);
            }
        } else if let Ok(t) = assets.image(&format!("{}.akb", t.cg_entries[entry].2)) {
            // The engine takes this cell of the strip three a row,
            // though the strips hold four a row.
            let src = ((first % 3) as u32 * tw, (first / 3) as u32 * th);
            part(canvas, &t, src, (x + 3, y + 3), THUMB);
        }
        let count = t.cg_entries[entry].1;
        if count != 1 {
            let seen = entry_pictures(t, entry)
                .filter(|&p| t.row_seen(d.bytes, p))
                .count();
            number(canvas, sheet, seen, x + 153, x + 176, y + 116);
            part(canvas, sheet, (1195, 201), (x + 199, y + 116), (26, 36));
            number(canvas, sheet, count, x + 222, x + 244, y + 116);
        }
        if d.omake.view.is_none() {
            draw_cell_state(canvas, sheet, ui, i as i32, (x, y));
        }
    }
    draw_pages(canvas, sheet, ui, d.page, &t.cg);
    Ok(())
}

fn draw_scenes(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Arc<Image>,
    ui: &Ui,
    d: &Draw,
) -> Result<()> {
    draw_frame(canvas, sheet, ui, d);
    let t = d.omake.t;
    let thumbs = assets.image("scene_thumb.akb").ok();
    for i in 0..t.scene.cells {
        let Some(scene) = item(t, Extras::Scenes, d.page, 0, i) else {
            continue;
        };
        let (x, y) = cell(&t.scene, i);
        if !scene_seen(t, d.bytes, scene) {
            part(canvas, sheet, (659, 474), (x, y), CARD);
            continue;
        }
        part(canvas, sheet, (659, 0), (x, y), CARD);
        let (tw, th) = THUMB;
        match d.omake.preview.filter(|p| p.0 == scene) {
            Some((_, since)) => draw_preview(
                t,
                canvas,
                assets,
                scene,
                d.clock.saturating_sub(since),
                (x, y),
            )?,
            None => {
                if let Some(t) = &thumbs {
                    // Three scenes a row of the sheet.
                    let row = (scene / 3) as u32;
                    part(
                        canvas,
                        t,
                        ((i % 3) as u32 * tw, row * th),
                        (x + 3, y + 3),
                        THUMB,
                    );
                }
            }
        }
        draw_cell_state(canvas, sheet, ui, i as i32, (x, y));
    }
    draw_pages(canvas, sheet, ui, d.page, &t.scene);
    Ok(())
}

/// The slide show of a scene under the pointer: each picture of
/// `scene_thumbNN` holds a second, then fades into the next over a
/// second; the first fade starts half a second in.
fn draw_preview(
    t: &Tables,
    canvas: &mut Canvas,
    assets: &mut Assets,
    scene: usize,
    ms: u64,
    at: (i32, i32),
) -> Result<()> {
    let strip = assets.image(&format!("scene_thumb{scene:02}.akb"))?;
    let n = t.scene_frames[scene].max(1) as u64;
    let t = ms + 500;
    let (k, r) = (t / 2000, t % 2000);
    let cur = (k % n) as u32;
    let (tw, th) = THUMB;
    let at = (at.0 + 3, at.1 + 3);
    part(canvas, &strip, (cur * tw, 0), at, THUMB);
    if r > 1000 {
        let next = ((k + 1) % n) as u32;
        let alpha = ((r - 1000) * 255 / 1000).min(255) as u8;
        canvas.draw_part(&strip, (next * tw, 0, tw, th), at.0, at.1, false, alpha);
    }
    Ok(())
}

/// Whether the slide show of `scene` is running (it needs redrawing).
pub fn animates(t: &Tables, scene: usize) -> bool {
    t.scene_frames.get(scene).is_some_and(|&n| n > 1)
}

/// Music volume bar: the bar, its filled part and the knob.
const VOLUME_BAR: (i32, i32) = (668, 659);
const VOLUME_FILL: i32 = 684;
const VOLUME_WIDTH: f32 = 439.0;

/// Volume for a pointer at `x`.
pub fn volume_at(x: i32) -> f32 {
    ((x - 688) as f32 / VOLUME_WIDTH).clamp(0.0, 1.0)
}

/// Music scroll for a pointer at `y` over the scroll bar.
pub fn scroll_at(t: &Tables, y: i32) -> usize {
    let y = (y - 114).clamp(0, 462) as usize;
    (y * t.scroll_tenths / 462 + 5) / 10
}

/// Top of the scroll thumb for a scroll.
fn thumb_top(t: &Tables, scroll: usize) -> i32 {
    66 + (scroll as i32 * 462) / t.max_scroll() as i32
}

fn draw_music(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Arc<Image>,
    ui: &Ui,
    d: &Draw,
) -> Result<()> {
    draw_frame(canvas, sheet, ui, d);
    part(canvas, sheet, (1280, 0), (1183, 63), (55, 565));
    let list = assets.image("music_title.akb")?;
    let tb = d.omake.t;
    for i in 0..ROWS {
        let Some(t) = item(tb, Extras::Music, 0, d.scroll, i) else {
            continue;
        };
        let row = if !track_heard(tb, d.bytes, t) {
            3
        } else if d.playing == Some(t) {
            2
        } else {
            st(ui, i as i32, true).min(2)
        };
        let src = ((t % 4) as u32 * 615, (row + (t / 4) as u32 * 4) * 67);
        part(
            canvas,
            &list,
            src,
            (515 - 30 * i as i32, 66 + 70 * i as i32),
            (615, 67),
        );
    }
    let up = if d.scroll == 0 {
        3
    } else {
        st(ui, ids::PREVIOUS, true)
    };
    part(canvas, sheet, (659, 97 + 30 * up), (1183, 31), (56, 30));
    let down = if d.scroll == tb.max_scroll() {
        3
    } else {
        st(ui, ids::NEXT, true)
    };
    part(canvas, sheet, (715, 97 + 30 * down), (1183, 630), (56, 30));
    let thumb = if d.omake.dragging_scroll {
        2
    } else {
        st(ui, ids::SCROLL, true).min(1)
    };
    part(
        canvas,
        sheet,
        (659 + 47 * thumb, 0),
        (1187, thumb_top(tb, d.scroll)),
        (47, 97),
    );
    part(canvas, sheet, (800, 0), VOLUME_BAR, (480, 28));
    let (volume, on) = d.volume;
    if on {
        let w = (volume.clamp(0.0, 1.0) * VOLUME_WIDTH) as u32;
        part(
            canvas,
            sheet,
            (816, 28),
            (VOLUME_FILL, VOLUME_BAR.1),
            (w, 28),
        );
        let knob = if d.omake.dragging_volume {
            2
        } else {
            st(ui, ids::VOLUME, true).min(1)
        };
        part(
            canvas,
            sheet,
            (800, (knob + 2) * 28),
            (VOLUME_FILL + w as i32, VOLUME_BAR.1),
            (13, 28),
        );
    }
    Ok(())
}

fn draw_view(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Arc<Image>,
    ui: &Ui,
    t: &Tables,
    v: &View,
    bytes: &[u8],
) -> Result<()> {
    canvas.fill(0, 0, 1280, 720, VIEW_DIM);
    part(canvas, sheet, (0, 632), (0, 630), (1280, 90));
    let (first, _, name) = t.cg_entries[v.entry];
    let strip = assets.image(&format!("{name}.akb")).ok();
    let n = view_count(t, v);
    let (tw, th) = THUMB;
    for i in 0..n {
        let k = v.page * GRID_CELLS + i;
        let (x, y) = view_cell(i, n);
        if !t.row_seen(bytes, first + k) {
            part(canvas, sheet, (659, 474), (x, y), CARD);
            continue;
        }
        part(canvas, sheet, (659, 0), (x, y), CARD);
        if let Some(t) = &strip {
            let src = ((k % 4) as u32 * tw, (k / 4) as u32 * th);
            part(canvas, t, src, (x + 3, y + 3), THUMB);
        }
        draw_cell_state(canvas, sheet, ui, ids::VIEW_CELL + i as i32, (x, y));
    }
    let pages = view_pages(t, v.entry);
    if pages > 1 {
        let prev = if v.page == 0 {
            3
        } else {
            st(ui, ids::VIEW_PREVIOUS, true)
        };
        part(canvas, sheet, (1446, 217 + 56 * prev), (536, 647), (30, 56));
        let next = if v.page + 1 == pages {
            3
        } else {
            st(ui, ids::VIEW_NEXT, true)
        };
        part(canvas, sheet, (1476, 217 + 56 * next), (716, 647), (30, 56));
        number(canvas, sheet, v.page + 1, 584, 606, 657);
        part(canvas, sheet, (1195, 201), (629, 657), (26, 36));
        number(canvas, sheet, pages, 652, 674, 657);
    }
    part(
        canvas,
        sheet,
        (444, 73 * st(ui, ids::VIEW_BACK, true)),
        (1055, 640),
        (215, 73),
    );
    Ok(())
}

/// A CG picture over the whole screen: the background, then its layers
/// with pure green as transparency.
fn draw_picture(
    canvas: &mut Canvas,
    assets: &mut Assets,
    t: &Tables,
    picture: usize,
) -> Result<()> {
    canvas.fill(0, 0, 1280, 720, [0, 0, 0, 255]);
    for (k, layer) in t.cg_images[picture].iter().enumerate() {
        let img = assets.image(&format!("{layer}.akb"))?;
        canvas.draw(&img, 0, 0, k > 0);
    }
    Ok(())
}

/// Music room tracks, from the executable.
const TRACK_FILES: [&str; 41] = [
    "ape_01.ogg",
    "ape_01_02.ogg",
    "ape_02.ogg",
    "ape_02_02.ogg",
    "ape_03.ogg",
    "ape_04.ogg",
    "ape_05.ogg",
    "ape_06.ogg",
    "ape_07.ogg",
    "ape_08.ogg",
    "ape_10.ogg",
    "ape_11.ogg",
    "ape_12.ogg",
    "ape_13.ogg",
    "ape_14.ogg",
    "ape_15.ogg",
    "ape_16.ogg",
    "ape_17.ogg",
    "ape_18.ogg",
    "ape_19.ogg",
    "ape_20.ogg",
    "ape_21.ogg",
    "ape_22.ogg",
    "ape_23.ogg",
    "ape_24.ogg",
    "ape_25.ogg",
    "ape_26.ogg",
    "ape_27.ogg",
    "ape_28.ogg",
    "ape_28_02.ogg",
    "ape_29.ogg",
    "ape_30.ogg",
    "ape_31.ogg",
    "ape_32.ogg",
    "ape_33.ogg",
    "ape_34.ogg",
    "ape_35.ogg",
    "ape_36.ogg",
    "apeiria_short_g_44-16.ogg",
    "shinjirareruyo_short_inst_44-16.ogg",
    "shinjirareruyo_short_44-16.ogg",
];

/// Pictures in each scene's `scene_thumbNN` strip, from the executable.
const SCENE_FRAMES: [u8; 66] = [
    3, 1, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 1, 1, 2, 3, 3, 3, 3, 3, 3, 3, 1, 3, 1, 3, 3, 3, 3, 3,
    3, 3, 1, 2, 2, 3, 2, 3, 3, 1, 3, 3, 3, 1, 1, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 1, 2, 2, 3, 3,
    3, 1,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_line_up() {
        // Apeiria's entries run in row order.
        let mut next = 0;
        for &(first, count, _) in &CG_ENTRIES {
            assert_eq!(first, next);
            next += count;
        }
        assert_eq!(next, CG_IMAGES.len());
        for t in [&APEIRIA, &CASABLANCA] {
            // Every row is in exactly one entry, and has one seen bit.
            let mut rows = vec![0; t.cg_images.len()];
            for &(first, count, _) in t.cg_entries {
                for r in first..first + count {
                    rows[r] += 1;
                }
            }
            assert!(rows.iter().all(|&n| n == 1));
            let mut bits: Vec<_> = (0..rows.len()).map(|r| t.cg_bit(r)).collect();
            bits.sort_unstable();
            bits.dedup();
            assert_eq!(bits.len(), rows.len());
            assert_eq!(t.scene_frames.len(), t.scenes);
            assert_eq!(t.track_files.len(), t.tracks);
            assert_eq!(t.track_bytes.len(), t.tracks);
            let pages = |l: &PageLayout, n: usize| n.div_ceil(l.cells) == l.pages;
            assert!(pages(&t.cg, t.cg_entries.len()));
            assert!(pages(&t.scene, t.scenes));
        }
    }

    #[test]
    fn walking_pictures() {
        let mut bytes = vec![0u8; 3400];
        bytes[CG_SEEN] = 0b101;
        bytes[CG_SEEN + 1] = 1 << 4;
        let t = &APEIRIA;
        assert_eq!(next_seen(t, &bytes, 0, 0), Some((0, 2)));
        // Picture 12 is the second of the third entry.
        assert_eq!(next_seen(t, &bytes, 0, 2), Some((2, 1)));
        assert_eq!(next_seen(t, &bytes, 2, 1), None);
        assert_eq!(previous_seen(t, &bytes, 2, 1), Some((0, 2)));
    }

    #[test]
    fn walking_in_table_order() {
        // Casablanca: the single pictures of entries 0 and 1 (rows 225,
        // 226, bits 223, 224) come before the group that starts at row 0.
        let t = &CASABLANCA;
        let mut bytes = vec![0u8; 3400];
        for bit in [223, 224, 0] {
            bytes[CG_SEEN + bit / 8] |= 1 << (bit % 8);
        }
        assert_eq!(next_seen(t, &bytes, 0, 0), Some((1, 0)));
        assert_eq!(next_seen(t, &bytes, 1, 0), Some((18, 0)));
        assert_eq!(previous_seen(t, &bytes, 18, 0), Some((1, 0)));
        assert_eq!(next_seen(t, &bytes, 18, 0), None);
    }

    #[test]
    fn grid_of_three() {
        // As the original draws an entry of three pictures.
        assert_eq!(view_cell(0, 3), (215, 276));
        assert_eq!(view_cell(2, 3), (787, 276));
    }

    #[test]
    fn last_tracks_swap() {
        let mut bytes = vec![0u8; 3100];
        bytes[MUSIC_FIRST + 40] = 1;
        assert!(track_heard(&APEIRIA, &bytes, 39));
        assert!(!track_heard(&APEIRIA, &bytes, 40));
        // Casablanca's tracks are not swapped.
        bytes[3093] = 1;
        assert!(track_heard(&CASABLANCA, &bytes, 31));
    }

    #[test]
    fn casablanca_seen_bits() {
        let t = &CASABLANCA;
        assert_eq!(t.cg_bit(22), 22);
        assert_eq!(t.cg_bit(23), 241);
        assert_eq!(t.cg_bit(28), 242);
        assert_eq!(t.cg_bit(29), 27);
        assert_eq!(t.cg_bit(242), 240);
    }
}

/// CG mode entries, from a table in the executable: first picture,
/// number of pictures and the strip of their thumbnails (four 270 x 152
/// a row).
pub const CG_ENTRIES: [(usize, usize, &str); 86] = [
    (0, 3, "cg_ev0100"),
    (3, 8, "cg_ev0402"),
    (11, 3, "cg_ev0500"),
    (14, 7, "cg_ev0600"),
    (21, 13, "cg_ev0702"),
    (34, 3, "cg_ev0800"),
    (37, 15, "cg_ev1000"),
    (52, 6, "cg_ev0900"),
    (58, 2, "cg_ev1100"),
    (60, 31, "cg_ev1200"),
    (91, 1, "cg_ev1400"),
    (92, 39, "cg_ev9900"),
    (131, 7, "cg_ev1500"),
    (138, 2, "cg_ev1300"),
    (140, 29, "cg_ev1600"),
    (169, 7, "cg_ev1700"),
    (176, 6, "cg_ev1805"),
    (182, 13, "cg_ev1900"),
    (195, 28, "cg_ev2000"),
    (223, 1, "cg_ev2100"),
    (224, 3, "cg_ev2200"),
    (227, 7, "cg_ev2306"),
    (234, 6, "cg_ev2400"),
    (240, 5, "cg_ev2501"),
    (245, 17, "cg_ev2600"),
    (262, 15, "cg_ev2700"),
    (277, 6, "cg_ev2900"),
    (283, 14, "cg_ev2800"),
    (297, 2, "cg_ev7200"),
    (299, 1, "cg_ev3200"),
    (300, 3, "cg_ev7300"),
    (303, 3, "cg_ev3000"),
    (306, 11, "cg_ev3300"),
    (317, 8, "cg_ev3400"),
    (325, 21, "cg_ev3500"),
    (346, 12, "cg_ev3600"),
    (358, 8, "cg_ev3700"),
    (366, 4, "cg_ev3800"),
    (370, 7, "cg_ev3900"),
    (377, 13, "cg_ev4000"),
    (390, 1, "cg_ev4100"),
    (391, 11, "cg_ev4200"),
    (402, 2, "cg_ev4300"),
    (404, 11, "cg_ev0305"),
    (415, 15, "cg_ev4400"),
    (430, 13, "cg_ev4500"),
    (443, 8, "cg_ev4600"),
    (451, 4, "cg_ev4700"),
    (455, 3, "cg_ev4800"),
    (458, 2, "cg_ev4900"),
    (460, 8, "cg_ev5000"),
    (468, 4, "cg_ev5100"),
    (472, 5, "cg_ev5300"),
    (477, 7, "cg_ev5200"),
    (484, 7, "cg_ev5400"),
    (491, 3, "cg_ev5600"),
    (494, 9, "cg_ev5500"),
    (503, 3, "cg_ev5700"),
    (506, 2, "cg_ev5800"),
    (508, 5, "cg_ev5902"),
    (513, 4, "cg_ev6000"),
    (517, 1, "cg_ev6100"),
    (518, 13, "cg_ev6200"),
    (531, 1, "cg_ev6400"),
    (532, 7, "cg_ev6301"),
    (539, 6, "cg_ev6500"),
    (545, 13, "cg_ev6602"),
    (558, 4, "cg_ev6700"),
    (562, 4, "cg_ev6800"),
    (566, 2, "cg_ev6900"),
    (568, 3, "cg_ev7000"),
    (571, 2, "cg_ev7100"),
    (573, 5, "cg_ev7603"),
    (578, 2, "cg_ev7700"),
    (580, 6, "cg_ev7800"),
    (586, 15, "cg_ev7900"),
    (601, 9, "cg_ev8000"),
    (610, 6, "cg_ev8100"),
    (616, 7, "cg_ev8500"),
    (623, 4, "cg_ev8200"),
    (627, 1, "cg_ev8300"),
    (628, 4, "cg_ev8400"),
    (632, 7, "cg_ev8600"),
    (639, 2, "cg_ev7400"),
    (641, 5, "cg_ev8702"),
    (646, 1, "cg_ev8900"),
];

/// CG mode pictures, from a table in the executable: background and the
/// layers over it in slots 0... The seen bit of picture `i` is bit `i % 8` of byte
/// [`CG_SEEN`]` + i / 8`.
pub const CG_IMAGES: [&[&str]; 647] = [
    &["ev0100"],
    &["ev0100", "ev0100_h01"],
    &["ev0100", "ev0100_h02"],
    &["ev0402"],
    &["ev0403"],
    &["ev0404"],
    &["ev0400"],
    &["ev0400a"],
    &["ev0405"],
    &["ev0406"],
    &["ev0400b"],
    &["ev0500"],
    &["ev0500", "ev0500_h01"],
    &["ev0500", "ev0500_h02"],
    &["ev0600", "ev0600_h02"],
    &["ev0600"],
    &["ev0600", "ev0600_h01"],
    &["ev0600", "ev0600_h05"],
    &["ev0600", "ev0600_h06"],
    &["ev0600", "ev0600_h04"],
    &["ev0600", "ev0600_h03"],
    &["ev0702"],
    &["ev0703"],
    &["ev0708"],
    &["ev0709"],
    &["ev0710"],
    &["ev0704"],
    &["ev0711"],
    &["ev0712"],
    &["ev0713"],
    &["ev0714"],
    &["ev0700"],
    &["ev0706"],
    &["ev0707"],
    &["ev0800"],
    &["ev0802"],
    &["ev0803"],
    &["ev1000"],
    &["ev1000a"],
    &["ev1000b"],
    &["ev1000c"],
    &["ev1000d"],
    &["ev1003"],
    &["ev1003_h01"],
    &["ev1000_h01"],
    &["ev1002_h01"],
    &["ev1006"],
    &["ev1006_h01"],
    &["ev1004"],
    &["ev1004_h01"],
    &["ev1004_h02"],
    &["ev1009"],
    &["ev0900"],
    &["ev0902"],
    &["ev0901"],
    &["ev0904"],
    &["ev0905"],
    &["ev0906"],
    &["ev1100"],
    &["ev1101"],
    &["ev1200"],
    &["ev1201"],
    &["ev1201_h01"],
    &["ev1202"],
    &["ev1203"],
    &["ev1220"],
    &["ev1207"],
    &["ev1207", "ev1207_h01"],
    &["ev1208_h04"],
    &["ev1209"],
    &["ev1208"],
    &["ev1210"],
    &["ev1209_h01"],
    &["ev1221_h01"],
    &["ev1221"],
    &["ev1212"],
    &["ev1207_h02"],
    &["ev1206"],
    &["ev1206_h01"],
    &["ev1206_h02"],
    &["ev1215"],
    &["ev1216"],
    &["ev1216_h01"],
    &["ev1222_h01"],
    &["ev1224"],
    &["ev1222"],
    &["ev1219"],
    &["ev1205"],
    &["ev1219_h01"],
    &["ev1223_h01"],
    &["ev1223"],
    &["ev1400"],
    &["ev9900"],
    &["ev9902"],
    &["ev9901"],
    &["ev9903"],
    &["ev9904"],
    &["ev9905"],
    &["ev9906"],
    &["ev9907"],
    &["ev9908"],
    &["ev9909"],
    &["ev9910"],
    &["ev9911"],
    &["ev9911_h01"],
    &["ev9935"],
    &["ev9912"],
    &["ev9913"],
    &["ev9936"],
    &["ev9914"],
    &["ev9917"],
    &["ev9916"],
    &["ev9916_h01"],
    &["ev9915_h01"],
    &["ev9915"],
    &["ev9918"],
    &["ev9919"],
    &["ev9921"],
    &["ev9920"],
    &["ev9922"],
    &["ev9924"],
    &["ev9929"],
    &["ev9930"],
    &["ev9925"],
    &["ev9926"],
    &["ev9927"],
    &["ev9928"],
    &["ev9931"],
    &["ev9932"],
    &["ev9933"],
    &["ev9934"],
    &["ev1500"],
    &["ev1501"],
    &["ev1502"],
    &["ev1504"],
    &["ev1504_h01"],
    &["ev1503"],
    &["ev1502_h01"],
    &["ev1300"],
    &["ev1301"],
    &["ev1600"],
    &["ev1600", "ev1600_h02"],
    &["ev1600", "ev1600_h01"],
    &["ev1600", "ev1600_h03"],
    &["ev1601", "ev1600_h01"],
    &["ev1601"],
    &["ev1601", "ev1600_h04"],
    &["ev1603"],
    &["ev1603", "ev1600_h04"],
    &["ev1604", "ev1600_h04"],
    &["ev1604", "ev1600_h02"],
    &["ev1605", "ev1600_h03"],
    &["ev1605", "ev1600_h02"],
    &["ev1605"],
    &["ev1605", "ev1600_h01"],
    &["ev1605", "ev1600_h04"],
    &["ev1606", "ev1600_h04"],
    &["ev1606"],
    &["ev1606", "ev1600_h01"],
    &["ev1606", "ev1600_h02"],
    &["ev1607", "ev1600_h03"],
    &["ev1607", "ev1600_h04"],
    &["ev1607"],
    &["ev1607", "ev1600_h02"],
    &["ev1607", "ev1600_h01"],
    &["ev1608", "ev1600_h03"],
    &["ev1608"],
    &["ev1608", "ev1600_h04"],
    &["ev1604"],
    &["ev1700"],
    &["ev1700_h01"],
    &["ev1701"],
    &["ev1702_h02"],
    &["ev1702_h01"],
    &["ev1704"],
    &["ev1705"],
    &["ev1805"],
    &["ev1801"],
    &["ev1800"],
    &["ev1802", "ev1800_h01"],
    &["ev1804"],
    &["ev1806"],
    &["ev1900"],
    &["ev1900", "ev1900_h01"],
    &["ev1900", "ev1900_h02"],
    &["ev1900", "ev1900_h11"],
    &["ev1900", "ev1900_h12"],
    &["ev1901", "ev1901_h03"],
    &["ev1901", "ev1901_h01"],
    &["ev1903", "ev1901_h04"],
    &["ev1902", "ev1902_h13"],
    &["ev1904", "ev1902_h14"],
    &["ev1900", "ev1900_h01", "ev1900_h12"],
    &["ev1900", "ev1900_h02", "ev1900_h12"],
    &["ev1905"],
    &["ev2000"],
    &["ev2000", "ev2000_h04"],
    &["ev2000", "ev2000_h04", "ev2000_h11"],
    &["ev2001", "ev2000_h12"],
    &["ev2001", "ev2000_h01"],
    &["ev2001"],
    &["ev2001", "ev2000_h04"],
    &["ev2001", "ev2000_h11"],
    &["ev2002", "ev2000_h13"],
    &["ev2002"],
    &["ev2002", "ev2000_h04", "ev2000_h12"],
    &["ev2002", "ev2000_h04", "ev2000_h13"],
    &["ev2002", "ev2000_h04", "ev2000_h14"],
    &["ev2003", "ev2000_h04", "ev2000_h14"],
    &["ev2003", "ev2000_h04", "ev2000_h15"],
    &["ev2003", "ev2000_h15"],
    &["ev2004", "ev2000_h01", "ev2000_h15"],
    &["ev2004", "ev2000_h01", "ev2000_h11"],
    &["ev2004", "ev2000_h02", "ev2000_h11"],
    &["ev2005", "ev2000_h02", "ev2000_h11"],
    &["ev2005", "ev2000_h03", "ev2000_h11"],
    &["ev2005", "ev2000_h05", "ev2000_h11"],
    &["ev2006"],
    &["ev2006", "ev2000_h02", "ev2000_h13"],
    &["ev2006", "ev2000_h03", "ev2000_h14"],
    &["ev2007", "ev2000_h03", "ev2000_h14"],
    &["ev2007", "ev2000_h03", "ev2000_h15"],
    &["ev2007", "ev2000_h05", "ev2000_h15"],
    &["ev2100"],
    &["ev2200"],
    &["ev2201"],
    &["ev2202"],
    &["ev2306"],
    &["ev2300"],
    &["ev2301"],
    &["ev2302"],
    &["ev2303"],
    &["ev2304"],
    &["ev2305"],
    &["ev2400"],
    &["ev2400", "ev2400_h01"],
    &["ev2400", "ev2400_h02"],
    &["ev2400", "ev2400_h03"],
    &["ev2400", "ev2400_h04"],
    &["ev2400", "ev2400_h05"],
    &["ev2501"],
    &["ev2504"],
    &["ev2502"],
    &["ev2500"],
    &["ev2503"],
    &["ev2600"],
    &["ev2601", "ev2600_h01"],
    &["ev2602", "ev2600_h02"],
    &["ev2602", "ev2600_h03"],
    &["ev2602", "ev2600_h01"],
    &["ev2602", "ev2600_h01", "ev2600_h11"],
    &["ev2602", "ev2600_h01", "ev2600_h12"],
    &["ev2603", "ev2600_h02", "ev2600_h11"],
    &["ev2603", "ev2600_h03", "ev2600_h11"],
    &["ev2603", "ev2600_h01", "ev2600_h11"],
    &["ev2603", "ev2600_h04", "ev2600_h13"],
    &["ev2603", "ev2600_h01"],
    &["ev2604", "ev2600_h03", "ev2600_h13"],
    &["ev2605", "ev2600_h03", "ev2600_h12"],
    &["ev2605", "ev2600_h01", "ev2600_h12"],
    &["ev2605", "ev2600_h04", "ev2600_h13"],
    &["ev2605"],
    &["ev2700"],
    &["ev2700", "ev2700_h04"],
    &["ev2700", "ev2700_h05"],
    &["ev2700", "ev2700_h01"],
    &["ev2701", "ev2701_h03"],
    &["ev2701", "ev2701_h05"],
    &["ev2701", "ev2701_h01"],
    &["ev2702", "ev2702_h02"],
    &["ev2702", "ev2702_h03"],
    &["ev2702", "ev2702_h05"],
    &["ev2703", "ev2703_h02"],
    &["ev2703", "ev2703_h01"],
    &["ev2703", "ev2703_h03"],
    &["ev2704", "ev2704_h04"],
    &["ev2704", "ev2704_h05"],
    &["ev2900"],
    &["ev2900", "ev2900_h01"],
    &["ev2901", "ev2901_h02"],
    &["ev2902", "ev2902_h03"],
    &["ev2903", "ev2903_h04"],
    &["ev2904", "ev2904_h05"],
    &["ev2800"],
    &["ev2801", "ev2800_h01"],
    &["ev2802", "ev2800_h01"],
    &["ev2802", "ev2800_h03"],
    &["ev2802", "ev2800_h04"],
    &["ev2803", "ev2800_h01"],
    &["ev2804", "ev2800_h03"],
    &["ev2804", "ev2800_h04"],
    &["ev2804", "ev2800_h01"],
    &["ev2805", "ev2800_h03"],
    &["ev2805", "ev2800_h04"],
    &["ev2805", "ev2800_h02"],
    &["ev2806", "ev2800_h03"],
    &["ev2806", "ev2800_h04"],
    &["ev7200"],
    &["ev7200", "ev7200_h01"],
    &["ev3200"],
    &["ev7300"],
    &["ev7301"],
    &["ev7302"],
    &["ev3000"],
    &["ev3002"],
    &["ev3003"],
    &["ev3300"],
    &["ev3300", "ev3300_h02"],
    &["ev3300", "ev3300_h01"],
    &["ev3301", "ev3301_h04"],
    &["ev3301", "ev3301_h02"],
    &["ev3302", "ev3301_h04"],
    &["ev3302", "ev3301_h05"],
    &["ev3302", "ev3301_h02"],
    &["ev3303", "ev3301_h04"],
    &["ev3303", "ev3301_h05"],
    &["ev3303", "ev3301_h02"],
    &["ev3400"],
    &["ev3401"],
    &["ev3401", "ev3400_h01"],
    &["ev3401", "ev3400_h02"],
    &["ev3402", "ev3400_h01"],
    &["ev3402"],
    &["ev3403", "ev3400_h02"],
    &["ev3403", "ev3400_h01"],
    &["ev3500"],
    &["ev3500", "ev3500_h01", "ev3500_h10"],
    &["ev3500", "ev3500_h01", "ev3500_h30"],
    &["ev3500", "ev3500_h30"],
    &["ev3501", "ev3501_h03", "ev3501_h30"],
    &["ev3501", "ev3501_h03", "ev3501_h10"],
    &["ev3501", "ev3501_h01", "ev3501_h10"],
    &["ev3502", "ev3502_h02"],
    &["ev3501"],
    &["ev3503", "ev3503_h04"],
    &["ev3503", "ev3503_h01"],
    &["ev3504", "ev3504_h03"],
    &["ev3504"],
    &["ev3504", "ev3504_h01"],
    &["ev3505", "ev3505_h01"],
    &["ev3506", "ev3505_h01", "ev3506_h30"],
    &["ev3506", "ev3506_h30"],
    &["ev3507", "ev3507_h40"],
    &["ev3507", "ev3505_h01", "ev3507_h40"],
    &["ev3507", "ev3505_h01", "ev3507_h30"],
    &["ev3508"],
    &["ev3600"],
    &["ev3601", "ev3600_h02", "ev3600_h11"],
    &["ev3601", "ev3600_h03", "ev3600_h11"],
    &["ev3602", "ev3600_h03", "ev3600_h11"],
    &["ev3602", "ev3600_h01", "ev3600_h11"],
    &["ev3603", "ev3600_h01", "ev3600_h12"],
    &["ev3603", "ev3600_h01", "ev3600_h13"],
    &["ev3604", "ev3600_h01", "ev3600_h13"],
    &["ev3604", "ev3600_h01", "ev3600_h11"],
    &["ev3604", "ev3600_h02", "ev3600_h12"],
    &["ev3605", "ev3605_h03", "ev3605_h13"],
    &["ev3605", "ev3605_h01", "ev3605_h11"],
    &["ev3700"],
    &["ev3701", "ev3700_h01"],
    &["ev3700", "ev3700_h01"],
    &["ev3700", "ev3700_h02"],
    &["ev3700", "ev3700_h03"],
    &["ev3701", "ev3700_h04"],
    &["ev3701", "ev3700_h042"],
    &["ev3700", "ev3700_h04"],
    &["ev3800"],
    &["ev3800", "ev3800_h01"],
    &["ev3800", "ev3800_h02"],
    &["ev3800", "ev3800_h03"],
    &["ev3900"],
    &["ev3900", "ev3900_h01"],
    &["ev3900", "ev3900_h02"],
    &["ev3901", "ev3900_h01"],
    &["ev3901", "ev3900_h03"],
    &["ev3901", "ev3900_h05"],
    &["ev3901", "ev3900_h06"],
    &["ev4000"],
    &["ev4001", "ev4000_h01"],
    &["ev4001"],
    &["ev4002"],
    &["ev4002", "ev4000_h02"],
    &["ev4003", "ev4000_h01"],
    &["ev4003", "ev4000_h02"],
    &["ev4003", "ev4000_h03"],
    &["ev4003", "ev4000_h04"],
    &["ev4004", "ev4000_h03"],
    &["ev4005", "ev4005_h04"],
    &["ev4005", "ev4005_h03"],
    &["ev4005", "ev4005_h02"],
    &["ev4100"],
    &["ev4200"],
    &["ev4200", "ev4200_h01"],
    &["ev4201", "ev4200_h01"],
    &["ev4201", "ev4200_h02"],
    &["ev4201", "ev4200_h03"],
    &["ev4202", "ev4200_h03"],
    &["ev4202", "ev4200_h04"],
    &["ev4202", "ev4200_h01"],
    &["ev4202", "ev4200_h02"],
    &["ev4203", "ev4200_h03"],
    &["ev4203", "ev4200_h04"],
    &["ev4300"],
    &["ev4301"],
    &["ev0305"],
    &["ev0305_01"],
    &["ev0305_02"],
    &["ev0309"],
    &["ev0305_03"],
    &["ev0309_01"],
    &["ev0310"],
    &["ev0305_h01"],
    &["ev0305_03_h01"],
    &["ev0306"],
    &["ev0307"],
    &["ev4400"],
    &["ev4400", "ev4400_h03"],
    &["ev4400", "ev4400_h01"],
    &["ev4401", "ev4401_h01"],
    &["ev4402"],
    &["ev4403"],
    &["ev4403", "ev4403_h01"],
    &["ev4403", "ev4403_h03"],
    &["ev4404", "ev4403_h05"],
    &["ev4405"],
    &["ev4406", "ev4406_h06"],
    &["ev4406", "ev4406_h03"],
    &["ev4407", "ev4406_h05"],
    &["ev4407", "ev4406_h06"],
    &["ev4407"],
    &["ev4500"],
    &["ev4501", "ev4500_h01"],
    &["ev4501", "ev4500_h03"],
    &["ev4502", "ev4500_h04"],
    &["ev4502", "ev4500_h05"],
    &["ev4503", "ev4500_h05"],
    &["ev4503", "ev4500_h01"],
    &["ev4504", "ev4500_h02"],
    &["ev4504", "ev4500_h03"],
    &["ev4504", "ev4500_h01"],
    &["ev4505", "ev4500_h04"],
    &["ev4505", "ev4500_h05"],
    &["ev4506", "ev4500_h05"],
    &["ev4600"],
    &["ev4601"],
    &["ev4602"],
    &["ev4603"],
    &["ev4603", "ev4603_h01"],
    &["ev4604"],
    &["ev4605"],
    &["ev4601", "ev4601_h01"],
    &["ev4700"],
    &["ev4700", "ev4700_h01"],
    &["ev4700", "ev4700_h11"],
    &["ev4700", "ev4700_h03", "ev4700_h11"],
    &["ev4800"],
    &["ev4800", "ev4800_h01"],
    &["ev4800", "ev4800_h02"],
    &["ev4900", "ev4900_h01"],
    &["ev4900"],
    &["ev5000"],
    &["ev5000", "ev5000_h01"],
    &["ev5001", "ev5000_h01"],
    &["ev5002", "ev5000_h02"],
    &["ev5002", "ev5000_h01"],
    &["ev5002"],
    &["ev5003", "ev5000_h02"],
    &["ev5003"],
    &["ev5100"],
    &["ev5100", "ev5100_h01"],
    &["ev5100", "ev5100_h02"],
    &["ev5100", "ev5100_h03"],
    &["ev5300"],
    &["ev5300", "ev5300_h01"],
    &["ev5301", "ev5301_h01"],
    &["ev5302", "ev5302_h04"],
    &["ev5303", "ev5303_h05"],
    &["ev5200"],
    &["ev5200", "ev5200_h01"],
    &["ev5201", "ev5200_h01"],
    &["ev5201", "ev5200_h02"],
    &["ev5201", "ev5200_h03"],
    &["ev5202", "ev5200_h02"],
    &["ev5203", "ev5200_h03"],
    &["ev5400"],
    &["ev5401", "ev5401_h01"],
    &["ev5402", "ev5401_h03"],
    &["ev5402", "ev5401_h01"],
    &["ev5403", "ev5401_h02"],
    &["ev5404", "ev5401_h02"],
    &["ev5404", "ev5401_h03"],
    &["ev5600"],
    &["ev5601"],
    &["ev5602"],
    &["ev5500"],
    &["ev5501"],
    &["ev5502", "ev5500_h01"],
    &["ev5503", "ev5500_h02"],
    &["ev5503"],
    &["ev5504"],
    &["ev5504", "ev5500_h02"],
    &["ev5505", "ev5500_h03"],
    &["ev5505"],
    &["ev5700"],
    &["ev5700", "ev5700_h01"],
    &["ev5701", "ev5701_h01"],
    &["ev5800"],
    &["ev5800", "ev5800_h01"],
    &["ev5902"],
    &["ev5903"],
    &["ev5904"],
    &["ev5900"],
    &["ev5900", "ev5901"],
    &["ev6000"],
    &["ev6001"],
    &["ev6002"],
    &["ev6003"],
    &["ev6100"],
    &["ev6200"],
    &["ev6200", "ev6200_h01"],
    &["ev6201"],
    &["ev6202", "ev6200_h02"],
    &["ev6203", "ev6200_h03"],
    &["ev6203", "ev6200_h01"],
    &["ev6204", "ev6200_h01"],
    &["ev6204", "ev6200_h03"],
    &["ev6204", "ev6200_h04"],
    &["ev6204", "ev6200_h02"],
    &["ev6205", "ev6200_h03"],
    &["ev6205", "ev6200_h04"],
    &["ev6206", "ev6200_h04"],
    &["ev6400"],
    &["ev6301"],
    &["ev6300"],
    &["ev6302", "ev6304_h02"],
    &["ev6303", "ev6304_h02"],
    &["ev6303", "ev6304_h03"],
    &["ev6304", "ev6304_h02"],
    &["ev6304", "ev6304_h03"],
    &["ev6500"],
    &["ev6500", "ev6500_h01"],
    &["ev6501", "ev6500_h01"],
    &["ev6501", "ev6500_h02"],
    &["ev6501"],
    &["ev6502", "ev6500_h01"],
    &["ev6602"],
    &["ev6600", "ev6600_h01"],
    &["ev6601", "ev6600_h01"],
    &["ev6601", "ev6600_h02"],
    &["ev6601"],
    &["ev6603", "ev6600_h01"],
    &["ev6603", "ev6600_h03"],
    &["ev6604", "ev6600_h03"],
    &["ev6604", "ev6600_h02"],
    &["ev6604", "ev6600_h01"],
    &["ev6605", "ev6600_h03"],
    &["ev6605", "ev6600_h02"],
    &["ev6605", "ev6600_h04"],
    &["ev6700"],
    &["ev6700", "ev6700_h01"],
    &["ev6701"],
    &["ev6702"],
    &["ev6800"],
    &["ev6800", "ev6800_h01"],
    &["ev6801"],
    &["ev6802"],
    &["ev6900"],
    &["ev6900", "ev6900_h01"],
    &["ev7000"],
    &["ev7000", "ev7000_h01"],
    &["ev7000", "ev7000_h02"],
    &["ev7100"],
    &["ev7100", "ev7100_h01", "ev7100_h11"],
    &["ev7603"],
    &["ev7601"],
    &["ev7604"],
    &["ev7605"],
    &["ev7602"],
    &["ev7700"],
    &["ev7700", "ev7700_h01"],
    &["ev7800"],
    &["ev7800", "ev7800_h01"],
    &["ev7801", "ev7800_h01"],
    &["ev7802", "ev7800_h02"],
    &["ev7802", "ev7800_h03"],
    &["ev7802"],
    &["ev7900"],
    &["ev7900", "ev7900_h01"],
    &["ev7901", "ev7900_h03"],
    &["ev7901", "ev7900_h01"],
    &["ev7901"],
    &["ev7901", "ev7900_h02"],
    &["ev7902", "ev7900_h02"],
    &["ev7902"],
    &["ev7902", "ev7900_h01"],
    &["ev7903", "ev7900_h01"],
    &["ev7903", "ev7900_h03"],
    &["ev7903", "ev7900_h02"],
    &["ev7904", "ev7900_h03"],
    &["ev7904"],
    &["ev7904", "ev7900_h01"],
    &["ev8000"],
    &["ev8001", "ev8000_h01"],
    &["ev8002", "ev8000_h02"],
    &["ev8002"],
    &["ev8003", "ev8000_h02"],
    &["ev8004", "ev8000_h03"],
    &["ev8004", "ev8000_h02"],
    &["ev8005", "ev8000_h02"],
    &["ev8005", "ev8000_h03"],
    &["ev8100"],
    &["ev8101", "ev8100_h01"],
    &["ev8101", "ev8100_h02"],
    &["ev8101"],
    &["ev8102", "ev8100_h01"],
    &["ev8102", "ev8100_h02"],
    &["ev8500"],
    &["ev8501"],
    &["ev8501", "ev8500_h01"],
    &["ev8502", "ev8500_h01"],
    &["ev8502", "ev8500_h02"],
    &["ev8503", "ev8500_h01"],
    &["ev8503", "ev8500_h02"],
    &["ev8200"],
    &["ev8200", "ev8200_h01"],
    &["ev8201", "ev8200_h01"],
    &["ev8201", "ev8200_h01", "ev8200_h11"],
    &["ev8300"],
    &["ev8400"],
    &["ev8401"],
    &["ev8402"],
    &["ev8403"],
    &["ev8600"],
    &["ev8601"],
    &["ev8602"],
    &["ev8603"],
    &["ev8604"],
    &["ev8605"],
    &["ev8606"],
    &["ev7400"],
    &["ev7400", "ev7400_h01"],
    &["ev8702"],
    &["ev8700"],
    &["ev8701"],
    &["ev8702", "ev8700_h01"],
    &["ev8702", "ev8700_h02"],
    &["ev8900"],
];
