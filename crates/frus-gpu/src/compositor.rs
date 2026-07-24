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

/// Nombre d'échantillons MSAA visé : 4× est un bon compromis qualité/coût et le
/// plus largement supporté (y compris le rasteriseur logiciel llvmpipe).
pub(crate) const MSAA_SAMPLES: u32 = 4;

/// Renvoie le nombre d'échantillons MSAA à utiliser pour `format` sur cet
/// adaptateur : [`MSAA_SAMPLES`] si supporté, sinon 1 (MSAA désactivé). Appelé
/// une fois à l'init par le renderer fenêtré comme par le rendu hors écran.
pub(crate) fn preferred_sample_count(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> u32 {
    let flags = adapter.get_texture_format_features(format).flags;
    if flags.sample_count_supported(MSAA_SAMPLES) {
        MSAA_SAMPLES
    } else {
        1
    }
}

/// Un calque prêt à composer : sa texture, son opacité, sa découpe et sa rotation
/// éventuelle (angle radians + pivot px).
struct LayerComposite {
    view: wgpu::TextureView,
    opacity: f32,
    clip: [f32; 4],
    /// `(angle, pivot_x, pivot_y)` — angle nul = pas de rotation.
    rotation: [f32; 3],
}

/// Texture d'un calque **conservée entre frames** (frontière de repaint côté
/// GPU), avec l'instantané de son contenu et ses dimensions : tant qu'ils ne
/// changent pas, on réutilise la texture telle quelle — la pré-passe (submit +
/// tessellation + dessin) est entièrement sautée.
struct CachedLayer {
    primitives: Vec<Primitive>,
    width: u32,
    height: u32,
    texture: wgpu::Texture,
}

/// Texture MSAA intermédiaire réutilisée comme cible de rendu : on peint dedans
/// (multi-échantillon) puis on **résout** vers la cible mono-échantillon. Une
/// seule suffit — pré-passes de calques et passe principale, toutes pleine
/// surface, l'emploient tour à tour (submits séquentiels).
struct MsaaScratch {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    texture: wgpu::Texture,
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
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, sample_count: u32) -> Self {
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
                // Le fragment lit aussi `viewport.size` (contre-rotation d'un calque).
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
            .map(|l| CompInstance {
                clip: l.clip,
                params: [l.opacity, l.rotation[0], l.rotation[1], l.rotation[2]],
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
    /// Nombre d'échantillons MSAA (1 = pas de multi-échantillon).
    sample_count: u32,
    /// Texture MSAA intermédiaire (créée à la demande, recréée au resize).
    msaa: Option<MsaaScratch>,
    /// Textures de calques conservées entre frames, indexées par rang du calque.
    layer_cache: Vec<CachedLayer>,
    /// Compteur de pré-passes de calque réellement rendues (preuve du cache).
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
            composite: CompositePainter::new(device, format, sample_count),
            sample_count,
            msaa: None,
            layer_cache: Vec::new(),
            layer_renders: 0,
        }
    }

    /// Nombre de pré-passes de calque rendues depuis la création (test du cache).
    #[cfg(test)]
    pub(crate) fn layer_render_count(&self) -> u64 {
        self.layer_renders
    }

    /// Renvoie une vue neuve de la texture MSAA (recréée si taille/format
    /// changent), ou `None` si le MSAA est désactivé (`sample_count == 1`). La vue
    /// est créée à la volée (peu coûteux) et renvoyée par valeur : l'emprunt de
    /// `self` est ainsi libéré avant l'ouverture du render pass.
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
                size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: self.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa = Some(MsaaScratch { width: w, height: h, format, texture });
        }
        self.msaa
            .as_ref()
            .map(|s| s.texture.create_view(&wgpu::TextureViewDescriptor::default()))
    }

    fn set_viewport(&self, queue: &wgpu::Queue, w: f32, h: f32) {
        self.rect.set_viewport(queue, w, h);
        self.image.set_viewport(queue, w, h);
        self.path.set_viewport(queue, w, h);
    }

    /// Rend `scene` (calques compris) dans `target` de taille `w×h`. `clear` :
    /// `Some(couleur)` pour effacer la cible, `None` pour peindre par-dessus.
    /// **Submise en interne** (une passe par calque, plus la passe principale).
    ///
    /// Note : sous MSAA, `clear == None` (peindre par-dessus) n'est pas pris en
    /// charge — la cible multi-échantillon ne contient pas le contenu existant de
    /// `target`. Tous les appelants actuels passent `Some(_)`.
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

        // Pré-passes : chaque calque sur sa propre texture pleine surface, mais
        // **réutilisée** telle quelle si son contenu n'a pas changé (cache GPU).
        let mut layers: Vec<LayerComposite> = Vec::new();
        let mut layer_index = 0usize;
        for primitive in scene.primitives() {
            if let Primitive::Layer { primitives, opacity, clip, transform, .. } = primitive {
                let view = self.layer_texture(device, queue, format, layer_index, primitives, w, h);
                let rotation = match transform {
                    Some(t) => [t.angle, t.pivot.x, t.pivot.y],
                    None => [0.0, 0.0, 0.0],
                };
                layers.push(LayerComposite {
                    view,
                    opacity: *opacity,
                    clip: clip.to_array(),
                    rotation,
                });
                layer_index += 1;
            }
        }
        // Oublie les calques disparus (la scène en a moins qu'à la frame passée).
        self.layer_cache.truncate(layer_index);

        self.composite.prepare(device, queue, &layers, w as f32, h as f32);
        let decorations = self.text.prepare_frame(device, queue, scene, w, h);
        let rect_count = self.rect.prepare_frame(device, queue, scene, &decorations);
        let image_count = self.image.prepare_frame(device, queue, scene);
        let path_count = self.path.prepare_frame(device, queue, scene);

        // MSAA : on peint dans la texture multi-échantillon puis on résout vers
        // `target` ; sans MSAA on peint directement dans `target`.
        let msaa_view = self.ensure_msaa(device, format, w, h);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frus.encoder") });
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

    /// Renvoie la vue de la texture d'un calque : **réutilisée** telle quelle si
    /// son contenu et ses dimensions sont inchangés depuis la frame précédente,
    /// sinon (re)rendue par [`Painters::render_group`]. `index` = rang du calque
    /// dans la scène, clé stable du cache ; une clé qui « glisse » (calques
    /// réordonnés) ne fait que rater le cache → re-render correct, jamais faux.
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
            let entry = CachedLayer { primitives: primitives.to_vec(), width: w, height: h, texture };
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
        self.layer_renders += 1;
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

        // MSAA : le calque est peint multi-échantillon puis résolu vers sa texture
        // mono-échantillon (celle échantillonnée ensuite par le compositeur).
        let msaa_view = self.ensure_msaa(device, format, w, h);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frus.layer.encoder") });
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
            size: wgpu::Extent3d { width: n, height: n, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    /// Un calque **statique** n'est rendu qu'une fois : la 2ᵉ frame réutilise sa
    /// texture (aucune nouvelle pré-passe). Changer son contenu force un
    /// re-render ; le supprimer purge le cache.
    #[test]
    fn static_layer_texture_is_reused_across_frames() {
        let Some((device, queue)) = headless() else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        // sample_count 1 : le cache est indépendant du MSAA.
        let mut painters = Painters::new(&device, &queue, format, 1);
        let tex = target(&device, format, 16);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let clear = Some(wgpu::Color::BLACK);

        let red = |s: &mut Scene| s.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(1.0, 0.0, 0.0));
        let mut scene = Scene::new();
        scene.layer(0.5, red);

        painters.render(&device, &queue, format, &view, 16, 16, &scene, clear);
        assert_eq!(painters.layer_render_count(), 1, "1re frame : calque rendu");
        painters.render(&device, &queue, format, &view, 16, 16, &scene, clear);
        assert_eq!(painters.layer_render_count(), 1, "calque inchangé : texture réutilisée");

        // Contenu changé → re-render.
        let mut scene2 = Scene::new();
        scene2.layer(0.5, |s| s.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(0.0, 1.0, 0.0)));
        painters.render(&device, &queue, format, &view, 16, 16, &scene2, clear);
        assert_eq!(painters.layer_render_count(), 2, "contenu changé : re-render");

        // Calque disparu → cache purgé (aucune pré-passe).
        painters.render(&device, &queue, format, &view, 16, 16, &Scene::new(), clear);
        assert_eq!(painters.layer_render_count(), 2, "plus de calque : rien à rendre");
        assert!(painters.layer_cache.is_empty(), "cache purgé");
    }
}
