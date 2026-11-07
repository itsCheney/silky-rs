//! Draws recorded frames ([`Canvas::recording`]) with wgpu, at output
//! size: Metal on Apple devices, Vulkan, D3D12 or GL elsewhere.
//!
//! Pictures, glyphs and E-mote models stay on the GPU between frames,
//! keyed by the shared pointers the recording holds (an entry goes when
//! its picture is dropped or has not been drawn for a while). A recorded
//! list is drawn into a texture of its own and kept while it lives, so an
//! unchanged frame costs nothing, and a transition mixes its two finished
//! frames in a shader every frame without drawing them again.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use silky_format::akb::Image;
use wgpu::util::DeviceExt;

use crate::canvas::{Canvas, Glyph, Op, Placing};
use crate::emote::{Gpu as EmoteGpu, ModelFrame, Placement};

/// Format of every frame texture: straight bytes, as the software renderer
/// writes them (no sRGB conversion).
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Renders an entry may go undrawn before it is dropped.
const KEEP: u64 = 600;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    clip: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    clamp: [f32; 4],
    kind: u32,
}

/// Coverage in the red channel instead of a premultiplied picture.
const KIND_MASK: u32 = 1;

struct Entry<T: ?Sized> {
    weak: Weak<T>,
    group: wgpu::BindGroup,
    used: u64,
}

struct ListEntry {
    weak: Weak<Vec<Op>>,
    texture: wgpu::Texture,
    group: wgpu::BindGroup,
    used: u64,
}

struct ModelEntry {
    weak: Weak<ModelFrame>,
    group: wgpu::BindGroup,
    /// Output pixel rectangle (x0, y0, x1, y1).
    rect: [f32; 4],
    used: u64,
}

/// One step of a list's pass.
enum Step {
    Quads(wgpu::BindGroup, std::ops::Range<u32>),
    Mix(wgpu::BindGroup),
}

pub struct Compositor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    quad: wgpu::RenderPipeline,
    mix: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    mix_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::BindGroup,
    /// A 1 x 1 rule for transitions without one.
    no_rule: wgpu::TextureView,
    images: HashMap<(usize, bool), Entry<Image>>,
    glyphs: HashMap<usize, Entry<Glyph>>,
    rules: HashMap<usize, (Weak<Vec<u8>>, wgpu::TextureView, u64)>,
    lists: HashMap<usize, ListEntry>,
    models: HashMap<usize, ModelEntry>,
    emote: EmoteGpu,
    /// Model ids whose textures the E-mote raster keeps, with the render
    /// they were last drawn in.
    model_ids: HashMap<u64, u64>,
    tick: u64,
}

impl Compositor {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compose"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compose.wgsl").into()),
        });
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compose picture"),
            entries: &[texture_entry(0), sampler_entry(1)],
        });
        let mix_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compose mix"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                texture_entry(2),
                sampler_entry(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let premultiplied = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let pipeline = |label, layout: &wgpu::BindGroupLayout, vs, fs, buffers: &[_], blend| {
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[layout],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Uint32
        ];
        let quad = pipeline(
            "compose quads",
            &layout,
            "vs_quad",
            "fs_quad",
            &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            }],
            Some(premultiplied),
        );
        let mix = pipeline("compose mix", &mix_layout, "vs_full", "fs_mix", &[], None);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("compose"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let emote = EmoteGpu::new(device.clone(), queue.clone());
        let mut c = Self {
            white: Self::empty_group(&device, &layout),
            no_rule: Self::r8(&device, &queue, 1, 1, &[0]).create_view(&Default::default()),
            device,
            queue,
            quad,
            mix,
            layout,
            mix_layout,
            sampler,
            images: HashMap::new(),
            glyphs: HashMap::new(),
            rules: HashMap::new(),
            lists: HashMap::new(),
            models: HashMap::new(),
            emote,
            model_ids: HashMap::new(),
            tick: 0,
        };
        let white = c.rgba_texture(1, 1, vec![vec![255; 4]]);
        c.white = c.group(&white.create_view(&Default::default()));
        c
    }

    /// On a device of its own (no window), if there is an adapter.
    pub fn headless() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("silky compose"),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .ok()?;
        Some(Self::new(device, queue))
    }

    /// A placeholder until the white texture exists.
    fn empty_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        let t = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let sampler = device.create_sampler(&Default::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &t.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Draws a recorded canvas; returns its frame at output size
    /// ([`Canvas::output_size`], format [`FORMAT`], opaque), or `None`
    /// for a canvas that was drawn, not recorded.
    pub fn render(&mut self, canvas: &Canvas) -> Option<&wgpu::Texture> {
        let ops = canvas.ops()?.clone();
        self.tick += 1;
        let size = canvas.output_size();
        let frame = Frame {
            size,
            scale: canvas.scale(),
            game: (canvas.width, canvas.height),
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("compose"),
            });
        let key = self.list(&mut encoder, &ops, frame);
        self.queue.submit([encoder.finish()]);
        self.evict();
        self.lists.get(&key).map(|e| &e.texture)
    }

    /// Draws a recorded canvas and reads it back as RGBA (tests, the
    /// headless runner).
    pub fn render_rgba(&mut self, canvas: &Canvas) -> Option<Canvas> {
        let texture = self.render(canvas)?.clone();
        let (w, h) = (texture.width(), texture.height());
        let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compose readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            texture.size(),
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::Wait);
        let mapped = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h as usize {
            rgba.extend_from_slice(&mapped[y * row as usize..][..w as usize * 4]);
        }
        drop(mapped);
        buffer.unmap();
        Some(Canvas::from_rgba(w, h, rgba))
    }

    /// Drops what was not drawn for a while or whose source is gone.
    fn evict(&mut self) {
        let tick = self.tick;
        let fresh = |used: u64| used + KEEP >= tick;
        self.images
            .retain(|_, e| e.weak.strong_count() > 0 && fresh(e.used));
        self.glyphs
            .retain(|_, e| e.weak.strong_count() > 0 && fresh(e.used));
        self.rules
            .retain(|_, (w, _, used)| w.strong_count() > 0 && fresh(*used));
        self.lists.retain(|_, e| e.weak.strong_count() > 0);
        self.models.retain(|_, e| e.weak.strong_count() > 0);
        let before = self.model_ids.len();
        self.model_ids.retain(|_, used| fresh(*used));
        if self.model_ids.len() != before {
            let live: HashSet<u64> = self.model_ids.keys().copied().collect();
            self.emote.retain(&live);
        }
    }

    // ---- textures ---------------------------------------------------------

    fn group(&self, view: &wgpu::TextureView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compose picture"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// An RGBA texture from its mip levels (largest first).
    fn rgba_texture(&self, width: u32, height: u32, levels: Vec<Vec<u8>>) -> wgpu::Texture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("compose picture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let (mut w, mut h) = (width, height);
        for (level, data) in levels.iter().enumerate() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        texture
    }

    fn r8(device: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32, data: &[u8]) -> wgpu::Texture {
        device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("compose mask"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        )
    }

    fn image(&mut self, img: &Arc<Image>, green_key: bool) -> wgpu::BindGroup {
        let key = (Arc::as_ptr(img) as usize, green_key);
        let tick = self.tick;
        if let Some(e) = self.images.get_mut(&key) {
            if e.weak.strong_count() > 0 {
                e.used = tick;
                return e.group.clone();
            }
        }
        let texture = self.rgba_texture(img.width, img.height, mip_levels(img, green_key));
        let group = self.group(&texture.create_view(&Default::default()));
        self.images.insert(
            key,
            Entry {
                weak: Arc::downgrade(img),
                group: group.clone(),
                used: tick,
            },
        );
        group
    }

    fn glyph(&mut self, g: &Arc<Glyph>) -> wgpu::BindGroup {
        let key = Arc::as_ptr(g) as usize;
        let tick = self.tick;
        if let Some(e) = self.glyphs.get_mut(&key) {
            if e.weak.strong_count() > 0 {
                e.used = tick;
                return e.group.clone();
            }
        }
        let t = Self::r8(
            &self.device,
            &self.queue,
            g.width as u32,
            g.height as u32,
            &g.mask,
        );
        let group = self.group(&t.create_view(&Default::default()));
        self.glyphs.insert(
            key,
            Entry {
                weak: Arc::downgrade(g),
                group: group.clone(),
                used: tick,
            },
        );
        group
    }

    fn rule(&mut self, rule: &Arc<Vec<u8>>, w: u32, h: u32) -> wgpu::TextureView {
        let key = Arc::as_ptr(rule) as usize;
        let tick = self.tick;
        if let Some((weak, view, used)) = self.rules.get_mut(&key) {
            if weak.strong_count() > 0 {
                *used = tick;
                return view.clone();
            }
        }
        let view = Self::r8(&self.device, &self.queue, w, h, &rule[..(w * h) as usize])
            .create_view(&Default::default());
        self.rules
            .insert(key, (Arc::downgrade(rule), view.clone(), tick));
        view
    }

    /// The model drawn into a texture of its own, and where it goes.
    fn model(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        m: &Arc<ModelFrame>,
        frame: Frame,
    ) -> Option<(wgpu::BindGroup, [f32; 4])> {
        let key = Arc::as_ptr(m) as usize;
        let tick = self.tick;
        self.model_ids.insert(m.id, tick);
        if let Some(e) = self.models.get_mut(&key) {
            e.used = tick;
            return Some((e.group.clone(), e.rect));
        }
        let ts = m.scale * frame.scale;
        let [x0, y0, x1, y1] = m.bounds;
        let (left, top) = ((x0 * ts).floor(), (y0 * ts).floor());
        let width = ((x1 * ts).ceil() - left).clamp(1.0, 8192.0) as u32;
        let height = ((y1 * ts).ceil() - top).clamp(1.0, 8192.0) as u32;
        self.emote.upload(m.id, &m.textures);
        let target = self
            .emote
            .target(width, height, wgpu::TextureUsages::TEXTURE_BINDING);
        let at = Placement {
            scale: ts,
            dx: -left,
            dy: -top,
        };
        self.emote.encode(encoder, m.id, &m.draws, at, &target);
        let group = self.group(&target.create_view(&Default::default()));
        let x = m.origin.0 * frame.scale + left;
        let y = m.origin.1 * frame.scale + top;
        let rect = [x, y, x + width as f32, y + height as f32];
        self.models.insert(
            key,
            ModelEntry {
                weak: Arc::downgrade(m),
                group: group.clone(),
                rect,
                used: tick,
            },
        );
        Some((group, rect))
    }

    // ---- lists ------------------------------------------------------------

    /// Draws a list into its texture, unless it is there already; the
    /// key of its entry in `lists`.
    fn list(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        ops: &Arc<Vec<Op>>,
        frame: Frame,
    ) -> usize {
        let key = Arc::as_ptr(ops) as usize;
        let tick = self.tick;
        if let Some(e) = self.lists.get_mut(&key) {
            if e.weak.strong_count() > 0 && e.texture.width() == frame.size.0 {
                e.used = tick;
                return key;
            }
        }
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut steps: Vec<Step> = Vec::new();
        let s = frame.scale;
        for op in ops.iter() {
            match op {
                Op::Image {
                    image,
                    part,
                    placing,
                    green_key,
                } => {
                    let (pl, pt, pw, ph) = *part;
                    let pw = pw.min(image.width.saturating_sub(pl));
                    let ph = ph.min(image.height.saturating_sub(pt));
                    if pw == 0 || ph == 0 {
                        continue;
                    }
                    let (rect, alpha) = match *placing {
                        Placing::Part { x, y, opacity } => {
                            let (x, y) = (x as f32, y as f32);
                            (
                                [x, y, x + pw as f32, y + ph as f32],
                                opacity as f32 / 255.0,
                            )
                        }
                        Placing::Scaled { x, y, scale } => {
                            let (x, y) = (x.round(), y.round());
                            let w = (image.width as f32 * scale).round();
                            let h = (image.height as f32 * scale).round();
                            ([x, y, x + w, y + h], 1.0)
                        }
                        Placing::Zoomed { x, y, scale } => (
                            [
                                x,
                                y,
                                x + image.width as f32 * scale,
                                y + image.height as f32 * scale,
                            ],
                            1.0,
                        ),
                    };
                    let group = self.image(image, *green_key);
                    let (iw, ih) = (image.width as f32, image.height as f32);
                    let uv = [
                        pl as f32 / iw,
                        pt as f32 / ih,
                        (pl + pw) as f32 / iw,
                        (pt + ph) as f32 / ih,
                    ];
                    let clamp = [
                        (pl as f32 + 0.5) / iw,
                        (pt as f32 + 0.5) / ih,
                        ((pl + pw) as f32 - 0.5) / iw,
                        ((pt + ph) as f32 - 0.5) / ih,
                    ];
                    let out = rect.map(|v| v * s);
                    push_quad(
                        &mut vertices,
                        &mut steps,
                        group,
                        frame,
                        out,
                        uv,
                        clamp,
                        [alpha; 4],
                        0,
                    );
                }
                Op::Fill { x, y, w, h, color } => {
                    let (x, y) = (*x as f32, *y as f32);
                    let rect = [x, y, x + *w as f32, y + *h as f32].map(|v| v * s);
                    push_quad(
                        &mut vertices,
                        &mut steps,
                        self.white.clone(),
                        frame,
                        rect,
                        [0.0, 0.0, 1.0, 1.0],
                        [0.0, 0.0, 1.0, 1.0],
                        premultiplied(*color),
                        0,
                    );
                }
                Op::Glyph {
                    glyph,
                    x,
                    y,
                    sharp,
                    color,
                } => {
                    if glyph.width == 0 || glyph.height == 0 {
                        continue;
                    }
                    let (x, y) = (*x as f32, *y as f32);
                    let (w, h) = (glyph.width as f32, glyph.height as f32);
                    let k = if *sharp { 1.0 } else { s };
                    let rect = [x * k, y * k, (x + w) * k, (y + h) * k];
                    let group = self.glyph(glyph);
                    push_quad(
                        &mut vertices,
                        &mut steps,
                        group,
                        frame,
                        rect,
                        [0.0, 0.0, 1.0, 1.0],
                        [0.5 / w, 0.5 / h, 1.0 - 0.5 / w, 1.0 - 0.5 / h],
                        premultiplied(*color),
                        KIND_MASK,
                    );
                }
                Op::Model(m) => {
                    if let Some((group, rect)) = self.model(encoder, m, frame) {
                        push_quad(
                            &mut vertices,
                            &mut steps,
                            group,
                            frame,
                            rect,
                            [0.0, 0.0, 1.0, 1.0],
                            [0.0, 0.0, 1.0, 1.0],
                            [1.0; 4],
                            0,
                        );
                    }
                }
                Op::Blend { from, to, t, rule } => {
                    let a = self.list(encoder, from, frame);
                    let b = self.list(encoder, to, frame);
                    let (gw, gh) = frame.game;
                    let rule = rule
                        .as_ref()
                        .filter(|r| r.len() >= (gw * gh) as usize)
                        .map(|r| self.rule(r, gw, gh));
                    let params = [
                        *t,
                        rule.is_some() as u32 as f32,
                        gw as f32,
                        gh as f32,
                    ];
                    let uniform = self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("compose mix"),
                            contents: bytemuck::cast_slice(&params),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                    let view = |k: usize| {
                        self.lists[&k]
                            .texture
                            .create_view(&Default::default())
                    };
                    let (va, vb) = (view(a), view(b));
                    let rule_view = rule.unwrap_or_else(|| self.no_rule.clone());
                    let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("compose mix"),
                        layout: &self.mix_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&va),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&vb),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(&rule_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::Sampler(&self.sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: uniform.as_entire_binding(),
                            },
                        ],
                    });
                    steps.push(Step::Mix(group));
                }
                Op::Transform {
                    ops: inner,
                    scale,
                    dx,
                    dy,
                } => {
                    let k = self.list(encoder, inner, frame);
                    let group = self.lists[&k].group.clone();
                    let (gw, gh) = (frame.game.0 as f32, frame.game.1 as f32);
                    let rect = [*dx, *dy, dx + gw * scale, dy + gh * scale].map(|v| v * s);
                    push_quad(
                        &mut vertices,
                        &mut steps,
                        group,
                        frame,
                        rect,
                        [0.0, 0.0, 1.0, 1.0],
                        [0.0, 0.0, 1.0, 1.0],
                        [1.0; 4],
                        0,
                    );
                }
            }
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("compose frame"),
            size: wgpu::Extent3d {
                width: frame.size.0,
                height: frame.size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let buffer = (!vertices.is_empty()).then(|| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("compose quads"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("compose"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for step in &steps {
                match step {
                    Step::Quads(group, range) => {
                        pass.set_pipeline(&self.quad);
                        if let Some(b) = &buffer {
                            pass.set_vertex_buffer(0, b.slice(..));
                        }
                        pass.set_bind_group(0, group, &[]);
                        pass.draw(range.clone(), 0..1);
                    }
                    Step::Mix(group) => {
                        pass.set_pipeline(&self.mix);
                        pass.set_bind_group(0, group, &[]);
                        pass.draw(0..3, 0..1);
                    }
                }
            }
        }
        let group = self.group(&view);
        self.lists.insert(
            key,
            ListEntry {
                weak: Arc::downgrade(ops),
                texture,
                group,
                used: tick,
            },
        );
        key
    }
}

/// Sizes of the frame a list is drawn for.
#[derive(Clone, Copy)]
struct Frame {
    /// Output pixels.
    size: (u32, u32),
    /// Output pixels per game pixel.
    scale: f32,
    /// Game pixels.
    game: (u32, u32),
}

/// Adds a quad (output pixel rectangle `rect`), joining the last step
/// when it uses the same picture.
#[allow(clippy::too_many_arguments)]
fn push_quad(
    vertices: &mut Vec<Vertex>,
    steps: &mut Vec<Step>,
    group: wgpu::BindGroup,
    frame: Frame,
    rect: [f32; 4],
    uv: [f32; 4],
    clamp: [f32; 4],
    color: [f32; 4],
    kind: u32,
) {
    let (w, h) = (frame.size.0 as f32, frame.size.1 as f32);
    let at = |x: f32, y: f32, u: f32, v: f32| Vertex {
        clip: [x / w * 2.0 - 1.0, 1.0 - y / h * 2.0],
        uv: [u, v],
        color,
        clamp,
        kind,
    };
    let [x0, y0, x1, y1] = rect;
    let [u0, v0, u1, v1] = uv;
    let start = vertices.len() as u32;
    vertices.extend([
        at(x0, y0, u0, v0),
        at(x1, y0, u1, v0),
        at(x0, y1, u0, v1),
        at(x1, y0, u1, v0),
        at(x1, y1, u1, v1),
        at(x0, y1, u0, v1),
    ]);
    let end = vertices.len() as u32;
    if let Some(Step::Quads(g, range)) = steps.last_mut() {
        if *g == group && range.end == start {
            range.end = end;
            return;
        }
    }
    steps.push(Step::Quads(group, start..end));
}

fn premultiplied(c: [u8; 4]) -> [f32; 4] {
    let a = c[3] as f32 / 255.0;
    [
        c[0] as f32 / 255.0 * a,
        c[1] as f32 / 255.0 * a,
        c[2] as f32 / 255.0 * a,
        a,
    ]
}

/// The picture premultiplied (pure green cleared with `green_key`), and
/// its halved levels down to 1 x 1.
fn mip_levels(img: &Image, green_key: bool) -> Vec<Vec<u8>> {
    let mut level: Vec<u8> = img
        .rgba
        .chunks_exact(4)
        .flat_map(|p| {
            if green_key && p[0] == 0 && p[1] == 255 && p[2] == 0 {
                return [0, 0, 0, 0];
            }
            let a = p[3] as u32;
            [
                ((p[0] as u32 * a + 127) / 255) as u8,
                ((p[1] as u32 * a + 127) / 255) as u8,
                ((p[2] as u32 * a + 127) / 255) as u8,
                p[3],
            ]
        })
        .collect();
    let (mut w, mut h) = (img.width.max(1), img.height.max(1));
    let mut out = Vec::new();
    loop {
        let done = w == 1 && h == 1;
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = Vec::new();
        if !done {
            next = vec![0u8; (nw * nh * 4) as usize];
            for y in 0..nh {
                for x in 0..nw {
                    for k in 0..4 {
                        let mut sum = 0u32;
                        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                            let sx = (x * 2 + dx).min(w - 1);
                            let sy = (y * 2 + dy).min(h - 1);
                            sum += level[((sy * w + sx) * 4 + k) as usize] as u32;
                        }
                        next[((y * nw + x) * 4 + k) as usize] = ((sum + 2) / 4) as u8;
                    }
                }
            }
        }
        out.push(std::mem::take(&mut level));
        if done {
            return out;
        }
        level = next;
        w = nw;
        h = nh;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(w: u32, h: u32) -> Arc<Image> {
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgba.extend([(x * 7) as u8, (y * 5) as u8, 128, if x < 4 { 0 } else { 200 }]);
            }
        }
        Arc::new(Image {
            width: w,
            height: h,
            rgba,
        })
    }

    /// Largest channel difference between the compositor's frame and the
    /// software playback of the same recording.
    fn worst(c: &mut Compositor, canvas: &Canvas) -> u8 {
        let gpu = c.render_rgba(canvas).unwrap();
        let cpu = canvas.rasterize().present();
        assert_eq!((gpu.width, gpu.height), (cpu.width, cpu.height));
        gpu.rgba
            .iter()
            .zip(&cpu.rgba)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap()
    }

    #[test]
    fn matches_software() {
        let Some(mut c) = Compositor::headless() else {
            eprintln!("no GPU adapter: skipped");
            return;
        };
        let img = picture(24, 16);
        let mut a = Canvas::recording(64, 48, 1.0);
        a.fill(0, 0, 64, 48, [30, 60, 90, 255]);
        a.draw(&img, 5, 7, false);
        a.draw_part(&img, (4, 0, 8, 8), 40, 30, false, 128);
        a.fill(10, 10, 20, 5, [255, 0, 0, 100]);
        assert!(worst(&mut c, &a) <= 2);

        let mut b = Canvas::recording(64, 48, 1.0);
        b.fill(0, 0, 64, 48, [200, 200, 200, 255]);
        let half = Canvas::blend_shared(&a, &b, 0.5, None);
        assert!(worst(&mut c, &half) <= 2);
        let rule: Vec<u8> = (0..64 * 48).map(|i| (i % 64 * 4) as u8).collect();
        let wipe = Canvas::blend_shared(&a, &b, 0.4, Some(Arc::new(rule)));
        assert!(worst(&mut c, &wipe) <= 2);

        let mut shaken = a.clone();
        shaken.shift(3, -2);
        assert!(worst(&mut c, &shaken) <= 2);
    }
}
