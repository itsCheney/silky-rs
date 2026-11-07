//! Software renderer: composes a [`Scene`] into one RGBA frame.
//!
//! The headless runner writes these frames to PNG; the desktop player will
//! draw the same layers with the GPU. Layout values marked "provisional"
//! are not confirmed against the original engine yet.

use std::collections::HashMap;
use std::sync::Arc;

use silky_format::akb::Image;

use crate::assets::{localized_name, Assets, Language};
use crate::kimaten2;
use crate::scene::Scene;
use crate::text::TextRenderer;
use crate::{Result, SCREEN_HEIGHT, SCREEN_WIDTH};
use silky_format::game::Title;

pub use crate::canvas::Canvas;

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

/// Where the message text goes.
#[derive(Debug, Clone, Copy)]
pub struct TextLayout {
    pub left: f32,
    /// Baseline of the first line.
    pub baseline: f32,
    pub width: f32,
    pub line: f32,
    pub size: f32,
    /// Pen advance of CJK characters, as a multiple of the size.
    pub wide_advance: f32,
}

impl TextLayout {
    pub fn for_title(title: Title) -> Self {
        match title {
            Title::Fuyukuru => Self {
                left: TEXT_LEFT,
                baseline: TEXT_BASELINE,
                width: TEXT_WIDTH,
                line: TEXT_LINE,
                size: TEXT_SIZE,
                wide_advance: TEXT_WIDE_ADVANCE,
            },
            // Measured on PC screenshots of the Chinese patch: 24 pixel
            // glyphs every 28 pixels from x = 180, lines 36 pixels apart.
            // The line length is not measured yet.
            Title::Kimaten2 => Self {
                left: kimaten2::window::TEXT_LEFT,
                baseline: kimaten2::window::TEXT_BASELINE,
                width: 30.0 * 28.0,
                line: 36.0,
                size: 24.0,
                wide_advance: 28.0 / 24.0,
            },
        }
    }
}

pub struct Renderer {
    text: Option<TextRenderer>,
    language: Language,
    characters: HashMap<(String, Option<String>), Arc<Image>>,
    /// Message window colour (RGB, 0..1) and opacity (settings).
    pub window: ([f32; 3], f32),
    /// Output pixels per game pixel, for text (see [`Canvas::present`]).
    pub scale: f32,
    /// Game whose message window is drawn.
    title: Title,
    pub text_layout: TextLayout,
    /// Name plate rows by speaker name (きまぐれテンプテーション2), for the
    /// language they were made for.
    speaker_rows: Option<(Language, HashMap<String, usize>)>,
    /// E-mote pictures (きまぐれテンプテーション2): the standing models by
    /// slot and the companion under [`COMPANION`].
    #[cfg(feature = "emote")]
    emote: std::collections::BTreeMap<i32, crate::emote::Stage>,
    /// Files that failed to open, not tried again.
    #[cfg(feature = "emote")]
    emote_failed: std::collections::HashSet<[String; 2]>,
    /// Where E-mote models are drawn, if not in software.
    #[cfg(feature = "emote")]
    emote_gpu: Option<crate::emote::Gpu>,
    /// Snow while the scene has it.
    snow: Option<crate::snow::Snow>,
}

impl Renderer {
    /// `text` is `None` to leave out message text (no font available).
    pub fn new(text: Option<TextRenderer>, language: Language) -> Self {
        let mut r = Self {
            text,
            language,
            characters: HashMap::new(),
            window: ([0.0; 3], WINDOW_OPACITY as f32 / 255.0),
            scale: 1.0,
            title: Title::Fuyukuru,
            text_layout: TextLayout::for_title(Title::Fuyukuru),
            speaker_rows: None,
            #[cfg(feature = "emote")]
            emote: Default::default(),
            #[cfg(feature = "emote")]
            emote_failed: Default::default(),
            #[cfg(feature = "emote")]
            emote_gpu: None,
            snow: None,
        };
        r.set_language(language);
        r
    }

    /// Draws the message window of this game.
    pub fn set_title(&mut self, title: Title) {
        self.title = title;
        self.text_layout = TextLayout::for_title(title);
        self.set_language(self.language);
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
                let mut canvas = Canvas::from_rgba(body.width, body.height, body.rgba.clone());
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

    pub fn text(&self) -> Option<&TextRenderer> {
        self.text.as_ref()
    }

    /// Switches the language of nameplates and text spacing.
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        let wide = self.text_layout.wide_advance;
        if let Some(t) = &mut self.text {
            t.set_wide_advance((language != Language::English).then_some(wide));
        }
    }

    /// Name plate row of a speaker given by name.
    fn speaker_row(&mut self, assets: &Assets, name: &str) -> Option<usize> {
        if self
            .speaker_rows
            .as_ref()
            .is_none_or(|(l, _)| *l != self.language)
        {
            let rows = kimaten2::speaker_rows(assets.game());
            self.speaker_rows = Some((self.language, rows));
        }
        self.speaker_rows.as_ref()?.1.get(name).copied()
    }

    /// Draws the message text of the scene, only its first `limit`
    /// characters if given (the layout stays that of the whole text).
    pub fn draw_text(&self, canvas: &mut Canvas, scene: &Scene, limit: Option<usize>) {
        if scene.cover.is_some() {
            return;
        }
        self.draw_text_at(canvas, scene, limit, scene.shake);
    }

    /// The message window of きまぐれテンプテーション2: the band of
    /// `message_p.akb` and the speaker's row of `name_p.akb` (a name not in
    /// its list is written as text).
    fn kimaten2_window(
        &mut self,
        canvas: &mut Canvas,
        scene: &Scene,
        assets: &mut Assets,
    ) -> Result<()> {
        use kimaten2::window;
        let parts = assets.image("message_p.akb")?;
        let (color, opacity) = self.window;
        let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
        canvas.draw_part(&parts, (0, 0, 1280, 300), 0, window::TOP, false, alpha);
        if color != [0.0; 3] {
            let rgb = color.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
            canvas.fill(
                0,
                window::TOP + 30,
                1280,
                215,
                [rgb[0], rgb[1], rgb[2], alpha / 2],
            );
        }
        if let Some(name) = &scene.speaker_name {
            let (x, y) = window::NAME;
            let (w, h) = window::NAME_SIZE;
            match self.speaker_row(assets, name) {
                Some(row) => {
                    let sheet = assets.image("name_p.akb")?;
                    canvas.draw_part(&sheet, (0, h * row as u32, w, h), x, y, false, 255);
                }
                None => {
                    if let Some(tr) = &self.text {
                        let size = self.text_layout.size * 0.9;
                        tr.draw_line(
                            canvas,
                            name,
                            x as f32 + 24.0,
                            y as f32 + h as f32 - 10.0,
                            size,
                            [255, 255, 255, 255],
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_text_at(
        &self,
        canvas: &mut Canvas,
        scene: &Scene,
        limit: Option<usize>,
        shift: (i32, i32),
    ) {
        let (Some(text), Some(tr)) = (&scene.text, &self.text) else {
            return;
        };
        let l = self.text_layout;
        let mut left = limit.unwrap_or(usize::MAX);
        for (i, line) in tr.wrap(text, l.size, l.width).iter().enumerate() {
            if left == 0 {
                break;
            }
            let shown: String = line.chars().take(left).collect();
            left = left.saturating_sub(line.chars().count());
            let y = l.baseline + l.line * i as f32 + shift.1 as f32;
            let x = l.left + shift.0 as f32;
            tr.draw_line(canvas, &shown, x, y, l.size, [255, 255, 255, 255]);
        }
    }

    /// Renders the scene with the message window, without engine-drawn
    /// buttons (see [`crate::ui`]).
    /// Brings the E-mote pictures up to date with the scene and returns
    /// the visible ones, back to front, with the screen position of their
    /// top left corners.
    #[cfg(feature = "emote")]
    fn emote_images(&mut self, scene: &Scene, assets: &Assets) -> Vec<(Arc<Image>, f32, f32)> {
        let wanted = emote_order(scene);
        self.emote
            .retain(|k, s| wanted.iter().any(|(w, e)| w == k && s.files() == &e.files));
        for (key, emote) in &wanted {
            if self.emote.contains_key(key) || self.emote_failed.contains(&emote.files) {
                continue;
            }
            match crate::emote::Stage::open(emote, |name| assets.data(name).ok()) {
                Ok(stage) => {
                    self.emote.insert(*key, stage);
                }
                Err(e) => {
                    eprintln!("E-mote {}: {e}", emote.files[0]);
                    self.emote_failed.insert(emote.files.clone());
                }
            }
        }
        #[cfg(feature = "gpu")]
        if let Some(gpu) = &mut self.emote_gpu {
            gpu.retain(&self.emote.values().flat_map(|s| s.model_ids()).collect());
        }
        let mut out = Vec::new();
        for (key, emote) in wanted {
            let Some(stage) = self.emote.get_mut(&key) else {
                continue;
            };
            stage.apply(emote);
            if emote.visible {
                out.extend(stage.image(self.emote_gpu.as_mut()));
            }
        }
        out
    }

    /// Draws E-mote models on `gpu` from now on.
    /// The E-mote picture of standing slot `slot`, once drawn.
    #[cfg(feature = "emote")]
    pub fn emote_stage(&mut self, slot: i32) -> Option<&mut crate::emote::Stage> {
        self.emote.get_mut(&slot)
    }

    #[cfg(feature = "emote")]
    pub fn set_emote_gpu(&mut self, gpu: crate::emote::Gpu) {
        self.emote_gpu = Some(gpu);
        self.emote.values_mut().for_each(|s| s.redraw());
    }

    /// Runs the snow for `ms`; true when it moved.
    pub fn progress_snow(&mut self, scene: &Scene, ms: u64) -> bool {
        if !scene.snow {
            return self.snow.take().is_some();
        }
        self.snow
            .get_or_insert_with(Default::default)
            .progress(ms as f32)
    }

    /// Runs the E-mote animation for `ms`; true when a picture changed.
    /// `talking` is the mouth opening while a voice plays.
    pub fn progress_emote(&mut self, scene: &Scene, ms: u64, talking: Option<f32>) -> bool {
        #[cfg(feature = "emote")]
        {
            // In software about 15 frames a second, on a GPU 30.
            let interval = if self.emote_gpu.is_some() { 33.0 } else { 66.0 };
            let mut changed = false;
            for (key, emote) in emote_order(scene) {
                if let Some(stage) = self.emote.get_mut(&key) {
                    if emote.visible {
                        changed |= stage.progress(ms as f32, talking, interval);
                    }
                }
            }
            changed
        }
        #[cfg(not(feature = "emote"))]
        {
            let _ = (scene, ms, talking);
            false
        }
    }

    pub fn render(&mut self, scene: &Scene, assets: &mut Assets) -> Result<Canvas> {
        self.render_scene(scene, assets, true)
    }

    /// Renders the scene; `window` is false while the player hides the
    /// message window.
    pub fn render_scene(
        &mut self,
        scene: &Scene,
        assets: &mut Assets,
        window: bool,
    ) -> Result<Canvas> {
        self.render_parts(scene, assets, window, true)
    }

    /// Renders the scene; with `text` false the message window is drawn
    /// empty and [`Renderer::draw_text`] puts the text on later (text
    /// that appears letter by letter).
    pub fn render_parts(
        &mut self,
        scene: &Scene,
        assets: &mut Assets,
        window: bool,
        text: bool,
    ) -> Result<Canvas> {
        let mut canvas = Canvas::with_scale(SCREEN_WIDTH, SCREEN_HEIGHT, self.scale);
        if let Some(bg) = &scene.background {
            let img = assets.image(bg)?;
            let x = (SCREEN_WIDTH as i32 - img.width as i32) / 2 + scene.background_offset.0;
            let y = (SCREEN_HEIGHT as i32 - img.height as i32) / 2 + scene.background_offset.1;
            match scene.background_zoom {
                0 | 100 => canvas.draw(&img, x, y, false),
                z if z > 100 => canvas.draw_zoomed(&img, x as f32, y as f32, z as f32 / 100.0),
                z => canvas.draw_scaled(&img, x as f32, y as f32, z as f32 / 100.0),
            }
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
            let jolt = scene.jolts.get(slot).copied().unwrap_or_default();
            let x = SCREEN_WIDTH as f32 / 2.0 + slot_layout(*slot).0 - img.width as f32 * s / 2.0
                + (c.offset.0 + jolt.0) as f32;
            let y = 300.0 - 700.0 * s + (c.offset.1 + jolt.1) as f32;
            canvas.draw_scaled(&img, x, y, s);
        }

        #[cfg(feature = "emote")]
        for (img, x, y) in self.emote_images(scene, assets) {
            canvas.draw(&img, x.round() as i32, y.round() as i32, false);
        }

        for name in scene.layers.values() {
            let img = assets.image(name)?;
            canvas.draw(&img, 0, 0, false);
        }
        if let Some((c, a)) = scene.dim {
            let rgba = [(c >> 16) as u8, (c >> 8) as u8, c as u8, a];
            canvas.fill(0, 0, canvas.width, canvas.height, rgba);
        }
        if scene.snow {
            self.snow
                .get_or_insert_with(Default::default)
                .draw(&mut canvas, assets);
        }

        if scene.text.is_some() && window && self.title == Title::Kimaten2 {
            self.kimaten2_window(&mut canvas, scene, assets)?;
            if text {
                self.draw_text_at(&mut canvas, scene, None, (0, 0));
            }
        } else if scene.text.is_some() && window {
            let parts = assets.image("message_p.akb")?;
            let (color, opacity) = self.window;
            let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
            for part in [WINDOW_BAND, WINDOW_TEXTURE] {
                canvas.draw_part(&parts, part, 0, WINDOW_TOP, false, alpha);
            }
            if color != [0.0; 3] {
                let rgb = color.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
                let (_, _, w, h) = WINDOW_BAND;
                canvas.fill(0, WINDOW_TOP, w, h, [rgb[0], rgb[1], rgb[2], alpha / 2]);
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
            if text {
                // Shaken with the rest below.
                self.draw_text_at(&mut canvas, scene, None, (0, 0));
            }
        }
        if let Some(c) = scene.cover {
            let rgb = [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255];
            canvas.fill(0, 0, canvas.width, canvas.height, rgb);
        }
        if scene.shake != (0, 0) {
            canvas.shift(scene.shake.0, scene.shake.1);
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

/// Key of the companion E-mote picture among the slots.
#[cfg(feature = "emote")]
const COMPANION: i32 = i32::MIN;

/// The scene's E-mote pictures back to front: standing models from the
/// last slot to slot 0 (0x4577a0 gives slot 0 the highest order), then the
/// companion.
#[cfg(feature = "emote")]
fn emote_order(scene: &Scene) -> Vec<(i32, &crate::scene::Emote)> {
    scene
        .emotes
        .iter()
        .rev()
        .map(|(k, e)| (*k, e))
        .chain(scene.emote.iter().map(|e| (COMPANION, e)))
        .collect()
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
    fn blend_with_rule() {
        let from = Canvas::new(2, 1);
        let mut to = Canvas::new(2, 1);
        to.rgba = vec![200, 200, 200, 255, 200, 200, 200, 255];
        let half = Canvas::blend(&from, &to, 0.5, None);
        assert_eq!(half.rgba[0], 100);
        // Pixel 0 changes first (mask 0), pixel 1 last (mask 255).
        let r = Canvas::blend(&from, &to, 0.5, Some(&[0, 255]));
        assert_eq!((r.rgba[0], r.rgba[4]), (200, 0));
        let done = Canvas::blend(&from, &to, 1.0, Some(&[0, 255]));
        assert_eq!(done.rgba, to.rgba);
    }

    #[test]
    fn zoom_keeps_corners() {
        let mut img = solid(2, 1, [0, 0, 0, 255]);
        img.rgba[4..8].copy_from_slice(&[200, 200, 200, 255]);
        let mut c = Canvas::new(4, 1);
        c.draw_zoomed(&img, 0.0, 0.0, 2.0);
        assert_eq!((c.rgba[0], c.rgba[12]), (0, 200));
        assert!(c.rgba[4] > 0 && c.rgba[4] < 200);
    }

    #[test]
    fn shift_moves_picture() {
        let mut c = Canvas::new(2, 1);
        c.rgba[0] = 9;
        c.shift(1, 0);
        assert_eq!((c.rgba[0], c.rgba[4]), (0, 9));
    }

    #[test]
    fn face_over_body() {
        let mut body = Canvas::from_rgba(1, 1, vec![10, 10, 10, 255]);
        overlay_rgba(&mut body, &solid(1, 1, [250, 0, 0, 255]));
        assert_eq!(body.rgba, [250, 0, 0, 255]);
    }
}
