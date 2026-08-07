//! The `Painter`: the primitive drawing logic, independent of any surface or
//! window. It can therefore paint onto a surface or onto an offscreen texture
//! alike, which is what makes rendering testable headlessly.

use bytemuck::{Pod, Zeroable};
use frus_core::{Color, Primitive, Scene};
use wgpu::util::DeviceExt;

use crate::text::DecorationQuad;

/// A vertex of the unit quad, its corner in `[0,1]²`.
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

/// The instance data handed to the GPU, one per rectangle, built from the
/// [`Scene`]'s primitives every frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Instance {
    rect: [f32; 4],
    color: [f32; 4],
    color2: [f32; 4],
    border_color: [f32; 4],
    /// (reserved), border_width, blur, (reserved).
    params: [f32; 4],
    /// The gradient direction (x, y), then (reserved, reserved).
    gradient: [f32; 4],
    /// The clip rectangle: x, y, width, height.
    clip: [f32; 4],
    /// Corner radii, per corner: tl, tr, br, bl.
    radii: [f32; 4],
}

impl Instance {
    /// The instance buffer's layout (locations 1..=8; 0 is the unit quad).
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

/// Two triangles covering the unit quad.
const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];
const QUAD_VERTEX_COUNT: u32 = 6;

/// The uniform handed to the shader: the surface size in pixels.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Viewport {
    size: [f32; 2],
    // Padding, to respect uniform buffers' 16-byte alignment.
    _pad: [f32; 2],
}

/// How many instances the buffer allocates to begin with.
const INITIAL_INSTANCE_CAPACITY: usize = 128;

/// Holds the pipeline and the buffers needed to draw the primitives.
pub(crate) struct Painter {
    pipeline: wgpu::RenderPipeline,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    /// A reused CPU buffer for building the instances out of the scene.
    instances: Vec<Instance>,
}

impl Painter {
    /// Builds the painter for a given target format, surface or texture.
    /// `sample_count` is the MSAA sample count; 1 means no multisampling.
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frus.quad.shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
        });

        let viewport_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.viewport"),
            size: std::mem::size_of::<Viewport>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.viewport.bgl"),
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
            label: Some("frus.viewport.bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[QuadVertex::layout(), Instance::layout()],
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
            label: Some("frus.quad_vertex_buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.instance_buffer"),
            size: (INITIAL_INSTANCE_CAPACITY * std::mem::size_of::<Instance>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            viewport_buffer,
            viewport_bind_group,
            instances: Vec::new(),
        }
    }

    /// Updates the surface size; call it at init and on resize.
    pub(crate) fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let viewport = Viewport {
            size: [width.max(1.0), height.max(1.0)],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));
    }

    /// Translates the scene's primitives into GPU instances and uploads them,
    /// growing the buffer if needed. Returns the instance count.
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        decorations: &[DecorationQuad],
    ) -> u32 {
        self.instances.clear();
        for primitive in scene.primitives() {
            match primitive {
                Primitive::Rect {
                    rect,
                    color,
                    color2,
                    gradient_dir,
                    radius,
                    border_width,
                    border_color,
                    blur,
                    clip,
                    ..
                } => self.instances.push(Instance {
                    rect: rect.to_array(),
                    color: color.to_array(),
                    color2: color2.to_array(),
                    border_color: border_color.to_array(),
                    params: [0.0, *border_width, *blur, 0.0],
                    gradient: [gradient_dir[0], gradient_dir[1], 0.0, 0.0],
                    clip: clip.to_array(),
                    // Negative radii are clamped to zero before rendering.
                    radii: radius.clamped().to_array(),
                }),
                // Text, vector paths and images are rendered by their own painters
                // (TextPainter, PathPainter, ImagePainter).
                Primitive::Text { .. }
                | Primitive::RichText { .. }
                | Primitive::Path { .. }
                | Primitive::Image { .. }
                | Primitive::Layer { .. } => {}
            }
        }

        // Text decoration quads (underline, strikethrough): plain rectangles, drawn
        // in the quad pass, and therefore beneath the glyphs.
        for quad in decorations {
            self.instances.push(Instance {
                rect: quad.rect.to_array(),
                color: quad.color.to_array(),
                color2: quad.color.to_array(),
                border_color: Color::TRANSPARENT.to_array(),
                params: [0.0, 0.0, 0.0, 0.0],
                gradient: [0.0, 0.0, 0.0, 0.0],
                clip: quad.clip.to_array(),
                radii: [0.0, 0.0, 0.0, 0.0],
            });
        }

        let count = self.instances.len();
        if count == 0 {
            return 0;
        }
        if count > self.instance_capacity {
            let new_capacity = count.next_power_of_two();
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frus.instance_buffer"),
                size: (new_capacity * std::mem::size_of::<Instance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = new_capacity;
        }
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
        count as u32
    }

    /// Prepares the render by uploading the instances, and returns their count.
    /// `decorations` are the quads coming from text (see
    /// [`TextPainter::prepare_frame`]). Call it **before** opening the render pass.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        decorations: &[DecorationQuad],
    ) -> u32 {
        self.prepare(device, queue, scene, decorations)
    }

    /// Draws the rectangles into an already-open render pass.
    pub(crate) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        instance_count: u32,
    ) {
        if instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.draw(0..QUAD_VERTEX_COUNT, 0..instance_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Rect};

    /// Instantiates a surfaceless (headless) wgpu device. `None` when there is no GPU.
    fn headless_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("frus.test.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .ok()?;
        Some((device, queue))
    }

    /// Renders a red rectangle covering a whole offscreen texture, then reads the
    /// centre pixel back: it must be opaque red. Automatic proof that the pipeline
    /// and the coordinate conversion work, with no window involved.
    #[test]
    fn renders_red_rect_to_center_pixel() {
        let Some((device, queue)) = headless_device() else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };

        const SIZE: u32 = 64; // 64 * 4 = 256 bytes/row => aligned for the copy.
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.test.target"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut painter = Painter::new(&device, format, 1);
        painter.set_viewport(&queue, SIZE as f32, SIZE as f32);

        let mut scene = Scene::new();
        scene.fill_rect(
            Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
            Color::rgb(1.0, 0.0, 0.0),
        );

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let count = painter.prepare_frame(&device, &queue, &scene, &[]);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
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
            painter.draw(&mut pass, count);
        }

        // Copy the texture into a CPU-readable buffer.
        let bytes_per_row = SIZE * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.test.readback"),
            size: (bytes_per_row * SIZE) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("map_async")
            .expect("buffer mapping failed");

        let data = slice.get_mapped_range();
        let (cx, cy) = (SIZE / 2, SIZE / 2);
        let idx = (cy * bytes_per_row + cx * 4) as usize;
        let pixel = [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]];

        assert_eq!(
            pixel,
            [255, 0, 0, 255],
            "the centre pixel must be opaque red"
        );
    }

    /// A heavily rounded rectangle covering the whole texture: the centre is
    /// filled, but the corner is cut away and stays the black background.
    #[test]
    fn rounded_rect_leaves_corner_transparent() {
        let Some((device, queue)) = headless_device() else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };

        const SIZE: u32 = 64;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.test.rounded"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut painter = Painter::new(&device, format, 1);
        painter.set_viewport(&queue, SIZE as f32, SIZE as f32);

        let mut scene = Scene::new();
        scene.draw_rect(
            Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
            Color::rgb(1.0, 0.0, 0.0),
            30.0, // a large radius
            0.0,
            Color::TRANSPARENT,
        );

        let count = painter.prepare_frame(&device, &queue, &scene, &[]);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
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
            painter.draw(&mut pass, count);
        }

        let bytes_per_row = SIZE * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * SIZE) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("map_async")
            .expect("buffer mapping failed");

        let data = slice.get_mapped_range();
        let px = |x: u32, y: u32| {
            let idx = (y * bytes_per_row + x * 4) as usize;
            [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]
        };

        // A red centre, with the (0,0) corner cut away and black.
        assert_eq!(px(SIZE / 2, SIZE / 2), [255, 0, 0, 255], "centre rouge");
        assert_eq!(px(0, 0), [0, 0, 0, 255], "corner cut away");
    }

    /// **Per-corner** radii: only the rounded top-left corner is cut away, the
    /// other three stay square — the GPU path proved by readback.
    #[test]
    fn per_corner_radius_rounds_only_selected_corners() {
        let Some((device, queue)) = headless_device() else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };

        const SIZE: u32 = 64;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.test.per_corner"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut painter = Painter::new(&device, format, 1);
        painter.set_viewport(&queue, SIZE as f32, SIZE as f32);

        let mut scene = Scene::new();
        scene.draw_rect(
            Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
            Color::rgb(1.0, 0.0, 0.0),
            frus_core::BorderRadius {
                top_left: 30.0,
                top_right: 0.0,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            0.0,
            Color::TRANSPARENT,
        );

        let count = painter.prepare_frame(&device, &queue, &scene, &[]);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
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
            painter.draw(&mut pass, count);
        }

        let bytes_per_row = SIZE * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * SIZE) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("map_async")
            .expect("buffer mapping failed");

        let data = slice.get_mapped_range();
        let px = |x: u32, y: u32| {
            let idx = (y * bytes_per_row + x * 4) as usize;
            [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]
        };

        // Only the top-left corner is cut away; the other three are square.
        assert_eq!(px(0, 0), [0, 0, 0, 255], "rounded top-left, cut away");
        assert_eq!(px(SIZE - 1, 0), [255, 0, 0, 255], "square top-right");
        assert_eq!(px(0, SIZE - 1), [255, 0, 0, 255], "square bottom-left");
        assert_eq!(
            px(SIZE - 1, SIZE - 1),
            [255, 0, 0, 255],
            "square bottom-right"
        );
        assert_eq!(px(SIZE / 2, SIZE / 2), [255, 0, 0, 255], "centre plein");
    }

    /// A solid red rectangle whose clip covers one corner only: the centre, outside
    /// the clip, stays background; the corner, inside it, is red.
    #[test]
    fn clip_excludes_pixels_outside() {
        let Some((device, queue)) = headless_device() else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };

        const SIZE: u32 = 64;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.test.clip"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut painter = Painter::new(&device, format, 1);
        painter.set_viewport(&queue, SIZE as f32, SIZE as f32);

        let mut scene = Scene::new();
        // The clip is limited to a 16x16 square at the top left.
        scene.set_clip(Rect::new(0.0, 0.0, 16.0, 16.0));
        scene.fill_rect(
            Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
            Color::rgb(1.0, 0.0, 0.0),
        );

        let count = painter.prepare_frame(&device, &queue, &scene, &[]);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
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
            painter.draw(&mut pass, count);
        }

        let bytes_per_row = SIZE * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * SIZE) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("map_async")
            .expect("buffer mapping failed");

        let data = slice.get_mapped_range();
        let px = |x: u32, y: u32| {
            let idx = (y * bytes_per_row + x * 4) as usize;
            [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]
        };

        assert_eq!(px(4, 4), [255, 0, 0, 255], "inside the clip → red");
        assert_eq!(px(SIZE / 2, SIZE / 2), [0, 0, 0, 255], "hors clip → fond");
    }
}
