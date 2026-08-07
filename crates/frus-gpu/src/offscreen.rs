//! **Offscreen** rendering: the scene is drawn into a texture then read back by
//! the CPU, with no window required. This is what the render tests are built on —
//! the `frus-test` goldens — and the only way to observe the pipeline under WSL or
//! CI.
//!
//! The pipeline is **the same** as the windowed one ([`crate::Renderer`]): quads,
//! text decoration quads included, then glyphs, onto an sRGB target — the bytes
//! read back match what a screenshot would give.

use frus_core::{Color, Scene};

use crate::compositor::Painters;

/// A frame rendered offscreen: **sRGB** RGBA bytes, row by row.
pub struct OffscreenFrame {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes in RGBA order, origin at the top left.
    pub rgba: Vec<u8>,
    /// The MSAA sample count actually used; 1 means no smoothing, on a GPU without
    /// MSAA support. Informative, and useful to the anti-aliasing tests.
    pub samples: u32,
}

/// Renders `scene` into a `width`×`height` texture cleared to `clear`, then reads
/// the pixels back. `None` when no GPU adapter is available — a machine with neither
/// GPU nor software rasteriser — in which case test callers skip themselves.
pub fn render_offscreen(
    scene: &Scene,
    width: u32,
    height: u32,
    clear: Color,
) -> Option<OffscreenFrame> {
    let (device, queue, sample_count) = headless_device()?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frus.offscreen.target"),
        size: wgpu::Extent3d {
            width,
            height,
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

    // The same pipeline as `Renderer::render`, through the compositor, layers
    // included. The clear colour is authored in sRGB, and an sRGB target wants linear.
    let clear_linear = clear.to_linear();
    let mut painters = Painters::new(&device, &queue, format, sample_count);
    painters.render(
        &device,
        &queue,
        format,
        &view,
        width,
        height,
        scene,
        Some(wgpu::Color {
            r: clear_linear.r as f64,
            g: clear_linear.g as f64,
            b: clear_linear.b as f64,
            a: clear.a as f64,
        }),
    );

    // Readback: wgpu requires rows aligned to 256 bytes, so we pad and then strip
    // the padding on the CPU side.
    let unpadded_bytes_per_row = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("frus.offscreen.readback"),
        size: (padded_bytes_per_row * height) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frus.offscreen.copy"),
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
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;

    let data = slice.get_mapped_range();
    let mut rgba = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        rgba.extend_from_slice(&data[start..start + unpadded_bytes_per_row as usize]);
    }

    Some(OffscreenFrame {
        width,
        height,
        rgba,
        samples: sample_count,
    })
}

/// Creates a surfaceless wgpu device, along with the MSAA sample count supported
/// for the sRGB target. `None` when there is no adapter.
fn headless_device() -> Option<(wgpu::Device, wgpu::Queue, u32)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let sample_count =
        crate::compositor::preferred_sample_count(&adapter, wgpu::TextureFormat::Rgba8UnormSrgb);
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("frus.offscreen.device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .ok()?;
    Some((device, queue, sample_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Path, Point, Rect};

    /// The offscreen path renders what the windowed pipeline does: a solid red
    /// rectangle gives a red pixel inside, and the clear colour outside.
    #[test]
    fn renders_rect_and_reads_back_srgb() {
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 20.0, 20.0), Color::rgb(1.0, 0.0, 0.0));
        // A width that is not a multiple of 64 exercises the readback padding.
        let Some(frame) = render_offscreen(&scene, 70, 40, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        assert_eq!(frame.rgba.len(), 70 * 40 * 4);
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        assert_eq!(px(10, 10), [255, 0, 0], "inside the rect → red");
        assert_eq!(px(60, 30), [0, 0, 0], "outside the rect → clear");
    }

    /// A **vector triangle** filled with green: a point well inside is green, and a
    /// corner outside the triangle keeps the clear colour. Proof that tessellation and
    /// the path pipeline do produce pixels.
    #[test]
    fn fills_a_vector_triangle() {
        // A triangle covering the bottom of the surface: apex at the top centre,
        // base spanning the full width along the bottom.
        let triangle = Path::new()
            .move_to(Point::new(32.0, 4.0))
            .line_to(Point::new(60.0, 60.0))
            .line_to(Point::new(4.0, 60.0))
            .close();
        let mut scene = Scene::new();
        scene.fill_path(&triangle, Color::rgb(0.0, 1.0, 0.0));

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // Near the base, centred: well inside → green.
        assert_eq!(px(32, 54), [0, 255, 0], "inside the triangle → green");
        // Top-left corner: above and beside the apex → outside → clear.
        assert_eq!(px(4, 4), [0, 0, 0], "outside the triangle → clear");
    }

    /// A path's **stroke** paints pixels on the outline but not in the middle, since
    /// the path is not filled.
    #[test]
    fn strokes_a_path_outline_only() {
        // A centred 40×40 square, stroked and not filled.
        let square = Path::rect(Rect::new(12.0, 12.0, 40.0, 40.0));
        let mut scene = Scene::new();
        scene.stroke_path(&square, Color::rgb(0.0, 0.0, 1.0), 4.0);

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // On the square's left edge → blue; in the middle → clear, since no fill.
        assert_eq!(px(12, 32), [0, 0, 255], "on the outline → blue");
        assert_eq!(px(32, 32), [0, 0, 0], "in the middle → clear, not filled");
    }

    /// Texture sampling: a 2×2 image — red, green, blue, white — stretched (`Fill`)
    /// over the whole surface, so each quadrant reads its own colour. Proof of the
    /// upload, the sampling and the UV mapping.
    #[test]
    fn samples_a_texture_by_quadrant() {
        use frus_core::{BoxFit, ImageData};
        // 2×2: (0,0) red, (1,0) green, (0,1) blue, (1,1) white.
        let pixels = vec![
            255, 0, 0, 255, /* R */ 0, 255, 0, 255, /* G */
            0, 0, 255, 255, /* B */ 255, 255, 255, 255, /* W */
        ];
        let image = ImageData::from_rgba(2, 2, pixels).into_handle();
        let mut scene = Scene::new();
        scene.image(&image, Rect::new(0.0, 0.0, 64.0, 64.0), BoxFit::Fill);

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // A point well inside each quadrant, away from the interpolated edges.
        assert_eq!(px(10, 10), [255, 0, 0], "top-left → red");
        assert_eq!(px(54, 10), [0, 255, 0], "top-right → green");
        assert_eq!(px(10, 54), [0, 0, 255], "bottom-left → blue");
        assert_eq!(px(54, 54), [255, 255, 255], "bottom-right → white");
    }

    /// **Anti-aliasing (MSAA)**: a triangle's **diagonal** edge produces
    /// **partially** covered pixels, blending background and shape — the signature of
    /// multisampling, impossible with hard-edged rendering. The test skips itself when
    /// the GPU does not support MSAA (`samples == 1`).
    #[test]
    fn msaa_smooths_a_diagonal_edge() {
        // A right triangle whose hypotenuse lies on the anti-diagonal x + y = 64.
        let triangle = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(64.0, 0.0))
            .line_to(Point::new(0.0, 64.0))
            .close();
        let mut scene = Scene::new();
        scene.fill_path(&triangle, Color::rgb(0.0, 1.0, 0.0));

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        if frame.samples == 1 {
            eprintln!("MSAA unsupported by this GPU: test skipped");
            return;
        }
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        // Along the diagonal edge at least one pixel must be an **in-between**
        // green, neither full 255 nor background 0 — the proof of smoothing. Hard-edged
        // rendering would only produce 0 or 255. We sweep the whole surface: only the
        // partially covered edge pixels come out in between.
        let found_partial = (0..frame.height).any(|y| {
            (0..frame.width).any(|x| {
                let g = px(x, y)[1];
                g > 20 && g < 235
            })
        });
        assert!(
            found_partial,
            "a smoothed diagonal edge has in-between green pixels"
        );
        // Deep inside the triangle → full green; far outside → black background.
        assert_eq!(px(4, 4), [0, 255, 0], "inside → full green");
        assert_eq!(px(60, 60), [0, 0, 0], "outside → background");
    }

    /// **Layer compositing**: two **opaque** red rectangles that overlap, grouped in
    /// a layer at opacity 0.5. The group alpha is **uniform** — the overlap has the
    /// same colour as a single coverage, with no double-blending — and it does come
    /// out at about half red over the black background.
    #[test]
    fn layer_group_opacity_is_uniform_over_overlap() {
        let mut scene = Scene::new();
        scene.layer(0.5, |inner| {
            inner.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), Color::rgb(1.0, 0.0, 0.0));
            inner.fill_rect(Rect::new(24.0, 24.0, 40.0, 40.0), Color::rgb(1.0, 0.0, 0.0));
        });

        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let px = |x: u32, y: u32| {
            let i = ((y * frame.width + x) * 4) as usize;
            [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
        };
        let single = px(8, 8); // covered by the first rectangle only
        let overlap = px(32, 32); // covered by both
        assert_eq!(
            single, overlap,
            "the group alpha is uniform across the overlap"
        );
        // About 50% red over black: neither full red (255) nor background (0).
        assert!(
            single[0] > 120 && single[0] < 215,
            "about half red (R={})",
            single[0]
        );
        assert_eq!(single[1], 0, "no green");
        assert_eq!(single[2], 0, "no blue");
        // Outside both rectangles: black background.
        assert_eq!(px(60, 8), [0, 0, 0], "outside the layer → background");
    }
}
