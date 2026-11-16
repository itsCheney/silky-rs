//! Message window of 同級生 Remake: the band and frame of `message_p.akb`,
//! the speaker's name plate, and the text in the game's own bitmap fonts.
//!
//! Measured under Wine at 1920 x 1080 and read from the executable (see
//! `docs/games/doukyuusei.md`).

use std::collections::HashMap;
use std::sync::Arc;

use silky_format::akb::Image;

use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// Top of the window: the band and the frame are each 350 rows of the sheet
/// (`mes_o.area` gives the window as 1920 x 350).
pub const TOP: i32 = 730;
/// Translucent black band (sheet rows 0..350) and its frame lines (rows
/// 350..700), both drawn at [`TOP`].
const BAND: (u32, u32, u32, u32) = (0, 0, 1920, 350);
const FRAME: (u32, u32, u32, u32) = (0, 350, 1920, 350);
/// Opacity of the band under Wine with the default settings: the picture
/// behind it keeps 37% of its brightness.
pub const BAND_OPACITY: u8 = 161;
/// Name plate (a star and a gold rule, sheet rows 1350..1400) and where its
/// text starts.
const NAME_PART: (u32, u32, u32, u32) = (0, 1350, 1500, 50);
const NAME_PART_POS: (i32, i32) = (231, 799);
const NAME_TEXT: (i32, i32) = (294, 800);
/// Top left of the first glyph cell of the message text.
pub const TEXT_POS: (i32, i32) = (242, 870);
/// A half-width letter takes one 20-pixel cell, a full-width one two;
/// lines are 48 pixels apart.
pub const CELL: i32 = 20;
pub const LINE: i32 = 48;
/// A word that would reach this many cells from the line start goes to the
/// next line (between 73 and 74 cells under Wine; the executable compares
/// pixel positions).
pub const LINE_CELLS: i32 = 74;
/// Turns italics on and off in shown text (the tables write `\i`).
pub const ITALIC: char = '\u{E000}';

/// One placed glyph: its character, the top left of its cell relative to
/// the first cell, and whether it comes from the italic sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub ch: char,
    pub x: i32,
    pub y: i32,
    pub italic: bool,
}

/// Whether a character takes a half cell, as the executable decides.
fn half(c: char) -> bool {
    (c as u32) < 0x100 || c == '\u{2014}'
}

fn cells(c: char) -> i32 {
    if half(c) {
        1
    } else {
        2
    }
}

/// Places the message text as the executable does: words of half-width
/// letters (with the space after them) and single full-width characters
/// move to the next line whole when they would reach [`LINE_CELLS`] (a
/// word's trailing space does not count); `\n` breaks the line. Each
/// placed glyph keeps its index in `text`.
pub fn layout(text: &str) -> Vec<(usize, Placed)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let (mut x, mut y) = (0, 0);
    let mut italic = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            x = 0;
            y += LINE;
            i += 1;
            continue;
        }
        if c == ITALIC {
            italic = !italic;
            i += 1;
            continue;
        }
        // The word: up to and including the next space, or one wide
        // character; italic switches inside it are kept.
        let mut end = i;
        if half(c) {
            while end < chars.len() && chars[end] != '\n' {
                let d = chars[end];
                if d != ITALIC && !half(d) {
                    break;
                }
                end += 1;
                if d == ' ' {
                    break;
                }
            }
        } else {
            end = i + 1;
        }
        let word = &chars[i..end];
        let mut width: i32 = word
            .iter()
            .filter(|&&d| d != ITALIC)
            .map(|&d| cells(d))
            .sum();
        if word.last() == Some(&' ') {
            width -= 1;
        }
        if x > 0 && (x + width) * CELL >= LINE_CELLS * CELL {
            x = 0;
            y += LINE;
        }
        for (k, &d) in word.iter().enumerate() {
            if d == ITALIC {
                italic = !italic;
                continue;
            }
            out.push((
                i + k,
                Placed {
                    ch: d,
                    x: x * CELL,
                    y,
                    italic,
                },
            ));
            x += cells(d);
        }
        if x >= LINE_CELLS {
            x = 0;
            y += LINE;
        }
        i = end;
    }
    out
}

/// One glyph sheet: cells of `w` x 40 pixels, 20 to a row, in the order of
/// its `.map` list; and the same sheet in black for the outline.
struct Sheet {
    white: Arc<Image>,
    black: Arc<Image>,
    w: u32,
}

impl Sheet {
    fn new(img: Arc<Image>, w: u32) -> Self {
        let mut rgba = img.rgba.clone();
        for p in rgba.chunks_exact_mut(4) {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
        }
        let black = Arc::new(Image {
            width: img.width,
            height: img.height,
            rgba,
        });
        Self {
            white: img,
            black,
            w,
        }
    }
}

/// The game's bitmap fonts: `font_han.akb` (half-width, 20 x 40),
/// `font_han_i.akb` (the same letters in italics) and `font_zen.akb`
/// (full-width, 40 x 40), indexed by `font_han.map` and `font_zen.map`
/// (u16 count, then that many UTF-16 characters).
pub struct Font {
    han: Sheet,
    han_i: Sheet,
    zen: Sheet,
    han_index: HashMap<char, u32>,
    zen_index: HashMap<char, u32>,
}

const GLYPH_H: u32 = 40;
const PER_ROW: u32 = 20;

fn index(data: &[u8]) -> HashMap<char, u32> {
    let n = data
        .get(..2)
        .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let units: Vec<u16> = data
        .get(2..2 + 2 * n)
        .unwrap_or(&[])
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    char::decode_utf16(units)
        .enumerate()
        .filter_map(|(i, c)| c.ok().map(|c| (c, i as u32)))
        .collect()
}

impl Font {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        Ok(Self {
            han: Sheet::new(assets.image("font_han.akb")?, 20),
            han_i: Sheet::new(assets.image("font_han_i.akb")?, 20),
            zen: Sheet::new(assets.image("font_zen.akb")?, 40),
            han_index: index(&assets.data("font_han.map")?),
            zen_index: index(&assets.data("font_zen.map")?),
        })
    }

    /// Draws a character with its cell's top left at (x, y): a 1-pixel
    /// black outline (the glyph drawn at its eight neighbours), then the
    /// glyph in white, as the executable does. Characters not in the font
    /// leave their cell empty.
    pub fn draw_char(
        &self,
        canvas: &mut Canvas,
        c: char,
        x: i32,
        y: i32,
        italic: bool,
        opacity: u8,
    ) {
        let (sheet, at) = if half(c) {
            let sheet = if italic { &self.han_i } else { &self.han };
            (sheet, self.han_index.get(&c))
        } else {
            (&self.zen, self.zen_index.get(&c))
        };
        let Some(&i) = at else {
            return;
        };
        let part = (
            (i % PER_ROW) * sheet.w,
            (i / PER_ROW) * GLYPH_H,
            sheet.w,
            GLYPH_H,
        );
        for (dx, dy) in [
            (0, -1),
            (0, 1),
            (-1, 0),
            (1, 0),
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ] {
            canvas.draw_part(&sheet.black, part, x + dx, y + dy, false, opacity);
        }
        canvas.draw_part(&sheet.white, part, x, y, false, opacity);
    }

    /// Draws the message text, only its first `limit` characters if given
    /// (counted in `text`, italic switches and line breaks included).
    pub fn draw_text(
        &self,
        canvas: &mut Canvas,
        text: &str,
        limit: Option<usize>,
        shift: (i32, i32),
    ) {
        let (x0, y0) = (TEXT_POS.0 + shift.0, TEXT_POS.1 + shift.1);
        for (i, p) in layout(text) {
            if limit.is_some_and(|n| i >= n) {
                break;
            }
            self.draw_char(canvas, p.ch, x0 + p.x, y0 + p.y, p.italic, 255);
        }
    }

    /// Draws a line of text from (x, y) without wrapping.
    pub fn draw_line(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32) {
        let mut cx = x;
        for c in text.chars() {
            self.draw_char(canvas, c, cx, y, false, 255);
            cx += cells(c) * CELL;
        }
    }
}

/// Draws the window band, frame and name plate. The band keeps the
/// original's default opacity; the window settings do not apply yet.
pub fn draw_window(
    canvas: &mut Canvas,
    assets: &mut Assets,
    font: &Font,
    speaker: Option<&str>,
) -> Result<()> {
    let parts = assets.image("message_p.akb")?;
    canvas.draw_part(&parts, BAND, 0, TOP, false, BAND_OPACITY);
    canvas.draw_part(&parts, FRAME, 0, TOP, false, 255);
    if let Some(name) = speaker {
        let (x, y) = NAME_PART_POS;
        canvas.draw_part(&parts, NAME_PART, x, y, false, 255);
        font.draw_line(canvas, name, NAME_TEXT.0, NAME_TEXT.1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut last_y = -1;
        for (_, p) in layout(text) {
            if p.y != last_y {
                out.push(String::new());
                last_y = p.y;
            }
            out.last_mut().unwrap().push(p.ch);
        }
        out.into_iter().map(|l| l.trim_end().to_string()).collect()
    }

    #[test]
    fn wraps_like_the_original() {
        // Both seen under Wine.
        assert_eq!(
            lines("Listen, I don't know who you are, but are you trying to pick a fight with me?!"),
            [
                "Listen, I don't know who you are, but are you trying to pick a fight with",
                "me?!"
            ]
        );
        assert_eq!(
            lines(
                "Sweet, I'll be waiting for you at Sakimake Station at 11 a.m. I can't \
                 believe I lucked out! I mean, you're actually free starting today!"
            ),
            [
                "Sweet, I'll be waiting for you at Sakimake Station at 11 a.m. I can't",
                "believe I lucked out! I mean, you're actually free starting today!"
            ]
        );
    }

    #[test]
    fn italics_and_breaks() {
        let t = format!("{ITALIC}*Slam!*{ITALIC}\nok");
        let placed = layout(&t);
        assert!(placed[0].1.italic && placed[0].1.ch == '*');
        let ok = placed.iter().find(|(_, p)| p.ch == 'o').unwrap().1;
        assert_eq!((ok.x, ok.y, ok.italic), (0, LINE, false));
    }
}
