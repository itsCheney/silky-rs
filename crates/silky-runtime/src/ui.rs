//! Engine-drawn interface: the title menu buttons, the button bar of the
//! message window, and the click-wait indicator.
//!
//! The scripts only ask for the title menu (game command 110) and show
//! text; the buttons themselves are drawn and hit-tested by the engine. The
//! title layout below is copied from tables in the game executable; the
//! message window buttons come from `mes_o.area` and `message_p.akb`.

use silky_format::area::{self, Area};
use std::sync::Arc;

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
    /// Option of a selection, by its index in the list.
    Choice(usize),
    /// Arrow of a selection that has more than one page, by its hot spot
    /// id (see [`crate::games::doukyuusei::select::PREV`]).
    ChoicePage(i32),
    /// Button of きまぐれテンプテーション2's action menu, by index.
    Action(usize),
    /// Button of きまぐれテンプテーション2's map, by its area id: places
    /// 0.., panel buttons 256.., back 512.
    Map(i32),
    /// Element of a screen of 同級生 Remake, by the id its
    /// [`crate::games::doukyuusei::Panel`] gives.
    Panel(i32),
    /// Element of a screen of きまぐれテンプテーション, by the id its
    /// [`crate::games::kimaten1::Screen`] gives.
    Kimaten1(i32),
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
    /// 先輩が私の妄想にドージンする: the extras page's first button, the
    /// doujinshi.
    pub const BODY_TOUCH: usize = 13;

    /// きまぐれテンプテーション2: extras entries and the byte variable that
    /// unlocks each (from the executable's title menu): music
    /// mode, アンネViewer, 捜査手帳, アンネの部屋. The endings set 3007.
    pub const KIMATEN2_UNLOCK: [(usize, usize); 4] = [
        (MUSIC_MODE, 3004),
        (VIEWER, 3007),
        (NOTEBOOK, 3006),
        (ROOM, 3007),
    ];

    /// 景の海のアペイリア: the extras entries and the byte variables that
    /// unlock them (from the executable's title menu).
    pub const APEIRIA_UNLOCK: [(usize, usize); 3] =
        [(CG_MODE, 3002), (SCENE_MODE, 3003), (MUSIC_MODE, 3004)];

    /// なないろリンカネーション: the same, and byte 3001 for settings and
    /// the bonus page (from the executable's title menu).
    pub const NANAIRO_UNLOCK: [(usize, usize); 5] = [
        (SETTINGS, 3001),
        (EXTRAS, 3001),
        (CG_MODE, 3002),
        (SCENE_MODE, 3003),
        (MUSIC_MODE, 3004),
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
    /// 景の海のアペイリア only: the ball in the middle of the button ring,
    /// which folds the ring away and back.
    pub const RING: i32 = 22;
    /// 同級生 Remake only: the schedule and the map jump of its menu (not
    /// done yet).
    pub const SCHEDULE: i32 = 23;
    pub const MAP: i32 = 24;
    /// きまぐれテンプテーション only: the notebook button of the bar.
    pub const K1_NOTEBOOK: i32 = 25;
}

/// Interface state kept by the frontend.
#[derive(Debug, Clone, Default)]
pub struct Ui {
    /// 同級生 Remake: the classic game mode is set (the easy mode adds the
    /// badges on the selection bars and the flag list button).
    pub classic: bool,
    /// The title menu is up; `true` shows the extras page.
    pub title: Option<bool>,
    pub hover: Option<Button>,
    /// Where the pointer last was.
    pub pointer: (i32, i32),
    pub pressed: Option<Button>,
    /// Message window hidden by the player (right click or the hide
    /// button); any click shows it again.
    pub window_hidden: bool,
    pub auto: bool,
    pub skip: bool,
    /// Skip goes through unread text too (きまぐれテンプテーション2's
    /// skip-all button).
    pub skip_all: bool,
    /// Skipping on to the next unread text or selection (the jump button).
    pub jumping: bool,
    /// The voice of the message plays again each time it ends
    /// (きまぐれテンプテーション2's voice loop button).
    pub loop_voice: bool,
    /// A message waits for a click: the indicator animates.
    pub waiting: bool,
    /// きまぐれテンプテーション2: the jump to the next unread text or
    /// selection is off while the message is unread (under Wine).
    pub jump_off: bool,
    /// きまぐれテンプテーション2: the costume button works (byte 700 set).
    pub costume: bool,
    /// きまぐれテンプテーション2: the bar pin is on, and how far below its
    /// place the bar has slid (0 up, [`BAR_DROP`] out of sight).
    pub menu_lock: bool,
    pub bar_drop: i32,
    /// Animation clock in milliseconds.
    pub clock: u64,
    /// Whether there are saves to load (enables Continue and Load).
    pub has_saves: bool,
    /// Options of a pending selection, as shown.
    pub choices: Vec<String>,
    /// Shown status of each option (empty when none applies).
    pub choice_status: Vec<crate::games::doukyuusei::select::Status>,
    /// Page of the selection shown.
    pub choice_page: usize,
    /// There is a selection to go back to.
    pub previous_choice: bool,
    /// きまぐれテンプテーション2's side menu is open.
    pub side_menu: bool,
    /// 同級生 Remake: the side buttons slide out or back; input waits.
    pub menu_anim: Option<crate::games::doukyuusei::menu::MenuAnim>,
    /// 同級生 Remake: the room's buttons, hint bar, menu button and plate
    /// slide out or in; input waits.
    pub room_fx: Option<crate::games::doukyuusei::room::Fx>,
    /// 同級生 Remake: byte 4084 keeps the status plate up through the
    /// room's look mode.
    pub plate_keep: bool,
    /// 景の海のアペイリア: the button ring is folded away.
    pub ring_closed: bool,
    /// Buttons of きまぐれテンプテーション2's action menu, while it is up.
    pub actions: Vec<crate::ActionButton>,
    /// きまぐれテンプテーション2's map, while it is up.
    pub map: Option<crate::games::kimaten2::map::Map>,
    /// A screen of 同級生 Remake (room, hours dial), while it is up.
    pub panel: Option<crate::games::doukyuusei::Panel>,
    /// The screen of きまぐれテンプテーション that is up, and the clock
    /// when it opened.
    pub k1: Option<crate::games::kimaten1::Screen>,
    /// きまぐれテンプテーション: a system screen opened from the message
    /// window bar or from the bar of a screen the script waits on (the
    /// notebook). It sits over whatever is up and takes all input; closing
    /// it answers nothing to the script.
    pub k1_menu: Option<crate::games::kimaten1::Screen>,
    pub k1_since: u64,
    /// きまぐれテンプテーション: the clock when the notebook button of the
    /// window started to blink.
    pub k1_blink: Option<u64>,
    /// きまぐれテンプテーション: the bar's buttons slide down (folding) or
    /// back up (unfolding): which way and the clock when it started.
    pub k1_fold: Option<(bool, u64)>,
    /// The notices of きまぐれテンプテーション that are up.
    pub k1_notices: Option<crate::games::kimaten1::Notices>,
    /// 同級生 Remake's status plate, while it is up.
    pub plate: Option<crate::games::doukyuusei::plate::Plate>,
    /// The backlog has entries (同級生 Remake's menu leaves its button out
    /// otherwise).
    pub has_log: bool,
    /// The map jump page is done for the map the walk is on.
    pub map_jump: bool,
    /// The flag list's button is usable (see `games::doukyuusei::flaglist`).
    pub flag_list: bool,
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
    /// Clock when the title menu itself came up (きまぐれテンプテーション's
    /// heroine changes her face on that clock; a page that came up later
    /// replaced the other one).
    pub title_opened: u64,
    /// Show tooltips over the window's buttons (the setting).
    pub tooltips: bool,
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
    /// 同級生 Remake: the status plate and the menu button are up.
    pub fn plate_up(&self) -> bool {
        self.chrome().menu.is_some()
    }

    /// 同級生 Remake: how the plate (opacity) and the menu button (shift to
    /// the right) show. The room's slides fade and move them, look mode
    /// has no menu button and no plate unless byte 4084 keeps it.
    pub fn chrome(&self) -> Chrome {
        use crate::games::doukyuusei::{room::Step, Panel};
        let up = self.plate.is_some() && !matches!(self.panel, Some(Panel::Jump(_)));
        let kept = if self.plate_keep { 255 } else { 0 };
        let looking = matches!(&self.panel, Some(Panel::Room(r)) if r.look && !r.is_plain());
        let pass = self.room_fx.map(|f| f.pass(self.clock));
        let (plate, menu) = match pass {
            Some(p) if matches!(p.step, Step::HideRoom | Step::ShowRoom) => {
                (p.plate_alpha().max(kept), Some(p.menu_dx()))
            }
            Some(_) => (kept, None),
            None if looking => (kept, None),
            None => (255, Some(0)),
        };
        Chrome {
            plate: if up { plate } else { 0 },
            menu: menu.filter(|_| up),
        }
    }

    pub fn enabled(&self, b: Button) -> bool {
        match b {
            Button::Title(t) if self.locked_titles.contains(&t) => false,
            Button::Title(title::CONTINUE | title::LOAD) => self.has_saves,
            // 同級生 Remake's menu: the flag list from the unlocked story
            // on, the map jump only where byte 7089 names a page (see
            // `games::doukyuusei::menu` and `jump::Jump::available`).
            Button::Bar(bar::SCHEDULE) if self.plate.is_some() => self.flag_list,
            Button::Bar(bar::MAP) if self.plate.is_some() => self.map_jump,
            Button::Bar(bar::BACKLOG) if self.plate.is_some() => self.has_log,
            Button::Bar(bar::LOAD | bar::QUICK_LOAD) => self.has_saves,
            Button::Bar(bar::PREVIOUS_CHOICE) => self.previous_choice,
            Button::Bar(bar::NEXT_CHOICE) => !self.jump_off,
            Button::Bar(bar::NOTEBOOK) => self.notebook,
            Button::Bar(bar::COSTUME) => self.costume,
            Button::Bar(bar::COLLECTION) => self.collection,
            Button::Bar(bar::K1_NOTEBOOK) => self.notebook,
            Button::Choice(i) => self.choice_status.get(i).map_or(true, |s| s.clickable),
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
    match lang {
        Language::Japanese => &TITLE_LAYOUTS[3],
        _ => &TITLE_LAYOUTS[lang.map_section()],
    }
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
/// The 2021 package draws it 32 pixels left and 31 up (under Wine).
const WAIT_POS_2021: (i32, i32) = (1183, 576);
const WAIT_FRAME_MS: u64 = 100;

/// Selection options of きまぐれテンプテーション2: bars of 692 x 76 on the
/// sheet, rows by state. Their place on screen is provisional.
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

/// Selection options of ふゆから、くるる。 (`CNormalSelect`): one bar width for all, built from `message_p.akb` row 440
/// (three 76-pixel state rows) as a 123-pixel left end, 72-pixel middle
/// pieces and a 141-pixel right end (from x = 555). The middle pieces
/// number the longest label's half-width letters times 15, over 72, less
/// one. Bars are centred across the screen, 84 pixels apart, centred
/// down the 495 pixels above the window. Text starts 123 pixels into the
/// bar, its baseline 46 pixels down, with the message text's shadow
/// (checked under Wine).
struct ChoiceBars {
    left: i32,
    middles: i32,
    count: usize,
}

impl ChoiceBars {
    const ROW: u32 = 440;
    const HEIGHT: i32 = 76;
    const PITCH: i32 = 84;
    const LEFT_END: i32 = 123;
    const MIDDLE: i32 = 72;
    const RIGHT_END: (u32, i32) = (555, 141);
    const TEXT_BASELINE: f32 = 46.0;
    /// The executable starts the text 123 pixels in; our font's glyphs
    /// sit 2 pixels further left than the game's.
    const TEXT_LEFT: f32 = 125.0;
    const TEXT_SIZE: f32 = 24.0;

    fn new(labels: &[String]) -> Self {
        let longest = labels
            .iter()
            .map(|l| {
                l.chars()
                    .map(|c| if (c as u32) < 0x100 { 1 } else { 2 })
                    .sum::<i32>()
            })
            .max()
            .unwrap_or(0);
        let middles = (longest * 15 / 72 - 1).max(0);
        let width = middles * Self::MIDDLE + Self::LEFT_END + Self::RIGHT_END.1;
        Self {
            left: (1280 - width) / 2,
            middles,
            count: labels.len().min(5),
        }
    }

    fn width(&self) -> i32 {
        self.middles * Self::MIDDLE + Self::LEFT_END + Self::RIGHT_END.1
    }

    fn top(&self, i: usize) -> i32 {
        (720 - 225 - self.count as i32 * Self::PITCH) / 2 + i as i32 * Self::PITCH
    }

    fn draw(
        &self,
        canvas: &mut Canvas,
        sheet: &Arc<silky_format::akb::Image>,
        i: usize,
        state: u32,
    ) {
        let (top, row) = (self.top(i), Self::ROW + Self::HEIGHT as u32 * state);
        let h = Self::HEIGHT as u32;
        canvas.draw_part(
            sheet,
            (0, row, Self::LEFT_END as u32, h),
            self.left,
            top,
            false,
            255,
        );
        for k in 0..self.middles {
            let x = self.left + Self::LEFT_END + k * Self::MIDDLE;
            canvas.draw_part(
                sheet,
                (Self::LEFT_END as u32, row, Self::MIDDLE as u32, h),
                x,
                top,
                false,
                255,
            );
        }
        let x = self.left + Self::LEFT_END + self.middles * Self::MIDDLE;
        let (rx, rw) = Self::RIGHT_END;
        canvas.draw_part(sheet, (rx, row, rw as u32, h), x, top, false, 255);
    }
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
/// jump starts, from the executable (one step per 60 Hz frame):
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

/// Title menu drawn from fixed cells of one sheet (景の海のアペイリア,
/// 同級生 Remake, なないろリンカネーション): click areas from `title.area`
/// and `title2.area`, cell positions from the executable, four state rows
/// per cell (normal, hover, pressed, disabled).
struct SheetTitle {
    main: Vec<TitleCell>,
    extras: Vec<TitleCell>,
    /// Name of the sheet.
    sheet: &'static str,
    /// The sheet comes in one picture per language.
    localized: bool,
    /// Entries and the byte variables that unlock them.
    unlocks: &'static [(usize, usize)],
    /// What the title call returns for an entry, where the game numbers
    /// them its own way.
    result: Option<fn(usize) -> Option<i32>>,
    /// How long a page change cross-fades, in milliseconds (0: it cuts).
    fade_ms: u64,
}

/// One entry of a [`SheetTitle`].
#[derive(Clone, Copy)]
struct TitleCell {
    /// `None` entries are drawn disabled and take no clicks.
    entry: Option<usize>,
    area: Area,
    /// Top left of the cell on the sheet.
    src: (u32, u32),
    /// Where it goes on screen, and its size.
    dst: (i32, i32),
    size: (u32, u32),
    /// Distance between state rows.
    row: u32,
    /// State row drawn when the entry is neither hovered nor pressed.
    idle: u32,
}

impl TitleCell {
    /// A cell drawn over its click area, `row` pixels high.
    fn on_area(entry: Option<usize>, area: Area, src: (u32, u32), row: u32) -> Self {
        Self {
            entry,
            area,
            src,
            dst: (area.left, area.top),
            size: (area.width(), row),
            row,
            idle: 0,
        }
    }
}

impl SheetTitle {
    /// Disabled state row.
    const DISABLED: u32 = 3;

    /// 同級生 Remake: `title_p_<lang>.akb` holds 494 x 78 cells, three to
    /// a row, four state rows (312 pixels) per block, in `title.area`
    /// order; the extras are a fourth column from x = 1482 (615 wide;
    /// music 655) and a fifth from 2097 for the hidden entry and back.
    /// Checked under Wine (English).
    fn load_doukyuusei(assets: &Assets) -> Result<Self> {
        let main_areas = area::parse(&assets.data("title.area")?)?;
        let extra_areas = area::parse(&assets.data("title2.area")?)?;
        const MAIN: [usize; 6] = [
            title::NEW_GAME,
            title::CONTINUE,
            title::LOAD,
            title::SETTINGS,
            title::EXTRAS,
            title::QUIT,
        ];
        let main = main_areas
            .iter()
            .filter(|a| (0..6).contains(&a.id))
            .map(|a| {
                let i = a.id as u32;
                let src = (494 * (i % 3), 312 * (i / 3));
                TitleCell::on_area(Some(MAIN[a.id as usize]), *a, src, 78)
            })
            .collect();
        const EXTRAS: [(usize, (u32, u32)); 5] = [
            (title::CG_MODE, (1482, 0)),
            (title::SCENE_MODE, (1482, 312)),
            (title::MUSIC_MODE, (1482, 624)),
            (title::VIEWER, (2097, 0)),
            (title::BACK, (2097, 312)),
        ];
        let extras = extra_areas
            .iter()
            .filter_map(|a| {
                let &(entry, cell) = EXTRAS.get(usize::try_from(a.id).ok()?)?;
                Some(TitleCell::on_area(Some(entry), *a, cell, 78))
            })
            .collect();
        Ok(Self {
            main,
            extras,
            sheet: "title_p.akb",
            localized: true,
            unlocks: &[],
            result: None,
            fade_ms: 0,
        })
    }

    fn load_apeiria(assets: &Assets) -> Result<Self> {
        let main_areas = area::parse(&assets.data("title.area")?)?;
        let extra_areas = area::parse(&assets.data("title2.area")?)?;
        // Area id, entry and sheet cell of each main entry; the bonus cell
        // is in the second block, 4 rows down.
        const MAIN: [(i32, usize, (u32, u32)); 6] = [
            (0, title::NEW_GAME, (0, 0)),
            (1, title::CONTINUE, (320, 0)),
            (2, title::LOAD, (160, 0)),
            (3, title::SETTINGS, (480, 0)),
            (4, title::EXTRAS, (0, 168)),
            (5, title::QUIT, (800, 0)),
        ];
        const EXTRAS: [(i32, usize, (u32, u32)); 4] = [
            (4, title::BACK, (0, 168)),
            (256, title::CG_MODE, (160, 168)),
            (257, title::SCENE_MODE, (320, 168)),
            (258, title::MUSIC_MODE, (480, 168)),
        ];
        let mut main = Vec::new();
        let mut extras = Vec::new();
        for (id, entry, cell) in MAIN {
            if let Some(a) = main_areas.iter().find(|a| a.id == id) {
                main.push(TitleCell::on_area(Some(entry), *a, cell, 42));
                // The extras page keeps the other main entries, disabled;
                // bonus stays live as the way back.
                if id != 4 {
                    extras.push(TitleCell::on_area(None, *a, cell, 42));
                }
            }
        }
        for (id, entry, cell) in EXTRAS {
            if let Some(a) = extra_areas.iter().find(|a| a.id == id) {
                extras.push(TitleCell::on_area(Some(entry), *a, cell, 42));
            }
        }
        Ok(Self {
            main,
            extras,
            sheet: "title_p.akb",
            localized: false,
            unlocks: &title::APEIRIA_UNLOCK,
            result: None,
            fade_ms: 0,
        })
    }

    /// なないろリンカネーション: `title.akb`, cells and places from the
    /// executable. The main entries sit on a 349 x 449 layer at (145, 255),
    /// the extras on a 352 x 209 layer at (144, 313), shown in turn; the
    /// round bonus button, at (467, 517), is the way to the extras and
    /// back. It has three state rows and shows pressed on the extras page.
    fn load_nanairo(assets: &Assets) -> Result<Self> {
        let main_areas = area::parse(&assets.data("title.area")?)?;
        let extra_areas = area::parse(&assets.data("title2.area")?)?;
        type Spec = (i32, usize, (i32, i32), (u32, u32), (u32, u32), u32);
        // Area id, entry, place, size, sheet cell, state row step.
        const MAIN: [Spec; 6] = [
            (
                0,
                title::NEW_GAME,
                (145 + 63, 255 + 75),
                (222, 72),
                (0, 0),
                72,
            ),
            (
                1,
                title::CONTINUE,
                (145, 255 + 150),
                (349, 73),
                (222, 0),
                73,
            ),
            (
                2,
                title::LOAD,
                (145 + 61, 255 + 225),
                (225, 72),
                (571, 0),
                72,
            ),
            (
                3,
                title::SETTINGS,
                (145 + 72, 255 + 300),
                (203, 74),
                (796, 0),
                74,
            ),
            (
                4,
                title::QUIT,
                (145 + 43, 255 + 375),
                (260, 73),
                (0, 292),
                73,
            ),
            (5, title::EXTRAS, (467, 517), (189, 189), (999, 0), 189),
        ];
        const EXTRAS: [Spec; 4] = [
            (
                256,
                title::CG_MODE,
                (144 + 75, 313),
                (203, 55),
                (450, 296),
                55,
            ),
            (
                257,
                title::SCENE_MODE,
                (144 + 47, 313 + 75),
                (258, 55),
                (653, 296),
                55,
            ),
            (
                258,
                title::MUSIC_MODE,
                (144, 313 + 150),
                (352, 59),
                (450, 516),
                59,
            ),
            (259, title::BACK, (467, 517), (189, 189), (999, 0), 189),
        ];
        let cells = |areas: &[Area], specs: &[Spec]| {
            specs
                .iter()
                .filter_map(|&(id, entry, dst, size, src, row)| {
                    let area = *areas.iter().find(|a| a.id == id)?;
                    let idle = if entry == title::BACK { 2 } else { 0 };
                    Some(TitleCell {
                        entry: Some(entry),
                        area,
                        src,
                        dst,
                        size,
                        row,
                        idle,
                    })
                })
                .collect()
        };
        Ok(Self {
            main: cells(&main_areas, &MAIN),
            extras: cells(&extra_areas, &EXTRAS),
            sheet: "title.akb",
            localized: false,
            unlocks: &title::NANAIRO_UNLOCK,
            result: None,
            fade_ms: 0,
        })
    }

    /// 先輩が私の妄想にドージンする: cells of `title_p.akb` laid over the
    /// click areas of `title.area` and `title2.area` (see
    /// [`crate::games::doujin::title`]).
    fn load_doujin(assets: &Assets) -> Result<Self> {
        use crate::games::doujin::title as d;
        let main_areas = area::parse(&assets.data("title.area")?)?;
        let extra_areas = area::parse(&assets.data("title2.area")?)?;
        let cells = |areas: &[Area], specs: &[d::Spec]| {
            specs
                .iter()
                .filter_map(|&(id, entry, src, width)| {
                    let area = *areas.iter().find(|a| a.id == id)?;
                    Some(TitleCell {
                        entry: Some(entry),
                        area,
                        src,
                        dst: (area.left, area.top),
                        size: (width, d::ROW),
                        row: d::ROW,
                        idle: 0,
                    })
                })
                .collect()
        };
        Ok(Self {
            main: cells(&main_areas, &d::MAIN),
            extras: cells(&extra_areas, &d::EXTRAS),
            sheet: "title_p.akb",
            localized: true,
            unlocks: &d::UNLOCKS,
            result: Some(d::result),
            fade_ms: d::FADE_MS,
        })
    }

    fn entries(&self, extras: bool) -> &[TitleCell] {
        if extras {
            &self.extras
        } else {
            &self.main
        }
    }

    fn hit(&self, extras: bool, x: i32, y: i32) -> Option<Button> {
        self.entries(extras)
            .iter()
            .find(|c| c.entry.is_some() && c.area.contains(x, y))
            .and_then(|c| c.entry.map(Button::Title))
    }

    fn draw_page(
        &self,
        canvas: &mut Canvas,
        sheet: &std::sync::Arc<silky_format::akb::Image>,
        ui: &Ui,
        extras: bool,
        opacity: u8,
    ) {
        for c in self.entries(extras) {
            let state = match c.entry {
                Some(e) => {
                    let b = Button::Title(e);
                    match ui.state(b, ui.enabled(b), false) as u32 {
                        0 => c.idle,
                        s => s,
                    }
                }
                None => Self::DISABLED,
            };
            let (sx, sy) = c.src;
            let part = (sx, sy + c.row * state, c.size.0, c.size.1);
            canvas.draw_part(&sheet, part, c.dst.0, c.dst.1, false, opacity);
        }
    }

    fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        extras: bool,
        language: Language,
    ) -> Result<()> {
        let sheet = if self.localized {
            assets.localized_image(self.sheet, language)?
        } else {
            assets.image(self.sheet)?
        };
        // A page change fades the old buttons out as the new ones come in.
        let fade = (ui.title_shown > ui.title_opened && self.fade_ms > 0)
            .then(|| ui.clock.saturating_sub(ui.title_shown))
            .filter(|&ms| ms < self.fade_ms);
        let alpha = |f: u64| (f * 255 / self.fade_ms.max(1)) as u8;
        if let Some(ms) = fade {
            self.draw_page(canvas, &sheet, ui, !extras, 255 - alpha(ms));
            self.draw_page(canvas, &sheet, ui, extras, alpha(ms));
        } else {
            self.draw_page(canvas, &sheet, ui, extras, 255);
        }
        Ok(())
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
    /// Title menu from fixed sheet cells, for games that have one.
    sheet_title: Option<SheetTitle>,
    /// 景の海のアペイリア: the message window buttons sit in a ring.
    ring: Option<Ring>,
    /// 同級生 Remake's message window buttons.
    douk: Option<DoukBar>,
    /// きまぐれテンプテーション's message window buttons.
    k1: Option<crate::games::kimaten1::window::Bar>,
    /// なないろリンカネーション's message window buttons.
    nanairo: Option<crate::games::nanairo::window::Bar>,
    doujin: Option<crate::games::doujin::window::Bar>,
}

/// Message window buttons of 景の海のアペイリア, from the executable: the
/// ring backdrop and the middle ball are 302-pixel cells of
/// `menu_p00.akb` (ball rows normal, hover, pressed; backdrop row 6),
/// the eleven ring buttons and the hide button 40-pixel cells of
/// `message_p.akb` from (0, 424), one column per button, rows by state.
/// `mes_o.area` holds the ring buttons as ids 1..=11 (bar id + 1) and
/// the ball as id 0 in screen pixels; the hide button (256) and the
/// window (512) are relative to the window top.
struct Ring {
    /// (bar id, click area, sheet column).
    buttons: Vec<(i32, Area, u32)>,
}

impl Ring {
    const POS: (i32, i32) = (957, 464);
    const BALL: u32 = 302;
    const BACKDROP_ROW: u32 = 6;
    const CELL: u32 = 40;
    const SHEET_TOP: u32 = 424;
    const HIDE_COLUMN: u32 = 11;
    const WINDOW_TOP: i32 = 508;
    /// Click-wait hourglass: ten 76 x 80 frames down `message_p.akb` from
    /// (1280, 0), at (1204, window top + 132). The original steps it every
    /// tenth tick, 200 ms as measured under Wine.
    const WAIT_SHEET: (u32, u32) = (1280, 0);
    const WAIT_CELL: (u32, u32) = (76, 80);
    const WAIT_POS: (i32, i32) = (1204, 642);
    const WAIT_FRAME_MS: u64 = 200;

    fn load(assets: &Assets) -> Result<Self> {
        let mut buttons = Vec::new();
        for a in area::parse(&assets.data("mes_o.area")?)? {
            match a.id {
                0 => buttons.push((bar::RING, a, 0)),
                1..=11 => buttons.push((a.id - 1, a, a.id as u32 - 1)),
                256 => {
                    let mut a = a;
                    a.top += Self::WINDOW_TOP;
                    a.bottom += Self::WINDOW_TOP;
                    buttons.push((bar::HIDE, a, Self::HIDE_COLUMN));
                }
                _ => {}
            }
        }
        Ok(Self { buttons })
    }

    /// Buttons shown: only the ball and hide while the ring is folded.
    fn shown<'a>(&'a self, ui: &'a Ui) -> impl Iterator<Item = &'a (i32, Area, u32)> + 'a {
        self.buttons
            .iter()
            .filter(move |(id, _, _)| !ui.ring_closed || matches!(*id, bar::RING | bar::HIDE))
    }

    fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        self.shown(ui)
            .find(|(_, a, _)| a.contains(x, y))
            .map(|&(id, _, _)| Button::Bar(id))
    }

    fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui) -> Result<()> {
        let balls = assets.image("menu_p00.akb")?;
        let (x, y) = Self::POS;
        let n = Self::BALL;
        if !ui.ring_closed {
            canvas.draw_part(&balls, (0, n * Self::BACKDROP_ROW, n, n), x, y, false, 255);
        }
        let sheet = assets.image("message_p.akb")?;
        for &(id, a, column) in self.shown(ui) {
            let b = Button::Bar(id);
            let active = match id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip,
                _ => false,
            };
            let state = ui.state(b, ui.enabled(b), active) as u32;
            if id == bar::RING {
                let row = state.min(2);
                canvas.draw_part(&balls, (0, n * row, n, n), x, y, false, 255);
            } else {
                let c = Self::CELL;
                let part = (c * column, Self::SHEET_TOP + c * state, c, c);
                canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
            }
        }
        if ui.waiting {
            let frame = (ui.clock / Self::WAIT_FRAME_MS) as u32 % WAIT_FRAMES;
            let (x, y) = Self::WAIT_SHEET;
            let (w, h) = Self::WAIT_CELL;
            let (dx, dy) = Self::WAIT_POS;
            canvas.draw_part(&sheet, (x, y + h * frame, w, h), dx, dy, false, 255);
        }
        Ok(())
    }
}

/// Whether a button of 同級生 Remake's menu is left out: the map jump
/// shows only on the walking maps, the backlog only with a log, and the
/// flag list button is hidden in the classic mode.
fn douk_menu_hidden(ui: &Ui, id: i32) -> bool {
    let walking = matches!(ui.panel, Some(crate::games::doukyuusei::Panel::Town));
    (id == bar::MAP && !walking)
        || (id == bar::BACKLOG && !ui.has_log)
        || (id == bar::SCHEDULE && ui.classic)
}

/// How 同級生 Remake's status plate and menu button show (see
/// [`Ui::chrome`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chrome {
    pub plate: u8,
    pub menu: Option<i32>,
}

/// Message window buttons of 同級生 Remake: `mes_o.area` ids 0..=4 (skip,
/// skip all, auto, quick save, quick load) in screen pixels, 5 and 6
/// (voice, close) relative to the window top; cells of `message_p.akb`,
/// rows by state (measured under Wine).
struct DoukBar {
    /// (bar id, click area on screen, sheet cell, cell size, row step).
    buttons: Vec<(i32, Area, (u32, u32), (u32, u32), u32)>,
    /// The bitmap fonts, for the room's hint bar, loaded when first
    /// needed.
    font: std::cell::OnceCell<Option<crate::games::doukyuusei::window::Font>>,
    /// The menu button at the top right (see
    /// [`crate::games::doukyuusei::menu`]).
    menu: Option<crate::games::doukyuusei::menu::Menu>,
}

impl DoukBar {
    /// Click-wait indicator: three 48-pixel frames down the sheet from
    /// (1716, 700), drawn at (1723, 996). The frame time is a guess.
    const WAIT_SHEET: (u32, u32) = (1716, 700);
    const WAIT_CELL: u32 = 48;
    const WAIT_FRAMES: u32 = 3;
    const WAIT_POS: (i32, i32) = (1723, 996);
    const WAIT_FRAME_MS: u64 = 150;

    fn load(assets: &Assets) -> Result<Self> {
        use crate::games::doukyuusei::window::TOP;
        let mut buttons = Vec::new();
        for a in area::parse(&assets.data("mes_o.area")?)? {
            let (id, cell, size, step) = match a.id {
                0 => (bar::SKIP, (1170, 700), (198, 53), 53),
                1 => (bar::SKIP_ALL, (1368, 700), (198, 53), 53),
                2 => (bar::AUTO, (1368, 916), (198, 53), 53),
                3 => (bar::QUICK_SAVE, (1170, 1132), (92, 53), 53),
                4 => (bar::QUICK_LOAD, (1262, 1132), (92, 53), 53),
                5 => (bar::VOICE, (1640, 700), (75, 76), 76),
                6 => (bar::HIDE, (1565, 700), (75, 76), 76),
                _ => continue,
            };
            let mut a = a;
            if a.id >= 5 {
                a.top += TOP;
                a.bottom += TOP;
            }
            buttons.push((id, a, cell, size, step));
        }
        Ok(Self {
            buttons,
            font: Default::default(),
            menu: crate::games::doukyuusei::menu::Menu::load(assets).ok(),
        })
    }

    /// The bitmap fonts, loaded on first use.
    fn font(&self, assets: &mut Assets) -> Option<&crate::games::doukyuusei::window::Font> {
        self.font
            .get_or_init(|| crate::games::doukyuusei::window::Font::load(assets).ok())
            .as_ref()
    }

    fn hit(&self, ui: &Ui, x: i32, y: i32) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(id, a, ..)| a.contains(x, y) && ui.enabled(Button::Bar(*id)))
            .map(|&(id, ..)| Button::Bar(id))
    }

    /// `round`: also the voice and close buttons of the window (not
    /// during a selection, when the window is gone).
    fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, ui: &Ui, round: bool) -> Result<()> {
        let sheet = assets.image("message_p.akb")?;
        for &(id, a, (sx, sy), (w, h), step) in &self.buttons {
            if !round && matches!(id, bar::VOICE | bar::HIDE) {
                continue;
            }
            let b = Button::Bar(id);
            let active = match id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip && !ui.skip_all,
                bar::SKIP_ALL => ui.skip && ui.skip_all,
                _ => false,
            };
            let state = ui.state(b, ui.enabled(b), active) as u32;
            // The round buttons sit a pixel left of their click areas.
            let x = if step == 76 { a.left - 1 } else { a.left };
            canvas.draw_part(&sheet, (sx, sy + step * state, w, h), x, a.top, false, 255);
        }
        if ui.waiting {
            let frame = (ui.clock / Self::WAIT_FRAME_MS) as u32 % Self::WAIT_FRAMES;
            let (x, y) = Self::WAIT_SHEET;
            let c = Self::WAIT_CELL;
            let (dx, dy) = Self::WAIT_POS;
            canvas.draw_part(&sheet, (x, y + c * frame, c, c), dx, dy, false, 255);
        }
        Ok(())
    }
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

/// きまぐれテンプテーション2's bar slides 44 pixels out of sight (the
/// buttons from y = 678 and the pin from 676 to 720), 2 pixels a step,
/// when the pin is off and the pointer is above y = 671.
pub const BAR_DROP: i32 = 44;
/// The bar's animation task runs once more than 15 ms have passed.
pub const BAR_STEP_MS: u64 = 16;
pub const BAR_SHOW_Y: i32 = 671;

impl Cell {
    /// How far the cell sits below its place: the buttons stop 2 pixels
    /// short of the pin's travel.
    fn drop(&self, ui: &Ui) -> i32 {
        match self.id {
            bar::PIN => ui.bar_drop,
            bar::HIDE => 0,
            _ => ui.bar_drop.min(BAR_DROP - 2),
        }
    }

    fn draw(
        &self,
        canvas: &mut Canvas,
        sheet: &Arc<silky_format::akb::Image>,
        ui: &Ui,
        active: bool,
    ) {
        self.draw_at(canvas, sheet, ui, active, 0);
    }

    fn draw_at(
        &self,
        canvas: &mut Canvas,
        sheet: &Arc<silky_format::akb::Image>,
        ui: &Ui,
        active: bool,
        drop: i32,
    ) {
        let b = Button::Bar(self.id);
        let state = match ui.state(b, ui.enabled(b), active) {
            // The side menu button shows dimmed while the menu is open.
            State::Pressed if self.id == bar::MENU && ui.side_menu => State::Disabled,
            s => s,
        };
        let state = (state as u32).min(self.states - 1);
        let (w, h) = self.size;
        // The pin's rows: closed padlock from y = 600, open from 714.
        let top = if self.id == bar::PIN && !ui.menu_lock {
            714
        } else {
            self.sheet.1
        };
        let part = (self.sheet.0, top + self.step * state, w, h);
        let dx = self.area.left + (self.area.width() as i32 - w as i32) / 2;
        let dy = self.area.top + (self.area.height() as i32 - h as i32) / 2 + drop;
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
    /// Tooltip lines of the bar and side menu (`MesButtonHelp.hlg`).
    help: Option<crate::assets::HelpLines>,
    /// The last tooltip balloon put together, by its size and point.
    balloon: std::sync::Mutex<Option<((i32, i32, BalloonPoint), Arc<silky_format::akb::Image>)>>,
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
    /// Click-wait icon: cells of `message_p` and where they go.
    const WAIT_CELL: (u32, u32, u32, u32) = (1262, 0, 74, 56);
    const WAIT_POS: (i32, i32) = (1062, 609);
    const WAIT_FRAME_MS: u64 = 150;

    fn load(assets: &Assets) -> Result<Self> {
        let window = crate::games::kimaten2::window::TOP;
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
            help: crate::assets::HelpLines::load(assets, "MesButtonHelp"),
            balloon: Default::default(),
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

    fn hit_map(
        &self,
        ui: &Ui,
        m: &crate::games::kimaten2::map::Map,
        x: i32,
        y: i32,
    ) -> Option<Button> {
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
        m: &crate::games::kimaten2::map::Map,
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
            .find(|c| c.area.contains(x, y))
            .or_else(|| {
                self.bar
                    .iter()
                    .find(|c| c.drop(ui) < BAR_DROP - 2 && c.area.contains(x, y - c.drop(ui)))
            })
            .map(|c| Button::Bar(c.id))
    }

    fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        help: Option<(&TextRenderer, Language)>,
    ) -> Result<()> {
        let sheet = assets.image("message_p.akb")?;
        for c in &self.bar {
            let drop = c.drop(ui);
            if drop >= BAR_DROP - 2 && c.id != bar::HIDE {
                continue;
            }
            let active = match c.id {
                bar::AUTO => ui.auto,
                bar::SKIP => ui.skip && !ui.skip_all,
                bar::SKIP_ALL => ui.skip && ui.skip_all,
                bar::LOOP_VOICE => ui.loop_voice,
                bar::MENU => ui.side_menu,
                _ => false,
            };
            c.draw_at(canvas, &sheet, ui, active, drop);
        }
        if ui.waiting {
            // Under Wine: the first three icons of the column at the right
            // of `message_p`, 150 ms each, at the right of the window.
            let frame = (ui.clock / Self::WAIT_FRAME_MS) as u32 % 3;
            let (x, y, w, h) = Self::WAIT_CELL;
            let (dx, dy) = Self::WAIT_POS;
            canvas.draw_part(&sheet, (x, y + h * frame, w, h), dx, dy, false, 255);
        }
        if ui.side_menu {
            let sheet = assets.image("message_menu.akb")?;
            // Backdrop: the dark gradient at the right of the sheet.
            canvas.draw_part(&sheet, (792, 0, 264, 720), 1016, 0, false, 255);
            for c in &self.menu {
                c.draw(canvas, &sheet, ui, false);
            }
        }
        if let Some((tr, language)) = help {
            self.draw_help(canvas, &sheet, ui, tr, language);
        }
        Ok(())
    }

    /// The tooltip of the button under the pointer: a balloon above a bar
    /// button with its point at the button's middle, or left of a side menu
    /// entry pointing right (layout from the executable).
    fn draw_help(
        &self,
        canvas: &mut Canvas,
        sheet: &Arc<silky_format::akb::Image>,
        ui: &Ui,
        tr: &TextRenderer,
        language: Language,
    ) {
        let Some(Button::Bar(id)) = ui.hover else {
            return;
        };
        let menu = if ui.side_menu { &self.menu[..] } else { &[] };
        let Some(cell) = menu.iter().chain(&self.bar).find(|c| c.id == id) else {
            return;
        };
        // Side menu entries that are off show none.
        let in_menu = (MENU_ID..MENU_ID + 0x100).contains(&cell.area.id);
        if in_menu && !ui.enabled(Button::Bar(id)) {
            return;
        }
        // While a mode is on, its button offers to stop it.
        let stop = match id {
            bar::SKIP_ALL if ui.skip && ui.skip_all => Some("既読・未読両方スキップを停止します"),
            bar::SKIP if ui.skip && !ui.skip_all => Some("既読文章スキップを停止します"),
            bar::AUTO if ui.auto => Some("文章の自動再生を停止します"),
            bar::LOOP_VOICE if ui.loop_voice => Some("音声のループ再生を停止します"),
            bar::NOTEBOOK if !ui.notebook => Some("未解放"),
            bar::COLLECTION if !ui.collection => Some("未解放"),
            _ => None,
        };
        let line = |index| self.help.as_ref()?.line(language, index);
        let text = match stop {
            Some(s) => Some(s),
            None => line(match id {
                bar::SKIP_ALL => 0,
                bar::SKIP => 1,
                bar::CONFIG => 2,
                bar::NEXT_CHOICE => 3,
                bar::AUTO => 4,
                bar::BACKLOG => 5,
                bar::LOOP_VOICE => 6,
                bar::SAVE => 7,
                bar::QUICK_SAVE => 8,
                bar::LOAD => 9,
                bar::QUICK_LOAD => 10,
                bar::HIDE => 11,
                bar::PIN => 12,
                bar::TITLE => 13,
                bar::QUIT => 14,
                bar::COSTUME => 15,
                bar::MENU => 16,
                bar::NOTEBOOK => 17,
                bar::COLLECTION => 18,
                _ => return,
            }),
        };
        let Some(text) = text else {
            return;
        };
        // `\` breaks the line. Characters take 16 pixels, single-byte
        // ones 8; the frame grows in 16 pixel steps around the text.
        let lines: Vec<&str> = text.split('\\').collect();
        let advance = |c: char| {
            if c.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&c) {
                8
            } else {
                16
            }
        };
        let text_w = lines
            .iter()
            .map(|l| l.chars().map(advance).sum::<i32>())
            .max()
            .unwrap_or(0);
        let text_h = 16 * lines.len() as i32;
        let cols = (text_w + 15) / 16;
        let (x, y, point) = if in_menu {
            // Side menu: left of the entry, pointing right.
            let k = cell.area.id - MENU_ID;
            let (px, py) = (1016, 24 + 66 * k);
            (
                px - (text_w + 60),
                py,
                (text_w + 30, 30, (140, 773, 30, 16)),
            )
        } else if id == bar::HIDE {
            let top = cell.area.top - text_h - 45;
            (
                cell.area.left - text_w,
                top,
                (text_w, text_h + 30, (140, 796, 30, 30)),
            )
        } else {
            let half = (text_w + 60) / 2;
            let quarter = half / 2;
            let x = cell.area.left + cell.area.width() as i32 / 2 - quarter - half;
            let top = cell.area.top - text_h - 45;
            (
                x,
                top,
                (half - 15 + quarter, text_h + 30, (140, 796, 30, 30)),
            )
        };
        let key = (cols, text_h, point);
        let mut cache = self.balloon.lock().unwrap_or_else(|e| e.into_inner());
        let img = match &*cache {
            Some((k, img)) if *k == key => img.clone(),
            _ => {
                let img = Arc::new(balloon(sheet, cols, text_h, point));
                *cache = Some((key, img.clone()));
                img
            }
        };
        drop(cache);
        canvas.draw_part(&img, (0, 0, img.width, img.height), x, y, false, 255);
        for (i, l) in lines.iter().enumerate() {
            let mut gx = x + 30;
            let base = (y + 30 + 16 * i as i32) as f32 + HELP_BASELINE;
            for c in l.chars() {
                let mut buf = [0; 4];
                tr.draw_outlined(
                    canvas,
                    c.encode_utf8(&mut buf),
                    gx as f32,
                    base,
                    HELP_SIZE,
                    [255, 255, 255, 255],
                    [0, 0, 0, 200],
                );
                gx += advance(c);
            }
        }
    }
}

/// Places of a balloon: where its point goes and the point's cell.
type BalloonPoint = (i32, i32, (u32, u32, u32, u32));

/// A tooltip balloon put together from the pieces on `message_p.akb`:
/// 30 pixel corners, 16 pixel edges and middle, then the point. The pieces
/// replace what is under them, as the engine composes the balloon on its
/// own sprite before showing it.
fn balloon(
    sheet: &silky_format::akb::Image,
    cols: i32,
    text_h: i32,
    point: BalloonPoint,
) -> silky_format::akb::Image {
    let width = (16 * cols + 60) as u32;
    let height = (text_h + 60) as u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let mut copy = |(sx, sy, w, h): (u32, u32, u32, u32), dx: i32, dy: i32| {
        for row in 0..h {
            for col in 0..w {
                let (tx, ty) = (dx + col as i32, dy + row as i32);
                if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
                    continue;
                }
                let (px, py) = (sx + col, sy + row);
                if px >= sheet.width || py >= sheet.height {
                    continue;
                }
                let s = ((py * sheet.width + px) * 4) as usize;
                let d = ((ty as u32 * width + tx as u32) * 4) as usize;
                rgba[d..d + 4].copy_from_slice(&sheet.rgba[s..s + 4]);
            }
        }
    };
    let rows = [(736, 30), (766, 16), (796, 30)];
    let mut dy = 0;
    for (r, &(sy, h)) in rows.iter().enumerate() {
        let count = if r == 1 { text_h / 16 } else { 1 };
        for _ in 0..count {
            copy((50, sy, 30, h), 0, dy);
            for i in 0..cols {
                copy((80, sy, 16, h), 30 + 16 * i, dy);
            }
            copy((110, sy, 30, h), 30 + 16 * cols, dy);
            dy += h as i32;
        }
    }
    let (ax, ay, part) = point;
    copy(part, ax, ay);
    silky_format::akb::Image {
        width,
        height,
        rgba,
    }
}

/// Side menu entries of きまぐれテンプテーション2 (`mesmenu.area` ids).
const MENU_ID: i32 = 3840;
/// Tooltip glyph size and baseline below the top of a line.
const HELP_SIZE: f32 = 12.0;
const HELP_BASELINE: f32 = 11.0;

impl Layout {
    pub fn is_kimaten2(&self) -> bool {
        self.kimaten2.is_some()
    }

    pub fn is_doujin(&self) -> bool {
        self.doujin.is_some()
    }

    pub fn is_kimaten1(&self) -> bool {
        self.k1.is_some()
    }

    /// Whether some byte variables are kept across games (see
    /// [`Layout::global_bytes`]).
    pub fn keeps_global_bytes(&self) -> bool {
        !self.global_bytes().is_empty()
    }

    /// The byte variables kept across games: from
    /// [`crate::system::GLOBAL_BYTES`] on in きまぐれテンプテーション2 and
    /// 景の海のアペイリア; in 同級生 Remake the album and ending flags
    /// (1720 to 1899) and the album, memory reel and music flags (4000 to
    /// 6999; the walking maps keep their own state from 1700 and 7088 on).
    pub fn global_bytes(&self) -> &'static [std::ops::Range<usize>] {
        use crate::system::GLOBAL_BYTES;
        #[allow(clippy::single_range_in_vec_init)]
        const OPEN: &[std::ops::Range<usize>] = &[GLOBAL_BYTES..usize::MAX];
        const DOUK: &[std::ops::Range<usize>] = &[1720..1900, 4000..7000];
        if self.kimaten2.is_some() || self.ring.is_some() {
            OPEN
        } else if self.douk.is_some() {
            DOUK
        } else {
            &[]
        }
    }

    pub fn load(assets: &Assets, language: Language) -> Result<Self> {
        if assets.game().title == silky_format::game::Title::Kimaten2 {
            return Ok(Self {
                language,
                bar: Vec::new(),
                kimaten2: Some(Kimaten2::load(assets)?),
                sheet_title: None,
                ring: None,
                douk: None,
                k1: None,
                nanairo: None,
                doujin: None,
            });
        }
        if assets.game().title.is_apeiria() {
            return Ok(Self {
                language,
                bar: Vec::new(),
                kimaten2: None,
                sheet_title: Some(SheetTitle::load_apeiria(assets)?),
                ring: Some(Ring::load(assets)?),
                douk: None,
                k1: None,
                nanairo: None,
                doujin: None,
            });
        }
        if assets.game().title == silky_format::game::Title::Doukyuusei {
            return Ok(Self {
                language,
                bar: Vec::new(),
                kimaten2: None,
                sheet_title: Some(SheetTitle::load_doukyuusei(assets)?),
                ring: None,
                douk: Some(DoukBar::load(assets)?),
                k1: None,
                nanairo: None,
                doujin: None,
            });
        }
        if assets.game().title == silky_format::game::Title::Kimaten1 {
            return Ok(Self {
                language,
                bar: Vec::new(),
                kimaten2: None,
                sheet_title: None,
                ring: None,
                douk: None,
                k1: Some(crate::games::kimaten1::window::Bar::load(assets)?),
                nanairo: None,
                doujin: None,
            });
        }
        let nanairo = if assets.game().title == silky_format::game::Title::Nanairo {
            Some(crate::games::nanairo::window::Bar::load(assets)?)
        } else {
            None
        };
        let sheet_title = match assets.game().title {
            silky_format::game::Title::Nanairo => Some(SheetTitle::load_nanairo(assets)?),
            silky_format::game::Title::Doujin => Some(SheetTitle::load_doujin(assets)?),
            _ => None,
        };
        // A game not known yet may have no `mes_o.area`: no buttons then.
        let mut bar = if assets.exists(silky_format::game::Kind::Data, "mes_o.area") {
            area::parse(&assets.data("mes_o.area")?)?
        } else {
            Vec::new()
        };
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
            sheet_title,
            ring: None,
            douk: None,
            k1: None,
            nanairo,
            doujin: if assets.game().title == silky_format::game::Title::Doujin {
                Some(crate::games::doujin::window::Bar::load(assets)?)
            } else {
                None
            },
        })
    }

    /// Value the title menu call returns to the script for an entry (see
    /// [`title::result`]); 同級生 Remake starts a new game with 0 and
    /// numbers its extras 257..=260.
    pub fn title_result(&self, entry: usize) -> Option<i32> {
        if self.k1.is_some() {
            return crate::games::kimaten1::title::result(entry);
        }
        if let Some(f) = self.sheet_title.as_ref().and_then(|t| t.result) {
            return f(entry);
        }
        if self.douk.is_some() {
            return match entry {
                title::NEW_GAME => Some(0),
                title::VIEWER => Some(260),
                title::QUIT => None,
                e => title::result(e),
            };
        }
        title::result(entry)
    }

    /// Extras entries and the byte variables that unlock them.
    pub fn title_unlocks(&self) -> &'static [(usize, usize)] {
        match (&self.kimaten2, &self.sheet_title) {
            (Some(_), _) => &title::KIMATEN2_UNLOCK,
            _ if self.k1.is_some() => &crate::games::kimaten1::title::UNLOCKS,
            (None, Some(t)) => t.unlocks,
            _ => &[],
        }
    }

    /// What the arrow keys of a 同級生 Remake selection work from; none
    /// when the game has no such selection up.
    pub fn douk_key_state(&self, ui: &Ui) -> Option<crate::games::doukyuusei::keys::State> {
        use crate::games::doukyuusei::keys::{Hover, State};
        use crate::games::doukyuusei::menu::BUTTONS;
        use crate::games::doukyuusei::select::{self, NEXT, PREV};
        self.douk.as_ref()?;
        if ui.choices.is_empty() {
            return None;
        }
        let count = ui.choices.len();
        let page = ui.choice_page.min(select::last_page(count));
        let range = select::page_range(count, page);
        let (mut focus, mut hover) = (-1, Hover::None);
        match ui.hover {
            Some(Button::Choice(i)) if range.contains(&i) => focus = (i - range.start) as i32,
            Some(Button::ChoicePage(PREV)) => focus = 4,
            Some(Button::ChoicePage(NEXT)) => focus = 5,
            Some(Button::Bar(id)) => {
                if let Some(k) = Self::douk_bottom_id(id) {
                    hover = Hover::Bottom(k);
                } else if let Some(k) = BUTTONS.iter().position(|&b| b == id) {
                    hover = Hover::Menu(k as i32);
                }
            }
            _ => {}
        }
        let mut hidden = [false; 7];
        for (h, &id) in hidden.iter_mut().zip(BUTTONS.iter()) {
            *h = douk_menu_hidden(ui, id);
        }
        Some(State {
            focus,
            hover,
            bars: range.len() as i32,
            page,
            last_page: select::last_page(count),
            menu_open: ui.side_menu,
            hidden,
        })
    }

    /// `mes_o.area` id of a message window button of 同級生 Remake.
    fn douk_bottom_id(id: i32) -> Option<i32> {
        Some(match id {
            bar::SKIP => 0,
            bar::SKIP_ALL => 1,
            bar::AUTO => 2,
            bar::QUICK_SAVE => 3,
            bar::QUICK_LOAD => 4,
            bar::VOICE => 5,
            bar::HIDE => 6,
            _ => return None,
        })
    }

    /// Where the pointer goes for a key target of a 同級生 Remake
    /// selection, if the element is there: the bars and arrows that are
    /// shown, the bottom buttons drawn during a selection (not voice and
    /// close, which go with the window) and the menu buttons that show.
    pub fn douk_key_point(
        &self,
        ui: &Ui,
        t: crate::games::doukyuusei::keys::Target,
    ) -> Option<(i32, i32)> {
        use crate::games::doukyuusei::keys::{Target, MENU_POINTS};
        use crate::games::doukyuusei::menu::BUTTONS;
        use crate::games::doukyuusei::select::{self, Bars, Hit};
        let d = self.douk.as_ref()?;
        let count = ui.choices.len();
        let bars = Bars::on_page(&ui.choices, ui.choice_page);
        match t {
            Target::Bar(i) => {
                let i = bars.range.start + usize::try_from(i).ok()?;
                bars.range.contains(&i).then(|| bars.centre(Hit::Bar(i)))
            }
            Target::Prev => select::prev_shown(count, bars.page).then(|| bars.centre(Hit::Prev)),
            Target::Next => select::next_shown(count, bars.page).then(|| bars.centre(Hit::Next)),
            Target::Bottom(k) => {
                let id = d
                    .buttons
                    .iter()
                    .map(|b| b.0)
                    .find(|&id| Self::douk_bottom_id(id) == Some(k))?;
                if matches!(id, bar::VOICE | bar::HIDE) {
                    return None;
                }
                let a = d.buttons.iter().find(|b| b.0 == id)?.1;
                Some(((a.left + a.right) / 2, (a.top + a.bottom) / 2))
            }
            Target::Menu(k) => {
                let k = usize::try_from(k).ok()?;
                d.menu.as_ref()?;
                let id = *BUTTONS.get(k)?;
                let shown = ui.plate_up() && (k == 0 || ui.side_menu) && !douk_menu_hidden(ui, id);
                shown.then(|| MENU_POINTS[k])
            }
        }
    }

    /// Button under a point (screen pixels).
    pub fn hit(&self, ui: &Ui, text_shown: bool, x: i32, y: i32) -> Option<Button> {
        if let (Some(m), true) = (
            self.douk.as_ref().and_then(|d| d.menu.as_ref()),
            ui.plate_up(),
        ) {
            let walking = matches!(ui.panel, Some(crate::games::doukyuusei::Panel::Town));
            match m.hit(ui.side_menu, x, y) {
                Some(bar::MAP) if !walking => {}
                Some(bar::SCHEDULE) if ui.classic => {}
                Some(id) => return Some(Button::Bar(id)),
                None => {}
            }
        }
        if let (false, Some(d)) = (ui.choices.is_empty(), &self.douk) {
            use crate::games::doukyuusei::select::{Bars, Hit, NEXT, PREV};
            let bars = Bars::on_page(&ui.choices, ui.choice_page);
            return match bars.hit(x, y) {
                // A disabled option takes no press and no hover.
                Some(Hit::Bar(i)) => ui.enabled(Button::Choice(i)).then_some(Button::Choice(i)),
                Some(Hit::Prev) => Some(Button::ChoicePage(PREV)),
                Some(Hit::Next) => Some(Button::ChoicePage(NEXT)),
                None => d.hit(ui, x, y),
            };
        }
        if !ui.choices.is_empty() && self.kimaten2.is_none() && self.ring.is_none() {
            let bars = ChoiceBars::new(&ui.choices);
            return (0..bars.count)
                .find(|&i| {
                    let top = bars.top(i);
                    x >= bars.left
                        && x < bars.left + bars.width()
                        && y >= top
                        && y < top + ChoiceBars::HEIGHT
                })
                .map(Button::Choice);
        }
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
        if let Some(p) = &ui.panel {
            return p.hit(x, y).map(Button::Panel);
        }
        if let Some(s) = ui.k1_menu.as_ref().or(ui.k1.as_ref()) {
            return s
                .hit(x, y)
                .filter(|&id| s.enabled(id))
                .map(Button::Kimaten1);
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
        if let (Some(extras), Some(_)) = (ui.title, &self.k1) {
            return crate::games::kimaten1::title::hit(extras, x, y);
        }
        if let (Some(extras), Some(t)) = (ui.title, &self.sheet_title) {
            return t.hit(extras, x, y);
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
        if let Some(r) = &self.ring {
            return r.hit(ui, x, y);
        }
        if let Some(d) = &self.douk {
            return d.hit(ui, x, y);
        }
        if let Some(k) = &self.k1 {
            return k.hit(ui, x, y);
        }
        if let Some(d) = &self.doujin {
            return d.hit(ui, x, y);
        }
        if let Some(n) = &self.nanairo {
            return n.hit(ui, x, y);
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
        // 先輩が私の妄想にドージンする: the jump to the next unread text or
        // selection covers the whole screen with one picture; plain skipping
        // does not.
        if let (true, true, true, None) = (
            self.doujin.is_some(),
            ui.skip && ui.jumping,
            text_shown,
            ui.title,
        ) {
            let skip = assets.image(crate::games::doujin::window::SKIP_PICTURE)?;
            canvas.draw(&skip, 0, 0, false);
            return Ok(());
        }
        // きまぐれテンプテーション: notices over the scene.
        if let Some(n) = &ui.k1_notices {
            n.draw(canvas, assets, text)?;
        }
        // 同級生 Remake: its own bars, with the bottom buttons of the
        // message window.
        let chrome = ui.chrome();
        if let (Some(plate), Some(d)) = (&ui.plate, &self.douk) {
            if chrome.plate > 0 {
                plate.draw(canvas, assets, self.language, chrome.plate)?;
            }
            if let (Some(m), Some(dx)) = (&d.menu, chrome.menu) {
                use crate::games::doukyuusei::menu::HIDDEN;
                let slide = ui.menu_anim.map(|a| a.slide(ui.clock));
                m.draw(canvas, assets, ui.side_menu, slide, dx, |id| {
                    if douk_menu_hidden(ui, id) {
                        return HIDDEN;
                    }
                    let b = Button::Bar(id);
                    ui.state(b, ui.enabled(b), false) as u32
                })?;
            }
        }
        if let (false, Some(d)) = (ui.choices.is_empty(), &self.douk) {
            use crate::games::doukyuusei::select::{Bars, Hit, NEXT, PREV};
            let bars = Bars::on_page(&ui.choices, ui.choice_page)
                .with_status(&ui.choice_status)
                .with_easy(!ui.classic);
            let font = d.font(assets);
            bars.draw(canvas, assets, font, &ui.choices, |h| {
                let b = match h {
                    Hit::Bar(i) => Button::Choice(i),
                    Hit::Prev => Button::ChoicePage(PREV),
                    Hit::Next => Button::ChoicePage(NEXT),
                };
                // A disabled option keeps its normal row.
                match ui.state(b, ui.enabled(b), false) {
                    State::Disabled => 0,
                    s => s as u32,
                }
            })?;
            return d.draw(canvas, assets, ui, false);
        }
        if !ui.choices.is_empty() && self.kimaten2.is_none() && self.ring.is_none() {
            let sheet = assets.image("message_p.akb")?;
            let bars = ChoiceBars::new(&ui.choices);
            for (i, label) in ui.choices.iter().take(bars.count).enumerate() {
                let state = ui.state(Button::Choice(i), true, false) as u32;
                bars.draw(canvas, &sheet, i, state);
                if let Some(tr) = text {
                    tr.draw_dropped(
                        canvas,
                        label,
                        bars.left as f32 + ChoiceBars::TEXT_LEFT,
                        bars.top(i) as f32 + ChoiceBars::TEXT_BASELINE,
                        ChoiceBars::TEXT_SIZE,
                        [255, 255, 255, 255],
                    );
                }
            }
        } else if !ui.choices.is_empty() {
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
        if let Some(s) = ui.k1_menu.as_ref().or(ui.k1.as_ref()) {
            let id = |b: Option<Button>| match b {
                Some(Button::Kimaten1(id)) => Some(id),
                _ => None,
            };
            let look = crate::games::kimaten1::Look {
                hover: id(ui.hover),
                pressed: id(ui.pressed),
                ms: ui.clock.saturating_sub(ui.k1_since),
                text,
            };
            return s.draw(canvas, assets, &look);
        }
        if let Some(p) = &ui.panel {
            let hover = match ui.hover {
                Some(Button::Panel(id)) => Some(id),
                _ => None,
            };
            let font = self.douk.as_ref().and_then(|d| d.font(assets));
            return p.draw(
                canvas,
                assets,
                self.language,
                |id| {
                    use crate::games::doukyuusei::{picker, room::QUICK_LOAD, Panel};
                    // The room's quick load is gray without a save.
                    let on = match p {
                        Panel::Room(_) => id != QUICK_LOAD as i32 || ui.has_saves,
                        Panel::Name(n) => n.enabled(id),
                        Panel::Picker(k) => match id {
                            picker::BACKLOG => ui.has_log,
                            picker::LOAD | picker::QUICK_LOAD => ui.has_saves,
                            _ => k.enabled(id),
                        },
                        _ => true,
                    };
                    ui.state(Button::Panel(id), on, false) as u32
                },
                hover,
                font,
                ui.room_fx.map(|f| f.pass(ui.clock)),
            );
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
        if let (Some(extras), Some(_)) = (ui.title, &self.k1) {
            let from = (ui.title_shown > ui.title_opened).then_some(!extras);
            return crate::games::kimaten1::title::draw(canvas, assets, ui, extras, from);
        }
        if let (Some(extras), Some(t)) = (ui.title, &self.sheet_title) {
            return t.draw(canvas, assets, ui, extras, self.language);
        }
        if let Some(extras) = ui.title {
            let sheet = assets.localized_image("title_p.akb", self.language)?;
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
            let help = text.filter(|_| ui.tooltips).map(|t| (t, self.language));
            return k.draw(canvas, assets, ui, help);
        }
        if let Some(r) = &self.ring {
            return r.draw(canvas, assets, ui);
        }
        if let Some(d) = &self.douk {
            return d.draw(canvas, assets, ui, true);
        }
        if let Some(k) = &self.k1 {
            return k.draw(canvas, assets, ui);
        }
        if let Some(n) = &self.nanairo {
            return n.draw(canvas, assets, ui);
        }
        if let Some(d) = &self.doujin {
            let help = text.filter(|_| ui.tooltips).map(|t| (t, self.language));
            return d.draw(canvas, assets, ui, help);
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
            let (dx, dy) = if assets.game().engine.profile.name == "engine-2021" {
                WAIT_POS_2021
            } else {
                WAIT_POS
            };
            canvas.draw_part(
                &sheet,
                (x, y + WAIT_CELL * frame, WAIT_CELL, WAIT_CELL),
                dx,
                dy,
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
