//! Engine screens drawn over the game: the save and load screen, the
//! backlog, and the yes/no dialog.
//!
//! None of these are scripted; the engine draws them from a background,
//! a sprite sheet and an `.area` file of click rectangles each. The cells
//! of the sheets were measured on the sheets themselves. Help lines for
//! the save screen come from `saveloadhelp.map`.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::{self, Area};

use crate::assets::{localized_name, Assets, HelpLines, Language};
use crate::render::Canvas;
use crate::save::{self, Kind};
use crate::text::TextRenderer;
use crate::ui::{Button, Ui};
use crate::{Result, SaveState};

/// Ids of `saveload.area`.
pub mod save_ids {
    /// Delete buttons of the seven rows: `DELETE + row`.
    pub const DELETE: i32 = 0;
    pub const SAVE_MODE: i32 = 256;
    pub const LOAD_MODE: i32 = 257;
    pub const DELETE_ALL: i32 = 512;
    pub const BACK: i32 = 768;
    pub const UP: i32 = 1024;
    pub const DOWN: i32 = 1025;
    pub const SCROLL: i32 = 1026;
    /// Tabs: manual saves, auto saves, quick saves.
    pub const TAB_MANUAL: i32 = 1280;
    pub const TAB_AUTO: i32 = 1281;
    pub const TAB_QUICK: i32 = 1282;
    /// Rows: `ROW + row`.
    pub const ROW: i32 = 1536;
}

/// Ids of `backlog00.area`.
pub mod log_ids {
    /// Entries: `TEXT + i` is the text, `VOICE + i` its voice button.
    pub const TEXT: i32 = 0;
    /// The area file gives the voice buttons the same ids as the text;
    /// they are moved here.
    pub const VOICE: i32 = 100;
    /// Mark an entry for "play the marked voices".
    pub const CHECK: i32 = 512;
    /// Go back to this entry.
    pub const JUMP: i32 = 768;
    pub const UP: i32 = 1024;
    pub const DOWN: i32 = 1025;
    pub const SCROLL: i32 = 1026;
    pub const PAGE_UP: i32 = 1280;
    pub const PAGE_DOWN: i32 = 1281;
    pub const PLAY_MARKED: i32 = 1536;
    pub const BACK: i32 = 1792;
}

/// Ids of `dialog.area`.
pub mod dialog_ids {
    pub const YES: i32 = 0;
    pub const NO: i32 = 1;
    /// "Do not ask again".
    pub const DONT_ASK: i32 = 256;
}

/// Questions of the dialog sheet, top to bottom.
pub mod question {
    pub const SAVE: usize = 0;
    pub const LOAD: usize = 1;
    pub const QUIT: usize = 2;
    pub const TITLE: usize = 3;
    pub const DELETE: usize = 4;
    pub const OVERWRITE: usize = 5;
    pub const DELETE_ALL: usize = 6;
    pub const JUMP: usize = 7;
    pub const SCENE_SELECT: usize = 8;
    /// きまぐれテンプテーション2: unlock all CG, scenes, music (two lines
    /// each under a "!注意!" line).
    pub const UNLOCK: usize = 10;
}

/// Rows the save screen shows at once, entries the backlog shows.
pub const SAVE_ROWS: usize = 7;
pub const LOG_ROWS: usize = 3;

/// A cell column of a sheet: left, top, width, height, and the distance
/// between rows (one row per button state).
#[derive(Clone, Copy)]
pub(crate) struct Cells {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub pitch: u32,
}

impl Cells {
    pub const fn new(x: u32, y: u32, w: u32, h: u32, pitch: u32) -> Self {
        Self { x, y, w, h, pitch }
    }

    pub fn part(self, row: u32) -> (u32, u32, u32, u32) {
        (self.x, self.y + self.pitch * row, self.w, self.h)
    }

    /// Draws row `row` centred on an area.
    pub fn draw(self, canvas: &mut Canvas, sheet: &Image, row: u32, a: &Area) {
        let x = a.left + (a.width() as i32 - self.w as i32) / 2;
        let y = a.top + (a.height() as i32 - self.h as i32) / 2;
        canvas.draw_part(sheet, self.part(row), x, y, false, 255);
    }
}

/// Look of the save and load screen: images, sheet cells, placement.
struct SaveStyle {
    save_bg: &'static str,
    load_bg: &'static str,
    sheet: &'static str,
    /// Image names take a language suffix (`save_bg_en.akb`).
    localized: bool,
    save_mode: Cells,
    load_mode: Cells,
    /// Up arrows in rows 0..4, down arrows in rows 4..8.
    arrow: Cells,
    delete_all: Cells,
    delete: Cells,
    back: Cells,
    /// Rows 0..3 "create a save", 3..6 a save, 6..9 the newest save.
    row: Cells,
    /// Manual tab: rows 0..4 save mode, 4..8 load mode. Auto rows 0..4,
    /// quick rows 4..8 of the second column.
    tab_manual: Cells,
    tab_other: Cells,
    /// Small digits for dates: top, height, '/', ':', then 0..9 as
    /// (left, width, pitch).
    digit_y: u32,
    digit_h: u32,
    slash: (u32, u32),
    colon: (u32, u32),
    digits: (u32, u32, u32),
    /// Scroll bar track, measured on the background.
    track: (i32, i32, i32, i32),
    /// A gem that runs down the track, if the thumb is not a plain bar.
    gem: Option<Cells>,
    help_pos: (f32, f32),
    help_ink: [u8; 4],
    ink: [u8; 4],
    /// Row label left of a save and of the newest save (after its badge).
    label_x: (f32, f32),
    /// Row column whose colour fills the black text field.
    field_fill: u32,
}

/// `save_load_p_<lang>.akb` of ふゆから、くるる。
const FUYUKURU_SAVE: SaveStyle = SaveStyle {
    save_bg: "save_bg.akb",
    load_bg: "load_bg.akb",
    sheet: "save_load_p.akb",
    localized: true,
    save_mode: Cells::new(0, 0, 186, 117, 117),
    load_mode: Cells::new(186, 0, 186, 117, 117),
    arrow: Cells::new(373, 0, 52, 53, 54),
    delete_all: Cells::new(427, 0, 71, 73, 74),
    delete: Cells::new(427, 296, 62, 62, 66),
    back: Cells::new(565, 0, 88, 91, 91),
    row: Cells::new(653, 0, 970, 82, 82),
    tab_manual: Cells::new(1625, 0, 270, 67, 67),
    tab_other: Cells::new(1895, 0, 270, 67, 67),
    digit_y: 738,
    digit_h: 14,
    slash: (833, 7),
    colon: (840, 6),
    digits: (846, 11, 11),
    track: (1008, 121, 26, 511),
    gem: None,
    help_pos: (96.0, 694.0),
    help_ink: INK,
    ink: INK,
    label_x: (160.0, 280.0),
    field_fill: 156,
};

/// `saveload_p.akb` of きまぐれテンプテーション2. The rows have the same
/// inner layout as in ふゆから、くるる。; the third tab column holds the
/// row delete button (rows 0..4) and "delete all" (rows 4..8).
const KIMATEN2_SAVE: SaveStyle = SaveStyle {
    save_bg: "savebg.akb",
    load_bg: "loadbg.akb",
    sheet: "saveload_p.akb",
    localized: false,
    save_mode: Cells::new(0, 0, 196, 125, 125),
    load_mode: Cells::new(196, 0, 196, 125, 125),
    arrow: Cells::new(698, 0, 48, 47, 47),
    delete_all: Cells::new(2053, 268, 64, 67, 67),
    delete: Cells::new(2053, 0, 64, 67, 67),
    back: Cells::new(392, 0, 188, 71, 72),
    row: Cells::new(747, 0, 970, 82, 82),
    tab_manual: Cells::new(1717, 0, 168, 67, 67),
    tab_other: Cells::new(1885, 0, 168, 67, 67),
    digit_y: 525,
    digit_h: 14,
    slash: (120, 12),
    colon: (132, 10),
    digits: (0, 12, 12),
    track: (988, 116, 45, 512),
    gem: Some(Cells::new(648, 1, 46, 62, 64)),
    help_pos: (164.0, 702.0),
    help_ink: [255, 255, 255, 255],
    ink: K2_INK,
    label_x: (160.0, 160.0),
    field_fill: 892,
};

/// Text on the pink screens of きまぐれテンプテーション2.
pub(crate) const K2_INK: [u8; 4] = [120, 32, 62, 255];

/// Inside a row: thumbnail, the text field (drawn black on the sheet),
/// the place of the label after the "new" badge.
const ROW_THUMB: (i32, i32) = (18, 4);
const ROW_FIELD: (u32, u32, u32, u32) = (160, 44, 729, 27);
/// Scroll bar track of the backlog, measured on the background.
const LOG_TRACK: (i32, i32, i32, i32) = (1069, 121, 26, 511);
pub(crate) const THUMB_COLOR: [u8; 4] = [18, 112, 190, 255];
/// Help line at the bottom of the save screen.
const HELP_SIZE: f32 = 17.0;
pub(crate) const INK: [u8; 4] = [34, 74, 120, 255];

// きまぐれテンプテーション2's `log_p.akb`: hearts with four state rows.
const K_LOG_VOICE: Cells = Cells::new(0, 0, 114, 102, 103);
const K_LOG_PLAY: Cells = Cells::new(114, 0, 114, 102, 103);
const K_LOG_PAGE_UP: Cells = Cells::new(228, 0, 108, 71, 72);
const K_LOG_PAGE_DOWN: Cells = Cells::new(336, 0, 108, 71, 72);
/// Up arrows in rows 0..4, down arrows in rows 4..8.
const K_LOG_ARROW: Cells = Cells::new(444, 0, 48, 47, 47);
/// Scroll thumb: normal, hover, pressed.
const K_LOG_GEM: Cells = Cells::new(494, 1, 43, 62, 64);
const K_LOG_BACK: Cells = Cells::new(540, 0, 128, 71, 72);
const K_LOG_JUMP: Cells = Cells::new(670, 0, 64, 67, 68);
/// Rows: empty, hover, pressed, marked.
const K_LOG_CHECK: Cells = Cells::new(734, 0, 32, 32, 32);
/// The scroll area of `backlog00.area`.
const K_LOG_TRACK: (i32, i32, i32, i32) = (1087, 68, 45, 561);
const K_LOG_TEXT_SIZE: f32 = 18.0;
const K_LOG_ADVANCE: f32 = 18.2;
const K_LOG_TEXT_LEFT: f32 = 15.0;
const K_LOG_BASELINE: f32 = 41.0;
/// Not measured (no screenshot of a long entry).
const K_LOG_LINE: f32 = 30.0;

// `log_p_<lang>.akb`.
const LOG_VOICE: Cells = Cells::new(0, 0, 74, 77, 79);
const LOG_PLAY: Cells = Cells::new(75, 0, 100, 103, 103);
/// Rows: empty, hover, pressed, marked.
const LOG_CHECK: Cells = Cells::new(175, 0, 36, 36, 36);
const LOG_JUMP: Cells = Cells::new(212, 0, 60, 62, 62);
const LOG_ARROW: Cells = Cells::new(274, 0, 52, 53, 54);
const LOG_PAGE: Cells = Cells::new(328, 0, 75, 75, 76);
const LOG_BACK: Cells = Cells::new(468, 0, 88, 91, 91);
const LOG_TEXT_SIZE: f32 = 21.0;
const LOG_LINE: f32 = 32.0;
const NAMEPLATE: (u32, u32) = (376, 56);

/// Look of the yes/no dialog.
struct DialogStyle {
    sheet: &'static str,
    localized: bool,
    yes: Cells,
    no: Cells,
    /// Rows: empty, hover, pressed, ticked.
    check: Cells,
    box_part: (u32, u32, u32, u32),
    box_pos: (i32, i32),
    /// Question images, one per `question_pitch` rows, to the right of
    /// the box; drawn centred on `question_center`, top at `question_y`.
    question: (u32, u32, u32, u32),
    question_pitch: u32,
    question_center: i32,
    question_y: i32,
}

/// `dialog_p_<lang>.akb` of ふゆから、くるる。
const FUYUKURU_DIALOG: DialogStyle = DialogStyle {
    sheet: "dialog_p.akb",
    localized: true,
    yes: Cells::new(0, 0, 90, 47, 48),
    no: Cells::new(90, 0, 90, 47, 48),
    check: Cells::new(0, 145, 26, 26, 28),
    box_part: (180, 0, 516, 203),
    box_pos: (382, 221),
    question: (700, 3, 780, 31),
    question_pitch: 36,
    question_center: 640,
    question_y: 251,
};

/// `dialog_p.akb` of きまぐれテンプテーション2: a winged box whose dark bar
/// takes the question. Box placed so that its "don't ask again" text
/// follows the check box of `dialog.area`.
const KIMATEN2_DIALOG: DialogStyle = DialogStyle {
    sheet: "dialog_p.akb",
    localized: false,
    yes: Cells::new(728, 0, 90, 47, 47),
    no: Cells::new(728, 141, 90, 47, 47),
    check: Cells::new(728, 482, 20, 20, 20),
    box_part: (0, 0, 724, 365),
    box_pos: (248, 74),
    question: (820, 1, 601, 36),
    question_pitch: 40,
    question_center: 616,
    question_y: 278,
};

/// Unlock dialog of きまぐれテンプテーション2's `dialog_p.akb`: the second
/// box (no question bar), the "!注意!" line (top, height) and blocks of
/// three small lines, one per screen (CG, scenes, music), 51 rows apart.
const K2_UNLOCK_BOX: (u32, u32, u32, u32) = (0, 368, 724, 312);
const K2_UNLOCK_NOTE: (u32, u32) = (361, 29);
const K2_UNLOCK_NOTE_Y: i32 = 236;
const K2_UNLOCK_TEXT: (u32, u32, u32, u32) = (830, 643, 590, 51);
const K2_UNLOCK_TEXT_POS: (i32, i32) = (421, 266);

/// The save screen is for saving or for loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Save,
    Load,
}

/// A save as listed.
pub struct SlotRow {
    pub slot: String,
    pub kind: Kind,
    pub number: u32,
    pub saved_at: u64,
    pub text: Option<String>,
    pub thumb: Option<Arc<Image>>,
    pub newest: bool,
}

/// What a row of the save screen holds.
pub enum Item<'a> {
    Create,
    Slot(&'a SlotRow),
}

pub struct SaveLoad {
    pub mode: Mode,
    pub tab: Kind,
    /// First item shown.
    pub scroll: usize,
    pub rows: Vec<SlotRow>,
    /// Whether the game can be saved now (not from the title menu).
    pub can_save: bool,
}

impl SaveLoad {
    /// Reads the list of the current tab.
    pub fn refresh(&mut self, saves: &save::Saves) {
        let newest = saves.list().into_iter().next();
        self.rows = saves
            .list_kind(self.tab)
            .into_iter()
            .filter_map(|slot| {
                let (kind, number) = Kind::of(&slot)?;
                let file = saves.read(&slot).ok()?;
                Some(SlotRow {
                    newest: newest.as_deref() == Some(slot.as_str()),
                    thumb: saves.thumb(&slot).map(Arc::new),
                    kind,
                    number,
                    saved_at: file.saved_at,
                    text: file.text,
                    slot,
                })
            })
            .collect();
        self.scroll = self.scroll.min(self.max_scroll());
    }

    fn has_create(&self) -> bool {
        self.mode == Mode::Save && self.tab == Kind::Manual
    }

    pub fn len(&self) -> usize {
        self.rows.len() + self.has_create() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn max_scroll(&self) -> usize {
        self.len().saturating_sub(SAVE_ROWS)
    }

    /// Item shown in a visible row.
    pub fn item(&self, row: usize) -> Option<Item<'_>> {
        let k = self.scroll + row;
        if self.has_create() {
            if k == 0 {
                return Some(Item::Create);
            }
            return self.rows.get(k - 1).map(Item::Slot);
        }
        self.rows.get(k).map(Item::Slot)
    }

    pub fn scroll_by(&mut self, d: isize) {
        self.scroll = (self.scroll as isize + d).clamp(0, self.max_scroll() as isize) as usize;
    }
}

/// A shown message, for the backlog.
#[derive(Clone)]
pub struct LogEntry {
    pub speaker: Option<i32>,
    /// Speaker given by name (きまぐれテンプテーション2).
    pub speaker_name: Option<String>,
    pub text: String,
    pub voice: Option<String>,
    /// Game state at this message, to go back to it.
    pub state: SaveState,
    pub marked: bool,
}

pub struct Backlog {
    /// First entry shown.
    pub top: usize,
}

impl Backlog {
    pub fn max_top(len: usize) -> usize {
        len.saturating_sub(LOG_ROWS)
    }

    pub fn scroll_by(&mut self, d: isize, len: usize) {
        self.top = (self.top as isize + d).clamp(0, Self::max_top(len) as isize) as usize;
    }
}

/// What "yes" does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Save into a new slot (`None`) or over one.
    Save(Option<String>),
    Load(String),
    Delete(String),
    DeleteAll,
    /// Go back to a backlog entry.
    Jump(usize),
    Title,
    Quit,
    /// Replay a scene of the scene mode.
    Scene(usize),
    /// きまぐれテンプテーション2: mark everything of an extras screen seen.
    Unlock(crate::Extras),
}

impl Action {
    pub fn question(&self) -> usize {
        match self {
            Action::Save(None) => question::SAVE,
            Action::Save(Some(_)) => question::OVERWRITE,
            Action::Load(_) => question::LOAD,
            Action::Delete(_) => question::DELETE,
            Action::DeleteAll => question::DELETE_ALL,
            Action::Jump(_) => question::JUMP,
            Action::Title => question::TITLE,
            Action::Quit => question::QUIT,
            Action::Scene(_) => question::SCENE_SELECT,
            Action::Unlock(k) => question::UNLOCK + *k as usize,
        }
    }
}

pub struct Dialog {
    pub action: Action,
    /// "Do not ask again" ticked.
    pub dont_ask: bool,
}

/// An engine screen that is up.
pub enum Screen {
    SaveLoad(SaveLoad),
    Backlog(Backlog),
    Settings(crate::menus::SettingsScreen),
    Extras(crate::menus::ExtrasScreen),
    /// きまぐれテンプテーション2's notebook and collection.
    Archive(crate::archive::ArchiveScreen),
    /// きまぐれテンプテーション2's costume screen.
    Costume(crate::archive::CostumeScreen),
}

/// Layouts of the screens, loaded once per game.
pub struct Screens {
    language: Language,
    save_load: Vec<Area>,
    backlog: Vec<Area>,
    dialog: Vec<Area>,
    help: Option<HelpLines>,
    /// きまぐれテンプテーション2: its own backlog look.
    kimaten2: bool,
}

pub(crate) fn area_file(assets: &Assets, name: &str) -> Vec<Area> {
    assets
        .data(name)
        .ok()
        .and_then(|d| area::parse(&d).ok())
        .unwrap_or_default()
}

fn find(areas: &[Area], x: i32, y: i32) -> Option<Button> {
    areas
        .iter()
        .find(|a| a.contains(x, y))
        .map(|a| Button::Screen(a.id))
}

pub(crate) fn area(areas: &[Area], id: i32) -> Option<&Area> {
    areas.iter().find(|a| a.id == id)
}

impl Screens {
    pub fn load(assets: &Assets, language: Language) -> Self {
        let mut backlog = area_file(assets, "backlog00.area");
        // Each entry has two rectangles under one id: the text, and the
        // narrower voice button (after the text in ふゆから、くるる。,
        // before it in きまぐれテンプテーション2).
        for id in 0..log_ids::CHECK {
            let pair: Vec<usize> = (0..backlog.len())
                .filter(|&i| backlog[i].id == id)
                .collect();
            if let [i, j] = pair[..] {
                let voice = if backlog[i].width() < backlog[j].width() {
                    i
                } else {
                    j
                };
                backlog[voice].id += log_ids::VOICE;
            }
        }
        Self {
            language,
            save_load: area_file(assets, "saveload.area"),
            backlog,
            dialog: area_file(assets, "dialog.area"),
            kimaten2: assets.game().title == silky_format::game::Title::Kimaten2,
            help: HelpLines::load(assets, "saveloadhelp"),
        }
    }

    fn sheet(&self, assets: &mut Assets, name: &str) -> Result<Arc<Image>> {
        assets.image(&localized_name(name, self.language))
    }

    fn save_style(&self) -> &'static SaveStyle {
        match self.kimaten2 {
            true => &KIMATEN2_SAVE,
            false => &FUYUKURU_SAVE,
        }
    }

    fn style_image(&self, assets: &mut Assets, st: &SaveStyle, name: &str) -> Result<Arc<Image>> {
        match st.localized {
            true => self.sheet(assets, name),
            false => assets.image(name),
        }
    }

    // Save and load screen.

    pub fn hit_save_load(&self, s: &SaveLoad, x: i32, y: i32) -> Option<Button> {
        use save_ids::*;
        let b = find(&self.save_load, x, y)?;
        let Button::Screen(id) = b else { return None };
        // Rows and delete buttons exist only where there is an item.
        let row = match id {
            DELETE..=6 => Some((id - DELETE) as usize),
            _ if (ROW..ROW + SAVE_ROWS as i32).contains(&id) => Some((id - ROW) as usize),
            _ => None,
        };
        match row.map(|r| s.item(r)) {
            Some(None) => return None,
            Some(Some(Item::Create)) if id < ROW => return None,
            _ => {}
        }
        Some(b)
    }

    pub fn save_load_enabled(&self, s: &SaveLoad, id: i32, any_saves: bool) -> bool {
        use save_ids::*;
        match id {
            SAVE_MODE => s.can_save,
            UP => s.scroll > 0,
            DOWN => s.scroll < s.max_scroll(),
            DELETE_ALL => any_saves,
            TAB_AUTO | TAB_QUICK => s.mode == Mode::Load,
            _ => true,
        }
    }

    pub fn draw_save_load(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        s: &SaveLoad,
        any_saves: bool,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        use save_ids::*;
        let sl = self.save_style();
        let bg = match s.mode {
            Mode::Save => sl.save_bg,
            Mode::Load => sl.load_bg,
        };
        canvas.draw(&*self.style_image(assets, sl, bg)?, 0, 0, false);
        let sheet = self.style_image(assets, sl, sl.sheet)?;
        let state = |id: i32, active: bool| {
            let b = Button::Screen(id);
            ui.state(b, self.save_load_enabled(s, id, any_saves), active) as u32
        };
        for a in &self.save_load {
            let id = a.id;
            match id {
                SAVE_MODE => sl
                    .save_mode
                    .draw(canvas, &sheet, state(id, s.mode == Mode::Save), a),
                LOAD_MODE => sl
                    .load_mode
                    .draw(canvas, &sheet, state(id, s.mode == Mode::Load), a),
                DELETE_ALL => sl.delete_all.draw(canvas, &sheet, state(id, false), a),
                BACK => sl.back.draw(canvas, &sheet, state(id, false), a),
                UP => sl.arrow.draw(canvas, &sheet, state(id, false), a),
                DOWN => sl.arrow.draw(canvas, &sheet, 4 + state(id, false), a),
                TAB_MANUAL => {
                    let base = if s.mode == Mode::Save { 0 } else { 4 };
                    let st = state(id, s.tab == Kind::Manual);
                    sl.tab_manual.draw(canvas, &sheet, base + st, a)
                }
                TAB_AUTO => sl
                    .tab_other
                    .draw(canvas, &sheet, state(id, s.tab == Kind::Auto), a),
                TAB_QUICK => {
                    sl.tab_other
                        .draw(canvas, &sheet, 4 + state(id, s.tab == Kind::Quick), a)
                }
                _ => {}
            }
        }
        for row in 0..SAVE_ROWS {
            let Some(item) = s.item(row) else { break };
            let id = ROW + row as i32;
            let Some(a) = area(&self.save_load, id) else {
                continue;
            };
            // Rows have no disabled look.
            let st = state(id, false).min(2);
            let (x, y) = (a.left, a.top);
            match item {
                Item::Create => {
                    canvas.draw_part(&sheet, sl.row.part(st), x, y, false, 255);
                }
                Item::Slot(r) => {
                    let cell = if r.newest { 6 } else { 3 } + st;
                    draw_row_cell(canvas, &sheet, sl.row.part(cell), sl.field_fill, x, y);
                    if let Some(t) = &r.thumb {
                        canvas.draw(t, x + ROW_THUMB.0, y + ROW_THUMB.1, false);
                    }
                    if let Some(tr) = text {
                        let label = self.slot_label(r);
                        let lx = if r.newest { sl.label_x.1 } else { sl.label_x.0 };
                        tr.draw_plain(canvas, &label, x as f32 + lx, y as f32 + 33.0, 18.0, sl.ink);
                        if let Some(t) = &r.text {
                            let line = first_line(tr, t, 16.0, ROW_FIELD.2 as f32 - 16.0);
                            tr.draw_plain(
                                canvas,
                                &line,
                                x as f32 + ROW_FIELD.0 as f32 + 8.0,
                                y as f32 + 64.0,
                                16.0,
                                sl.ink,
                            );
                        }
                    }
                    let date = format_time(r.saved_at);
                    let right = x + ROW_FIELD.0 as i32 + ROW_FIELD.2 as i32;
                    draw_digits(canvas, &sheet, sl, &date, right, y + 18);
                    if let Some(a) = area(&self.save_load, DELETE + row as i32) {
                        sl.delete
                            .draw(canvas, &sheet, state(DELETE + row as i32, false), a);
                    }
                }
            }
        }
        if area(&self.save_load, SCROLL).is_some() {
            match sl.gem {
                Some(gem) => {
                    let pressed = ui.pressed == Some(Button::Screen(SCROLL));
                    let hover = ui.hover == Some(Button::Screen(SCROLL));
                    // At the top while there is nothing to scroll.
                    let max = s.max_scroll().max(1);
                    draw_gem_thumb(canvas, &sheet, gem, sl.track, s.scroll, max, pressed, hover);
                }
                None => draw_scroll_thumb(canvas, sl.track, s.scroll, s.len(), SAVE_ROWS),
            }
        }
        if let (Some(tr), Some(Button::Screen(id))) = (text, ui.hover) {
            if let Some(help) = self.help_line(s, id) {
                self.draw_help(tr, canvas, &help, sl.help_pos, sl.help_ink);
            }
        }
        Ok(())
    }

    fn help(&self, index: usize) -> Option<&str> {
        self.help.as_ref()?.line(self.language, index)
    }

    /// A help line: plain ink, or white with an edge on the pink bar of
    /// きまぐれテンプテーション2.
    fn draw_help(
        &self,
        tr: &TextRenderer,
        canvas: &mut Canvas,
        line: &str,
        pos: (f32, f32),
        ink: [u8; 4],
    ) {
        match self.kimaten2 {
            true => tr.draw_outlined(canvas, line, pos.0, pos.1, HELP_SIZE, ink, K2_INK),
            false => tr.draw_plain(canvas, line, pos.0, pos.1, HELP_SIZE, ink),
        }
    }

    /// Row title: "Save Data 001", "Auto Save #01", ...
    fn slot_label(&self, r: &SlotRow) -> String {
        let index = match r.kind {
            Kind::Quick => 19,
            Kind::Auto => 20,
            Kind::Manual => 21,
        };
        match self.help(index) {
            Some(f) => format_number(f, r.number),
            None => r.slot.clone(),
        }
    }

    /// Help line for the element under the pointer.
    fn help_line(&self, s: &SaveLoad, id: i32) -> Option<String> {
        use save_ids::*;
        let by_kind = |quick, auto, manual, r: &SlotRow| match r.kind {
            Kind::Quick => quick,
            Kind::Auto => auto,
            Kind::Manual => manual,
        };
        let (index, number) = match id {
            SAVE_MODE => (0, 0),
            LOAD_MODE => (1, 0),
            DELETE_ALL => (2, 0),
            BACK if s.mode == Mode::Load => (3, 0),
            BACK => (4, 0),
            UP => (5, 0),
            DOWN => (6, 0),
            SCROLL => (7, 0),
            TAB_MANUAL => (8, 0),
            TAB_AUTO => (9, 0),
            TAB_QUICK => (10, 0),
            DELETE..=6 => match s.item((id - DELETE) as usize)? {
                Item::Slot(r) => (by_kind(16, 17, 18, r), r.number),
                Item::Create => return None,
            },
            _ if id >= ROW => match s.item((id - ROW) as usize)? {
                Item::Create => (14, 0),
                Item::Slot(r) if s.mode == Mode::Save => (15, r.number),
                Item::Slot(r) => (by_kind(11, 12, 13, r), r.number),
            },
            _ => return None,
        };
        Some(format_number(self.help(index)?, number))
    }

    // Backlog.

    pub fn hit_backlog(&self, b: &Backlog, log: &[LogEntry], x: i32, y: i32) -> Option<Button> {
        use log_ids::*;
        let hit = find(&self.backlog, x, y)?;
        let Button::Screen(id) = hit else { return None };
        let entry = match id {
            _ if (TEXT..TEXT + LOG_ROWS as i32).contains(&id) => return None,
            _ if (VOICE..VOICE + LOG_ROWS as i32).contains(&id) => Some((id - VOICE, true)),
            _ if (CHECK..CHECK + LOG_ROWS as i32).contains(&id) => Some((id - CHECK, true)),
            _ if (JUMP..JUMP + LOG_ROWS as i32).contains(&id) => Some((id - JUMP, false)),
            _ => None,
        };
        if let Some((i, needs_voice)) = entry {
            let e = log.get(b.top + i as usize)?;
            if needs_voice && e.voice.is_none() {
                return None;
            }
        }
        Some(hit)
    }

    pub fn backlog_enabled(b: &Backlog, log: &[LogEntry], id: i32) -> bool {
        use log_ids::*;
        match id {
            UP | PAGE_UP => b.top > 0,
            DOWN | PAGE_DOWN => b.top < Backlog::max_top(log.len()),
            PLAY_MARKED => log.iter().any(|e| e.marked),
            _ => true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_backlog(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        b: &Backlog,
        log: &[LogEntry],
        playing: bool,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        use log_ids::*;
        if self.kimaten2 {
            return self.draw_backlog_kimaten2(canvas, assets, ui, b, log, playing, text);
        }
        canvas.draw(&*self.sheet(assets, "log_bg.akb")?, 0, 0, false);
        let sheet = self.sheet(assets, "log_p.akb")?;
        let nameplate = self.sheet(assets, "nameplate.akb")?;
        let state = |id: i32, active: bool| {
            let btn = Button::Screen(id);
            ui.state(btn, Self::backlog_enabled(b, log, id), active) as u32
        };
        for a in &self.backlog {
            let id = a.id;
            match id {
                UP => LOG_ARROW.draw(canvas, &sheet, state(id, false), a),
                DOWN => LOG_ARROW.draw(canvas, &sheet, 4 + state(id, false), a),
                PAGE_UP => LOG_PAGE.draw(canvas, &sheet, state(id, false), a),
                PAGE_DOWN => LOG_PAGE.draw(canvas, &sheet, 4 + state(id, false), a),
                PLAY_MARKED => LOG_PLAY.draw(canvas, &sheet, state(id, playing), a),
                BACK => LOG_BACK.draw(canvas, &sheet, state(id, false), a),
                _ => {}
            }
        }
        for i in 0..LOG_ROWS as i32 {
            let Some(e) = log.get(b.top + i as usize) else {
                break;
            };
            if let Some(a) = area(&self.backlog, TEXT + i) {
                let mut baseline = a.top as f32 + 40.0;
                if let Some(row) = e.speaker {
                    let (w, h) = NAMEPLATE;
                    let plate = inked(&nameplate, (0, h * row as u32, w, h));
                    canvas.draw(&plate, a.left, a.top, false);
                    baseline = a.top as f32 + h as f32 + 30.0;
                }
                if let Some(tr) = text {
                    let width = a.width() as f32 - 24.0;
                    let lines = tr.wrap(&e.text, LOG_TEXT_SIZE, width);
                    for line in lines.iter().take(4) {
                        if baseline > a.bottom as f32 {
                            break;
                        }
                        tr.draw_plain(
                            canvas,
                            line,
                            a.left as f32 + 12.0,
                            baseline,
                            LOG_TEXT_SIZE,
                            INK,
                        );
                        baseline += LOG_LINE;
                    }
                }
            }
            if e.voice.is_some() {
                if let Some(a) = area(&self.backlog, VOICE + i) {
                    LOG_VOICE.draw(canvas, &sheet, state(VOICE + i, false), a);
                }
                if let Some(a) = area(&self.backlog, CHECK + i) {
                    let row = if e.marked {
                        3
                    } else {
                        state(CHECK + i, false).min(2)
                    };
                    LOG_CHECK.draw(canvas, &sheet, row, a);
                }
            }
            if let Some(a) = area(&self.backlog, JUMP + i) {
                LOG_JUMP.draw(canvas, &sheet, state(JUMP + i, false), a);
            }
        }
        draw_scroll_thumb(canvas, self.log_track(), b.top, log.len(), LOG_ROWS);
        Ok(())
    }

    fn log_track(&self) -> (i32, i32, i32, i32) {
        match self.kimaten2 {
            true => K_LOG_TRACK,
            false => LOG_TRACK,
        }
    }

    /// The backlog of きまぐれテンプテーション2: `logbg.akb` and the heart
    /// buttons of `log_p.akb`, white text with a dark edge (measured on a
    /// PC screenshot of the Chinese patch). The speaker's name goes on a
    /// line of its own above the text (not seen on a screenshot yet).
    #[allow(clippy::too_many_arguments)]
    fn draw_backlog_kimaten2(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        b: &Backlog,
        log: &[LogEntry],
        playing: bool,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        use log_ids::*;
        canvas.draw(&*assets.image("logbg.akb")?, 0, 0, false);
        let sheet = assets.image("log_p.akb")?;
        let state = |id: i32, active: bool| {
            let btn = Button::Screen(id);
            ui.state(btn, Self::backlog_enabled(b, log, id), active) as u32
        };
        for a in &self.backlog {
            let id = a.id;
            match id {
                UP => K_LOG_ARROW.draw(canvas, &sheet, state(id, false), a),
                DOWN => K_LOG_ARROW.draw(canvas, &sheet, 4 + state(id, false), a),
                PAGE_UP => K_LOG_PAGE_UP.draw(canvas, &sheet, state(id, false), a),
                PAGE_DOWN => K_LOG_PAGE_DOWN.draw(canvas, &sheet, state(id, false), a),
                PLAY_MARKED => K_LOG_PLAY.draw(canvas, &sheet, state(id, playing), a),
                BACK => K_LOG_BACK.draw(canvas, &sheet, state(id, false), a),
                _ => {}
            }
        }
        for i in 0..LOG_ROWS as i32 {
            let Some(e) = log.get(b.top + i as usize) else {
                break;
            };
            if let (Some(a), Some(tr)) = (area(&self.backlog, TEXT + i), text) {
                let mut lines = Vec::new();
                tr.with_wide_advance(K_LOG_ADVANCE / K_LOG_TEXT_SIZE, |tr| {
                    if let Some(name) = &e.speaker_name {
                        lines.push(name.clone());
                    }
                    let width = a.width() as f32 - 2.0 * K_LOG_TEXT_LEFT;
                    lines.extend(tr.wrap(&e.text, K_LOG_TEXT_SIZE, width));
                    let mut baseline = a.top as f32 + K_LOG_BASELINE;
                    for line in &lines {
                        if baseline > a.bottom as f32 {
                            break;
                        }
                        tr.draw_outlined(
                            canvas,
                            line,
                            a.left as f32 + K_LOG_TEXT_LEFT,
                            baseline,
                            K_LOG_TEXT_SIZE,
                            [255, 255, 255, 255],
                            [40, 30, 34, 255],
                        );
                        baseline += K_LOG_LINE;
                    }
                });
            }
            if e.voice.is_some() {
                if let Some(a) = area(&self.backlog, VOICE + i) {
                    K_LOG_VOICE.draw(canvas, &sheet, state(VOICE + i, false), a);
                }
                if let Some(a) = area(&self.backlog, CHECK + i) {
                    let row = if e.marked {
                        3
                    } else {
                        state(CHECK + i, false).min(2)
                    };
                    K_LOG_CHECK.draw(canvas, &sheet, row, a);
                }
            }
            if let Some(a) = area(&self.backlog, JUMP + i) {
                K_LOG_JUMP.draw(canvas, &sheet, state(JUMP + i, false), a);
            }
        }
        // The thumb is a gem that runs down the track.
        let max = Backlog::max_top(log.len());
        let pressed = ui.pressed == Some(Button::Screen(SCROLL));
        let hover = ui.hover == Some(Button::Screen(SCROLL));
        draw_gem_thumb(
            canvas,
            &sheet,
            K_LOG_GEM,
            K_LOG_TRACK,
            b.top,
            max,
            pressed,
            hover,
        );
        Ok(())
    }

    /// Position on a scroll bar track as a fraction, if (x, y) is on it.
    pub fn scroll_fraction(&self, backlog: bool, x: i32, y: i32) -> Option<f32> {
        let (areas, id, track) = if backlog {
            (&self.backlog, log_ids::SCROLL, self.log_track())
        } else {
            (&self.save_load, save_ids::SCROLL, self.save_style().track)
        };
        area(areas, id).filter(|a| a.contains(x, y))?;
        Some(((y - track.1) as f32 / track.3 as f32).clamp(0.0, 1.0))
    }

    // Dialog.

    pub fn hit_dialog(&self, d: &Dialog, x: i32, y: i32) -> Option<Button> {
        let unlock = matches!(d.action, Action::Unlock(_));
        self.dialog
            .iter()
            .filter(|a| !(unlock && a.id == dialog_ids::DONT_ASK))
            .find(|a| a.contains(x, y))
            .map(|a| Button::Dialog(a.id))
    }

    pub fn draw_dialog(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        d: &Dialog,
    ) -> Result<()> {
        use dialog_ids::*;
        canvas.fill(0, 0, canvas.width, canvas.height, [0, 0, 0, 110]);
        let ds = match self.kimaten2 {
            true => &KIMATEN2_DIALOG,
            false => &FUYUKURU_DIALOG,
        };
        let sheet = match ds.localized {
            true => self.sheet(assets, ds.sheet)?,
            false => assets.image(ds.sheet)?,
        };
        if let Action::Unlock(k) = d.action {
            // The box without the question bar, "!注意!" centred, then
            // three small lines from the left.
            canvas.draw_part(
                &sheet,
                K2_UNLOCK_BOX,
                ds.box_pos.0,
                ds.box_pos.1,
                false,
                255,
            );
            let (ny, nh) = K2_UNLOCK_NOTE;
            if let Some((l, r)) = opaque_columns(&sheet, ds.question.0, ny, ds.question.2, nh) {
                let x = ds.question_center - (r - l) as i32 / 2;
                canvas.draw_part(&sheet, (l, ny, r - l, nh), x, K2_UNLOCK_NOTE_Y, false, 255);
            }
            let (bx, by, bw, bh) = K2_UNLOCK_TEXT;
            let part = (bx, by + bh * k as u32, bw, bh);
            canvas.draw_part(
                &sheet,
                part,
                K2_UNLOCK_TEXT_POS.0,
                K2_UNLOCK_TEXT_POS.1,
                false,
                255,
            );
        } else {
            canvas.draw_part(&sheet, ds.box_part, ds.box_pos.0, ds.box_pos.1, false, 255);
            // Question: centre the drawn part of its strip.
            let (qx, qy, qw, qh) = ds.question;
            let qy = qy + ds.question_pitch * d.action.question() as u32;
            if let Some((l, r)) = opaque_columns(&sheet, qx, qy, qw, qh) {
                let w = r - l;
                let x = ds.question_center - w as i32 / 2;
                canvas.draw_part(&sheet, (l, qy, w, qh), x, ds.question_y, false, 255);
            }
        }
        for a in &self.dialog {
            let st = ui.state(Button::Dialog(a.id), true, false) as u32;
            match a.id {
                YES => ds.yes.draw(canvas, &sheet, st.min(2), a),
                NO => ds.no.draw(canvas, &sheet, st.min(2), a),
                DONT_ASK if matches!(d.action, Action::Unlock(_)) => {}
                DONT_ASK => {
                    let row = if d.dont_ask { 3 } else { st.min(2) };
                    let y = a.top + (a.height() as i32 - ds.check.h as i32) / 2;
                    canvas.draw_part(&sheet, ds.check.part(row), a.left, y, false, 255);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Draws a save row, leaving out the black text field of the sheet: the
/// field is filled with the row colour next to it.
fn draw_row_cell(
    canvas: &mut Canvas,
    sheet: &Image,
    part: (u32, u32, u32, u32),
    fill: u32,
    x: i32,
    y: i32,
) {
    let (sx, sy, w, h) = part;
    let (fx, fy, fw, fh) = ROW_FIELD;
    let pieces = [
        (0, 0, fx, h),
        (fx + fw, 0, w - fx - fw, h),
        (fx, 0, fw, fy),
        (fx, fy + fh, fw, h - fy - fh),
    ];
    for (px, py, pw, ph) in pieces {
        canvas.draw_part(
            sheet,
            (sx + px, sy + py, pw, ph),
            x + px as i32,
            y + py as i32,
            false,
            255,
        );
    }
    for col in 0..fw {
        canvas.draw_part(
            sheet,
            (sx + fill, sy + fy, 1, fh),
            x + (fx + col) as i32,
            y + fy as i32,
            false,
            255,
        );
    }
}

/// Draws a date in the small digits of the sheet, right-aligned at `right`.
fn draw_digits(canvas: &mut Canvas, sheet: &Image, sl: &SaveStyle, s: &str, right: i32, y: i32) {
    let cell = |c: char| -> Option<(u32, u32)> {
        match c {
            '/' => Some(sl.slash),
            ':' => Some(sl.colon),
            '0'..='9' => {
                let (x, w, pitch) = sl.digits;
                Some((x + pitch * (c as u32 - '0' as u32), w))
            }
            _ => None,
        }
    };
    let width: i32 = s
        .chars()
        .map(|c| cell(c).map_or(6, |(_, w)| w as i32))
        .sum();
    let mut x = right - width;
    for c in s.chars() {
        match cell(c) {
            Some((sx, w)) => {
                canvas.draw_part(sheet, (sx, sl.digit_y, w, sl.digit_h), x, y, false, 255);
                x += w as i32;
            }
            None => x += 6,
        }
    }
}

/// A gem thumb running down a scroll track (きまぐれテンプテーション2).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_gem_thumb(
    canvas: &mut Canvas,
    sheet: &Image,
    gem: Cells,
    track: (i32, i32, i32, i32),
    pos: usize,
    max: usize,
    pressed: bool,
    hover: bool,
) {
    let (x, y, w, h) = track;
    let travel = h - gem.h as i32;
    let top = y + if max == 0 {
        travel
    } else {
        travel * pos.min(max) as i32 / max as i32
    };
    let row = match (pressed, hover) {
        (true, _) => 2,
        (_, true) => 1,
        _ => 0,
    };
    let left = x + (w - gem.w as i32) / 2;
    canvas.draw_part(sheet, gem.part(row), left, top, false, 255);
}

pub(crate) fn draw_scroll_thumb(
    canvas: &mut Canvas,
    track: (i32, i32, i32, i32),
    pos: usize,
    len: usize,
    shown: usize,
) {
    let (x, y, w, h) = track;
    if len <= shown {
        canvas.fill(x, y, w as u32, h as u32, THUMB_COLOR);
        return;
    }
    let size = (h as usize * shown / len).max(40) as i32;
    let max = len - shown;
    let top = y + ((h - size) as usize * pos / max) as i32;
    canvas.fill(x, top, w as u32, size as u32, THUMB_COLOR);
}

/// A part of a nameplate with its white lettering turned to ink colour:
/// the plates are made for the dark message window, the backlog is light.
fn inked(img: &Image, part: (u32, u32, u32, u32)) -> Image {
    let (x, y, w, h) = part;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for row in y..(y + h).min(img.height) {
        for col in x..(x + w).min(img.width) {
            let i = ((row * img.width + col) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            let light = p[0].min(p[1]).min(p[2]);
            if light > 180 {
                rgba.extend_from_slice(&[INK[0], INK[1], INK[2], p[3]]);
            } else {
                rgba.extend_from_slice(p);
            }
        }
    }
    let w = (rgba.len() / 4) as u32 / h.max(1);
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// Left and right end of the opaque columns of a sheet region.
fn opaque_columns(img: &Image, x: u32, y: u32, w: u32, h: u32) -> Option<(u32, u32)> {
    let opaque = |cx: u32| {
        (y..(y + h).min(img.height))
            .any(|cy| img.rgba[((cy * img.width + cx) * 4 + 3) as usize] > 8)
    };
    let cols = x..(x + w).min(img.width);
    let l = cols.clone().find(|&c| opaque(c))?;
    let r = cols.rev().find(|&c| opaque(c))? + 1;
    Some((l, r))
}

/// First line of a text, with an ellipsis if it goes on.
fn first_line(tr: &TextRenderer, text: &str, size: f32, width: f32) -> String {
    let lines = tr.wrap(text, size, width - size);
    match lines.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [first, ..] => format!("{first}…"),
    }
}

/// Fills a `%02u` or `%03u` in a help line.
pub(crate) fn format_number(f: &str, n: u32) -> String {
    f.replace("%02u", &format!("{n:02}"))
        .replace("%03u", &format!("{n:03}"))
}

/// Local date and time, as `2026/11/06 12:34`.
fn format_time(secs: u64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(secs as i64, 0).single() {
        Some(t) => t.format("%Y/%m/%d %H:%M").to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_numbers() {
        assert_eq!(format_number("Save Data %03u", 7), "Save Data 007");
        assert_eq!(format_number("Quick Save #%02u", 1), "Quick Save #01");
    }

    #[test]
    fn save_list_items() {
        let row = |n| SlotRow {
            slot: format!("save{n:03}"),
            kind: Kind::Manual,
            number: n,
            saved_at: 0,
            text: None,
            thumb: None,
            newest: false,
        };
        let mut s = SaveLoad {
            mode: Mode::Save,
            tab: Kind::Manual,
            scroll: 0,
            rows: (1..=8).map(row).collect(),
            can_save: true,
        };
        assert_eq!(s.len(), 9);
        assert!(matches!(s.item(0), Some(Item::Create)));
        s.scroll_by(10);
        assert_eq!(s.scroll, 2);
        assert!(matches!(s.item(6), Some(Item::Slot(r)) if r.number == 8));
        s.mode = Mode::Load;
        s.scroll = 0;
        assert!(matches!(s.item(0), Some(Item::Slot(r)) if r.number == 1));
    }
}
