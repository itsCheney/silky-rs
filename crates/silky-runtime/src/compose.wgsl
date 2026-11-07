// Frame compositor: textured quads into a premultiplied RGBA target, and
// the transition mix of two finished frames.

struct Quad {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) clamp_rect: vec4<f32>,
    @location(3) @interpolate(flat) kind: u32,
};

struct QuadIn {
    @location(0) clip: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) clamp_rect: vec4<f32>,
    @location(4) kind: u32,
};

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@vertex
fn vs_quad(v: QuadIn) -> Quad {
    var out: Quad;
    out.position = vec4<f32>(v.clip, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    out.clamp_rect = v.clamp_rect;
    out.kind = v.kind;
    return out;
}

// kind 0: premultiplied texture times `color`; kind 1: coverage in the red
// channel times `color` (premultiplied).
@fragment
fn fs_quad(q: Quad) -> @location(0) vec4<f32> {
    let uv = clamp(q.uv, q.clamp_rect.xy, q.clamp_rect.zw);
    let t = textureSample(tex, samp, uv);
    if (q.kind == 1u) {
        return q.color * t.r;
    }
    return t * q.color;
}

struct Mix {
    // t, rule present (0 or 1), screen width and height in game pixels.
    params: vec4<f32>,
};

@group(0) @binding(0) var from_tex: texture_2d<f32>;
@group(0) @binding(1) var to_tex: texture_2d<f32>;
@group(0) @binding(2) var rule_tex: texture_2d<f32>;
@group(0) @binding(3) var mix_samp: sampler;
@group(0) @binding(4) var<uniform> mix_params: Mix;

struct Full {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> Full {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Full;
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

// As `Canvas::blend`: all at once by t, or pixel by pixel in the order of
// the rule mask with a soft edge of 32 steps.
@fragment
fn fs_mix(v: Full) -> @location(0) vec4<f32> {
    let p = mix_params.params;
    var a = p.x;
    if (p.y > 0.5) {
        let size = vec2<i32>(textureDimensions(rule_tex));
        let at = clamp(vec2<i32>(v.uv * p.zw), vec2<i32>(0), size - 1);
        let r = textureLoad(rule_tex, at, 0).r * 255.0;
        a = clamp((p.x * (255.0 + 32.0) - r) / 32.0, 0.0, 1.0);
    }
    let f = textureSample(from_tex, mix_samp, v.uv);
    let g = textureSample(to_tex, mix_samp, v.uv);
    return vec4<f32>(mix(f.rgb, g.rgb, a), 1.0);
}
