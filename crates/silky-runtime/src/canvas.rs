//! RGBA frames in game pixels, with text kept sharp at the output size.
//!
//! The games lay everything out in their own pixels (1280 x 720), and their
//! pictures are stored at that size, so pictures are composed at that size.
//! The output can be larger (1920 x 1080, a phone screen): a canvas with an
//! output scale then keeps a second, larger buffer for the places where
//! text was drawn. Where text goes, that part of the frame is copied up to
//! the larger buffer and the glyphs are drawn there at full resolution;
//! anything drawn over such a place later is drawn into both buffers, so
//! the order of drawing holds. [`Canvas::present`] scales the picture up
//! once and puts the sharp parts on top.

use silky_format::akb::Image;

/// Side of the squares, in game pixels, that are kept at output size once
/// text touches them.
const TILE: u32 = 16;

/// An RGBA8 frame in game pixels.
#[derive(Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Output pixels per game pixel; 1 keeps everything at game size.
    scale: f32,
    /// The parts at output size, made when text is first drawn.
    sharp: Option<Box<Sharp>>,
}

#[derive(Clone)]
struct Sharp {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    /// Tiles (row by row, [`TILE`] game pixels square) held in `rgba`.
    tiles: Vec<bool>,
    columns: u32,
}

/// Blends `src` (straight alpha) onto an opaque pixel.
fn blend_into(d: &mut [u8], src: [u8; 4]) {
    let a = src[3] as u32;
    if a == 0 {
        return;
    }
    for k in 0..3 {
        d[k] = ((src[k] as u32 * a + d[k] as u32 * (255 - a) + 127) / 255) as u8;
    }
    d[3] = 255;
}

/// Bilinear sample of the `part` (left, top, width, height) of an image at
/// (u, v), in part pixels with (0, 0) the top left corner. `None` outside.
/// With `green_key`, pure green counts as transparent, nearest pixel.
fn sample(
    img: &Image,
    part: (u32, u32, u32, u32),
    u: f32,
    v: f32,
    green_key: bool,
) -> Option<[u8; 4]> {
    let (pl, pt, pw, ph) = part;
    let pw = pw.min(img.width.saturating_sub(pl));
    let ph = ph.min(img.height.saturating_sub(pt));
    if u < 0.0 || v < 0.0 || u >= pw as f32 || v >= ph as f32 {
        return None;
    }
    let at = |x: u32, y: u32| {
        let i = (((pt + y) * img.width + pl + x) * 4) as usize;
        [
            img.rgba[i],
            img.rgba[i + 1],
            img.rgba[i + 2],
            img.rgba[i + 3],
        ]
    };
    if green_key {
        let p = at(u as u32, v as u32);
        return (p != [0, 255, 0, p[3]]).then_some(p);
    }
    let fx = (u - 0.5).clamp(0.0, (pw - 1) as f32);
    let fy = (v - 0.5).clamp(0.0, (ph - 1) as f32);
    let (x0, y0) = (fx as u32, fy as u32);
    let (x1, y1) = ((x0 + 1).min(pw - 1), (y0 + 1).min(ph - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let mut acc = [0f32; 4];
    for (p, w) in [
        (at(x0, y0), (1.0 - tx) * (1.0 - ty)),
        (at(x1, y0), tx * (1.0 - ty)),
        (at(x0, y1), (1.0 - tx) * ty),
        (at(x1, y1), tx * ty),
    ] {
        let a = p[3] as f32 * w;
        for k in 0..3 {
            acc[k] += p[k] as f32 * a;
        }
        acc[3] += a;
    }
    if acc[3] <= 0.0 {
        return Some([0, 0, 0, 0]);
    }
    Some([
        (acc[0] / acc[3]).round() as u8,
        (acc[1] / acc[3]).round() as u8,
        (acc[2] / acc[3]).round() as u8,
        acc[3].round().min(255.0) as u8,
    ])
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for px in rgba.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Self::from_rgba(width, height, rgba)
    }

    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
            scale: 1.0,
            sharp: None,
        }
    }

    /// A black frame whose text is drawn at `scale` output pixels per game
    /// pixel (see [`Canvas::present`]).
    pub fn with_scale(width: u32, height: u32, scale: f32) -> Self {
        let mut c = Self::new(width, height);
        c.scale = if scale > 1.0 { scale } else { 1.0 };
        c
    }

    /// Output pixels per game pixel.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Size of [`Canvas::present`]'s frame.
    pub fn output_size(&self) -> (u32, u32) {
        (
            (self.width as f32 * self.scale).round() as u32,
            (self.height as f32 * self.scale).round() as u32,
        )
    }

    // ---- the output-size parts ---------------------------------------

    fn sharp_mut(&mut self) -> &mut Sharp {
        if self.sharp.is_none() {
            let (w, h) = self.output_size();
            let columns = self.width.div_ceil(TILE);
            let rows = self.height.div_ceil(TILE);
            self.sharp = Some(Box::new(Sharp {
                width: w,
                height: h,
                rgba: vec![0; (w * h * 4) as usize],
                tiles: vec![false; (columns * rows) as usize],
                columns,
            }));
        }
        self.sharp.as_mut().unwrap()
    }

    /// Output pixel rectangle (x0, y0, x1, y1) of a tile.
    fn tile_rect(&self, column: u32, row: u32) -> (u32, u32, u32, u32) {
        let (w, h) = self.output_size();
        let s = self.scale;
        let edge = |t: u32, max: u32| (((t * TILE) as f32 * s).round() as u32).min(max);
        (
            edge(column, w),
            edge(row, h),
            edge(column + 1, w),
            edge(row + 1, h),
        )
    }

    /// Bilinear sample of the game-size picture at game coordinates.
    fn sample_low(&self, gx: f32, gy: f32) -> [u8; 4] {
        let (w, h) = (self.width, self.height);
        let fx = (gx - 0.5).clamp(0.0, (w - 1) as f32);
        let fy = (gy - 0.5).clamp(0.0, (h - 1) as f32);
        let (x0, y0) = (fx as u32, fy as u32);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let at = |x: u32, y: u32, k: usize| self.rgba[((y * w + x) * 4) as usize + k] as f32;
        let mut out = [255u8; 4];
        for (k, o) in out.iter_mut().enumerate().take(3) {
            let top = at(x0, y0, k) + (at(x1, y0, k) - at(x0, y0, k)) * tx;
            let bottom = at(x0, y1, k) + (at(x1, y1, k) - at(x0, y1, k)) * tx;
            *o = (top + (bottom - top) * ty).round() as u8;
        }
        out
    }

    /// Copies the tiles under the game rectangle up to output size, so
    /// text can be drawn there.
    fn make_sharp(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        if self.scale <= 1.0 {
            return;
        }
        let tiles = self.tiles_in(x0, y0, x1, y1);
        let s = self.scale;
        self.sharp_mut();
        for (c, r) in tiles {
            let sharp = self.sharp.as_ref().unwrap();
            let i = (r * sharp.columns + c) as usize;
            if sharp.tiles[i] {
                continue;
            }
            let (ox0, oy0, ox1, oy1) = self.tile_rect(c, r);
            let mut pixels = Vec::with_capacity(((ox1 - ox0) * (oy1 - oy0)) as usize);
            for oy in oy0..oy1 {
                for ox in ox0..ox1 {
                    pixels.push(self.sample_low((ox as f32 + 0.5) / s, (oy as f32 + 0.5) / s));
                }
            }
            let sharp = self.sharp.as_mut().unwrap();
            let mut it = pixels.into_iter();
            for oy in oy0..oy1 {
                for ox in ox0..ox1 {
                    let d = ((oy * sharp.width + ox) * 4) as usize;
                    sharp.rgba[d..d + 4].copy_from_slice(&it.next().unwrap());
                }
            }
            sharp.tiles[i] = true;
        }
    }

    /// Tiles (column, row) under a game rectangle.
    fn tiles_in(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(u32, u32)> {
        let columns = self.width.div_ceil(TILE) as i64;
        let rows = self.height.div_ceil(TILE) as i64;
        let t = TILE as f32;
        let c0 = ((x0 / t).floor() as i64).max(0);
        let r0 = ((y0 / t).floor() as i64).max(0);
        let c1 = ((x1 / t).ceil() as i64).min(columns);
        let r1 = ((y1 / t).ceil() as i64).min(rows);
        let mut out = Vec::new();
        for r in r0..r1 {
            for c in c0..c1 {
                out.push((c as u32, r as u32));
            }
        }
        out
    }

    /// Draws into the sharp tiles under a game rectangle: `paint` gives
    /// the colour at a game position, or `None` to leave it.
    fn paint_sharp(
        &mut self,
        (x0, y0, x1, y1): (f32, f32, f32, f32),
        paint: impl Fn(f32, f32) -> Option<[u8; 4]>,
    ) {
        let Some(sharp) = &self.sharp else {
            return;
        };
        let s = self.scale;
        let held: Vec<(u32, u32)> = self
            .tiles_in(x0, y0, x1, y1)
            .into_iter()
            .filter(|&(c, r)| sharp.tiles[(r * sharp.columns + c) as usize])
            .collect();
        for (c, r) in held {
            let (tx0, ty0, tx1, ty1) = self.tile_rect(c, r);
            let ox0 = tx0.max((x0 * s).floor().max(0.0) as u32);
            let oy0 = ty0.max((y0 * s).floor().max(0.0) as u32);
            let ox1 = tx1.min((x1 * s).ceil().max(0.0) as u32);
            let oy1 = ty1.min((y1 * s).ceil().max(0.0) as u32);
            let sharp = self.sharp.as_mut().unwrap();
            for oy in oy0..oy1 {
                for ox in ox0..ox1 {
                    if let Some(px) = paint((ox as f32 + 0.5) / s, (oy as f32 + 0.5) / s) {
                        let d = ((oy * sharp.width + ox) * 4) as usize;
                        blend_into(&mut sharp.rgba[d..d + 4], px);
                    }
                }
            }
        }
    }

    /// The frame at output size: the picture scaled up, with the parts
    /// that hold text at full resolution.
    pub fn present(&self) -> Canvas {
        if self.scale <= 1.0 {
            return Canvas::from_rgba(self.width, self.height, self.rgba.clone());
        }
        let (w, h) = self.output_size();
        let s = self.scale;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for oy in 0..h {
            for ox in 0..w {
                let d = ((oy * w + ox) * 4) as usize;
                rgba[d..d + 4].copy_from_slice(
                    &self.sample_low((ox as f32 + 0.5) / s, (oy as f32 + 0.5) / s),
                );
            }
        }
        if let Some(sharp) = &self.sharp {
            for (i, _) in sharp.tiles.iter().enumerate().filter(|(_, &t)| t) {
                let (c, r) = (i as u32 % sharp.columns, i as u32 / sharp.columns);
                let (x0, y0, x1, y1) = self.tile_rect(c, r);
                for oy in y0..y1 {
                    let a = ((oy * w + x0) * 4) as usize;
                    let b = ((oy * w + x1) * 4) as usize;
                    rgba[a..b].copy_from_slice(&sharp.rgba[a..b]);
                }
            }
        }
        Canvas::from_rgba(w, h, rgba)
    }

    // ---- drawing -------------------------------------------------------

    fn blend_px(&mut self, x: i32, y: i32, src: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || src[3] == 0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        blend_into(&mut self.rgba[i..i + 4], src);
    }

    /// Draws an image at (x, y), 1:1. Pure green (0, 255, 0) counts as
    /// transparent when `green_key` is set, as in event difference pictures.
    pub fn draw(&mut self, img: &Image, x: i32, y: i32, green_key: bool) {
        self.draw_part(img, (0, 0, img.width, img.height), x, y, green_key, 255);
    }

    /// Draws the rectangle (left, top, width, height) of an image at (x, y),
    /// with its alpha scaled by `opacity` (255 = as stored).
    pub fn draw_part(
        &mut self,
        img: &Image,
        part: (u32, u32, u32, u32),
        x: i32,
        y: i32,
        green_key: bool,
        opacity: u8,
    ) {
        let (pl, pt, pw, ph) = part;
        let pw = pw.min(img.width.saturating_sub(pl));
        let ph = ph.min(img.height.saturating_sub(pt));
        for row in 0..ph {
            let dy = y + row as i32;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for col in 0..pw {
                let i = (((pt + row) * img.width + pl + col) * 4) as usize;
                let s = &img.rgba[i..i + 4];
                if green_key && s[0] == 0 && s[1] == 255 && s[2] == 0 {
                    continue;
                }
                let a = (s[3] as u32 * opacity as u32 + 127) / 255;
                self.blend_px(x + col as i32, dy, [s[0], s[1], s[2], a as u8]);
            }
        }
        let (fx, fy) = (x as f32, y as f32);
        self.paint_sharp((fx, fy, fx + pw as f32, fy + ph as f32), |gx, gy| {
            sample(img, (pl, pt, pw, ph), gx - fx, gy - fy, green_key).map(|mut p| {
                p[3] = ((p[3] as u32 * opacity as u32 + 127) / 255) as u8;
                p
            })
        });
    }

    /// Draws an image scaled by `scale` with its top left at (x, y),
    /// averaging the source pixels under each target pixel.
    pub fn draw_scaled(&mut self, img: &Image, x: f32, y: f32, scale: f32) {
        let dw = (img.width as f32 * scale).round() as i32;
        let dh = (img.height as f32 * scale).round() as i32;
        let (x0, y0) = (x.round() as i32, y.round() as i32);
        let inv = 1.0 / scale;
        let taps = (inv.ceil() as u32).clamp(1, 4);
        for ty in 0..dh {
            let sy = y0 + ty;
            if sy < 0 || sy >= self.height as i32 {
                continue;
            }
            for tx in 0..dw {
                let sx = x0 + tx;
                if sx < 0 || sx >= self.width as i32 {
                    continue;
                }
                let mut acc = [0u32; 4];
                let mut n = 0;
                for j in 0..taps {
                    for i in 0..taps {
                        let u = ((tx as f32 + (i as f32 + 0.5) / taps as f32) * inv) as u32;
                        let v = ((ty as f32 + (j as f32 + 0.5) / taps as f32) * inv) as u32;
                        if u >= img.width || v >= img.height {
                            continue;
                        }
                        let k = ((v * img.width + u) * 4) as usize;
                        let a = img.rgba[k + 3] as u32;
                        for (sum, &v) in acc.iter_mut().zip(&img.rgba[k..k + 3]) {
                            *sum += v as u32 * a;
                        }
                        acc[3] += a;
                        n += 1;
                    }
                }
                if n == 0 || acc[3] == 0 {
                    continue;
                }
                let px = [
                    (acc[0] / acc[3]) as u8,
                    (acc[1] / acc[3]) as u8,
                    (acc[2] / acc[3]) as u8,
                    (acc[3] / n) as u8,
                ];
                self.blend_px(sx, sy, px);
            }
        }
        let (fx, fy) = (x0 as f32, y0 as f32);
        let full = (0, 0, img.width, img.height);
        self.paint_sharp((fx, fy, fx + dw as f32, fy + dh as f32), |gx, gy| {
            sample(img, full, (gx - fx) * inv, (gy - fy) * inv, false)
        });
    }

    /// Draws an opaque picture enlarged by `scale` (> 1) with its top left
    /// at (x, y), with bilinear filtering.
    pub fn draw_zoomed(&mut self, img: &Image, x: f32, y: f32, scale: f32) {
        if img.width == 0 || img.height == 0 {
            return;
        }
        let (w, h) = (img.width as usize, img.height as usize);
        let px = |u: usize, v: usize| &img.rgba[(v * w + u) * 4..(v * w + u) * 4 + 4];
        for ty in 0..self.height as usize {
            let fy = ((ty as f32 + 0.5 - y) / scale - 0.5).clamp(0.0, (h - 1) as f32);
            if (ty as f32) < y || ty as f32 >= y + h as f32 * scale {
                continue;
            }
            let (v0, wy) = (fy as usize, fy.fract());
            let v1 = (v0 + 1).min(h - 1);
            for tx in 0..self.width as usize {
                if (tx as f32) < x || tx as f32 >= x + w as f32 * scale {
                    continue;
                }
                let fx = ((tx as f32 + 0.5 - x) / scale - 0.5).clamp(0.0, (w - 1) as f32);
                let (u0, wx) = (fx as usize, fx.fract());
                let u1 = (u0 + 1).min(w - 1);
                let (a, b, c, d) = (px(u0, v0), px(u1, v0), px(u0, v1), px(u1, v1));
                let mut out = [0u8; 4];
                for k in 0..4 {
                    let top = a[k] as f32 * (1.0 - wx) + b[k] as f32 * wx;
                    let bottom = c[k] as f32 * (1.0 - wx) + d[k] as f32 * wx;
                    out[k] = (top * (1.0 - wy) + bottom * wy + 0.5) as u8;
                }
                self.blend_px(tx as i32, ty as i32, out);
            }
        }
        let full = (0, 0, img.width, img.height);
        let (x1, y1) = (x + w as f32 * scale, y + h as f32 * scale);
        self.paint_sharp((x, y, x1, y1), |gx, gy| {
            sample(img, full, (gx - x) / scale, (gy - y) / scale, false)
        });
    }

    /// Moves the picture by (dx, dy), filling the uncovered edge with black.
    pub fn shift(&mut self, dx: i32, dy: i32) {
        if self.sharp.is_some() {
            // Rare (a shake over drawn text): keep the text sharp by
            // shifting the whole output-size frame.
            let mut big = self.present();
            let s = self.scale;
            big.shift(
                (dx as f32 * s).round() as i32,
                (dy as f32 * s).round() as i32,
            );
            self.shift_low(dx, dy);
            let sharp = self.sharp_mut();
            sharp.rgba = big.rgba;
            sharp.tiles.iter_mut().for_each(|t| *t = true);
            return;
        }
        self.shift_low(dx, dy);
    }

    fn shift_low(&mut self, dx: i32, dy: i32) {
        let (w, h) = (self.width as i32, self.height as i32);
        let src = std::mem::replace(&mut self.rgba, Canvas::new(self.width, self.height).rgba);
        for y in 0..h {
            let sy = y - dy;
            if sy < 0 || sy >= h {
                continue;
            }
            for x in 0..w {
                let sx = x - dx;
                if sx < 0 || sx >= w {
                    continue;
                }
                let (d, s) = (((y * w + x) * 4) as usize, ((sy * w + sx) * 4) as usize);
                self.rgba[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        }
    }

    /// Mixes two frames of the same size: `t` from 0 (all `from`) to 1
    /// (all `to`). With a rule mask (one byte per pixel), pixels change in
    /// the order of their mask value, with a soft edge.
    pub fn blend(from: &Canvas, to: &Canvas, t: f32, rule: Option<&[u8]>) -> Canvas {
        const EDGE: f32 = 32.0;
        let t = t.clamp(0.0, 1.0);
        let mut out = to.clone();
        out.scale = from.scale.max(to.scale);
        let n = (to.width * to.height) as usize;
        let rule = rule.filter(|r| r.len() >= n);
        let amount = |i: usize| match rule {
            Some(r) => ((t * (255.0 + EDGE) - r[i] as f32) / EDGE).clamp(0.0, 1.0),
            None => t,
        };
        for i in 0..n {
            let a = amount(i);
            if a >= 1.0 {
                continue;
            }
            for k in 0..3 {
                let (f, g) = (from.rgba[i * 4 + k] as f32, to.rgba[i * 4 + k] as f32);
                out.rgba[i * 4 + k] = (f + (g - f) * a).round() as u8;
            }
        }
        if from.sharp.is_none() && to.sharp.is_none() {
            return out;
        }
        // Text on either side: mix at output size where either has it.
        let mut from = from.clone();
        let mut to = to.clone();
        let (w, h) = (to.width as f32, to.height as f32);
        for side in [&mut from, &mut to] {
            side.scale = out.scale.max(side.scale);
        }
        let held = |c: &Canvas| c.sharp.as_ref().map(|s| s.tiles.clone());
        let tiles: Vec<bool> = match (held(&from), held(&to)) {
            (Some(a), Some(b)) => a.iter().zip(&b).map(|(x, y)| *x || *y).collect(),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => unreachable!(),
        };
        let columns = to.width.div_ceil(TILE);
        for (i, _) in tiles.iter().enumerate().filter(|(_, &x)| x) {
            let (c, r) = ((i as u32 % columns) as f32, (i as u32 / columns) as f32);
            let t = TILE as f32;
            let rect = (c * t, r * t, ((c + 1.0) * t).min(w), ((r + 1.0) * t).min(h));
            from.make_sharp(rect.0, rect.1, rect.2, rect.3);
            to.make_sharp(rect.0, rect.1, rect.2, rect.3);
            out.make_sharp(rect.0, rect.1, rect.2, rect.3);
        }
        let s = out.scale;
        let (fs, ts) = (from.sharp.take().unwrap(), to.sharp.take().unwrap());
        let os = out.sharp.as_mut().unwrap();
        for (i, _) in tiles.iter().enumerate().filter(|(_, &x)| x) {
            let (c, r) = (i as u32 % columns, i as u32 / columns);
            let (x0, y0, x1, y1) = to.tile_rect(c, r);
            for oy in y0..y1 {
                for ox in x0..x1 {
                    let gx = ((ox as f32 + 0.5) / s) as usize;
                    let gy = ((oy as f32 + 0.5) / s) as usize;
                    let a = amount(
                        gy.min(to.height as usize - 1) * to.width as usize
                            + gx.min(to.width as usize - 1),
                    );
                    let d = ((oy * os.width + ox) * 4) as usize;
                    for k in 0..3 {
                        let (f, g) = (fs.rgba[d + k] as f32, ts.rgba[d + k] as f32);
                        os.rgba[d + k] = (f + (g - f) * a).round() as u8;
                    }
                    os.rgba[d + 3] = 255;
                }
            }
        }
        out
    }

    /// Fills a rectangle with a colour (alpha blended).
    pub fn fill(&mut self, x: i32, y: i32, w: u32, h: u32, color: [u8; 4]) {
        for row in 0..h as i32 {
            for col in 0..w as i32 {
                self.blend_px(x + col, y + row, color);
            }
        }
        let (fx, fy) = (x as f32, y as f32);
        self.paint_sharp((fx, fy, fx + w as f32, fy + h as f32), |_, _| Some(color));
    }

    /// Blends a coverage mask (a glyph) in `color`, in game pixels.
    pub fn blend_mask(&mut self, x: i32, y: i32, w: usize, h: usize, mask: &[u8], color: [u8; 4]) {
        for row in 0..h {
            for col in 0..w {
                let cov = mask[row * w + col] as u32;
                if cov == 0 {
                    continue;
                }
                let a = (cov * color[3] as u32 / 255) as u8;
                self.blend_px(
                    x + col as i32,
                    y + row as i32,
                    [color[0], color[1], color[2], a],
                );
            }
        }
    }

    /// Blends a coverage mask (a glyph rasterized at output size) with its
    /// top left at output pixel (x, y). Only for a canvas with an output
    /// scale above 1 (see [`Canvas::scale`]).
    pub fn blend_mask_sharp(
        &mut self,
        x: i32,
        y: i32,
        w: usize,
        h: usize,
        mask: &[u8],
        color: [u8; 4],
    ) {
        let s = self.scale;
        let (gx0, gy0) = (x as f32 / s, y as f32 / s);
        self.make_sharp(gx0, gy0, gx0 + w as f32 / s, gy0 + h as f32 / s);
        let Some(sharp) = self.sharp.as_mut() else {
            return;
        };
        for row in 0..h {
            let oy = y + row as i32;
            if oy < 0 || oy >= sharp.height as i32 {
                continue;
            }
            for col in 0..w {
                let ox = x + col as i32;
                let cov = mask[row * w + col] as u32;
                if cov == 0 || ox < 0 || ox >= sharp.width as i32 {
                    continue;
                }
                let a = (cov * color[3] as u32 / 255) as u8;
                let d = ((oy as u32 * sharp.width + ox as u32) * 4) as usize;
                blend_into(&mut sharp.rgba[d..d + 4], [color[0], color[1], color[2], a]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_tiles_stay_sharp_and_ordered() {
        let mut c = Canvas::with_scale(32, 16, 2.0);
        c.fill(0, 0, 32, 16, [100, 100, 100, 255]);
        // A one-output-pixel glyph at output (3, 3): game (1.5, 1.5).
        c.blend_mask_sharp(3, 3, 1, 1, &[255], [255, 255, 255, 255]);
        let out = c.present();
        assert_eq!((out.width, out.height), (64, 32));
        let at = |o: &Canvas, x: u32, y: u32| o.rgba[((y * o.width + x) * 4) as usize];
        assert_eq!(at(&out, 3, 3), 255);
        assert_eq!(at(&out, 2, 3), 100);
        // Drawn over afterwards: covered in the output too.
        c.fill(0, 0, 4, 4, [0, 0, 0, 255]);
        assert_eq!(at(&c.present(), 3, 3), 0);
        // The right half has no text: scaled from the game-size picture.
        assert_eq!(at(&c.present(), 40, 3), 100);
    }

    #[test]
    fn no_scale_presents_as_is() {
        let c = Canvas::new(2, 2);
        assert_eq!(c.present().rgba, c.rgba);
    }
}
