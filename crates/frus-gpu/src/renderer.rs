//! Renderer lié à une surface (fenêtre) : configure wgpu et délègue le dessin
//! des primitives au [`Painter`].

use frus_core::Scene;

use crate::painter::Painter;
use crate::path::PathPainter;
use crate::text::TextPainter;

/// Couleur de fond (bleu nuit).
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.05,
    b: 0.08,
    a: 1.0,
};

/// Détient l'état GPU lié à une surface et présente les frames.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    painter: Painter,
    path: PathPainter,
    text: TextPainter,
}

impl Renderer {
    /// Initialise le contexte GPU pour une surface donnée.
    ///
    /// `target` est typiquement un `Arc<Window>` fourni par la couche plateforme.
    /// `width`/`height` doivent être > 0.
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(target)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("aucun adaptateur GPU compatible trouvé"))?;

        log::info!("Adaptateur GPU : {:?}", adapter.get_info());

        // Limites downlevel (compat GLES) mais avec la **résolution réelle** de
        // l'adaptateur : sur mobile, l'écran (ex. 1080×2340) dépasse la texture
        // max downlevel de 2048 — sans ça, `surface.configure` panique.
        let required_limits =
            wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("frus.device"),
                    required_features: wgpu::Features::empty(),
                    required_limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let painter = Painter::new(&device, format);
        painter.set_viewport(&queue, width as f32, height as f32);

        let path = PathPainter::new(&device, format);
        path.set_viewport(&queue, width as f32, height as f32);

        let text = TextPainter::new(&device, &queue, format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            painter,
            path,
            text,
        })
    }

    /// Reconfigure la surface après un redimensionnement de la fenêtre.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.painter.set_viewport(&self.queue, width as f32, height as f32);
            self.path.set_viewport(&self.queue, width as f32, height as f32);
        }
    }

    /// Réapplique la configuration courante (surface perdue/obsolète).
    pub fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Dessine la scène (rectangles + texte) et la présente.
    pub fn render(&mut self, scene: &Scene) -> Result<(), wgpu::SurfaceError> {
        // Préparation (téléversements) avant l'ouverture du render pass. Le
        // texte d'abord : il produit les quads de décoration (soulignement…)
        // que la passe des rectangles dessine sous les glyphes.
        let decorations = self.text.prepare_frame(
            &self.device,
            &self.queue,
            scene,
            self.config.width,
            self.config.height,
        );
        let rect_count =
            self.painter
                .prepare_frame(&self.device, &self.queue, scene, &decorations);
        let path_index_count = self.path.prepare_frame(&self.device, &self.queue, scene);

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frus.encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.painter.draw(&mut pass, rect_count);
            self.path.draw(&mut pass, path_index_count);
            self.text.draw(&mut pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
