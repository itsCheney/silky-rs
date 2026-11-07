//! The extras of きまぐれテンプテーション2 (CG, scene and music modes,
//! `Game.30` to `Game.32`), read from its executable. They share the area
//! files of ふゆから、くるる。 (`cgmode.area`, `scmode.area`,
//! `soundmode.area`) but draw from one sheet, `mode_p.akb`, and keep what
//! was seen in the script's byte variables (kept across games):
//!
//! - CG: a bit set from byte 3300, one bit per picture of [`CG_IMAGES`]. The
//!   engine sets a bit when a picture starting with `hg` and its layers
//!   exactly match a row of the table (and sets byte 3002).
//! - Scenes: bytes 3101..=3106.
//! - Music: bytes 3051..=3079, set by the scripts as tracks play.
//!
//! The scripts draw the backgrounds (`cgmodebg`, `scenemodebg`,
//! `musicmodebg`); this module draws them too, as it keeps the screens
//! itself while switching between modes.

use silky_format::akb::Image;
use silky_format::area::Area;

use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::ui::{Button, Ui};
use crate::{Extras, Result};

pub const CG_SEEN: usize = 3300;
/// Set once some CG picture was seen, and by "unlock all".
pub const CG_ANY: usize = 3002;
pub const CG_UNLOCKED: usize = 3001;
pub const SCENE_FIRST: usize = 3101;
pub const SCENES: usize = 6;
pub const MUSIC_FIRST: usize = 3051;
pub const TRACKS: usize = 29;

/// Cells per page, four by three.
const CELLS: usize = 12;
pub const CG_PAGES: usize = CG_ENTRIES.len().div_ceil(CELLS);
const CELL: (i32, i32, i32, i32) = (197, 135, 266, 154);
const CARD: (u32, u32) = (262, 150);
const THUMB: (u32, u32) = (258, 146);
/// Music rows shown at once, and the last first row.
const ROWS: usize = 7;
pub const MAX_SCROLL: usize = TRACKS - ROWS;
/// Viewer cells and the full picture: ids of no area file.
pub const VIEW_CELL: i32 = 0x1000;
/// A click beside the viewer's cells closes it.
pub const VIEW_CLOSE: i32 = 0xfff;

/// Ids of the area files, as in [`crate::menus::extras_ids`].
pub mod ids {
    pub const PREVIOUS: i32 = 0x100;
    pub const NEXT: i32 = 0x101;
    pub const SCROLL: i32 = 0x102;
    pub const PAGE: i32 = 0x200;
    pub const VOLUME: i32 = 0x200;
    pub const BACK: i32 = 0x300;
    pub const UNLOCK: i32 = 0x400;
    pub const MODE: i32 = 0xa00;
}

/// The viewer of one CG entry: a grid of its pictures over the dimmed
/// screen, and one of them over the whole screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub entry: usize,
    pub page: usize,
    /// Index into the entry's pictures, when one is shown.
    pub picture: Option<usize>,
}

/// Byte variables as the VM holds them.
pub fn byte(bytes: &[u8], i: usize) -> u8 {
    bytes.get(i).copied().unwrap_or(0)
}

pub fn cg_seen(bytes: &[u8], picture: usize) -> bool {
    byte(bytes, CG_SEEN + picture / 8) & (1 << (picture % 8)) != 0
}

pub fn scene_seen(bytes: &[u8], scene: usize) -> bool {
    scene < SCENES && byte(bytes, SCENE_FIRST + scene) != 0
}

pub fn track_heard(bytes: &[u8], track: usize) -> bool {
    track < TRACKS && byte(bytes, MUSIC_FIRST + track) == 1
}

pub fn track_file(track: usize) -> String {
    format!("kima2_{:02}.wav", track + 1)
}

/// Pictures of a CG entry, as indices of [`CG_IMAGES`].
fn entry_pictures(entry: usize) -> std::ops::Range<usize> {
    let (first, count, _) = CG_ENTRIES[entry];
    first..first + count
}

/// First picture of an entry that was seen, by its place in the entry.
fn first_seen(bytes: &[u8], entry: usize) -> Option<usize> {
    entry_pictures(entry).position(|p| cg_seen(bytes, p))
}

/// Next seen picture of an entry after `from`.
pub fn next_seen(bytes: &[u8], entry: usize, from: usize) -> Option<usize> {
    entry_pictures(entry)
        .enumerate()
        .skip(from + 1)
        .find(|&(_, p)| cg_seen(bytes, p))
        .map(|(i, _)| i)
}

/// Completion in per cent, at least 1 once anything was seen (as the
/// engine rounds it).
fn percent(seen: usize, total: usize) -> u32 {
    match seen {
        0 => 0,
        _ => ((seen as f32 / total as f32 * 100.0) as u32).max(1),
    }
}

/// Marks the picture on screen as seen when it is a CG of the table: the
/// background and the layers in slot order, compared without case and
/// extension. Returns whether a bit was new.
pub fn register_cg<'a>(
    bytes: &mut [u8],
    background: &str,
    layers: impl Iterator<Item = &'a str>,
) -> Option<usize> {
    let norm = |s: &str| {
        let s = s.to_ascii_lowercase();
        s.strip_suffix(".akb").map(str::to_string).unwrap_or(s)
    };
    let bg = norm(background);
    if !bg.starts_with("hg") {
        return None;
    }
    let names: Vec<String> = std::iter::once(bg).chain(layers.map(norm)).collect();
    let i = CG_IMAGES
        .iter()
        .position(|row| row.len() == names.len() && row.iter().zip(&names).all(|(a, b)| a == b))?;
    let at = CG_SEEN + i / 8;
    if at >= bytes.len() || bytes[at] & (1 << (i % 8)) != 0 {
        return None;
    }
    bytes[at] |= 1 << (i % 8);
    Some(i)
}

/// What "unlock all" sets: (variable, value).
pub fn unlock_all(kind: Extras) -> Vec<(usize, u8)> {
    match kind {
        Extras::Gallery => (0..CG_IMAGES.len().div_ceil(8))
            .map(|i| {
                let bits = (CG_IMAGES.len() - i * 8).min(8);
                (CG_SEEN + i, (0xffu16 >> (8 - bits)) as u8)
            })
            .chain([(CG_UNLOCKED, 1), (CG_ANY, 1)])
            .collect(),
        Extras::Scenes => (0..SCENES).map(|i| (SCENE_FIRST + i, 1)).collect(),
        Extras::Music => (0..TRACKS).map(|i| (MUSIC_FIRST + i, 1)).collect(),
    }
}

/// The screen as the session keeps it.
#[derive(Debug, Clone, Default)]
pub struct Omake {
    pub view: Option<View>,
    pub dragging_volume: bool,
}

fn sheet_name(base: &str, assets: &Assets, language: Language) -> String {
    let suffix = match language {
        Language::English => Some("en"),
        Language::SimplifiedChinese | Language::TraditionalChinese => Some("ch"),
        Language::Japanese => None,
    };
    if let Some(s) = suffix {
        let name = format!("{base}_{s}.akb");
        if assets.exists(silky_format::game::Kind::Layer, &name) {
            return name;
        }
    }
    format!("{base}.akb")
}

/// Cell `i` of a page on screen.
fn cell(i: usize) -> (i32, i32) {
    let (x, y, dx, dy) = CELL;
    (x + dx * (i % 4) as i32, y + dy * (i / 4) as i32)
}

/// Viewer grid: where cell `i` of `n` goes.
fn view_cell(i: usize, n: usize) -> (i32, i32) {
    let cols = n.min(4) as i32;
    let rows = n.div_ceil(4) as i32;
    let x0 = (1280 - cols * 266) / 2 + 4;
    let y0 = (614 - rows * 154) / 2 + 16;
    (x0 + 266 * (i % 4) as i32, y0 + 154 * (i / 4) as i32)
}

fn view_count(v: &View) -> usize {
    (CG_ENTRIES[v.entry].1 - v.page * CELLS).min(CELLS)
}

/// The item behind cell or row `i`: CG entry, scene, track.
pub fn item(kind: Extras, page: usize, scroll: usize, i: usize) -> Option<usize> {
    let (k, n) = match kind {
        Extras::Gallery => (page * CELLS + i, CG_ENTRIES.len()),
        Extras::Scenes => (page * CELLS + i, SCENES),
        Extras::Music => (scroll + i, TRACKS),
    };
    (i < if kind == Extras::Music { ROWS } else { CELLS } && k < n).then_some(k)
}

pub fn open(kind: Extras, bytes: &[u8], item: usize) -> bool {
    match kind {
        Extras::Gallery => first_seen(bytes, item).is_some(),
        Extras::Scenes => scene_seen(bytes, item),
        Extras::Music => track_heard(bytes, item),
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
    if let Some(v) = &o.view {
        if v.picture.is_some() {
            return None;
        }
        let n = view_count(v);
        return (0..n)
            .find(|&i| {
                let (cx, cy) = view_cell(i, n);
                x >= cx && x < cx + CARD.0 as i32 && y >= cy && y < cy + CARD.1 as i32
            })
            .map_or(Some(VIEW_CLOSE), |i| {
                cg_seen(bytes, CG_ENTRIES[v.entry].0 + v.page * CELLS + i)
                    .then_some(VIEW_CELL + i as i32)
            })
            .map(Button::Screen);
    }
    let a = areas.iter().find(|a| a.contains(x, y))?;
    let id = a.id;
    if (0..0x100).contains(&id) {
        let it = item(kind, page, scroll, id as usize)?;
        if !open(kind, bytes, it) {
            return None;
        }
    }
    if kind == Extras::Scenes && (ids::PREVIOUS..ids::MODE).contains(&id) && id >> 8 <= 2 {
        // One page of scenes: no paging.
        return None;
    }
    Some(Button::Screen(id))
}

/// Draws the screen.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    canvas: &mut Canvas,
    assets: &mut Assets,
    language: Language,
    ui: &Ui,
    kind: Extras,
    page: usize,
    scroll: usize,
    playing: Option<usize>,
    o: &Omake,
    bytes: &[u8],
    volume: f32,
) -> Result<()> {
    if let Some(v) = &o.view {
        if let Some(p) = v.picture {
            return draw_picture(canvas, assets, CG_ENTRIES[v.entry].0 + p);
        }
    }
    let bg = match kind {
        Extras::Gallery => "cgmodebg.akb",
        Extras::Scenes => "scenemodebg.akb",
        Extras::Music => "musicmodebg.akb",
    };
    canvas.draw(&*assets.image(bg)?, 0, 0, false);
    let sheet = assets.image(&sheet_name("mode_p", assets, language))?;
    let st = |id: i32, enabled: bool| ui.state(Button::Screen(id), enabled, false) as u32;
    let part = |c: &mut Canvas, img: &Image, src: (u32, u32), dst: (i32, i32), size: (u32, u32)| {
        c.draw_part(
            img,
            (src.0, src.1, size.0, size.1),
            dst.0,
            dst.1,
            false,
            255,
        )
    };
    // Mode buttons, back and unlock.
    for k in 0..3u32 {
        let id = ids::MODE + k as i32;
        let row = if k == kind as u32 { 2 } else { st(id, true) };
        part(
            canvas,
            &sheet,
            (190 * k, 144 * row),
            (0, 154 + 144 * k as i32),
            (190, 144),
        );
    }
    let back = match kind {
        Extras::Gallery => (570, 106 * st(ids::BACK, true)),
        Extras::Scenes => (570, 106 * (st(ids::BACK, true) + 3)),
        Extras::Music => (676, 106 * st(ids::BACK, true)),
    };
    part(canvas, &sheet, back, (8, 607), (106, 106));
    let unlock_at = match kind {
        Extras::Music => (160, 577),
        _ => (190, 597),
    };
    part(
        canvas,
        &sheet,
        (676, 318 + 86 * st(ids::UNLOCK, true)),
        unlock_at,
        (85, 86),
    );
    match kind {
        Extras::Gallery => draw_cg(canvas, assets, &sheet, ui, page, bytes)?,
        Extras::Scenes => draw_scenes(canvas, assets, &sheet, ui, bytes)?,
        Extras::Music => draw_music(
            canvas, assets, language, &sheet, ui, scroll, playing, bytes, volume,
        )?,
    }
    if let Some(v) = &o.view {
        draw_view(canvas, assets, &sheet, ui, v, bytes)?;
    }
    Ok(())
}

/// Big digits of the completion, right aligned before the per cent sign.
fn draw_percent(canvas: &mut Canvas, sheet: &Image, row: u32, mut value: u32) {
    canvas.draw_part(sheet, (480, row, 48, 56), 1175, 52, false, 255);
    let mut x = 1127;
    loop {
        canvas.draw_part(sheet, ((value % 10) * 48, row, 48, 56), x, 52, false, 255);
        value /= 10;
        x -= 48;
        if value == 0 {
            break;
        }
    }
}

/// Hover frame over a cell (normal frames at `x`, rows 0..2).
fn draw_hover(canvas: &mut Canvas, sheet: &Image, ui: &Ui, id: i32, x: u32, at: (i32, i32)) {
    let row = match ui.state(Button::Screen(id), true, false) as u32 {
        0 => return,
        r => r.min(2),
    };
    if row == 1 {
        canvas.draw_part(sheet, (x, 150, CARD.0, CARD.1), at.0, at.1, false, 255);
    } else {
        canvas.draw_part(sheet, (x, 300, CARD.0, CARD.1), at.0, at.1, false, 96);
    }
}

fn draw_cg(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Image,
    ui: &Ui,
    page: usize,
    bytes: &[u8],
) -> Result<()> {
    let st = |id: i32, enabled: bool| ui.state(Button::Screen(id), enabled, false) as u32;
    let thumbs = assets.image(&format!("cgmode_thm{page:02}.akb")).ok();
    for i in 0..CELLS {
        let Some(entry) = item(Extras::Gallery, page, 0, i) else {
            continue;
        };
        let (x, y) = cell(i);
        canvas.draw_part(sheet, (782, 450, CARD.0, CARD.1), x, y, false, 255);
        let Some(first) = first_seen(bytes, entry) else {
            continue;
        };
        let (tw, th) = THUMB;
        if first == 0 {
            if let Some(t) = &thumbs {
                let src = ((i % 4) as u32 * tw, (i / 4) as u32 * th, tw, th);
                canvas.draw_part(t, src, x + 2, y + 2, false, 255);
            }
        } else if let Ok(t) = assets.image(&format!("{}.akb", CG_ENTRIES[entry].2)) {
            let src = ((first % 4) as u32 * tw, (first / 4) as u32 * th, tw, th);
            canvas.draw_part(&t, src, x + 2, y + 2, false, 255);
        }
        let count = CG_ENTRIES[entry].1;
        if count != 1 {
            let seen = entry_pictures(entry).filter(|&p| cg_seen(bytes, p)).count();
            let digit = |c: &mut Canvas, d: usize, at: i32| {
                c.draw_part(sheet, (d as u32 * 21, 544, 21, 27), at, y + 109, false, 255)
            };
            if seen % 100 / 10 != 0 {
                digit(canvas, seen % 100 / 10, x + 120);
            }
            digit(canvas, seen % 10, x + 141);
            digit(canvas, 10, x + 162);
            let mut at = x + 197;
            if count % 100 / 10 != 0 {
                digit(canvas, count % 100 / 10, at);
                at = x + 218;
            }
            digit(canvas, count % 10, at);
        }
        draw_hover(canvas, sheet, ui, i as i32, 782, (x, y));
    }
    // Pages.
    let prev = if page == 0 {
        3
    } else {
        st(ids::PREVIOUS, true)
    };
    canvas.draw_part(sheet, (1308, prev * 40, 54, 40), 534, 665, false, 255);
    let next = if page + 1 == CG_PAGES {
        3
    } else {
        st(ids::NEXT, true)
    };
    canvas.draw_part(sheet, (1584, next * 40, 54, 40), 866, 665, false, 255);
    for p in 0..CG_PAGES {
        let id = ids::PAGE + p as i32;
        let y = if p == page { 80 } else { 40 * st(id, true) };
        canvas.draw_part(
            sheet,
            (1362 + 74 * p as u32, y, 74, 40),
            608 + 82 * p as i32,
            666,
            false,
            255,
        );
    }
    let total = CG_IMAGES.len();
    let seen = (0..total).filter(|&p| cg_seen(bytes, p)).count();
    draw_percent(canvas, sheet, 432, percent(seen, total));
    Ok(())
}

fn draw_scenes(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Image,
    ui: &Ui,
    bytes: &[u8],
) -> Result<()> {
    let thumbs = assets.image("scene_thumb.akb").ok();
    for i in 0..SCENES {
        let (x, y) = cell(i);
        if !scene_seen(bytes, i) {
            canvas.draw_part(sheet, (1046, 450, CARD.0, CARD.1), x, y, false, 255);
            continue;
        }
        canvas.draw_part(sheet, (1046, 0, CARD.0, CARD.1), x, y, false, 255);
        if let Some(t) = &thumbs {
            let (tw, th) = THUMB;
            let src = ((i % 4) as u32 * tw, (i / 4) as u32 * th, tw, th);
            canvas.draw_part(t, src, x + 2, y + 2, false, 255);
        }
        draw_hover(canvas, sheet, ui, i as i32, 1046, (x, y));
    }
    let seen = (0..SCENES).filter(|&i| scene_seen(bytes, i)).count();
    draw_percent(canvas, sheet, 488, percent(seen, SCENES));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_music(
    canvas: &mut Canvas,
    assets: &mut Assets,
    language: Language,
    sheet: &Image,
    ui: &Ui,
    scroll: usize,
    playing: Option<usize>,
    bytes: &[u8],
    volume: f32,
) -> Result<()> {
    let st = |id: i32| ui.state(Button::Screen(id), true, false) as u32;
    let list = assets.image(&sheet_name("music_list", assets, language))?;
    for i in 0..ROWS {
        let Some(t) = item(Extras::Music, 0, scroll, i) else {
            continue;
        };
        let row = if !track_heard(bytes, t) {
            3
        } else if playing == Some(t) {
            2
        } else {
            st(i as i32).min(2)
        };
        let src = (
            (t % 2) as u32 * 840,
            (row + (t / 2) as u32 * 4) * 64,
            840,
            64,
        );
        canvas.draw_part(&list, src, 257, 143 + 68 * i as i32, false, 255);
    }
    let up = if scroll == 0 { 3 } else { st(ids::PREVIOUS) };
    canvas.draw_part(sheet, (1348, 392 + 47 * up, 49, 47), 1119, 79, false, 255);
    let down = if scroll == MAX_SCROLL {
        3
    } else {
        st(ids::NEXT)
    };
    canvas.draw_part(
        sheet,
        (1348, 580 + 47 * down, 49, 47),
        1119,
        623,
        false,
        255,
    );
    let ty = 126 + (scroll as i32 * 433) / MAX_SCROLL as i32;
    canvas.draw_part(
        sheet,
        (1397, 392 + 64 * st(ids::SCROLL).min(2), 45, 64),
        1120,
        ty,
        false,
        255,
    );
    // Volume.
    canvas.draw_part(sheet, (1308, 320, 202, 24), 849, 636, false, 255);
    let w = (volume.clamp(0.0, 1.0) * 202.0).round() as u32;
    canvas.draw_part(sheet, (1308, 344, w, 24), 849, 636, false, 255);
    let knob = st(ids::VOLUME).min(2);
    canvas.draw_part(
        sheet,
        (1308, 392 + 40 * knob, 40, 40),
        830 + w as i32,
        628,
        false,
        255,
    );
    Ok(())
}

/// Volume for a pointer at `x` over the volume bar.
pub fn volume_at(x: i32) -> f32 {
    ((x - 849) as f32 / 202.0).clamp(0.0, 1.0)
}

/// Music scroll for a pointer at `y` over the scroll bar.
pub fn scroll_at(y: i32) -> usize {
    let f = ((y - 126 - 32) as f32 / 433.0).clamp(0.0, 1.0);
    (f * MAX_SCROLL as f32).round() as usize
}

fn draw_view(
    canvas: &mut Canvas,
    assets: &mut Assets,
    sheet: &Image,
    ui: &Ui,
    v: &View,
    bytes: &[u8],
) -> Result<()> {
    canvas.fill(0, 0, 1280, 720, [0, 0, 0, 0xcb]);
    let (first, _, name) = CG_ENTRIES[v.entry];
    let thumbs = assets.image(&format!("{name}.akb")).ok();
    let n = view_count(v);
    for i in 0..n {
        let k = v.page * CELLS + i;
        let (x, y) = view_cell(i, n);
        if !cg_seen(bytes, first + k) {
            canvas.draw_part(sheet, (782, 450, CARD.0, CARD.1), x, y, false, 255);
            continue;
        }
        canvas.draw_part(sheet, (782, 0, CARD.0, CARD.1), x, y, false, 255);
        if let Some(t) = &thumbs {
            let (tw, th) = THUMB;
            let src = ((k % 4) as u32 * tw, (k / 4) as u32 * th, tw, th);
            canvas.draw_part(t, src, x + 2, y + 2, false, 255);
        }
        draw_hover(canvas, sheet, ui, VIEW_CELL + i as i32, 782, (x, y));
    }
    Ok(())
}

/// A CG picture over the whole screen, centred like a background.
fn draw_picture(canvas: &mut Canvas, assets: &mut Assets, picture: usize) -> Result<()> {
    canvas.fill(0, 0, 1280, 720, [0, 0, 0, 255]);
    for (k, layer) in CG_IMAGES[picture].iter().enumerate() {
        let img = assets.image(&format!("{layer}.akb"))?;
        let x = (1280 - img.width as i32) / 2;
        let y = (720 - img.height as i32) / 2;
        canvas.draw(
            &img,
            if k == 0 { x } else { 0 },
            if k == 0 { y } else { 0 },
            k > 0,
        );
    }
    Ok(())
}

/// Pages of the viewer of an entry.
pub fn view_pages(entry: usize) -> usize {
    CG_ENTRIES[entry].1.div_ceil(CELLS)
}

/// CG mode entries, from the table at 0x575a78: first picture, number of
/// pictures and the sheet with their small thumbnails (4 x 258 x 146).
pub const CG_ENTRIES: [(usize, usize, &str); 29] = [
    (0, 9, "cg_hg020100_00_01"),
    (9, 16, "cg_hg020200_00"),
    (25, 6, "cg_hg010100"),
    (31, 1, "cg_hg010000_03"),
    (32, 4, "cg_hg020300"),
    (36, 6, "cg_hg020400"),
    (42, 8, "cg_hg020000"),
    (50, 56, "cg_hg030000_00"),
    (106, 12, "cg_hg010200_00"),
    (118, 9, "cg_hg010300"),
    (127, 1, "cg_hg011200"),
    (128, 2, "cg_hg020500"),
    (130, 1, "cg_hg011300"),
    (131, 1, "cg_hg011400"),
    (132, 2, "cg_hg020600"),
    (134, 8, "cg_hg020700"),
    (142, 7, "cg_hg020800"),
    (149, 2, "cg_hg011500"),
    (151, 1, "cg_hg011600"),
    (152, 6, "cg_hg010400"),
    (158, 13, "cg_hg010500"),
    (171, 7, "cg_hg010600"),
    (178, 8, "cg_hg010700"),
    (186, 12, "cg_hg010800"),
    (198, 19, "cg_hg010900_00"),
    (217, 10, "cg_hg011000"),
    (227, 16, "cg_hg011100"),
    (243, 16, "cg_hg011700"),
    (259, 9, "cg_hg011800"),
];

/// CG mode pictures, from the table at 0x573488: base picture and the
/// layers over it in slots 0.., as the scripts show them. The seen bit of
/// picture `i` is bit `i % 8` of byte `CG_SEEN + i / 8`.
pub const CG_IMAGES: [&[&str]; 268] = [
    &["hg020100_00_01"],
    &["hg020100_00"],
    &["hg020100_00", "hg020100_00_00_02"],
    &["hg020100_00", "hg020100_00_00_01"],
    &["hg020100_01_00"],
    &["hg020100_01_00", "hg020100_01_00_02"],
    &["hg020100_01_00", "hg020100_01_00_01"],
    &["hg020100_01_10"],
    &["hg020100_01_10", "hg020100_01_10_03"],
    &["hg020200_00", "hg020200_00_02", "hg020200_01"],
    &["hg020200_00", "hg020200_00_02"],
    &["hg020200_00", "hg020200_00_03"],
    &["hg020200_00", "hg020200_00_01"],
    &["hg020200_00", "hg020200_00_03", "hg020200_01"],
    &["hg020200_00"],
    &["hg020200_00", "hg020200_01"],
    &["hg020200_00", "hg020200_00_01", "hg020200_01"],
    &["hg020200_10"],
    &["hg020200_10", "hg020200_10_03"],
    &["hg020200_10", "hg020200_10_01"],
    &["hg020200_10", "hg020200_11"],
    &["hg020200_10", "hg020200_10_03", "hg020200_11"],
    &["hg020200_10", "hg020200_01"],
    &["hg020200_10", "hg020200_10_05"],
    &["hg020200_10", "hg020200_10_04"],
    &["hg010100"],
    &["hg010100_01_00"],
    &["hg010100_01_01"],
    &["hg010100_01_02"],
    &["hg010100_01"],
    &["hg010100_02"],
    &["hg010000_03"],
    &["hg020300"],
    &["hg020300", "hg020300_01"],
    &["hg020300", "hg020300_03"],
    &["hg020300_10"],
    &["hg020400"],
    &["hg020400_02"],
    &["hg020400_01"],
    &["hg020400_01_00"],
    &["hg020400_03"],
    &["hg020400_00"],
    &["hg020000"],
    &["hg020000", "hg020000_03"],
    &["hg020000", "hg020000_01"],
    &["hg020000", "hg020000_05"],
    &["hg020000", "hg020000_07"],
    &["hg020000", "hg020000_04"],
    &["hg020000", "hg020000_02"],
    &["hg020000", "hg020000_06"],
    &["hg030000_00", "hg030000_00_04"],
    &["hg030000_00"],
    &["hg030000_00", "hg030000_00_03"],
    &["hg030000_00", "hg030000_00_02"],
    &["hg030000_00", "hg030000_00_05"],
    &["hg030000_00", "hg030000_00_06"],
    &["hg030000_00", "hg030000_00_01"],
    &["hg030000_01", "hg030000_01_04"],
    &["hg030000_01"],
    &["hg030000_01", "hg030000_01_03"],
    &["hg030000_01", "hg030000_01_02"],
    &["hg030000_01", "hg030000_01_05"],
    &["hg030000_01", "hg030000_01_06"],
    &["hg030000_01", "hg030000_01_01"],
    &["hg030000_10", "hg030000_10_04"],
    &["hg030000_10"],
    &["hg030000_10", "hg030000_10_03"],
    &["hg030000_10", "hg030000_10_02"],
    &["hg030000_10", "hg030000_10_05"],
    &["hg030000_10", "hg030000_10_06"],
    &["hg030000_10", "hg030000_10_01"],
    &["hg030000_11", "hg030000_11_04"],
    &["hg030000_11"],
    &["hg030000_11", "hg030000_11_03"],
    &["hg030000_11", "hg030000_11_02"],
    &["hg030000_11", "hg030000_11_05"],
    &["hg030000_11", "hg030000_11_06"],
    &["hg030000_11", "hg030000_11_01"],
    &["hg030000_20", "hg030000_20_04"],
    &["hg030000_20"],
    &["hg030000_20", "hg030000_20_03"],
    &["hg030000_20", "hg030000_20_02"],
    &["hg030000_20", "hg030000_20_05"],
    &["hg030000_20", "hg030000_20_06"],
    &["hg030000_20", "hg030000_20_01"],
    &["hg030000_21", "hg030000_21_04"],
    &["hg030000_21"],
    &["hg030000_21", "hg030000_21_03"],
    &["hg030000_21", "hg030000_21_02"],
    &["hg030000_21", "hg030000_21_05"],
    &["hg030000_21", "hg030000_21_06"],
    &["hg030000_21", "hg030000_21_01"],
    &["hg030000_30", "hg030000_30_04"],
    &["hg030000_30"],
    &["hg030000_30", "hg030000_30_03"],
    &["hg030000_30", "hg030000_30_02"],
    &["hg030000_30", "hg030000_30_05"],
    &["hg030000_30", "hg030000_30_06"],
    &["hg030000_30", "hg030000_30_01"],
    &["hg030000_31", "hg030000_31_04"],
    &["hg030000_31"],
    &["hg030000_31", "hg030000_31_03"],
    &["hg030000_31", "hg030000_31_02"],
    &["hg030000_31", "hg030000_31_05"],
    &["hg030000_31", "hg030000_31_06"],
    &["hg030000_31", "hg030000_31_01"],
    &["hg010200_00"],
    &["hg010200_00", "hg010200_00_01"],
    &["hg010200_00", "hg010200_00_03"],
    &["hg010200_00", "hg010200_00_02"],
    &["hg010200_00", "hg010200_00_04"],
    &["hg010200_00", "hg010200_00_05"],
    &["hg010200_00", "hg010200_01"],
    &["hg010200_00", "hg010200_01", "hg010200_01_02"],
    &["hg010200_00", "hg010200_01", "hg010200_01_05"],
    &["hg010200_00", "hg010200_01", "hg010200_01_04"],
    &["hg010200_00", "hg010200_02", "hg010200_01_05"],
    &["hg010200_00", "hg010200_02", "hg010200_01_02"],
    &["hg010300"],
    &["hg010300", "hg010300_10", "hg010300_01"],
    &["hg010300", "hg010300_10", "hg010300_02"],
    &["hg010300", "hg010300_10", "hg010300_03"],
    &["hg010300", "hg010300_10"],
    &["hg010300", "hg010300_20", "hg010300_02"],
    &["hg010300", "hg010300_20", "hg010300_03"],
    &["hg010300", "hg010300_30", "hg010300_03"],
    &["hg010300", "hg010300_30", "hg010300_01"],
    &["hg011200"],
    &["hg020500"],
    &["hg020500_01"],
    &["hg011300"],
    &["hg011400"],
    &["hg020600"],
    &["hg020600_01"],
    &["hg020700"],
    &["hg020700_02"],
    &["hg020700_01"],
    &["hg020700_00_00"],
    &["hg020700_00_01"],
    &["hg020700_00_02"],
    &["hg020700_00_03"],
    &["hg020700_00"],
    &["hg020800", "hg020800_00_04"],
    &["hg020800"],
    &["hg020800", "hg020800_00_20"],
    &["hg020800", "hg020800_00_02", "hg020800_00_20"],
    &["hg020800", "hg020800_00_30"],
    &["hg020800", "hg020800_00_03", "hg020800_00_30"],
    &["hg020800_01"],
    &["hg011500"],
    &["hg011500", "hg011500_01"],
    &["hg011600"],
    &["hg010400"],
    &["hg010400", "hg010400_01"],
    &["hg010400", "hg010400_03"],
    &["hg010400", "hg010400_02"],
    &["hg010400", "hg010400_10", "hg010400_03"],
    &["hg010400", "hg010400_10", "hg010400_02"],
    &["hg010500"],
    &["hg010500", "hg010500_01"],
    &["hg010500", "hg010500_03"],
    &["hg010500", "hg010500_04"],
    &["hg010500", "hg010500_10", "hg010500_04"],
    &["hg010500", "hg010500_10"],
    &["hg010500", "hg010500_10", "hg010500_01"],
    &["hg010500", "hg010500_10", "hg010500_03"],
    &["hg010500", "hg010500_10", "hg010500_02"],
    &["hg010500", "hg010500_10", "hg010500_05"],
    &["hg010500", "hg010500_20", "hg010500_05"],
    &["hg010500", "hg010500_20", "hg010500_06"],
    &["hg010500", "hg010500_20"],
    &["hg010600"],
    &["hg010600", "hg010600_03"],
    &["hg010600", "hg010600_04"],
    &["hg010600", "hg010600_01"],
    &["hg010600", "hg010600_02"],
    &["hg010600", "hg010600_10", "hg010600_02"],
    &["hg010600", "hg010600_10", "hg010600_01"],
    &["hg010700"],
    &["hg010700", "hg010700_01"],
    &["hg010700", "hg010700_05"],
    &["hg010700", "hg010700_02"],
    &["hg010700", "hg010700_03"],
    &["hg010700", "hg010700_04"],
    &["hg010700", "hg010700_10", "hg010700_04"],
    &["hg010700", "hg010700_10", "hg010700_06"],
    &["hg010800"],
    &["hg010800", "hg010800_00_02"],
    &["hg010800", "hg010800_00_03"],
    &["hg010800", "hg010800_00_04"],
    &["hg010800", "hg010800_00_01"],
    &["hg010800", "hg010800_01_01"],
    &["hg010800", "hg010800_01_04"],
    &["hg010800", "hg010800_01_00"],
    &["hg010800", "hg010800_01_03"],
    &["hg010800", "hg010800_01_02"],
    &["hg010800", "hg010800_02", "hg010800_02_04"],
    &["hg010800", "hg010800_02", "hg010800_02_02"],
    &["hg010900_00", "hg010900_00_03"],
    &["hg010900_00"],
    &["hg010900_00", "hg010900_00_01"],
    &["hg010900_00", "hg010900_00_02"],
    &["hg010900_00", "hg010900_01", "hg010900_00_02"],
    &["hg010900_00", "hg010900_01", "hg010900_00_03"],
    &["hg010900_00", "hg010900_01"],
    &["hg010900_00", "hg010900_01", "hg010900_00_01"],
    &["hg010900_00", "hg010900_01", "hg010900_00_04"],
    &["hg010900_00", "hg010900_01", "hg010900_00_05"],
    &["hg010900_00", "hg010900_01", "hg010900_00_06"],
    &["hg010900_00", "hg010900_02", "hg010900_00_05"],
    &["hg010900_00", "hg010900_02", "hg010900_00_06"],
    &["hg010900_00", "hg010900_02", "hg010900_00_02"],
    &["hg010900_00", "hg010900_02", "hg010900_00_03"],
    &["hg010900_00", "hg010900_02", "hg010900_00_04"],
    &["hg010900_00", "hg010900_03", "hg010900_00_06"],
    &["hg010900_00", "hg010900_03", "hg010900_00_02"],
    &["hg010900_00", "hg010900_03", "hg010900_00_03"],
    &["hg011000"],
    &["hg011000", "hg011000_02"],
    &["hg011000", "hg011000_01"],
    &["hg011000", "hg011000_10", "hg011000_02"],
    &["hg011000", "hg011000_10", "hg011000_01"],
    &["hg011000", "hg011000_10"],
    &["hg011000", "hg011000_10", "hg011000_03"],
    &["hg011000", "hg011000_10", "hg011000_20", "hg011000_03"],
    &["hg011000", "hg011000_10", "hg011000_20", "hg011000_02"],
    &["hg011000", "hg011000_10", "hg011000_20", "hg011000_01"],
    &["hg011100"],
    &["hg011100", "hg011100_02"],
    &["hg011100", "hg011100_03"],
    &["hg011100", "hg011100_01"],
    &["hg011100", "hg011100_20", "hg011100_01"],
    &["hg011100", "hg011100_20"],
    &["hg011100", "hg011100_20", "hg011100_02"],
    &["hg011100", "hg011100_20", "hg011100_03"],
    &["hg011100", "hg011100_40", "hg011100_03"],
    &["hg011100", "hg011100_40"],
    &["hg011100", "hg011100_40", "hg011100_02"],
    &["hg011100", "hg011100_40", "hg011100_01"],
    &["hg011100", "hg011100_30", "hg011100_03"],
    &["hg011100", "hg011100_30", "hg011100_01"],
    &["hg011100", "hg011100_30", "hg011100_02"],
    &["hg011100", "hg011100_30"],
    &["hg011700"],
    &["hg011700", "hg011700_01"],
    &["hg011700", "hg011700_02"],
    &["hg011700", "hg011700_03"],
    &["hg011700", "hg011700_04"],
    &["hg011700", "hg011700_05"],
    &["hg011700", "hg011700_06"],
    &["hg011700", "hg011700_10"],
    &["hg011700", "hg011700_10", "hg011700_01"],
    &["hg011700", "hg011700_20", "hg011700_04"],
    &["hg011700", "hg011700_20", "hg011700_03"],
    &["hg011700", "hg011700_20", "hg011700_01"],
    &["hg011700", "hg011700_20", "hg011700_05"],
    &["hg011700", "hg011700_20", "hg011700_30", "hg011700_05"],
    &["hg011700", "hg011700_20", "hg011700_30", "hg011700_06"],
    &["hg011700", "hg011700_10", "hg011700_30", "hg011700_06"],
    &["hg011800"],
    &["hg011800", "hg011800_01"],
    &["hg011800", "hg011800_02"],
    &["hg011800", "hg011800_10", "hg011800_03"],
    &["hg011800", "hg011800_10", "hg011800_02"],
    &["hg011800", "hg011800_20", "hg011800_03"],
    &["hg011800", "hg011800_20", "hg011800_02"],
    &["hg011800", "hg011800_30", "hg011800_03"],
    &["hg011800", "hg011800_30", "hg011800_04"],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_line_up() {
        let mut next = 0;
        for &(first, count, _) in &CG_ENTRIES {
            assert_eq!(first, next);
            next += count;
        }
        assert_eq!(next, CG_IMAGES.len());
    }

    #[test]
    fn registers_shown_pictures() {
        let mut bytes = vec![0u8; 4096];
        assert_eq!(
            register_cg(
                &mut bytes,
                "HG020100_00.akb",
                ["HG020100_00_00_02.akb"].into_iter()
            ),
            Some(2)
        );
        assert!(cg_seen(&bytes, 2));
        assert_eq!(
            register_cg(
                &mut bytes,
                "HG020100_00.akb",
                ["HG020100_00_00_02.akb"].into_iter()
            ),
            None
        );
        assert_eq!(
            register_cg(&mut bytes, "bg010000.akb", std::iter::empty()),
            None
        );
        assert!(open(Extras::Gallery, &bytes, 0));
        for (i, v) in unlock_all(Extras::Gallery) {
            bytes[i] = v;
        }
        assert!((0..CG_IMAGES.len()).all(|p| cg_seen(&bytes, p)));
        assert!(!cg_seen(&bytes, CG_IMAGES.len()));
    }
}
