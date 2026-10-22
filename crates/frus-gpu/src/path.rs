//! The `PathPainter`: rendering of vector paths ([`Primitive::Path`]).
//!
//! Paths are **tessellated on the CPU** by lyon (fill and/or stroke) into an
//! indexed triangle list, then drawn in a single pass. The triangles are kept from frame
//! to frame by outline ([`MeshCache`](crate::mesh::MeshCache)), so a path that comes back
//! is not tessellated again. Each vertex carries its own
//! colour (sRGB) and clip rectangle, mirroring the rectangle `Painter` — the
//! `path.wgsl` shader projects and clips.
//!
//! Antialiasing: the tessellated geometry is crisp, and **oblique edges** are
//! smoothed by the pass's MSAA (see `compositor.rs`), the pipeline being compiled
//! at the target's `sample_count`.

use bytemuck::{Pod, Zeroable};
use frus_core::{Affine, GradientRamp, PathGradient, Point, Primitive, RampShape, Scene, TileMode};
use std::ops::Range;

use crate::batch::{Batch, Kind};
use crate::mesh::{Mesh, MeshCache, Op};
use crate::painter::{
    bake_with, ramp_layout, ramp_sampler, ramp_texture, RAMP_MAX_ROWS, RAMP_WIDTH,
};

/// A tessellated vertex: position (px), colour (sRGB), gradient, clip.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PathVertex {
    pos: [f32; 2],
    /// The fill colour, or the gradient's start colour.
    color: [f32; 4],
    /// The gradient's end colour; equal to `color` when the fill is flat.
    color2: [f32; 4],
    /// The gradient's geometry, in the same pixel space as `pos`: `(from, to)` for a
    /// straight fade, `(centre, radii)` for a radial one.
    geometry: [f32; 4],
    /// `(kind, inner)`: kind `0` flat, `1` straight, `2` radial, `3` a ramp. `inner` is
    /// where a radial fade starts, as a fraction of the radii; for a ramp, its row.
    kind: [f32; 2],
    clip: [f32; 4],
    /// Where the vertex is in the ramp's own space: its position mapped back through the
    /// ramp's transform. Interpolated, which is exact, the map being affine.
    grad_pos: [f32; 2],
    /// For a ramp: the focal point, then `shape + 4 × tile`.
    extra: [f32; 4],
}

impl PathVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x2,
            5 => Float32x4,
            6 => Float32x2,
            7 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PathVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

/// What a path's vertices are made of. Everything it injects — colour, clip,
/// gradient — is constant over a whole path, so the vertices differ only in position
/// and the fragment shader resolves the fade from that.
///
/// Resolving the fade **per vertex** was the first shape of this and it does not
/// work: it makes the gradient a function of the tessellation. A straight fade is
/// affine, so it survived; a radial one is not, and an ellipse tessellates into
/// triangles whose corners all sit on the boundary — every one of them at the far end
/// of the fade — so the whole shape would come out at the end colour (milestone 302).
struct Ctor {
    color: [f32; 4],
    color2: [f32; 4],
    geometry: [f32; 4],
    kind: [f32; 2],
    clip: [f32; 4],
    /// The path's space to a ramp's, for a ramp.
    to_ramp: Option<Affine>,
    extra: [f32; 4],
}

impl Ctor {
    /// A flat fill: the end colour is the fill and the kind is `0`, so the fragment
    /// mixes nothing.
    fn flat(color: [f32; 4], clip: [f32; 4]) -> Self {
        Self {
            color,
            color2: color,
            geometry: [0.0; 4],
            kind: [0.0, 0.0],
            clip,
            to_ramp: None,
            extra: [0.0; 4],
        }
    }

    /// A ramp fill, `tint` multiplying it, its colours in ramp row `row`.
    fn ramp(ramp: &GradientRamp, tint: [f32; 4], row: f32, clip: [f32; 4]) -> Self {
        let tile = match ramp.tile {
            TileMode::Clamp => 0.0,
            TileMode::Repeated => 1.0,
            TileMode::Mirror => 2.0,
            TileMode::Decal => 3.0,
        };
        let (shape, geometry, focal) = match ramp.shape {
            RampShape::Linear { from, to } => (1.0, [from.x, from.y, to.x, to.y], [0.0, 0.0]),
            RampShape::Radial {
                center,
                radius,
                focal,
                focal_radius,
            } => (
                2.0,
                [center.x, center.y, radius, focal_radius],
                [focal.x, focal.y],
            ),
        };
        Self {
            color: tint,
            color2: tint,
            geometry,
            kind: [3.0, row],
            clip,
            to_ramp: Some(ramp.transform.inverse()),
            extra: [focal[0], focal[1], shape + 4.0 * tile, 0.0],
        }
    }

    fn vertex(&self, x: f32, y: f32) -> PathVertex {
        let grad_pos = match self.to_ramp {
            Some(map) => {
                let p = map.apply(frus_core::Point::new(x, y));
                [p.x, p.y]
            }
            None => [x, y],
        };
        PathVertex {
            pos: [x, y],
            color: self.color,
            color2: self.color2,
            geometry: self.geometry,
            kind: self.kind,
            clip: self.clip,
            grad_pos,
            extra: self.extra,
        }
    }
}

/// A frame's paths, as one indexed triangle list.
#[derive(Default)]
struct Geometry {
    vertices: Vec<PathVertex>,
    indices: Vec<u32>,
}

impl Geometry {
    /// Appends `mesh`, moved to `origin`, its vertices made by `ctor`.
    fn append(&mut self, mesh: &Mesh, origin: Point, ctor: &Ctor) {
        let base = self.vertices.len() as u32;
        self.vertices.extend(
            mesh.positions
                .iter()
                .map(|p| ctor.vertex(p[0] + origin.x, p[1] + origin.y)),
        );
        self.indices.extend(mesh.indices.iter().map(|i| base + i));
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
    geometry: Geometry,
    meshes: MeshCache,
    /// The ramps' colours, one row per ramp in this frame, and where they go.
    ramp_layout: wgpu::BindGroupLayout,
    ramp_sampler: wgpu::Sampler,
    ramp_texture: wgpu::Texture,
    ramp_bind_group: wgpu::BindGroup,
    ramp_rows: Vec<[u8; RAMP_WIDTH as usize * 4]>,
}

impl PathPainter {
    /// Builds the painter for a given target format.
    /// `sample_count` is the MSAA sample count; 1 means no multisampling.
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        cache: Option<&wgpu::PipelineCache>,
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

        let ramp_layout = ramp_layout(device);
        let ramp_sampler = ramp_sampler(device);
        let (ramp_texture, ramp_bind_group) = ramp_texture(device, &ramp_layout, &ramp_sampler, 1);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.path.pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout), Some(&ramp_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.path.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(PathVertex::layout())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
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
            multiview_mask: None,
            cache,
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
            geometry: Geometry::default(),
            meshes: MeshCache::default(),
            ramp_layout,
            ramp_sampler,
            ramp_texture,
            ramp_bind_group,
            ramp_rows: Vec::new(),
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
        self.ramp_rows.clear();

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
                    gradient,
                    stroke,
                    clip,
                    ..
                } = &scene.primitives()[member]
                {
                    let clip = clip.to_array();
                    if let Some(color) = fill {
                        let color = color.to_array();
                        let ctor = match gradient {
                            None => Ctor::flat(color, clip),
                            // Baked into the next row; past the last one, the fill colour
                            // stands in, as a box's gradient does.
                            Some(PathGradient::Ramp(ramp))
                                if (self.ramp_rows.len() as u32) < RAMP_MAX_ROWS =>
                            {
                                let row = self.ramp_rows.len() as f32;
                                self.ramp_rows.push(bake_with(|t| ramp.sample(t)));
                                Ctor::ramp(ramp, color, row, clip)
                            }
                            Some(PathGradient::Ramp(_)) => Ctor::flat(color, clip),
                            Some(PathGradient::Linear { to_color, from, to }) => Ctor {
                                color,
                                color2: to_color.to_array(),
                                geometry: [from.x, from.y, to.x, to.y],
                                kind: [1.0, 0.0],
                                clip,
                                to_ramp: None,
                                extra: [0.0; 4],
                            },
                            Some(PathGradient::Radial {
                                to_color,
                                center,
                                radii,
                                inner,
                            }) => Ctor {
                                color,
                                color2: to_color.to_array(),
                                geometry: [center.x, center.y, radii.width, radii.height],
                                kind: [2.0, *inner],
                                clip,
                                to_ramp: None,
                                extra: [0.0; 4],
                            },
                        };
                        if let Some((mesh, origin)) =
                            self.meshes.get(path, Op::Fill(path.fill_rule()))
                        {
                            self.geometry.append(&mesh, origin, &ctor);
                        }
                    }
                    if let Some(s) = stroke {
                        // A stroke is never given a gradient today.
                        let ctor = Ctor::flat(s.color.to_array(), clip);
                        if let Some((mesh, origin)) =
                            self.meshes.get(path, Op::stroke(s.width, s.cap))
                        {
                            self.geometry.append(&mesh, origin, &ctor);
                        }
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
        // The frame's ramps, uploaded in one go; the texture grows to hold them.
        if !self.ramp_rows.is_empty() {
            let rows = self.ramp_rows.len() as u32;
            if rows > self.ramp_texture.height() {
                let (texture, bind_group) = ramp_texture(
                    device,
                    &self.ramp_layout,
                    &self.ramp_sampler,
                    rows.next_power_of_two(),
                );
                self.ramp_texture = texture;
                self.ramp_bind_group = bind_group;
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.ramp_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&self.ramp_rows),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(RAMP_WIDTH * 4),
                    rows_per_image: Some(rows),
                },
                wgpu::Extent3d {
                    width: RAMP_WIDTH,
                    height: rows,
                    depth_or_array_layers: 1,
                },
            );
        }
        ranges
    }

    /// Ends a frame for the kept triangles. Called once a frame, after the frame's last
    /// `prepare_frame` — layers prepare their own paths first, several times a frame.
    pub(crate) fn end_frame(&mut self) {
        self.meshes.end_frame();
    }

    /// Draws one batch's paths into an already-open render pass.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, range: Range<u32>) {
        if range.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_bind_group(1, &self.ramp_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(range, 0, 0..1);
    }
}
