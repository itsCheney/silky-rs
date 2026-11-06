//! Software renderer: composes a [`Scene`] into one RGBA frame.
//!
//! The headless runner writes these frames to PNG; the desktop player will
//! draw the same layers with the GPU. Layout values marked "provisional"
//! are not confirmed against the original engine yet.

use std::collections::HashMap;
use std::sync::Arc;

use silky_format::akb::Image;

use crate::assets::{localized_name, Assets, Language};
use crate::scene::Scene;
use crate::text::TextRenderer;
use crate::{Result, SCREEN_HEIGHT, SCREEN_WIDTH};

/// An RGBA8 frame.
#[derive(Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for px in rgba.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    fn blend_px(&mut self, x: i32, y: i32, src: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || src[3] == 0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let d = &mut self.rgba[i..i + 4];
        let a = src[3] as u32;
        for k in 0..3 {
            d[k] = ((src[k] as u32 * a + d[k] as u32 * (255 - a) + 127) / 255) as u8;
        }
        d[3] = 255;
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
        for row in 0..ph.min(img.height.saturating_sub(pt)) {
            let dy = y + row as i32;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for col in 0..pw.min(img.width.saturating_sub(pl)) {
                let i = (((pt + row) * img.width + pl + col) * 4) as usize;
                let s = &img.rgba[i..i + 4];
                if green_key && s[0] == 0 && s[1] == 255 && s[2] == 0 {
                    continue;
                }
                let a = (s[3] as u32 * opacity as u32 + 127) / 255;
                self.blend_px(x + col as i32, dy, [s[0], s[1], s[2], a as u8]);
            }
        }
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
    }

    /// Fills a rectangle with a colour (alpha blended).
    pub fn fill(&mut self, x: i32, y: i32, w: u32, h: u32, color: [u8; 4]) {
        for row in 0..h as i32 {
            for col in 0..w as i32 {
                self.blend_px(x + col, y + row, color);
            }
        }
    }

    /// Blends a coverage mask (a glyph) in `color`.
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
}

/// Horizontal offset of a character slot from the screen centre, and its
/// stacking order (higher is drawn later), from the engine's slot table.
fn slot_layout(slot: i32) -> (f32, i32) {
    match slot {
        0 => (0.0, 3),
        1 => (-300.0, 2),
        2 => (300.0, 1),
        _ => (300.0, 0),
    }
}

/// Message window, measured on screenshots of the original (Windows,
/// Chinese text). `message_p.akb` holds a dark band (rows 0..220) and a
/// paper texture (rows 220..440); both are drawn at the window position
/// at half opacity.
const WINDOW_BAND: (u32, u32, u32, u32) = (0, 0, 1280, 220);
const WINDOW_TEXTURE: (u32, u32, u32, u32) = (0, 220, 1280, 220);
const WINDOW_TOP: i32 = 500;
const WINDOW_OPACITY: u8 = 128;
/// Nameplate: row `n` of `nameplate_<lang>.akb` is a 376 x 56 plate.
const NAMEPLATE_SIZE: (u32, u32) = (376, 56);
const NAMEPLATE_POS: (i32, i32) = (264, 500);
/// Window portrait: body (with face) scaled to 40 % with its top left here.
const PORTRAIT_SCALE: f32 = 0.4;
const PORTRAIT_POS: (f32, f32) = (-288.0, 320.0);
/// Text: left edge and first baseline measured; Chinese characters sit on
/// a fixed 26.5 px pitch at a 21 px size. Width and line pitch are
/// provisional (no two-line sample yet).
const TEXT_LEFT: f32 = 282.0;
const TEXT_BASELINE: f32 = 578.0;
const TEXT_WIDTH: f32 = 800.0;
const TEXT_LINE: f32 = 34.0;
const TEXT_SIZE: f32 = 21.0;
const TEXT_WIDE_ADVANCE: f32 = 26.5 / TEXT_SIZE;

pub struct Renderer {
    text: Option<TextRenderer>,
    language: Language,
    characters: HashMap<(String, Option<String>), Arc<Image>>,
}

impl Renderer {
    /// `text` is `None` to leave out message text (no font available).
    pub fn new(text: Option<TextRenderer>, language: Language) -> Self {
        let text = text.map(|mut t| {
            if language != Language::English {
                t.wide_advance = Some(TEXT_WIDE_ADVANCE);
            }
            t
        });
        Self {
            text,
            language,
            characters: HashMap::new(),
        }
    }

    /// Body with its expression drawn on, cached.
    fn character(
        &mut self,
        assets: &mut Assets,
        body: &str,
        face: Option<&str>,
    ) -> Result<Arc<Image>> {
        let key = (
            body.to_ascii_lowercase(),
            face.map(|f| f.to_ascii_lowercase()),
        );
        if let Some(img) = self.characters.get(&key) {
            return Ok(img.clone());
        }
        let body = assets.image(body)?;
        let img = match face {
            Some(face) => {
                let face = assets.image(face)?;
                let mut canvas = Canvas {
                    width: body.width,
                    height: body.height,
                    rgba: body.rgba.clone(),
                };
                overlay_rgba(&mut canvas, &face);
                Arc::new(Image {
                    width: canvas.width,
                    height: canvas.height,
                    rgba: canvas.rgba,
                })
            }
            None => body,
        };
        if self.characters.len() > 32 {
            self.characters.clear();
        }
        self.characters.insert(key, img.clone());
        Ok(img)
    }

    pub fn render(&mut self, scene: &Scene, assets: &mut Assets) -> Result<Canvas> {
        let mut canvas = Canvas::new(SCREEN_WIDTH, SCREEN_HEIGHT);
        if let Some(bg) = &scene.background {
            let img = assets.image(bg)?;
            let x = (SCREEN_WIDTH as i32 - img.width as i32) / 2;
            let y = (SCREEN_HEIGHT as i32 - img.height as i32) / 2;
            canvas.draw(&img, x, y, false);
        }
        for name in scene.overlays.values() {
            let img = assets.image(name)?;
            canvas.draw(&img, 0, 0, true);
        }

        // Characters: canvas scaled by size percent, centred on the slot's
        // position; the top edge follows the engine's formula
        // 300 - 700 * scale, which keeps faces in view at larger sizes.
        let mut chars: Vec<_> = scene.characters.iter().collect();
        chars.sort_by_key(|(slot, _)| slot_layout(**slot).1);
        for (slot, c) in chars {
            let img = self.character(assets, &c.body, c.face.as_deref())?;
            let s = c.scale as f32 / 100.0;
            let x = SCREEN_WIDTH as f32 / 2.0 + slot_layout(*slot).0 - img.width as f32 * s / 2.0;
            let y = 300.0 - 700.0 * s;
            canvas.draw_scaled(&img, x, y, s);
        }

        for name in scene.layers.values() {
            let img = assets.image(name)?;
            canvas.draw(&img, 0, 0, false);
        }

        if let Some(text) = &scene.text {
            let parts = assets.image("message_p.akb")?;
            for part in [WINDOW_BAND, WINDOW_TEXTURE] {
                canvas.draw_part(&parts, part, 0, WINDOW_TOP, false, WINDOW_OPACITY);
            }
            if let (true, Some((body, face))) = (scene.portrait_visible, &scene.portrait) {
                let img = self.character(assets, body, face.as_deref())?;
                let (x, y) = PORTRAIT_POS;
                canvas.draw_scaled(&img, x, y, PORTRAIT_SCALE);
            }
            if let Some(row) = scene.speaker {
                let sheet = assets.image(&localized_name("nameplate.akb", self.language))?;
                let (w, h) = NAMEPLATE_SIZE;
                let (x, y) = NAMEPLATE_POS;
                canvas.draw_part(&sheet, (0, h * row as u32, w, h), x, y, false, 255);
            }
            if let Some(tr) = &self.text {
                for (i, line) in tr.wrap(text, TEXT_SIZE, TEXT_WIDTH).iter().enumerate() {
                    let y = TEXT_BASELINE + TEXT_LINE * i as f32;
                    tr.draw_line(
                        &mut canvas,
                        line,
                        TEXT_LEFT,
                        y,
                        TEXT_SIZE,
                        [255, 255, 255, 255],
                    );
                }
            }
        }
        Ok(canvas)
    }
}

/// Alpha-composites `top` over `canvas` (same size).
fn overlay_rgba(canvas: &mut Canvas, top: &Image) {
    let n = (canvas.width.min(top.width) * canvas.height.min(top.height)) as usize;
    if top.width != canvas.width {
        return;
    }
    for i in 0..n {
        let s = &top.rgba[i * 4..i * 4 + 4];
        let a = s[3] as u32;
        if a == 0 {
            continue;
        }
        let d = &mut canvas.rgba[i * 4..i * 4 + 4];
        let da = d[3] as u32;
        let out_a = a + da * (255 - a) / 255;
        for k in 0..3 {
            let v = (s[k] as u32 * a + d[k] as u32 * da * (255 - a) / 255) / out_a.max(1);
            d[k] = v as u8;
        }
        d[3] = out_a as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Image {
        Image {
            width: w,
            height: h,
            rgba: px.repeat((w * h) as usize),
        }
    }

    #[test]
    fn green_key_and_alpha() {
        let mut c = Canvas::new(4, 1);
        c.draw(&solid(2, 1, [0, 255, 0, 255]), 0, 0, true);
        assert_eq!(&c.rgba[..4], &[0, 0, 0, 255]);
        c.draw(&solid(2, 1, [200, 100, 50, 128]), 2, 0, false);
        assert_eq!(&c.rgba[8..12], &[100, 50, 25, 255]);
    }

    #[test]
    fn scaled_draw_averages() {
        let mut img = solid(2, 2, [255, 255, 255, 255]);
        img.rgba[0..4].copy_from_slice(&[0, 0, 0, 255]);
        let mut c = Canvas::new(1, 1);
        c.draw_scaled(&img, 0.0, 0.0, 0.5);
        assert_eq!(c.rgba[0], 191);
    }

    #[test]
    fn face_over_body() {
        let mut body = Canvas {
            width: 1,
            height: 1,
            rgba: vec![10, 10, 10, 255],
        };
        overlay_rgba(&mut body, &solid(1, 1, [250, 0, 0, 255]));
        assert_eq!(body.rgba, [250, 0, 0, 255]);
    }
}
