//! Software rasterizer for E-mote draws: textured, colored triangles with
//! the six native blend equations and an 8-bit stencil, into a
//! premultiplied float target.

use std::collections::HashMap;

use super::draws::{Draw, Mesh, Style, Vertex};

/// A straight-alpha RGBA texture.
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Texture {
    /// Halves the size (2x2 box filter, alpha weighted).
    pub fn half(&self) -> Texture {
        let (w, h) = ((self.width / 2).max(1), (self.height / 2).max(1));
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(self.width - 1);
                    let sy = (y * 2 + dy).min(self.height - 1);
                    let i = ((sy * self.width + sx) * 4) as usize;
                    let a = self.rgba[i + 3] as u32;
                    for k in 0..3 {
                        acc[k] += self.rgba[i + k] as u32 * a;
                    }
                    acc[3] += a;
                }
                let o = ((y * w + x) * 4) as usize;
                if acc[3] > 0 {
                    for k in 0..3 {
                        rgba[o + k] = (acc[k] / acc[3]) as u8;
                    }
                }
                rgba[o + 3] = (acc[3] / 4) as u8;
            }
        }
        Texture {
            width: w,
            height: h,
            rgba,
        }
    }

    /// Bilinear, clamped, straight RGBA in 0..1; (u, v) normalized.
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let fx = (u * self.width as f32 - 0.5).clamp(0.0, (self.width - 1) as f32);
        let fy = (v * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (fx as u32, fy as u32);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let px = |x: u32, y: u32| {
            let i = ((y * self.width + x) * 4) as usize;
            &self.rgba[i..i + 4]
        };
        let (a, b, c, d) = (px(x0, y0), px(x1, y0), px(x0, y1), px(x1, y1));
        std::array::from_fn(|k| {
            let top = a[k] as f32 + (b[k] as f32 - a[k] as f32) * tx;
            let bottom = c[k] as f32 + (d[k] as f32 - c[k] as f32) * tx;
            (top + (bottom - top) * ty) / 255.0
        })
    }
}

/// Model coordinates to target pixels: `x * scale + dx`.
#[derive(Clone, Copy)]
pub struct Placement {
    pub scale: f32,
    pub dx: f32,
    pub dy: f32,
}

pub struct Target {
    pub width: u32,
    pub height: u32,
    /// Premultiplied RGBA.
    pub color: Vec<[f32; 4]>,
    stencil: Vec<u8>,
}

impl Target {
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        Self {
            width,
            height,
            color: vec![[0.0; 4]; n],
            stencil: vec![0; n],
        }
    }

    /// Straight RGBA8.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.color.len() * 4);
        for c in &self.color {
            let a = c[3].clamp(0.0, 1.0);
            if a <= 0.0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            for k in 0..3 {
                out.push(((c[k] / a).clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.push((a * 255.0).round() as u8);
        }
        out
    }

    pub fn draw_all(&mut self, draws: &[Draw], textures: &HashMap<u32, Texture>, at: Placement) {
        for draw in draws {
            let Some(texture) = textures.get(&draw.mesh.style.texture) else {
                continue;
            };
            if draw.stencil.is_empty() {
                self.mesh(&draw.mesh, texture, at, Pass::Color(None));
                continue;
            }
            // The stencil starts at the initial reference over the area the
            // masks and the sprite cover.
            let mut area = bounds(&draw.mesh, at, self.width, self.height);
            for group in &draw.stencil {
                for mask in &group.masks {
                    area = union(area, bounds(mask, at, self.width, self.height));
                }
            }
            if let Some((x0, y0, x1, y1)) = area {
                for y in y0..y1 {
                    let row = (y * self.width) as usize;
                    self.stencil[row + x0 as usize..row + x1 as usize].fill(draw.stencil_initial);
                }
            }
            let mut reference = draw.stencil_initial;
            for group in draw.stencil.iter().filter(|g| g.phase == 1) {
                for mask in &group.masks {
                    if let Some(t) = textures.get(&mask.style.texture) {
                        self.mesh(mask, t, at, Pass::Increment(reference));
                    }
                }
                reference = reference.saturating_add(1);
            }
            for group in draw.stencil.iter().filter(|g| g.phase == 2) {
                for mask in &group.masks {
                    if let Some(t) = textures.get(&mask.style.texture) {
                        self.mesh(mask, t, at, Pass::Decrement(draw.stencil_final));
                    }
                }
            }
            self.mesh(
                &draw.mesh,
                texture,
                at,
                Pass::Color(Some(draw.stencil_final)),
            );
        }
    }

    fn mesh(&mut self, mesh: &Mesh, texture: &Texture, at: Placement, pass: Pass) {
        for tri in mesh.vertices.chunks_exact(3) {
            self.triangle(tri, &mesh.style, texture, at, pass);
        }
    }

    fn triangle(
        &mut self,
        v: &[Vertex],
        style: &Style,
        texture: &Texture,
        at: Placement,
        pass: Pass,
    ) {
        let p: [(f32, f32); 3] =
            std::array::from_fn(|i| (v[i].x * at.scale + at.dx, v[i].y * at.scale + at.dy));
        let area = edge(p[0], p[1], p[2]);
        if area.abs() < 1e-6 || !area.is_finite() {
            return;
        }
        // Counter-clockwise in y-down pixels; flip the order otherwise.
        let (i1, i2) = if area > 0.0 { (1, 2) } else { (2, 1) };
        let (a, b, c) = (p[0], p[i1], p[i2]);
        let (va, vb, vc) = (&v[0], &v[i1], &v[i2]);
        let area = area.abs();
        let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as i64;
        let max_x = (a.0.max(b.0).max(c.0).ceil() as i64).min(self.width as i64);
        let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as i64;
        let max_y = (a.1.max(b.1).max(c.1).ceil() as i64).min(self.height as i64);
        if min_x >= max_x || min_y >= max_y {
            return;
        }
        // Top-left rule, so that triangles sharing an edge draw each pixel once.
        let bias = |p0: (f32, f32), p1: (f32, f32)| {
            let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
            if (dy == 0.0 && dx > 0.0) || dy < 0.0 {
                0.0
            } else {
                -1e-7
            }
        };
        let (bias0, bias1, bias2) = (bias(b, c), bias(c, a), bias(a, b));
        for y in min_y..max_y {
            let py = y as f32 + 0.5;
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let w0 = edge(b, c, (px, py));
                let w1 = edge(c, a, (px, py));
                let w2 = edge(a, b, (px, py));
                if w0 + bias0 < 0.0 || w1 + bias1 < 0.0 || w2 + bias2 < 0.0 {
                    continue;
                }
                let (l0, l1, l2) = (w0 / area, w1 / area, w2 / area);
                let lerp = |f: fn(&Vertex) -> f32| f(va) * l0 + f(vb) * l1 + f(vc) * l2;
                let (mx, my) = (lerp(|v| v.x), lerp(|v| v.y));
                let [cl, ct, cr, cb] = style.clip;
                if mx < cl || my < ct || mx > cr || my > cb {
                    continue;
                }
                let i = (y as u32 * self.width + x as u32) as usize;
                match pass {
                    Pass::Color(Some(r)) | Pass::Increment(r) | Pass::Decrement(r)
                        if self.stencil[i] != r =>
                    {
                        continue
                    }
                    _ => {}
                }
                let t = texture.sample(lerp(|v| v.u), lerp(|v| v.v));
                let color: [f32; 4] = std::array::from_fn(|k| {
                    t[k] * (va.color[k] * l0 + vb.color[k] * l1 + vc.color[k] * l2)
                });
                let Some(src) = fragment(color, style) else {
                    continue;
                };
                match pass {
                    Pass::Increment(_) => self.stencil[i] = self.stencil[i].saturating_add(1),
                    Pass::Decrement(_) => self.stencil[i] = self.stencil[i].saturating_sub(1),
                    Pass::Color(_) => blend(&mut self.color[i], src, style.blend_mode & 0xFF0F),
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Pass {
    /// Color, where the stencil equals the reference if there is one.
    Color(Option<u8>),
    Increment(u8),
    Decrement(u8),
}

fn edge(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
}

type Rect = Option<(u32, u32, u32, u32)>;

fn bounds(mesh: &Mesh, at: Placement, width: u32, height: u32) -> Rect {
    let mut r: Option<(f32, f32, f32, f32)> = None;
    for v in &mesh.vertices {
        let (x, y) = (v.x * at.scale + at.dx, v.y * at.scale + at.dy);
        r = Some(match r {
            None => (x, y, x, y),
            Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
        });
    }
    let (x0, y0, x1, y1) = r?;
    let clamp = |v: f32, max: u32| v.clamp(0.0, max as f32) as u32;
    let rect = (
        clamp(x0.floor(), width),
        clamp(y0.floor(), height),
        clamp(x1.ceil(), width),
        clamp(y1.ceil(), height),
    );
    (rect.0 < rect.2 && rect.1 < rect.3).then_some(rect)
}

fn union(a: Rect, b: Rect) -> Rect {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))),
        (a, None) => a,
        (None, b) => b,
    }
}

/// Premultiplied RGBA8 to straight, in place.
#[cfg_attr(not(feature = "gpu"), allow(dead_code))]
pub fn unpremultiply(rgba: &mut [u8]) {
    for p in rgba.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a == 0 {
            p[..3].fill(0);
        } else if a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// The pixel shader: wipe, the texture stage of the blend mode, alpha test.
fn fragment(mut c: [f32; 4], style: &Style) -> Option<[f32; 4]> {
    if let Some((scale, bias)) = style.wipe {
        c[3] = (c[3] * scale + bias).clamp(0.0, 1.0);
    }
    let mode = style.blend_mode;
    if mode & 0xF0 == 0x10 {
        for k in 0..3 {
            c[k] = (c[k] * 2.0).min(1.0);
        }
    }
    match mode & 0xFF0F {
        3 | 4 => {
            for k in 0..3 {
                c[k] *= c[3];
            }
        }
        5 => {
            for k in 0..3 {
                c[k] = 1.0 - c[k];
            }
        }
        _ => {}
    }
    (c[3] > 0.003).then_some(c)
}

/// The fixed-function blend of mode 0..=5 into a premultiplied target.
fn blend(d: &mut [f32; 4], s: [f32; 4], mode: u32) {
    let a = s[3];
    match mode {
        1 => {
            for k in 0..3 {
                d[k] += s[k] * a;
            }
        }
        2 | 5 => {
            for k in 0..3 {
                d[k] = (d[k] - s[k] * a).max(0.0);
            }
        }
        3 => {
            for k in 0..3 {
                d[k] = s[k] * d[k] + d[k] * (1.0 - a);
            }
        }
        4 => {
            for k in 0..3 {
                d[k] += s[k] * (1.0 - d[k]);
            }
        }
        _ => {
            for k in 0..3 {
                d[k] = s[k] * a + d[k] * (1.0 - a);
            }
            d[3] = a + d[3] * (1.0 - a);
        }
    }
    for k in 0..3 {
        d[k] = d[k].min(1.0);
    }
}
