//! The `PathPainter`: rendering of vector paths ([`Primitive::Path`]).
//!
//! Paths are **tessellated on the CPU** by lyon (fill and/or stroke) into an
//! indexed triangle list, then drawn in a single pass. Each vertex carries its own
//! colour (sRGB) and clip rectangle, mirroring the rectangle `Painter` — the
//! `path.wgsl` shader projects and clips.
//!
//! Antialiasing: the tessellated geometry is crisp, and **oblique edges** are
//! smoothed by the pass's MSAA (see `compositor.rs`), the pipeline being compiled
//! at the target's `sample_count`.

use bytemuck::{Pod, Zeroable};
use frus_core::{Path, PathVerb, Primitive, Scene};
use std::ops::Range;

use crate::batch::{Batch, Kind};
use lyon::math::point;
use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, StrokeOptions,
    StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};

/// A tessellated vertex: position (px), colour (sRGB), clip.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PathVertex {
    pos: [f32; 2],
    color: [f32; 4],
    clip: [f32; 4],
}

impl PathVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x4,
            2 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PathVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

/// The vertex constructor handed to lyon: it injects the colour and clip — both
/// constant over a whole path — into every vertex the tessellation produces.
struct Ctor {
    color: [f32; 4],
    clip: [f32; 4],
}

impl FillVertexConstructor<PathVertex> for Ctor {
    fn new_vertex(&mut self, vertex: FillVertex) -> PathVertex {
        let p = vertex.position();
        PathVertex {
            pos: [p.x, p.y],
            color: self.color,
            clip: self.clip,
        }
    }
}

impl StrokeVertexConstructor<PathVertex> for Ctor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> PathVertex {
        let p = vertex.position();
        PathVertex {
            pos: [p.x, p.y],
            color: self.color,
            clip: self.clip,
        }
    }
}

/// Uniform: the surface size in pixels, the same as the `Painter`'s.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Viewport {
    size: [f32; 2],
    _pad: [f32; 2],
}

const INITIAL_VERTEX_CAPACITY: usize = 512;
const INITIAL_INDEX_CAPACITY: usize = 1024;

/// Holds the pipeline and buffers for path rendering.
pub(crate) struct PathPainter {
    pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_buffer: wgpu::Buffer,
    index_capacity: usize,
    /// CPU geometry reused from frame to frame, with fill and stroke merged.
    geometry: VertexBuffers<PathVertex, u32>,
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
}

impl PathPainter {
    /// Builds the painter for a given target format.
    /// `sample_count` is the MSAA sample count; 1 means no multisampling.
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frus.path.shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/path.wgsl").into()),
        });

        let viewport_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.path.viewport"),
            size: std::mem::size_of::<Viewport>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.path.viewport.bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frus.path.viewport.bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.path.pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.path.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[PathVertex::layout()],
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

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.path.vertex_buffer"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<PathVertex>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.path.index_buffer"),
            size: (INITIAL_INDEX_CAPACITY * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            viewport_buffer,
            viewport_bind_group,
            vertex_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            index_buffer,
            index_capacity: INITIAL_INDEX_CAPACITY,
            geometry: VertexBuffers::new(),
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
        }
    }

    /// Updates the surface size, at init and on resize.
    pub(crate) fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let viewport = Viewport {
            size: [width.max(1.0), height.max(1.0)],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));
    }

    /// Tessellates every path in the scene and uploads the geometry. Returns the
    /// number of **indices** to draw. Call it before the render pass.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        batches: &[Batch],
    ) -> Vec<Range<u32>> {
        self.geometry.vertices.clear();
        self.geometry.indices.clear();

        // Tessellated **in batch order**, so a batch is one contiguous index range
        // and one indexed draw. Scene order survives inside a batch.
        let mut ranges = Vec::with_capacity(batches.len());
        for batch in batches {
            let start = self.geometry.indices.len() as u32;
            if batch.kind != Kind::Path {
                ranges.push(start..start);
                continue;
            }
            for &member in &batch.members {
                if let Primitive::Path {
                    path,
                    fill,
                    stroke,
                    clip,
                    ..
                } = &scene.primitives()[member]
                {
                    let lyon_path = to_lyon(path);
                    let clip = clip.to_array();
                    if let Some(color) = fill {
                        let ctor = Ctor {
                            color: color.to_array(),
                            clip,
                        };
                        let _ = self.fill_tess.tessellate_path(
                            &lyon_path,
                            &FillOptions::default(),
                            &mut BuffersBuilder::new(&mut self.geometry, ctor),
                        );
                    }
                    if let Some(s) = stroke {
                        let ctor = Ctor {
                            color: s.color.to_array(),
                            clip,
                        };
                        let options = StrokeOptions::default().with_line_width(s.width);
                        let _ = self.stroke_tess.tessellate_path(
                            &lyon_path,
                            &options,
                            &mut BuffersBuilder::new(&mut self.geometry, ctor),
                        );
                    }
                }
            }
            ranges.push(start..self.geometry.indices.len() as u32);
        }

        let vertex_count = self.geometry.vertices.len();
        let index_count = self.geometry.indices.len();
        if index_count == 0 {
            return ranges;
        }

        if vertex_count > self.vertex_capacity {
            let new_capacity = vertex_count.next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frus.path.vertex_buffer"),
                size: (new_capacity * std::mem::size_of::<PathVertex>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vertex_capacity = new_capacity;
        }
        if index_count > self.index_capacity {
            let new_capacity = index_count.next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frus.path.index_buffer"),
                size: (new_capacity * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.index_capacity = new_capacity;
        }

        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&self.geometry.vertices),
        );
        queue.write_buffer(
            &self.index_buffer,
            0,
            bytemuck::cast_slice(&self.geometry.indices),
        );
        ranges
    }

    /// Draws one batch's paths into an already-open render pass.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, range: Range<u32>) {
        if range.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(range, 0, 0..1);
    }
}

/// Converts a frus [`Path`] into a lyon path, handling sub-path opening and
/// closing — lyon requires `begin`/`end` around every contour.
fn to_lyon(path: &Path) -> LyonPath {
    let mut builder = LyonPath::builder();
    let mut open = false;
    for verb in path.verbs() {
        match verb {
            PathVerb::MoveTo(p) => {
                if open {
                    builder.end(false);
                }
                builder.begin(point(p.x, p.y));
                open = true;
            }
            PathVerb::LineTo(p) => {
                if open {
                    builder.line_to(point(p.x, p.y));
                }
            }
            PathVerb::QuadTo { ctrl, to } => {
                if open {
                    builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(to.x, to.y));
                }
            }
            PathVerb::CubicTo { c1, c2, to } => {
                if open {
                    builder.cubic_bezier_to(
                        point(c1.x, c1.y),
                        point(c2.x, c2.y),
                        point(to.x, to.y),
                    );
                }
            }
            PathVerb::Close => {
                if open {
                    builder.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        builder.end(false);
    }
    builder.build()
}
