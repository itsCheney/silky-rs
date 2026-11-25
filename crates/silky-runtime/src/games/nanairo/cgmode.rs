//! The game's own CG mode (`Game.30`): a grid of 15 pictures on each of six
//! pages, a picker for the variants of one picture, and the frame the three
//! mode screens share (see [`super::mode`]).
//!
//! The script draws the background before it calls this; the screen draws
//! the frame and its own parts over it. A slot shows the first variant that
//! was seen (a closed slot shows the logo card), and a counter of the
//! variants seen over the variants there are when there is more than one. A
//! press on an open slot darkens the screen and lays out the variants of the
//! picture in a block in the middle; a press on a seen variant ends the
//! screen with the picture, and the script then shows that variant and the
//! ones after it by itself. The answer is the picture's index; the variant
//! is left in script variable 17.

use std::sync::Arc;

use silky_format::akb::Image;

use super::cg::ROWS;
use super::keys::{self, Arrow};
use super::mode::{
    self, Frame, Mode, Phase, BACK, FRAME_MS, QUIT, SLIDE_IN_FRAMES, SLIDE_OUT_FRAMES,
};
use crate::assets::Assets;
use crate::games::kimaten1::{Click, Look};
use crate::render::Canvas;

pub const SHEET: &str = "CGmode.akb";
const OK_SOUND: &str = "ses_001.wav";
const CANCEL_SOUND: &str = "ses_002.wav";

/// The pictures: how many variants each has and the sheet of their
/// thumbnails. The variants of a picture are consecutive rows of the CG
/// table, the pictures in table order.
pub const ITEMS: [(usize, &str); 86] = [
    (3, "cg_HG090001.akb"),
    (1, "cg_HG070000.akb"),
    (4, "cg_HG080001_h0.akb"),
    (1, "cg_HG010000.akb"),
    (12, "cg_HG080101_h0.akb"),
    (8, "cg_HG080201_h0.akb"),
    (5, "cg_HG080301_h0.akb"),
    (4, "cg_HG040001_h0.akb"),
    (5, "cg_HG010105.akb"),
    (3, "cg_HG010200_h0.akb"),
    (8, "cg_HG090101_h0.akb"),
    (10, "cg_HG080401_h0.akb"),
    (2, "cg_HG010301.akb"),
    (3, "cg_HG020001.akb"),
    (12, "cg_HG040101_h0.akb"),
    (5, "cg_HG040201_h0.akb"),
    (2, "cg_HG050000_h0.akb"),
    (6, "cg_HG030001_h0.akb"),
    (7, "cg_HG090201.akb"),
    (9, "cg_HG050101_h0.akb"),
    (11, "cg_HG050201_h0.akb"),
    (6, "cg_HG060003.akb"),
    (6, "cg_HG020101.akb"),
    (3, "cg_HG090300.akb"),
    (5, "cg_HG010500_h0.akb"),
    (12, "cg_HG030101_h0.akb"),
    (5, "cg_HG030201_h0.akb"),
    (4, "cg_HG070101_h0.akb"),
    (7, "cg_HG070201_h0.akb"),
    (10, "cg_HG090401_h0.akb"),
    (21, "cg_HG090501_h0.akb"),
    (8, "cg_HG060101_h0.akb"),
    (31, "cg_HG020201_h0.akb"),
    (6, "cg_HG060201_h0.akb"),
    (9, "cg_HG060301_h0.akb"),
    (15, "cg_HG050301_h0.akb"),
    (9, "cg_HG020301_h0.akb"),
    (5, "cg_HG030301_h0.akb"),
    (15, "cg_HG030401_h0.akb"),
    (9, "cg_HG020401_h0.akb"),
    (10, "cg_HG040301_h0.akb"),
    (14, "cg_HG020500_h0.akb"),
    (4, "cg_HG010601_h0.akb"),
    (1, "cg_HG090601_h0.akb"),
    (20, "cg_HG010701_h0.akb"),
    (8, "cg_HG010801_h0.akb"),
    (15, "cg_HG090711_h0.akb"),
    (11, "cg_HG091201_h0.akb"),
    (2, "cg_HG090801.akb"),
    (3, "cg_HG090903.akb"),
    (9, "cg_HG010901_h0.akb"),
    (12, "cg_HG011001_h0.akb"),
    (9, "cg_HG091101.akb"),
    (9, "cg_HG011201_h0.akb"),
    (23, "cg_HG091001_h0.akb"),
    (23, "cg_HG070301_h0.akb"),
    (5, "cg_HG070401_h0.akb"),
    (10, "cg_HG070501.akb"),
    (1, "cg_HG070600.akb"),
    (1, "cg_HG070700.akb"),
    (14, "cg_HG011301_h0.akb"),
    (7, "cg_HG011401_h0.akb"),
    (4, "cg_HG011502_h0.akb"),
    (2, "cg_HG011601_h0.akb"),
    (14, "cg_HG011701_h0.akb"),
    (5, "cg_HG011801_h0.akb"),
    (3, "cg_HG011901.akb"),
    (7, "cg_HG012001_h0.akb"),
    (6, "cg_HG012101_h0.akb"),
    (1, "cg_HG012200.akb"),
    (7, "cg_HG020601_h0.akb"),
    (15, "cg_HG020701_h0.akb"),
    (20, "cg_HG020801_h0.akb"),
    (1, "cg_HG020900.akb"),
    (3, "cg_HG030501_h0.akb"),
    (12, "cg_HG030601_h0.akb"),
    (8, "cg_HG030701_h0.akb"),
    (5, "cg_HG030801_h0.akb"),
    (6, "cg_HG030901_h0.akb"),
    (1, "cg_HG031000.akb"),
    (1, "cg_MovieSkip.akb"),
    (1, "cg_staff01.akb"),
    (1, "cg_staff02.akb"),
    (1, "cg_staff03.akb"),
    (1, "cg_staff04.akb"),
    (1, "cg_staff05.akb"),
];
pub const COUNT: usize = ITEMS.len();
/// Pictures on a page and pages.
pub const PER_PAGE: usize = 15;
pub const PAGES: usize = 6;
/// Variants the picker shows on a page.
const VIEW_PER_PAGE: usize = 15;

/// Area ids besides the slots (0..15): the page arrows.
const PREV: i32 = 0x100;
const NEXT: i32 = 0x101;
/// A press beside the cells of the picker.
const OUTSIDE: i32 = 0x200;

/// The row of the CG table that holds variant `v` of `item`.
pub fn row_of(item: usize, v: usize) -> usize {
    ITEMS[..item].iter().map(|i| i.0).sum::<usize>() + v
}

/// Top left of the frame of slot `s` on the grid.
pub fn slot_pos(s: usize) -> (i32, i32) {
    (26 + 250 * (s % 5) as i32, 138 + 156 * (s / 5) as i32)
}

/// Pictures on grid page `p`.
fn on_page(p: usize) -> usize {
    COUNT.saturating_sub(p * PER_PAGE).min(PER_PAGE)
}

/// Top left of the picker's block for `np` cells on a page.
fn view_origin(np: usize) -> (i32, i32) {
    let w = if np < 5 { np * 248 } else { 1240 } as i32;
    let h = if np <= 5 { 155 } else { np.div_ceil(5) * 155 } as i32;
    ((1280 - w) / 2, (690 - h) / 2 + 14)
}

fn view_cell(np: usize, k: usize) -> (i32, i32) {
    let (x0, y0) = view_origin(np);
    (x0 + 248 * (k % 5) as i32, y0 + 155 * (k / 5) as i32)
}

/// The areas (left, top, right, bottom) of a slot or button, as
/// `SCMODE.AREA` has them.
fn rect(id: i32) -> Option<(i32, i32, i32, i32)> {
    Some(match id {
        0..=14 => {
            let (x, y) = slot_pos(id as usize);
            (x + 2, y + 2, x + 227, y + 129)
        }
        PREV => (551, 609, 583, 641),
        NEXT => (697, 609, 729, 641),
        0xf00 => (25, 6, 118, 117),
        0xf01 => (166, 6, 259, 117),
        0xf02 => (305, 6, 398, 117),
        BACK => (1077, 605, 1170, 716),
        QUIT => (1178, 605, 1271, 716),
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    No,
    /// Asked to leave; the slide out starts when the clock is next seen.
    Pending(i32),
    At(u64, i32),
}

/// The picker of one picture's variants.
#[derive(Debug, Clone, PartialEq, Eq)]
struct View {
    item: usize,
    /// Page of variants, 15 to a page.
    page: usize,
    /// When it opened, once the clock has been seen.
    since: Option<u64>,
}

/// The picker darkens the screen over 10 frames, to 75 per cent.
const VIEW_FADE_FRAMES: u64 = 10;
const VIEW_SHADE: u32 = 0xc0;

#[derive(Debug, Clone)]
pub struct CgMode {
    /// Whether each row of the CG table has been seen.
    seen: Vec<bool>,
    /// How many rows have a flag that is not 0 (the completion counts
    /// those, a row is shown only when its flag is 1).
    set: usize,
    /// The tab bytes in the order of the strip (CG, music, scene).
    tabs: [u8; 3],
    page: usize,
    view: Option<View>,
    leave: Leave,
    sound: Option<&'static str>,
    globals: Vec<(usize, i32)>,
}

impl CgMode {
    /// `flags` are the bytes of the CG table's rows (1 when seen), `tabs`
    /// the bytes 0xbba..0xbbc in the order of the strip. With `first` the
    /// screen opens on that picture's page with its picker up.
    pub fn open(flags: &[u8], tabs: [u8; 3], first: Option<usize>) -> Self {
        let mut s = CgMode {
            set: flags.iter().take(ROWS.len()).filter(|&&f| f != 0).count(),
            seen: (0..ROWS.len())
                .map(|r| flags.get(r).copied() == Some(1))
                .collect(),
            tabs,
            page: 0,
            view: None,
            leave: Leave::No,
            sound: None,
            globals: Vec::new(),
        };
        if let Some(item) = first.filter(|&i| i < COUNT) {
            s.page = item / PER_PAGE;
            if s.item_open(item) {
                s.view = Some(View {
                    item,
                    page: 0,
                    since: Some(0),
                });
            }
        }
        s
    }

    fn variants(&self, item: usize) -> usize {
        ITEMS[item].0
    }

    fn variant_seen(&self, item: usize, v: usize) -> bool {
        self.seen.get(row_of(item, v)).copied().unwrap_or(false)
    }

    /// How many variants of a picture were seen.
    pub fn seen_count(&self, item: usize) -> usize {
        (0..self.variants(item))
            .filter(|&v| self.variant_seen(item, v))
            .count()
    }

    /// The first variant seen, if any.
    fn first_seen(&self, item: usize) -> Option<usize> {
        (0..self.variants(item)).find(|&v| self.variant_seen(item, v))
    }

    fn item_open(&self, item: usize) -> bool {
        self.first_seen(item).is_some()
    }

    /// Share of the table's rows seen, as the counter shows it.
    pub fn percent(&self) -> u32 {
        mode::percent(self.set, ROWS.len())
    }

    fn tab_open(&self, i: usize) -> bool {
        self.tabs[i] != 0
    }

    fn view_pages(&self, item: usize) -> usize {
        self.variants(item).div_ceil(VIEW_PER_PAGE)
    }

    fn view_cells(&self, v: &View) -> usize {
        (self.variants(v.item) - v.page * VIEW_PER_PAGE).min(VIEW_PER_PAGE)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let inside =
            |id: i32| rect(id).is_some_and(|(l, t, r, b)| x >= l && x < r && y >= t && y < b);
        if let Some(v) = &self.view {
            let np = self.view_cells(v);
            let cell = (0..np).find(|&k| {
                let (cx, cy) = view_cell(np, k);
                x >= cx + 2 && x < cx + 227 && y >= cy + 2 && y < cy + 129
            });
            if let Some(k) = cell {
                return Some(k as i32);
            }
            if self.view_pages(v.item) > 1 {
                if let Some(id) = [PREV, NEXT].into_iter().find(|&id| inside(id)) {
                    return Some(id);
                }
            }
            return Some(OUTSIDE);
        }
        (0..on_page(self.page) as i32)
            .chain([PREV, NEXT, 0xf00, 0xf01, 0xf02, BACK, QUIT])
            .find(|&id| inside(id))
    }

    /// Where an arrow key puts the pointer: the button it is on is `at`.
    pub fn arrow(&self, at: Option<i32>, key: Arrow) -> Option<i32> {
        if self.leave != Leave::No {
            return None;
        }
        if let Some(v) = &self.view {
            let np = self.view_cells(v);
            let to = keys::picker(at, key, &|k| k < np)?;
            // The arrows of the picker are there only with several pages.
            return (to < PREV || to == OUTSIDE || self.view_pages(v.item) > 1).then_some(to);
        }
        keys::grid(at, key, self.page, &|k| k < on_page(self.page))
    }

    /// The middle of a button, where the pointer goes.
    pub fn centre(&self, id: i32) -> Option<(i32, i32)> {
        if let Some(v) = &self.view {
            let np = self.view_cells(v);
            if id == OUTSIDE {
                return Some((640, 360));
            }
            if (0..np as i32).contains(&id) {
                let (x, y) = view_cell(np, id as usize);
                return Some(keys::middle((x + 2, y + 2, x + 227, y + 129)));
            }
        }
        rect(id).map(keys::middle)
    }

    pub fn enabled(&self, id: i32) -> bool {
        if let Some(v) = &self.view {
            let np = self.view_cells(v);
            return match id {
                0..=14 => {
                    (id as usize) < np
                        && self.variant_seen(v.item, v.page * VIEW_PER_PAGE + id as usize)
                }
                PREV => v.page > 0,
                NEXT => v.page + 1 < self.view_pages(v.item),
                _ => true,
            };
        }
        match id {
            0..=14 => {
                (id as usize) < on_page(self.page)
                    && self.item_open(self.page * PER_PAGE + id as usize)
            }
            PREV => self.page > 0,
            NEXT => self.page + 1 < PAGES,
            0xf00..=0xf02 => {
                let i = (id - mode::TAB_FIRST) as usize;
                i != Mode::Cg.tab() && self.tab_open(i)
            }
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.leave != Leave::No {
            return Click::None;
        }
        if self.view.is_some() {
            self.click_view(id);
            return Click::None;
        }
        match id {
            0..=14 => {
                if self.enabled(id) {
                    self.sound = Some(OK_SOUND);
                    self.view = Some(View {
                        item: self.page * PER_PAGE + id as usize,
                        page: 0,
                        since: None,
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
            0xf01 | 0xf02 => {
                let i = (id - mode::TAB_FIRST) as usize;
                if self.tab_open(i) {
                    self.sound = Some(OK_SOUND);
                    let to = if i == Mode::Music.tab() {
                        Mode::Music
                    } else {
                        Mode::Scene
                    };
                    // The next screen starts from the top: the viewing flag
                    // and the variant are cleared.
                    self.globals = vec![(16, 0), (17, 0)];
                    self.leave = Leave::Pending(to.code());
                }
            }
            BACK => {
                self.sound = Some(OK_SOUND);
                self.leave = Leave::Pending(mode::LEAVE);
            }
            QUIT => {
                self.sound = Some(OK_SOUND);
                return Click::Bar(crate::ui::bar::QUIT);
            }
            _ => {}
        }
        Click::None
    }

    fn click_view(&mut self, id: i32) {
        let Some(v) = self.view.clone() else { return };
        let np = self.view_cells(&v);
        match id {
            0..=14 if (id as usize) < np => {
                let var = v.page * VIEW_PER_PAGE + id as usize;
                if self.variant_seen(v.item, var) {
                    self.sound = Some(OK_SOUND);
                    // The script shows the picture from this variant on.
                    self.globals = vec![(8, 0), (16, 0), (17, var as i32)];
                    self.leave = Leave::Pending(v.item as i32);
                }
            }
            PREV if v.page > 0 => {
                self.sound = Some(OK_SOUND);
                self.view = Some(View {
                    page: v.page - 1,
                    ..v
                });
            }
            NEXT if v.page + 1 < self.view_pages(v.item) => {
                self.sound = Some(OK_SOUND);
                self.view = Some(View {
                    page: v.page + 1,
                    ..v
                });
            }
            OUTSIDE => {
                self.sound = Some(OK_SOUND);
                self.view = None;
            }
            _ => {}
        }
    }

    /// A right click or Escape: the picker closes without a sound; on the
    /// grid it leaves with the cancel sound.
    pub fn cancel(&mut self) -> Click {
        if self.leave != Leave::No {
            return Click::None;
        }
        if self.view.take().is_none() {
            self.sound = Some(CANCEL_SOUND);
            self.leave = Leave::Pending(mode::LEAVE);
        }
        Click::None
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    /// Script variables (`g` index, value) the screen set, taken once.
    pub fn take_globals(&mut self) -> Vec<(usize, i32)> {
        std::mem::take(&mut self.globals)
    }

    /// Headless run: the first thing to press.
    pub fn auto_pick(&self) -> Option<i32> {
        Some(BACK)
    }

    /// Whether the frame is sliding or the picker is fading at `ms` since
    /// the screen opened.
    pub fn animating(&self, ms: u64) -> bool {
        self.leave != Leave::No
            || ms < SLIDE_IN_FRAMES * FRAME_MS
            || self
                .view
                .as_ref()
                .is_some_and(|v| v.since.is_none_or(|t| ms < t + VIEW_FADE_FRAMES * FRAME_MS))
    }

    /// Runs the slide out; the answer comes when it is over.
    pub fn tick(&mut self, ms: u64) -> Option<Click> {
        if let Some(v) = &mut self.view {
            v.since.get_or_insert(ms);
        }
        match self.leave {
            Leave::No => None,
            Leave::Pending(a) => {
                self.leave = Leave::At(ms, a);
                None
            }
            Leave::At(t0, a) => {
                (ms >= t0 + SLIDE_OUT_FRAMES * FRAME_MS).then_some(Click::Result(a))
            }
        }
    }

    /// A run without time: the answer the slide out holds back, now.
    pub fn settle(&mut self) -> Option<Click> {
        match self.leave {
            Leave::No => None,
            Leave::Pending(a) | Leave::At(_, a) => Some(Click::Result(a)),
        }
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

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> crate::Result<()> {
        let sheet = assets.image(SHEET)?;
        let phase = match self.leave {
            Leave::At(t0, _) => Phase::at(look.ms, Some(t0), true),
            _ => Phase::at(look.ms, None, true),
        };
        let alpha = phase.page_alpha(look.ms);
        // While the picker is up nothing under it answers the pointer.
        let under = self.view.is_none();
        Frame {
            sheet: &sheet,
            mode: Mode::Cg,
            open: [true, self.tab_open(1), self.tab_open(2)],
            hover: look.hover.filter(|_| under),
            pressed: look.pressed.filter(|_| under),
            phase,
        }
        .draw(canvas);
        self.draw_page(canvas, assets, &sheet, look, alpha)?;
        if let Some(v) = &self.view {
            self.draw_view(canvas, assets, &sheet, v, look, alpha)?;
        }
        Ok(())
    }

    fn draw_page(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        look: &Look,
        alpha: u8,
    ) -> crate::Result<()> {
        let thumbs = assets.image(&format!("cgmode_thm{:02}.akb", self.page))?;
        let under = self.view.is_none();
        for s in 0..on_page(self.page) {
            let item = self.page * PER_PAGE + s;
            let (x, y) = slot_pos(s);
            canvas.draw_part(sheet, (465, 301, 228, 131), x, y, false, alpha);
            let Some(first) = self.first_seen(item) else {
                continue;
            };
            if first == 0 {
                let (c, r) = ((s % 5) as u32, (s / 5) as u32);
                let part = (c * 222, r * 125, 222, 125);
                canvas.draw_part(&thumbs, part, x + 3, y + 3, false, alpha);
            } else {
                let own = assets.image(ITEMS[item].1)?;
                let part = ((first % 5) as u32 * 222, (first / 5) as u32 * 125, 222, 125);
                canvas.draw_part(&own, part, x + 3, y + 3, false, alpha);
            }
            let n = self.variants(item);
            if n > 1 {
                draw_counter(canvas, sheet, (x, y), self.seen_count(item), n, alpha);
            }
        }
        // The pointer's slot gets the frame that lights it (open slots only).
        for s in 0..on_page(self.page) {
            let id = s as i32;
            if !under || !self.enabled(id) {
                continue;
            }
            let src = if look.pressed == Some(id) {
                921
            } else if look.hover == Some(id) {
                693
            } else {
                continue;
            };
            let (x, y) = slot_pos(s);
            canvas.draw_part(sheet, (src, 301, 228, 131), x, y, false, alpha);
        }
        self.draw_pages(canvas, sheet, look, (self.page, PAGES), under, alpha);
        mode::draw_percent(canvas, sheet, self.percent(), alpha);
        Ok(())
    }

    /// The page arrows and the page number (current and total).
    fn draw_pages(
        &self,
        canvas: &mut Canvas,
        sheet: &Arc<Image>,
        look: &Look,
        (page, total): (usize, usize),
        live: bool,
        alpha: u8,
    ) {
        for (id, sx) in [(PREV, 465), (NEXT, 497)] {
            let st = if live {
                self.state(look, id)
            } else if self.enabled(id) {
                0
            } else {
                3
            };
            let (l, t, _, _) = rect(id).unwrap();
            canvas.draw_part(sheet, (sx, 173 + 32 * st, 32, 32), l, t, false, alpha);
        }
        let digit = |n: usize| (826 + 32 * n as u32, 135, 32, 36);
        canvas.draw_part(sheet, digit(page + 1), 596, 607, false, alpha);
        canvas.draw_part(sheet, (903, 91, 23, 41), 629, 604, false, alpha);
        canvas.draw_part(sheet, digit(total), 652, 607, false, alpha);
    }

    fn draw_view(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        v: &View,
        look: &Look,
        alpha: u8,
    ) -> crate::Result<()> {
        let t = look.ms.saturating_sub(v.since.unwrap_or(look.ms)) / FRAME_MS;
        let fade = (t.min(VIEW_FADE_FRAMES) * 255 / VIEW_FADE_FRAMES) as u32;
        let shade = (VIEW_SHADE * fade / 255 * alpha as u32 / 255) as u8;
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, shade]);
        let a = (fade * alpha as u32 / 255) as u8;
        let np = self.view_cells(v);
        let own = assets.image(ITEMS[v.item].1)?;
        for k in 0..np {
            let var = v.page * VIEW_PER_PAGE + k;
            let (x, y) = view_cell(np, k);
            canvas.draw_part(sheet, (465, 301, 228, 131), x, y, false, a);
            if !self.variant_seen(v.item, var) {
                continue;
            }
            let part = ((var % 5) as u32 * 222, (var / 5) as u32 * 125, 222, 125);
            canvas.draw_part(&own, part, x + 3, y + 3, false, a);
            let src = if look.pressed == Some(k as i32) {
                921
            } else if look.hover == Some(k as i32) {
                693
            } else {
                continue;
            };
            canvas.draw_part(sheet, (src, 301, 228, 131), x, y, false, a);
        }
        let pages = self.view_pages(v.item);
        if pages > 1 {
            self.draw_pages(canvas, sheet, look, (v.page, pages), true, a);
        }
        Ok(())
    }
}

/// The counter on a slot: variants seen over variants there are, in the
/// small digits of the sheet.
fn draw_counter(
    canvas: &mut Canvas,
    sheet: &Arc<Image>,
    (x, y): (i32, i32),
    seen: usize,
    total: usize,
    alpha: u8,
) {
    let digit = |canvas: &mut Canvas, d: usize, dx: i32| {
        let part = (483 + 25 * d as u32, 141, 25, 27);
        canvas.draw_part(sheet, part, x + dx, y + 0x61, false, alpha);
    };
    canvas.draw_part(sheet, (465, 141, 18, 32), x + 0x9e, y + 0x5e, false, alpha);
    if seen > 9 {
        digit(canvas, seen / 10 % 10, 0x6b);
    }
    digit(canvas, seen % 10, 0x84);
    if total > 9 {
        digit(canvas, total / 10 % 10, 0xaf);
    }
    digit(canvas, total % 10, 0xc8);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flags with every row of the given pictures seen.
    fn flags(items: &[usize]) -> Vec<u8> {
        let mut f = vec![0u8; ROWS.len()];
        for &i in items {
            for v in 0..ITEMS[i].0 {
                f[row_of(i, v)] = 1;
            }
        }
        f
    }

    fn open(items: &[usize]) -> CgMode {
        CgMode::open(&flags(items), [1; 3], None)
    }

    #[test]
    fn table_follows_the_game() {
        assert_eq!(ITEMS.iter().map(|i| i.0).sum::<usize>(), ROWS.len());
        assert_eq!(COUNT, 86);
        // The first row of each picture holds the flag the game's own table
        // of pictures names.
        assert_eq!(ROWS[row_of(0, 0)].flag, 3200);
        assert_eq!(ROWS[row_of(1, 0)].flag, 3203);
        assert_eq!(ROWS[row_of(2, 0)].flag, 3204);
        assert_eq!(ROWS[row_of(85, 0)].flag, 3845);
        assert_eq!(row_of(85, 0), 643);
        assert_eq!(ITEMS[32], (31, "cg_HG020201_h0.akb"));
        assert_eq!(on_page(0), 15);
        assert_eq!(on_page(5), 11);
        assert_eq!(PAGES * PER_PAGE - 4, COUNT);
    }

    #[test]
    fn slots_and_areas() {
        assert_eq!(slot_pos(0), (26, 138));
        assert_eq!(slot_pos(14), (1026, 450));
        assert_eq!(rect(0), Some((28, 140, 253, 267)));
        assert_eq!(rect(14), Some((1028, 452, 1253, 579)));
        let m = open(&[0, 1]);
        assert_eq!(m.hit(30, 142), Some(0));
        assert_eq!(m.hit(300, 142), Some(1));
        assert_eq!(m.hit(260, 142), None);
        assert_eq!(m.hit(560, 620), Some(PREV));
        assert_eq!(m.hit(700, 620), Some(NEXT));
        assert_eq!(m.hit(30, 10), Some(0xf00));
        assert_eq!(m.hit(1100, 650), Some(BACK));
        assert_eq!(m.hit(1200, 650), Some(QUIT));
    }

    #[test]
    fn counter_and_first_variant() {
        let mut f = vec![0u8; ROWS.len()];
        f[row_of(4, 3)] = 1;
        f[row_of(4, 7)] = 1;
        let m = CgMode::open(&f, [1; 3], None);
        assert_eq!(m.first_seen(4), Some(3));
        assert_eq!(m.seen_count(4), 2);
        assert!(!m.item_open(3));
        assert_eq!(m.percent(), 1);
        assert_eq!(open(&[]).percent(), 1);
        // A fresh game reads 1 (the counter is never under 1); 7 rows of
        // 644 are the first to read 1 by their share; any flag that is not
        // 0 counts.
        let mut f = vec![0u8; ROWS.len()];
        for r in 0..6 {
            f[r] = 1;
        }
        assert_eq!(CgMode::open(&f, [1; 3], None).percent(), 1);
        f[6] = 2;
        assert_eq!(CgMode::open(&f, [1; 3], None).percent(), 1);
        for r in 0..13 {
            f[r] = 1;
        }
        assert_eq!(CgMode::open(&f, [1; 3], None).percent(), 2);
        let all: Vec<usize> = (0..COUNT).collect();
        assert_eq!(open(&all).percent(), 100);
    }

    #[test]
    fn closed_slots_do_nothing() {
        let mut m = open(&[1]);
        assert!(!m.enabled(0) && m.enabled(1));
        m.click(0);
        assert!(m.view.is_none());
        assert_eq!(m.take_sound(), None);
        m.click(1);
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        assert_eq!(m.view.as_ref().map(|v| v.item), Some(1));
    }

    #[test]
    fn pages_turn() {
        let mut m = open(&[]);
        assert!(!m.enabled(PREV) && m.enabled(NEXT));
        m.click(PREV);
        assert_eq!((m.page, m.take_sound()), (0, None));
        for p in 1..PAGES {
            m.click(NEXT);
            assert_eq!(m.page, p);
        }
        assert!(!m.enabled(NEXT));
        m.click(NEXT);
        assert_eq!(m.page, 5);
        // The last page has 11 slots.
        assert_eq!(m.hit(300, 460), None);
        assert_eq!(m.hit(30, 460), Some(10));
        m.click(PREV);
        assert_eq!(m.page, 4);
    }

    #[test]
    fn picker_layout() {
        // Fewer than five cells are centred in steps of 248; five or more
        // fill 1240.
        assert_eq!(view_origin(3), ((1280 - 744) / 2, (690 - 155) / 2 + 14));
        assert_eq!(view_origin(5).0, 20);
        assert_eq!(view_origin(15), (20, (690 - 465) / 2 + 14));
        assert_eq!(view_cell(15, 7), (20 + 2 * 248, (690 - 465) / 2 + 14 + 155));
        assert_eq!(view_origin(6).1, (690 - 310) / 2 + 14);
    }

    #[test]
    fn picking_a_variant_answers_the_picture() {
        // Picture 4 has 12 variants; seen are 2 and 5.
        let mut f = vec![0u8; ROWS.len()];
        f[row_of(4, 2)] = 1;
        f[row_of(4, 5)] = 1;
        let mut m = CgMode::open(&f, [1; 3], None);
        m.click(4);
        m.take_sound();
        assert!(m.animating(0));
        // The picker's cells: only seen ones answer.
        let np = 12;
        let (x, y) = view_cell(np, 5);
        assert_eq!(m.hit(x + 10, y + 10), Some(5));
        assert!(m.enabled(5) && !m.enabled(4));
        m.click(4);
        assert_eq!(m.take_sound(), None);
        assert_eq!(m.settle(), None);
        m.click(5);
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        assert_eq!(m.take_globals(), vec![(8, 0), (16, 0), (17, 5)]);
        assert_eq!(m.tick(100), None);
        assert_eq!(m.tick(100 + 12 * 16), Some(Click::Result(4)));
        assert_eq!(mode::next(Mode::Cg, 4), mode::Next::Leave(4));
    }

    #[test]
    fn variant_pages() {
        // Picture 32 has 31 variants: three pages: 15, 15 and 1.
        let mut m = CgMode::open(&flags(&[32]), [1; 3], Some(32));
        assert_eq!(m.page, 2);
        assert_eq!(m.view_pages(32), 3);
        assert!(m.view.is_some());
        assert!(!m.enabled(PREV) && m.enabled(NEXT));
        assert_eq!(m.hit(560, 620), Some(PREV));
        m.click(NEXT);
        m.click(NEXT);
        assert_eq!(m.view.as_ref().unwrap().page, 2);
        assert_eq!(m.view_cells(m.view.as_ref().unwrap()), 1);
        assert!(!m.enabled(NEXT));
        // Variant 30 is the one cell of the last page.
        m.click(0);
        assert_eq!(m.take_globals(), vec![(8, 0), (16, 0), (17, 30)]);
        // A single page has no arrows.
        let m = CgMode::open(&flags(&[4]), [1; 3], Some(4));
        assert_eq!(m.hit(560, 620), Some(OUTSIDE));
    }

    #[test]
    fn closing_the_picker() {
        let mut m = open(&[0]);
        m.click(0);
        m.take_sound();
        // A press beside the cells closes it with the confirm sound.
        assert_eq!(m.hit(5, 5), Some(OUTSIDE));
        m.click(OUTSIDE);
        assert!(m.view.is_none());
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        // A right click closes it silently and stays on the page.
        m.click(0);
        m.take_sound();
        m.cancel();
        assert!(m.view.is_none());
        assert_eq!(m.take_sound(), None);
        assert_eq!(m.settle(), None);
    }

    #[test]
    fn tabs_back_quit_and_cancel() {
        let mut m = open(&[]);
        // The CG tab itself does nothing.
        assert!(!m.enabled(0xf00));
        m.click(0xf00);
        assert_eq!(m.settle(), None);
        m.click(0xf01);
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        assert_eq!(m.take_globals(), vec![(16, 0), (17, 0)]);
        assert_eq!(m.settle(), Some(Click::Result(0x101)));
        assert_eq!(mode::next(Mode::Cg, 0x101), mode::Next::Go(Mode::Music));
        let mut m = open(&[]);
        m.click(0xf02);
        assert_eq!(m.settle(), Some(Click::Result(0x102)));
        // Dead tabs do nothing.
        let mut m = CgMode::open(&[], [1, 0, 0], None);
        m.click(0xf01);
        m.click(0xf02);
        assert_eq!(m.settle(), None);
        // Back, quit and the right click.
        let mut m = open(&[]);
        m.click(BACK);
        assert_eq!(m.settle(), Some(Click::Result(-1)));
        let mut m = open(&[]);
        assert_eq!(m.click(QUIT), Click::Bar(crate::ui::bar::QUIT));
        let mut m = open(&[]);
        m.cancel();
        assert_eq!(m.take_sound().as_deref(), Some("ses_002.wav"));
        assert_eq!(m.settle(), Some(Click::Result(-1)));
    }

    #[test]
    fn slide_in_and_out_run_on_the_clock() {
        let mut m = open(&[]);
        assert!(m.animating(0) && !m.animating(14 * 16));
        m.click(BACK);
        assert_eq!(m.tick(1000), None);
        assert_eq!(m.tick(1000 + 11 * 16), None);
        assert_eq!(m.tick(1000 + 12 * 16), Some(Click::Result(-1)));
    }
}
