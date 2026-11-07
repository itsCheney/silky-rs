//! E-mote characters. Eluna (vendored, MPL-2.0) reads the models and runs
//! their timelines and physics; [`draws`] turns its scene into triangles and
//! [`raster`] draws them in software.
//!
//! きまぐれテンプテーション2 shows Anneliese's standing pictures this way
//! (`Game.10`): two models of one costume, `_h0007` with faces 0 to 7 and
//! `_h0813` with faces 8 to 13. Poses (main timelines) play on both; the
//! face picks the model that is drawn.

mod draws;
#[cfg(feature = "gpu")]
pub mod gpu;
mod raster;

/// The GPU E-mote models are drawn on (feature `gpu`).
#[cfg(feature = "gpu")]
pub use gpu::GpuRaster as Gpu;
/// Without the `gpu` feature there is none.
#[cfg(not(feature = "gpu"))]
pub enum Gpu {}

use std::collections::HashMap;
use std::path::Path;

use eluna::{
    bruteforce_emote_key, collect_emote_runtime_pipeline, collect_emote_timelines,
    collect_emote_variables, ElunaPlayer, EmoteModelSchema, EmotePlayerControl, EmoteStaticScene,
    EmoteTextureSource, PsbBruteforceOptions, PsbDecryptionKey, PsbFile, PsbNormalizeOptions,
    PsbValue, TimelinePlayMode,
};
use silky_format::akb::Image;

pub use raster::Placement;
use raster::{Target, Texture};

/// Keys of the games' E-mote files, tried before a key search.
const KNOWN_KEYS: &[u32] = &[
    // きまぐれテンプテーション2
    0x03C4_006B,
];

fn normalize_options(key: u32) -> PsbNormalizeOptions {
    PsbNormalizeOptions {
        decrypt_key: Some(PsbDecryptionKey::emote_key(key)),
        decode_mdf: true,
        decode_lz4: true,
    }
}

/// The key of an E-mote file: a known key, the one cached in `cache`, or
/// one found by searching all 2^32 keys (seconds to a minute), which is
/// then written to `cache`.
pub fn find_key(sample: &[u8], cache: Option<&Path>) -> Option<u32> {
    let opens = |key: u32| PsbFile::parse_normalized(sample, &normalize_options(key)).is_ok();
    let cached = cache
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());
    if let Some(key) = cached
        .into_iter()
        .chain(KNOWN_KEYS.iter().copied())
        .find(|&k| opens(k))
    {
        return Some(key);
    }
    let found = bruteforce_emote_key(sample, PsbBruteforceOptions::default()).ok()??;
    if let Some(p) = cache {
        let _ = std::fs::write(p, format!("0x{:08X}\n", found.key));
    }
    Some(found.key)
}

/// One E-mote model with its player.
pub struct Model {
    data: Vec<u8>,
    psb: PsbFile,
    schema: EmoteModelSchema,
    motion: Option<String>,
    player: ElunaPlayer,
    /// Main and difference timelines in the file's order, which is the
    /// order the game numbers them in.
    main: Vec<String>,
    diff: Vec<String>,
    /// Textures by resource index, reduced by `1 << level`.
    textures: HashMap<u32, Texture>,
    /// Names the model's textures on a GPU.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    id: u64,
}

static NEXT_MODEL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Model {
    /// Opens a model; textures are kept halved `level` times.
    pub fn open(file: &[u8], key: u32, level: u32) -> Result<Self, String> {
        let (data, psb) =
            PsbFile::parse_normalized(file, &normalize_options(key)).map_err(|e| e.to_string())?;
        let schema = EmoteModelSchema::from_psb(&psb).map_err(|e| e.to_string())?;
        let motion = psb
            .root
            .field("metadata")
            .and_then(|m| m.field("base"))
            .and_then(|b| b.field_str("motion"))
            .filter(|m| !m.is_empty())
            .map(str::to_string)
            .or_else(|| schema.default_motion_name(&psb).ok().flatten());
        let variables = collect_emote_variables(&psb);
        let initial = variables
            .iter()
            .map(|v| (v.name.clone(), v.default_value))
            .collect();
        let scene = match motion.as_deref() {
            Some(m) => schema
                .build_motion_scene_at_with_resources_and_variables(&psb, &data, m, 0.0, &initial),
            None => schema.build_static_scene(&psb),
        }
        .map_err(|e| e.to_string())?;
        let player = ElunaPlayer::from_scene_variables_timelines_runtime(
            scene,
            variables,
            collect_emote_timelines(&psb),
            collect_emote_runtime_pipeline(&psb),
        );
        let (mut main, mut diff) = (Vec::new(), Vec::new());
        if let Some(list) = psb
            .root
            .field("metadata")
            .and_then(|m| m.field("timelineControl"))
            .and_then(PsbValue::as_list)
        {
            for item in list {
                let Some(label) = item.field_str("label") else {
                    continue;
                };
                let is_diff = item
                    .field("diff")
                    .and_then(PsbValue::as_i64)
                    .is_some_and(|d| d != 0);
                if is_diff { &mut diff } else { &mut main }.push(label.to_string());
            }
        }
        let mut textures = HashMap::new();
        for source in schema.textures.values() {
            let Some(bytes) = psb.resource_bytes(&data, source.resource_index as usize) else {
                continue;
            };
            if let Some(mut t) = decode_texture(source, bytes) {
                for _ in 0..level {
                    t = t.half();
                }
                textures.insert(source.resource_index, t);
            }
        }
        let mut model = Self {
            data,
            psb,
            schema,
            motion,
            player,
            main,
            diff,
            textures,
            id: NEXT_MODEL.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        model.rebuild(0.0)?;
        Ok(model)
    }

    /// Builds the scene at the player's time; with `physics` ticks, runs
    /// the physics and builds it again.
    fn rebuild(&mut self, physics: f32) -> Result<(), String> {
        let Some(motion) = self.motion.as_deref() else {
            return Ok(());
        };
        let (schema, psb, data) = (&self.schema, &self.psb, &self.data);
        let build = |player: &ElunaPlayer, previous: &EmoteStaticScene| {
            schema
                .build_motion_scene_at_with_resources_variables_and_previous_scene(
                    psb,
                    data,
                    motion,
                    player.elapsed_ticks(),
                    &player.evaluated_variable_values(),
                    previous,
                )
                .map_err(|e| e.to_string())
        };
        let scene = build(&self.player, self.player.scene())?;
        let previous = self.player.swap_scene(scene);
        if physics > 0.0 && self.player.is_physics_enabled() {
            self.player.evaluate_physics_for_current_scene(physics);
            let scene = build(&self.player, &previous)?;
            self.player.replace_scene(scene);
        }
        Ok(())
    }

    pub fn progress(&mut self, ticks: f32) -> Result<(), String> {
        self.player.progress_ticks_without_physics(ticks);
        self.rebuild(ticks)
    }

    /// Plays main timeline `index`, fading the one before out over 12
    /// ticks, as the game does.
    fn play_main(&mut self, index: usize, previous: Option<usize>) {
        if let Some(old) = previous.and_then(|i| self.main.get(i)) {
            if previous != Some(index) {
                self.player.fade_out_timeline(&old.clone(), 12.0, 1.0);
            }
        }
        if let Some(name) = self.main.get(index).cloned() {
            self.player
                .play_timeline(&name, TimelinePlayMode::from_flags(0));
        }
    }

    /// Plays difference timeline `index` in parallel, stopping `previous`.
    fn play_diff(&mut self, index: usize, previous: Option<usize>) {
        if let Some(old) = previous.and_then(|i| self.diff.get(i)).cloned() {
            self.player.stop_timeline(&old);
        }
        if let Some(name) = self.diff.get(index).cloned() {
            self.player
                .play_timeline(&name, TimelinePlayMode::from_flags(1));
        }
    }

    pub fn set_variable(&mut self, name: &str, value: f32) {
        self.player.set_variable_immediate(name, value);
    }

    /// Draws the model into a new image: model point (x, y) lands on
    /// (x * scale - left, y * scale - top) of the image, where (left, top)
    /// is returned with it.
    pub fn render(&self, scale: f32, gpu: Option<&mut Gpu>) -> Option<(Image, f32, f32)> {
        let scene = self.player.scene();
        let draws = draws::plan(scene, |i| self.textures.contains_key(&i));
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for d in &draws {
            for v in &d.mesh.vertices {
                x0 = x0.min(v.x);
                y0 = y0.min(v.y);
                x1 = x1.max(v.x);
                y1 = y1.max(v.y);
            }
        }
        if x0 > x1 || !(x1 - x0).is_finite() || !(y1 - y0).is_finite() {
            return None;
        }
        let (left, top) = ((x0 * scale).floor(), (y0 * scale).floor());
        let width = ((x1 * scale).ceil() - left).clamp(1.0, 8192.0) as u32;
        let height = ((y1 * scale).ceil() - top).clamp(1.0, 8192.0) as u32;
        let at = Placement {
            scale,
            dx: -left,
            dy: -top,
        };
        #[cfg(feature = "gpu")]
        if let Some(gpu) = gpu {
            gpu.upload(self.id, &self.textures);
            let mut rgba = gpu.draw_all(self.id, &draws, at, width, height);
            raster::unpremultiply(&mut rgba);
            return Some((
                Image {
                    width,
                    height,
                    rgba,
                },
                left,
                top,
            ));
        }
        #[cfg(not(feature = "gpu"))]
        let _ = gpu;
        let mut target = Target::new(width, height);
        target.draw_all(&draws, &self.textures, at);
        let image = Image {
            width,
            height,
            rgba: target.to_rgba(),
        };
        Some((image, left, top))
    }
}

/// Anneliese's standing picture in きまぐれテンプテーション2: both models of
/// one costume and the timelines the script chose.
pub struct Character {
    /// `_h0007` and `_h0813`.
    models: Vec<Model>,
    /// The model drawn.
    active: usize,
    pose: Option<usize>,
    face: Option<usize>,
    extra: Option<usize>,
    sub: Option<usize>,
}

impl Character {
    /// Opens the two models (`_h0007` first).
    pub fn open(files: [&[u8]; 2], key: u32, level: u32) -> Result<Self, String> {
        let models = files
            .iter()
            .map(|f| Model::open(f, key, level))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            models,
            active: 0,
            pose: None,
            face: None,
            extra: None,
            sub: None,
        })
    }

    /// `Game.10(extra, face, 7)`: faces 0 to 7 are on the first model,
    /// 8 to 13 on the second (which lists face 0, then 8 to 13). The extra
    /// timeline is counted from the one after the faces in each model's
    /// list (`h00`, `h01`, then the `a` ones).
    pub fn set_face(&mut self, face: usize, extra: usize) {
        let (model, face_index, extra_base) = if face < 8 {
            (0, face, 8)
        } else {
            (1, face - 7, 7)
        };
        let model = model.min(self.models.len() - 1);
        let (old_face, old_extra) = if model == self.active {
            (self.face, self.extra)
        } else {
            (None, None)
        };
        let m = &mut self.models[model];
        m.play_diff(extra_base + extra, old_extra);
        m.play_diff(face_index, old_face);
        self.active = model;
        self.face = Some(face_index);
        self.extra = Some(extra_base + extra);
    }

    /// `Game.10(n, 13)`: main timeline `n` on both models.
    pub fn set_pose(&mut self, index: usize) {
        let previous = self.pose;
        for m in &mut self.models {
            m.play_main(index, previous);
        }
        self.pose = Some(index);
    }

    /// `Game.10(n, 16)`: the `a` timelines (n from 0) on both models.
    pub fn set_sub(&mut self, n: usize) {
        let previous = self.sub;
        for (i, m) in self.models.iter_mut().enumerate() {
            let base = if i == 0 { 10 } else { 9 };
            m.play_diff(base + n, previous.map(|p| p + base));
        }
        self.sub = Some(n);
    }

    /// The mouth while the voice speaks (0 closed .. 1 open).
    pub fn set_talk(&mut self, value: f32) {
        for m in &mut self.models {
            m.set_variable("face_talk", value);
        }
    }

    pub fn progress(&mut self, ms: f32) -> Result<(), String> {
        let ticks = ms * 60.0 / 1000.0;
        for m in &mut self.models {
            m.progress(ticks)?;
        }
        Ok(())
    }

    pub fn render(&self, scale: f32, gpu: Option<&mut Gpu>) -> Option<(Image, f32, f32)> {
        #[cfg(feature = "gpu")]
        let mut gpu = gpu;
        #[cfg(feature = "gpu")]
        if let Some(g) = gpu.as_deref_mut() {
            g.retain(&self.models.iter().map(|m| m.id).collect());
        }
        self.models.get(self.active)?.render(scale, gpu)
    }
}

/// Decodes a texture resource: DXT1/DXT5, A8, or raw 32-bit (`RL`
/// compressed or not).
fn decode_texture(source: &EmoteTextureSource, bytes: &[u8]) -> Option<Texture> {
    let (w, h) = (source.width.max(1), source.height.max(1));
    let format = source.format.as_deref().unwrap_or("").to_ascii_uppercase();
    let rgba = match format.as_str() {
        "DXT5" => decode_dxt(bytes, w, h, true)?,
        "DXT1" => decode_dxt(bytes, w, h, false)?,
        "A8" | "L8" => {
            let n = (w * h) as usize;
            let src = bytes.get(..n)?;
            let mut out = vec![0u8; n * 4];
            for (o, &a) in out.chunks_exact_mut(4).zip(src) {
                if format == "A8" {
                    o[3] = a;
                } else {
                    o.copy_from_slice(&[a, a, a, 255]);
                }
            }
            out
        }
        _ => {
            let n = (w * h * 4) as usize;
            let raw = if source
                .compress
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case("RL"))
            {
                decode_rl(bytes, n)?
            } else {
                bytes.get(..n)?.to_vec()
            };
            let mut out = raw;
            if !matches!(format.as_str(), "RGBA8" | "BERGBA8") {
                for p in out.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
            }
            out
        }
    };
    Some(Texture {
        width: w,
        height: h,
        rgba,
    })
}

/// PSB run-length coding of 4-byte pixels.
fn decode_rl(src: &[u8], len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len && i < src.len() {
        let c = src[i] as usize;
        i += 1;
        if c & 0x80 != 0 {
            let n = (c & 0x7f) + 3;
            let px = src.get(i..i + 4)?;
            i += 4;
            for _ in 0..n {
                out.extend_from_slice(px);
            }
        } else {
            let n = (c + 1) * 4;
            out.extend_from_slice(src.get(i..i + n)?);
            i += n;
        }
    }
    out.resize(len, 0);
    Some(out)
}

/// BC1 (DXT1) or BC3 (DXT5) blocks to RGBA8.
fn decode_dxt(src: &[u8], w: u32, h: u32, bc3: bool) -> Option<Vec<u8>> {
    let block = if bc3 { 16 } else { 8 };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    if src.len() < (bw * bh) as usize * block {
        return None;
    }
    let mut out = vec![0u8; (w * h * 4) as usize];
    let rgb565 = |c: u16| {
        let r = ((c >> 11) & 31) as u32;
        let g = ((c >> 5) & 63) as u32;
        let b = (c & 31) as u32;
        [
            (r * 527 + 23) >> 6,
            (g * 259 + 33) >> 6,
            (b * 527 + 23) >> 6,
        ]
    };
    for by in 0..bh {
        for bx in 0..bw {
            let b = &src[((by * bw + bx) as usize) * block..][..block];
            let (alpha, color) = if bc3 { b.split_at(8) } else { b.split_at(0) };
            let mut alphas = [255u8; 16];
            if bc3 {
                let (a0, a1) = (alpha[0] as u32, alpha[1] as u32);
                let table: [u32; 8] = if a0 > a1 {
                    std::array::from_fn(|i| match i {
                        0 => a0,
                        1 => a1,
                        _ => ((8 - i as u32) * a0 + (i as u32 - 1) * a1) / 7,
                    })
                } else {
                    std::array::from_fn(|i| match i {
                        0 => a0,
                        1 => a1,
                        6 => 0,
                        7 => 255,
                        _ => ((6 - i as u32) * a0 + (i as u32 - 1) * a1) / 5,
                    })
                };
                let bits = alpha[2..8]
                    .iter()
                    .enumerate()
                    .fold(0u64, |acc, (i, &v)| acc | (v as u64) << (8 * i));
                for (k, a) in alphas.iter_mut().enumerate() {
                    *a = table[((bits >> (3 * k)) & 7) as usize] as u8;
                }
            }
            let c0 = u16::from_le_bytes([color[0], color[1]]);
            let c1 = u16::from_le_bytes([color[2], color[3]]);
            let (p0, p1) = (rgb565(c0), rgb565(c1));
            let four = bc3 || c0 > c1;
            let palette: [[u32; 4]; 4] = std::array::from_fn(|i| {
                let mix = |a: u32, b: u32| match (i, four) {
                    (2, true) => (2 * a + b) / 3,
                    (3, true) => (a + 2 * b) / 3,
                    (2, false) => (a + b) / 2,
                    _ => 0,
                };
                match i {
                    0 => [p0[0], p0[1], p0[2], 255],
                    1 => [p1[0], p1[1], p1[2], 255],
                    3 if !four => [0, 0, 0, 0],
                    _ => [mix(p0[0], p1[0]), mix(p0[1], p1[1]), mix(p0[2], p1[2]), 255],
                }
            });
            let idx = u32::from_le_bytes([color[4], color[5], color[6], color[7]]);
            for k in 0..16u32 {
                let (x, y) = (bx * 4 + k % 4, by * 4 + k / 4);
                if x >= w || y >= h {
                    continue;
                }
                let c = palette[((idx >> (2 * k)) & 3) as usize];
                let o = ((y * w + x) * 4) as usize;
                out[o] = c[0] as u8;
                out[o + 1] = c[1] as u8;
                out[o + 2] = c[2] as u8;
                out[o + 3] = if bc3 { alphas[k as usize] } else { c[3] as u8 };
            }
        }
    }
    Some(out)
}

/// Size of the E-mote standing picture: game pixels per model unit.
pub const STAGE_SCALE: f32 = 0.35;
/// Where the model's origin lands on the 1280 x 720 screen. Provisional:
/// centred, feet at the bottom edge; not yet checked against the game.
pub const STAGE_ORIGIN: (f32, f32) = (640.0, 400.0);

/// The E-mote picture of a scene: the open models and the last image.
pub struct Stage {
    character: Character,
    applied: crate::scene::Emote,
    image: Option<(std::sync::Arc<Image>, f32, f32)>,
    /// Milliseconds of animation not yet run.
    pending: f32,
    talk: f32,
}

impl Stage {
    /// Opens the models of `emote` with `read` (file name to bytes).
    pub fn open(
        emote: &crate::scene::Emote,
        read: impl Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, String> {
        let a = read(&emote.files[0]).ok_or_else(|| format!("{} not found", emote.files[0]))?;
        let b = read(&emote.files[1]).ok_or_else(|| format!("{} not found", emote.files[1]))?;
        let key = find_key(&a, None).ok_or("no E-mote key opens the file")?;
        let character = Character::open([&a, &b], key, 1)?;
        let mut stage = Self {
            character,
            applied: crate::scene::Emote {
                files: emote.files.clone(),
                ..Default::default()
            },
            image: None,
            pending: 0.0,
            talk: 0.0,
        };
        stage.apply(emote);
        // Settle the first pose before it is shown.
        let _ = stage.character.progress(500.0);
        Ok(stage)
    }

    pub fn files(&self) -> &[String; 2] {
        &self.applied.files
    }

    /// Plays what changed in `emote` since the last call.
    pub fn apply(&mut self, emote: &crate::scene::Emote) {
        if emote.face != self.applied.face {
            if let Some((face, extra)) = emote.face {
                self.character
                    .set_face(face.clamp(0, 13) as usize, extra.max(0) as usize);
            }
        }
        if emote.sub != self.applied.sub {
            if let Some(n) = emote.sub {
                self.character.set_sub(n.max(0) as usize);
            }
        }
        if emote.pose != self.applied.pose {
            if let Some(n) = emote.pose {
                self.character.set_pose(n.max(0) as usize);
            }
        }
        if *emote != self.applied {
            self.image = None;
        }
        self.applied = emote.clone();
    }

    /// Runs the animation in steps of at least `interval` ms; true when
    /// the picture changed.
    pub fn progress(&mut self, ms: f32, talking: Option<f32>, interval: f32) -> bool {
        let talk = talking.unwrap_or(0.0);
        if talk != self.talk {
            self.talk = talk;
            self.character.set_talk(talk);
        }
        self.pending += ms;
        if self.pending < interval {
            return false;
        }
        let step = std::mem::take(&mut self.pending).min(250.0);
        if self.character.progress(step).is_err() {
            return false;
        }
        self.image = None;
        true
    }

    /// Draws the picture again on the next call to [`Stage::image`].
    pub fn redraw(&mut self) {
        self.image = None;
    }

    /// The picture and the screen position of its top left corner.
    pub fn image(&mut self, gpu: Option<&mut Gpu>) -> Option<(std::sync::Arc<Image>, f32, f32)> {
        if self.image.is_none() {
            let (img, left, top) = self.character.render(STAGE_SCALE, gpu)?;
            self.image = Some((
                std::sync::Arc::new(img),
                STAGE_ORIGIN.0 + left,
                STAGE_ORIGIN.1 + top,
            ));
        }
        self.image.clone()
    }
}
