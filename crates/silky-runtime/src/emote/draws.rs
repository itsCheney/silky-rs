// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from siglus_rs (crates/siglus_scene_vm/src/render_plan/emote.rs,
// MPL-2.0): the same sprite geometry and stencil grouping, producing
// triangles in model coordinates for the software rasterizer instead of
// GPU vertices; a sprite without mesh transform is placed by its top left
// corner (see `rect_origin`).

//! The sprites of an Eluna scene as triangle lists, in drawing order, with
//! the stencil masks that clip them.

use std::collections::{HashMap, HashSet};

use eluna::{
    EmoteDrawFrameInfo, EmoteDrawPass, EmoteMeshPatch, EmoteStaticScene, EmoteStaticSprite,
};

/// One vertex, in model coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    /// Straight RGBA, opacity included.
    pub color: [f32; 4],
}

/// Fragment state shared by a sprite's triangles.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub texture: u32,
    /// Native blend mode: low nibble the equation, 0x10 modulate 2x.
    pub blend_mode: u32,
    /// Left, top, right, bottom in model coordinates.
    pub clip: [f32; 4],
    /// Alpha scale and bias of a stencil wipe.
    pub wipe: Option<(f32, f32)>,
}

#[derive(Debug)]
pub struct Mesh {
    pub style: Style,
    /// Three vertices per triangle.
    pub vertices: Vec<Vertex>,
}

/// Masks of one stencil phase: 1 increments inside, 2 decrements.
#[derive(Debug)]
pub struct StencilGroup {
    pub phase: u32,
    pub masks: Vec<Mesh>,
}

#[derive(Debug)]
pub struct Draw {
    pub mesh: Mesh,
    pub stencil: Vec<StencilGroup>,
    pub stencil_initial: u8,
    pub stencil_final: u8,
}

/// The draws of `scene`, in order. Sprites whose texture is not in
/// `has_texture` (and feedback sprites) are left out.
pub fn plan(scene: &EmoteStaticScene, has_texture: impl Fn(u32) -> bool) -> Vec<Draw> {
    let visible: Vec<&EmoteStaticSprite> = scene
        .sprites
        .iter()
        .filter(|s| s.visible && s.opacity > 0.0)
        .collect();
    let by_key: HashMap<&[u64], &EmoteStaticSprite> = visible
        .iter()
        .map(|s| (s.draw_frame_info.native_draw_key.as_slice(), *s))
        .collect();
    let layers: HashMap<&[u64], &EmoteDrawFrameInfo> = scene
        .layer_states
        .iter()
        .map(|state| {
            (
                state.draw_frame_info.native_draw_key.as_slice(),
                &state.draw_frame_info,
            )
        })
        .collect();
    let usable =
        |s: &EmoteStaticSprite| !s.feedback_history && has_texture(s.texture_resource_index);

    let mut draws = Vec::new();
    for sprite in visible.iter().copied() {
        if matches!(
            sprite.draw_frame_info.pass,
            EmoteDrawPass::MaskGeneration | EmoteDrawPass::StencilCompositeMask
        ) || !usable(sprite)
        {
            continue;
        }
        let mesh = sprite_mesh(sprite);
        if mesh.vertices.is_empty() {
            continue;
        }
        let (stencil, stencil_initial, stencil_final) =
            stencil_groups(scene, &by_key, &layers, sprite, &usable);
        draws.push(Draw {
            mesh,
            stencil,
            stencil_initial,
            stencil_final,
        });
    }
    draws
}

fn stencil_groups(
    scene: &EmoteStaticScene,
    by_key: &HashMap<&[u64], &EmoteStaticSprite>,
    layers: &HashMap<&[u64], &EmoteDrawFrameInfo>,
    sprite: &EmoteStaticSprite,
    usable: &impl Fn(&EmoteStaticSprite) -> bool,
) -> (Vec<StencilGroup>, u8, u8) {
    let mut chain = Vec::<&EmoteDrawFrameInfo>::new();
    let mut cursor = sprite.draw_frame_info.stencil_parent_native_key.as_deref();
    let mut seen = HashSet::<&[u64]>::new();
    while let Some(key) = cursor {
        if !seen.insert(key) {
            break;
        }
        let Some(info) = layers.get(key).copied() else {
            break;
        };
        if matches!(info.stencil_phase, 1 | 2) {
            chain.push(info);
        }
        cursor = info.stencil_parent_native_key.as_deref();
    }
    if chain.is_empty() {
        return (Vec::new(), 0, 0);
    }
    let initial: u8 = chain.iter().any(|info| info.stencil_phase == 2).into();
    let mut last = initial;
    let mut groups = Vec::new();
    for phase in [1, 2] {
        for info in chain.iter().filter(|info| info.stencil_phase == phase) {
            let keys = if info.stencil_type & 4 != 0 {
                scene
                    .composite_mask_sources_by_key
                    .get(&info.native_draw_key)
                    .cloned()
                    .unwrap_or_default()
            } else {
                vec![info.native_draw_key.clone()]
            };
            let masks = keys
                .iter()
                .filter_map(|key| by_key.get(key.as_slice()).copied())
                .filter(|source| usable(source))
                .map(sprite_mesh)
                .filter(|mesh| !mesh.vertices.is_empty())
                .collect();
            groups.push(StencilGroup {
                phase: phase as u32,
                masks,
            });
            if phase == 1 {
                last = last.saturating_add(1);
            }
        }
    }
    (groups, initial, last)
}

/// The frame's `bm` in the form the rasterizers take: the 0x10 MODULATE2X
/// bit and the blend equation in the low nibble. The models' shading parts
/// (eye and hair shadows) use 0x118, drawn as multiply; other high bits are
/// dropped.
fn blend_mode(bm: u32) -> u32 {
    let low = match bm & 0xF {
        8 => 3,
        low => low,
    };
    (bm & 0xF0) | low
}

fn sprite_mesh(sprite: &EmoteStaticSprite) -> Mesh {
    let info = &sprite.draw_frame_info;
    let style = Style {
        texture: sprite.texture_resource_index,
        blend_mode: blend_mode(sprite.blend_mode),
        clip: info.clip_rect.unwrap_or([-1.0e30, -1.0e30, 1.0e30, 1.0e30]),
        wipe: info
            .stencil_wipe_enabled
            .then_some((info.stencil_wipe_scale, info.stencil_wipe_bias)),
    };
    let corners = sprite.corner_colors.map(|c| corner_color(sprite, c));
    let vertices = match &sprite.mesh {
        Some(mesh) => mesh_vertices(sprite, mesh, corners),
        None => {
            let (l, t) = rect_origin(sprite);
            let (r, b) = (l + sprite.width, t + sprite.height);
            let at = |x: f32, y: f32, u: f32, v: f32, color: [f32; 4]| {
                let [x, y] = transform(sprite, [x, y]);
                Vertex { x, y, u, v, color }
            };
            let tl = at(l, t, sprite.uv_left, sprite.uv_top, corners[0]);
            let tr = at(r, t, sprite.uv_right, sprite.uv_top, corners[1]);
            let bl = at(l, b, sprite.uv_left, sprite.uv_bottom, corners[2]);
            let br = at(r, b, sprite.uv_right, sprite.uv_bottom, corners[3]);
            vec![tl, bl, tr, tr, bl, br]
        }
    };
    Mesh { style, vertices }
}

fn mesh_vertices(
    sprite: &EmoteStaticSprite,
    mesh: &EmoteMeshPatch,
    corners: [[f32; 4]; 4],
) -> Vec<Vertex> {
    let nx = mesh.division_x.max(1) as usize;
    let ny = mesh.division_y.max(1) as usize;
    let (left, top) = rect_origin(sprite);
    let mut grid = Vec::with_capacity((nx + 1) * (ny + 1));
    for iy in 0..=ny {
        for ix in 0..=nx {
            let u = ix as f32 / nx as f32;
            let v = iy as f32 / ny as f32;
            let local = match mesh.domain {
                Some([dl, dt, dw, dh]) if dw.abs() > f32::EPSILON && dh.abs() > f32::EPSILON => {
                    let lx = left + u * sprite.width;
                    let ly = top + v * sprite.height;
                    let p = mesh.sample((lx - dl) / dw, (ly - dt) / dh);
                    [dl + p[0] * dw, dt + p[1] * dh]
                }
                _ => {
                    let p = mesh.sample(u, v);
                    [left + p[0] * sprite.width, top + p[1] * sprite.height]
                }
            };
            let [x, y] = transform(sprite, local);
            grid.push(Vertex {
                x,
                y,
                u: sprite.uv_left + (sprite.uv_right - sprite.uv_left) * u,
                v: sprite.uv_top + (sprite.uv_bottom - sprite.uv_top) * v,
                color: bilerp(corners, u, v),
            });
        }
    }
    let at = |x: usize, y: usize| grid[y * (nx + 1) + x];
    let mut out = Vec::with_capacity(nx * ny * 6);
    for y in 0..ny {
        for x in 0..nx {
            let (tl, tr, bl, br) = (at(x, y), at(x + 1, y), at(x, y + 1), at(x + 1, y + 1));
            out.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }
    out
}

/// A corner color (0xRRGGBBAA) with the sprite's opacity.
fn corner_color(sprite: &EmoteStaticSprite, packed: u32) -> [f32; 4] {
    let byte = |shift: u32| ((packed >> shift) & 0xff) as f32 / 255.0;
    [
        byte(24),
        byte(16),
        byte(8),
        (byte(0) * sprite.opacity).clamp(0.0, 1.0),
    ]
}

fn bilerp(c: [[f32; 4]; 4], u: f32, v: f32) -> [f32; 4] {
    std::array::from_fn(|i| {
        let top = c[0][i] + (c[1][i] - c[0][i]) * u;
        let bottom = c[2][i] + (c[3][i] - c[2][i]) * u;
        top + (bottom - top) * v
    })
}

fn transform(sprite: &EmoteStaticSprite, p: [f32; 2]) -> [f32; 2] {
    let finite = |v: f32, default: f32| if v.is_finite() { v } else { default };
    let sx = finite(sprite.scale_x, 1.0);
    let sy = finite(sprite.scale_y, 1.0);
    let (sin, cos) = finite(sprite.rotation_degrees, 0.0).to_radians().sin_cos();
    let dx = (p[0] - sprite.center_x) * sx;
    let dy = (p[1] - sprite.center_y) * sy;
    let x = sprite.center_x + dx * cos - dy * sin;
    let y = sprite.center_y + dx * sin + dy * cos;
    let m = sprite.world_transform;
    [m[0] * x + m[1] * y + m[4], m[2] * x + m[3] * y + m[5]]
}

/// Top left corner of a sprite's rectangle. Eluna gives a sprite without
/// mesh transform (`meshTransform` 0) its position minus the icon's origin
/// as `center`; that point is the icon's top left corner: centred there,
/// faces and hair of the 2024 models land half an icon up and left.
fn rect_origin(sprite: &EmoteStaticSprite) -> (f32, f32) {
    if sprite.draw_frame_info.mesh_transform == 0 {
        (sprite.center_x, sprite.center_y)
    } else {
        (sprite.left(), sprite.top())
    }
}
