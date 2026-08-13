//! Render orchestration: gathers the content painters — rectangles, images,
//! paths, text — and handles **layers** ([`Primitive::Layer`]).
//!
//! A layer is rendered **separately** into a full-surface texture (a pre-pass,
//! submitted on its own so the instance buffers are not aliased), then composited
//! as a single block at its group opacity by the [`CompositePainter`]. Group alpha
//! is therefore correct, with no double-blending where children overlap.
//!
//! Every pipeline is created up front ([`Painters::new`]) then **warmed up**
//! ([`Painters::warm_up`]): the first real frame compiles no shader, so it never
//! stutters.

use crate::batch;
use bytemuck::{Pod, Zeroable};
use frus_core::{BoxFit, Color, ImageData, Path, Point, Primitive, Rect, Scene};
use wgpu::util::DeviceExt;

use crate::image::ImagePainter;
use crate::painter::Painter;
use crate::path::PathPainter;
use crate::text::TextPainter;

/// A vertex of the unit quad, which covers the whole surface.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

impl QuadVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];
const QUAD_VERTEX_COUNT: u32 = 6;

/// A composite instance: the clip, the **inverse** transform (screen → texture)
/// and the group opacity.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CompInstance {
    clip: [f32; 4],
    /// The linear part of the affine inverse: `ia, ib, ic, id`.
    inv_lin: [f32; 4],
    /// `ie, if, opacity, _` — the inverse's translation plus the group opacity.
    inv_tr_opacity: [f32; 4],
    /// The clip shape: `[kind, _, _, _]` (0 = rect, 1 = rrect, 2 = oval).
    shape: [f32; 4],
    /// The corner radii of a rrect: `[tl, tr, br, bl]`.
    radii: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Viewport {
    size: [f32; 2],
    _pad: [f32; 2],
}

/// The MSAA sample count we aim for: 4× is a good quality/cost trade-off and the
/// most widely supported, including the llvmpipe software rasteriser.
pub(crate) const MSAA_SAMPLES: u32 = 4;

/// Returns the MSAA sample count to use for `format` on this adapter:
/// [`MSAA_SAMPLES`] when supported, otherwise 1, which disables MSAA. Called once
/// at init by the windowed renderer and by the offscreen path alike.
pub(crate) fn preferred_sample_count(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> u32 {
    let flags = adapter.get_texture_format_features(format).flags;
    if flags.sample_count_supported(MSAA_SAMPLES) {
        MSAA_SAMPLES
    } else {
        1
    }
}

/// A layer ready to be composited: its texture, opacity and clip, plus the
/// **inverse** transform (screen → texture) to apply when sampling.
struct LayerComposite {
    view: wgpu::TextureView,
    /// The coverage mask (`ClipShape::Path`): the path rendered in white. Other
    /// shapes get a solid 1×1 white texture, a neutral multiplication.
    mask: wgpu::TextureView,
    opacity: f32,
    clip: [f32; 4],
    /// The clip shape: `[kind, _, _, _]` — `kind` 0 = rect, 1 = rrect, 2 = oval,
    /// 3 = path, meaning a mask.
    shape: [f32; 4],
    /// The corner radii of a rrect: `[tl, tr, br, bl]`.
    radii: [f32; 4],
    /// The affine inverse `[ia, ib, ic, id, ie, if]`; identity means no transform.
    inverse: [f32; 6],
}

/// A layer texture **kept across frames** — a repaint boundary on the GPU side —
/// together with a snapshot of its content and its dimensions: as long as those do
/// not change the texture is reused as is, and the pre-pass (submit, tessellation
/// and draw) is skipped entirely.
struct CachedLayer {
    primitives: Vec<Primitive>,
    width: u32,
    height: u32,
    texture: wgpu::Texture,
}

/// An intermediate MSAA texture reused as a render target: painting happens there,
/// multisampled, then **resolves** to the single-sample target. One is enough — the
/// layer pre-passes and the main pass are all full-surface and take turns using it,
/// since their submits are sequential.
struct MsaaScratch {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    texture: wgpu::Texture,
}

/// The layer compositing pipeline.
struct CompositePainter {
    pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    /// The bind groups for this frame's layer textures.
    bind_groups: Vec<wgpu::BindGroup>,
    /// The **neutral** mask, 1×1 opaque white: bound when a layer has no path clip,
    /// so alpha is multiplied by 1 and nothing changes.
    white_mask: wgpu::Texture,
}

impl CompositePainter {
    /// A view of the neutral mask, opaque white, bound to layers with no path clip.
    fn white_mask_view(&self) -> wgpu::TextureView {
        self.white_mask
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

impl CompositePainter {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frus.composite.shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/composite.wgsl").into()),
        });

        let viewport_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.composite.viewport"),
            size: std::mem::size_of::<Viewport>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let viewport_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.composite.viewport.bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                // The fragment also reads `viewport.size`, to counter-rotate a layer.
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frus.composite.viewport.bind_group"),
            layout: &viewport_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.composite.texture.bgl"),
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
                // The path clip mask, sampled in the fragment shader.
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("frus.composite.sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.composite.pipeline_layout"),
            bind_group_layouts: &[&viewport_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.composite.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[QuadVertex::layout(), comp_instance_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frus.composite.quad_vertex_buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.composite.instance_buffer"),
            size: (8 * std::mem::size_of::<CompInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // The neutral mask: 1×1 opaque white, so alpha is 1 everywhere.
        let white = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("frus.composite.white_mask"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255, 255, 255, 255],
        );

        Self {
            pipeline,
            viewport_buffer,
            viewport_bind_group,
            texture_layout,
            sampler,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: 8,
            bind_groups: Vec::new(),
            white_mask: white,
        }
    }

    /// Prepares the compositing of `layers`: instances and bind groups.
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layers: &[LayerComposite],
        w: f32,
        h: f32,
    ) {
        let viewport = Viewport {
            size: [w.max(1.0), h.max(1.0)],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));

        self.bind_groups.clear();
        if layers.is_empty() {
            return;
        }
        let instances: Vec<CompInstance> = layers
            .iter()
            .map(|l| {
                let i = l.inverse;
                CompInstance {
                    clip: l.clip,
                    inv_lin: [i[0], i[1], i[2], i[3]],
                    inv_tr_opacity: [i[4], i[5], l.opacity, 0.0],
                    shape: l.shape,
                    radii: l.radii,
                }
            })
            .collect();
        if instances.len() > self.instance_capacity {
            let cap = instances.len().next_power_of_two();
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frus.composite.instance_buffer"),
                size: (cap * std::mem::size_of::<CompInstance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = cap;
        }
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        for layer in layers {
            self.bind_groups
                .push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("frus.composite.texture.bind_group"),
                    layout: &self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&layer.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&layer.mask),
                        },
                    ],
                }));
        }
    }

    fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.bind_groups.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        for (i, bind_group) in self.bind_groups.iter().enumerate() {
            pass.set_bind_group(1, bind_group, &[]);
            let inst = i as u32;
            pass.draw(0..QUAD_VERTEX_COUNT, inst..inst + 1);
        }
    }
}

fn comp_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<CompInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRS,
    }
}

/// The full set of content painters, plus layer compositing.
pub(crate) struct Painters {
    rect: Painter,
    image: ImagePainter,
    path: PathPainter,
    text: TextPainter,
    composite: CompositePainter,
    /// The MSAA sample count; 1 means no multisampling.
    sample_count: u32,
    /// The intermediate MSAA texture, created on demand and recreated on resize.
    msaa: Option<MsaaScratch>,
    /// Layer textures kept across frames, indexed by the layer's rank.
    layer_cache: Vec<CachedLayer>,
    /// How many batches the planner produced since creation — the batching's proof:
    /// a frame of a real screen should be a handful, not one per widget.
    batch_count: u64,
    /// How many layer pre-passes were actually rendered — the cache's proof.
    #[allow(dead_code)]
    layer_renders: u64,
}

impl Painters {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        Self {
            batch_count: 0,
            rect: Painter::new(device, format, sample_count),
            image: ImagePainter::new(device, format, sample_count),
            path: PathPainter::new(device, format, sample_count),
            text: TextPainter::new(device, queue, format, sample_count),
            composite: CompositePainter::new(device, queue, format, sample_count),
            sample_count,
            msaa: None,
            layer_cache: Vec::new(),
            layer_renders: 0,
        }
    }

    /// Batches planned since creation, for the batching test.
    pub(crate) fn batch_count(&self) -> u64 {
        self.batch_count
    }

    /// Layer pre-passes rendered since creation, for the cache test.
    #[cfg(test)]
    pub(crate) fn layer_render_count(&self) -> u64 {
        self.layer_renders
    }

    /// Returns a fresh view of the MSAA texture, recreating it when the size or the
    /// format changes, or `None` when MSAA is off (`sample_count == 1`). The view is
    /// created on the fly — it is cheap — and returned by value, which releases the
    /// borrow of `self` before the render pass opens.
    fn ensure_msaa(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        w: u32,
        h: u32,
    ) -> Option<wgpu::TextureView> {
        if self.sample_count == 1 {
            return None;
        }
        let stale = match &self.msaa {
            Some(s) => s.width != w || s.height != h || s.format != format,
            None => true,
        };
        if stale {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("frus.msaa.scratch"),
                size: wgpu::Extent3d {
                    width: w.max(1),
                    height: h.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa = Some(MsaaScratch {
                width: w,
                height: h,
                format,
                texture,
            });
        }
        self.msaa.as_ref().map(|s| {
            s.texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        })
    }

    fn set_viewport(&self, queue: &wgpu::Queue, w: f32, h: f32) {
        self.rect.set_viewport(queue, w, h);
        self.image.set_viewport(queue, w, h);
        self.path.set_viewport(queue, w, h);
    }

    /// Renders `scene`, layers included, into `target` of size `w×h`. `clear` is
    /// `Some(colour)` to clear the target, `None` to paint over it. **Submitted
    /// internally**: one pass per layer, plus the main pass.
    ///
    /// Note: under MSAA, `clear == None` — painting over — is not supported, since
    /// the multisampled target does not hold `target`'s existing content. Every
    /// current caller passes `Some(_)`.
    pub(crate) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        target: &wgpu::TextureView,
        w: u32,
        h: u32,
        scene: &Scene,
        clear: Option<wgpu::Color>,
    ) {
        self.set_viewport(queue, w as f32, h as f32);

        // Pre-passes: each layer into its own full-surface texture — but **reused**
        // as is when its content has not changed, which is the GPU-side cache.
        let mut layers: Vec<LayerComposite> = Vec::new();
        let mut layer_index = 0usize;
        for primitive in scene.primitives() {
            if let Primitive::Layer {
                primitives,
                opacity,
                clip,
                clip_shape,
                transform,
                ..
            } = primitive
            {
                let view = self.layer_texture(device, queue, format, layer_index, primitives, w, h);
                // The fragment samples at the **counter-transformed** position, so
                // we pass the inverse (screen → texture); identity when there is none.
                let inverse = match transform {
                    Some(t) => t.affine.inverse().m,
                    None => frus_core::Affine::IDENTITY.m,
                };
                // The clip shape: (kind, per-corner radii) — an SDF in the fragment
                // shader for rect/rrect/oval, a rendered **mask** for a free path.
                let (shape, radii) = match clip_shape {
                    frus_core::ClipShape::Rect => ([0.0, 0.0, 0.0, 0.0], [0.0; 4]),
                    frus_core::ClipShape::RRect(br) => ([1.0, 0.0, 0.0, 0.0], br.to_array()),
                    frus_core::ClipShape::Oval => ([2.0, 0.0, 0.0, 0.0], [0.0; 4]),
                    frus_core::ClipShape::Path(_) => ([3.0, 0.0, 0.0, 0.0], [0.0; 4]),
                };
                let mask = match clip_shape {
                    frus_core::ClipShape::Path(path) => {
                        self.render_mask(device, queue, format, path, w, h)
                    }
                    _ => self.composite.white_mask_view(),
                };
                layers.push(LayerComposite {
                    view,
                    mask,
                    opacity: *opacity,
                    clip: clip.to_array(),
                    shape,
                    radii,
                    inverse,
                });
                layer_index += 1;
            }
        }
        // Forget vanished layers: the scene has fewer than it had last frame.
        self.layer_cache.truncate(layer_index);

        self.composite
            .prepare(device, queue, &layers, w as f32, h as f32);
        let decorations = self.text.prepare_frame(device, queue, scene, w, h);
        // What may share a draw call, and in what order — see `batch`. Rectangles,
        // images and paths are interleaved by the scene's order wherever they cover
        // one another; text keeps its own pass above them.
        let batches = batch::plan(scene);
        let (rect_ranges, decoration_range) =
            self.rect
                .prepare_frame(device, queue, scene, &decorations, &batches);
        let image_ranges = self.image.prepare_frame(device, queue, scene, &batches);
        let path_ranges = self.path.prepare_frame(device, queue, scene, &batches);
        self.batch_count += batches.len() as u64;

        // With MSAA we paint into the multisampled texture then resolve to `target`;
        // without it we paint straight into `target`.
        let msaa_view = self.ensure_msaa(device, format, w, h);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.encoder"),
        });
        {
            let load = match clear {
                Some(c) => wgpu::LoadOp::Clear(c),
                None => wgpu::LoadOp::Load,
            };
            let (view, resolve_target) = match &msaa_view {
                Some(msaa) => (msaa, Some(target)),
                None => (target, None),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for (i, batch) in batches.iter().enumerate() {
                match batch.kind {
                    batch::Kind::Rect => self.rect.draw(&mut pass, rect_ranges[i].clone()),
                    batch::Kind::Image => self.image.draw(&mut pass, image_ranges[i].clone()),
                    batch::Kind::Path => self.path.draw(&mut pass, path_ranges[i].clone()),
                }
            }
            // The decoration quads go with the text they underline, not with the
            // rectangles they happen to be made of.
            self.rect.draw(&mut pass, decoration_range);
            self.text.draw(&mut pass);
            self.composite.draw(&mut pass);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Returns the view of a layer's texture: **reused** as is when its content and
    /// dimensions are unchanged since the previous frame, otherwise (re)rendered by
    /// [`Painters::render_group`]. `index` is the layer's rank in the scene, a stable
    /// cache key; a key that slips — because layers were reordered — only misses the
    /// cache, which re-renders correctly and never wrongly.
    fn layer_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        index: usize,
        primitives: &[Primitive],
        w: u32,
        h: u32,
    ) -> wgpu::TextureView {
        let hit = matches!(
            self.layer_cache.get(index),
            Some(c) if c.width == w && c.height == h && c.primitives.as_slice() == primitives
        );
        if !hit {
            let texture = self.render_group(device, queue, format, primitives, w, h);
            let entry = CachedLayer {
                primitives: primitives.to_vec(),
                width: w,
                height: h,
                texture,
            };
            if index < self.layer_cache.len() {
                self.layer_cache[index] = entry;
            } else {
                self.layer_cache.push(entry);
            }
        }
        self.layer_cache[index]
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Renders a **clip mask**: `path`, in absolute screen coordinates, filled with
    /// opaque white on a transparent background, at the surface's size. The returned
    /// view keeps the texture alive, since wgpu counts references.
    fn render_mask(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        path: &frus_core::Path,
        w: u32,
        h: u32,
    ) -> wgpu::TextureView {
        let prim = Primitive::Path {
            path: path.clone(),
            fill: Some(frus_core::Color::WHITE),
            stroke: None,
            clip: frus_core::Rect::UNBOUNDED,
            owner: 0,
        };
        let tex = self.render_group(device, queue, format, &[prim], w, h);
        tex.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Renders a group of primitives into a full-surface texture with a transparent
    /// background, for later compositing. **Nested** layers are not re-composited at
    /// this level, an accepted limitation.
    fn render_group(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        primitives: &[Primitive],
        w: u32,
        h: u32,
    ) -> wgpu::Texture {
        self.layer_renders += 1;
        let mut sub = Scene::new();
        for primitive in primitives {
            sub.push_primitive(primitive.clone());
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.layer.texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let decorations = self.text.prepare_frame(device, queue, &sub, w, h);
        let batches = batch::plan(&sub);
        let (rect_ranges, decoration_range) =
            self.rect
                .prepare_frame(device, queue, &sub, &decorations, &batches);
        let image_ranges = self.image.prepare_frame(device, queue, &sub, &batches);
        let path_ranges = self.path.prepare_frame(device, queue, &sub, &batches);
        self.batch_count += batches.len() as u64;

        // With MSAA the layer is painted multisampled then resolved to its
        // single-sample texture, the one the compositor samples afterwards.
        let msaa_view = self.ensure_msaa(device, format, w, h);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.layer.encoder"),
        });
        {
            let (attachment, resolve_target) = match &msaa_view {
                Some(msaa) => (msaa, Some(&view)),
                None => (&view, None),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.layer.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: attachment,
                    resolve_target,
                    ops: wgpu::Operations {
                        // Transparent background: a layer covers only its primitives.
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for (i, batch) in batches.iter().enumerate() {
                match batch.kind {
                    batch::Kind::Rect => self.rect.draw(&mut pass, rect_ranges[i].clone()),
                    batch::Kind::Image => self.image.draw(&mut pass, image_ranges[i].clone()),
                    batch::Kind::Path => self.path.draw(&mut pass, path_ranges[i].clone()),
                }
            }
            self.rect.draw(&mut pass, decoration_range);
            self.text.draw(&mut pass);
        }
        queue.submit(std::iter::once(encoder.finish()));
        texture
    }

    /// **Warms up** every pipeline by rendering a small scene that exercises each
    /// render path — rectangle, image, path, text, layer → composite — so the first
    /// real frame compiles nothing.
    pub(crate) fn warm_up(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) {
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color::WHITE);
        scene.fill_path(&Path::rect(Rect::new(0.0, 0.0, 2.0, 2.0)), Color::WHITE);
        let img = ImageData::from_rgba(1, 1, vec![255, 255, 255, 255]).into_handle();
        scene.image(&img, Rect::new(0.0, 0.0, 2.0, 2.0), BoxFit::Fill);
        scene.text(Point::new(0.0, 0.0), "x", 8.0, Color::WHITE);
        scene.layer(0.5, |inner| {
            inner.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color::WHITE)
        });

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.warmup.texture"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render(
            device,
            queue,
            format,
            &view,
            4,
            4,
            &scene,
            Some(wgpu::Color::BLACK),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headless() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("frus.compositor.test.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .ok()
    }

    fn target(device: &wgpu::Device, format: wgpu::TextureFormat, n: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.compositor.test.target"),
            size: wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    /// A **static** layer is rendered once: the second frame reuses its texture, with
    /// no new pre-pass. Changing its content forces a re-render; removing it purges
    /// the cache.
    #[test]
    fn static_layer_texture_is_reused_across_frames() {
        let Some((device, queue)) = headless() else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        // sample_count 1: the cache does not depend on MSAA.
        let mut painters = Painters::new(&device, &queue, format, 1);
        let tex = target(&device, format, 16);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let clear = Some(wgpu::Color::BLACK);

        let red =
            |s: &mut Scene| s.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(1.0, 0.0, 0.0));
        let mut scene = Scene::new();
        scene.layer(0.5, red);

        painters.render(&device, &queue, format, &view, 16, 16, &scene, clear);
        assert_eq!(
            painters.layer_render_count(),
            1,
            "first frame: layer rendered"
        );
        painters.render(&device, &queue, format, &view, 16, 16, &scene, clear);
        assert_eq!(
            painters.layer_render_count(),
            1,
            "layer unchanged: texture reused"
        );

        // Content changed → re-render.
        let mut scene2 = Scene::new();
        scene2.layer(0.5, |s| {
            s.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(0.0, 1.0, 0.0))
        });
        painters.render(&device, &queue, format, &view, 16, 16, &scene2, clear);
        assert_eq!(
            painters.layer_render_count(),
            2,
            "content changed: re-render"
        );

        // Layer gone → the cache is purged, with no pre-pass.
        painters.render(&device, &queue, format, &view, 16, 16, &Scene::new(), clear);
        assert_eq!(
            painters.layer_render_count(),
            2,
            "no layer left: nothing to render"
        );
        assert!(painters.layer_cache.is_empty(), "cache purged");
    }
}
