//! きまぐれテンプテーション2's own engine screens: the investigation
//! notebook (捜査手帳), the collection (収集品) and the costume screen.
//!
//! Like the save screen they are drawn by the engine from a background,
//! a sheet and an `.area` file (`notebook.area`, `itemselect.area`,
//! `cosselect.area`). What they show comes from the script's byte
//! variables, read the way the executable reads them:
//!
//! * notebook entry `i` (36) is open when `b[900 + i]` is set, collection
//!   item `i` (22) when `b[801 + i]` is set. The scripts set each flag
//!   twice, at `n` and at `1000 + n`; the executable reads the second copy
//!   when no game is running (from the title menu).
//! * the costume is `b[701]` (form 1..4) and `b[708]` (the second outfit
//!   of a form); forms C and D are open when `b[711]` / `b[712]` (or
//!   `b[3012]` / `b[3013]`, the cleared-game copies) is 1. Closing
//!   the screen writes both back.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;

use crate::assets::Assets;
use crate::render::Canvas;
use crate::screens::{area, area_file, draw_gem_thumb, Cells};
use crate::ui::{Button, Ui};
use crate::Result;

pub const NOTEBOOK_ENTRIES: usize = 36;
pub const ITEMS: usize = 22;
pub const NOTEBOOK_VAR: usize = 900;
pub const ITEM_VAR: usize = 801;
/// Offset of the copies read outside a game.
pub const GLOBAL_COPY: usize = 1000;
pub const COSTUME_VAR: usize = 701;
pub const OUTFIT_VAR: usize = 708;
/// Forms C and D: (game flag, cleared-game flag).
pub const COSTUME_UNLOCK: [(usize, usize); 2] = [(711, 3012), (712, 3013)];

/// Ids shared by `notebook.area` and `itemselect.area`.
pub mod ids {
    /// Notebook rows or collection slots: `ENTRY + i`.
    pub const ENTRY: i32 = 0;
    pub const TEXT_UP: i32 = 256;
    pub const TEXT_DOWN: i32 = 257;
    pub const TEXT_SCROLL: i32 = 258;
    pub const BACK: i32 = 512;
    /// Notebook list pages.
    pub const PAGE_UP: i32 = 768;
    pub const PAGE_DOWN: i32 = 769;
    pub const NOTEBOOK_TAB: i32 = 1792;
    pub const ITEMS_TAB: i32 = 1793;
}

/// Ids of `cosselect.area`.
pub mod costume_ids {
    /// Form cards: `FORM + i`.
    pub const FORM: i32 = 0;
    pub const OUTFIT: i32 = 256;
    pub const BACK: i32 = 512;
    pub const NEXT: i32 = 768;
}

const NOTEBOOK_ROWS: usize = 12;
const TEXT_STEP: i32 = 40;

// `item_p.akb`: tabs (normal, hover, chosen, disabled), back, list page
// strips (up rows 0..4, down rows 4..8), gem thumb, heart arrows (up rows
// 0..4, down 4..8), slot frames (normal, hover, chosen, locked).
const TAB_NOTEBOOK: Cells = Cells::new(0, 0, 154, 123, 124);
const TAB_ITEMS: Cells = Cells::new(155, 0, 154, 123, 124);
const BACK_BUTTON: Cells = Cells::new(0, 492, 316, 54, 55);
const PAGE_STRIP: Cells = Cells::new(310, 0, 308, 30, 31);
const GEM: Cells = Cells::new(615, 1, 50, 62, 64);
const ARROW: Cells = Cells::new(660, 0, 52, 47, 47);
const FRAME: Cells = Cells::new(712, 0, 106, 106, 106);
/// `arechive_list.akb`: entry `i` at column `i % 4`, block `i / 4`; rows
/// normal, hover, chosen, locked.
const LIST_CELL: (u32, u32) = (304, 32);
const LIST_BLOCK: u32 = 224;
/// `archive_item_t.akb`: titles of notebook entries (left) and items
/// (right), one per 29 rows.
const TITLE_NOTEBOOK: (u32, u32) = (0, 558);
const TITLE_ITEMS: (u32, u32) = (558, 460);
const TITLE_H: u32 = 29;
/// `item_ics.akb`: 106-pixel icons, nine a row; `item_ic.akb`: 256-pixel
/// pictures, five a row (the sixth column holds play and pause buttons
/// of the last item, a voice recording, not done here).
const ICON_SMALL: u32 = 106;
const ICON_BIG: u32 = 256;
const BIG_PER_ROW: u32 = 5;
/// Placement on the backgrounds (measured on `archivebg` / `itembg`).
const NB_TITLE_POS: (i32, i32) = (566, 42);
const NB_TEXT: (i32, i32, i32) = (430, 100, 530);
const NB_TRACK: (i32, i32, i32, i32) = (1231, 93, 45, 520);
const IT_TITLE_POS: (i32, i32) = (712, 350);
const IT_PICTURE: (i32, i32) = (376, 385);
const IT_TEXT: (i32, i32, i32) = (668, 400, 255);
const IT_TRACK: (i32, i32, i32, i32) = (1230, 384, 45, 278);

// `coschange_p.akb`: four form cards of three columns (normal, chosen,
// locked) each, first outfit in rows 0..520, second below; back and next
// buttons; the outfit switch (to the second outfit rows 0..3, back to the
// first 3..6, disabled 6).
const CARD: (u32, u32) = (160, 520);
const CO_BACK: Cells = Cells::new(1920, 0, 188, 71, 72);
const CO_NEXT: Cells = Cells::new(2108, 0, 188, 71, 72);
const CO_OUTFIT: Cells = Cells::new(1918, 516, 118, 58, 58);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Book {
    Notebook,
    Items,
}

/// The notebook and collection screen (one screen with two tabs).
#[derive(Debug, Clone)]
pub struct ArchiveScreen {
    pub book: Book,
    /// Flags of the notebook entries and items, read when opened.
    pub notebook: Vec<u8>,
    pub items: Vec<u8>,
    /// First notebook row shown (a multiple of 12).
    pub top: usize,
    pub chosen: [Option<usize>; 2],
    /// Scroll of the shown text, in pixels.
    pub scroll: i32,
}

impl ArchiveScreen {
    /// Reads the flags; `global` reads the copies kept outside a game.
    pub fn new(book: Book, var: impl Fn(usize) -> u8, global: bool) -> Self {
        let base = if global { GLOBAL_COPY } else { 0 };
        Self {
            book,
            notebook: (0..NOTEBOOK_ENTRIES)
                .map(|i| var(base + NOTEBOOK_VAR + i))
                .collect(),
            items: (0..ITEMS).map(|i| var(base + ITEM_VAR + i)).collect(),
            top: 0,
            chosen: [None, None],
            scroll: 0,
        }
    }

    fn flags(&self) -> &[u8] {
        match self.book {
            Book::Notebook => &self.notebook,
            Book::Items => &self.items,
        }
    }

    pub fn open(&self, i: usize) -> bool {
        self.flags().get(i).is_some_and(|&v| v != 0)
    }

    pub fn chosen(&self) -> Option<usize> {
        self.chosen[self.book as usize]
    }

    /// Entry behind a row or slot id.
    pub fn entry(&self, id: i32) -> Option<usize> {
        let i = (id - ids::ENTRY) as usize;
        match self.book {
            Book::Notebook if i < NOTEBOOK_ROWS => Some(self.top + i),
            Book::Items if i < ITEMS => Some(i),
            _ => None,
        }
        .filter(|&e| e < self.flags().len())
    }
}

/// The costume screen.
#[derive(Debug, Clone)]
pub struct CostumeScreen {
    pub form: usize,
    pub second_outfit: bool,
    pub unlocked: [bool; 4],
}

impl CostumeScreen {
    pub fn new(var: impl Fn(usize) -> u8) -> Self {
        let mut unlocked = [true; 4];
        for (k, (game, cleared)) in COSTUME_UNLOCK.iter().enumerate() {
            unlocked[2 + k] = var(*game) == 1 || var(*cleared) == 1;
        }
        Self {
            form: (var(COSTUME_VAR) as usize).saturating_sub(1).min(3),
            second_outfit: var(OUTFIT_VAR) != 0,
            unlocked,
        }
    }

    /// The byte variables to write back: (index, value).
    pub fn result(&self) -> [(usize, u8); 2] {
        [
            (COSTUME_VAR, self.form as u8 + 1),
            (OUTFIT_VAR, self.second_outfit as u8),
        ]
    }
}

/// Layouts of the three screens.
pub struct Archive {
    notebook: Vec<Area>,
    items: Vec<Area>,
    costume: Vec<Area>,
}

fn find(areas: &[Area], x: i32, y: i32) -> Option<i32> {
    areas.iter().find(|a| a.contains(x, y)).map(|a| a.id)
}

impl Archive {
    pub fn load(assets: &Assets) -> Self {
        Self {
            notebook: area_file(assets, "notebook.area"),
            items: area_file(assets, "itemselect.area"),
            costume: area_file(assets, "cosselect.area"),
        }
    }

    fn areas(&self, book: Book) -> &[Area] {
        match book {
            Book::Notebook => &self.notebook,
            Book::Items => &self.items,
        }
    }

    // Notebook and collection.

    fn text_image(assets: &mut Assets, s: &ArchiveScreen) -> Option<Arc<Image>> {
        let e = s.chosen()?;
        let name = match s.book {
            Book::Notebook => format!("archive_text{e:02}.akb"),
            Book::Items => format!("item_text_{e:02}.akb"),
        };
        assets.image(&name).ok()
    }

    fn text_view(book: Book) -> (i32, i32, i32) {
        match book {
            Book::Notebook => NB_TEXT,
            Book::Items => IT_TEXT,
        }
    }

    fn track(book: Book) -> (i32, i32, i32, i32) {
        match book {
            Book::Notebook => NB_TRACK,
            Book::Items => IT_TRACK,
        }
    }

    /// How far the shown text can scroll.
    pub fn max_scroll(&self, assets: &mut Assets, s: &ArchiveScreen) -> i32 {
        let view = Self::text_view(s.book).2;
        Self::text_image(assets, s).map_or(0, |t| (t.height as i32 - view).max(0))
    }

    pub fn enabled(s: &ArchiveScreen, id: i32, max_scroll: i32) -> bool {
        use ids::*;
        match id {
            TEXT_UP => s.scroll > 0,
            TEXT_DOWN => s.scroll < max_scroll,
            PAGE_UP => s.book == Book::Notebook && s.top > 0,
            PAGE_DOWN => s.book == Book::Notebook && s.top + NOTEBOOK_ROWS < NOTEBOOK_ENTRIES,
            _ => match s.entry(id) {
                Some(e) => s.open(e),
                None => true,
            },
        }
    }

    pub fn hit(&self, s: &ArchiveScreen, x: i32, y: i32) -> Option<Button> {
        let id = find(self.areas(s.book), x, y)?;
        // Unknown ids (`itemselect.area` 768) do nothing.
        let known = matches!(
            id,
            ids::TEXT_UP
                | ids::TEXT_DOWN
                | ids::TEXT_SCROLL
                | ids::BACK
                | ids::NOTEBOOK_TAB
                | ids::ITEMS_TAB
        ) || (s.book == Book::Notebook && matches!(id, ids::PAGE_UP | ids::PAGE_DOWN))
            || s.entry(id).is_some();
        known.then_some(Button::Screen(id))
    }

    /// Fraction of the text track at `y`, if (x, y) is on it.
    pub fn track_fraction(&self, s: &ArchiveScreen, x: i32, y: i32) -> Option<f32> {
        area(self.areas(s.book), ids::TEXT_SCROLL).filter(|a| a.contains(x, y))?;
        let (_, ty, _, th) = Self::track(s.book);
        Some(((y - ty) as f32 / th as f32).clamp(0.0, 1.0))
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        s: &ArchiveScreen,
    ) -> Result<()> {
        use ids::*;
        let bg = match s.book {
            Book::Notebook => "archivebg.akb",
            Book::Items => "itembg.akb",
        };
        canvas.draw(&*assets.image(bg)?, 0, 0, false);
        let sheet = assets.image("item_p.akb")?;
        let max = self.max_scroll(assets, s);
        let state = |id: i32, active: bool| {
            ui.state(Button::Screen(id), Self::enabled(s, id, max), active) as u32
        };
        match s.book {
            Book::Notebook => self.draw_list(canvas, assets, ui, s)?,
            Book::Items => self.draw_slots(canvas, assets, ui, s)?,
        }
        for a in self.areas(s.book) {
            match a.id {
                NOTEBOOK_TAB => {
                    TAB_NOTEBOOK.draw(canvas, &sheet, state(a.id, s.book == Book::Notebook), a)
                }
                ITEMS_TAB => TAB_ITEMS.draw(canvas, &sheet, state(a.id, s.book == Book::Items), a),
                BACK => BACK_BUTTON.draw(canvas, &sheet, state(a.id, false).min(2), a),
                TEXT_UP => ARROW.draw(canvas, &sheet, state(a.id, false), a),
                TEXT_DOWN => ARROW.draw(canvas, &sheet, 4 + state(a.id, false), a),
                PAGE_UP if s.book == Book::Notebook => {
                    PAGE_STRIP.draw(canvas, &sheet, state(a.id, false), a)
                }
                PAGE_DOWN if s.book == Book::Notebook => {
                    PAGE_STRIP.draw(canvas, &sheet, 4 + state(a.id, false), a)
                }
                _ => {}
            }
        }
        // Title and text of the chosen entry.
        if let Some(e) = s.chosen() {
            let titles = assets.image("archive_item_t.akb")?;
            let ((tx, tw), pos) = match s.book {
                Book::Notebook => (TITLE_NOTEBOOK, NB_TITLE_POS),
                Book::Items => (TITLE_ITEMS, IT_TITLE_POS),
            };
            canvas.draw_part(
                &titles,
                (tx, TITLE_H * e as u32, tw, TITLE_H),
                pos.0,
                pos.1,
                false,
                255,
            );
            if s.book == Book::Items {
                let pictures = assets.image("item_ic.akb")?;
                let (px, py) = (e as u32 % BIG_PER_ROW, e as u32 / BIG_PER_ROW);
                canvas.draw_part(
                    &pictures,
                    (px * ICON_BIG, py * ICON_BIG, ICON_BIG, ICON_BIG),
                    IT_PICTURE.0,
                    IT_PICTURE.1,
                    false,
                    255,
                );
            }
            if let Some(text) = Self::text_image(assets, s) {
                let (x, y, view) = Self::text_view(s.book);
                let top = s.scroll.clamp(0, max) as u32;
                let h = (text.height - top).min(view as u32);
                canvas.draw_part(&text, (0, top, text.width, h), x, y, false, 255);
            }
        }
        let pressed = ui.pressed == Some(Button::Screen(TEXT_SCROLL));
        let hover = ui.hover == Some(Button::Screen(TEXT_SCROLL));
        draw_gem_thumb(
            canvas,
            &sheet,
            GEM,
            Self::track(s.book),
            s.scroll.max(0) as usize,
            max.max(1) as usize,
            pressed,
            hover,
        );
        Ok(())
    }

    fn draw_list(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        s: &ArchiveScreen,
    ) -> Result<()> {
        let list = assets.image("arechive_list.akb")?;
        for a in &self.notebook {
            let Some(e) = s.entry(a.id) else { continue };
            if a.id >= ids::TEXT_UP {
                continue;
            }
            let row = if !s.open(e) {
                3
            } else {
                ui.state(Button::Screen(a.id), true, s.chosen() == Some(e)) as u32
            };
            let (w, h) = LIST_CELL;
            let sx = (e as u32 % 4) * w;
            let sy = (e as u32 / 4) * LIST_BLOCK + row * h;
            canvas.draw_part(&list, (sx, sy, w, h), a.left, a.top, false, 255);
        }
        Ok(())
    }

    fn draw_slots(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        s: &ArchiveScreen,
    ) -> Result<()> {
        let icons = assets.image("item_ics.akb")?;
        let sheet = assets.image("item_p.akb")?;
        let per_row = icons.width / ICON_SMALL;
        for a in &self.items {
            let Some(e) = s.entry(a.id) else { continue };
            if a.id >= ids::TEXT_UP {
                continue;
            }
            if !s.open(e) {
                FRAME.draw(canvas, &sheet, 3, a);
                continue;
            }
            let chosen = s.chosen() == Some(e);
            if chosen {
                FRAME.draw(canvas, &sheet, 2, a);
            }
            let (ix, iy) = (e as u32 % per_row, e as u32 / per_row);
            let x = a.left + (a.width() as i32 - ICON_SMALL as i32) / 2;
            let y = a.top + (a.height() as i32 - ICON_SMALL as i32) / 2;
            canvas.draw_part(
                &icons,
                (ix * ICON_SMALL, iy * ICON_SMALL, ICON_SMALL, ICON_SMALL),
                x,
                y,
                false,
                255,
            );
            let hover = ui.hover == Some(Button::Screen(a.id));
            FRAME.draw(canvas, &sheet, hover as u32, a);
        }
        Ok(())
    }

    /// What a press does on the notebook or collection.
    pub fn press(&self, s: &mut ArchiveScreen, id: i32, max_scroll: i32) -> ArchiveAction {
        use ids::*;
        match id {
            BACK => return ArchiveAction::Close,
            NOTEBOOK_TAB | ITEMS_TAB => {
                s.book = if id == NOTEBOOK_TAB {
                    Book::Notebook
                } else {
                    Book::Items
                };
                s.scroll = 0;
            }
            TEXT_UP => s.scroll = (s.scroll - TEXT_STEP).max(0),
            TEXT_DOWN => s.scroll = (s.scroll + TEXT_STEP).min(max_scroll),
            PAGE_UP => s.top = s.top.saturating_sub(NOTEBOOK_ROWS),
            PAGE_DOWN if s.top + NOTEBOOK_ROWS < NOTEBOOK_ENTRIES => s.top += NOTEBOOK_ROWS,
            _ => {
                if let Some(e) = s.entry(id).filter(|&e| s.open(e)) {
                    s.chosen[s.book as usize] = Some(e);
                    s.scroll = 0;
                }
            }
        }
        ArchiveAction::None
    }

    /// Text scroll for a wheel turn.
    pub fn wheel(s: &mut ArchiveScreen, up: bool, max_scroll: i32) {
        let d = if up { -TEXT_STEP } else { TEXT_STEP };
        s.scroll = (s.scroll + d).clamp(0, max_scroll);
    }

    // Costume.

    pub fn hit_costume(&self, c: &CostumeScreen, x: i32, y: i32) -> Option<Button> {
        let id = find(&self.costume, x, y)?;
        match id {
            0..=3 if !c.unlocked[id as usize] => None,
            _ => Some(Button::Screen(id)),
        }
    }

    pub fn draw_costume(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        c: &CostumeScreen,
    ) -> Result<()> {
        use costume_ids::*;
        canvas.draw(&*assets.image("coschangebg.akb")?, 0, 0, false);
        let figure = match c.second_outfit {
            false => format!("cos{:02}f.akb", c.form),
            true => format!("cos{:02}fh.akb", c.form),
        };
        if let Ok(img) = assets.image(&figure) {
            canvas.draw(&img, 0, 0, false);
        }
        let sheet = assets.image("coschange_p.akb")?;
        for a in &self.costume {
            let st = |active: bool| ui.state(Button::Screen(a.id), true, active) as u32;
            match a.id {
                FORM..=3 => {
                    let k = (a.id - FORM) as u32;
                    let col = if !c.unlocked[k as usize] {
                        2
                    } else if c.form == k as usize || st(false) > 0 {
                        1
                    } else {
                        0
                    };
                    let (w, h) = CARD;
                    let sy = if c.second_outfit { h } else { 0 };
                    canvas.draw_part(
                        &sheet,
                        ((3 * k + col) * w, sy, w, h),
                        a.left,
                        a.top,
                        false,
                        255,
                    );
                }
                OUTFIT => {
                    let row = st(false).min(2) + if c.second_outfit { 3 } else { 0 };
                    CO_OUTFIT.draw(canvas, &sheet, row, a)
                }
                BACK => CO_BACK.draw(canvas, &sheet, st(false).min(2), a),
                NEXT => CO_NEXT.draw(canvas, &sheet, st(false).min(2), a),
                _ => {}
            }
        }
        Ok(())
    }

    /// What a press does on the costume screen.
    pub fn press_costume(c: &mut CostumeScreen, id: i32) -> ArchiveAction {
        use costume_ids::*;
        match id {
            FORM..=3 if c.unlocked[id as usize] => c.form = id as usize,
            OUTFIT => c.second_outfit ^= true,
            BACK | NEXT => return ArchiveAction::Close,
            _ => {}
        }
        ArchiveAction::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveAction {
    None,
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_and_entries() {
        let s = ArchiveScreen::new(Book::Notebook, |i| (i == 900 + 13) as u8, false);
        assert!(s.open(13) && !s.open(12));
        let mut s = s;
        s.top = 12;
        assert_eq!(s.entry(1), Some(13));
        assert_eq!(s.entry(256), None);
        let g = ArchiveScreen::new(Book::Items, |i| (i == 1000 + 801) as u8, true);
        assert!(g.items[0] == 1 && g.items[1] == 0);
    }

    #[test]
    fn costume_vars() {
        let c = CostumeScreen::new(|i| match i {
            701 => 3,
            708 => 1,
            3013 => 1,
            _ => 0,
        });
        assert_eq!(c.form, 2);
        assert!(c.second_outfit);
        assert_eq!(c.unlocked, [true, true, false, true]);
        assert_eq!(c.result(), [(701, 3), (708, 1)]);
    }
}
