//! Text drawing with system fonts.
//!
//! The game itself draws with GDI fonts named in its settings (Arial for
//! English, Microsoft YaHei / JhengHei for Chinese). Fonts are not part of
//! the game files, so the frontend passes font files in; glyphs missing
//! from the first font are taken from the next.

use fontdue::{Font, FontSettings};

use crate::render::Canvas;
use crate::{Error, Result};

pub struct TextRenderer {
    fonts: Vec<Font>,
    /// Pen advance of CJK characters (and the punctuation used with them),
    /// as a multiple of the text size. `None` uses the font's own advance;
    /// set it for Chinese text only. A screen with its own spacing changes
    /// it for a while with [`TextRenderer::with_wide_advance`].
    wide_advance: std::cell::Cell<Option<f32>>,
}

impl TextRenderer {
    /// Loads fonts from file contents, in fallback order. A `.ttc` uses its
    /// first face.
    pub fn new(files: &[Vec<u8>]) -> Result<Self> {
        let fonts = files
            .iter()
            .map(|data| {
                Font::from_bytes(data.as_slice(), FontSettings::default())
                    .map_err(|e| Error::Font(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        if fonts.is_empty() {
            return Err(Error::Font("no font given".into()));
        }
        Ok(Self {
            fonts,
            wide_advance: Default::default(),
        })
    }

    pub fn set_wide_advance(&mut self, k: Option<f32>) {
        self.wide_advance.set(k);
    }

    /// Runs `f` with CJK characters `k` times the size apart (if the text
    /// uses a fixed CJK advance at all).
    pub fn with_wide_advance<R>(&self, k: f32, f: impl FnOnce(&Self) -> R) -> R {
        let old = self.wide_advance.get();
        self.wide_advance.set(old.map(|_| k));
        let r = f(self);
        self.wide_advance.set(old);
        r
    }

    fn font_for(&self, c: char) -> &Font {
        self.fonts
            .iter()
            .find(|f| f.lookup_glyph_index(c) != 0)
            .unwrap_or(&self.fonts[0])
    }

    fn advance(&self, c: char, size: f32) -> f32 {
        match self.wide_advance.get() {
            Some(k) if takes_cell(c) => size * k,
            _ => self.font_for(c).metrics(c, size).advance_width,
        }
    }

    /// Breaks text into lines no wider than `width`: at spaces for words,
    /// between any two characters for CJK text.
    pub fn wrap(&self, text: &str, size: f32, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            let mut line_w = 0.0;
            for word in split_words(para) {
                let w: f32 = word.chars().map(|c| self.advance(c, size)).sum();
                if line_w + w > width && !line.trim().is_empty() {
                    lines.push(line.trim_end().to_string());
                    line = String::new();
                    line_w = 0.0;
                    if word == " " {
                        continue;
                    }
                }
                line.push_str(word);
                line_w += w;
            }
            lines.push(line.trim_end().to_string());
        }
        lines
    }

    /// Draws one line with its baseline at `y`, in `color` (RGBA) with a
    /// soft dark outline.
    pub fn draw_line(
        &self,
        canvas: &mut Canvas,
        line: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 4],
    ) {
        self.draw_layers(
            canvas,
            line,
            x,
            y,
            size,
            &[
                (1.0, 1.0, [0, 0, 0, 160]),
                (-1.0, 0.0, [0, 0, 0, 90]),
                (0.0, 0.0, color),
            ],
        );
    }

    /// Draws one line with a one-pixel outline all round.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_outlined(
        &self,
        canvas: &mut Canvas,
        line: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 4],
        outline: [u8; 4],
    ) {
        let mut layers: Vec<(f32, f32, [u8; 4])> = [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ]
        .iter()
        .map(|&(dx, dy)| (dx, dy, outline))
        .collect();
        layers.push((0.0, 0.0, color));
        self.draw_layers(canvas, line, x, y, size, &layers);
    }

    /// Draws one line without an outline, for text on light screens.
    pub fn draw_plain(
        &self,
        canvas: &mut Canvas,
        line: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 4],
    ) {
        self.draw_layers(canvas, line, x, y, size, &[(0.0, 0.0, color)]);
    }

    /// Width of a line.
    pub fn width(&self, line: &str, size: f32) -> f32 {
        line.chars().map(|c| self.advance(c, size)).sum()
    }

    fn draw_layers(
        &self,
        canvas: &mut Canvas,
        line: &str,
        x: f32,
        y: f32,
        size: f32,
        layers: &[(f32, f32, [u8; 4])],
    ) {
        // Positions stay in game pixels; on a canvas with an output scale
        // the glyphs are rasterized at output size.
        let s = canvas.scale();
        for &(dx, dy, c) in layers {
            let mut pen = x + dx;
            for ch in line.chars() {
                let font = self.font_for(ch);
                if s > 1.0 {
                    let (m, bitmap) = font.rasterize(ch, size * s);
                    let gx = (pen * s).round() as i32 + m.xmin;
                    let gy = ((y + dy) * s).round() as i32 - m.height as i32 - m.ymin;
                    canvas.blend_mask_sharp(gx, gy, m.width, m.height, &bitmap, c);
                } else {
                    let (m, bitmap) = font.rasterize(ch, size);
                    let gx = pen.round() as i32 + m.xmin;
                    let gy = (y + dy).round() as i32 - m.height as i32 - m.ymin;
                    canvas.blend_mask(gx, gy, m.width, m.height, &bitmap, c);
                }
                pen += self.advance(ch, size);
            }
        }
    }
}

/// CJK characters and full-width forms: a line may break around them.
fn is_wide(c: char) -> bool {
    matches!(
        c as u32,
        0x2e80..=0xd7ff | 0xf900..=0xfaff | 0xfe30..=0xfe4f | 0xff00..=0xff60 | 0xffe0..=0xffe6
    )
}

/// Characters that take a full cell in CJK text, including the quotes and
/// dashes that Latin fonts draw narrow.
fn takes_cell(c: char) -> bool {
    is_wide(c) || matches!(c, '“' | '”' | '‘' | '’' | '…' | '—')
}

/// Words with their following space kept as separate items; each CJK
/// character is its own item.
fn split_words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == ' ' || is_wide(c) {
            if start < i {
                out.push(&text[start..i]);
            }
            out.push(&text[i..i + c.len_utf8()]);
            start = i + c.len_utf8();
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::split_words;

    #[test]
    fn words_and_cjk() {
        assert_eq!(split_words("Hi there"), ["Hi", " ", "there"]);
        assert_eq!(split_words("你好a"), ["你", "好", "a"]);
    }
}
