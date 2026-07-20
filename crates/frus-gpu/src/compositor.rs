//! Orchestration du rendu : regroupe les painters de contenu (rectangles,
//! images, chemins, texte) et gère les **calques** ([`Primitive::Layer`]).
//!
//! Un calque est rendu **à part** sur une texture pleine surface (pré-passe,
//! *submit* séparé pour ne pas aliaser les buffers d'instances), puis composité
//! d'un bloc à son opacité de groupe par le [`CompositePainter`]. L'alpha de
//! groupe est ainsi correct (pas de double-superposition), façon `saveLayer` de
//! Flutter.
//!
//! Tous les pipelines sont créés à la construction ([`Painters::new`]) puis
//! **échauffés** ([`Painters::warm_up`]) : la première vraie frame ne paie aucune
//! compilation de shader (pas de « shader jank » à la Flutter/Skia).

use bytemuck::{Pod, Zeroable};
use frus_core::{BoxFit, Color, ImageData, Path, Point, Primitive, Rect, Scene};
use wgpu::util::DeviceExt;

use crate::image::ImagePainter;
use crate::painter::Painter;
use crate::path::PathPainter;
use crate::text::TextPainter;

/// Sommet du quad unité (plein écran).
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

/// Instance de composite : découpe + opacité de groupe.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CompInstance {
    clip: [f32; 4],
    params: [f32; 4], // x = opacité
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Viewport {
    size: [f32; 2],
    _pad: [f32; 2],
}

/// Un calque prêt à composer : sa texture, son opacité et sa découpe.
struct LayerComposite {
    view: wgpu::TextureView,
    opacity: f32,
    clip: [f32; 4],
}

/// Pipeline de compositing des calques.
struct CompositePainter {
    pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    /// Groupes de liaison des textures de calques de la frame courante.
    bind_groups: Vec<wgpu::BindGroup>,
}

impl CompositePainter {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
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
            multisample: wgpu::MultisampleState::default(),
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
        }
    }

    /// Prépare le compositing des `layers` (instances + groupes de liaison).
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layers: &[LayerComposite], w: f32, h: f32) {
        let viewport = Viewport { size: [w.max(1.0), h.max(1.0)], _pad: [0.0, 0.0] };
        queue.write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));

        self.bind_groups.clear();
        if layers.is_empty() {
            return;
        }
        let instances: Vec<CompInstance> = layers
            .iter()
            .map(|l| CompInstance { clip: l.clip, params: [l.opacity, 0.0, 0.0, 0.0] })
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
            self.bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
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
    const ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<CompInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRS,
    }
}

/// L'ensemble des painters de contenu + le compositing des calques.
pub(crate) struct Painters {
    rect: Painter,
    image: ImagePainter,
    path: PathPainter,
    text: TextPainter,
    composite: CompositePainter,
}

impl Painters {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self {
            rect: Painter::new(device, format),
            image: ImagePainter::new(device, format),
            path: PathPainter::new(device, format),
            text: TextPainter::new(device, queue, format),
            composite: CompositePainter::new(device, format),
        }
    }

    fn set_viewport(&self, queue: &wgpu::Queue, w: f32, h: f32) {
        self.rect.set_viewport(queue, w, h);
        self.image.set_viewport(queue, w, h);
        self.path.set_viewport(queue, w, h);
    }

    /// Rend `scene` (calques compris) dans `target` de taille `w×h`. `clear` :
    /// `Some(couleur)` pour effacer la cible, `None` pour peindre par-dessus.
    /// **Submise en interne** (une passe par calque, plus la passe principale).
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

        // Pré-passes : chaque calque rendu sur sa propre texture pleine surface.
        let mut layers: Vec<LayerComposite> = Vec::new();
        for primitive in scene.primitives() {
            if let Primitive::Layer { primitives, opacity, clip, .. } = primitive {
                let texture = self.render_group(device, queue, format, primitives, w, h);
                layers.push(LayerComposite {
                    view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    opacity: *opacity,
                    clip: clip.to_array(),
                });
            }
        }

        self.composite.prepare(device, queue, &layers, w as f32, h as f32);
        let decorations = self.text.prepare_frame(device, queue, scene, w, h);
        let rect_count = self.rect.prepare_frame(device, queue, scene, &decorations);
        let image_count = self.image.prepare_frame(device, queue, scene);
        let path_count = self.path.prepare_frame(device, queue, scene);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frus.encoder") });
        {
            let load = match clear {
                Some(c) => wgpu::LoadOp::Clear(c),
                None => wgpu::LoadOp::Load,
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.rect.draw(&mut pass, rect_count);
            self.image.draw(&mut pass, image_count);
            self.path.draw(&mut pass, path_count);
            self.text.draw(&mut pass);
            self.composite.draw(&mut pass);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Rend un groupe de primitives sur une texture pleine surface (fond
    /// transparent), pour compositing ultérieur. Les calques **imbriqués** ne
    /// sont pas recompositionnés à ce niveau (limite assumée).
    fn render_group(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        primitives: &[Primitive],
        w: u32,
        h: u32,
    ) -> wgpu::Texture {
        let mut sub = Scene::new();
        for primitive in primitives {
            sub.push_primitive(primitive.clone());
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.layer.texture"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let decorations = self.text.prepare_frame(device, queue, &sub, w, h);
        let rect_count = self.rect.prepare_frame(device, queue, &sub, &decorations);
        let image_count = self.image.prepare_frame(device, queue, &sub);
        let path_count = self.path.prepare_frame(device, queue, &sub);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frus.layer.encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.layer.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Fond transparent : le calque ne couvre que ses primitives.
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.rect.draw(&mut pass, rect_count);
            self.image.draw(&mut pass, image_count);
            self.path.draw(&mut pass, path_count);
            self.text.draw(&mut pass);
        }
        queue.submit(std::iter::once(encoder.finish()));
        texture
    }

    /// **Échauffe** tous les pipelines en rendant une petite scène qui exerce
    /// chaque chemin de rendu (rectangle, image, chemin, texte, calque →
    /// composite) — la première vraie frame ne compile alors plus rien.
    pub(crate) fn warm_up(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) {
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color::WHITE);
        scene.fill_path(&Path::rect(Rect::new(0.0, 0.0, 2.0, 2.0)), Color::WHITE);
        let img = ImageData::from_rgba(1, 1, vec![255, 255, 255, 255]).into_handle();
        scene.image(&img, Rect::new(0.0, 0.0, 2.0, 2.0), BoxFit::Fill);
        scene.text(Point::new(0.0, 0.0), "x", 8.0, Color::WHITE);
        scene.layer(0.5, |inner| inner.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color::WHITE));

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.warmup.texture"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render(device, queue, format, &view, 4, 4, &scene, Some(wgpu::Color::BLACK));
    }
}
