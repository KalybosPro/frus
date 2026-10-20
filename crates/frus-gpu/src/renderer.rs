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

/// **The window's background** where nothing is drawn — the clear colour, as a scene colour —
/// for a [`see-through`](Renderer::see_through) renderer, whose frames start transparent and
/// whose owner paints it where the window is not meant to be seen through (milestone 643).
pub fn backdrop() -> frus_core::Color {
    // The clear colour is linear; a scene colour is sRGB.
    frus_core::Color::rgb(
        CLEAR_COLOR.r as f32,
        CLEAR_COLOR.g as f32,
        CLEAR_COLOR.b as f32,
    )
    .to_srgb()
}

/// Where the compiled pipelines are kept between launches, where the device can keep them
/// (Vulkan): set by the shell — on Android the application's own storage — before the
/// renderer is made (milestone 685, #217).
static PIPELINE_CACHE_DIR: std::sync::Mutex<Option<std::path::PathBuf>> =
    std::sync::Mutex::new(None);

/// Keeps the renderer's compiled pipelines in `dir` between launches, where the device can: the
/// next start reads them instead of compiling them again (Vulkan; nothing elsewhere).
pub fn set_pipeline_cache_dir(dir: std::path::PathBuf) {
    if let Ok(mut slot) = PIPELINE_CACHE_DIR.lock() {
        *slot = Some(dir);
    }
}

/// Holds the GPU state bound to a surface and presents the frames.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// The format the frames are drawn in: the surface's own when it is sRGB, its sRGB twin
    /// (a view of the same image) when the surface offers none, as a WebGPU canvas (#234).
    render_format: wgpu::TextureFormat,
    painters: Painters,
    /// What making it took, phase by phase.
    startup: StartupTimes,
    /// Where the last frame's rendering spent its time.
    timings: RenderTimings,
    /// The GPU's own timing of each frame, where the device has a clock (milestone 612).
    gpu_timer: Option<crate::gpu_timer::GpuTimer>,
    /// Whether the frames are composed with what is behind the window (milestone 643).
    see_through: bool,
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
        Self::make(target, width, height, false).await
    }

    /// **A renderer whose frames let what is behind the window show** where they are
    /// transparent (milestone 643): its frames start transparent, are composed with the
    /// window's surroundings by the system's compositor, and are made of premultiplied
    /// colours — which is what this renderer's blending produces over a transparent start.
    ///
    /// On Windows it presents through DirectComposition (Direct3D 12): where a frame is
    /// transparent on the title bar's line, the system's own caption shows — its backdrop
    /// and its three buttons. Fails where that is not available — another system, no
    /// Direct3D 12 adapter, no premultiplied composition — and the caller then makes an
    /// ordinary [`new`](Self::new) one. Its owner paints [`backdrop`] wherever the window is
    /// not meant to be seen through.
    pub async fn see_through(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self> {
        Self::make(target, width, height, true).await
    }

    /// Whether this renderer's frames let what is behind the window show where they are
    /// transparent: made by [`see_through`](Self::see_through).
    pub fn sees_through(&self) -> bool {
        self.see_through
    }

    async fn make(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        see_through: bool,
    ) -> anyhow::Result<Self> {
        if see_through && !cfg!(windows) {
            anyhow::bail!("a see-through window is composed by DirectComposition, on Windows");
        }
        let started = web_time::Instant::now();
        let mut descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        };
        if see_through {
            // Presented through a DirectComposition visual made from the window: the one
            // way a Direct3D swap chain is composed with what is behind it.
            descriptor.backends = wgpu::Backends::DX12;
            descriptor.backend_options.dx12.presentation_system =
                wgpu::Dx12SwapchainKind::DxgiFromVisual;
        }
        // No debug names for GPU objects on Android: a debug build asks for them, and the
        // emulator's Vulkan driver crashes in `vkSetDebugUtilsObjectNameEXT` as the device is
        // made (a release build never asks). `WGPU_DEBUG=1` still asks for them on a device.
        #[cfg(target_os = "android")]
        if std::env::var_os("WGPU_DEBUG").is_none() {
            descriptor.flags.remove(wgpu::InstanceFlags::DEBUG);
        }
        // The Android emulator's Vulkan driver (gfxstream) crashes in its own code at the first
        // submission: there, the GPU is reached through OpenGL ES instead.
        #[cfg(target_os = "android")]
        if emulator_vulkan(descriptor.flags).await {
            log::info!("the emulator's Vulkan driver fails at the first frame: OpenGL ES instead");
            descriptor.backends = wgpu::Backends::GL;
        }
        let instance = wgpu::Instance::new(descriptor);

        let surface = instance.create_surface(target)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // The discrete GPU where there is a choice — but not in a browser, which
                // ignores the hint and **says so** in the console (Chrome, on Windows) on
                // every page load, for something the page cannot change.
                power_preference: if cfg!(target_arch = "wasm32") {
                    wgpu::PowerPreference::None
                } else {
                    wgpu::PowerPreference::HighPerformance
                },
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await?;

        log::info!("GPU adapter: {:?}", adapter.get_info());

        // Downlevel limits, for GLES compatibility, but with the adapter's **real**
        // resolution: on mobile a screen of, say, 1080x2340 exceeds the downlevel
        // maximum texture size of 2048, and without this `surface.configure` panics.
        // A GPU without compute shaders (OpenGL ES 3.0, as the Android emulator's) cannot give
        // even the downlevel compute limits: the WebGL 2 ones, which ask for none.
        let downlevel = if adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
        {
            wgpu::Limits::downlevel_defaults()
        } else {
            wgpu::Limits::downlevel_webgl2_defaults()
        };
        let required_limits = downlevel.using_resolution(adapter.limits());

        let adapter_time = started.elapsed();
        let device_started = web_time::Instant::now();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("frus.device"),
                // The GPU's own clock where it has one, for the frame statistics
                // (milestone 612). Nothing is asked of a device that has not.
                required_features: adapter.features()
                    & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::PIPELINE_CACHE),
                required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await?;
        let device_time = device_started.elapsed();

        let caps = surface.get_capabilities(&adapter);
        let alpha_mode = if see_through {
            if !caps
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
            {
                anyhow::bail!(
                    "no premultiplied composition on this surface: {:?}",
                    caps.alpha_modes
                );
            }
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            caps.alpha_modes[0]
        };
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        // The shaders write linear values, which an sRGB target encodes. A WebGPU canvas
        // offers only plain formats (`bgra8unorm`): drawn as they are, the linear values are
        // shown as if encoded, and every colour comes out darker (#234). The canvas lets its
        // image be drawn through an sRGB view: the frames are drawn in that twin, and the
        // hardware encodes, as everywhere else.
        let render_format = format.add_srgb_suffix();
        let view_formats = if render_format == format {
            vec![]
        } else {
            vec![render_format]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            // **Fifo, always** (milestone 610): one image per refresh of the display, in
            // order. It was the first mode the surface offered, and on the test phone that
            // was Mailbox, under which the application draws as fast as it can, the
            // display shows whichever image is latest, and the rest are drawn for nothing:
            // frames at 14 ms intervals on a 16.7 ms display, the battery spent on images
            // never seen, and a motion that is not paced by the display. Fifo is the one
            // mode every surface supports.
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats,
            desired_maximum_frame_latency: 2,
        };
        log::info!(
            "present mode: {:?}, of {:?}",
            config.present_mode,
            caps.present_modes
        );
        surface.configure(&device, &config);

        // MSAA when the adapter supports it for this format; otherwise 1, disabled.
        let sample_count = preferred_sample_count(&adapter, render_format);
        log::info!("MSAA: {sample_count}×");

        let pipelines_started = web_time::Instant::now();
        let (pipeline_cache, cache_file, cached) = open_pipeline_cache(&adapter, &device);
        let mut painters = Painters::new(
            &device,
            &queue,
            render_format,
            sample_count,
            pipeline_cache.as_ref(),
        );
        let pipelines = pipelines_started.elapsed();
        // Warms every pipeline before the first real frame, to avoid jank.
        let warm_up_started = web_time::Instant::now();
        painters.warm_up(&device, &queue, render_format);
        if let (Some(cache), Some(file)) = (pipeline_cache.as_ref(), cache_file.as_ref()) {
            keep_pipeline_cache(cache, file, cached.as_deref());
        }
        let startup = StartupTimes {
            adapter: adapter_time,
            device: device_time,
            pipelines,
            warm_up: warm_up_started.elapsed(),
        };

        let gpu_timer = crate::gpu_timer::GpuTimer::new(&device, &queue);
        log::info!(
            "GPU clock: {}",
            if gpu_timer.is_some() {
                "frames timed on the GPU"
            } else {
                "none, frames timed on the CPU only"
            }
        );
        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_format,
            gpu_timer,
            painters,
            startup,
            timings: RenderTimings::default(),
            see_through,
        })
    }

    /// What making this renderer took, phase by phase (#211).
    pub fn startup_times(&self) -> StartupTimes {
        self.startup
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

    /// Where the last [`render`](Self::render) spent its time, in milliseconds: waiting for
    /// a surface image to draw into, drawing into it (tessellating, encoding and submitting),
    /// and handing it to the display (milestone 609). Zero on the web, which has no clock
    /// here.
    pub fn last_timings(&self) -> RenderTimings {
        self.timings
    }

    /// Draws the scene — rectangles, images, paths, text, layers — and presents it.
    pub fn render(&mut self, scene: &Scene) -> RenderOutcome {
        let clock = Clock::start();
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return RenderOutcome::Skipped;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                return RenderOutcome::NeedsReconfigure;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                log::warn!("frame skipped: surface validation error");
                return RenderOutcome::Skipped;
            }
        };
        let acquired = clock.lap();
        let timed = self
            .gpu_timer
            .as_mut()
            .and_then(|timer| timer.begin(&self.device, &self.queue));
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });

        self.painters.render(
            &self.device,
            &self.queue,
            self.render_format,
            &view,
            self.config.width,
            self.config.height,
            scene,
            // A see-through frame starts transparent: what its owner paints is all there is.
            Some(if self.see_through {
                wgpu::Color::TRANSPARENT
            } else {
                CLEAR_COLOR
            }),
        );

        if let (Some(timer), Some(at)) = (self.gpu_timer.as_mut(), timed) {
            timer.end(&self.device, &self.queue, at);
        }
        let drawn = clock.lap();
        self.queue.present(frame);
        let presented = clock.lap();
        self.timings = RenderTimings {
            acquire: acquired,
            draw: drawn - acquired,
            present: presented - drawn,
            gpu: self.gpu_timer.as_ref().and_then(|timer| timer.last_ms()),
        };
        RenderOutcome::Presented
    }
}

/// Whether the only Vulkan this device offers is the Android emulator's (gfxstream, the
/// "Goldfish" GPU), which crashes in its own code at the first submission. `WGPU_BACKEND`, when
/// set, decides instead.
#[cfg(target_os = "android")]
async fn emulator_vulkan(flags: wgpu::InstanceFlags) -> bool {
    if std::env::var_os("WGPU_BACKEND").is_some() {
        return false;
    }
    let probe = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        flags,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = probe.enumerate_adapters(wgpu::Backends::VULKAN).await;
    !adapters.is_empty()
        && adapters.iter().all(|adapter| {
            let info = adapter.get_info();
            info.driver.eq_ignore_ascii_case("gfxstream") || info.name.contains("Goldfish")
        })
}

/// Where a frame's rendering spent its time, in milliseconds. See
/// [`Renderer::last_timings`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderTimings {
    /// Waiting for a surface image: time the display, not the drawing, decides.
    pub acquire: f32,
    /// Tessellating, encoding and submitting the scene.
    pub draw: f32,
    /// Handing the image to the display.
    pub present: f32,
    /// What the GPU took over the latest frame it has finished, by its own clock, a few
    /// frames behind: `None` where the device has no clock (milestone 612).
    pub gpu: Option<f32>,
}

/// Milliseconds since it started; nothing on the web, where `std` has no clock.
struct Clock {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

impl Clock {
    fn start() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    }

    fn lap(&self) -> f32 {
        #[cfg(not(target_arch = "wasm32"))]
        return self.start.elapsed().as_secs_f32() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        0.0
    }
}

/// What happened when [`Renderer::render`] tried to present a frame.
pub enum RenderOutcome {
    /// The frame was drawn and presented.
    Presented,
    /// The frame was skipped (occlusion, timeout, or a validation error); the caller
    /// should just try again on the next redraw.
    Skipped,
    /// The surface is lost or outdated: call [`Renderer::reconfigure`] before retrying.
    NeedsReconfigure,
}

/// The device's pipeline cache, filled from the file a previous launch wrote for this adapter
/// and driver, where the device has one and a folder was given: the cache, its file, and what
/// was read from it.
fn open_pipeline_cache(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
) -> (
    Option<wgpu::PipelineCache>,
    Option<std::path::PathBuf>,
    Option<Vec<u8>>,
) {
    if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        return (None, None, None);
    }
    let dir = PIPELINE_CACHE_DIR.lock().ok().and_then(|slot| slot.clone());
    let Some(file) = dir
        .zip(wgpu::util::pipeline_cache_key(&adapter.get_info()))
        .map(|(dir, key)| dir.join(format!("frus-{key}")))
    else {
        return (None, None, None);
    };
    let cached = std::fs::read(&file).ok();
    // SAFETY: the data is what `PipelineCache::get_data` gave on a previous launch, kept in a
    // file named for this adapter and driver; wgpu checks its header against the device, and
    // `fallback` starts an empty cache when it does not match.
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("frus.pipelines"),
            data: cached.as_deref(),
            fallback: true,
        })
    };
    log::info!(
        "pipeline cache: {}",
        match &cached {
            Some(data) => format!("{} KB read from the previous launch", data.len() / 1024),
            None => "none yet: the pipelines are compiled, and kept for the next launch".into(),
        }
    );
    (Some(cache), Some(file), cached)
}

/// The cache written back to its file when it holds more than what was read.
fn keep_pipeline_cache(cache: &wgpu::PipelineCache, file: &std::path::Path, read: Option<&[u8]>) {
    let Some(data) = cache.get_data() else {
        return;
    };
    if read == Some(data.as_slice()) {
        return;
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Written aside, then moved over: a launch that stops halfway leaves no half a cache.
    let partial = file.with_extension("partial");
    if std::fs::write(&partial, &data).is_ok() {
        let _ = std::fs::rename(&partial, file);
    }
}

/// What making the renderer took, phase by phase (milestone 683, #211): a part of an
/// application's time to its first frame. See [`Renderer::startup_times`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StartupTimes {
    /// The GPU instance, the surface and the adapter.
    pub adapter: std::time::Duration,
    /// The device and its queue.
    pub device: std::time::Duration,
    /// The pipelines and resources every frame draws with.
    pub pipelines: std::time::Duration,
    /// A frame drawn before the first real one, which builds what drivers build lazily.
    pub warm_up: std::time::Duration,
}
