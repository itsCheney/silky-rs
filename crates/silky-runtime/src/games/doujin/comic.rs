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
//! Two round buttons (`comic.akb`, at the areas 2 and 3 of
//! `DOUJINSHI.AREA`) turn the page: the left one forward (the wait returns
//! 2), the right one back (3). A click on the page itself ends the wait
//! (0). Sub 6 names a small rectangle of the lettered page that appears
//! with the next wait (a sound word written in a bubble) instead of text; a
//! click first lets it stand, the next goes on.
//!
//! Not carried out: the characters appear at once, not one by one at the
//! reading speed; the buttons have no hover or pressed look and make no
//! sound; the fade-in of the hot spot is instant; right click, the
//! auto mode and skipping do not return the codes 1 for leaving.

use std::sync::Arc;

use silky_vm::{Flow, Vm};

use crate::assets::Assets;
use crate::games::Call;
use crate::host::{Pause, Runtime};
use crate::render::Canvas;
use crate::text::TextRenderer;

/// Message number of the first wait in `DOJIN.MES`.
const FIRST_MESSAGE: i32 = 6167;
/// Size of a character cell.
const CELL: i32 = 10;
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

/// What has been put on the page.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shown {
    /// Characters at cells (left, top).
    Text(Vec<(i32, i32, char)>),
    /// A rectangle of the lettered page.
    Spot(Rect),
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
    /// A spot just put up that the next click only lets stand.
    spot_up: bool,
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

    /// Puts the words of message `n` in a bubble from the anchor.
    fn put_text(&mut self, n: i32) {
        let text = n
            .checked_sub(FIRST_MESSAGE)
            .and_then(|k| self.texts.get(k as usize))
            .map(String::as_str)
            .unwrap_or("");
        let (x0, y0) = self.anchor;
        let mut cells = Vec::new();
        for (col, line) in text.split('\\').enumerate() {
            for (row, c) in line.chars().enumerate() {
                cells.push((x0 - col as i32 * CELL, y0 + row as i32 * CELL, c));
            }
        }
        if !cells.is_empty() {
            self.shown.push(Shown::Text(cells));
        }
    }

    /// Draws what is on the page and the buttons over the picture.
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, text: Option<&TextRenderer>) {
        let lettered = self
            .lettered
            .as_ref()
            .and_then(|name| assets.image(name).ok());
        for item in &self.shown {
            match item {
                Shown::Spot(r) => {
                    if let Some(img) = &lettered {
                        canvas.draw_part(
                            img,
                            (r.0.max(0) as u32, r.1.max(0) as u32, r.2 as u32, r.3 as u32),
                            r.0,
                            r.1,
                            false,
                            255,
                        );
                    }
                }
                Shown::Text(cells) => {
                    for &(x, y, c) in cells {
                        if let Some(img) = &lettered {
                            canvas.draw_part(
                                img,
                                ((x - 1).max(0) as u32, (y - 1).max(0) as u32, 12, 12),
                                x - 1,
                                y - 1,
                                false,
                                255,
                            );
                        } else if let Some(tr) = text {
                            tr.draw_plain(
                                canvas,
                                &c.to_string(),
                                x as f32,
                                (y + CELL - 1) as f32,
                                CELL as f32,
                                [0, 0, 0, 255],
                            );
                        }
                    }
                }
            }
        }
        if self.arrows {
            if let Ok(sheet) = assets.image("comic.akb") {
                for (id, (x, y), (sx, sy)) in ARROWS {
                    let (x, y) = self
                        .areas
                        .iter()
                        .find(|(a, _)| *a == id)
                        .map_or((x, y), |(_, r)| (r.0, r.1));
                    canvas.draw_part(&sheet, (sx, sy, ARROW, ARROW), x, y, false, 255);
                }
            }
        }
    }
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
    /// Used up (it let a spot stand).
    Used,
    /// Changed the wait's result; the script goes on.
    Answered,
}

/// A click at (x, y) while the screen waits.
pub fn click(rt: &mut Runtime, vm: &mut Vm, x: i32, y: i32) -> Click {
    let Some(comic) = &mut rt.scene.comic else {
        return Click::Pass;
    };
    if std::mem::take(&mut comic.spot_up) {
        return Click::Used;
    }
    match comic.button_at(x, y) {
        Some(code) => {
            // The wait pushed its 0 already.
            let _ = vm.pop();
            vm.push(code);
            Click::Answered
        }
        None => Click::Pass,
    }
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
                if let Some(comic) = &mut self.scene.comic {
                    comic.lettered = background
                        .as_deref()
                        .map(|b| b.trim_end_matches(".akb").trim_end_matches(".AKB"))
                        .map(|stem| format!("{stem}a.akb"))
                        .filter(|name| self.assets.image(name).is_ok());
                    match comic.next_spot.take() {
                        Some(r) => {
                            comic.shown.push(Shown::Spot(r));
                            comic.spot_up = true;
                        }
                        None => comic.put_text(n),
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
                    comic.spot_up = false;
                }
                self.done()
            }
            // Creates the screen, with the two round buttons up.
            3 => {
                if self.scene.comic.is_none() {
                    self.scene.comic = Some(Comic::new(&self.assets));
                }
                if let Some(comic) = &mut self.scene.comic {
                    comic.arrows = true;
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
                if let Some(comic) = &mut self.scene.comic {
                    comic.anchor = at;
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
                if let Some(comic) = &mut self.scene.comic {
                    comic.arrows = true;
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
            ..Comic::default()
        };
        c.put_text(FIRST_MESSAGE);
        assert_eq!(
            c.shown,
            [Shown::Text(vec![
                (100, 20, 'a'),
                (100, 30, 'b'),
                (90, 20, 'c')
            ])]
        );
    }
}
