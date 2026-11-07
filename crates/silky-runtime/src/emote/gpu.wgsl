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
    if (low == 3u || low == 4u) {
        rgb = rgb * c.a;
    } else if (low == 5u) {
        rgb = vec3<f32>(1.0) - rgb;
    }
    if (c.a <= 0.003) {
        discard;
    }
    return vec4<f32>(rgb, c.a);
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
