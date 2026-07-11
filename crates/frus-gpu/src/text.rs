//! Rendu de texte via [`glyphon`](https://docs.rs/glyphon) (cosmic-text + atlas
//! de glyphes wgpu). Dessine les primitives [`Primitive::Text`] dans le render
//! pass, par-dessus les rectangles.

use frus_core::{Primitive, Scene};

/// Rapport interligne / taille de police (cohérent avec `frus-text`).
const LINE_HEIGHT_FACTOR: f32 = 1.2;

fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Détient l'état glyphon nécessaire au rendu de texte.
pub(crate) struct TextPainter {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    renderer: glyphon::TextRenderer,
}

impl TextPainter {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);
        let mut atlas = glyphon::TextAtlas::new(device, queue, &cache, format);
        let renderer =
            glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderer,
        }
    }

    /// Prépare le rendu du texte de la scène. À appeler avant le render pass.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        width: u32,
        height: u32,
    ) {
        self.viewport.update(queue, glyphon::Resolution { width, height });

        // Construit un buffer glyphon par primitive de texte.
        let mut buffers = Vec::new();
        for primitive in scene.primitives() {
            if let Primitive::Text {
                position,
                text,
                size,
                color,
                clip,
                ..
            } = primitive
            {
                let metrics = glyphon::Metrics::new(*size, *size * LINE_HEIGHT_FACTOR);
                let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                buffer.set_size(&mut self.font_system, Some(width as f32), Some(height as f32));
                buffer.set_text(
                    &mut self.font_system,
                    text,
                    glyphon::Attrs::new(),
                    glyphon::Shaping::Advanced,
                );
                buffer.shape_until_scroll(&mut self.font_system, false);

                // Cible sRGB : on envoie du linéaire (comme les quads) pour éviter
                // le double encodage (texte délavé). L'alpha reste tel quel.
                let linear = color.to_linear();
                let color = glyphon::Color::rgba(
                    to_u8(linear.r),
                    to_u8(linear.g),
                    to_u8(linear.b),
                    to_u8(color.a),
                );
                // Découpe = clip de la primitive, borné à la surface.
                let bounds = glyphon::TextBounds {
                    left: clip.x.max(0.0) as i32,
                    top: clip.y.max(0.0) as i32,
                    right: (clip.x + clip.width).min(width as f32) as i32,
                    bottom: (clip.y + clip.height).min(height as f32) as i32,
                };
                buffers.push((buffer, position.x, position.y, color, bounds));
            }
        }

        let areas = buffers
            .iter()
            .map(|(buffer, left, top, color, bounds)| glyphon::TextArea {
                buffer,
                left: *left,
                top: *top,
                scale: 1.0,
                bounds: *bounds,
                default_color: *color,
                custom_glyphs: &[],
            });

        if let Err(err) = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        ) {
            log::warn!("glyphon prepare a échoué : {err:?}");
        }
    }

    /// Dessine le texte préparé dans un render pass ouvert.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if let Err(err) = self.renderer.render(&self.atlas, &self.viewport, pass) {
            log::warn!("glyphon render a échoué : {err:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Point, Scene};

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

    /// Rend un texte blanc sur fond noir dans une texture offscreen et vérifie
    /// qu'il produit des pixels non-noirs : preuve que la rasterisation marche.
    #[test]
    fn renders_text_to_non_background_pixels() {
        let Some((device, queue)) = headless_device() else {
            eprintln!("aucun adaptateur GPU disponible : test ignoré");
            return;
        };

        const SIZE: u32 = 128; // 128 * 4 = 512, aligné pour la copie.
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.test.text_target"),
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

        let mut painter = TextPainter::new(&device, &queue, format);
        let mut scene = Scene::new();
        scene.text(Point::new(4.0, 4.0), "Hello", 48.0, Color::WHITE);
        painter.prepare_frame(&device, &queue, &scene, SIZE, SIZE);

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
            painter.draw(&mut pass);
        }

        let bytes_per_row = SIZE * 4;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.test.text_readback"),
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
        let lit_pixels = data.chunks_exact(4).filter(|px| px[0] > 16).count();

        assert!(
            lit_pixels > 0,
            "le texte devrait produire des pixels non-noirs (trouvés : {lit_pixels})"
        );
    }
}
