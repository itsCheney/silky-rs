//! The name entry of 同級生 Remake (`Game.18`): the hero's name, typed on
//! a letter grid or on the keyboard.
//!
//! Read from the executable and compared with a screenshot of the
//! original. `keyboard_bg.akb` (a script background) holds the dark
//! panel; `keyboard_p_<lang>.akb` the plates, buttons and grid cells.
//! The grid is 18 columns of 58 x 58 cells from (450, 51), 60 x 58
//! apart; a click adds the cell's letter (English: the half-width one)
//! at the cursor. The name holds 10 letters (English) or 5 characters
//! (Chinese), starts as the default name each time, and is shown from
//! (742, 958), 30 or 60 pixels per character. Confirm is gray while the
//! name is empty; a name of the cast shows a notice (`dialog2_p`) and
//! stays. The name is kept at byte 18500 of the raw area
//! ([`NAME_AT`]). See `docs/games/doukyuusei.md`.

use silky_format::area::Area;

use super::keys::Arrow;
use super::room::Click;
use super::window::Font;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

/// Where the name is kept in the raw area (NUL-terminated UTF-8) and
/// how many bytes the field has.
pub const NAME_AT: usize = 18500;
pub const NAME_BYTES: usize = 100;

/// Hot spot ids of `NameInput.area`. The ten tabs (0 to 9), the three
/// kinds (0x100 to 0x102), the two page scrolls (0x300, 0x301) and the
/// bar (0x600) have no picture in this build and do nothing, but take the
/// focus.
pub const KIND: i32 = 0x100;
pub const SCROLL: i32 = 0x300;
pub const HIDDEN_BAR: i32 = 0x600;
pub const LEFT: i32 = 512;
pub const RIGHT: i32 = 513;
pub const BACKSPACE: i32 = 514;
pub const DEFAULT: i32 = 1024;
pub const CONFIRM: i32 = 1280;
/// Grid cell ids: `CELL + 18 * row + column`.
pub const CELL: i32 = 0x1000;
/// The Close button of the notice.
pub const CLOSE: i32 = 0x2000;

const SHEET: &str = "keyboard_p.akb";
const DIALOG: &str = "dialog2_p.akb";
const COLUMNS: i32 = 18;
const CELL_SIZE: u32 = 58;
const GRID: (i32, i32) = (450, 51);
const GRID_STEP: (i32, i32) = (60, 58);
const GLYPH: (i32, i32) = (10, 8);
const CELL_PART: (u32, u32) = (864, 376);
const BAR: (u32, u32, u32, u32) = (0, 807, 1157, 64);
const BAR_POS: (i32, i32) = (268, 874);
const PLATE: (u32, u32, u32, u32) = (0, 727, 878, 80);
const PLATE_POS: (i32, i32) = (547, 942);
const LABEL: (u32, u32, u32, u32) = (922, 434, 120, 64);
const LABEL_POS: (i32, i32) = (269, 874);
/// Arrow and backspace buttons: left, top of the first state, width,
/// row step, and where they go.
const ARROW_L: (u32, i32) = (564, 946);
const ARROW_R: (u32, i32) = (752, 1156);
const BACK_PART: (u32, i32) = (940, 1366);
const ARROW_TOP: (u32, u32, i32) = (144, 58, 810);
const ARROW_W: u32 = 188;
const BACK_W: u32 = 238;
const DEFAULT_PART: (u32, u32, u32, u32) = (0, 376, 264, 86);
const DEFAULT_POS: (i32, i32) = (268, 940);
const CONFIRM_PART: (u32, u32, u32, u32) = (1178, 0, 234, 154);
const CONFIRM_POS: (i32, i32) = (1436, 875);
const NAME_POS: (i32, i32) = (742, 958);
const CARET_X: i32 = 733;
const CARET_Y: i32 = 949;
const BAR_CARET: (i32, i32, u32, u32) = (398, 886, 2, 40);
const PINK_BOX: (u32, u32, u32, u32) = (1040, 0, 59, 59);
const BLINK_MS: u64 = 300;
const WHITE: [u8; 4] = [255, 255, 255, 255];
const DIM: [u8; 4] = [0, 0, 0, 0x7f];
const NOTICE_BOX: (u32, u32, u32, u32) = (0, 0, 880, 250);
const NOTICE_POS: (i32, i32) = (520, 415);
const NOTICE_TEXT: (u32, u32, u32, u32) = (0, 250, 805, 106);
const NOTICE_TEXT_POS: (i32, i32) = (558, 444);
const CLOSE_AT: (i32, i32, u32, u32) = (849, 560, 222, 73);
/// The click sound.
pub const OK_SOUND: &str = "ses_001.wav";
/// The cancel sound.
pub const CANCEL_SOUND: &str = "ses_002.wav";

/// Grid rows as shown; `_` is an empty cell.
const ROWS_EN: [&str; 6] = [
    "ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲ",
    "ＳＴＵＶＷＸＹＺ__________",
    "ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒ",
    "ｓｔｕｖｗｘｙｚ__________",
    "０１２３４５６７８９________",
    "－：；（）！＃＄％＆______",
];
/// What a cell of the English grid inserts.
const KEYS_EN: [&str; 6] = [
    "ABCDEFGHIJKLMNOPQR",
    "STUVWXYZ",
    "abcdefghijklmnopqr",
    "stuvwxyz",
    "0123456789",
    "-:;()!#$%&",
];
const ROWS_CN: [&str; 7] = [
    "ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲ",
    "ＳＴＵＶＷＸＹＺ__________",
    "ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒ",
    "ｓｔｕｖｗｘｙｚ__________",
    "０１２３４５６７８９________",
    "￥－：；（）！＃＄％＆♪______",
    "☆★○●◎◇◆□■△▲▽▼_____",
];

/// Names of the cast, which cannot be used.
const RESERVED: [&str; 142] = [
    "舞",
    "さとみ",
    "くるみ",
    "美穂",
    "美沙",
    "夏子",
    "真子",
    "亜子",
    "よしこ",
    "かおり",
    "麗子",
    "ひろみ",
    "やよい",
    "ちはる",
    "一哉",
    "健二",
    "冬彦",
    "太郎",
    "マタロウ",
    "桜木舞",
    "黒川さとみ",
    "仁科くるみ",
    "鈴木美穂",
    "田中美沙",
    "正樹夏子",
    "斎藤真子",
    "斎藤亜子",
    "芹沢よしこ",
    "成瀬かおり",
    "真行司麗子",
    "田町ひろみ",
    "草薙やよい",
    "佐久間ちはる",
    "坂上一哉",
    "相原健二",
    "間太郎",
    "くるみの父",
    "さとみの母",
    "一哉の母",
    "太郎の母",
    "美沙の母",
    "美穂の母",
    "おでん屋のおやじ",
    "おねーさん",
    "おばさん",
    "おやじ",
    "さとし",
    "じじい",
    "ちさと",
    "つまづき石の精",
    "にーちゃん",
    "にいちゃん",
    "はなこ",
    "ひろし",
    "まみ",
    "みずほ",
    "やんきい",
    "よい",
    "らあむ",
    "りか",
    "アナウンス",
    "ガラの悪い男",
    "クイズ太郎",
    "クイズ次郎",
    "クイズ三郎",
    "クイズ四郎",
    "クイズ五郎",
    "ケンちゃん",
    "スーツの精",
    "タクシーの運転手",
    "バーテン",
    "バイオリンの生徒",
    "バイオリンの先生",
    "ピアノの生徒",
    "ピアノの先生",
    "ベンチの精",
    "フケてて肌がフニャフニャしてる女",
    "フケてて肌がフニャフニャしてて入れ歯の女",
    "フケてる女",
    "マスター",
    "モデル",
    "ランプの精",
    "レイラ",
    "悪ガキども",
    "悪ガキ其の一",
    "悪ガキ其の二",
    "悪徳じじい",
    "医者",
    "一つ目小僧",
    "因業ばばあ",
    "家政婦",
    "花屋のおやじ",
    "画家",
    "看護婦",
    "係員",
    "恵子",
    "見るからに暴漢",
    "現場監督",
    "呼び込み",
    "行商人",
    "黒服の男",
    "室外機の精",
    "社長",
    "受け付け",
    "受付の看護婦",
    "受付嬢",
    "祝福する人々",
    "女",
    "女の子",
    "女の子の声",
    "女の子其の１",
    "女の子其の２",
    "女の子其の３",
    "女の人",
    "女の声",
    "女性の声",
    "場内アナウンス",
    "神の声",
    "水泳部員",
    "酔っぱらい",
    "政義",
    "占いばばあ",
    "太った看護婦",
    "男",
    "男の子",
    "男の人",
    "男の声",
    "天の声",
    "店主",
    "電話の相手",
    "秘密の家の主",
    "美人秘書",
    "部長",
    "変な男",
    "陸上部女子一同",
    "隣で着替えてる男",
    "隣の客",
    "隣の男",
    "隣の部屋の客",
    "隣の隣の部屋の客",
    "霊媒師",
    "路路園の店主",
];

#[derive(Debug, Clone)]
pub struct Name {
    chinese: bool,
    pub name: Vec<char>,
    pub cursor: usize,
    blink: bool,
    since: u64,
    /// The "not available" notice is up.
    pub notice: bool,
    sound: bool,
    /// The hot spots of `NameInput.area`, in file order.
    areas: Vec<Area>,
    /// The item just activated: it is drawn normal until the focus leaves.
    quiet: Option<i32>,
    /// The input method's composition, shown after the name.
    pub preedit: String,
}

/// The characters a paste keeps in English: the program's table.
const PASTE_KEEPS: &str =
    " !\"#$%&'()*,-./0123456789:;>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\abcdefghijklmnopqrstuvwxyz}~×àéö—";

/// A paste puts in at most this many columns; in the other branch it
/// overwrites within the first five characters.
const PASTE_COLUMNS: usize = 10;
const PASTE_CHARS: usize = 5;
/// Where the composition starts, and its height.
const PREEDIT_AT: (i32, i32) = (400, 886);

impl Name {
    pub fn new(language: Language, areas: Vec<Area>) -> Self {
        let mut n = Self {
            chinese: language != Language::English,
            name: Vec::new(),
            cursor: 0,
            blink: true,
            since: 0,
            notice: false,
            sound: false,
            areas,
            quiet: None,
            preedit: String::new(),
        };
        n.reset(language);
        n
    }

    fn reset(&mut self, language: Language) {
        self.name = super::default_hero_name(language).chars().collect();
        self.cursor = self.name.len();
        self.blink = true;
        self.since = 0;
    }

    fn language(&self) -> Language {
        if self.chinese {
            Language::SimplifiedChinese
        } else {
            Language::English
        }
    }

    fn max(&self) -> usize {
        if self.chinese {
            5
        } else {
            10
        }
    }

    pub fn text(&self) -> String {
        self.name.iter().collect()
    }

    /// Confirm is gray while there is no name.
    pub fn enabled(&self, id: i32) -> bool {
        id != CONFIRM || !self.name.is_empty()
    }

    /// Whether the last click asks for the click sound (cleared by the
    /// call).
    pub fn take_sound(&mut self) -> bool {
        std::mem::take(&mut self.sound)
    }

    /// Whether typed text is taken now.
    pub fn typing(&self) -> bool {
        !self.notice
    }

    fn rows(&self) -> &'static [&'static str] {
        if self.chinese {
            &ROWS_CN
        } else {
            &ROWS_EN
        }
    }

    /// The letter of a grid cell, if there is a cell.
    fn cell_char(&self, id: i32) -> Option<char> {
        let (row, col) = (
            ((id - CELL) / COLUMNS) as usize,
            ((id - CELL) % COLUMNS) as usize,
        );
        let shown = self.rows().get(row)?.chars().nth(col)?;
        if shown == '_' {
            return None;
        }
        if self.chinese {
            Some(shown)
        } else {
            KEYS_EN.get(row)?.chars().nth(col)
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let inside = |l: i32, t: i32, w: u32, h: u32| {
            x >= l && y >= t && x < l + w as i32 && y < t + h as i32
        };
        if self.notice {
            let (l, t, w, h) = CLOSE_AT;
            return inside(l, t, w, h).then_some(CLOSE);
        }
        // The areas of the data come first, in file order, then the cells.
        if let Some(a) = self.areas.iter().find(|a| a.contains(x, y)) {
            return Some(a.id);
        }
        let (cx, cy) = (x - GRID.0, y - GRID.1);
        if cx >= 0 && cy >= 0 {
            let (col, row) = (cx / GRID_STEP.0, cy / GRID_STEP.1);
            let inner =
                (cx % GRID_STEP.0 < CELL_SIZE as i32) && (cy % GRID_STEP.1 < CELL_SIZE as i32);
            let id = CELL + COLUMNS * row + col;
            if inner && col < COLUMNS && self.cell_char(id).is_some() {
                return Some(id);
            }
        }
        None
    }

    /// Whether `id` is a cell of the grid with a letter.
    fn is_cell(&self, id: i32) -> bool {
        id >= CELL && self.cell_char(id).is_some()
    }

    /// Whether the page has an item with this id.
    fn has(&self, id: i32) -> bool {
        self.is_cell(id) || self.areas.iter().any(|a| a.id == id)
    }

    fn rows_used(&self) -> usize {
        self.rows().len()
    }

    /// Whether grid cell (`row`, `col`) holds a letter.
    fn used(&self, row: usize, col: usize) -> bool {
        row < self.rows_used()
            && col < COLUMNS as usize
            && self.rows()[row].chars().nth(col).is_some_and(|c| c != '_')
    }

    /// The id of the lowest cell of a column.
    fn bottom(&self, col: usize) -> Option<i32> {
        (0..self.rows_used())
            .rev()
            .find(|&r| self.used(r, col))
            .map(|r| CELL + COLUMNS * r as i32 + col as i32)
    }

    /// The focus after an arrow key from `from` (-1 for none), or `None`
    /// when it stays. The ten tabs and the two scrolls are not drawn but
    /// take the focus; nothing wraps.
    pub fn arrow(&self, from: i32, key: Arrow) -> Option<i32> {
        use Arrow::*;
        if self.notice {
            return Some(CLOSE);
        }
        let target = match from {
            -1 => Some(if key == Up { 0 } else { CONFIRM }),
            0..=9 => Some(CONFIRM),
            0x100 => match key {
                Up => self.bottom(0),
                Down => Some(DEFAULT),
                Left => None,
                Right => Some(0x101),
            },
            0x101 => match key {
                Up => self.bottom(2),
                Down => Some(DEFAULT),
                Left => Some(0x100),
                Right => Some(0x102),
            },
            0x102 => match key {
                Up => self.bottom(6),
                Down => Some(DEFAULT),
                Left => Some(0x101),
                Right => Some(LEFT),
            },
            LEFT => match key {
                Up => self.bottom(9),
                Down => Some(CONFIRM),
                Left => Some(0x102),
                Right => Some(RIGHT),
            },
            RIGHT => match key {
                Up => self.bottom(13),
                Down => Some(CONFIRM),
                Left => Some(LEFT),
                Right => Some(BACKSPACE),
            },
            BACKSPACE => match key {
                Up => self.bottom(16),
                Down => Some(CONFIRM),
                Left => Some(RIGHT),
                Right => None,
            },
            0x300 | 0x301 => match key {
                Right => None,
                _ => Some(CONFIRM),
            },
            DEFAULT => match key {
                Up => Some(0x100),
                Right => Some(CONFIRM),
                _ => None,
            },
            CONFIRM => match key {
                Up => Some(BACKSPACE),
                Left => Some(DEFAULT),
                _ => None,
            },
            id if self.is_cell(id) => self.cell_arrow(id, key),
            _ => None,
        };
        target.filter(|&t| self.has(t))
    }

    fn cell_arrow(&self, id: i32, key: Arrow) -> Option<i32> {
        let (row, col) = (
            ((id - CELL) / COLUMNS) as usize,
            ((id - CELL) % COLUMNS) as usize,
        );
        let cell = |r: usize, c: usize| CELL + COLUMNS * r as i32 + c as i32;
        match key {
            Arrow::Up => (0..row)
                .rev()
                .find(|&r| self.used(r, col))
                .map(|r| cell(r, col)),
            Arrow::Down => (row + 1..self.rows_used())
                .find(|&r| self.used(r, col))
                .map(|r| cell(r, col))
                .or(Some(match col {
                    0 => 0x100,
                    1..=3 => 0x101,
                    4..=7 => 0x102,
                    8..=11 => LEFT,
                    12..=14 => RIGHT,
                    _ => BACKSPACE,
                })),
            Arrow::Left => (0..col)
                .rev()
                .find(|&c| self.used(row, c))
                .map(|c| cell(row, c)),
            Arrow::Right => (col + 1..COLUMNS as usize)
                .find(|&c| self.used(row, c))
                .map(|c| cell(row, c)),
        }
    }

    /// The focus moved (or left, with `None`): an item drawn normal after
    /// its activation goes back to following the focus.
    pub fn set_focus(&mut self, id: Option<i32>) {
        if id != self.quiet {
            self.quiet = None;
        }
    }

    /// Whether `id` is the item just activated, which draws normal.
    fn calm(&self, id: i32) -> bool {
        self.quiet == Some(id)
    }

    fn insert(&mut self, c: char) -> bool {
        if self.cursor >= self.max() {
            return false;
        }
        self.name.insert(self.cursor.min(self.name.len()), c);
        self.name.truncate(self.max());
        self.cursor += 1;
        self.blink = true;
        self.since = 0;
        true
    }

    fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        if self.cursor < self.name.len() {
            self.name.remove(self.cursor);
        }
        self.blink = true;
        self.since = 0;
        true
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.notice {
            if id == CLOSE {
                self.notice = false;
                self.sound = true;
            }
            return Click::None;
        }
        // Whatever is activated draws normal until the focus moves.
        self.quiet = Some(id);
        match id {
            i if i >= CELL => {
                if let Some(c) = self.cell_char(i) {
                    self.sound = self.insert(c);
                }
            }
            // The caret moves only with these two buttons, and with the
            // click sound only when it moves.
            LEFT if self.cursor > 0 => {
                self.cursor -= 1;
                self.sound = true;
            }
            RIGHT if self.cursor < self.max().min(self.name.len()) => {
                self.cursor += 1;
                self.sound = true;
            }
            BACKSPACE if self.cursor > 0 => {
                self.backspace();
                self.sound = true;
            }
            DEFAULT => {
                self.reset(self.language());
                self.sound = true;
            }
            CONFIRM if !self.name.is_empty() => {
                self.sound = true;
                if RESERVED.contains(&self.text().as_str()) {
                    self.notice = true;
                } else {
                    return Click::Name;
                }
            }
            _ => {}
        }
        Click::None
    }

    /// The secondary click and Escape: they close the notice and do
    /// nothing else on the page. Whether the notice was up.
    pub fn secondary(&mut self) -> bool {
        std::mem::take(&mut self.notice)
    }

    /// Backspace typed: erases before the cursor without the sound; the
    /// caret goes off. Also closes the notice.
    pub fn cancel(&mut self) {
        if self.notice {
            self.notice = false;
            return;
        }
        self.backspace();
        self.blink = false;
    }

    /// Typed or committed text. English takes letters and digits at the
    /// end of the name; Chinese puts the text in at the cursor as
    /// full-width characters. Returns whether the name changed.
    pub fn type_text(&mut self, text: &str) -> bool {
        if self.notice {
            return false;
        }
        let before = (self.name.clone(), self.cursor);
        for c in text.chars() {
            if c == '\u{8}' {
                self.cancel();
            } else if self.chinese {
                let c = match c {
                    ' ' => '\u{3000}',
                    '!'..='~' => char::from_u32(c as u32 - 0x21 + 0xff01).unwrap_or(c),
                    c if c.is_control() => continue,
                    c => c,
                };
                self.insert(c);
            } else if c.is_ascii_alphanumeric() {
                self.name.push(c);
                self.name.truncate(self.max());
                self.cursor = self.name.len();
                self.blink = true;
                self.since = 0;
            }
        }
        (self.name.clone(), self.cursor) != before
    }

    /// Ctrl+V: the clipboard's text. English keeps only what the table
    /// of the program allows (no wide character); Chinese keeps it all.
    /// Past the end of the name the text replaces what follows the
    /// caret, cut to ten columns; inside it, it overwrites in place, as
    /// the program does, with five characters as the bound whatever the
    /// language. Returns whether the name or caret changed.
    pub fn paste(&mut self, clip: &str) -> bool {
        if self.notice {
            return false;
        }
        let before = (self.name.clone(), self.cursor);
        let clip: Vec<char> = if self.chinese {
            clip.chars().collect()
        } else {
            clip.chars().filter(|c| PASTE_KEEPS.contains(*c)).collect()
        };
        let caret = self.cursor.min(self.name.len());
        if caret + clip.len() > self.name.len() {
            let mut text: String = self.name[..caret].iter().chain(clip.iter()).collect();
            text = super::savenote::truncate(&text, PASTE_COLUMNS);
            self.name = text.chars().collect();
            self.cursor = self.name.len();
        } else {
            let n = clip.len().min(PASTE_CHARS.saturating_sub(caret));
            self.name.splice(caret..caret + n, clip.iter().copied());
            self.cursor = (caret + clip.len()).min(PASTE_CHARS);
            let text: String = self.name.iter().collect();
            self.name = super::savenote::truncate(&text, PASTE_COLUMNS)
                .chars()
                .collect();
            self.cursor = self.cursor.min(self.name.len());
        }
        self.blink = true;
        self.since = 0;
        (self.name.clone(), self.cursor) != before
    }

    /// Advances the caret's blink; true when it flipped.
    pub fn tick(&mut self, ms: u64) -> bool {
        self.since += ms;
        let mut flipped = false;
        while self.since >= BLINK_MS {
            self.since -= BLINK_MS;
            self.blink = !self.blink;
            flipped = true;
        }
        flipped
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed, 3 gray) of
    /// an id.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
        font: Option<&Font>,
    ) -> Result<()> {
        let sheet = assets.localized_image(SHEET, language)?;
        // An item just activated is drawn normal, not as hovered.
        let state = |id: i32| {
            let s = state(id);
            if s == 1 && self.calm(id) {
                0
            } else {
                s
            }
        };
        let part = |c: &mut Canvas, p: (u32, u32, u32, u32), at: (i32, i32)| {
            c.draw_part(&sheet, p, at.0, at.1, false, 255)
        };
        part(canvas, BAR, BAR_POS);
        part(canvas, PLATE, PLATE_POS);
        part(canvas, LABEL, LABEL_POS);
        for (id, (sx, x)) in [(LEFT, ARROW_L), (RIGHT, ARROW_R), (BACKSPACE, BACK_PART)] {
            let w = if id == BACKSPACE { BACK_W } else { ARROW_W };
            let s = state(id).min(2);
            part(
                canvas,
                (sx, ARROW_TOP.0 + ARROW_TOP.1 * s, w, ARROW_TOP.1),
                (x, ARROW_TOP.2),
            );
        }
        let s = state(DEFAULT).min(2);
        let (x, y, w, h) = DEFAULT_PART;
        part(canvas, (x, y + h * s, w, h), DEFAULT_POS);
        let s = if self.name.is_empty() {
            3
        } else {
            state(CONFIRM).min(3)
        };
        let (x, _, w, h) = CONFIRM_PART;
        part(canvas, (x, h * s, w, h), CONFIRM_POS);
        for (row, line) in self.rows().iter().enumerate() {
            for (col, c) in line.chars().enumerate() {
                if c == '_' {
                    continue;
                }
                let id = CELL + COLUMNS * row as i32 + col as i32;
                let s = state(id).min(2);
                let at = (
                    GRID.0 + GRID_STEP.0 * col as i32,
                    GRID.1 + GRID_STEP.1 * row as i32,
                );
                part(
                    canvas,
                    (
                        CELL_PART.0,
                        CELL_PART.1 + CELL_SIZE * s,
                        CELL_SIZE,
                        CELL_SIZE,
                    ),
                    at,
                );
                // The bitmap font lacks most full-width letters, so English
                // draws the letter it inserts, centred in the cell.
                if let (Some(font), Some(g)) = (font, self.cell_char(id)) {
                    let dx = if g.is_ascii() { GLYPH.0 + 10 } else { GLYPH.0 };
                    font.draw_char(canvas, g, at.0 + dx, at.1 + GLYPH.1, false, 255);
                }
            }
        }
        if self.blink {
            let (x, y, w, h) = BAR_CARET;
            canvas.fill(x, y, w, h, WHITE);
            let (max, pitch) = if self.chinese { (5, 60) } else { (10, 30) };
            if self.cursor < max {
                let x = CARET_X + pitch * self.cursor as i32;
                if self.chinese {
                    part(canvas, PINK_BOX, (x, CARET_Y));
                } else {
                    canvas.fill(x, CARET_Y, 2, 59, WHITE);
                }
            }
        }
        if let Some(font) = font {
            let mut x = NAME_POS.0;
            for &c in &self.name {
                font.draw_char(canvas, c, x, NAME_POS.1, false, 255);
                x += if c.is_ascii() { 30 } else { 60 };
            }
            // The composition of an input method goes in the bar, while
            // the name has room.
            if self.chinese && !self.preedit.is_empty() && self.name.len() < 5 {
                font.draw_composition(
                    canvas,
                    &self.preedit,
                    PREEDIT_AT.0,
                    PREEDIT_AT.1,
                    PREEDIT_AT.0 + 998,
                );
            }
        }
        if self.notice {
            let dialog = assets.localized_image(DIALOG, language)?;
            canvas.fill(0, 0, 1920, 1080, DIM);
            let draw = |c: &mut Canvas, p: (u32, u32, u32, u32), at: (i32, i32)| {
                c.draw_part(&dialog, p, at.0, at.1, false, 255)
            };
            draw(canvas, NOTICE_BOX, NOTICE_POS);
            draw(canvas, NOTICE_TEXT, NOTICE_TEXT_POS);
            let (l, t, w, h) = CLOSE_AT;
            let s = state(CLOSE).min(3);
            draw(canvas, (880, 73 * s, w, h), (l, t));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `NameInput.area` as the game ships it.
    fn areas() -> Vec<Area> {
        let r = |id, left, top, right, bottom| Area {
            id,
            left,
            top,
            right,
            bottom,
        };
        let mut v: Vec<Area> = (0..10)
            .map(|i| r(i, 368 + 120 * i, 766, 472 + 120 * i, 802))
            .collect();
        v.extend([
            r(256, 316, 810, 504, 868),
            r(257, 526, 810, 714, 868),
            r(258, 736, 810, 924, 868),
            r(512, 946, 810, 1134, 868),
            r(513, 1156, 810, 1344, 868),
            r(514, 1366, 810, 1604, 868),
            r(768, 1577, 45, 1652, 396),
            r(769, 1577, 405, 1652, 756),
            r(1024, 268, 940, 532, 1026),
            r(1280, 1436, 875, 1670, 1029),
            r(1536, 268, 874, 1425, 938),
        ]);
        v
    }

    fn cell(row: i32, col: i32) -> i32 {
        CELL + COLUMNS * row + col
    }

    #[test]
    fn hidden_items_take_the_focus_and_do_nothing() {
        let mut n = Name::new(Language::English, areas());
        assert_eq!(n.hit(400, 780), Some(0));
        assert_eq!(n.hit(1500, 780), Some(9));
        assert_eq!(n.hit(400, 840), Some(0x100));
        assert_eq!(n.hit(1000, 900), Some(0x600));
        assert_eq!(n.hit(1600, 100), Some(0x300));
        let before = (n.text(), n.cursor);
        for id in [0, 5, 0x100, 0x102, 0x300, 0x301, 0x600] {
            assert_eq!(n.click(id), Click::None);
            assert!(!n.take_sound(), "{id}");
        }
        assert_eq!((n.text(), n.cursor), before);
    }

    #[test]
    fn the_caret_buttons_move_it_only_when_they_can() {
        let mut n = Name::new(Language::English, areas());
        n.click(RIGHT);
        assert!(!n.take_sound());
        n.click(LEFT);
        assert!(n.take_sound());
        assert_eq!(n.cursor, 6);
        n.click(BACKSPACE);
        assert!(n.take_sound());
        assert_eq!(n.text(), "Takuru");
        n.cursor = 0;
        n.click(LEFT);
        n.click(BACKSPACE);
        assert!(!n.take_sound());
        assert_eq!(n.text(), "Takuru");
    }

    #[test]
    fn arrows_from_nothing_and_from_hidden_items() {
        use Arrow::*;
        let n = Name::new(Language::English, areas());
        assert_eq!(n.arrow(-1, Up), Some(0));
        for k in [Down, Left, Right] {
            assert_eq!(n.arrow(-1, k), Some(CONFIRM));
        }
        for id in 0..=9 {
            for k in [Up, Down, Left, Right] {
                assert_eq!(n.arrow(id, k), Some(CONFIRM));
            }
        }
        for k in [Up, Down, Left, Right] {
            assert_eq!(n.arrow(0x600, k), None);
        }
        assert_eq!(n.arrow(0x300, Up), Some(CONFIRM));
        assert_eq!(n.arrow(0x301, Right), None);
    }

    #[test]
    fn arrows_between_the_buttons() {
        use Arrow::*;
        let n = Name::new(Language::English, areas());
        assert_eq!(n.arrow(0x100, Left), None);
        assert_eq!(n.arrow(0x100, Right), Some(0x101));
        assert_eq!(n.arrow(0x102, Right), Some(LEFT));
        assert_eq!(n.arrow(0x100, Down), Some(DEFAULT));
        assert_eq!(n.arrow(LEFT, Down), Some(CONFIRM));
        assert_eq!(n.arrow(BACKSPACE, Right), None);
        assert_eq!(n.arrow(DEFAULT, Up), Some(0x100));
        assert_eq!(n.arrow(DEFAULT, Down), None);
        assert_eq!(n.arrow(DEFAULT, Right), Some(CONFIRM));
        assert_eq!(n.arrow(CONFIRM, Left), Some(DEFAULT));
        assert_eq!(n.arrow(CONFIRM, Up), Some(BACKSPACE));
        assert_eq!(n.arrow(CONFIRM, Right), None);
        // Up from the visible buttons lands on the lowest cell of a column.
        assert_eq!(n.arrow(LEFT, Up), Some(cell(5, 9)));
        assert_eq!(n.arrow(RIGHT, Up), Some(cell(2, 13)));
        assert_eq!(n.arrow(BACKSPACE, Up), Some(cell(2, 16)));
        assert_eq!(n.arrow(0x100, Up), Some(cell(5, 0)));
        let cn = Name::new(Language::SimplifiedChinese, areas());
        assert_eq!(cn.arrow(LEFT, Up), Some(cell(6, 9)));
        assert_eq!(cn.arrow(RIGHT, Up), Some(cell(2, 13)));
    }

    #[test]
    fn arrows_in_the_grid_skip_empty_cells_and_do_not_wrap() {
        use Arrow::*;
        let n = Name::new(Language::English, areas());
        // Row 1 holds eight letters: Right from the last one stops.
        assert_eq!(n.arrow(cell(1, 7), Right), None);
        assert_eq!(n.arrow(cell(1, 0), Left), None);
        assert_eq!(n.arrow(cell(1, 3), Right), Some(cell(1, 4)));
        // Up skips nothing here, but stops at the top.
        assert_eq!(n.arrow(cell(0, 3), Up), None);
        assert_eq!(n.arrow(cell(2, 3), Up), Some(cell(1, 3)));
        // Column 12 is empty below row 2 in English: Down leaves the grid.
        assert_eq!(n.arrow(cell(2, 12), Down), Some(RIGHT));
        // Column 9 goes on to row 5.
        assert_eq!(n.arrow(cell(2, 9), Down), Some(cell(4, 9)));
        assert_eq!(n.arrow(cell(4, 9), Down), Some(cell(5, 9)));
        assert_eq!(n.arrow(cell(5, 9), Down), Some(LEFT));
        // Down from the lowest cell goes by column.
        assert_eq!(n.arrow(cell(5, 0), Down), Some(0x100));
        assert_eq!(n.arrow(cell(5, 3), Down), Some(0x101));
        assert_eq!(n.arrow(cell(3, 3), Down), Some(cell(4, 3)));
        assert_eq!(n.arrow(cell(2, 16), Down), Some(BACKSPACE));
        assert_eq!(n.arrow(cell(2, 15), Down), Some(BACKSPACE));
        assert_eq!(n.arrow(cell(2, 14), Down), Some(RIGHT));
        assert_eq!(n.arrow(cell(2, 11), Down), Some(LEFT));
        assert_eq!(n.arrow(cell(2, 7), Down), Some(cell(3, 7)));
        assert_eq!(n.arrow(cell(3, 7), Down), Some(cell(4, 7)));
        assert_eq!(n.arrow(cell(4, 7), Down), Some(cell(5, 7)));
        assert_eq!(n.arrow(cell(5, 7), Down), Some(0x102));
    }

    #[test]
    fn an_activated_item_draws_normal_until_the_focus_moves() {
        let mut n = Name::new(Language::English, areas());
        n.click(LEFT);
        assert!(n.calm(LEFT));
        n.set_focus(Some(LEFT));
        assert!(n.calm(LEFT));
        n.set_focus(Some(RIGHT));
        assert!(!n.calm(LEFT));
    }

    #[test]
    fn paste_in_english_keeps_the_table_and_appends_past_the_end() {
        let mut n = Name::new(Language::English, areas());
        n.cursor = 3;
        // Inside the name: the filtered "Z1" overwrites in place.
        assert!(n.paste("Z_+あ[1]"));
        assert_eq!(n.text(), "TakZ1ou");
        assert_eq!(n.cursor, 5);
        // Past the end: the text after the caret goes, ten columns bound.
        n.paste(&"x".repeat(20));
        assert_eq!(n.text(), "TakZ1xxxxx");
        assert_eq!(n.cursor, 10);
    }

    #[test]
    fn paste_inside_the_name_overwrites_in_place() {
        let mut n = Name::new(Language::English, areas());
        n.cursor = 1;
        assert!(n.paste("AB"));
        assert_eq!(n.text(), "TABurou");
        assert_eq!(n.cursor, 3);
        // Five characters bound the overwrite and the caret.
        n.cursor = 4;
        n.paste("Q");
        assert_eq!(n.text(), "TABuQou");
        assert_eq!(n.cursor, 5);
        n.cursor = 6;
        assert!(n.paste(""));
        assert_eq!(n.cursor, 5);
    }

    #[test]
    fn paste_in_chinese_is_not_filtered() {
        let mut n = Name::new(Language::SimplifiedChinese, areas());
        n.cursor = 0;
        assert!(n.paste("あ_\n"));
        assert_eq!(n.text(), "あ_\n");
        n.paste("拓郎拓郎拓郎");
        assert_eq!(n.text(), "あ_\n拓郎拓");
        assert!(!{
            n.notice = true;
            n.paste("x")
        });
    }

    #[test]
    fn edit() {
        let mut n = Name::new(Language::English, areas());
        assert_eq!((n.text().as_str(), n.cursor), ("Takurou", 7));
        // Cell (0, 0) is "A"; the grid is shown full-width.
        assert_eq!(n.hit(460, 60), Some(CELL));
        assert_eq!(n.click(CELL), Click::None);
        assert_eq!(n.text(), "TakurouA");
        for _ in 0..4 {
            n.click(CELL);
        }
        assert_eq!(n.text(), "TakurouAAA");
        n.click(BACKSPACE);
        assert_eq!(n.text(), "TakurouAA");
        n.type_text("zz9!");
        assert_eq!(n.text(), "TakurouAAz");
        assert_eq!(n.cursor, 10);
        // No cell at column 9 of row 1.
        assert_eq!(n.hit(450 + 60 * 9 + 5, 51 + 58 + 5), None);
        n.click(DEFAULT);
        assert_eq!(n.text(), "Takurou");
    }

    #[test]
    fn confirm() {
        let mut n = Name::new(Language::English, areas());
        assert_eq!(n.click(CONFIRM), Click::Name);
        n.name = "男".chars().collect();
        assert_eq!(n.click(CONFIRM), Click::None);
        assert!(n.notice);
        n.cancel();
        assert!(!n.notice);
        // The secondary click leaves the name as it is.
        let before = n.text();
        assert!(!n.secondary());
        assert_eq!(n.text(), before);
        n.notice = true;
        assert!(n.secondary());
        assert!(!n.notice);
        n.name.clear();
        n.cursor = 0;
        assert!(!n.enabled(CONFIRM));
        assert_eq!(n.click(CONFIRM), Click::None);
    }

    #[test]
    fn chinese() {
        let mut n = Name::new(Language::SimplifiedChinese, areas());
        n.cursor = 2;
        n.type_text("ab c");
        assert_eq!(n.text(), "拓郎ａｂ\u{3000}");
        assert_eq!(n.cursor, 5);
    }
}
