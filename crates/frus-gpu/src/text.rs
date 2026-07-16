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
        // Police embarquée + repli système : même politique que la mesure texte
        // (`frus-text`), pour un rendu déterministe et un défaut résoluble partout.
        let font_system = frus_text::new_font_system();
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

        // Cible sRGB : on envoie du linéaire (comme les quads) pour éviter le
        // double encodage (texte délavé). L'alpha reste tel quel.
        let to_glyphon = |color: &frus_core::Color| {
            let linear = color.to_linear();
            glyphon::Color::rgba(
                to_u8(linear.r),
                to_u8(linear.g),
                to_u8(linear.b),
                to_u8(color.a),
            )
        };
        // Découpe = clip de la primitive, borné à la surface.
        let to_bounds = |clip: &frus_core::Rect| glyphon::TextBounds {
            left: clip.x.max(0.0) as i32,
            top: clip.y.max(0.0) as i32,
            right: (clip.x + clip.width).min(width as f32) as i32,
            bottom: (clip.y + clip.height).min(height as f32) as i32,
        };

        // Construit un buffer glyphon par primitive de texte (simple ou riche).
        let mut buffers = Vec::new();
        for primitive in scene.primitives() {
            match primitive {
                Primitive::Text {
                    position,
                    text,
                    size,
                    color,
                    weight,
                    italic,
                    max_width,
                    clip,
                    ..
                } => {
                    let metrics = glyphon::Metrics::new(*size, *size * LINE_HEIGHT_FACTOR);
                    let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                    // Un paragraphe se replie à sa largeur de mise en page ; un
                    // texte libre ne se replie qu'à la surface (jamais atteint).
                    let wrap_w = max_width.unwrap_or(width as f32);
                    buffer.set_size(&mut self.font_system, Some(wrap_w), Some(height as f32));
                    // Graisse + italique : cosmic-text choisit la face correspondante
                    // de la famille (repli sur la plus proche si absente).
                    let attrs = glyphon::Attrs::new()
                        .weight(glyphon::Weight(weight.to_u16()))
                        .style(if *italic {
                            glyphon::Style::Italic
                        } else {
                            glyphon::Style::Normal
                        });
                    buffer.set_text(&mut self.font_system, text, attrs, glyphon::Shaping::Advanced);
                    buffer.shape_until_scroll(&mut self.font_system, false);

                    buffers.push((buffer, position.x, position.y, to_glyphon(color), to_bounds(clip)));
                }
                Primitive::RichText {
                    position,
                    runs,
                    clip,
                    ..
                } => {
                    if runs.is_empty() {
                        continue;
                    }
                    // Métriques de base : le plus grand run (les runs plus petits
                    // portent leurs propres métriques par-span).
                    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);
                    let metrics = glyphon::Metrics::new(base, base * LINE_HEIGHT_FACTOR);
                    let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                    buffer.set_size(&mut self.font_system, Some(width as f32), Some(height as f32));
                    let spans = runs.iter().map(|run| {
                        (
                            run.text.as_str(),
                            glyphon::Attrs::new()
                                .weight(glyphon::Weight(run.weight.to_u16()))
                                .style(if run.italic {
                                    glyphon::Style::Italic
                                } else {
                                    glyphon::Style::Normal
                                })
                                .metrics(glyphon::Metrics::new(
                                    run.size,
                                    run.size * LINE_HEIGHT_FACTOR,
                                ))
                                .color(to_glyphon(&run.color)),
                        )
                    });
                    buffer.set_rich_text(
                        &mut self.font_system,
                        spans,
                        glyphon::Attrs::new(),
                        glyphon::Shaping::Advanced,
                    );
                    buffer.shape_until_scroll(&mut self.font_system, false);

                    // Chaque run porte sa couleur par attrs ; le défaut ne sert
                    // qu'aux glyphes sans couleur (il n'y en a pas).
                    let default_color = to_glyphon(&runs[0].color);
                    buffers.push((buffer, position.x, position.y, default_color, to_bounds(clip)));
                }
                Primitive::Rect { .. } => {}
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

    /// Rend `scene` (sur fond noir) dans une texture offscreen et compte les
    /// pixels « allumés » (canal rouge > 16). `None` si aucun GPU n'est dispo.
    fn lit_pixels_for(scene: &Scene) -> Option<usize> {
        let (device, queue) = headless_device()?;

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
        painter.prepare_frame(&device, &queue, scene, SIZE, SIZE);

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
        Some(data.chunks_exact(4).filter(|px| px[0] > 16).count())
    }

    /// Preuve de rasterisation : un texte blanc produit des pixels non-noirs.
    #[test]
    fn renders_text_to_non_background_pixels() {
        let mut scene = Scene::new();
        scene.text(Point::new(4.0, 4.0), "Hello", 48.0, Color::WHITE);
        match lit_pixels_for(&scene) {
            None => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
            Some(lit) => {
                assert!(lit > 0, "le texte devrait produire des pixels non-noirs ({lit})")
            }
        }
    }

    /// Le texte **riche** (`set_rich_text`, runs mêlés tailles/graisses) rasterise
    /// aussi — preuve de bout en bout du nouveau chemin GPU.
    #[test]
    fn renders_rich_text_to_non_background_pixels() {
        use frus_core::{FontWeight, TextRun};
        let run = |text: &str, size: f32, weight: FontWeight| TextRun {
            text: text.to_string(),
            size,
            weight,
            italic: false,
            color: Color::WHITE,
        };
        let mut scene = Scene::new();
        scene.rich_text(
            Point::new(4.0, 4.0),
            vec![run("Ri", 40.0, FontWeight::Regular), run("ch", 24.0, FontWeight::Bold)],
        );
        match lit_pixels_for(&scene) {
            None => eprintln!("aucun adaptateur GPU disponible : test ignoré"),
            Some(lit) => {
                assert!(lit > 0, "le texte riche devrait produire des pixels non-noirs ({lit})")
            }
        }
    }
}
