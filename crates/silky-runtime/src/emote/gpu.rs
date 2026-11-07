//! E-mote draws with wgpu: the same passes as the software rasterizer
//! (`raster.rs`), drawn into an offscreen target. The compositor
//! ([`crate::compose`]) draws that target into the frame; without it the
//! target is read back as an image.

use std::collections::{HashMap, HashSet};

use super::draws::{Draw, Mesh};
use super::raster::{Placement, Texture};

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const STENCIL: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuVertex {
    clip: [f32; 2],
    model: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    clip_rect: [f32; 4],
    mode: [f32; 4],
}

/// What a range of vertices is drawn as.
#[derive(Clone, Copy)]
enum Pass {
    /// Sets the whole stencil to the reference.
    Fill(u8),
    Increment(u8),
    Decrement(u8),
    /// Color with blend mode (0..=5), where the stencil equals the
    /// reference if there is one.
    Color(u32, Option<u8>),
}

struct Batch {
    pass: Pass,
    texture: Option<(u64, u32)>,
    range: std::ops::Range<u32>,
}

/// A wgpu device drawing E-mote models.
pub struct GpuRaster {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Blend modes 0..=5, without and with the stencil test.
    color: Vec<[wgpu::RenderPipeline; 2]>,
    increment: wgpu::RenderPipeline,
    decrement: wgpu::RenderPipeline,
    fill: wgpu::RenderPipeline,
    /// Uploaded textures by (model, resource index).
    textures: HashMap<(u64, u32), wgpu::BindGroup>,
}

impl GpuRaster {
    /// On a device of the frontend's.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("emote"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("emote texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("emote"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let textured = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("emote"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let plain = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("emote fill"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x2, 2 => Float32x2,
            3 => Float32x4, 4 => Float32x4, 5 => Float32x4
        ];
        let buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &attributes,
        }];
        let fill_attributes = wgpu::vertex_attr_array![0 => Float32x2];
        let fill_buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &fill_attributes,
        }];
        let pipeline = |label: &str,
                        blend: Option<wgpu::BlendState>,
                        write: bool,
                        stencil: wgpu::StencilFaceState,
                        fill: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(if fill { &plain } else { &textured }),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if fill { "vs_fill" } else { "vs_main" }),
                    buffers: if fill { &fill_buffers } else { &buffers },
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if fill { "fs_fill" } else { "fs_main" }),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR,
                        blend,
                        write_mask: if write {
                            wgpu::ColorWrites::ALL
                        } else {
                            wgpu::ColorWrites::empty()
                        },
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: STENCIL,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState {
                        front: stencil,
                        back: stencil,
                        read_mask: 0xff,
                        write_mask: 0xff,
                    },
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let face = |compare, pass_op| wgpu::StencilFaceState {
            compare,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op,
        };
        use wgpu::{CompareFunction as C, StencilOperation as S};
        let color = (0..6)
            .map(|mode| {
                [
                    pipeline(
                        "emote color",
                        Some(blend(mode)),
                        true,
                        face(C::Always, S::Keep),
                        false,
                    ),
                    pipeline(
                        "emote masked",
                        Some(blend(mode)),
                        true,
                        face(C::Equal, S::Keep),
                        false,
                    ),
                ]
            })
            .collect();
        let increment = pipeline(
            "emote increment",
            None,
            false,
            face(C::Equal, S::IncrementClamp),
            false,
        );
        let decrement = pipeline(
            "emote decrement",
            None,
            false,
            face(C::Equal, S::DecrementClamp),
            false,
        );
        let fill = pipeline("emote fill", None, false, face(C::Always, S::Replace), true);
        Self {
            device,
            queue,
            layout,
            sampler,
            color,
            increment,
            decrement,
            fill,
            textures: HashMap::new(),
        }
    }

    /// On a device of its own (no window), if there is an adapter.
    pub fn headless() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("silky emote"),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .ok()?;
        Some(Self::new(device, queue))
    }

    /// Uploads the textures of model `id` once.
    pub fn upload(&mut self, id: u64, textures: &HashMap<u32, Texture>) {
        for (&index, t) in textures {
            if self.textures.contains_key(&(id, index)) {
                continue;
            }
            let size = wgpu::Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("emote texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: COLOR,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                texture.as_image_copy(),
                &t.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(t.width * 4),
                    rows_per_image: Some(t.height),
                },
                size,
            );
            let view = texture.create_view(&Default::default());
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("emote texture"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.textures.insert((id, index), group);
        }
    }

    /// Drops the textures of models not in `live`.
    pub fn retain(&mut self, live: &HashSet<u64>) {
        self.textures.retain(|(id, _), _| live.contains(id));
    }

    /// Draws model `id` (uploaded) into a `width` x `height` image,
    /// premultiplied RGBA8.
    pub fn draw_all(
        &mut self,
        id: u64,
        draws: &[Draw],
        at: Placement,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let target = self.target(width, height, wgpu::TextureUsages::COPY_SRC);
        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("emote readback"),
            size: (row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("emote"),
            });
        self.encode(&mut encoder, id, draws, at, &target);
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::Wait);
        let mapped = slice.get_mapped_range();
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            let start = y * row as usize;
            out.extend_from_slice(&mapped[start..start + width as usize * 4]);
        }
        drop(mapped);
        readback.unmap();
        out
    }

    /// A transparent target for [`GpuRaster::encode`], usable as
    /// `usage` too.
    pub fn target(&self, width: u32, height: u32, usage: wgpu::TextureUsages) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("emote target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | usage,
            view_formats: &[],
        })
    }

    /// Records the draws of model `id` (uploaded) into `target` (from
    /// [`GpuRaster::target`]), premultiplied, cleared first.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        id: u64,
        draws: &[Draw],
        at: Placement,
        target: &wgpu::Texture,
    ) {
        let (width, height) = (target.width(), target.height());
        let (vertices, batches) = self.batches(id, draws, at, width, height);
        let stencil = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("emote stencil"),
            size: target.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: STENCIL,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let vertex_buffer = (!vertices.is_empty()).then(|| {
            use wgpu::util::DeviceExt;
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("emote vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let color_view = target.create_view(&Default::default());
        let stencil_view = stencil.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("emote"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Discard,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(buffer) = &vertex_buffer {
            pass.set_vertex_buffer(0, buffer.slice(..));
        }
        for b in &batches {
            let (pipeline, reference) = match b.pass {
                Pass::Fill(r) => (&self.fill, r),
                Pass::Increment(r) => (&self.increment, r),
                Pass::Decrement(r) => (&self.decrement, r),
                Pass::Color(mode, r) => (
                    &self.color[mode as usize][r.is_some() as usize],
                    r.unwrap_or(0),
                ),
            };
            pass.set_pipeline(pipeline);
            pass.set_stencil_reference(reference as u32);
            if let Some(key) = b.texture {
                let Some(group) = self.textures.get(&key) else {
                    continue;
                };
                pass.set_bind_group(0, group, &[]);
            }
            pass.draw(b.range.clone(), 0..1);
        }
    }

    /// The vertices and draw calls of `draws`, in the order of
    /// `raster::Target::draw_all`.
    fn batches(
        &self,
        id: u64,
        draws: &[Draw],
        at: Placement,
        width: u32,
        height: u32,
    ) -> (Vec<GpuVertex>, Vec<Batch>) {
        let mut vertices = Vec::new();
        let mut batches = Vec::new();
        let has = |t: u32| self.textures.contains_key(&(id, t));
        let mut push = |mesh: Option<&Mesh>, pass: Pass, vertices: &mut Vec<GpuVertex>| {
            let start = vertices.len() as u32;
            let texture = match mesh {
                Some(mesh) => {
                    let s = &mesh.style;
                    let (wipe_scale, wipe_bias, wipe) = match s.wipe {
                        Some((a, b)) => (a, b, 1.0),
                        None => (1.0, 0.0, 0.0),
                    };
                    for v in &mesh.vertices {
                        let px = v.x * at.scale + at.dx;
                        let py = v.y * at.scale + at.dy;
                        vertices.push(GpuVertex {
                            clip: [
                                px / width as f32 * 2.0 - 1.0,
                                1.0 - py / height as f32 * 2.0,
                            ],
                            model: [v.x, v.y],
                            uv: [v.u, v.v],
                            color: v.color,
                            clip_rect: s.clip,
                            mode: [s.blend_mode as f32, wipe_scale, wipe_bias, wipe],
                        });
                    }
                    Some((id, s.texture))
                }
                None => {
                    for clip in [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]] {
                        vertices.push(GpuVertex {
                            clip,
                            ..bytemuck::Zeroable::zeroed()
                        });
                    }
                    None
                }
            };
            let end = vertices.len() as u32;
            if end > start {
                batches.push(Batch {
                    pass,
                    texture,
                    range: start..end,
                });
            }
        };
        for draw in draws {
            let style = &draw.mesh.style;
            if !has(style.texture) {
                continue;
            }
            let mode = match style.blend_mode & 0xFF0F {
                m @ 0..=5 => m,
                _ => 0,
            };
            if draw.stencil.is_empty() {
                push(Some(&draw.mesh), Pass::Color(mode, None), &mut vertices);
                continue;
            }
            push(None, Pass::Fill(draw.stencil_initial), &mut vertices);
            let mut reference = draw.stencil_initial;
            for group in draw.stencil.iter().filter(|g| g.phase == 1) {
                for mask in group.masks.iter().filter(|m| has(m.style.texture)) {
                    push(Some(mask), Pass::Increment(reference), &mut vertices);
                }
                reference = reference.saturating_add(1);
            }
            for group in draw.stencil.iter().filter(|g| g.phase == 2) {
                for mask in group.masks.iter().filter(|m| has(m.style.texture)) {
                    push(
                        Some(mask),
                        Pass::Decrement(draw.stencil_final),
                        &mut vertices,
                    );
                }
            }
            push(
                Some(&draw.mesh),
                Pass::Color(mode, Some(draw.stencil_final)),
                &mut vertices,
            );
        }
        (vertices, batches)
    }
}

/// The fixed-function blend of a mode into a premultiplied target (the
/// shader outputs straight color), as `raster::blend`.
fn blend(mode: u32) -> wgpu::BlendState {
    use wgpu::{BlendComponent as B, BlendFactor as F, BlendOperation as O};
    let keep = B {
        src_factor: F::Zero,
        dst_factor: F::One,
        operation: O::Add,
    };
    let color = |src_factor, dst_factor, operation| B {
        src_factor,
        dst_factor,
        operation,
    };
    match mode {
        1 => wgpu::BlendState {
            color: color(F::SrcAlpha, F::One, O::Add),
            alpha: keep,
        },
        2 | 5 => wgpu::BlendState {
            color: color(F::SrcAlpha, F::One, O::ReverseSubtract),
            alpha: keep,
        },
        3 => wgpu::BlendState {
            color: color(F::Dst, F::OneMinusSrcAlpha, O::Add),
            alpha: keep,
        },
        4 => wgpu::BlendState {
            color: color(F::OneMinusDst, F::One, O::Add),
            alpha: keep,
        },
        _ => wgpu::BlendState {
            color: color(F::SrcAlpha, F::OneMinusSrcAlpha, O::Add),
            alpha: color(F::One, F::OneMinusSrcAlpha, O::Add),
        },
    }
}
