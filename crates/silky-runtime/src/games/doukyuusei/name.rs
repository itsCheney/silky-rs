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

use super::room::Click;
use super::window::Font;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

/// Where the name is kept in the raw area (NUL-terminated UTF-8) and
/// how many bytes the field has.
pub const NAME_AT: usize = 18500;
pub const NAME_BYTES: usize = 100;

/// Hot spot ids.
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
}

impl Name {
    pub fn new(language: Language) -> Self {
        let mut n = Self {
            chinese: language != Language::English,
            name: Vec::new(),
            cursor: 0,
            blink: true,
            since: 0,
            notice: false,
            sound: false,
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
        let buttons = [
            (LEFT, ARROW_L.1, ARROW_TOP.2, ARROW_W, 58),
            (RIGHT, ARROW_R.1, ARROW_TOP.2, ARROW_W, 58),
            (BACKSPACE, BACK_PART.1, ARROW_TOP.2, BACK_W, 58),
            (DEFAULT, DEFAULT_POS.0, DEFAULT_POS.1, 264, 86),
            (CONFIRM, CONFIRM_POS.0, CONFIRM_POS.1, 234, 154),
        ];
        buttons
            .into_iter()
            .find(|&(_, l, t, w, h)| inside(l, t, w, h))
            .map(|b| b.0)
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
        match id {
            i if i >= CELL => {
                if let Some(c) = self.cell_char(i) {
                    self.sound = self.insert(c);
                }
            }
            LEFT => {
                self.cursor = self.cursor.saturating_sub(1);
                self.sound = true;
            }
            RIGHT => {
                self.cursor = (self.cursor + 1).min(self.max().min(self.name.len()));
                self.sound = true;
            }
            BACKSPACE => {
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

    /// The secondary click: Backspace without the sound; the caret goes
    /// off. Also closes the notice.
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

    #[test]
    fn edit() {
        let mut n = Name::new(Language::English);
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
        let mut n = Name::new(Language::English);
        assert_eq!(n.click(CONFIRM), Click::Name);
        n.name = "男".chars().collect();
        assert_eq!(n.click(CONFIRM), Click::None);
        assert!(n.notice);
        n.cancel();
        assert!(!n.notice);
        n.name.clear();
        n.cursor = 0;
        assert!(!n.enabled(CONFIRM));
        assert_eq!(n.click(CONFIRM), Click::None);
    }

    #[test]
    fn chinese() {
        let mut n = Name::new(Language::SimplifiedChinese);
        n.cursor = 2;
        n.type_text("ab c");
        assert_eq!(n.text(), "拓郎ａｂ\u{3000}");
        assert_eq!(n.cursor, 5);
    }
}
