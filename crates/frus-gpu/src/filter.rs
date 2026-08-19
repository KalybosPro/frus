//! The **image filter** pre-pass: a blur, a dilate or an erode over a layer's
//! rendered texture, before it is composited.
//!
//! It runs as two passes, one per axis, because the three filters offered are all
//! separable: the two-dimensional result is the two one-dimensional ones applied in
//! turn. That turns `(2n+1)²` samples per pixel into `2(2n+1)`, which is the whole
//! reason a wide blur is affordable at all.
//!
//! The intermediate lives in a scratch texture reused across layers and frames; the
//! result is a **fresh** texture, because it becomes the layer's own and the layer
//! cache keeps it until the layer's content changes.

use bytemuck::{Pod, Zeroable};
use frus_core::ImageFilter;

/// What one pass is told: the texture size, the axis, the reach, and which of the
/// three filters to run.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    size: [f32; 2],
    dir: [f32; 2],
    radius: f32,
    kind: f32,
    _pad: [f32; 2],
}

/// A vertex of the unit quad, which covers the whole surface.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];

/// Uniform buffers must start at a device-aligned offset, and 256 bytes is the
/// alignment every backend asks for or less. Two passes therefore share one buffer,
/// the second starting here.
const PARAMS_STRIDE: wgpu::BufferAddress = 256;

/// Three slots: the two filter passes, and the blit that puts a staged frame on the
/// screen — which is this same pipeline with nothing to do.
const PARAMS_SLOTS: wgpu::BufferAddress = 3;

/// The separable image-filter pipeline, plus the scratch texture the first pass
/// writes and the second reads.
pub(crate) struct FilterPainter {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quad: wgpu::Buffer,
    params: wgpu::Buffer,
    scratch: Option<Scratch>,
}

struct Scratch {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    texture: wgpu::Texture,
}

impl FilterPainter {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frus.filter.shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/filter.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.filter.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.filter.pipeline_layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.filter.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRS,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // The pass **replaces** its target rather than painting over it:
                    // the filtered value is the whole answer, and the target was
                    // cleared to nothing anyway.
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let quad = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("frus.filter.quad"),
                contents: bytemuck::cast_slice(QUAD_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.filter.params"),
            size: PARAMS_STRIDE * PARAMS_SLOTS,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("frus.filter.sampler"),
            // Clamping is what makes an erode behave: the edge repeats rather than
            // an imaginary transparent border eating into the shape.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            layout,
            sampler,
            quad,
            params,
            scratch: None,
        }
    }

    /// The scratch texture the horizontal pass writes into, created on demand and
    /// recreated when the surface changes size.
    fn scratch(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        w: u32,
        h: u32,
    ) -> &wgpu::Texture {
        let stale = match &self.scratch {
            Some(s) => s.width != w || s.height != h || s.format != format,
            None => true,
        };
        if stale {
            self.scratch = Some(Scratch {
                width: w,
                height: h,
                format,
                texture: new_target(device, format, w, h, "frus.filter.scratch"),
            });
        }
        &self.scratch.as_ref().expect("just created").texture
    }

    /// Runs `filter` over `source`, returning a **new** texture of the same size.
    ///
    /// Two passes, horizontal then vertical, with the scratch texture in between.
    /// They are submitted together: nothing outside reads the intermediate, so there
    /// is no reason to make the queue wait twice.
    // Device, queue, format, source, filter, and the two dimensions: every one of
    // them is needed, and grouping any of them would only move the same values
    // through an extra struct.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        source: &wgpu::Texture,
        filter: ImageFilter,
        w: u32,
        h: u32,
    ) -> wgpu::Texture {
        let (rx, ry) = filter.radius();
        let kind = filter.code() as f32;
        let size = [w.max(1) as f32, h.max(1) as f32];
        for (i, (dir, radius)) in [([1.0, 0.0], rx), ([0.0, 1.0], ry)].iter().enumerate() {
            let params = Params {
                size,
                dir: *dir,
                radius: *radius,
                kind,
                _pad: [0.0, 0.0],
            };
            queue.write_buffer(
                &self.params,
                i as wgpu::BufferAddress * PARAMS_STRIDE,
                bytemuck::bytes_of(&params),
            );
        }

        let scratch_view = self
            .scratch(device, format, w, h)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let output = new_target(device, format, w, h, "frus.filter.output");
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.filter.encoder"),
        });
        for (i, (from, to)) in [(&source_view, &scratch_view), (&scratch_view, &output_view)]
            .iter()
            .enumerate()
        {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("frus.filter.bind_group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.params,
                            offset: i as wgpu::BufferAddress * PARAMS_STRIDE,
                            size: wgpu::BufferSize::new(std::mem::size_of::<Params>() as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(from),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.filter.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: to,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad.slice(..));
            pass.draw(0..QUAD_VERTICES.len() as u32, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));
        output
    }
}

impl FilterPainter {
    /// Copies `source` onto `target`, one to one.
    ///
    /// It is the filter pipeline with a radius of zero, which the shader answers by
    /// returning the sample it centred on. A blit deserves no pipeline of its own, and
    /// the one place it is needed — putting a staged frame on the screen once the
    /// backdrops in it have been drawn — is downstream of this file anyway.
    pub(crate) fn blit(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
    ) {
        const SLOT: wgpu::BufferAddress = 2;
        let params = Params {
            size: [1.0, 1.0],
            dir: [0.0, 0.0],
            radius: 0.0,
            kind: 0.0,
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(
            &self.params,
            SLOT * PARAMS_STRIDE,
            bytemuck::bytes_of(&params),
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frus.filter.blit.bind_group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.params,
                        offset: SLOT * PARAMS_STRIDE,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Params>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frus.filter.blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad.slice(..));
        pass.draw(0..QUAD_VERTICES.len() as u32, 0..1);
    }
}

/// A full-surface texture that can be painted into and sampled from.
fn new_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    w: u32,
    h: u32,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
