//! The doujinshi screen of 先輩が私の妄想にドージンする (`Game.40`, script
//! `DOJIN.MES`).
//!
//! A page is a full-screen picture (`do00.akb` to `do06.akb`) shown as the
//! background, with empty speech bubbles. The screen puts the bubbles'
//! words in one bubble at a time: the script sets where the bubble's text
//! starts (variables 50 and 51: left edge of the first column, top), plays
//! the voice line and waits (`Game.40` sub 0). The words are set vertically
//! in cells of 10 by 10 pixels, the first column at the right, further
//! columns to its left; a backslash in the text starts a column. Where the
//! page has a version with the lettering done (`do00a.akb`, `do04a.akb` to
//! `do06a.akb`) the cells are taken from it, else the characters are drawn
//! with the font. The texts are the program's own: they are read from the
//! game's program file, in the order of the messages of the script (one per
//! wait, from message number 6167).
//!
//! The words appear one by one: a bubble's characters come at the
//! interval the text speed setting gives (150 ms at the slowest, none at
//! the fastest or with the one-by-one setting off), a little longer after
//! a stop mark when that setting is on. A click, or leaving by the keys,
//! puts the rest up at once; skipping does too.
//!
//! Two round buttons (`comic.akb`, at the areas 2 and 3 of
//! `DOUJINSHI.AREA`) turn the page: the left one forward (the wait returns
//! 2), the right one back (3). The sheet has four looks for each: normal,
//! pointer over, pressed and unavailable; the back button is unavailable
//! on the first page and does nothing there. A click on a button plays the
//! confirm sound. A click on the page itself ends the wait (0). Sub 6
//! names a small rectangle of the lettered page that appears with the next
//! wait (a sound word written in a bubble) instead of text: it is wiped in
//! from the top over 24 frames, and a click during that finishes it.
//! The wait can also come back with 1, the game's own leaving of a scene
//! replay; the script then returns at once.

use std::sync::Arc;

use silky_vm::{Flow, Vm};

use crate::assets::Assets;
use crate::games::Call;
use crate::host::{Pause, Runtime};
use crate::render::Canvas;
use crate::text::TextRenderer;

/// Message number of the first wait in `DOJIN.MES`.
const FIRST_MESSAGE: i32 = 6167;
/// Size of a character cell when the script gives none: width, height.
const CELL: (i32, i32) = (11, 10);
/// Variables the script sets the cell size in, and the top of the columns.
const VAR_CELL_W: usize = 30;
const VAR_CELL_H: usize = 31;
const VAR_TOP: usize = 47;
/// Variable that is 0 on the first page.
const VAR_PAGE: usize = 15;
/// Most the columns can run left of the bubble's start; the text goes no
/// further.
const COLUMNS_SPAN: i32 = 400;
/// Longest wait between two characters (at the slowest text speed), and
/// the extra wait after a stop mark.
const CHAR_MS: f32 = 150.0;
const STOP_MS: u64 = 300;
/// Characters that make the longer wait.
const STOPS: [char; 4] = ['。', '、', '・', '\u{3000}'];
/// A frame of the game's animations.
const FRAME_MS: u64 = 16;
/// Frames the hot spot takes to appear.
const SPOT_FRAMES: u64 = 24;
/// Sound of a click on a round button.
const CLICK_SOUND: &str = "ses_001.wav";
/// Rows of `comic.akb`.
const ROW_NORMAL: u32 = 0;
const ROW_OVER: u32 = 1;
const ROW_UNAVAILABLE: u32 = 3;
/// Variables the script sets the bubble's text start in.
const VAR_X: usize = 50;
const VAR_Y: usize = 51;
/// Round buttons: places (areas 2 and 3), size and cells on `comic.akb`.
const ARROWS: [(i32, (i32, i32), (u32, u32)); 2] =
    [(2, (18, 312), (0, 0)), (3, (1166, 312), (96, 0))];
const ARROW: u32 = 96;

/// Result of a wait, as the script reads it.
pub const NEXT_PAGE: i32 = 2;
pub const PREVIOUS_PAGE: i32 = 3;

/// A rectangle: left, top, width, height.
type Rect = (i32, i32, i32, i32);

/// One character of a bubble.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Glyph {
    x: i32,
    y: i32,
    c: char,
    /// Width of its cell.
    w: i32,
    /// Milliseconds after the wait began at which it appears.
    at: u64,
}

/// What has been put on the page.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shown {
    /// Characters of a bubble.
    Text(Vec<Glyph>),
    /// A rectangle of the lettered page; true while it is being wiped in.
    Spot(Rect, bool),
}

/// The screen object of `Game.40`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Comic {
    /// The bubbles' texts, by message order.
    texts: Arc<Vec<String>>,
    /// Where the round buttons are, by area id.
    areas: Vec<(i32, Rect)>,
    /// The round buttons are drawn.
    arrows: bool,
    /// Where the next bubble's text starts.
    anchor: (i32, i32),
    /// Rectangle for the next wait (sub 6).
    next_spot: Option<Rect>,
    /// Size of a character cell.
    cell: (i32, i32),
    /// Top of the columns of the bubble.
    top: i32,
    /// Milliseconds since the wait began.
    elapsed: u64,
    /// The back button is unavailable (the first page).
    first_page: bool,
    /// The round button under the pointer.
    over: Option<i32>,
    /// The lettered version of the page.
    lettered: Option<String>,
    shown: Vec<Shown>,
}

impl Comic {
    fn new(assets: &Assets) -> Self {
        let areas = assets
            .data("doujinshi.area")
            .ok()
            .and_then(|b| silky_format::area::parse(&b).ok())
            .map(|v| {
                v.iter()
                    .map(|a| (a.id, (a.left, a.top, a.right - a.left, a.bottom - a.top)))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            texts: Arc::new(
                program_file(&assets.game().root)
                    .map(|b| bubble_texts(&b))
                    .unwrap_or_default(),
            ),
            areas,
            cell: CELL,
            ..Self::default()
        }
    }

    /// Area under a point: 2 or 3 for the buttons.
    fn button_at(&self, x: i32, y: i32) -> Option<i32> {
        if !self.arrows {
            return None;
        }
        self.areas
            .iter()
            .find(|(id, (l, t, w, h))| {
                (*id == NEXT_PAGE || *id == PREVIOUS_PAGE)
                    && x >= *l
                    && x < l + w
                    && y >= *t
                    && y < t + h
            })
            .map(|(id, _)| *id)
    }

    /// Puts the words of message `n` in a bubble from the anchor, with the
    /// time each character comes up.
    fn put_text(&mut self, n: i32, timing: Timing) {
        let text = n
            .checked_sub(FIRST_MESSAGE)
            .and_then(|k| self.texts.get(k as usize))
            .map(String::as_str)
            .unwrap_or("");
        let (cw, ch) = self.cell;
        let (mut x, mut y) = self.anchor;
        let mut at = 0;
        let mut glyphs = Vec::new();
        for c in text.chars() {
            let mut extra = 0;
            if c == '\\' {
                // A new column to the left.
                x -= cw;
                y = self.top;
            } else {
                let wide = !c.is_ascii();
                if wide && timing.stop_pause && STOPS.contains(&c) {
                    extra = STOP_MS;
                }
                glyphs.push(Glyph {
                    x,
                    y,
                    c,
                    w: if wide { cw } else { cw / 2 },
                    at,
                });
                y += ch;
            }
            at += timing.interval + extra;
            if x - cw < self.anchor.0 - COLUMNS_SPAN {
                break;
            }
        }
        if !glyphs.is_empty() {
            self.shown.push(Shown::Text(glyphs));
        }
    }

    /// Starts a wait: the clock of the words and of the hot spot starts,
    /// the words before it are all there.
    fn begin_wait(&mut self) {
        self.elapsed = 0;
        for item in &mut self.shown {
            match item {
                Shown::Text(glyphs) => glyphs.iter_mut().for_each(|g| g.at = 0),
                Shown::Spot(_, fading) => *fading = false,
            }
        }
    }

    /// Time the words of the newest bubble take to come up in all.
    fn typing_ms(&self) -> u64 {
        match self.shown.last() {
            Some(Shown::Text(glyphs)) => glyphs.last().map_or(0, |g| g.at),
            _ => 0,
        }
    }

    /// Whether the words are still coming up or the hot spot being wiped
    /// in.
    fn busy(&self) -> bool {
        match self.shown.last() {
            Some(Shown::Text(glyphs)) => glyphs.last().is_some_and(|g| g.at > self.elapsed),
            Some(Shown::Spot(_, true)) => self.elapsed < SPOT_FRAMES * FRAME_MS,
            _ => false,
        }
    }

    /// Puts the rest up at once. True if there was a rest.
    pub fn finish(&mut self) -> bool {
        if !self.busy() {
            return false;
        }
        self.elapsed = u64::MAX / 2;
        true
    }

    /// Goes on for `ms` milliseconds. True if the picture changes.
    pub fn advance(&mut self, ms: u64) -> bool {
        let busy = self.busy();
        self.elapsed = self.elapsed.saturating_add(ms);
        busy
    }

    /// Look of a round button.
    fn row(&self, id: i32) -> u32 {
        if id == PREVIOUS_PAGE && self.first_page {
            ROW_UNAVAILABLE
        } else if self.over == Some(id) {
            ROW_OVER
        } else {
            ROW_NORMAL
        }
    }

    /// Draws what is on the page and the buttons over the picture.
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, text: Option<&TextRenderer>) {
        let lettered = self
            .lettered
            .as_ref()
            .and_then(|name| assets.image(name).ok());
        let (_, ch) = self.cell;
        for item in &self.shown {
            match item {
                Shown::Spot(r, fading) => {
                    if let Some(img) = &lettered {
                        // Wiped in from the top.
                        let h = if *fading {
                            let frame = (self.elapsed / FRAME_MS).min(SPOT_FRAMES);
                            (r.3 as u64 * frame / SPOT_FRAMES) as i32
                        } else {
                            r.3
                        };
                        if h > 0 {
                            canvas.draw_part(
                                img,
                                (r.0.max(0) as u32, r.1.max(0) as u32, r.2 as u32, h as u32),
                                r.0,
                                r.1,
                                false,
                                255,
                            );
                        }
                    }
                }
                Shown::Text(glyphs) => {
                    for g in glyphs.iter().filter(|g| g.at <= self.elapsed) {
                        if let Some(img) = &lettered {
                            canvas.draw_part(
                                img,
                                (g.x.max(0) as u32, g.y.max(0) as u32, g.w as u32, ch as u32),
                                g.x,
                                g.y,
                                false,
                                255,
                            );
                        } else if let Some(tr) = text {
                            tr.draw_plain(
                                canvas,
                                &g.c.to_string(),
                                g.x as f32,
                                (g.y + ch - 1) as f32,
                                ch as f32,
                                [0, 0, 0, 255],
                            );
                        }
                    }
                }
            }
        }
        if self.arrows {
            if let Ok(sheet) = assets.image("comic.akb") {
                for (id, (x, y), (sx, _)) in ARROWS {
                    let (x, y) = self
                        .areas
                        .iter()
                        .find(|(a, _)| *a == id)
                        .map_or((x, y), |(_, r)| (r.0, r.1));
                    let row = self.row(id) * ARROW;
                    canvas.draw_part(&sheet, (sx, row, ARROW, ARROW), x, y, false, 255);
                }
            }
        }
    }
}

/// How the characters of a bubble are spaced in time.
#[derive(Debug, Clone, Copy)]
struct Timing {
    interval: u64,
    stop_pause: bool,
}

/// The game's program file in its folder.
fn program_file(root: &std::path::Path) -> Option<Vec<u8>> {
    let path = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("doujin.exe"))
        })?;
    std::fs::read(path).ok()
}

/// The bubble texts in the program: a run of strings that starts at the
/// voice name `db0001.ogg` and ends at `comic.akb`. A voice name starts an
/// entry, the text after it fills it; a text with no name before it is an
/// entry of its own, and a name followed by another name has no text.
fn bubble_texts(exe: &[u8]) -> Vec<String> {
    let Some(from) = find(exe, b"db0001.ogg\0") else {
        return Vec::new();
    };
    let to = find(&exe[from..], b"comic.akb\0").map_or(exe.len(), |n| from + n);
    let mut entries: Vec<(bool, Option<String>)> = Vec::new();
    for lit in exe[from..to].split(|&b| b == 0).filter(|s| !s.is_empty()) {
        let s = String::from_utf8_lossy(lit).into_owned();
        if s.ends_with(".ogg") && s.is_ascii() {
            entries.push((true, None));
        } else {
            match entries.last_mut() {
                Some((true, text @ None)) => *text = Some(s),
                _ => entries.push((false, Some(s))),
            }
        }
    }
    entries
        .into_iter()
        .map(|(_, t)| t.unwrap_or_default())
        .collect()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// What the session does with a click while the screen waits.
#[derive(Debug, PartialEq, Eq)]
pub enum Click {
    /// Not the screen's: the click goes on as usual.
    Pass,
    /// Used up (it put the rest of the words up, or did nothing).
    Used,
    /// Changed the wait's result; the script goes on.
    Answered,
}

/// A click at (x, y) while the screen waits.
pub fn click(rt: &mut Runtime, vm: &mut Vm, x: i32, y: i32) -> Click {
    let Some(comic) = &mut rt.scene.comic else {
        return Click::Pass;
    };
    match comic.button_at(x, y) {
        Some(PREVIOUS_PAGE) if comic.first_page => Click::Used,
        Some(code) => {
            // The wait pushed its 0 already.
            let _ = vm.pop();
            vm.push(code);
            let _ = rt.play_sound(CLICK_SOUND);
            Click::Answered
        }
        // The first click while words come up or the hot spot appears
        // finishes that.
        None if comic.finish() => Click::Used,
        None => Click::Pass,
    }
}

/// Puts the rest of the words up (keys that go on do this first). True if
/// there was a rest.
pub fn finish(rt: &mut Runtime) -> bool {
    rt.scene.comic.as_mut().is_some_and(Comic::finish)
}

/// Time the words of the bubble take to come up, `None` without the
/// screen.
pub fn typing_ms(rt: &Runtime) -> Option<u64> {
    rt.scene.comic.as_ref().map(Comic::typing_ms)
}

/// The clock goes on by `ms`; skipping puts everything up at once. True if
/// the picture changes.
pub fn tick(rt: &mut Runtime, ms: u64, skipping: bool) -> bool {
    let Some(comic) = &mut rt.scene.comic else {
        return false;
    };
    if skipping {
        return comic.finish();
    }
    comic.advance(ms)
}

/// The pointer is at (x, y). True if a button changes its look.
pub fn hover(rt: &mut Runtime, x: i32, y: i32) -> bool {
    let Some(comic) = &mut rt.scene.comic else {
        return false;
    };
    let over = comic.button_at(x, y);
    std::mem::replace(&mut comic.over, over) != over
}

impl Runtime {
    /// `Game.40`, the doujinshi screen.
    pub(crate) fn comic_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        match c.sub {
            // (message, mode): the screen waits for the player; the code
            // is 0 to go on, 2 and 3 from the buttons. Sub 1 is the same.
            0 | 1 => {
                let n = c.int(0);
                let background = self.scene.background.clone();
                let st = &self.system.settings;
                let typed = st.typewriter && st.text_speed < 1.0;
                let timing = Timing {
                    interval: if typed {
                        ((1.0 - st.text_speed.clamp(0.0, 1.0)) * CHAR_MS) as u64
                    } else {
                        0
                    },
                    stop_pause: st.punctuation_pause,
                };
                let first_page = vm.var_g(VAR_PAGE).int() == 0;
                if let Some(comic) = &mut self.scene.comic {
                    comic.lettered = background
                        .as_deref()
                        .map(|b| b.trim_end_matches(".akb").trim_end_matches(".AKB"))
                        .map(|stem| format!("{stem}a.akb"))
                        .filter(|name| self.assets.image(name).is_ok());
                    comic.first_page = first_page;
                    comic.begin_wait();
                    match comic.next_spot.take() {
                        Some(r) => comic.shown.push(Shown::Spot(r, true)),
                        None => comic.put_text(n, timing),
                    }
                }
                let name = self
                    .message_script
                    .is_empty()
                    .then(|| vm.script_name().unwrap_or("").to_ascii_lowercase());
                if let Some(name) = name {
                    self.message_script = name;
                }
                let key = format!("{}:{n}", self.message_script);
                self.read_before = !self.system.read.insert(key);
                self.system_changed |= !self.read_before;
                self.pause = Some(Pause::Text {
                    index: n.max(0) as usize,
                });
                vm.push(0);
                self.handled += 1;
                Some(Ok(Flow::Yield))
            }
            // Clears both draw layers: the text and the buttons.
            2 => {
                if let Some(comic) = &mut self.scene.comic {
                    comic.shown.clear();
                    comic.arrows = false;
                    comic.next_spot = None;
                }
                self.done()
            }
            // Creates the screen, with the two round buttons up.
            3 => {
                if self.scene.comic.is_none() {
                    self.scene.comic = Some(Comic::new(&self.assets));
                }
                let first_page = vm.var_g(VAR_PAGE).int() == 0;
                if let Some(comic) = &mut self.scene.comic {
                    comic.arrows = true;
                    comic.first_page = first_page;
                }
                self.done()
            }
            // Ends it.
            4 => {
                self.scene.comic = None;
                self.done()
            }
            // Next bubble: where its text starts.
            5 => {
                let at = (vm.var_g(VAR_X).int(), vm.var_g(VAR_Y).int());
                let top = vm.var_g(VAR_TOP).int();
                let cell = (vm.var_g(VAR_CELL_W).int(), vm.var_g(VAR_CELL_H).int());
                if let Some(comic) = &mut self.scene.comic {
                    comic.anchor = at;
                    comic.top = top;
                    if cell.0 > 0 && cell.1 > 0 {
                        comic.cell = cell;
                    }
                }
                self.done()
            }
            // (h, w, y, x): the rectangle that shows with the next wait.
            6 => {
                let r = (c.int(3), c.int(2), c.int(1), c.int(0));
                if let Some(comic) = &mut self.scene.comic {
                    comic.next_spot = Some(r);
                }
                self.done()
            }
            // Puts the two round buttons up again (after the layers were
            // cleared); nothing happens before the screen is created.
            7 => {
                let first_page = vm.var_g(VAR_PAGE).int() == 0;
                if let Some(comic) = &mut self.scene.comic {
                    comic.arrows = true;
                    comic.first_page = first_page;
                }
                self.done()
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_follow_the_messages() {
        let mut exe = b"xx db0001.ogg\0\0".to_vec();
        exe.extend("あ\\い".as_bytes());
        exe.extend(b"\0\0dc0001.ogg\0\0");
        exe.extend("う".as_bytes());
        exe.extend(b"\0\0");
        exe.extend("え".as_bytes());
        exe.extend(b"\0\0db0002.ogg\0\0db0003.ogg\0\0");
        exe.extend("お".as_bytes());
        exe.extend(b"\0comic.akb\0");
        assert_eq!(bubble_texts(&exe), ["あ\\い", "う", "え", "", "お"]);
    }

    #[test]
    fn columns_run_right_to_left() {
        let mut c = Comic {
            texts: Arc::new(vec!["ab\\c".to_string()]),
            anchor: (100, 20),
            top: 20,
            cell: (11, 10),
            ..Comic::default()
        };
        let timing = Timing {
            interval: 40,
            stop_pause: false,
        };
        c.put_text(FIRST_MESSAGE, timing);
        let glyph = |x, y, c, at| Glyph { x, y, c, w: 5, at };
        let Shown::Text(g) = &c.shown[0] else {
            panic!("no text");
        };
        // Latin letters take half a cell; the break takes a turn too.
        assert_eq!(
            g,
            &[
                glyph(100, 20, 'a', 0),
                glyph(100, 30, 'b', 40),
                glyph(89, 20, 'c', 120)
            ]
        );
    }

    #[test]
    fn stop_marks_wait_longer() {
        let mut c = Comic {
            texts: Arc::new(vec!["あ、い".to_string()]),
            anchor: (100, 20),
            top: 20,
            cell: (11, 10),
            ..Comic::default()
        };
        let timing = Timing {
            interval: 50,
            stop_pause: true,
        };
        c.put_text(FIRST_MESSAGE, timing);
        let Shown::Text(g) = &c.shown[0] else {
            panic!("no text");
        };
        let at: Vec<u64> = g.iter().map(|g| g.at).collect();
        assert_eq!(at, [0, 50, 400]);
        assert_eq!(c.typing_ms(), 400);
        assert!(c.advance(100));
        assert!(c.finish());
        assert!(!c.busy());
    }

    #[test]
    fn spot_wipes_in_over_24_frames() {
        let mut c = Comic::default();
        c.shown.push(Shown::Spot((0, 0, 10, 24), true));
        assert!(c.busy());
        assert!(c.advance(SPOT_FRAMES * FRAME_MS));
        assert!(!c.busy());
    }
}
