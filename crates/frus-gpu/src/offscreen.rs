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
    use frus_core::{
        Backdrop, ColorFilter, ImageFilter, LayerFilter, MaskShader, Path, Point, Primitive, Rect,
        ShaderMask,
    };

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

    /// Builds a layer around one filled rectangle, carrying `filter`.
    fn filtered(rect: Rect, color: Color, filter: LayerFilter) -> Scene {
        let mut inner = Scene::new();
        inner.fill_rect(rect, color);
        let mut scene = Scene::new();
        scene.push_primitive(Primitive::Layer {
            primitives: inner.primitives().to_vec(),
            opacity: 1.0,
            clip: Rect::UNBOUNDED,
            clip_shape: frus_core::ClipShape::Rect,
            transform: None,
            filter,
            owner: 0,
        });
        scene
    }

    /// A **colour matrix**, sampled on the rendered pixel rather than believed from
    /// the scene: pure red through a greyscale filter must come out at the luminance
    /// of red, which is 0.2126 — and 0.2126 of the **encoded** value, not of the
    /// light. Evaluated in linear light the same matrix would give 0.48, more than
    /// twice as bright, which is exactly the slip this asserts against.
    #[test]
    fn a_greyscale_filter_gives_the_luminance_of_red() {
        let scene = filtered(
            Rect::new(0.0, 0.0, 64.0, 64.0),
            Color::rgb(1.0, 0.0, 0.0),
            LayerFilter {
                color: Some(ColorFilter::grayscale()),
                ..LayerFilter::NONE
            },
        );
        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let i = ((32 * frame.width + 32) * 4) as usize;
        let (r, g, b) = (frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]);
        let want = (0.2126f32 * 255.0).round() as i32;
        assert!(
            (r as i32 - want).abs() <= 3,
            "red at its luminance: {r} (wanted about {want})"
        );
        assert_eq!((r, g, b), (r, r, r), "grey: the three channels agree");
    }

    /// A **blur** spreads a shape past its own edge and takes the middle with it: a
    /// pixel outside the rectangle lights up, and the very centre stays bright.
    #[test]
    fn a_blur_spreads_past_the_edge() {
        let square = Rect::new(24.0, 24.0, 16.0, 16.0);
        let sharp = filtered(square, Color::WHITE, LayerFilter::NONE);
        let blurred = filtered(
            square,
            Color::WHITE,
            LayerFilter {
                image: Some(ImageFilter::blur(5.0)),
                ..LayerFilter::NONE
            },
        );
        let Some(a) = render_offscreen(&sharp, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let b = render_offscreen(&blurred, 64, 64, Color::BLACK).expect("the adapter is there");
        let at = |f: &OffscreenFrame, x: u32, y: u32| f.rgba[((y * f.width + x) * 4) as usize];
        // Eight pixels outside the square, level with its middle.
        assert_eq!(at(&a, 16, 32), 0, "sharp: nothing outside the square");
        assert!(
            at(&b, 16, 32) > 20,
            "blurred: the edge has reached here ({})",
            at(&b, 16, 32)
        );
        // And the spread came from somewhere: the centre is no longer full white.
        assert_eq!(at(&a, 32, 32), 255, "sharp: white in the middle");
        assert!(
            at(&b, 32, 32) < 255,
            "blurred: the middle gave some away ({})",
            at(&b, 32, 32)
        );
    }

    /// A **mask** fades a white block from opaque at its top to gone at its bottom.
    /// The middle is the test that matters: it must sit near the halfway point
    /// between the two ends, not collapse toward the background — which is what a
    /// coverage applied twice would do.
    #[test]
    fn a_mask_fades_the_layer_from_top_to_bottom() {
        let scene = filtered(
            Rect::new(0.0, 0.0, 64.0, 64.0),
            Color::WHITE,
            LayerFilter {
                mask: Some(ShaderMask::new(MaskShader::Linear {
                    from: Point::new(0.0, 0.0),
                    to: Point::new(0.0, 64.0),
                    from_color: Color::WHITE,
                    to_color: Color::WHITE.fade(0.0),
                })),
                ..LayerFilter::NONE
            },
        );
        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let at = |y: u32| frame.rgba[((y * frame.width + 32) * 4) as usize] as i32;
        let (top, middle, bottom) = (at(1), at(32), at(62));
        assert!(top > 240, "opaque at the top: {top}");
        // A fifty-fifth of the light, and still 42 out of 255 once encoded: the sRGB
        // curve spends most of its range on the dark end, which is why a fade that
        // looks finished is not, and why a byte threshold here has to be generous.
        assert!(bottom < 60, "all but gone at the bottom: {bottom}");
        assert!(
            top > middle && middle > bottom,
            "monotonic: {top} {middle} {bottom}"
        );
        // Half of white over black is 0.5 of the light, which is 188 encoded — not
        // the 137 a doubled coverage would give.
        assert!(
            (middle - 188).abs() <= 12,
            "half way is half the light: {middle}"
        );
    }
    /// A **backdrop** filters what is already painted, not the layer that carries it.
    ///
    /// The scene is a hard black/white split down the middle with a backdrop blur over
    /// the bottom half. Along the seam, the top half must stay a clean edge and the
    /// bottom half must be a gradient — which is the whole claim: the blur reached
    /// pixels that were painted by something else entirely.
    #[test]
    fn a_backdrop_blurs_what_is_underneath_it() {
        let mut scene = Scene::new();
        // The picture underneath: white on the left, nothing (the clear colour) right.
        scene.fill_rect(Rect::new(0.0, 0.0, 32.0, 64.0), Color::WHITE);
        // A backdrop over the bottom half only, with no content of its own.
        scene.push_primitive(Primitive::Layer {
            primitives: Vec::new(),
            opacity: 1.0,
            clip: Rect::new(0.0, 32.0, 64.0, 32.0),
            clip_shape: frus_core::ClipShape::Rect,
            transform: None,
            filter: LayerFilter {
                backdrop: Some(Backdrop::blur(6.0)),
                ..LayerFilter::NONE
            },
            owner: 0,
        });
        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let at = |x: u32, y: u32| frame.rgba[((y * frame.width + x) * 4) as usize] as i32;
        // Above the backdrop: the seam is untouched, black one side and white the other.
        assert_eq!(at(36, 16), 0, "above: still the clear colour");
        assert_eq!(at(28, 16), 255, "above: still white");
        // Below it: the same two places have bled into each other.
        assert!(
            at(36, 48) > 20,
            "below: the white has spread right ({})",
            at(36, 48)
        );
        assert!(
            at(28, 48) < 255,
            "below: the white has given some away ({})",
            at(28, 48)
        );
    }

    /// The backdrop is bounded by the layer's clip and by nothing else: outside it,
    /// the frame is exactly what it was.
    #[test]
    fn a_backdrop_stops_at_its_clip() {
        let mut scene = Scene::new();
        // The top half white, the bottom the clear colour: an erode has something to
        // eat only where the two meet.
        scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 32.0), Color::WHITE);
        scene.push_primitive(Primitive::Layer {
            primitives: Vec::new(),
            opacity: 1.0,
            clip: Rect::new(0.0, 0.0, 20.0, 64.0),
            clip_shape: frus_core::ClipShape::Rect,
            transform: None,
            filter: LayerFilter {
                // An erode eats the shape inwards, so its effect is unmistakable
                // against a flat fill and cannot be confused with a rounding error.
                backdrop: Some(Backdrop {
                    filter: ImageFilter::Erode {
                        radius_x: 8.0,
                        radius_y: 8.0,
                    },
                    ..Backdrop::blur(0.0)
                }),
                ..LayerFilter::NONE
            },
            owner: 0,
        });
        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let at = |x: u32, y: u32| frame.rgba[((y * frame.width + x) * 4) as usize] as i32;
        // Four pixels above the seam, inside the clip: eaten by the black below it.
        assert!(
            at(2, 28) < 255,
            "inside the clip, the white was eaten ({})",
            at(2, 28)
        );
        // The same place outside the clip: untouched white.
        assert_eq!(at(40, 28), 255, "outside the clip, nothing happened");
    }

    /// Two backdrops sharing a key are filtered **once**: they read the same copy, so
    /// the second one cannot see what the first one drew. That is the price of the
    /// sharing and the reason overlapping backdrops must not share a key — and it is
    /// also the only observable proof that the sharing happened at all.
    #[test]
    fn a_shared_key_filters_once() {
        let backdrop = |key: Option<u64>, y: f32| Primitive::Layer {
            primitives: Vec::new(),
            opacity: 1.0,
            clip: Rect::new(0.0, y, 64.0, 32.0),
            clip_shape: frus_core::ClipShape::Rect,
            transform: None,
            filter: LayerFilter {
                backdrop: Some(Backdrop {
                    key,
                    ..Backdrop::blur(6.0)
                }),
                ..LayerFilter::NONE
            },
            owner: 0,
        };
        // The two backdrops cover the same region. Unshared, the second blurs the
        // first's output and the seam softens twice; shared, both blur the same copy
        // and it softens once.
        let build = |key: Option<u64>| {
            let mut scene = Scene::new();
            scene.fill_rect(Rect::new(0.0, 0.0, 32.0, 64.0), Color::WHITE);
            scene.push_primitive(backdrop(key, 0.0));
            scene.push_primitive(backdrop(key, 0.0));
            scene
        };
        let Some(twice) = render_offscreen(&build(None), 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let once = render_offscreen(&build(Some(7)), 64, 64, Color::BLACK).expect("adapter");
        let at = |f: &OffscreenFrame, x: u32| f.rgba[((16 * f.width + x) * 4) as usize] as i32;
        // Twelve pixels past the seam: two blurs reach further than one.
        assert!(
            at(&twice, 44) > at(&once, 44),
            "unshared blurs twice ({}) and shared blurs once ({})",
            at(&twice, 44),
            at(&once, 44)
        );
    }
    /// A layer **inside** a layer is composited, not dropped.
    ///
    /// A group is rendered into a texture of its own and that texture is composited;
    /// a layer found inside it is not a primitive the group can paint, so for a long
    /// time it was simply skipped — a rounded card around a fading group, a clip
    /// around a transform, gone. The group now renders its nested layers first and
    /// composites them into its own pass, and this is the test that says so.
    #[test]
    fn a_layer_inside_a_layer_is_drawn() {
        let mut inner = Scene::new();
        inner.fill_rect(Rect::new(8.0, 8.0, 48.0, 48.0), Color::WHITE);
        let group = Primitive::Layer {
            primitives: inner.primitives().to_vec(),
            // A group opacity, so the test also proves the nested layer keeps its own
            // compositing rather than being flattened into the parent.
            opacity: 0.5,
            clip: Rect::UNBOUNDED,
            clip_shape: frus_core::ClipShape::Rect,
            transform: None,
            filter: LayerFilter::NONE,
            owner: 0,
        };
        let mut scene = Scene::new();
        scene.push_primitive(Primitive::Layer {
            primitives: vec![group],
            opacity: 1.0,
            clip: Rect::new(0.0, 0.0, 64.0, 64.0),
            clip_shape: frus_core::ClipShape::Rect,
            transform: None,
            filter: LayerFilter::NONE,
            owner: 0,
        });
        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let at = |x: u32, y: u32| frame.rgba[((y * frame.width + x) * 4) as usize] as i32;
        // Half of white over black is half the light, which is 188 once encoded.
        assert!(
            (at(32, 32) - 188).abs() <= 12,
            "the nested group is drawn, at its own opacity: {}",
            at(32, 32)
        );
        assert_eq!(at(2, 2), 0, "and only where it is");
    }

    /// Three deep, with the middle one clipped to an ellipse: the recursion is not a
    /// special case for one level.
    #[test]
    fn nesting_goes_as_deep_as_the_scene_does() {
        let mut leaf = Scene::new();
        leaf.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), Color::WHITE);
        let level = |primitives: Vec<Primitive>, shape: frus_core::ClipShape| Primitive::Layer {
            primitives,
            opacity: 1.0,
            clip: Rect::new(0.0, 0.0, 64.0, 64.0),
            clip_shape: shape,
            transform: None,
            filter: LayerFilter::NONE,
            owner: 0,
        };
        let inner = level(leaf.primitives().to_vec(), frus_core::ClipShape::Rect);
        let middle = level(vec![inner], frus_core::ClipShape::Oval);
        let mut scene = Scene::new();
        scene.push_primitive(level(vec![middle], frus_core::ClipShape::Rect));
        let Some(frame) = render_offscreen(&scene, 64, 64, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let at = |x: u32, y: u32| frame.rgba[((y * frame.width + x) * 4) as usize] as i32;
        assert_eq!(at(32, 32), 255, "the middle of the disc is white");
        assert_eq!(at(2, 2), 0, "its corner is not: the ellipse clipped it");
    }
}
