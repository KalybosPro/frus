//! The renderer bound to a surface (a window): it configures wgpu and delegates
//! primitive drawing to the [`Painter`].

use frus_core::Scene;

use crate::compositor::{preferred_sample_count, Painters};

/// The background colour, a midnight blue.
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.05,
    b: 0.08,
    a: 1.0,
};

/// Holds the GPU state bound to a surface and presents the frames.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    painters: Painters,
}

impl Renderer {
    /// Initialises the GPU context for a given surface.
    ///
    /// `target` is typically an `Arc<Window>` supplied by the platform layer.
    /// `width` and `height` must both be > 0.
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
            .ok_or_else(|| anyhow::anyhow!("no compatible GPU adapter found"))?;

        log::info!("Adaptateur GPU : {:?}", adapter.get_info());

        // Downlevel limits, for GLES compatibility, but with the adapter's **real**
        // resolution: on mobile a screen of, say, 1080x2340 exceeds the downlevel
        // maximum texture size of 2048, and without this `surface.configure` panics.
        let required_limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());

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

        // MSAA when the adapter supports it for this format; otherwise 1, disabled.
        let sample_count = preferred_sample_count(&adapter, format);
        log::info!("MSAA: {sample_count}×");

        let mut painters = Painters::new(&device, &queue, format, sample_count);
        // Warms every pipeline before the first real frame, to avoid jank.
        painters.warm_up(&device, &queue, format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            painters,
        })
    }

    /// Reconfigures the surface after the window is resized. The painters' viewports
    /// are set every frame by `Painters::render`.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Reapplies the current configuration, after a lost or outdated surface.
    pub fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Draws the scene — rectangles, images, paths, text, layers — and presents it.
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
