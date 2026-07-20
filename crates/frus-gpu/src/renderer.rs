//! Renderer lié à une surface (fenêtre) : configure wgpu et délègue le dessin
//! des primitives au [`Painter`].

use frus_core::Scene;

use crate::compositor::Painters;

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
    painters: Painters,
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

        let mut painters = Painters::new(&device, &queue, format);
        // Échauffe tous les pipelines avant la première vraie frame (anti-jank).
        painters.warm_up(&device, &queue, format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            painters,
        })
    }

    /// Reconfigure la surface après un redimensionnement de la fenêtre. Les
    /// viewports des painters sont réglés à chaque frame par [`Painters::render`].
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Réapplique la configuration courante (surface perdue/obsolète).
    pub fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Dessine la scène (rectangles, images, chemins, texte, calques) et la présente.
    pub fn render(&mut self, scene: &Scene) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.painters.render(
            &self.device,
            &self.queue,
            self.config.format,
            &view,
            self.config.width,
            self.config.height,
            scene,
            Some(CLEAR_COLOR),
        );

        frame.present();
        Ok(())
    }
}
