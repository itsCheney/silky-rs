//! The system menu of 同級生 Remake (the menu button's save and load) and
//! its save and load screen.
//!
//! Read from the executable and compared under Wine. The system menu
//! screens share a column of diamond buttons down the left edge
//! (`menu_p.akb`, 164 x 164 cells; rows normal, hover, current, gray) whose
//! click areas come from `sysmenu.carea` (a byte per screen pixel): system
//! settings, sound, save, load, log, title, quit and back. Under them are a
//! strip along the bottom and the hint bar, which shows a line of
//! `SYSTEM_<lang>.MAP` for the button under the pointer.
//!
//! The save screen (`save_bg.akb`, or `load_bg.akb` for loading) shows ten
//! cards of `saveload_p_<lang>.akb` at the places `saveload.area` gives:
//! fourteen pages of manual saves, two of auto saves (A1, A2) and one of
//! quick saves (Q1). A card is 324 x 350, its picture a column of the sheet
//! (no data, data, the newest save) and a row by state; a card with data
//! holds the save's picture, when it was made, and the day, time and money
//! of the game then. Above the cards are the newest quick save and the
//! newest auto save, which load (or quick save) with one click. Saving
//! never goes to the auto or quick pages, so their tabs are gray squares
//! then. The bottom buttons switch the deletion mode (only when loading)
//! and delete everything.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;
use silky_format::map::MapFile;
use silky_vm::Snapshot;

use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::save::{SaveFile, Saves, QUICK};
use crate::Result;

use super::window::Font;

const W: usize = 1920;
const H: usize = 1080;

/// Ids of `saveload.area`, plus the side menu's from [`SIDE`].
pub mod ids {
    /// The ten cards, 0..10.
    pub const CARD: i32 = 0;
    /// Pages 01..14.
    pub const PAGE: i32 = 256;
    /// Auto save pages A1, A2.
    pub const AUTO_PAGE: i32 = 512;
    /// The quick save page Q1.
    pub const QUICK_PAGE: i32 = 768;
    pub const PREVIOUS: i32 = 1024;
    pub const NEXT: i32 = 1025;
    /// The newest quick save and the newest auto save, above the cards.
    pub const QUICK: i32 = 1280;
    pub const AUTO: i32 = 1536;
    /// The deletion mode switch and delete all.
    pub const DELETE: i32 = 1792;
    pub const DELETE_ALL: i32 = 1793;
    /// The side menu's buttons, by their `sysmenu.carea` value.
    pub const SIDE: i32 = 2048;
}

/// The side menu's buttons, in `sysmenu.carea` order.
pub mod side {
    pub const SETTINGS: usize = 0;
    pub const SOUND: usize = 1;
    pub const SAVE: usize = 2;
    pub const LOAD: usize = 3;
    pub const LOG: usize = 4;
    pub const TITLE: usize = 5;
    pub const QUIT: usize = 6;
    pub const BACK: usize = 7;
    pub const COUNT: usize = 8;
}

/// Where each side button's cell goes (measured under Wine), and where its
/// column starts in `menu_p.akb`.
const SIDE_POS: [(i32, i32); side::COUNT] = [
    (86, 200),
    (2, 285),
    (86, 370),
    (2, 455),
    (86, 540),
    (2, 625),
    (86, 710),
    (3, 795),
];
const SIDE_COLUMN: [u32; side::COUNT] = [0, 164, 328, 492, 656, 1148, 1312, 1624];
const SIDE_CELL: u32 = 164;
/// The corner piece at the bottom left.
const CORNER: (u32, u32, u32, u32) = (1476, 0, 148, 144);
const CORNER_POS: (i32, i32) = (0, 936);
/// The bottom strip (`menu_p.akb` rows 656..742), the hint bar (rows
/// 742..808) and the hint's first letter cell.
const STRIP: (u32, u32, u32, u32) = (0, 656, 1758, 86);
const STRIP_POS: (i32, i32) = (162, 994);
const HINT: (u32, u32, u32, u32) = (0, 742, 582, 66);
const HINT_POS: (i32, i32) = (1271, 1005);
const HINT_TEXT: (i32, i32) = (1338, 1029);
/// `SYSTEM_<lang>.MAP` lines of the side buttons, from settings to back.
const SIDE_HINT: u32 = 79;

/// Pages: fourteen of manual saves, two of auto saves, one of quick saves.
pub const MANUAL_PAGES: usize = 14;
const AUTO_PAGES: usize = 2;
pub const PAGES: usize = MANUAL_PAGES + AUTO_PAGES + 1;
const Q_PAGE: usize = PAGES - 1;
const CARDS: usize = 10;

/// Card pictures: 324 x 350, columns no data / data / newest, rows normal
/// / hover / pressed.
const CARD_SIZE: (u32, u32) = (324, 350);
/// The newest quick and auto saves: 612 x 170 from x 972 and 1584, rows
/// empty (normal, hover, pressed) then with data.
const HEADER: (u32, u32) = (612, 170);
const HEADER_SHEET_X: [u32; 2] = [972, 1584];
/// Page tabs: 78 x 56 from (56, 1050), rows normal / hover / current /
/// gray; the arrows are 56 wide at x 0 and 1382.
const TAB: (u32, u32) = (78, 56);
const TAB_SHEET: (u32, u32) = (56, 1050);
const ARROW_W: u32 = 56;
const NEXT_SHEET_X: u32 = 1382;
/// Delete and delete all: 128 x 78 from (1438, 1020), rows as the tabs.
const DELETE_SHEET: (u32, u32, u32, u32) = (1438, 1020, 128, 78);
/// Gray shade over a card or header that does nothing: black at half over
/// its own pixels.
const SHADE: u32 = 128;

fn draw_shade(canvas: &mut Canvas, sheet: &Image, part: (u32, u32, u32, u32), x: i32, y: i32) {
    let (px, py, w, h) = part;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for row in 0..h.min(sheet.height.saturating_sub(py)) {
        for col in 0..w.min(sheet.width.saturating_sub(px)) {
            let a = sheet.rgba[(((py + row) * sheet.width + px + col) * 4 + 3) as usize] as u32;
            rgba[((row * w + col) * 4 + 3) as usize] = (a * SHADE / 255) as u8;
        }
    }
    let img = Arc::new(Image {
        width: w,
        height: h,
        rgba,
    });
    canvas.draw(&img, x, y, false);
}

/// The game's state shown on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Info {
    /// 0 Sunday to 6 Saturday.
    pub weekday: i32,
    /// Month × 100 + day.
    pub date: i32,
    /// Hours × 100 + minutes.
    pub time: i32,
    /// In hundreds of yen.
    pub money: i32,
}

impl Info {
    pub fn of(vm: &Snapshot) -> Self {
        use super::plate::{DATE, MONEY, TIME, WEEKDAY};
        let g = |i| vm.var_g(i).int();
        Self {
            weekday: g(WEEKDAY),
            date: g(DATE),
            time: g(TIME),
            money: g(MONEY),
        }
    }
}

/// A save shown on a card or above the cards.
#[derive(Debug, Clone)]
pub struct Card {
    pub slot: String,
    pub thumb: Option<Arc<Image>>,
    pub saved_at: u64,
    pub info: Info,
}

impl Card {
    fn read(saves: &Saves, slot: &str) -> Option<Self> {
        let file: SaveFile = saves.read(slot).ok()?;
        Some(Self {
            slot: slot.to_string(),
            thumb: saves.thumb(slot).map(Arc::new),
            saved_at: file.saved_at,
            info: Info::of(&file.state.vm),
        })
    }
}

/// The save file name of card `i` of `page`: `save001`.. for the manual
/// pages, `auto01`.. for the auto pages; the quick page holds the one
/// quick save.
pub fn slot_name(page: usize, i: usize) -> Option<String> {
    match page {
        p if p < MANUAL_PAGES => Some(format!("save{:03}", p * CARDS + i + 1)),
        p if p < Q_PAGE => Some(format!("auto{:02}", (p - MANUAL_PAGES) * CARDS + i + 1)),
        _ if i == 0 => Some(QUICK.to_string()),
        _ => None,
    }
}

/// The save and load screen's state.
#[derive(Debug, Clone)]
pub struct SaveScreen {
    pub load: bool,
    pub page: usize,
    /// Clicking a card deletes it.
    pub deleting: bool,
    pub cards: Vec<Option<Card>>,
    pub quick: Option<Card>,
    pub auto: Option<Card>,
    /// The save written last, drawn with a clip.
    pub newest: Option<String>,
    pub any: bool,
    /// The game can be saved now (not from the title menu).
    pub can_save: bool,
    /// The backlog has something to show.
    pub log: bool,
}

impl SaveScreen {
    pub fn new(load: bool, can_save: bool, saves: Option<&Saves>) -> Self {
        let mut s = Self {
            can_save,
            load,
            page: 0,
            deleting: false,
            cards: Vec::new(),
            quick: None,
            auto: None,
            newest: None,
            any: false,
            log: false,
        };
        if let Some(saves) = saves {
            s.refresh(saves);
        }
        s
    }

    /// Reads the cards of the page and the newest quick and auto saves.
    pub fn refresh(&mut self, saves: &Saves) {
        let list = saves.list();
        self.any = !list.is_empty();
        self.newest = list.first().cloned();
        self.cards = (0..CARDS)
            .map(|i| {
                let slot = slot_name(self.page, i)?;
                list.contains(&slot).then(|| Card::read(saves, &slot))?
            })
            .collect();
        self.quick = list
            .contains(&QUICK.to_string())
            .then(|| Card::read(saves, QUICK))
            .flatten();
        self.auto = list
            .iter()
            .find(|s| s.starts_with("auto"))
            .and_then(|s| Card::read(saves, s));
        if !self.load || !self.any {
            self.deleting = false;
        }
    }

    pub fn card(&self, i: usize) -> Option<&Card> {
        self.cards.get(i).and_then(Option::as_ref)
    }

    /// Whether a page's tab can be picked: saving never goes to the auto
    /// and quick pages.
    pub fn page_open(&self, page: usize) -> bool {
        self.load || page < MANUAL_PAGES
    }

    fn last_page(&self) -> usize {
        if self.load {
            PAGES - 1
        } else {
            MANUAL_PAGES - 1
        }
    }

    /// Whether an element does something now.
    pub fn enabled(&self, id: i32) -> bool {
        use ids::*;
        match id {
            _ if (CARD..CARD + CARDS as i32).contains(&id) => {
                let i = (id - CARD) as usize;
                let card = self.card(i).is_some();
                if self.deleting {
                    card && self.page < MANUAL_PAGES || card && self.page == Q_PAGE
                } else if self.load {
                    card
                } else {
                    self.page < MANUAL_PAGES
                }
            }
            _ if (PAGE..PAGE + MANUAL_PAGES as i32).contains(&id) => true,
            AUTO_PAGE | 513 | QUICK_PAGE => self.load,
            PREVIOUS => self.page > 0,
            NEXT => self.page < self.last_page(),
            QUICK => {
                if self.deleting {
                    self.quick.is_some()
                } else {
                    !self.load || self.quick.is_some()
                }
            }
            AUTO => self.load && !self.deleting && self.auto.is_some(),
            DELETE => self.load && self.any,
            DELETE_ALL => self.any && !self.deleting,
            // The sound screen is not done yet.
            _ if id >= SIDE => match (id - SIDE) as usize {
                side::SOUND => false,
                side::LOG => self.log,
                side::SAVE => self.can_save,
                _ => true,
            },
            _ => true,
        }
    }

    /// The tab id of the current page.
    pub fn page_id(&self) -> i32 {
        page_id(self.page)
    }

    pub fn turn(&mut self, d: isize) {
        self.page = (self.page as isize + d).clamp(0, self.last_page() as isize) as usize;
    }

    /// The page of a tab id.
    pub fn page_of(id: i32) -> Option<usize> {
        use ids::*;
        match id {
            _ if (PAGE..PAGE + MANUAL_PAGES as i32).contains(&id) => Some((id - PAGE) as usize),
            _ if (AUTO_PAGE..AUTO_PAGE + AUTO_PAGES as i32).contains(&id) => {
                Some(MANUAL_PAGES + (id - AUTO_PAGE) as usize)
            }
            QUICK_PAGE => Some(Q_PAGE),
            _ => None,
        }
    }
}

fn page_id(page: usize) -> i32 {
    match page {
        p if p < MANUAL_PAGES => ids::PAGE + p as i32,
        p if p < Q_PAGE => ids::AUTO_PAGE + (p - MANUAL_PAGES) as i32,
        _ => ids::QUICK_PAGE,
    }
}

/// The loaded parts of the system menu screens.
pub struct System {
    pub(super) language: Language,
    side: Arc<Vec<u8>>,
    areas: Vec<Area>,
    /// `config00.area` and `config01.area` (the settings' two pages).
    pub(super) config: [Vec<Area>; 2],
    pub(super) system: Option<Arc<MapFile>>,
    font: std::cell::OnceCell<Option<Font>>,
}

impl std::fmt::Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("System")
    }
}

impl System {
    pub fn load(assets: &Assets, language: Language) -> Result<Self> {
        let mut side = assets.data("sysmenu.carea")?;
        side.resize(W * H, 255);
        Ok(Self {
            language,
            side: Arc::new(side),
            areas: crate::screens::area_file(assets, "saveload.area"),
            config: [
                crate::screens::area_file(assets, "config00.area"),
                crate::screens::area_file(assets, "config01.area"),
            ],
            system: assets
                .data(&format!("system_{}.map", language.suffix()))
                .ok()
                .and_then(|d| MapFile::parse(&d).ok())
                .map(Arc::new),
            font: std::cell::OnceCell::new(),
        })
    }

    pub(super) fn font(&self, assets: &mut Assets) -> Option<&Font> {
        self.font.get_or_init(|| Font::load(assets).ok()).as_ref()
    }

    /// The side button under (x, y), as an id from [`ids::SIDE`].
    pub fn hit_side(&self, x: i32, y: i32) -> Option<i32> {
        if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 {
            return None;
        }
        let v = self.side[y as usize * W + x as usize] as usize;
        (v < side::COUNT).then_some(ids::SIDE + v as i32)
    }

    /// The element of the save screen under (x, y).
    pub fn hit_save(&self, s: &SaveScreen, x: i32, y: i32) -> Option<i32> {
        if let Some(id) = self.hit_side(x, y) {
            return s.enabled(id).then_some(id);
        }
        let a = self.areas.iter().find(|a| a.contains(x, y))?;
        s.enabled(a.id).then_some(a.id)
    }

    /// The hint for an element of the save screen.
    fn save_hint(&self, s: &SaveScreen, id: i32) -> Option<String> {
        use ids::*;
        let line =
            |n: u32| -> Option<String> { Some(self.system.as_ref()?.get(0, n)?.to_string()) };
        let num = |n: u32, v: usize| -> Option<String> {
            let t = line(n)?;
            Some(
                t.replace("%03u", &format!("{v:03}"))
                    .replace("%02u", &format!("{v:02}"))
                    .replace("%02d", &format!("{v:02}")),
            )
        };
        match id {
            _ if id >= SIDE => line(SIDE_HINT + (id - SIDE) as u32),
            _ if (CARD..CARD + CARDS as i32).contains(&id) => {
                let i = (id - CARD) as usize;
                let n = s.page * CARDS + i + 1;
                match s.page {
                    p if p < MANUAL_PAGES => {
                        if s.deleting {
                            num(134, n)
                        } else if s.load {
                            num(137, n)
                        } else {
                            num(140, n)
                        }
                    }
                    p if p < Q_PAGE => {
                        let n = (p - MANUAL_PAGES) * CARDS + i + 1;
                        if s.deleting {
                            line(133)
                        } else {
                            num(136, n)
                        }
                    }
                    _ if s.deleting => num(132, i + 1),
                    _ => num(135, i + 1),
                }
            }
            _ if (PAGE..PAGE + MANUAL_PAGES as i32).contains(&id) => {
                num(141, (id - PAGE) as usize + 1)
            }
            AUTO_PAGE | 513 => num(142, (id - AUTO_PAGE) as usize + 1),
            ids::QUICK_PAGE => line(143),
            PREVIOUS => line(144),
            NEXT => line(145),
            QUICK if s.deleting => line(146),
            QUICK if s.load => line(147),
            QUICK => line(148),
            AUTO if s.deleting => line(149),
            AUTO if s.load => line(150),
            AUTO => line(151),
            DELETE if s.deleting => line(152),
            DELETE => line(153),
            DELETE_ALL => line(154),
            _ => None,
        }
    }

    /// Draws the bottom strip and the hint bar with its line, under a
    /// screen's own parts.
    pub fn draw_base(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        hint: Option<&str>,
    ) -> Result<()> {
        let sheet = assets.image("menu_p.akb")?;
        canvas.draw_part(&sheet, STRIP, STRIP_POS.0, STRIP_POS.1, false, 255);
        canvas.draw_part(&sheet, HINT, HINT_POS.0, HINT_POS.1, false, 255);
        if let (Some(text), Some(font)) = (hint, self.font(assets)) {
            font.draw_small(canvas, text, HINT_TEXT.0, HINT_TEXT.1);
        }
        Ok(())
    }

    /// Draws the side menu and the corner piece. `state` gives the row of a
    /// side button (0 normal, 1 hover, 2 current, 3 gray).
    pub fn draw_side(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        state: impl Fn(usize) -> u32,
    ) -> Result<()> {
        let sheet = assets.image("menu_p.akb")?;
        canvas.draw_part(&sheet, CORNER, CORNER_POS.0, CORNER_POS.1, false, 255);
        for (i, &(x, y)) in SIDE_POS.iter().enumerate() {
            let part = (SIDE_COLUMN[i], state(i) * SIDE_CELL, SIDE_CELL, SIDE_CELL);
            canvas.draw_part(&sheet, part, x, y, false, 255);
        }
        Ok(())
    }

    /// Draws the save screen. `state` gives the row (0 normal, 1 hover, 2
    /// pressed) of an element; `hover` is the element under the pointer.
    pub fn draw_save(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        s: &SaveScreen,
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
    ) -> Result<()> {
        use ids::*;
        let bg = assets.image(if s.load { "load_bg.akb" } else { "save_bg.akb" })?;
        canvas.draw(&bg, 0, 0, false);
        let hint = hover.and_then(|id| self.save_hint(s, id));
        self.draw_base(canvas, assets, hint.as_deref())?;
        let sheet = assets.localized_image("saveload_p.akb", self.language)?;
        let area = |id: i32| self.areas.iter().find(|a| a.id == id);
        // The newest quick and auto saves.
        for (k, (id, card)) in [(QUICK, &s.quick), (AUTO, &s.auto)].into_iter().enumerate() {
            let Some(a) = area(id) else { continue };
            let on = s.enabled(id);
            let row = if on { state(id).min(2) } else { 0 } + 3 * card.is_some() as u32;
            let (w, h) = HEADER;
            let part = (HEADER_SHEET_X[k], row * h, w, h);
            canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
            if let Some(c) = card {
                draw_info(canvas, &sheet, c, a.left, a.top, &HEADER_PLACES);
            }
            if !on {
                draw_shade(canvas, &sheet, part, a.left, a.top);
            }
        }
        // The cards.
        for i in 0..CARDS {
            let id = CARD + i as i32;
            let Some(a) = area(id) else { continue };
            let card = s.card(i);
            let on = s.enabled(id);
            let column = match card {
                None => 0,
                Some(c) if s.newest.as_deref() == Some(c.slot.as_str()) => 2,
                Some(_) => 1,
            };
            let row = if on { state(id).min(2) } else { 0 };
            let (w, h) = CARD_SIZE;
            let part = (column * w, row * h, w, h);
            canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
            let label = match s.page {
                p if p < MANUAL_PAGES => format!("{:03}", p * CARDS + i + 1),
                p if p < Q_PAGE => format!("A{:02}", (p - MANUAL_PAGES) * CARDS + i + 1),
                _ => format!("Q{:02}", i + 1),
            };
            draw_label(canvas, &sheet, &label, a.left + 8, a.top + 12);
            if let Some(c) = card {
                // The black field below holds the player's comment, which
                // is not kept here.
                draw_info(canvas, &sheet, c, a.left, a.top, &CARD_PLACES);
            }
            if !on {
                draw_shade(canvas, &sheet, part, a.left, a.top);
            }
        }
        // Page tabs and arrows.
        for p in 0..PAGES {
            let id = page_id(p);
            let Some(a) = area(id) else { continue };
            let row = if !s.page_open(p) {
                3
            } else if p == s.page {
                2
            } else {
                state(id).min(1)
            };
            let (w, h) = TAB;
            let x = TAB_SHEET.0 + w * p as u32;
            canvas.draw_part(
                &sheet,
                (x, TAB_SHEET.1 + row * h, w, h),
                a.left,
                a.top,
                false,
                255,
            );
        }
        for (id, sx) in [(PREVIOUS, 0), (NEXT, NEXT_SHEET_X)] {
            let Some(a) = area(id) else { continue };
            let row = if s.enabled(id) { state(id).min(2) } else { 3 };
            let h = TAB.1;
            canvas.draw_part(
                &sheet,
                (sx, TAB_SHEET.1 + row * h, ARROW_W, h),
                a.left,
                a.top,
                false,
                255,
            );
        }
        // Delete and delete all.
        for (k, id) in [DELETE, DELETE_ALL].into_iter().enumerate() {
            let Some(a) = area(id) else { continue };
            let row = if !s.enabled(id) {
                3
            } else if id == DELETE && s.deleting {
                2
            } else {
                state(id).min(2)
            };
            let (x, y, w, h) = DELETE_SHEET;
            canvas.draw_part(
                &sheet,
                (x + w * k as u32, y + h * row, w, h),
                a.left,
                a.top,
                false,
                255,
            );
        }
        let current = if s.load { side::LOAD } else { side::SAVE };
        self.draw_side(canvas, assets, |i| {
            let id = ids::SIDE + i as i32;
            if i == current {
                2
            } else if !s.enabled(id) {
                3
            } else {
                state(id).min(1)
            }
        })
    }
}

/// Where the parts of a save's state go on a card or a header (relative to
/// its top left): picture, date and time it was made, day, weekday tag, am
/// or pm, time, and the right end of the money (one digit further left
/// from 1000 hundreds of yen up).
struct Places {
    thumb: (i32, i32),
    made: (i32, i32),
    day: (i32, i32),
    tag: (i32, i32),
    half: (i32, i32),
    time: (i32, i32),
    money: [(i32, i32); 2],
}

const CARD_PLACES: Places = Places {
    thumb: (17, 49),
    made: (96, 31),
    day: (11, 217),
    tag: (97, 207),
    half: (282, 209),
    time: (180, 214),
    money: [(257, 241), (257, 223)],
};
const HEADER_PLACES: Places = Places {
    thumb: (12, 4),
    made: (309, 14),
    day: (305, 46),
    tag: (391, 34),
    half: (577, 36),
    time: (471, 41),
    money: [(550, 68), (532, 68)],
};
/// The size of a save's picture on a card.
pub const THUMB: (u32, u32) = (280, 158);

fn draw_info(canvas: &mut Canvas, sheet: &Arc<Image>, c: &Card, x: i32, y: i32, p: &Places) {
    if let Some(t) = &c.thumb {
        let (tx, ty) = (x + p.thumb.0, y + p.thumb.1);
        if (t.width, t.height) == THUMB {
            canvas.draw(t, tx, ty, false);
        } else if t.width > 0 {
            canvas.draw_scaled(t, tx as f32, ty as f32, THUMB.0 as f32 / t.width as f32);
        }
    }
    draw_made(
        canvas,
        sheet,
        &made_text(c.saved_at),
        x + p.made.0,
        y + p.made.1,
    );
    let info = c.info;
    let day = format!("{:2}", info.date.rem_euclid(100));
    draw_day(canvas, sheet, &day, info.weekday, x + p.day.0, y + p.day.1);
    // The weekday tag: Monday to Saturday then Sunday, 34 x 57 from x 339.
    let w = info.weekday.rem_euclid(7);
    let tag_x = 305 + 34 * if w == 0 { 7 } else { w } as u32;
    canvas.draw_part(
        sheet,
        (tag_x, 1274, 34, 57),
        x + p.tag.0,
        y + p.tag.1,
        false,
        255,
    );
    let (time, pm) = if info.time >= 1200 {
        (info.time - 1200, true)
    } else {
        (info.time, false)
    };
    let half_y = if pm { 1302 } else { 1274 };
    canvas.draw_part(
        sheet,
        (310, half_y, 29, 28),
        x + p.half.0,
        y + p.half.1,
        false,
        255,
    );
    let time = format!("{:02}:{:02}", time.max(0) / 100, time.max(0) % 100);
    draw_clock(canvas, sheet, &time, x + p.time.0, y + p.time.1);
    let yen = info.money.max(0) as u64 * 100;
    let (mx, my) = p.money[(yen >= 100_000) as usize];
    draw_money(canvas, sheet, &group(yen), x + mx, y + my);
}

/// `26/11/18 11:40:33`, local time.
fn made_text(secs: u64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(secs as i64, 0).single() {
        Some(t) => t.format("%y/%m/%d %H:%M:%S").to_string(),
        None => String::new(),
    }
}

/// `50000` -> `50,000`.
fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The card number (`001`, `A01`, `Q01`): outlined letters 30 high from
/// row 1400, A and Q 28 wide, digits 22 wide from x 56.
fn draw_label(canvas: &mut Canvas, sheet: &Arc<Image>, text: &str, x: i32, y: i32) {
    let mut cx = x;
    for c in text.chars() {
        let (sx, w) = match c {
            'A' => (0, 28),
            'Q' => (28, 28),
            '0'..='9' => (56 + 22 * (c as u32 - '0' as u32), 22),
            _ => continue,
        };
        canvas.draw_part(sheet, (sx, 1400, w, 30), cx, y, false, 255);
        cx += w as i32;
    }
}

/// When a save was made: small letters from row 1430, `/` and `:` 7 x
/// 16, digits 11 x 13 from x 14; a space takes 13.
fn draw_made(canvas: &mut Canvas, sheet: &Arc<Image>, text: &str, x: i32, y: i32) {
    let mut cx = x;
    for c in text.chars() {
        let step = match c {
            '/' | ':' => {
                let sx = if c == '/' { 0 } else { 7 };
                canvas.draw_part(sheet, (sx, 1430, 7, 16), cx, y, false, 255);
                7
            }
            '0'..='9' => {
                let sx = 14 + 11 * (c as u32 - '0' as u32);
                canvas.draw_part(sheet, (sx, 1430, 11, 13), cx, y, false, 255);
                11
            }
            _ => 13,
        };
        cx += step;
    }
}

/// The day: large digits 31 x 35, gray from row 1274, red on Sundays (row
/// 1309), blue on Saturdays (row 1344); each takes 35.
fn draw_day(canvas: &mut Canvas, sheet: &Arc<Image>, text: &str, weekday: i32, x: i32, y: i32) {
    let row = match weekday {
        0 => 1309,
        6 => 1344,
        _ => 1274,
    };
    let mut cx = x;
    for c in text.chars() {
        if let Some(d) = c.to_digit(10) {
            canvas.draw_part(sheet, (31 * d, row, 31, 35), cx, y, false, 255);
            cx += 35;
        } else {
            cx += 33;
        }
    }
}

/// The time: digits 18 x 21 from (20, 1379), `:` 10 wide at x 0.
fn draw_clock(canvas: &mut Canvas, sheet: &Arc<Image>, text: &str, x: i32, y: i32) {
    let mut cx = x;
    for c in text.chars() {
        if let Some(d) = c.to_digit(10) {
            canvas.draw_part(sheet, (20 + 18 * d, 1379, 18, 21), cx, y, false, 255);
            cx += 18;
        } else if c == ':' {
            canvas.draw_part(sheet, (0, 1379, 10, 21), cx, y, false, 255);
            cx += 10;
        }
    }
}

/// Money, drawn from its last digit leftwards with the last digit at `x`;
/// a comma (10 wide at x 10) takes 7.
fn draw_money(canvas: &mut Canvas, sheet: &Arc<Image>, text: &str, x: i32, y: i32) {
    let mut cx = x;
    for c in text.chars().rev() {
        if let Some(d) = c.to_digit(10) {
            canvas.draw_part(sheet, (20 + 18 * d, 1379, 18, 21), cx, y, false, 255);
            cx -= 18;
        } else if c == ',' {
            canvas.draw_part(sheet, (10, 1379, 10, 21), cx + 10, y, false, 255);
            cx -= 7;
        }
    }
}

/// The yes/no dialog (`dialog_p_<lang>.akb`, places from `dialog.area`):
/// the screen shaded, an 880 x 330 box at (520, 375) (the one with the
/// comment field when saving), the question (727 x 52 strips from x 880,
/// in the sheet's order), yes and no (222 x 73 from (96, 660) and (318,
/// 660), rows normal, hover, pressed) and the "don't show again" box (48 x
/// 48 from (0, 660), the fourth row ticked). The comment field takes no
/// typing here.
pub fn draw_dialog(
    canvas: &mut Canvas,
    sheet: &Arc<Image>,
    d: &crate::screens::Dialog,
    areas: &[Area],
    state: impl Fn(i32) -> u32,
) {
    use crate::screens::{dialog_ids, Action};
    canvas.fill(0, 0, canvas.width, canvas.height, [0, 0, 0, 127]);
    let saving = matches!(d.action, Action::Save(_));
    let box_y = if saving { 0 } else { 330 };
    canvas.draw_part(sheet, (0, box_y, 880, 330), 520, 375, false, 255);
    if saving {
        // The comment field's clear button.
        canvas.draw_part(sheet, (48, 660, 48, 48), 1159, 483, false, 255);
    }
    let question = match d.action {
        Action::Save(_) if d.fresh => 0,
        Action::Load(_) => 1,
        Action::Title => 2,
        Action::Quit => 3,
        Action::Save(_) => 4,
        Action::Delete(_) => 5,
        Action::DeleteAll => 6,
        Action::Jump(_) => 7,
        Action::Scene(_) => 8,
        Action::Unlock(_) => 9,
    };
    let question_y = if saving { 411 } else { 443 };
    canvas.draw_part(
        sheet,
        (880, 52 * question, 727, 52),
        597,
        question_y,
        false,
        255,
    );
    for a in areas {
        let st = state(a.id);
        match a.id {
            dialog_ids::YES | dialog_ids::NO => {
                let sx = if a.id == dialog_ids::YES { 96 } else { 318 };
                canvas.draw_part(
                    sheet,
                    (sx, 660 + 73 * st.min(2), 222, 73),
                    a.left,
                    a.top,
                    false,
                    255,
                );
            }
            dialog_ids::DONT_ASK => {
                let row = if d.dont_ask { 3 } else { st.min(2) };
                canvas.draw_part(
                    sheet,
                    (0, 660 + 48 * row, 48, 48),
                    a.left,
                    a.top,
                    false,
                    255,
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_and_pages() {
        assert_eq!(slot_name(0, 0).as_deref(), Some("save001"));
        assert_eq!(slot_name(13, 9).as_deref(), Some("save140"));
        assert_eq!(slot_name(15, 0).as_deref(), Some("auto11"));
        assert_eq!(slot_name(Q_PAGE, 0).as_deref(), Some("quick"));
        assert_eq!(slot_name(Q_PAGE, 1), None);
        assert_eq!(SaveScreen::page_of(ids::AUTO_PAGE + 1), Some(15));
        assert_eq!(page_id(Q_PAGE), ids::QUICK_PAGE);
        assert_eq!(group(50000), "50,000");
        assert_eq!(group(1234567), "1,234,567");
        assert_eq!(group(500), "500");
    }
}
