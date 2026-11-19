//! Message window of 同級生 Remake: the band and frame of `message_p.akb`,
//! the speaker's name plate, and the text in the game's own bitmap fonts.
//!
//! Measured under Wine at 1920 x 1080 and read from the executable (see
//! `docs/games/doukyuusei.md`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use silky_format::akb::Image;

use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// Top of the window: the band and the frame are each 350 rows of the sheet
/// (`mes_o.area` gives the window as 1920 x 350).
pub const TOP: i32 = 730;
/// Translucent black band (sheet rows 0..350) and its frame lines (rows
/// 350..700), both drawn at [`TOP`]. The band's alpha masks the window
/// colour of the settings, drawn at the window opacity.
pub const BAND: (u32, u32, u32, u32) = (0, 0, 1920, 350);
const FRAME: (u32, u32, u32, u32) = (0, 350, 1920, 350);
/// Name plate (a star and a gold rule, sheet rows 1350..1400) and where its
/// text starts.
const NAME_PART: (u32, u32, u32, u32) = (0, 1350, 1500, 50);
const NAME_PART_POS: (i32, i32) = (231, 799);
const NAME_TEXT: (i32, i32) = (294, 800);
/// Speaker icons: 144 x 144 cells of `message_icon.akb`, four to a row,
/// 118 pixels below the window top.
const ICONS: &str = "message_icon.akb";
const ICON: u32 = 144;
const ICON_POS: (i32, i32) = (78, TOP + 118);
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

/// Width of a text in cells (full-width characters take two).
pub fn text_cells(text: &str) -> i32 {
    text.chars().map(cells).sum()
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
        // A full-width character may end on the limit (Chinese lines
        // under Wine hold 37 of them); a word of letters may not.
        let over = if half(c) {
            x + width >= LINE_CELLS
        } else {
            x + width > LINE_CELLS
        };
        if x > 0 && over {
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
    /// The glyph in gray (0x808080), for disabled options.
    gray: Arc<Image>,
    w: u32,
}

/// Half the size, each pixel the average of four (straight alpha).
fn halve(img: &Image) -> Image {
    let (w, h) = (img.width / 2, img.height / 2);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = (((2 * y + dy) * img.width + 2 * x + dx) * 4) as usize;
                let a = img.rgba[i + 3] as u32;
                for k in 0..3 {
                    acc[k] += img.rgba[i + k] as u32 * a;
                }
                acc[3] += a;
            }
            let o = ((y * w + x) * 4) as usize;
            if acc[3] > 0 {
                for k in 0..3 {
                    rgba[o + k] = (acc[k] / acc[3]) as u8;
                }
                rgba[o + 3] = ((acc[3] + 2) / 4) as u8;
            }
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// A part of an image resized to `w` x `h`, each pixel the area-weighted
/// average of the pixels it covers (straight alpha).
pub(super) fn resized(img: &Image, part: (u32, u32, u32, u32), w: u32, h: u32) -> Image {
    let (px, py, pw, ph) = part;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    // Weights in 1/(w*h) source pixels: output pixel (x, y) covers source
    // [x*pw/w, (x+1)*pw/w).
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u64; 4];
            let mut total = 0u64;
            let (sx0, sx1) = (x * pw, (x + 1) * pw);
            let (sy0, sy1) = (y * ph, (y + 1) * ph);
            for sy in sy0 / h..sy1.div_ceil(h) {
                let wy = ((sy + 1) * h).min(sy1).saturating_sub((sy * h).max(sy0)) as u64;
                for sx in sx0 / w..sx1.div_ceil(w) {
                    let wx = ((sx + 1) * w).min(sx1).saturating_sub((sx * w).max(sx0)) as u64;
                    let i = (((py + sy) * img.width + px + sx) * 4) as usize;
                    let a = img.rgba[i + 3] as u64 * wx * wy;
                    for k in 0..3 {
                        acc[k] += img.rgba[i + k] as u64 * a;
                    }
                    acc[3] += a;
                    total += wx * wy;
                }
            }
            let o = ((y * w + x) * 4) as usize;
            if acc[3] > 0 {
                for k in 0..3 {
                    rgba[o + k] = (acc[k] / acc[3]) as u8;
                }
                rgba[o + 3] = ((acc[3] + total / 2) / total) as u8;
            }
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

impl Sheet {
    fn new(img: Arc<Image>, w: u32) -> Self {
        // Same glyph with its colour replaced, the alpha kept.
        let tint = |v: u8| {
            let mut rgba = img.rgba.clone();
            for p in rgba.chunks_exact_mut(4) {
                p[0] = v;
                p[1] = v;
                p[2] = v;
            }
            Arc::new(Image {
                width: img.width,
                height: img.height,
                rgba,
            })
        };
        Self {
            black: tint(0),
            gray: tint(0x80),
            white: img,
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
    /// Half-size sheets, for the room's hint bar (size 20 in the
    /// executable).
    han_small: Sheet,
    zen_small: Sheet,
    han_index: HashMap<char, u32>,
    zen_index: HashMap<char, u32>,
    /// Cells of characters drawn with the fallback face (black, white and
    /// gray copies), by character.
    extra: Mutex<HashMap<char, Option<Arc<[Image; 3]>>>>,
}

/// The TrueType faces the frontend gave, for characters missing from
/// `font_zen.map`.
static FALLBACK: OnceLock<Vec<fontdue::Font>> = OnceLock::new();

/// Sets the faces used for characters the bitmap font lacks (the first
/// call wins). Without it such characters are skipped.
pub fn set_fallback(fonts: &[fontdue::Font]) {
    if !fonts.is_empty() {
        let _ = FALLBACK.set(fonts.to_vec());
    }
}

/// Pixel size of the fallback face in a 40 pixel cell: the executable's
/// GDI font is 40 pixels high with its leading, about this tall in the em.
const FALLBACK_SIZE: f32 = 34.0;
/// Baseline of the fallback face below the top of the cell.
const FALLBACK_BASE: i32 = 31;

const GLYPH_H: u32 = 40;
/// Where the black copies of a glyph go.
const OUTLINE: [(i32, i32); 8] = [
    (0, -1),
    (0, 1),
    (-1, 0),
    (1, 0),
    (-1, -1),
    (-1, 1),
    (1, -1),
    (1, 1),
];
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
        let han = assets.image("font_han.akb")?;
        let zen = assets.image("font_zen.akb")?;
        Ok(Self {
            han_small: Sheet::new(Arc::new(halve(&han)), 10),
            zen_small: Sheet::new(Arc::new(halve(&zen)), 20),
            han: Sheet::new(han, 20),
            han_i: Sheet::new(assets.image("font_han_i.akb")?, 20),
            zen: Sheet::new(zen, 40),
            han_index: index(&assets.data("font_han.map")?),
            zen_index: index(&assets.data("font_zen.map")?),
            extra: Default::default(),
        })
    }

    /// Whether the bitmap font has no glyph for a full-width character
    /// that the fallback face can draw.
    fn lacks(&self, c: char) -> bool {
        !half(c) && !self.zen_index.contains_key(&c) && self.extra_cell(c).is_some()
    }

    /// The 40 x 40 cell of `c` in the fallback face (black, white, gray).
    fn extra_cell(&self, c: char) -> Option<Arc<[Image; 3]>> {
        let faces = FALLBACK.get()?;
        let mut cache = self.extra.lock().ok()?;
        cache
            .entry(c)
            .or_insert_with(|| {
                let face = faces.iter().find(|f| f.lookup_glyph_index(c) != 0)?;
                let (m, mask) = face.rasterize(c, FALLBACK_SIZE);
                let n = GLYPH_H as i32;
                let mut cells = [0u8, 0xff, 0x80].map(|v| Image {
                    width: n as u32,
                    height: n as u32,
                    rgba: [v, v, v, 0].repeat((n * n) as usize),
                });
                let left = (n - m.width as i32) / 2 - 2;
                let top = FALLBACK_BASE - m.height as i32 - m.ymin;
                for gy in 0..m.height as i32 {
                    for gx in 0..m.width as i32 {
                        let (x, y) = (left + gx, top + gy);
                        if (0..n).contains(&x) && (0..n).contains(&y) {
                            // Strokes a pixel wider, as the heavier GDI face
                            // under the executable.
                            let a = mask[(gy * m.width as i32 + gx) as usize];
                            for cell in cells.iter_mut() {
                                for x in [x, x + 1].into_iter().filter(|x| *x < n) {
                                    let at = ((y * n + x) * 4 + 3) as usize;
                                    cell.rgba[at] = cell.rgba[at].max(a);
                                }
                            }
                        }
                    }
                }
                Some(Arc::new(cells))
            })
            .clone()
    }

    /// Draws a full-width character with the fallback face, outlined as
    /// the bitmap glyphs are.
    fn fallback_glyph(
        &self,
        canvas: &mut Canvas,
        c: char,
        x: i32,
        y: i32,
        opacity: u8,
        gray: bool,
    ) {
        let Some(cells) = self.extra_cell(c) else {
            return;
        };
        let part = (0, 0, GLYPH_H, GLYPH_H);
        let black = Arc::new(cells[0].clone());
        for (dx, dy) in OUTLINE {
            canvas.draw_part(&black, part, x + dx, y + dy, false, opacity);
        }
        let fill = Arc::new(cells[if gray { 2 } else { 1 }].clone());
        canvas.draw_part(&fill, part, x, y, false, opacity);
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
        self.glyph(canvas, c, x, y, italic, opacity, false);
    }

    /// The glyph of [`Font::draw_char`], in gray when `gray` is set.
    fn glyph(
        &self,
        canvas: &mut Canvas,
        c: char,
        x: i32,
        y: i32,
        italic: bool,
        opacity: u8,
        gray: bool,
    ) {
        let (sheet, at) = if half(c) {
            let sheet = if italic { &self.han_i } else { &self.han };
            (sheet, self.han_index.get(&c))
        } else {
            (&self.zen, self.zen_index.get(&c))
        };
        let Some(&i) = at else {
            if !half(c) {
                self.fallback_glyph(canvas, c, x, y, opacity, gray);
            }
            return;
        };
        let part = (
            (i % PER_ROW) * sheet.w,
            (i / PER_ROW) * GLYPH_H,
            sheet.w,
            GLYPH_H,
        );
        for (dx, dy) in OUTLINE {
            canvas.draw_part(&sheet.black, part, x + dx, y + dy, false, opacity);
        }
        let fill = if gray { &sheet.gray } else { &sheet.white };
        canvas.draw_part(fill, part, x, y, false, opacity);
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

    /// Draws a line of text at half size from (x, y), outlined like the
    /// message text.
    pub fn draw_small(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32) {
        let mut cx = x;
        for c in text.chars() {
            let (sheet, at) = if half(c) {
                (&self.han_small, self.han_index.get(&c))
            } else {
                (&self.zen_small, self.zen_index.get(&c))
            };
            if let Some(&i) = at {
                let h = GLYPH_H / 2;
                let part = ((i % PER_ROW) * sheet.w, (i / PER_ROW) * h, sheet.w, h);
                for (dx, dy) in OUTLINE {
                    canvas.draw_part(&sheet.black, part, cx + dx, y + dy, false, 255);
                }
                canvas.draw_part(&sheet.white, part, cx, y, false, 255);
            }
            cx += cells(c) * CELL / 2;
        }
    }

    /// Draws a line of text from (x, y) at a font size below the glyphs'
    /// 40 (the cells scaled down), outlined like the message text.
    pub fn draw_sized(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32, size: u32) {
        let mut cx = x;
        for c in text.chars() {
            let (sheet, at) = if half(c) {
                (&self.han, self.han_index.get(&c))
            } else {
                (&self.zen, self.zen_index.get(&c))
            };
            if let Some(&i) = at {
                let part = (
                    (i % PER_ROW) * sheet.w,
                    (i / PER_ROW) * GLYPH_H,
                    sheet.w,
                    GLYPH_H,
                );
                let (w, h) = (sheet.w * size / GLYPH_H, size);
                let white = Arc::new(resized(&sheet.white, part, w, h));
                let black = Arc::new(resized(&sheet.black, part, w, h));
                for (dx, dy) in OUTLINE {
                    canvas.draw(&black, cx + dx, y + dy, false);
                }
                canvas.draw(&white, cx, y, false);
            }
            cx += cells(c) * CELL * size as i32 / GLYPH_H as i32;
        }
    }

    /// Draws a name from (x, y). One with a full-width character the
    /// bitmap font lacks is drawn whole with the fallback face, as the
    /// executable draws such names with a GDI font.
    pub fn draw_name(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32) {
        if !text.chars().any(|c| self.lacks(c)) {
            return self.draw_line(canvas, text, x, y);
        }
        let mut cx = x;
        for c in text.chars() {
            if half(c) {
                self.glyph(canvas, c, cx, y, false, 255, false);
            } else {
                self.fallback_glyph(canvas, c, cx, y, 255, false);
            }
            cx += cells(c) * CELL;
        }
    }

    /// Draws a line of text from (x, y) without wrapping.
    pub fn draw_line(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32) {
        self.line(canvas, text, x, y, false);
    }

    /// As [`Font::draw_line`], with the text in gray (0x808080).
    pub fn draw_line_gray(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32) {
        self.line(canvas, text, x, y, true);
    }

    fn line(&self, canvas: &mut Canvas, text: &str, x: i32, y: i32, gray: bool) {
        let mut cx = x;
        for c in text.chars() {
            self.glyph(canvas, c, cx, y, false, 255, gray);
            cx += cells(c) * CELL;
        }
    }
}

/// Draws the window band (the [`BAND`] mask filled with the window colour,
/// at an opacity), frame and name plate.
pub fn draw_window(
    canvas: &mut Canvas,
    assets: &mut Assets,
    font: &Font,
    (band, opacity): (&Arc<Image>, u8),
    speaker: Option<&str>,
    icon: Option<u32>,
) -> Result<()> {
    let parts = assets.image("message_p.akb")?;
    canvas.draw_part(
        band,
        (0, 0, band.width, band.height),
        0,
        TOP,
        false,
        opacity,
    );
    canvas.draw_part(&parts, FRAME, 0, TOP, false, 255);
    if let Some(i) = icon {
        let sheet = assets.image(ICONS)?;
        let part = (i % 4 * ICON, i / 4 * ICON, ICON, ICON);
        canvas.draw_part(&sheet, part, ICON_POS.0, ICON_POS.1, false, 255);
    }
    if let Some(name) = speaker {
        let (x, y) = NAME_PART_POS;
        canvas.draw_part(&parts, NAME_PART, x, y, false, 255);
        font.draw_name(canvas, name, NAME_TEXT.0, NAME_TEXT.1);
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
    fn missing_glyph_uses_the_fallback_face() {
        let path = "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc";
        let Ok(data) = std::fs::read(path) else {
            return;
        };
        let face =
            fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).unwrap();
        set_fallback(&[face]);
        let sheet = || {
            Sheet::new(
                Arc::new(Image {
                    width: 20,
                    height: 40,
                    rgba: vec![0; 3200],
                }),
                20,
            )
        };
        let font = Font {
            han: sheet(),
            han_i: sheet(),
            zen: sheet(),
            han_small: sheet(),
            zen_small: sheet(),
            han_index: HashMap::new(),
            zen_index: HashMap::new(),
            extra: Default::default(),
        };
        assert!(font.lacks('拓'));
        assert!(!font.lacks('a'));
        let cell = font.extra_cell('拓').unwrap();
        assert!(cell[1].rgba.chunks_exact(4).any(|p| p[3] > 200));
        assert!(cell[0].rgba.chunks_exact(4).all(|p| p[0] == 0));
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
    fn chinese_line_holds_37() {
        // Matched under Wine: the 37th character ends on the limit.
        let text =
            "因为我过着学生版的单身出差生活。那种被温暖家庭仔细呵护长大的家伙是不会懂我的辛苦的。";
        let rows = lines(text);
        assert_eq!(rows[0].chars().count(), 37);
        assert!(rows[1].starts_with('的'));
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
