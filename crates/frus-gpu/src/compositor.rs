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
use frus_core::{
    BoxFit, Color, ColorFilter, ImageData, ImageFilter, LayerFilter, MaskShader, Path, Point,
    Primitive, Rect, Scene,
};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

use crate::filter::FilterPainter;
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

/// A layer's colour filter and mask, as the fragment shader reads them.
///
/// This rides in a **uniform** rather than in the instance, because a colour matrix
/// alone is twenty floats and a vertex attribute budget is a small, fixed thing. Each
/// layer already has its own bind group for its texture, so a slice of one shared
/// buffer costs nothing extra.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct FilterParams {
    /// `[colour kind, mask kind, colour blend, mask blend]`. Colour kind: 0 none,
    /// 1 matrix, 2 a colour blended in. Mask kind: 0 none, 1 linear, 2 radial.
    flags: [f32; 4],
    /// The colour matrix, one row per output channel, then the constant column.
    rows: [[f32; 4]; 4],
    constants: [f32; 4],
    /// The colour of a `Mode` colour filter.
    mode_color: [f32; 4],
    /// Linear: `(from.x, from.y, to.x, to.y)`. Radial: `(cx, cy, radius, _)`.
    mask_geom: [f32; 4],
    mask_c0: [f32; 4],
    mask_c1: [f32; 4],
}

impl FilterParams {
    /// Uniform bindings must start at a device-aligned offset; 256 bytes is the
    /// strictest alignment in practice, so the layers take one slot each.
    const STRIDE: wgpu::BufferAddress = 256;

    /// Translates a scene-level filter into the numbers the shader wants. The image
    /// filter is not here: it has already run, as a pre-pass over the layer texture.
    fn new(filter: &LayerFilter) -> FilterParams {
        let mut p = FilterParams::default();
        match filter.color {
            None => {}
            Some(ColorFilter::Matrix(m)) => {
                p.flags[0] = 1.0;
                for row in 0..4 {
                    p.rows[row] = [m[row * 5], m[row * 5 + 1], m[row * 5 + 2], m[row * 5 + 3]];
                    p.constants[row] = m[row * 5 + 4];
                }
            }
            Some(ColorFilter::Mode(color, mode)) => {
                p.flags[0] = 2.0;
                p.flags[2] = mode.code() as f32;
                p.mode_color = color.to_array();
            }
        }
        if let Some(mask) = filter.mask {
            p.flags[1] = mask.shader.code() as f32 + 1.0;
            p.flags[3] = mask.blend.code() as f32;
            let (geom, c0, c1) = match mask.shader {
                MaskShader::Linear {
                    from,
                    to,
                    from_color,
                    to_color,
                } => ([from.x, from.y, to.x, to.y], from_color, to_color),
                MaskShader::Radial {
                    center,
                    radius,
                    from_color,
                    to_color,
                } => ([center.x, center.y, radius, 0.0], from_color, to_color),
            };
            p.mask_geom = geom;
            p.mask_c0 = c0.to_array();
            p.mask_c1 = c1.to_array();
        }
        p
    }
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
    /// The colour filter and mask, applied in the compositing fragment.
    filter: LayerFilter,
}

/// A backdrop waiting for the frame underneath it to exist: where in the draw list
/// it sits, what to do to the pixels, and whether it shares its filtered copy.
struct BackdropDraw {
    /// Its index in the composite draw list. Everything before it is what it filters.
    at: usize,
    /// The scene index of the layer it belongs to, which is how the frame is cut: the
    /// copy is taken just before the **batch** that draws that layer.
    scene: usize,
    filter: ImageFilter,
    /// `true` for [`frus_core::BlendMode::Src`]: the filtered copy **replaces** what
    /// it was copied from rather than being painted over it.
    replace: bool,
    /// The sharing key, when several backdrops are filtered once between them.
    key: Option<u64>,
}

/// This frame's draws, prepared once and replayed by whichever pass draws them.
///
/// The batches are the **whole** order of the frame, layers included: a layer is one
/// batch of its own, holding the scene index of the group, and `layer_draws` says which
/// composite draws that batch issues — two when the layer takes a backdrop (the filtered
/// copy, then the layer over it), one otherwise.
struct ContentPlan<'a> {
    batches: &'a [batch::Batch],
    rect_ranges: &'a [std::ops::Range<u32>],
    image_ranges: &'a [std::ops::Range<u32>],
    path_ranges: &'a [std::ops::Range<u32>],
    decoration_ranges: &'a [std::ops::Range<u32>],
    decoration_base: u32,
    /// Scene index of a layer → the composite draws that put it on the screen.
    layer_draws: &'a HashMap<usize, std::ops::Range<usize>>,
    /// How many text batches precede each batch, so a pass over a *range* of batches
    /// still hands the text painter the right renderer.
    text_slots: &'a [usize],
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
    /// The image filter already baked into `texture`. Part of the key: the same
    /// primitives blurred by a different amount are a different picture, and an
    /// animated blur would otherwise hold the first frame it drew for ever.
    image: Option<ImageFilter>,
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
    /// The same pipeline with blending off, so a draw can **replace** what is under
    /// it instead of painting over it. Only a backdrop asks for that
    /// ([`frus_core::BlendMode::Src`]).
    pipeline_src: wgpu::RenderPipeline,
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
    /// This frame's filter parameters, one aligned slot per layer.
    filter_buffer: wgpu::Buffer,
    filter_capacity: usize,
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
                // The colour filter and mask.
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
                    // **Premultiplied**, unlike every other pipeline here, because
                    // what this one samples is a layer texture and a layer texture is
                    // premultiplied: it was painted over a transparent target. Passing
                    // it through the straight-alpha blend would multiply the colour by
                    // the coverage a second time, which is invisible on opaque content
                    // and wrong everywhere else — a mask fade would run to black
                    // instead of to the background.
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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

        let pipeline_src = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.composite.pipeline.src"),
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
                    // No blending at all: the source is written as it stands.
                    blend: None,
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

        let filter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.composite.filter_buffer"),
            size: FilterParams::STRIDE * 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
            pipeline_src,
            viewport_buffer,
            viewport_bind_group,
            texture_layout,
            sampler,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: 8,
            bind_groups: Vec::new(),
            white_mask: white,
            filter_buffer,
            filter_capacity: 8,
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

        if layers.len() > self.filter_capacity {
            let cap = layers.len().next_power_of_two();
            self.filter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frus.composite.filter_buffer"),
                size: FilterParams::STRIDE * cap as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.filter_capacity = cap;
        }
        for (i, layer) in layers.iter().enumerate() {
            let params = FilterParams::new(&layer.filter);
            queue.write_buffer(
                &self.filter_buffer,
                i as wgpu::BufferAddress * FilterParams::STRIDE,
                bytemuck::bytes_of(&params),
            );
        }

        for (i, layer) in layers.iter().enumerate() {
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
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &self.filter_buffer,
                                offset: i as wgpu::BufferAddress * FilterParams::STRIDE,
                                size: wgpu::BufferSize::new(
                                    std::mem::size_of::<FilterParams>() as u64
                                ),
                            }),
                        },
                    ],
                }));
        }
    }

    /// Points draw `index` at a texture that did not exist when the frame was
    /// prepared — which is every backdrop, since what a backdrop filters is the frame
    /// itself and the frame is drawn after this.
    fn rebind(&mut self, device: &wgpu::Device, index: usize, view: &wgpu::TextureView) {
        let white = self.white_mask_view();
        self.bind_groups[index] = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frus.composite.texture.bind_group.backdrop"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&white),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.filter_buffer,
                        offset: index as wgpu::BufferAddress * FilterParams::STRIDE,
                        size: wgpu::BufferSize::new(std::mem::size_of::<FilterParams>() as u64),
                    }),
                },
            ],
        });
    }

    /// Draws a **range** of the composite list. `replace_at` names the one draw that
    /// writes instead of blending, which only a backdrop asks for.
    fn draw_range<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        range: std::ops::Range<usize>,
        replace_at: Option<usize>,
    ) {
        if range.is_empty() || self.bind_groups.is_empty() {
            return;
        }
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        // `None` rather than `Some(false)`, so the first draw always sets a pipeline.
        let mut bound: Option<bool> = None;
        for i in range {
            let replace = replace_at == Some(i);
            if bound != Some(replace) {
                pass.set_pipeline(if replace {
                    &self.pipeline_src
                } else {
                    &self.pipeline
                });
                bound = Some(replace);
            }
            pass.set_bind_group(1, &self.bind_groups[i], &[]);
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
    /// The separable image-filter pre-pass: blur, dilate, erode.
    filter: FilterPainter,
    /// The MSAA sample count; 1 means no multisampling.
    sample_count: u32,
    /// The intermediate MSAA texture, created on demand and recreated on resize.
    msaa: Option<MsaaScratch>,
    /// The staging texture a frame **with backdrops** is built in, so that a backdrop
    /// can read the frame so far. `None` until the first such frame.
    stage: Option<MsaaScratch>,
    /// Layer textures kept across frames, indexed by the layer's rank.
    layer_cache: Vec<CachedLayer>,
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
            rect: Painter::new(device, format, sample_count),
            image: ImagePainter::new(device, format, sample_count),
            path: PathPainter::new(device, format, sample_count),
            text: TextPainter::new(device, queue, format, sample_count),
            composite: CompositePainter::new(device, queue, format, sample_count),
            filter: FilterPainter::new(device, format),
            sample_count,
            msaa: None,
            stage: None,
            layer_cache: Vec::new(),
            layer_renders: 0,
        }
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
    /// internally**: one pass per layer, plus the main pass — and, when a layer asks
    /// for a backdrop, one more pass per backdrop.
    ///
    /// Note: under MSAA, `clear == None` — painting over — is not supported, since
    /// the multisampled target does not hold `target`'s existing content. Every
    /// current caller passes `Some(_)`.
    // The device, the queue, the target and its size: a render call's irreducible
    // arguments.
    #[allow(clippy::too_many_arguments)]
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
        //
        // The result is a flat list of composite draws in the order they happen. A
        // layer that asks for a backdrop contributes **two** entries: the filtered
        // copy of what is underneath, and then the layer itself over it.
        let mut draws: Vec<LayerComposite> = Vec::new();
        let mut backdrops: Vec<BackdropDraw> = Vec::new();
        // Which composite draws each layer issues, by its index in the scene. The batch
        // planner orders layers among the content, and this is how a layer batch finds
        // the draws that belong to it.
        let mut layer_draws: HashMap<usize, std::ops::Range<usize>> = HashMap::new();
        let mut layer_index = 0usize;
        for (scene_index, primitive) in scene.primitives().iter().enumerate() {
            if let Primitive::Layer {
                primitives,
                opacity,
                clip,
                clip_shape,
                transform,
                filter,
                ..
            } = primitive
            {
                let view = self.layer_texture(
                    device,
                    queue,
                    format,
                    layer_index,
                    primitives,
                    filter.image.filter(|f| !f.is_identity()),
                    w,
                    h,
                );
                let entry = self.layer_entry(
                    device, queue, format, view, *opacity, *clip, clip_shape, transform, filter, w,
                    h,
                );
                let first_draw = draws.len();
                // The backdrop goes **first**: it is a picture of what was already
                // there, and the layer is painted over it. It borrows the layer's clip
                // and shape, which is the region it applies to.
                if let Some(backdrop) = filter.backdrop {
                    backdrops.push(BackdropDraw {
                        at: draws.len(),
                        scene: scene_index,
                        filter: backdrop.filter,
                        replace: backdrop.blend == frus_core::BlendMode::Src,
                        key: backdrop.key,
                    });
                    draws.push(LayerComposite {
                        // Filled in once the frame so far has actually been drawn;
                        // until then any texture will do, since nothing samples it.
                        view: self.composite.white_mask_view(),
                        mask: self.composite.white_mask_view(),
                        opacity: 1.0,
                        clip: entry.clip,
                        shape: entry.shape,
                        radii: entry.radii,
                        // The copy is of the screen, at the screen's own coordinates.
                        inverse: frus_core::Affine::IDENTITY.m,
                        filter: LayerFilter::NONE,
                    });
                }
                draws.push(entry);
                layer_draws.insert(scene_index, first_draw..draws.len());
                layer_index += 1;
            }
        }
        // Forget vanished layers: the scene has fewer than it had last frame.
        self.layer_cache.truncate(layer_index);

        self.composite
            .prepare(device, queue, &draws, w as f32, h as f32);
        // What may share a draw call, and in what order — see `batch`. The plan comes
        // first now: text is interleaved with the rest, so the text painter needs to
        // know the batches before it can prepare a renderer for each.
        let batches = batch::plan(scene);
        let (decorations, decoration_ranges) = self
            .text
            .prepare_frame(device, queue, scene, w, h, &batches);
        let (rect_ranges, decoration_base) =
            self.rect
                .prepare_frame(device, queue, scene, &decorations, &batches);
        let image_ranges = self.image.prepare_frame(device, queue, scene, &batches);
        let path_ranges = self.path.prepare_frame(device, queue, scene, &batches);
        // The text painter keeps one renderer per text batch, in order; a pass drawing
        // batches `a..b` needs to know how many of them came before `a`.
        let mut text_slots = Vec::with_capacity(batches.len());
        let mut slots = 0usize;
        for batch in &batches {
            text_slots.push(slots);
            if batch.kind == batch::Kind::Text {
                slots += 1;
            }
        }
        let content = ContentPlan {
            batches: &batches,
            rect_ranges: &rect_ranges,
            image_ranges: &image_ranges,
            path_ranges: &path_ranges,
            decoration_ranges: &decoration_ranges,
            decoration_base: decoration_base.start,
            layer_draws: &layer_draws,
            text_slots: &text_slots,
        };

        // With MSAA we paint into the multisampled texture then resolve; without it
        // we paint straight into whatever the pass is aimed at.
        let msaa_view = self.ensure_msaa(device, format, w, h);
        let load = match clear {
            Some(c) => wgpu::LoadOp::Clear(c),
            None => wgpu::LoadOp::Load,
        };

        if backdrops.is_empty() {
            // The ordinary frame: one pass, straight into the target, exactly as it
            // was before backdrops existed.
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frus.encoder"),
            });
            self.pass(
                &mut encoder,
                msaa_view.as_ref(),
                target,
                load,
                &content,
                0..batches.len(),
                None,
            );
            queue.submit(std::iter::once(encoder.finish()));
            return;
        }

        // A backdrop is a filter of **what is already painted**, so the frame has to
        // be readable half-way through. It cannot be read out of `target`: a surface
        // texture is a render attachment and nothing else. So the frame is built in a
        // staging texture we own, cut into segments at each backdrop, and blitted to
        // the target at the end.
        let stage = self
            .stage(device, format, w, h)
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Backdrops sharing a key are filtered **once**: the copy is taken before the
        // first of them and every other one reads the same texture. That is what
        // turns a list of frosted rows from one full-surface blur per row into one.
        let mut shared: HashMap<u64, wgpu::Texture> = HashMap::new();
        // The frame is cut at the **batch** that draws the backdrop's layer, because
        // that is where "what is already painted" ends now that layers are ordered
        // among the content rather than after it.
        let cut_at = |scene: usize| {
            batches
                .iter()
                .position(|b| b.kind == batch::Kind::Layer && b.members.first() == Some(&scene))
                .unwrap_or(batches.len())
        };
        let mut cursor = 0usize;
        let mut first = true;
        // Every segment after the first **starts** with a backdrop — that is what ends
        // the segment before it — so a segment has at most one, and this is it.
        let mut replace_at: Option<usize> = None;
        for backdrop in &backdrops {
            {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frus.encoder.segment"),
                });
                self.pass(
                    &mut encoder,
                    msaa_view.as_ref(),
                    &stage,
                    if first { load } else { wgpu::LoadOp::Load },
                    &content,
                    cursor..cut_at(backdrop.scene),
                    replace_at,
                );
                // Submitted here and not at the end: the filter below reads the stage,
                // and a command buffer still being recorded has not written it yet.
                queue.submit(std::iter::once(encoder.finish()));
            }
            first = false;

            let reuse = backdrop
                .key
                .and_then(|k| shared.get(&k))
                .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
            let view = match reuse {
                Some(view) => view,
                None => {
                    // Two disjoint fields of `self`: the stage is read, the filter
                    // painter is driven. Naming them separately is what lets the
                    // borrow checker see that.
                    let stage_texture = &self
                        .stage
                        .as_ref()
                        .expect("the stage exists whenever a backdrop is drawn")
                        .texture;
                    let filtered = self.filter.apply(
                        device,
                        queue,
                        format,
                        stage_texture,
                        backdrop.filter,
                        w,
                        h,
                    );
                    let view = filtered.create_view(&wgpu::TextureViewDescriptor::default());
                    if let Some(k) = backdrop.key {
                        shared.insert(k, filtered);
                    }
                    view
                }
            };
            self.composite.rebind(device, backdrop.at, &view);
            cursor = cut_at(backdrop.scene);
            replace_at = backdrop.replace.then_some(backdrop.at);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.encoder.tail"),
        });
        self.pass(
            &mut encoder,
            msaa_view.as_ref(),
            &stage,
            wgpu::LoadOp::Load,
            &content,
            cursor..batches.len(),
            replace_at,
        );
        // And the staged frame onto the target it was always meant for.
        self.filter
            .blit(device, queue, &mut encoder, &stage, target);
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Turns one [`Primitive::Layer`] into the composite draw that puts it on the
    /// screen: its clip shape as an SDF kind plus radii, or a rendered coverage mask
    /// for a free path, and the **inverse** of its transform, since the fragment
    /// samples at the counter-transformed position.
    ///
    /// Shared by the top-level frame and by a group compositing the layers nested
    /// inside it: the two ask exactly the same question.
    #[allow(clippy::too_many_arguments)]
    fn layer_entry(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        view: wgpu::TextureView,
        opacity: f32,
        clip: Rect,
        clip_shape: &frus_core::ClipShape,
        transform: &Option<frus_core::LayerTransform>,
        filter: &LayerFilter,
        w: u32,
        h: u32,
    ) -> LayerComposite {
        let inverse = match transform {
            Some(t) => t.affine.inverse().m,
            None => frus_core::Affine::IDENTITY.m,
        };
        let (shape, radii) = match clip_shape {
            frus_core::ClipShape::Rect => ([0.0, 0.0, 0.0, 0.0], [0.0; 4]),
            frus_core::ClipShape::RRect(br) => ([1.0, 0.0, 0.0, 0.0], br.to_array()),
            frus_core::ClipShape::Oval => ([2.0, 0.0, 0.0, 0.0], [0.0; 4]),
            frus_core::ClipShape::Path(_) => ([3.0, 0.0, 0.0, 0.0], [0.0; 4]),
        };
        let mask = match clip_shape {
            frus_core::ClipShape::Path(path) => self.render_mask(device, queue, format, path, w, h),
            _ => self.composite.white_mask_view(),
        };
        LayerComposite {
            view,
            mask,
            opacity,
            clip: clip.to_array(),
            shape,
            radii,
            inverse,
            // The image filter has already been spent, in the pre-pass; the backdrop
            // is a draw of its own.
            filter: LayerFilter {
                image: None,
                backdrop: None,
                ..*filter
            },
        }
    }

    /// One render pass: the content batches (when `content` is given — only the first
    /// pass of a frame draws them) followed by a range of composite draws.
    ///
    /// `replace_at` names a draw that **replaces** rather than paints over, which only
    /// a backdrop asks for.
    // A pass is its attachment, its load, and what to put in it; the arguments are the
    // pass.
    #[allow(clippy::too_many_arguments)]
    /// One render pass over a **range of batches**, in order.
    ///
    /// The order is the whole point. A layer is composited from a texture of its own,
    /// and for a long time every layer was composited after all of the content — so a
    /// group that something had covered came back on top of it. A device found it as the
    /// home screen's translucent square painted over the Kanban board (milestone 349).
    /// The batches now hold the layers too, in the order they must be drawn, and this
    /// walks them: content pipelines and composite draws interleaved.
    fn pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        msaa_view: Option<&wgpu::TextureView>,
        target: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
        content: &ContentPlan,
        batches: std::ops::Range<usize>,
        replace_at: Option<usize>,
    ) {
        // An empty range still has to run when the pass is the one that clears: a frame
        // with nothing in it is a frame of the clear colour, not a frame of whatever was
        // in the buffer before.
        if batches.is_empty() && !matches!(load, wgpu::LoadOp::Clear(_)) {
            return;
        }
        let (view, resolve_target) = match msaa_view {
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
        for i in batches {
            let batch = &content.batches[i];
            match batch.kind {
                batch::Kind::Rect => self.rect.draw(&mut pass, content.rect_ranges[i].clone()),
                batch::Kind::Image => self.image.draw(&mut pass, content.image_ranges[i].clone()),
                batch::Kind::Path => self.path.draw(&mut pass, content.path_ranges[i].clone()),
                batch::Kind::Text => {
                    // The underlines first — they are rectangles, and they belong
                    // beneath the glyphs they decorate rather than in a batch of
                    // their own.
                    let base = content.decoration_base;
                    let d = &content.decoration_ranges[i];
                    self.rect.draw(&mut pass, base + d.start..base + d.end);
                    self.text.draw(&mut pass, content.text_slots[i]);
                }
                batch::Kind::Layer => {
                    // One member, always: the scene index of the group. Its draws are
                    // the layer itself, preceded by its backdrop when it takes one.
                    if let Some(range) = batch
                        .members
                        .first()
                        .and_then(|scene| content.layer_draws.get(scene))
                    {
                        self.composite
                            .draw_range(&mut pass, range.clone(), replace_at);
                    }
                }
            }
        }
    }

    /// The staging texture a frame with backdrops is built in, created on demand and
    /// recreated on resize. It exists because a backdrop has to *read* the frame so
    /// far, and a surface texture cannot be read.
    fn stage(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        w: u32,
        h: u32,
    ) -> &wgpu::Texture {
        let stale = match &self.stage {
            Some(s) => s.width != w || s.height != h || s.format != format,
            None => true,
        };
        if stale {
            self.stage = Some(MsaaScratch {
                width: w,
                height: h,
                format,
                texture: device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("frus.stage"),
                    size: wgpu::Extent3d {
                        width: w.max(1),
                        height: h.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                }),
            });
        }
        &self.stage.as_ref().expect("just created").texture
    }

    /// Returns the view of a layer's texture: **reused** as is when its content and
    /// dimensions are unchanged since the previous frame, otherwise (re)rendered by
    /// [`Painters::render_group`] and, if the layer asks for one, put through the
    /// image-filter pre-pass. `index` is the layer's rank in the scene, a stable
    /// cache key; a key that slips — because layers were reordered — only misses the
    /// cache, which re-renders correctly and never wrongly.
    // Same as `render`, one layer at a time.
    #[allow(clippy::too_many_arguments)]
    fn layer_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        index: usize,
        primitives: &[Primitive],
        image: Option<ImageFilter>,
        w: u32,
        h: u32,
    ) -> wgpu::TextureView {
        let hit = matches!(
            self.layer_cache.get(index),
            Some(c) if c.width == w && c.height == h && c.image == image
                && c.primitives.as_slice() == primitives
        );
        if !hit {
            let mut texture = self.render_group(device, queue, format, primitives, w, h);
            if let Some(f) = image {
                texture = self.filter.apply(device, queue, format, &texture, f, w, h);
            }
            let entry = CachedLayer {
                primitives: primitives.to_vec(),
                width: w,
                height: h,
                texture,
                image,
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
            // A coverage mask is flat by definition.
            gradient: None,
            stroke: None,
            clip: frus_core::Rect::UNBOUNDED,
            owner: 0,
        };
        let tex = self.render_group(device, queue, format, &[prim], w, h);
        tex.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Renders a group of primitives into a full-surface texture with a transparent
    /// background, for later compositing.
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

        // **Nested** layers first, depth-first: each rendered into a texture of its
        // own so that this group's pass can composite it. Without this a layer inside
        // a layer simply vanished — a rounded card around a fading group, a clip
        // around a transform — because a group's pass draws primitives and a layer is
        // not one.
        //
        // The recursion is safe to interleave with the shared instance buffers because
        // every level prepares, records and **submits** before returning: the deepest
        // group is on the queue before its parent writes a single instance.
        let mut nested: Vec<LayerComposite> = Vec::new();
        // Which nested composite belongs to which primitive, so the pass below can put
        // it back in scene order among this group's own drawing rather than after all
        // of it — the same ordering the top-level frame does.
        let mut nested_of: HashMap<usize, usize> = HashMap::new();
        for (scene_index, primitive) in primitives.iter().enumerate() {
            if let Primitive::Layer {
                primitives: inner,
                opacity,
                clip,
                clip_shape,
                transform,
                filter,
                ..
            } = primitive
            {
                let mut texture = self.render_group(device, queue, format, inner, w, h);
                if let Some(f) = filter.image.filter(|f| !f.is_identity()) {
                    texture = self.filter.apply(device, queue, format, &texture, f, w, h);
                }
                // The view holds a reference to its texture, so the texture outlives
                // this loop without being named again.
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let entry = self.layer_entry(
                    device, queue, format, view, *opacity, *clip, clip_shape, transform, filter, w,
                    h,
                );
                nested_of.insert(scene_index, nested.len());
                nested.push(entry);
            }
        }

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

        let batches = batch::plan(&sub);
        let (decorations, decoration_ranges) =
            self.text.prepare_frame(device, queue, &sub, w, h, &batches);
        let (rect_ranges, decoration_base) =
            self.rect
                .prepare_frame(device, queue, &sub, &decorations, &batches);
        let image_ranges = self.image.prepare_frame(device, queue, &sub, &batches);
        let path_ranges = self.path.prepare_frame(device, queue, &sub, &batches);
        if !nested.is_empty() {
            self.composite
                .prepare(device, queue, &nested, w as f32, h as f32);
        }

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
            let mut slot = 0;
            for (i, batch) in batches.iter().enumerate() {
                match batch.kind {
                    batch::Kind::Rect => self.rect.draw(&mut pass, rect_ranges[i].clone()),
                    batch::Kind::Image => self.image.draw(&mut pass, image_ranges[i].clone()),
                    batch::Kind::Path => self.path.draw(&mut pass, path_ranges[i].clone()),
                    batch::Kind::Text => {
                        let base = decoration_base.start;
                        let d = &decoration_ranges[i];
                        self.rect.draw(&mut pass, base + d.start..base + d.end);
                        self.text.draw(&mut pass, slot);
                        slot += 1;
                    }
                    // A nested layer, in its place among this group's own drawing.
                    batch::Kind::Layer => {
                        if let Some(&n) = batch.members.first().and_then(|s| nested_of.get(s)) {
                            self.composite.draw_range(&mut pass, n..n + 1, None);
                        }
                    }
                }
            }
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
