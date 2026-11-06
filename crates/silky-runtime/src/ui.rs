//! Engine-drawn interface: the title menu buttons, the button bar of the
//! message window, and the click-wait indicator.
//!
//! The scripts only ask for the title menu (game command 110) and show
//! text; the buttons themselves are drawn and hit-tested by the engine. The
//! title layout below is copied from tables in the game executable; the
//! message window buttons come from `mes_o.area` and `message_p.akb`.

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
    /// A message waits for a click: the indicator animates.
    pub waiting: bool,
    /// Animation clock in milliseconds.
    pub clock: u64,
    /// Whether there are saves to load (enables Continue and Load).
    pub has_saves: bool,
    /// Options of a pending selection, as shown.
    pub choices: Vec<String>,
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
            Button::Title(title::CONTINUE | title::LOAD) => self.has_saves,
            Button::Bar(bar::LOAD | bar::QUICK_LOAD) => self.has_saves,
            Button::Bar(bar::PREVIOUS_CHOICE) => false,
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

/// Screen layout of the interface, loaded once per game.
pub struct Layout {
    language: Language,
    /// Message window buttons from `mes_o.area`, close button moved to
    /// screen coordinates.
    bar: Vec<Area>,
}

impl Layout {
    pub fn load(assets: &Assets, language: Language) -> Result<Self> {
        let mut bar = area::parse(&assets.data("mes_o.area")?)?;
        bar.retain(|a| a.id <= bar::CLOSE);
        for a in &mut bar {
            if a.id == bar::CLOSE {
                a.top += CLOSE_AREA_TOP;
                a.bottom += CLOSE_AREA_TOP;
            }
        }
        Ok(Self { language, bar })
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
                let (sx, sy) = CHOICE_SHEET;
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
