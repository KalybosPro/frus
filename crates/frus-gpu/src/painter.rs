//! Le `Painter` : logique de dessin des primitives, indépendante de toute
//! surface (fenêtre). Il peut donc peindre aussi bien sur une surface que sur
//! une texture offscreen — ce qui rend le rendu testable en headless.

use bytemuck::{Pod, Zeroable};
use frus_core::{Primitive, Scene};
use wgpu::util::DeviceExt;

/// Sommet du quad unité (coin dans `[0,1]²`).
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

/// Données d'une instance transmises au GPU (une par rectangle). Construites à
/// partir des primitives de la [`Scene`] à chaque frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Instance {
    rect: [f32; 4],
    color: [f32; 4],
}

impl Instance {
    /// Layout du buffer d'instances (locations 1 et 2 ; la 0 est le quad unité).
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

/// Deux triangles couvrant le quad unité.
const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];
const QUAD_VERTEX_COUNT: u32 = 6;

/// Uniform passé au shader : la taille de la surface en pixels.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Viewport {
    size: [f32; 2],
    // Padding pour respecter l'alignement 16 octets des uniform buffers.
    _pad: [f32; 2],
}

/// Nombre d'instances alloué initialement dans le buffer.
const INITIAL_INSTANCE_CAPACITY: usize = 128;

/// Détient le pipeline et les buffers nécessaires au dessin des primitives.
pub(crate) struct Painter {
    pipeline: wgpu::RenderPipeline,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    /// Tampon CPU réutilisé pour construire les instances à partir de la scène.
    instances: Vec<Instance>,
}

impl Painter {
    /// Construit le painter pour un format de cible donné (surface ou texture).
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
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

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            multisample: wgpu::MultisampleState::default(),
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
            size: (INITIAL_INSTANCE_CAPACITY * std::mem::size_of::<Instance>()) as wgpu::BufferAddress,
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

    /// Met à jour la taille de la surface (à appeler à l'init et au resize).
    pub(crate) fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let viewport = Viewport {
            size: [width.max(1.0), height.max(1.0)],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));
    }

    /// Traduit les primitives de la scène en instances GPU et les téléverse,
    /// en agrandissant le buffer si besoin. Renvoie le nombre d'instances.
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) -> u32 {
        self.instances.clear();
        for primitive in scene.primitives() {
            match *primitive {
                Primitive::Rect { rect, color } => self.instances.push(Instance {
                    rect: rect.to_array(),
                    color: color.to_array(),
                }),
            }
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
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&self.instances));
        count as u32
    }

    /// Enregistre le rendu de la scène dans `encoder`, vers `target`.
    pub(crate) fn record(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clear: wgpu::Color,
        scene: &Scene,
    ) {
        let instance_count = self.prepare(device, queue, scene);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frus.render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if instance_count > 0 {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.viewport_bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.draw(0..QUAD_VERTEX_COUNT, 0..instance_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Rect};

    /// Instancie un device wgpu sans surface (headless). `None` si aucun GPU.
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

    /// Rend un rectangle rouge couvrant toute une texture offscreen, puis relit
    /// le pixel central : il doit être rouge opaque. Preuve automatique que le
    /// pipeline + la conversion de coordonnées fonctionnent, sans fenêtre.
    #[test]
    fn renders_red_rect_to_center_pixel() {
        let Some((device, queue)) = headless_device() else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };

        const SIZE: u32 = 64; // 64 * 4 = 256 octets/ligne => aligné pour la copie.
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

        let mut painter = Painter::new(&device, format);
        painter.set_viewport(&queue, SIZE as f32, SIZE as f32);

        let mut scene = Scene::new();
        scene.fill_rect(
            Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
            Color::rgb(1.0, 0.0, 0.0),
        );

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        painter.record(&device, &queue, &mut encoder, &view, wgpu::Color::BLACK, &scene);

        // Copie de la texture vers un buffer lisible par le CPU.
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
        rx.recv().expect("map_async").expect("mapping échoué");

        let data = slice.get_mapped_range();
        let (cx, cy) = (SIZE / 2, SIZE / 2);
        let idx = (cy * bytes_per_row + cx * 4) as usize;
        let pixel = [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]];

        assert_eq!(pixel, [255, 0, 0, 255], "le pixel central doit être rouge opaque");
    }
}
