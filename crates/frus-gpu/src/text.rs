//! Text rendering through [`glyphon`](https://docs.rs/glyphon) — cosmic-text plus
//! a wgpu glyph atlas. Draws the [`Primitive::Text`] primitives in the render pass,
//! on top of the rectangles.

use std::ops::Range;

use crate::batch::{Batch, Kind};
use frus_core::{Color, Point, Primitive, Rect, Scene, TextDecoration};

/// The line-height to font-size ratio, kept consistent with `frus-text`.
const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// A **text decoration** quad — underline, strikethrough and so on — computed from
/// the laid-out lines. Rendered by the rectangle pipeline, *before* the glyphs: in
/// the same colour it is indistinguishable, and otherwise the text stays readable.
pub(crate) struct DecorationQuad {
    pub rect: Rect,
    pub color: Color,
    pub clip: Rect,
}

/// The underline's position below the baseline, as a fraction of the size.
const UNDERLINE_OFFSET: f32 = 0.12;
/// The strikethrough's position above the baseline, roughly half the x-height.
const STRIKETHROUGH_OFFSET: f32 = 0.28;
/// The overline's position, roughly the ascender height.
const OVERLINE_OFFSET: f32 = 0.90;

/// Emits the quads of one decorated line: `[x0, x1]` as advances from `origin`,
/// `baseline` relative to the top of the paragraph, thickness derived from `size`.
fn push_line_quads(
    quads: &mut Vec<DecorationQuad>,
    origin: Point,
    x0: f32,
    x1: f32,
    baseline: f32,
    size: f32,
    decoration: TextDecoration,
    color: Color,
    clip: Rect,
) {
    if x1 <= x0 {
        return;
    }
    let thickness = (size / 14.0).max(1.0);
    let mut line = |y: f32| {
        quads.push(DecorationQuad {
            rect: Rect::new(
                origin.x + x0,
                origin.y + baseline + y - thickness / 2.0,
                x1 - x0,
                thickness,
            ),
            color,
            clip,
        });
    };
    if decoration.underline {
        line(UNDERLINE_OFFSET * size);
    }
    if decoration.strikethrough {
        line(-STRIKETHROUGH_OFFSET * size);
    }
    if decoration.overline {
        line(-OVERLINE_OFFSET * size);
    }
}

fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Holds the glyphon state text rendering needs.
pub(crate) struct TextPainter {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    /// One renderer per text batch of the frame. Text is interleaved with the other
    /// primitives now (milestone 295), so a frame can need several — a glyphon
    /// renderer draws everything it was prepared with, in one call. Kept between
    /// frames and grown as needed; building one allocates a pipeline.
    renderers: Vec<glyphon::TextRenderer>,
    /// The multisample state the renderers must be built with: it has to match the
    /// pass, or the pipeline mismatches under MSAA.
    multisample: wgpu::MultisampleState,
}

impl TextPainter {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        // Bundled font plus system fallback — the same policy as text measurement
        // in `frus-text`, for deterministic rendering and a default that resolves
        // anywhere.
        let font_system = frus_text::new_font_system();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);
        let atlas = glyphon::TextAtlas::new(device, queue, &cache, format);
        let multisample = wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderers: Vec::new(),
            multisample,
        }
    }

    /// Makes sure there are at least `count` renderers to prepare into.
    fn ensure_renderers(&mut self, device: &wgpu::Device, count: usize) {
        while self.renderers.len() < count {
            self.renderers.push(glyphon::TextRenderer::new(
                &mut self.atlas,
                device,
                self.multisample,
                None,
            ));
        }
    }

    /// Prepares the scene's text for rendering; call this before the render pass.
    /// Returns the **decoration quads** — underline, strikethrough and so on —
    /// computed from the laid-out lines, to be drawn by the rectangle pipeline.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        width: u32,
        height: u32,
        batches: &[Batch],
    ) -> (Vec<DecorationQuad>, Vec<Range<u32>>) {
        self.viewport
            .update(queue, glyphon::Resolution { width, height });
        let text_batches = batches.iter().filter(|b| b.kind == Kind::Text).count();
        self.ensure_renderers(device, text_batches);
        let mut decorations = Vec::new();
        // One entry per batch; the decorations of a text batch, drawn by the
        // rectangle pipeline just under its glyphs.
        let mut decoration_ranges = Vec::with_capacity(batches.len());

        // The target is sRGB, so we send linear values — as the quads do — to avoid
        // encoding twice, which washes the text out. Alpha passes through as is.
        let to_glyphon = |color: &frus_core::Color| {
            let linear = color.to_linear();
            glyphon::Color::rgba(
                to_u8(linear.r),
                to_u8(linear.g),
                to_u8(linear.b),
                to_u8(color.a),
            )
        };
        // The clip is the primitive's, bounded to the surface.
        let to_bounds = |clip: &frus_core::Rect| glyphon::TextBounds {
            left: clip.x.max(0.0) as i32,
            top: clip.y.max(0.0) as i32,
            right: (clip.x + clip.width).min(width as f32) as i32,
            bottom: (clip.y + clip.height).min(height as f32) as i32,
        };

        // One glyphon buffer per text primitive, plain or rich — batch by batch, so
        // each batch's renderer is prepared with only its own.
        let mut slot = 0;
        for batch in batches {
            let decoration_start = decorations.len() as u32;
            if batch.kind != Kind::Text {
                decoration_ranges.push(decoration_start..decoration_start);
                continue;
            }
            let mut buffers = Vec::new();
            for &member in &batch.members {
            match &scene.primitives()[member] {
                Primitive::Text {
                    position,
                    text,
                    size,
                    color,
                    weight,
                    italic,
                    max_width,
                    decoration,
                    decoration_color,
                    clip,
                    ..
                } => {
                    let metrics = glyphon::Metrics::new(*size, *size * LINE_HEIGHT_FACTOR);
                    let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                    // A paragraph wraps at its layout width. Free text stays
                    // **unconstrained** (`None`) — and above all is not bounded to
                    // the surface: in RTL, cosmic-text right-aligns to the buffer's
                    // width, which would push the glyphs off screen past the right
                    // edge once `position.x` shifts them.
                    buffer.set_size(&mut self.font_system, *max_width, Some(height as f32));
                    // Weight and italic: cosmic-text picks the matching face of the
                    // family, falling back to the closest one when it is missing.
                    let attrs = glyphon::Attrs::new()
                        // Family by script (Arabic → Noto): Android has no
                        // cross-family fallback, so we choose at the source.
                        .family(frus_text::family_for(text))
                        .weight(glyphon::Weight(frus_text::available_weight(*weight)))
                        // Upright when no oblique face is loaded: an application
                        // that dropped `bundled-italic` gets straight text, not none.
                        .style(frus_text::available_style(*italic));
                    buffer.set_text(
                        &mut self.font_system,
                        text,
                        attrs,
                        glyphon::Shaping::Advanced,
                    );
                    buffer.shape_until_scroll(&mut self.font_system, false);

                    // Decorations: one line per layout run, from the first glyph
                    // advance to the last.
                    if !decoration.is_none() {
                        let deco_color = decoration_color.unwrap_or(*color);
                        for run in buffer.layout_runs() {
                            let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last())
                            else {
                                continue;
                            };
                            push_line_quads(
                                &mut decorations,
                                *position,
                                first.x,
                                last.x + last.w,
                                run.line_y,
                                *size,
                                *decoration,
                                deco_color,
                                *clip,
                            );
                        }
                    }

                    buffers.push((
                        buffer,
                        position.x,
                        position.y,
                        to_glyphon(color),
                        to_bounds(clip),
                    ));
                }
                Primitive::RichText {
                    position,
                    runs,
                    max_width,
                    clip,
                    ..
                } => {
                    if runs.is_empty() {
                        continue;
                    }
                    // Base metrics come from the largest run; smaller runs carry
                    // their own per-span metrics.
                    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);
                    let metrics = glyphon::Metrics::new(base, base * LINE_HEIGHT_FACTOR);
                    let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
                    // As for plain text: a rich paragraph wraps at its layout
                    // width, otherwise it is unconstrained (`None`) and never bounded
                    // to the surface, which would push RTL alignment off screen.
                    buffer.set_size(&mut self.font_system, *max_width, Some(height as f32));
                    let spans = runs.iter().enumerate().map(|(index, run)| {
                        (
                            run.text.as_str(),
                            glyphon::Attrs::new()
                                .family(frus_text::family_for(&run.text))
                                .weight(glyphon::Weight(frus_text::available_weight(run.weight)))
                                .style(frus_text::available_style(run.italic))
                                .metrics(glyphon::Metrics::new(
                                    run.size,
                                    run.size * LINE_HEIGHT_FACTOR,
                                ))
                                .color(to_glyphon(&run.color))
                                // Ties each glyph to its source run, for the
                                // per-span decorations.
                                .metadata(index),
                        )
                    });
                    buffer.set_rich_text(
                        &mut self.font_system,
                        spans,
                        glyphon::Attrs::new(),
                        glyphon::Shaping::Advanced,
                    );
                    buffer.shape_until_scroll(&mut self.font_system, false);

                    // Per-run decorations: consecutive glyphs sharing a run —
                    // through the metadata — form one decorated segment.
                    if runs.iter().any(|r| !r.decoration.is_none()) {
                        for lrun in buffer.layout_runs() {
                            let glyphs = lrun.glyphs;
                            let mut start = 0;
                            while start < glyphs.len() {
                                let meta = glyphs[start].metadata;
                                let mut end = start + 1;
                                while end < glyphs.len() && glyphs[end].metadata == meta {
                                    end += 1;
                                }
                                if let Some(run) = runs.get(meta) {
                                    if !run.decoration.is_none() {
                                        let last = &glyphs[end - 1];
                                        push_line_quads(
                                            &mut decorations,
                                            *position,
                                            glyphs[start].x,
                                            last.x + last.w,
                                            lrun.line_y,
                                            run.size,
                                            run.decoration,
                                            run.decoration_color.unwrap_or(run.color),
                                            *clip,
                                        );
                                    }
                                }
                                start = end;
                            }
                        }
                    }

                    // Every run carries its colour through attrs; the default only
                    // serves colourless glyphs, of which there are none.
                    let default_color = to_glyphon(&runs[0].color);
                    buffers.push((
                        buffer,
                        position.x,
                        position.y,
                        default_color,
                        to_bounds(clip),
                    ));
                }
                // Rectangles, paths, images and layers are none of text's business.
                Primitive::Rect { .. }
                | Primitive::Path { .. }
                | Primitive::Image { .. }
                | Primitive::Layer { .. } => {}
            }
            }
            decoration_ranges.push(decoration_start..decorations.len() as u32);

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

            if let Err(err) = self.renderers[slot].prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash_cache,
            ) {
                log::warn!("glyphon prepare failed: {err:?}");
            }
            slot += 1;
        }
        (decorations, decoration_ranges)
    }

    /// Draws one prepared text batch into an open render pass. `slot` is the batch's
    /// rank among the frame's text batches, which is the renderer it was prepared into.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, slot: usize) {
        let Some(renderer) = self.renderers.get(slot) else {
            return;
        };
        if let Err(err) = renderer.render(&self.atlas, &self.viewport, pass) {
            log::warn!("glyphon render failed: {err:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Point, Scene};

    /// Renders `scene` on a black background through the shared offscreen path and
    /// counts the "lit" pixels, those whose red channel is above 16. `None` when no
    /// GPU is available.
    fn lit_pixels_for(scene: &Scene) -> Option<usize> {
        let frame = crate::offscreen::render_offscreen(scene, 128, 128, Color::BLACK)?;
        Some(frame.rgba.chunks_exact(4).filter(|px| px[0] > 16).count())
    }

    /// Proof of rasterisation: white text produces non-black pixels.
    #[test]
    fn renders_text_to_non_background_pixels() {
        let mut scene = Scene::new();
        scene.text(Point::new(4.0, 4.0), "Hello", 48.0, Color::WHITE);
        match lit_pixels_for(&scene) {
            None => eprintln!("no GPU adapter available: test skipped"),
            Some(lit) => {
                assert!(lit > 0, "text should produce non-black pixels ({lit})")
            }
        }
    }

    /// **Arabic** rasterises through the bundled Naskh face — `family_for` routes
    /// the Arabic script to "Noto Naskh Arabic". Proof that the render path, and not
    /// only measurement, does shape Arabic glyphs.
    #[test]
    fn renders_arabic_to_non_background_pixels() {
        let mut scene = Scene::new();
        scene.text(Point::new(4.0, 40.0), "مهامي", 40.0, Color::WHITE);
        match lit_pixels_for(&scene) {
            None => eprintln!("no GPU adapter available: test skipped"),
            Some(lit) => {
                assert!(lit > 20, "Arabic should rasterise glyphs ({lit})")
            }
        }
    }

    /// **Rich** text (`set_rich_text`, runs mixing sizes and weights) rasterises
    /// too — end-to-end proof of the rich GPU path.
    #[test]
    fn renders_rich_text_to_non_background_pixels() {
        use frus_core::{FontWeight, TextRun};
        let run = |text: &str, size: f32, weight: FontWeight| TextRun {
            text: text.to_string(),
            size,
            weight,
            italic: false,
            color: Color::WHITE,
            decoration: TextDecoration::NONE,
            decoration_color: None,
        };
        let mut scene = Scene::new();
        scene.rich_text(
            Point::new(4.0, 4.0),
            vec![
                run("Ri", 40.0, FontWeight::Regular),
                run("ch", 24.0, FontWeight::Bold),
            ],
        );
        match lit_pixels_for(&scene) {
            None => eprintln!("no GPU adapter available: test skipped"),
            Some(lit) => {
                assert!(lit > 0, "rich text should produce non-black pixels ({lit})")
            }
        }
    }

    /// An **underline** lights more pixels than the same text bare — readback proof
    /// that the decoration quads travel the whole path: computed from the laid-out
    /// lines, then drawn by the rectangle pass.
    #[test]
    fn underline_lights_more_pixels_than_plain_text() {
        use frus_core::TextStyle;
        let plain = {
            let mut scene = Scene::new();
            scene.text_styled(
                Point::new(4.0, 4.0),
                "Hello",
                &TextStyle::new(40.0),
                Color::WHITE,
            );
            scene
        };
        let underlined = {
            let mut scene = Scene::new();
            scene.text_styled(
                Point::new(4.0, 4.0),
                "Hello",
                &TextStyle::new(40.0).underline(),
                Color::WHITE,
            );
            scene
        };
        match (lit_pixels_for(&plain), lit_pixels_for(&underlined)) {
            (Some(bare), Some(deco)) => {
                assert!(
                    deco > bare + 50,
                    "the underline must add pixels (bare {bare}, decorated {deco})"
                );
            }
            _ => eprintln!("no GPU adapter available: test skipped"),
        }
    }

    /// A rich text's **per-span strikethrough** adds pixels only on the decorated
    /// run, through the metadata → per-run segments path.
    #[test]
    fn rich_text_strikethrough_is_per_run() {
        use frus_core::{FontWeight, TextRun};
        let run = |text: &str, decoration: TextDecoration| TextRun {
            text: text.to_string(),
            size: 40.0,
            weight: FontWeight::Regular,
            italic: false,
            color: Color::WHITE,
            decoration,
            decoration_color: None,
        };
        let plain = {
            let mut scene = Scene::new();
            scene.rich_text(
                Point::new(4.0, 4.0),
                vec![
                    run("ab", TextDecoration::NONE),
                    run("cd", TextDecoration::NONE),
                ],
            );
            scene
        };
        let struck = {
            let mut scene = Scene::new();
            scene.rich_text(
                Point::new(4.0, 4.0),
                vec![
                    run("ab", TextDecoration::NONE),
                    run("cd", TextDecoration::STRIKETHROUGH),
                ],
            );
            scene
        };
        match (lit_pixels_for(&plain), lit_pixels_for(&struck)) {
            (Some(bare), Some(deco)) => {
                assert!(
                    deco > bare,
                    "run 2's strikethrough must add pixels (bare {bare}, decorated {deco})"
                );
            }
            _ => eprintln!("no GPU adapter available: test skipped"),
        }
    }
}
