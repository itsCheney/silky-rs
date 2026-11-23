// E-mote sprites: the same fragment stage as the software rasterizer
// (`raster.rs`): clip rectangle in model coordinates, stencil wipe, the
// texture stage of the blend mode, alpha test. Blending and the stencil
// are fixed-function state.

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VertexIn {
    @location(0) clip: vec2<f32>,
    @location(1) model: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) clip_rect: vec4<f32>,
    // Blend mode, wipe scale, wipe bias, wipe on.
    @location(5) mode: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) model: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) clip_rect: vec4<f32>,
    @location(4) mode: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(v.clip, 0.0, 1.0);
    out.model = v.model;
    out.uv = v.uv;
    out.color = v.color;
    out.clip_rect = v.clip_rect;
    out.mode = v.mode;
    return out;
}

@fragment
fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    if (v.model.x < v.clip_rect.x || v.model.y < v.clip_rect.y ||
        v.model.x > v.clip_rect.z || v.model.y > v.clip_rect.w) {
        discard;
    }
    var c = textureSample(tex, samp, v.uv) * v.color;
    if (v.mode.w > 0.5) {
        c.a = clamp(c.a * v.mode.y + v.mode.z, 0.0, 1.0);
    }
    let mode = u32(v.mode.x + 0.5);
    var rgb = c.rgb;
    if ((mode & 0xF0u) == 0x10u) {
        rgb = min(rgb * 2.0, vec3<f32>(1.0));
    }
    let low = mode & 0xFF0Fu;
    if (low == 3u || low == 4u || low == 8u) {
        rgb = rgb * c.a;
    } else if (low == 5u) {
        rgb = vec3<f32>(1.0) - rgb;
    }
    if (c.a <= 0.003) {
        discard;
    }
    return vec4<f32>(rgb, c.a);
}

// Blends that read the picture below (`bm` 0x100 to 0x10f), as
// `raster::buffered`: drawn with blending off over a copy of the target.
@group(1) @binding(0) var below: texture_2d<f32>;

fn buffered(mode: u32, b: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    let one = vec3<f32>(1.0);
    var r = b;
    switch mode {
        case 0u: { r = b + c; }
        case 1u: { r = b + c - one; }
        case 2u: { r = select(one - (one - b) * (one - c) * 2.0, b * c * 2.0, b < vec3<f32>(0.5)); }
        case 3u: { r = select(one - (one - b) * (one - c) * 2.0, b * c * 2.0, c < vec3<f32>(0.5)); }
        case 4u: { r = pow(b, select((one - c) * 2.0, vec3<f32>(0.5) / c, c > vec3<f32>(0.5))); }
        case 5u: { r = b / max(one - c, vec3<f32>(1e-6)); }
        case 6u: { r = one - (one - b) / max(c, vec3<f32>(1e-6)); }
        case 7u: { r = max(b, c); }
        case 8u: { r = min(b, c); }
        case 9u: { r = abs(b - c); }
        case 10u: { r = b + c - 2.0 * b * c; }
        case 11u: { r = dot(b, vec3<f32>(0.298912, 0.586611, 0.114478)) * c; }
        default: {}
    }
    return clamp(r, vec3<f32>(0.0), one);
}

@fragment
fn fs_buffered(v: VertexOut) -> @location(0) vec4<f32> {
    if (v.model.x < v.clip_rect.x || v.model.y < v.clip_rect.y ||
        v.model.x > v.clip_rect.z || v.model.y > v.clip_rect.w) {
        discard;
    }
    var c = textureSample(tex, samp, v.uv) * v.color;
    if (v.mode.w > 0.5) {
        c.a = clamp(c.a * v.mode.y + v.mode.z, 0.0, 1.0);
    }
    let mode = u32(v.mode.x + 0.5);
    var rgb = c.rgb;
    if ((mode & 0xF0u) == 0x10u) {
        rgb = min(rgb * 2.0, vec3<f32>(1.0));
    }
    if (c.a <= 0.003) {
        discard;
    }
    let d = textureLoad(below, vec2<i32>(v.position.xy), 0);
    if (d.a <= 0.0) {
        return d;
    }
    let r = buffered(mode & 0xFu, d.rgb / d.a, rgb) * d.a;
    return vec4<f32>(mix(d.rgb, r, c.a), d.a);
}

// Full-target quad that sets the stencil before a masked sprite.
@vertex
fn vs_fill(@location(0) clip: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(clip, 0.0, 1.0);
}

@fragment
fn fs_fill() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}
