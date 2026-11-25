//! The GDI text of 同級生 Remake: the hint bars, flag list, save card and
//! comment field, backlog hint, name entry and the hero's name plate.
//!
//! The executable draws these with a GDI font, one glyph at a time, and
//! never with the bitmap font (see `docs/games/doukyuusei.md`). Here the
//! face is the TrueType face the frontend passes, standing in for SimHei
//! (and MS Gothic in the name entry).
//!
//! A font set of size `H` has three fonts: weight 400 with cell height `H`,
//! 700 with `H - 2` and 900 with `H - 4`. The glyph's bitmap has 65 coverage
//! levels, turned to alpha as `level * 255 / 64`, and sits so that the
//! font's ascent lies below the cell's top. Bold weights are made here by
//! widening the glyph sideways (an approximation of the heavier face).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::canvas::{Canvas, Glyph};

/// The style flags: bold weight 700.
pub const BOLD: u32 = 1;
/// Outline: the glyph in the outline colour at nine offsets, then the fill.
pub const OUTLINE: u32 = 2;
/// Shadow: the outline colour one pixel down and right.
pub const SHADOW: u32 = 8;
/// Weight 900 (when [`BOLD`] is not set).
pub const HEAVY: u32 = 0x10;

/// How a glyph is coloured and drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fill: [u8; 3],
    pub outline: [u8; 3],
    pub flags: u32,
}

impl Style {
    /// White with a black outline, weight 700: the game's text style
    /// (flags 3), as the hint bars and the name entry take it.
    pub const OUTLINED: Style = Style {
        fill: [255; 3],
        outline: [0; 3],
        flags: BOLD | OUTLINE,
    };
    /// Black, plain weight 400 (the save dialog's comment field).
    pub const PLAIN_BLACK: Style = Style {
        fill: [0; 3],
        outline: [255; 3],
        flags: 0,
    };
}

/// A glyph's coverage with its place relative to the pen (the cell's top
/// left).
#[derive(Debug)]
pub struct Raster {
    pub dx: i32,
    pub dy: i32,
    pub glyph: Arc<Glyph>,
}

/// Coverage 0..255 to the 65 levels of `GGO_GRAY8`, then to the alpha the
/// executable writes: `level * 255 / 64`.
pub fn quantize(cov: u8) -> u8 {
    let level = (cov as u32 * 64 + 127) / 255;
    (level * 255 / 64) as u8
}

/// The mask widened sideways by `by` pixels (each pixel takes the largest
/// of itself and the `by` pixels to its left).
pub fn dilate(mask: &[u8], w: usize, h: usize, by: usize) -> (Vec<u8>, usize) {
    let nw = w + by;
    let mut out = vec![0u8; nw * h];
    for y in 0..h {
        for x in 0..w {
            let v = mask[y * w + x];
            for k in 0..=by {
                let o = &mut out[y * nw + x + k];
                *o = (*o).max(v);
            }
        }
    }
    (out, nw)
}

/// The cell height and weight of the font a style flag set picks at `size`.
pub fn font_of(flags: u32, size: u32) -> (u32, u32) {
    if flags & BOLD != 0 {
        (size.saturating_sub(2), 700)
    } else if flags & HEAVY != 0 {
        (size.saturating_sub(4), 900)
    } else {
        (size, 400)
    }
}

/// One glyph of `face` in a cell `h` high: the face is scaled so that its
/// ascent plus descent is `h`, the bitmap's top is the ascent minus the
/// glyph's top above the baseline.
pub fn rasterize(face: &fontdue::Font, c: char, h: u32, weight: u32) -> Option<Raster> {
    if h == 0 || face.lookup_glyph_index(c) == 0 {
        return None;
    }
    let lm = face.horizontal_line_metrics(1.0)?;
    let span = lm.ascent - lm.descent;
    if span <= 0.0 {
        return None;
    }
    let px = h as f32 / span;
    let ascent = (lm.ascent * px).round() as i32;
    let (m, mask) = face.rasterize(c, px);
    if m.width == 0 || m.height == 0 {
        return None;
    }
    let mask: Vec<u8> = mask.into_iter().map(quantize).collect();
    let by = match weight {
        700 => 1,
        900 => 2,
        _ => 0,
    };
    let (mask, width) = dilate(&mask, m.width, m.height, by);
    Some(Raster {
        dx: m.xmin,
        dy: ascent - (m.ymin + m.height as i32),
        glyph: Arc::new(Glyph {
            xmin: 0,
            ymin: 0,
            width,
            height: m.height,
            mask,
        }),
    })
}

type Cache = Mutex<HashMap<(char, u32, u32), Option<Arc<Raster>>>>;

/// The glyph of `c` in the frontend's faces (the first that has it).
pub fn glyph(c: char, h: u32, weight: u32) -> Option<Arc<Raster>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let faces = super::window::faces()?;
    let mut cache = CACHE.get_or_init(Default::default).lock().ok()?;
    cache
        .entry((c, h, weight))
        .or_insert_with(|| {
            let face = faces.iter().find(|f| f.lookup_glyph_index(c) != 0)?;
            rasterize(face, c, h, weight).map(Arc::new)
        })
        .clone()
}

/// Whether the executable counts `c` as wide (East Asian Width: F, W or A;
/// the pen then moves a whole size, else half).
pub fn is_wide(c: char) -> bool {
    let u = c as u32;
    // Latin-1 letters and signs with ambiguous width.
    const LATIN1: &[u32] = &[
        0xA1, 0xA4, 0xA7, 0xA8, 0xAA, 0xAD, 0xAE, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB6, 0xB7, 0xB8,
        0xB9, 0xBA, 0xBC, 0xBD, 0xBE, 0xBF, 0xC6, 0xD0, 0xD7, 0xD8, 0xDE, 0xDF, 0xE0, 0xE1, 0xE6,
        0xE8, 0xE9, 0xEA, 0xEC, 0xED, 0xF0, 0xF2, 0xF3, 0xF7, 0xF8, 0xF9, 0xFA, 0xFC, 0xFE,
    ];
    // Wide (W, F) and the common ambiguous (A) ranges.
    const WIDE: &[(u32, u32)] = &[
        (0x0391, 0x03C9),
        (0x0401, 0x0451),
        (0x1100, 0x115F),
        (0x2010, 0x2027),
        (0x2030, 0x203E),
        (0x2103, 0x2103),
        (0x2105, 0x2105),
        (0x2109, 0x2109),
        (0x2113, 0x2113),
        (0x2116, 0x2116),
        (0x2121, 0x2122),
        (0x2160, 0x216B),
        (0x2170, 0x2179),
        (0x2190, 0x2199),
        (0x21D2, 0x21D2),
        (0x21D4, 0x21D4),
        (0x2200, 0x22FF),
        (0x2460, 0x24E9),
        (0x24EB, 0x254B),
        (0x2550, 0x2573),
        (0x2580, 0x258F),
        (0x2592, 0x2595),
        (0x25A0, 0x25FF),
        (0x2605, 0x2606),
        (0x2640, 0x2642),
        (0x2660, 0x266F),
        (0x2E80, 0x303E),
        (0x3041, 0x33FF),
        (0x3400, 0x4DBF),
        (0x4E00, 0x9FFF),
        (0xA000, 0xA4CF),
        (0xAC00, 0xD7A3),
        (0xE000, 0xF8FF),
        (0xF900, 0xFAFF),
        (0xFE30, 0xFE6F),
        (0xFF00, 0xFF60),
        (0xFFE0, 0xFFE6),
        (0x1F300, 0x1F64F),
        (0x20000, 0x3FFFD),
    ];
    if u < 0x100 {
        return LATIN1.contains(&u);
    }
    WIDE.iter().any(|&(a, b)| (a..=b).contains(&u))
}

/// The pen's move after `c`: `adv.0` when wide, else `adv.1`.
pub fn advance(c: char, adv: (i32, i32)) -> i32 {
    if is_wide(c) {
        adv.0
    } else {
        adv.1
    }
}

/// Blends `r` in `color` at (x, y) (the cell's top left), cutting what
/// lies right of `clip`.
fn blit(canvas: &mut Canvas, r: &Raster, x: i32, y: i32, color: [u8; 3], clip: Option<i32>) {
    let (gx, gy) = (x + r.dx, y + r.dy);
    let rgba = [color[0], color[1], color[2], 255];
    match clip {
        Some(limit) if gx + r.glyph.width as i32 > limit => {
            let keep = (limit - gx).max(0) as usize;
            if keep == 0 {
                return;
            }
            let g = &r.glyph;
            let mut mask = Vec::with_capacity(keep * g.height);
            for row in 0..g.height {
                mask.extend_from_slice(&g.mask[row * g.width..row * g.width + keep]);
            }
            let cut = Arc::new(Glyph {
                xmin: 0,
                ymin: 0,
                width: keep,
                height: g.height,
                mask,
            });
            canvas.draw_glyph(&cut, gx, gy, false, rgba);
        }
        _ => canvas.draw_glyph(&r.glyph, gx, gy, false, rgba),
    }
}

/// Draws one character with its cell's top left at (x, y) in a font set of
/// `size`, as the executable does: the nine outline offsets then the fill
/// one pixel in (outline), the shadow then the fill (shadow), or the fill
/// alone. A character the faces lack leaves nothing.
pub fn draw_char(canvas: &mut Canvas, c: char, x: i32, y: i32, size: u32, style: Style) {
    draw_clipped(canvas, c, x, y, size, style, None);
}

fn draw_clipped(
    canvas: &mut Canvas,
    c: char,
    x: i32,
    y: i32,
    size: u32,
    style: Style,
    clip: Option<i32>,
) {
    let (h, weight) = font_of(style.flags, size);
    let Some(r) = glyph(c, h, weight) else {
        return;
    };
    if style.flags & OUTLINE != 0 {
        for dy in 0..3 {
            for dx in 0..3 {
                blit(canvas, &r, x + dx, y + dy, style.outline, clip);
            }
        }
        blit(canvas, &r, x + 1, y + 1, style.fill, clip);
    } else if style.flags & SHADOW != 0 {
        blit(canvas, &r, x + 1, y + 1, style.outline, clip);
        blit(canvas, &r, x, y, style.fill, clip);
    } else {
        blit(canvas, &r, x, y, style.fill, clip);
    }
}

/// Draws a line from (x, y), the pen moving `adv` (wide, narrow) after each
/// character; returns the pen's x. Characters whose cell starts at or past
/// `clip` are left out and one that crosses it is cut.
pub fn draw_line(
    canvas: &mut Canvas,
    text: &str,
    x: i32,
    y: i32,
    size: u32,
    adv: (i32, i32),
    style: Style,
    clip: Option<i32>,
) -> i32 {
    let mut cx = x;
    for c in text.chars() {
        if clip.is_some_and(|l| cx >= l) {
            break;
        }
        draw_clipped(canvas, c, cx, y, size, style, clip);
        cx += advance(c, adv);
    }
    cx
}

/// Where the hint drawer puts each character of `text`: (char, x, line),
/// with x from the left edge. It wraps before a character whose end would
/// pass `width` rounded down to whole half cells; `\n` breaks the line.
/// Also the number of lines.
pub fn hint_layout(text: &str, width: i32, size: i32) -> (Vec<(char, i32, i32)>, i32) {
    let half = size / 2;
    let max = if half > 0 { width / half * half } else { width };
    let adv = (size, half);
    let (mut x, mut line) = (0, 0);
    let mut out = Vec::new();
    for c in text.chars() {
        if c == '\n' {
            x = 0;
            line += 1;
            continue;
        }
        let a = advance(c, adv);
        if x > 0 && x + a > max {
            x = 0;
            line += 1;
        }
        out.push((c, x, line));
        x += a;
    }
    (out, line + 1)
}

/// Draws a hint as the executable's hint drawer does: white with a black
/// outline, left aligned from `x`, `size` wide and high per full cell, wrapped
/// within `width`. `y` is where a one-line hint starts; more lines move the
/// first one up by half a line each, which centres the text in its bar.
pub fn draw_hint(canvas: &mut Canvas, text: &str, x: i32, y: i32, size: u32, width: i32) {
    let (cells, lines) = hint_layout(text, width, size as i32);
    let top = y - (lines - 1) * size as i32 / 2;
    for (c, cx, line) in cells {
        draw_char(
            canvas,
            c,
            x + cx,
            top + line * size as i32,
            size,
            Style::OUTLINED,
        );
    }
}

/// [`draw_hint`] seen through `opacity` (255 = as it is): the text is laid
/// out as one picture first, so the outline does not darken where its
/// passes overlap, then drawn as a whole (a sprite that fades).
pub fn draw_hint_faded(
    canvas: &mut Canvas,
    text: &str,
    pos: (i32, i32),
    (size, width): (u32, i32),
    opacity: u8,
) {
    if opacity == 255 {
        return draw_hint(canvas, text, pos.0, pos.1, size, width);
    }
    if opacity == 0 {
        return;
    }
    let (cells, lines) = hint_layout(text, width, size as i32);
    let top = pos.1 - (lines - 1) * size as i32 / 2;
    let (h, weight) = font_of(Style::OUTLINED.flags, size);
    // Every glyph with the place of its cell, and the box around them.
    let placed: Vec<_> = cells
        .into_iter()
        .filter_map(|(c, cx, line)| {
            let r = glyph(c, h, weight)?;
            Some((r, pos.0 + cx, top + line * size as i32))
        })
        .collect();
    let at = |r: &Raster, x: i32, y: i32| (x + r.dx, y + r.dy);
    let left = placed.iter().map(|(r, x, y)| at(r, *x, *y).0).min();
    let up = placed.iter().map(|(r, x, y)| at(r, *x, *y).1).min();
    let right = placed
        .iter()
        .map(|(r, x, y)| at(r, *x, *y).0 + r.glyph.width as i32 + 2)
        .max();
    let down = placed
        .iter()
        .map(|(r, x, y)| at(r, *x, *y).1 + r.glyph.height as i32 + 2)
        .max();
    let (Some(left), Some(up), Some(right), Some(down)) = (left, up, right, down) else {
        return;
    };
    let (w, hgt) = ((right - left) as usize, (down - up) as usize);
    let mut rgba = vec![0u8; w * hgt * 4];
    // The glyph in `color` over what the picture holds (straight alpha).
    let mut over = |r: &Raster, x: i32, y: i32, color: [u8; 3]| {
        let g = &r.glyph;
        for row in 0..g.height {
            for col in 0..g.width {
                let cov = g.mask[row * g.width + col] as u32;
                if cov == 0 {
                    continue;
                }
                let px = (x + r.dx + col as i32 - left) as usize;
                let py = (y + r.dy + row as i32 - up) as usize;
                let d = &mut rgba[(py * w + px) * 4..][..4];
                let da = d[3] as u32;
                let out = cov + da * (255 - cov) / 255;
                for k in 0..3 {
                    let mixed = color[k] as u32 * cov + d[k] as u32 * da * (255 - cov) / 255;
                    d[k] = (mixed / out.max(1)) as u8;
                }
                d[3] = out as u8;
            }
        }
    };
    let style = Style::OUTLINED;
    for (r, x, y) in &placed {
        for dy in 0..3 {
            for dx in 0..3 {
                over(r, x + dx, y + dy, style.outline);
            }
        }
        over(r, x + 1, y + 1, style.fill);
    }
    let image = Arc::new(silky_format::akb::Image {
        width: w as u32,
        height: hgt as u32,
        rgba,
    });
    canvas.draw_part(
        &image,
        (0, 0, w as u32, hgt as u32),
        left,
        up,
        false,
        opacity,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_levels() {
        assert_eq!(quantize(0), 0);
        assert_eq!(quantize(255), 255);
        assert_eq!(quantize(128), 127);
        // One level of 64 is alpha 3.
        assert_eq!(quantize(4), 3);
    }

    #[test]
    fn weights_by_flags() {
        assert_eq!(font_of(0, 40), (40, 400));
        assert_eq!(font_of(BOLD | OUTLINE, 40), (38, 700));
        assert_eq!(font_of(HEAVY, 40), (36, 900));
        // Bold has priority over heavy.
        assert_eq!(font_of(BOLD | HEAVY, 40), (38, 700));
    }

    #[test]
    fn dilation_widens() {
        let (m, w) = dilate(&[0, 9, 0], 3, 1, 1);
        assert_eq!((m, w), (vec![0, 9, 9, 0], 4));
        let (m, w) = dilate(&[5, 0], 2, 1, 2);
        assert_eq!((m, w), (vec![5, 5, 5, 0], 4));
    }

    #[test]
    fn wide_by_east_asian_width() {
        assert!(!is_wide('a') && !is_wide(' ') && !is_wide('7'));
        assert!(is_wide('拓') && is_wide('あ') && is_wide('Ａ') && is_wide('—'));
        assert!(!is_wide('\u{FF71}'));
        assert_eq!(advance('a', (26, 13)), 13);
        assert_eq!(advance('拓', (26, 13)), 26);
    }

    #[test]
    fn hint_wraps_in_half_cells() {
        // 500 wide, size 20: 25 wide characters to a line.
        let text: String = "あ".repeat(26);
        let (cells, lines) = hint_layout(&text, 500, 20);
        assert_eq!(lines, 2);
        assert_eq!(cells[24], ('あ', 480, 0));
        assert_eq!(cells[25], ('あ', 0, 1));
        // A break and a narrow run.
        let (cells, lines) = hint_layout("ab\ncd", 500, 20);
        assert_eq!(lines, 2);
        assert_eq!((cells[1].1, cells[2].1, cells[2].2), (10, 0, 1));
        // The width rounds down to whole half cells.
        let (_, lines) = hint_layout(&"あ".repeat(12), 255, 20);
        assert_eq!(lines, 1);
        let (_, lines) = hint_layout(&"あ".repeat(13), 255, 20);
        assert_eq!(lines, 2);
    }

    fn face() -> Option<fontdue::Font> {
        let data = std::fs::read("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc").ok()?;
        fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).ok()
    }

    #[test]
    fn glyph_sits_on_the_ascent() {
        let Some(face) = face() else { return };
        let r = rasterize(&face, '拓', 40, 400).unwrap();
        // Inside the 40 high cell, alpha only from the 65 levels.
        assert!(r.dy >= 0 && r.dy + r.glyph.height as i32 <= 40);
        assert!(r.glyph.mask.iter().all(|&a| quantize(a) == a));
        assert!(r.glyph.mask.iter().any(|&a| a == 255));
        let bold = rasterize(&face, '拓', 38, 700).unwrap();
        let plain = rasterize(&face, '拓', 38, 400).unwrap();
        assert_eq!(bold.glyph.width, plain.glyph.width + 1);
        assert!(rasterize(&face, '\u{E000}', 40, 400).is_none());
    }

    #[test]
    fn outline_surrounds_the_fill() {
        let Some(face) = face() else { return };
        super::super::window::set_fallback(&[face]);
        let mut canvas = Canvas::new(60, 60);
        draw_char(&mut canvas, '拓', 5, 5, 40, Style::OUTLINED);
        let px = |x: usize, y: usize| &canvas.rgba[(y * 60 + x) * 4..(y * 60 + x) * 4 + 4];
        let white = (0..60 * 60)
            .filter(|&i| px(i % 60, i / 60) == [255, 255, 255, 255])
            .count();
        let black = (0..60 * 60)
            .filter(|&i| px(i % 60, i / 60) == [0, 0, 0, 255])
            .count();
        assert!(white > 50 && black > 50);
    }
}
