//! Engine-drawn interface: the title menu buttons, the button bar of the
//! message window, and the click-wait indicator.
//!
//! The scripts only ask for the title menu (game command 110) and show
//! text; the buttons themselves are drawn and hit-tested by the engine. The
//! title layout below is copied from tables in the game executable; the
//! message window buttons come from `mes_o.area` and `message_p.akb`.

use std::sync::Arc;
use silky_format::area::{self, Area};

use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::text::TextRenderer;
use crate::Result;

/// A clickable element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// Title menu entry, see [`title`].
    Title(usize),
    /// Message window button, by its `mes_o.area` id, see [`bar`].
    Bar(i32),
    /// Option of a selection.
    Choice(usize),
    /// Button of きまぐれテンプテーション2's action menu, by index.
    Action(usize),
    /// Button of きまぐれテンプテーション2's map, by its area id: places
    /// 0.., panel buttons 256.., back 512.
    Map(i32),
    /// Element of an engine screen (save/load, backlog), by area id.
    Screen(i32),
    /// Yes, no or "do not ask again" of a dialog, by `dialog.area` id.
    Dialog(i32),
}

/// Title menu entries: 0..6 on the main menu, 6..10 on the extras menu.
pub mod title {
    pub const NEW_GAME: usize = 0;
    pub const CONTINUE: usize = 1;
    pub const LOAD: usize = 2;
    pub const SETTINGS: usize = 3;
    pub const EXTRAS: usize = 4;
    pub const QUIT: usize = 5;
    pub const CG_MODE: usize = 6;
    pub const SCENE_MODE: usize = 7;
    pub const MUSIC_MODE: usize = 8;
    pub const BACK: usize = 9;
    /// きまぐれテンプテーション2's extras page also has アンネViewer,
    /// 捜査手帳 and アンネの部屋.
    pub const VIEWER: usize = 10;
    pub const NOTEBOOK: usize = 11;
    pub const ROOM: usize = 12;

    /// きまぐれテンプテーション2: extras entries and the byte variable that
    /// unlocks each (from the executable's title menu, 0x4be440): music
    /// mode, アンネViewer, 捜査手帳, アンネの部屋. The endings set 3007.
    pub const KIMATEN2_UNLOCK: [(usize, usize); 4] = [
        (MUSIC_MODE, 3004),
        (VIEWER, 3007),
        (NOTEBOOK, 3006),
        (ROOM, 3007),
    ];

    /// Value the title menu call returns to the script for an entry, if
    /// the script handles it. Continue, Load and Settings open engine
    /// screens instead; Extras and Back switch the menu page.
    pub fn result(entry: usize) -> Option<i32> {
        Some(match entry {
            NEW_GAME => 1,
            QUIT => 5,
            CG_MODE => 257,
            SCENE_MODE => 258,
            MUSIC_MODE => 259,
            VIEWER => 260,
            NOTEBOOK => 261,
            ROOM => 262,
            _ => return None,
        })
    }
}

/// Message window buttons by `mes_o.area` id.
pub mod bar {
    pub const SAVE: i32 = 0;
    pub const LOAD: i32 = 1;
    pub const QUICK_SAVE: i32 = 2;
    pub const QUICK_LOAD: i32 = 3;
    pub const VOICE: i32 = 4;
    pub const PREVIOUS_CHOICE: i32 = 5;
    pub const BACKLOG: i32 = 6;
    pub const AUTO: i32 = 7;
    pub const SKIP: i32 = 8;
    pub const NEXT_CHOICE: i32 = 9;
    pub const CONFIG: i32 = 10;
    pub const TITLE: i32 = 11;
    pub const QUIT: i32 = 12;
    pub const HIDE: i32 = 13;
    pub const CLOSE: i32 = 14;
    /// きまぐれテンプテーション2 only: the side menu button, the costume
    /// screen, the notebook and collection, loop the voice, skip unread
    /// text too, and the pin of the button bar (not done yet).
    pub const MENU: i32 = 15;
    pub const PIN: i32 = 16;
    pub const COSTUME: i32 = 17;
    pub const LOOP_VOICE: i32 = 18;
    pub const SKIP_ALL: i32 = 19;
    pub const NOTEBOOK: i32 = 20;
    pub const COLLECTION: i32 = 21;
}

/// Interface state kept by the frontend.
#[derive(Debug, Clone, Default)]
pub struct Ui {
    /// The title menu is up; `true` shows the extras page.
    pub title: Option<bool>,
    pub hover: Option<Button>,
    pub pressed: Option<Button>,
    /// Message window hidden by the player (right click or the hide
    /// button); any click shows it again.
    pub window_hidden: bool,
    pub auto: bool,
    pub skip: bool,
    /// Skip goes through unread text too (きまぐれテンプテーション2's
    /// skip-all button).
    pub skip_all: bool,
    /// The voice of the message plays again each time it ends
    /// (きまぐれテンプテーション2's voice loop button).
    pub loop_voice: bool,
    /// A message waits for a click: the indicator animates.
    pub waiting: bool,
    /// Animation clock in milliseconds.
    pub clock: u64,
    /// Whether there are saves to load (enables Continue and Load).
    pub has_saves: bool,
    /// Options of a pending selection, as shown.
    pub choices: Vec<String>,
    /// There is a selection to go back to.
    pub previous_choice: bool,
    /// きまぐれテンプテーション2's side menu is open.
    pub side_menu: bool,
    /// Buttons of きまぐれテンプテーション2's action menu, while it is up.
    pub actions: Vec<crate::ActionButton>,
    /// きまぐれテンプテーション2's map, while it is up.
    pub map: Option<crate::kimaten2::map::Map>,
    /// きまぐれテンプテーション2: some notebook entry, some collection
    /// item is open (enables their buttons).
    pub notebook: bool,
    pub collection: bool,
    /// Title menu entries shown gray and not clickable (きまぐれテンプテーション2's
    /// extras before the game unlocks them).
    pub locked_titles: Vec<usize>,
    /// Clock when the current title menu page came up (its buttons jump
    /// in from below in きまぐれテンプテーション2).
    pub title_shown: u64,
}

/// Button state, as the row of the sprite sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Normal = 0,
    Hover = 1,
    Pressed = 2,
    Disabled = 3,
}

impl Ui {
    pub(crate) fn state(&self, b: Button, enabled: bool, active: bool) -> State {
        if !enabled {
            State::Disabled
        } else if self.pressed == Some(b) || active {
            State::Pressed
        } else if self.hover == Some(b) {
            State::Hover
        } else {
            State::Normal
        }
    }

    /// Whether a button does anything right now.
    pub fn enabled(&self, b: Button) -> bool {
        match b {
            Button::Title(t) if self.locked_titles.contains(&t) => false,
            Button::Title(title::CONTINUE | title::LOAD) => self.has_saves,
            Button::Bar(bar::LOAD | bar::QUICK_LOAD) => self.has_saves,
            Button::Bar(bar::PREVIOUS_CHOICE) => self.previous_choice,
            Button::Bar(bar::NOTEBOOK) => self.notebook,
            Button::Bar(bar::COLLECTION) => self.collection,
            _ => true,
        }
    }
}

/// Title menu layout of one language: where each entry is on the sheet,
/// where it goes on screen, and its size. Entries are as in [`title`].
struct TitleLayout {
    src: [(u32, u32); 10],
    dst: [(i32, i32); 10],
    size: [(u32, u32); 10],
}

/// From the executable, in its order: English, Simplified Chinese,
/// Traditional Chinese, Japanese (the Japanese one matches `title.area`).
/// Each sheet has four rows per entry: normal, hover, pressed, disabled,
/// one entry height apart.
const TITLE_LAYOUTS: [TitleLayout; 4] = [
    TitleLayout {
        src: [
            (0, 0),
            (177, 0),
            (331, 0),
            (507, 0),
            (652, 0),
            (772, 0),
            (0, 272),
            (193, 272),
            (437, 272),
            (681, 272),
        ],
        dst: [
            (130, 636),
            (322, 636),
            (491, 636),
            (683, 636),
            (844, 636),
            (980, 636),
            (210, 628),
            (427, 628),
            (695, 628),
            (963, 638),
        ],
        size: [
            (176, 68),
            (153, 68),
            (176, 68),
            (145, 68),
            (120, 68),
            (170, 68),
            (193, 78),
            (244, 78),
            (244, 78),
            (107, 68),
        ],
    },
    TitleLayout {
        src: [
            (0, 0),
            (150, 0),
            (344, 0),
            (489, 0),
            (632, 0),
            (744, 0),
            (0, 272),
            (163, 272),
            (339, 272),
            (511, 272),
        ],
        dst: [
            (155, 636),
            (321, 636),
            (531, 636),
            (692, 636),
            (851, 636),
            (979, 636),
            (295, 628),
            (482, 628),
            (682, 628),
            (878, 638),
        ],
        size: [
            (150, 68),
            (194, 68),
            (145, 68),
            (143, 68),
            (112, 68),
            (146, 68),
            (163, 78),
            (176, 78),
            (172, 78),
            (107, 68),
        ],
    },
    // Traditional Chinese: same as Simplified.
    TitleLayout {
        src: [
            (0, 0),
            (150, 0),
            (344, 0),
            (489, 0),
            (632, 0),
            (744, 0),
            (0, 272),
            (163, 272),
            (339, 272),
            (511, 272),
        ],
        dst: [
            (155, 636),
            (321, 636),
            (531, 636),
            (692, 636),
            (851, 636),
            (979, 636),
            (295, 628),
            (482, 628),
            (682, 628),
            (878, 638),
        ],
        size: [
            (150, 68),
            (194, 68),
            (145, 68),
            (143, 68),
            (112, 68),
            (146, 68),
            (163, 78),
            (176, 78),
            (172, 78),
            (107, 68),
        ],
    },
    TitleLayout {
        src: [
            (0, 0),
            (155, 0),
            (358, 0),
            (496, 0),
            (1027, 0),
            (1177, 0),
            (0, 272),
            (193, 272),
            (437, 272),
            (681, 272),
        ],
        dst: [
            (73, 636),
            (243, 636),
            (459, 636),
            (607, 636),
            (776, 636),
            (940, 636),
            (195, 628),
            (408, 628),
            (672, 628),
            (936, 638),
        ],
        size: [
            (155, 68),
            (203, 68),
            (138, 68),
            (156, 68),
            (150, 68),
            (225, 68),
            (193, 78),
            (244, 78),
            (244, 78),
            (150, 68),
        ],
    },
];

fn title_layout(lang: Language) -> &'static TitleLayout {
    &TITLE_LAYOUTS[lang.map_section()]
}

fn title_entries(extras: bool) -> std::ops::Range<usize> {
    if extras {
        6..10
    } else {
        0..6
    }
}

/// Message window button cells on `message_p.akb`: 13 bar buttons of
/// 37 x 41 side by side, rows by state.
const BAR_SHEET: (u32, u32) = (776, 440);
const BAR_CELL: (u32, u32) = (37, 41);
/// Hide (down arrow) and close (cross) cells: 41 x 43, rows by state. The
/// arrow column holds three up arrows, then three down arrows.
const HIDE_SHEET: (u32, u32) = (695, 569);
const CLOSE_SHEET: (u32, u32) = (737, 440);
const ROUND_CELL: (u32, u32) = (41, 43);
/// `mes_o.area` gives the close button relative to the window top.
const CLOSE_AREA_TOP: i32 = 500;
/// Click-wait hourglass: 10 frames of 67 x 67 down the sheet, drawn at the
/// right edge of the window (measured on screenshots).
const WAIT_SHEET: (u32, u32) = (1347, 0);
const WAIT_CELL: u32 = 67;
const WAIT_FRAMES: u32 = 10;
const WAIT_POS: (i32, i32) = (1215, 607);
const WAIT_FRAME_MS: u64 = 100;

/// Selection options: bars of 692 x 76 on the sheet, rows by state.
/// Their place on screen is provisional (no screenshot of a selection).
const CHOICE_SHEET: (u32, u32) = (2, 440);
const CHOICE_CELL: (u32, u32) = (692, 76);
const CHOICE_LEFT: i32 = (1280 - 692) / 2;
const CHOICE_CENTER_Y: i32 = 300;
const CHOICE_PITCH: i32 = 90;
const CHOICE_TEXT_LEFT: f32 = 120.0;
const CHOICE_TEXT_SIZE: f32 = 21.0;

fn choice_top(i: usize, n: usize) -> i32 {
    CHOICE_CENTER_Y - CHOICE_PITCH * n as i32 / 2 + CHOICE_PITCH * i as i32
}

/// Title menu of きまぐれテンプテーション2: click areas from `title.area`
/// (main page) and `title2.area` (extras), cells of `title_p.akb` side by
/// side in area order, four 63-pixel state rows per block.
struct Kimaten2Title {
    /// (entry, screen area, sheet cell top left).
    main: Vec<(usize, Area, (u32, u32))>,
    extras: Vec<(usize, Area, (u32, u32))>,
}

/// How long the title buttons of きまぐれテンプテーション2 take to jump in:
/// the last of seven starts after 600 ms and settles within two seconds.
pub const TITLE_JUMP_MS: u64 = 2600;

/// Height of a title button of きまぐれテンプテーション2 `ms` after its
/// jump starts, from the executable (0x4bed40, one step per 60 Hz frame):
/// it leaves y = 720 at -16 pixels a frame, falls back by 0.98 a frame,
/// and bounces on its place (y = 636) at half its speed until it stops.
fn title_jump_y(ms: u64) -> f32 {
    const PLACE: f32 = 636.0;
    let (mut y, mut v) = (720.0f32, -16.0f32);
    for _ in 0..ms * 60 / 1000 {
        v += 0.98;
        y += v;
        if v > 0.0 && y >= PLACE {
            y = PLACE;
            v *= -0.5;
            if v as i32 == 0 {
                return PLACE;
            }
        }
    }
    y
}

impl Kimaten2Title {
    const ROW: u32 = 63;

    fn load(assets: &Assets) -> Result<Self> {
        let main_areas = area::parse(&assets.data("title.area")?)?;
        let extra_areas = area::parse(&assets.data("title2.area")?)?;
        // Sheet blocks: the main entries from (0, 0); CG, scene, music and
        // back from (0, 252); viewer, notebook and room from (490, 656).
        let mut main = Vec::new();
        let mut x = 0;
        for a in main_areas.iter().filter(|a| (0..6).contains(&a.id)) {
            main.push((a.id as usize, *a, (x, 0)));
            x += a.width();
        }
        let find = |id: i32| extra_areas.iter().find(|a| a.id == id).copied();
        let mut extras = Vec::new();
        for (block, ids) in [
            (
                (0, 252),
                [
                    (256, title::CG_MODE),
                    (257, title::SCENE_MODE),
                    (258, title::MUSIC_MODE),
                    (262, title::BACK),
                ]
                .as_slice(),
            ),
            (
                (490, 656),
                [
                    (259, title::VIEWER),
                    (260, title::NOTEBOOK),
                    (261, title::ROOM),
                ]
                .as_slice(),
            ),
        ] {
            let mut x = block.0;
            for &(id, entry) in ids {
                if let Some(a) = find(id) {
                    extras.push((entry, a, (x, block.1)));
                    x += a.width();
                }
            }
        }
        Ok(Self { main, extras })
    }

    fn entries(&self, extras: bool) -> &[(usize, Area, (u32, u32))] {
        if extras {
            &self.extras
        } else {
            &self.main
        }
    }
}

/// Screen layout of the interface, loaded once per game.
pub struct Layout {
    language: Language,
    /// Message window buttons from `mes_o.area`, close button moved to
    /// screen coordinates.
    bar: Vec<Area>,
    /// Set for きまぐれテンプテーション2, whose interface differs.
    kimaten2: Option<Kimaten2>,
}

/// A button of きまぐれテンプテーション2: its click area, the [`bar`] id it
/// acts as, and its cell on the sheet (state rows `step` apart).
#[derive(Debug, Clone, Copy)]
struct Cell {
    id: i32,
    area: Area,
    sheet: (u32, u32),
    size: (u32, u32),
    step: u32,
    /// Number of state rows; states past the last use the last one.
    states: u32,
}

impl Cell {
    fn draw(&self, canvas: &mut Canvas, sheet: &Arc<silky_format::akb::Image>, ui: &Ui, active: bool) {
        let b = Button::Bar(self.id);
        let state = match ui.state(b, ui.enabled(b), active) {
            // The side menu button shows dimmed while the menu is open.
            State::Pressed if self.id == bar::MENU && ui.side_menu => State::Disabled,
            s => s,
        };
        let state = (state as u32).min(self.states - 1);
        let (w, h) = self.size;
        let part = (self.sheet.0, self.sheet.1 + self.step * state, w, h);
        let dx = self.area.left + (self.area.width() as i32 - w as i32) / 2;
        let dy = self.area.top + (self.area.height() as i32 - h as i32) / 2;
        canvas.draw_part(sheet, part, dx, dy, false, 255);
    }
}

/// Interface of きまぐれテンプテーション2.
struct Kimaten2 {
    title: Kimaten2Title,
    /// Message window buttons (`mes_o.area`, `message_p.akb`).
    bar: Vec<Cell>,
    /// Side menu entries (`mesmenu.area`, `message_menu.akb`).
    menu: Vec<Cell>,
    /// Small buttons under the action menu (`actselect.area`).
    action_bar: Vec<Cell>,
    /// Map areas: `map01.area`, `map02.area` (places, back, small
    /// buttons), `map03.area` (panels of the second map).
    map_areas: [Vec<Area>; 2],
    panel_areas: Vec<Area>,
}

/// Map sheets: places are 304 x 184 cells side by side, four state rows;
/// panel buttons 240 x 88 and back buttons 112 x 72 lower on
/// `map2_p.akb`, with the 探索終了 stamps.
const MAP_PLACE: (u32, u32) = (304, 184);
const MAP_PANEL_X: [u32; 8] = [0, 240, 480, 1440, 720, 960, 1200, 1440];
const MAP_PANEL: (u32, u32) = (240, 88);
const MAP_BACK: (u32, u32) = (112, 72);
const MAP_SHEET_Y: u32 = 920;
const MAP_STAMP: (u32, u32, u32, u32) = (1552, 920, 271, 168);
const MAP_STAMP_SMALL: (u32, u32, u32, u32) = (1552, 1088, 144, 88);
const MAP_BACK_ID: i32 = 512;

/// Action menu buttons (304 x 94) by how many there are, top left corners
/// (from the executable).
const ACTION_SIZE: (u32, u32) = (304, 94);
fn action_positions(n: usize) -> &'static [(i32, i32)] {
    const ONE: [(i32, i32); 1] = [(488, 320)];
    const TWO: [(i32, i32); 2] = [(328, 320), (649, 320)];
    const FOUR: [(i32, i32); 4] = [(328, 240), (649, 240), (328, 357), (649, 357)];
    const SIX: [(i32, i32); 6] = [
        (328, 160),
        (649, 160),
        (328, 277),
        (649, 277),
        (328, 395),
        (649, 395),
    ];
    match n {
        0 => &[],
        1 => &ONE,
        2 => &TWO,
        3 | 4 => &FOUR[..n],
        _ => &SIX[..n.min(6)],
    }
}

/// Cell of an action icon on `select_p.akb` (three state rows 94 apart).
fn action_cell(icon: u32) -> (u32, u32) {
    match icon {
        1..=4 => (304 * (icon - 1), 0),
        5..=8 => (304 * (icon - 5), 282),
        9 => (608, 564),
        10 => (912, 564),
        _ => (912, 846),
    }
}

impl Kimaten2 {
    fn load(assets: &Assets) -> Result<Self> {
        let window = crate::kimaten2::window::TOP;
        let mut bar = Vec::new();
        for a in area::parse(&assets.data("mes_o.area")?)? {
            // Buttons 0..8 left to right: costume, quick save, quick load,
            // voice loop, jump to the next unread text or selection, auto,
            // skip read text, skip all, side menu. 50 x 34 cells from
            // (50, 600), four state rows.
            let ids = [
                bar::COSTUME,
                bar::QUICK_SAVE,
                bar::QUICK_LOAD,
                bar::LOOP_VOICE,
                bar::NEXT_CHOICE,
                bar::AUTO,
                bar::SKIP,
                bar::SKIP_ALL,
                bar::MENU,
            ];
            let cell = match a.id {
                0..=8 => Cell {
                    id: ids[a.id as usize],
                    area: a,
                    sheet: (50 + 50 * a.id as u32, 600),
                    size: (50, 34),
                    step: 34,
                    states: 4,
                },
                // The pin (padlock) left of them: 50 x 38 cells.
                9 => Cell {
                    id: bar::PIN,
                    area: a,
                    sheet: (0, 600),
                    size: (50, 38),
                    step: 38,
                    states: 3,
                },
                // Hide the window: a cross on the window, relative to it.
                10 => Cell {
                    id: bar::HIDE,
                    area: Area {
                        top: a.top + window,
                        bottom: a.bottom + window,
                        ..a
                    },
                    sheet: (1280, 392),
                    size: (36, 36),
                    step: 36,
                    states: 3,
                },
                _ => continue,
            };
            bar.push(cell);
        }
        // Side menu: 264 x 56 entries, three to a row of the sheet, each
        // with four state rows below it.
        let ids = [
            bar::SAVE,
            bar::LOAD,
            bar::CONFIG,
            bar::BACKLOG,
            bar::NOTEBOOK,
            bar::COLLECTION,
            bar::TITLE,
            bar::QUIT,
        ];
        let mut menu = Vec::new();
        for a in area::parse(&assets.data("mesmenu.area")?)? {
            let k = (a.id - 3840) as usize;
            if let Some(&id) = ids.get(k) {
                menu.push(Cell {
                    id,
                    area: a,
                    sheet: (264 * (k % 3) as u32, 224 * (k / 3) as u32),
                    size: (264, 56),
                    step: 56,
                    states: 4,
                });
            }
        }
        // Save, load, quick save, quick load, notebook, collection: 56 x
        // 58 cells of `select_p.akb` from (168, 564), four state rows.
        let ids = [
            bar::SAVE,
            bar::LOAD,
            bar::QUICK_SAVE,
            bar::QUICK_LOAD,
            bar::NOTEBOOK,
            bar::COLLECTION,
        ];
        let mut action_bar = Vec::new();
        for a in area::parse(&assets.data("actselect.area")?)? {
            let k = (a.id - 3840) as usize;
            if let Some(&id) = ids.get(k) {
                action_bar.push(Cell {
                    id,
                    area: a,
                    sheet: (168 + 56 * k as u32, 564),
                    size: (56, 58),
                    step: 58,
                    states: 4,
                });
            }
        }
        Ok(Self {
            title: Kimaten2Title::load(assets)?,
            bar,
            menu,
            action_bar,
            map_areas: [
                area::parse(&assets.data("map01.area")?)?,
                area::parse(&assets.data("map02.area")?)?,
            ],
            panel_areas: area::parse(&assets.data("map03.area")?)?,
        })
    }

    /// The small save/load/... buttons at their place on a screen.
    fn small_buttons<'a>(&'a self, areas: &'a [Area]) -> impl Iterator<Item = Cell> + 'a {
        self.action_bar.iter().filter_map(move |c| {
            let k = c.area.id;
            areas
                .iter()
                .find(|a| a.id == k)
                .map(|a| Cell { area: *a, ..*c })
        })
    }

    fn hit_map(&self, ui: &Ui, m: &crate::kimaten2::map::Map, x: i32, y: i32) -> Option<Button> {
        let areas = &self.map_areas[m.second as usize];
        let found = |list: &[Area], id: i32| {
            list.iter()
                .find(|a| a.id == id && a.contains(x, y))
                .is_some()
        };
        if let Some(p) = m.panel {
            let (spots, back) = m.panel_spots(p);
            for (k, _) in spots {
                if found(&self.panel_areas, 256 + k as i32) {
                    return Some(Button::Map(256 + k as i32));
                }
            }
            let k = p as i32 * 4 + 3;
            if back && found(&self.panel_areas, 256 + k) {
                return Some(Button::Map(256 + k));
            }
        } else {
            for i in 0..m.places() {
                if m.place(i).shown && found(areas, i as i32) {
                    return Some(Button::Map(i as i32));
                }
            }
        }
        if m.back && found(areas, MAP_BACK_ID) {
            return Some(Button::Map(MAP_BACK_ID));
        }
        let _ = ui;
        self.small_buttons(areas)
            .find(|c| c.area.contains(x, y))
            .map(|c| Button::Bar(c.id))
    }

    fn draw_map(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        m: &crate::kimaten2::map::Map,
    ) -> Result<()> {
        let areas = &self.map_areas[m.second as usize];
        let bg = assets.image(if m.second { "map2.akb" } else { "map1.akb" })?;
        canvas.draw(&bg, 0, 0, false);
        let places = assets.image(if m.second { "map2_p.akb" } else { "map1_p.akb" })?;
        let sheet2 = assets.image("map2_p.akb")?;
        let state = |b: Button, enabled: bool| {
            if enabled {
                (ui.state(b, true, false) as u32).min(2)
            } else {
                3
            }
        };
        let at = |list: &[Area], id: i32| list.iter().find(|a| a.id == id).copied();
        for i in 0..m.places() {
            let spot = m.place(i);
            let Some(a) = at(areas, i as i32).filter(|_| spot.shown) else {
                continue;
            };
            let (w, h) = MAP_PLACE;
            let st = state(Button::Map(i as i32), spot.enabled && m.panel.is_none());
            canvas.draw_part(
                &places,
                (w * i as u32, h * st, w, h),
                a.left,
                a.top,
                false,
                255,
            );
            if spot.done {
                let (sx, sy, sw, sh) = MAP_STAMP;
                canvas.draw_part(
                    &sheet2,
                    (sx, sy, sw, sh),
                    a.left + 16,
                    a.top + 8,
                    false,
                    255,
                );
            }
        }
        let back = |canvas: &mut Canvas, id: i32, a: Area| {
            let (w, h) = MAP_BACK;
            let st = state(Button::Map(id), true);
            canvas.draw_part(
                &sheet2,
                (1440, MAP_SHEET_Y + h * st, w, h),
                a.left,
                a.top,
                false,
                255,
            );
        };
        if let Some(p) = m.panel {
            let (spots, has_back) = m.panel_spots(p);
            for (k, spot) in spots {
                let id = 256 + k as i32;
                let Some(a) = at(&self.panel_areas, id) else {
                    continue;
                };
                let (w, h) = MAP_PANEL;
                let st = state(Button::Map(id), spot.enabled);
                canvas.draw_part(
                    &sheet2,
                    (MAP_PANEL_X[k], MAP_SHEET_Y + h * st, w, h),
                    a.left,
                    a.top,
                    false,
                    255,
                );
                if spot.done {
                    let (sx, sy, sw, sh) = MAP_STAMP_SMALL;
                    canvas.draw_part(&sheet2, (sx, sy, sw, sh), a.left + 48, a.top, false, 255);
                }
            }
            let id = 256 + p as i32 * 4 + 3;
            if let (true, Some(a)) = (has_back, at(&self.panel_areas, id)) {
                back(canvas, id, a);
            }
        }
        if let (true, Some(a)) = (m.back, at(areas, MAP_BACK_ID)) {
            back(canvas, MAP_BACK_ID, a);
        }
        let select = assets.image("select_p.akb")?;
        for c in self.small_buttons(areas) {
            c.draw(canvas, &select, ui, false);
        }
        Ok(())
    }

    fn hit_actions(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        let (w, h) = ACTION_SIZE;
        let at = action_positions(ui.actions.len());
        if let Some(i) = at
            .iter()
            .position(|&(ax, ay)| x >= ax && y >= ay && x < ax + w as i32 && y < ay + h as i32)
        {
            return Some(Button::Action(i));
        }
        self.action_bar
            .iter()
            .find(|c| c.area.contains(x, y))
            .map(|c| Button::Bar(c.id))
    }

    fn draw_actions(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui) -> Result<()> {
        let sheet = assets.image("select_p.akb")?;
        let (w, h) = ACTION_SIZE;
        let at = action_positions(ui.actions.len());
        for (i, (a, &(x, y))) in ui.actions.iter().zip(at).enumerate() {
            let state = (ui.state(Button::Action(i), true, false) as u32).min(2);
            let (sx, sy) = action_cell(a.icon);
            canvas.draw_part(&sheet, (sx, sy + h * state, w, h), x, y, false, 255);
            // Ticks are whole cells drawn over the button.
            match a.tick {
                1 => canvas.draw_part(&sheet, (304, 846, w, h), x, y, false, 255),
                2 => canvas.draw_part(&sheet, (608, 846, w, h), x, y, false, 255),
                _ => {}
            }
        }
        for c in &self.action_bar {
            c.draw(canvas, &sheet, ui, false);
        }
        Ok(())
    }

    fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        let menu = if ui.side_menu { &self.menu[..] } else { &[] };
        menu.iter()
            .chain(&self.bar)
            .find(|c| c.area.contains(x, y))
            .map(|c| Button::Bar(c.id))
    }

    fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui) -> Result<()> {
        let sheet = assets.image("message_p.akb")?;
        for c in &self.bar {
            let active = match c.id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip && !ui.skip_all,
                bar::SKIP_ALL => ui.skip && ui.skip_all,
                bar::LOOP_VOICE => ui.loop_voice,
                bar::MENU => ui.side_menu,
                _ => false,
            };
            c.draw(canvas, &sheet, ui, active);
        }
        if ui.side_menu {
            let sheet = assets.image("message_menu.akb")?;
            // Backdrop: the dark gradient at the right of the sheet.
            canvas.draw_part(&sheet, (792, 0, 264, 720), 1016, 0, false, 255);
            for c in &self.menu {
                c.draw(canvas, &sheet, ui, false);
            }
        }
        Ok(())
    }
}

impl Layout {
    pub fn is_kimaten2(&self) -> bool {
        self.kimaten2.is_some()
    }

    pub fn load(assets: &Assets, language: Language) -> Result<Self> {
        if assets.game().title == silky_format::game::Title::Kimaten2 {
            return Ok(Self {
                language,
                bar: Vec::new(),
                kimaten2: Some(Kimaten2::load(assets)?),
            });
        }
        let mut bar = area::parse(&assets.data("mes_o.area")?)?;
        bar.retain(|a| a.id <= bar::CLOSE);
        for a in &mut bar {
            if a.id == bar::CLOSE {
                a.top += CLOSE_AREA_TOP;
                a.bottom += CLOSE_AREA_TOP;
            }
        }
        Ok(Self {
            language,
            bar,
            kimaten2: None,
        })
    }

    /// Button under a point (screen pixels).
    pub fn hit(&self, ui: &Ui, text_shown: bool, x: i32, y: i32) -> Option<Button> {
        if !ui.choices.is_empty() {
            let n = ui.choices.len();
            return (0..n)
                .find(|&i| {
                    let top = choice_top(i, n);
                    x >= CHOICE_LEFT
                        && x < CHOICE_LEFT + CHOICE_CELL.0 as i32
                        && y >= top
                        && y < top + CHOICE_CELL.1 as i32
                })
                .map(Button::Choice);
        }
        if let (Some(m), Some(k)) = (&ui.map, &self.kimaten2) {
            return k.hit_map(ui, m, x, y);
        }
        if let (false, Some(k)) = (ui.actions.is_empty(), &self.kimaten2) {
            return k.hit_actions(ui, x, y);
        }
        if let (Some(extras), Some(k)) = (ui.title, &self.kimaten2) {
            return k
                .title
                .entries(extras)
                .iter()
                .find(|(_, a, _)| a.contains(x, y))
                .map(|(e, _, _)| Button::Title(*e));
        }
        if let Some(extras) = ui.title {
            let l = title_layout(self.language);
            return title_entries(extras)
                .find(|&i| {
                    let (dx, dy) = l.dst[i];
                    let (w, h) = l.size[i];
                    x >= dx && y >= dy && x < dx + w as i32 && y < dy + h as i32
                })
                .map(Button::Title);
        }
        if !text_shown || ui.window_hidden {
            return None;
        }
        if let Some(k) = &self.kimaten2 {
            return k.hit(ui, x, y);
        }
        self.bar
            .iter()
            .find(|a| a.contains(x, y))
            .map(|a| Button::Bar(a.id))
    }

    /// Draws the title buttons, or the message window buttons and the
    /// click-wait indicator when text is shown.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        text_shown: bool,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        if !ui.choices.is_empty() {
            let sheet = assets.image("message_p.akb")?;
            let n = ui.choices.len();
            for (i, label) in ui.choices.iter().enumerate() {
                let state = ui.state(Button::Choice(i), true, false) as u32;
                // きまぐれテンプテーション2: the same size of cell, lower
                // on `message_p.akb` (provisional placement).
                let (sx, sy) = match self.kimaten2 {
                    Some(_) => (500, 600),
                    None => CHOICE_SHEET,
                };
                let (w, h) = CHOICE_CELL;
                let top = choice_top(i, n);
                canvas.draw_part(
                    &sheet,
                    (sx, sy + h * state, w, h),
                    CHOICE_LEFT,
                    top,
                    false,
                    255,
                );
                if let Some(tr) = text {
                    let line = tr
                        .wrap(label, CHOICE_TEXT_SIZE, w as f32 - CHOICE_TEXT_LEFT - 40.0)
                        .into_iter()
                        .next()
                        .unwrap_or_default();
                    tr.draw_line(
                        canvas,
                        &line,
                        CHOICE_LEFT as f32 + CHOICE_TEXT_LEFT,
                        top as f32 + h as f32 / 2.0 + CHOICE_TEXT_SIZE / 3.0,
                        CHOICE_TEXT_SIZE,
                        [255, 255, 255, 255],
                    );
                }
            }
        }
        if let (Some(m), Some(k)) = (&ui.map, &self.kimaten2) {
            return k.draw_map(canvas, assets, ui, m);
        }
        if let (false, Some(k)) = (ui.actions.is_empty(), &self.kimaten2) {
            return k.draw_actions(canvas, assets, ui);
        }
        if let (Some(extras), Some(k)) = (ui.title, &self.kimaten2) {
            let sheet = assets.image("title_p.akb")?;
            let shown = ui.clock.saturating_sub(ui.title_shown);
            for (k, &(entry, a, (sx, sy))) in k.title.entries(extras).iter().enumerate() {
                let b = Button::Title(entry);
                let state = ui.state(b, ui.enabled(b), false) as u32;
                let part = (sx, sy + Kimaten2Title::ROW * state, a.width(), a.height());
                // Each button starts its jump 100 ms after the one before.
                let y = match shown.checked_sub(100 * k as u64) {
                    Some(ms) => title_jump_y(ms),
                    None => 720.0,
                };
                let top = a.top + (y - 636.0) as i32;
                canvas.draw_part(&sheet, part, a.left, top, false, 255);
            }
            return Ok(());
        }
        if let Some(extras) = ui.title {
            let sheet =
                assets.image(&crate::assets::localized_name("title_p.akb", self.language))?;
            let l = title_layout(self.language);
            for i in title_entries(extras) {
                let b = Button::Title(i);
                let state = ui.state(b, ui.enabled(b), false) as u32;
                let (sx, sy) = l.src[i];
                let (w, h) = l.size[i];
                let (dx, dy) = l.dst[i];
                canvas.draw_part(&sheet, (sx, sy + h * state, w, h), dx, dy, false, 255);
            }
            return Ok(());
        }
        if !text_shown || ui.window_hidden {
            return Ok(());
        }
        if let Some(k) = &self.kimaten2 {
            return k.draw(canvas, assets, ui);
        }
        let sheet = assets.image("message_p.akb")?;
        for a in &self.bar {
            let b = Button::Bar(a.id);
            let active = match a.id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip,
                _ => false,
            };
            let state = ui.state(b, ui.enabled(b), active);
            let (part, (cw, ch)) = match a.id {
                bar::HIDE | bar::CLOSE => {
                    let (x, y) = if a.id == bar::HIDE {
                        HIDE_SHEET
                    } else {
                        CLOSE_SHEET
                    };
                    // Round buttons have no disabled row.
                    let row = (state as u32).min(2);
                    (
                        (x, y + ROUND_CELL.1 * row, ROUND_CELL.0, ROUND_CELL.1),
                        ROUND_CELL,
                    )
                }
                id => {
                    let (x, y) = BAR_SHEET;
                    (
                        (
                            x + BAR_CELL.0 * id as u32,
                            y + BAR_CELL.1 * state as u32,
                            BAR_CELL.0,
                            BAR_CELL.1,
                        ),
                        BAR_CELL,
                    )
                }
            };
            // Centre the cell on its click area.
            let dx = a.left + (a.width() as i32 - cw as i32) / 2;
            let dy = a.top + (a.height() as i32 - ch as i32) / 2;
            canvas.draw_part(&sheet, part, dx, dy, false, 255);
        }
        if ui.waiting {
            let frame = (ui.clock / WAIT_FRAME_MS) as u32 % WAIT_FRAMES;
            let (x, y) = WAIT_SHEET;
            canvas.draw_part(
                &sheet,
                (x, y + WAIT_CELL * frame, WAIT_CELL, WAIT_CELL),
                WAIT_POS.0,
                WAIT_POS.1,
                false,
                255,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_buttons_jump_and_settle() {
        assert_eq!(title_jump_y(0), 720.0);
        assert!(title_jump_y(200) < 636.0);
        assert_eq!(title_jump_y(TITLE_JUMP_MS - 600), 636.0);
    }
}
