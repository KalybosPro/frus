// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **Mipmaps** for image textures (milestone 743).
//!
//! An image drawn much smaller than its pixels covers many of them per screen pixel, and a
//! bilinear sample reads only four: thin lines vanish and fine patterns shimmer. A mip
//! chain holds the image pre-shrunk by halves, so the sampler reads the level whose
//! texels match the screen's.
//!
//! The chain is made **on the GPU**, one render pass per level, by `mipmap.wgsl`: an
//! sRGB render target and `textureLoad` work on every backend frus runs on, WebGL2
//! included. The average is taken in linear light and premultiplied by alpha.
//!
//! The pure arithmetic — how many levels, their sizes, which texels each one averages and
//! when an image needs a chain at all — lives here as plain functions, tested on the CPU.

use frus_core::Rect;

/// The format every image texture has: the chain is rendered into it.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// How many levels a full chain for a `width`×`height` image has: halving down to one
/// texel along the longer side, the image itself included.
pub(crate) fn level_count(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

#[cfg_attr(not(test), allow(dead_code))]
/// The size of `level`: each level half the one above, rounded down, never below one.
pub(crate) fn level_size(width: u32, height: u32, level: u32) -> (u32, u32) {
    let shrink = |v: u32| v.checked_shr(level).unwrap_or(0).max(1);
    (shrink(width), shrink(height))
}

/// Which texels of a source row (or column) `span` long texel `x` of the next level
/// averages: the first of up to three, and their weights. The same arithmetic as
/// `axis_taps` in `mipmap.wgsl`.
///
/// An even span averages pairs. An odd one, `2d + 1` down to `d`, is a box of width
/// `2 + 1/d`, which covers three texels in part, with weights `(d - x, d, x + 1)` over
/// `2d + 1`: every source texel counts, and counts as much as every other.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn axis_taps(span: u32, x: u32) -> (u32, [f32; 3]) {
    if span <= 1 {
        return (0, [1.0, 0.0, 0.0]);
    }
    if span.is_multiple_of(2) {
        return (2 * x, [0.5, 0.5, 0.0]);
    }
    let d = (span / 2) as f32;
    let xf = x as f32;
    let n = 2.0 * d + 1.0;
    (2 * x, [(d - xf) / n, d / n, (xf + 1.0) / n])
}

/// Whether a draw of `uv` (in `0..1`) of a `width`×`height` texture into `rect` shows it
/// **smaller** than its pixels along either axis: the draw reads below the first level,
/// so the image needs its chain.
///
/// Any shrinking counts, not only below one half: between one and one half the sampler
/// already blends in the second level, and a chain made only later would change the
/// picture the frame it appears.
pub(crate) fn minifies(rect: Rect, uv: Rect, width: u32, height: u32) -> bool {
    // A thousandth of slack, for a draw at its own size that rounding made a hair small.
    const SLACK: f32 = 1.0 - 1e-3;
    let texels_x = uv.width.abs() * width as f32;
    let texels_y = uv.height.abs() * height as f32;
    rect.width.abs() < texels_x * SLACK || rect.height.abs() < texels_y * SLACK
}

/// The pipeline that renders one level from the one above.
pub(crate) struct MipGenerator {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl MipGenerator {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frus.mipmap.shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mipmap.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.mipmap.bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.mipmap.pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.mipmap.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout }
    }

    /// Renders levels `1..levels` of `texture`, each from the one above, and submits
    /// them. Level 0 must already hold the image (a `write_texture` queued before is
    /// enough: the queue runs it first).
    pub(crate) fn generate(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        levels: u32,
    ) {
        if levels <= 1 {
            return;
        }
        let view_of = |level: u32| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("frus.mipmap.level"),
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.mipmap.encoder"),
        });
        for level in 1..levels {
            let from = view_of(level - 1);
            let to = view_of(level);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("frus.mipmap.bind_group"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&from),
                }],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frus.mipmap.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &to,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chain_halves_down_to_one_texel_along_the_longer_side() {
        assert_eq!(level_count(1, 1), 1);
        assert_eq!(level_count(2, 2), 2);
        assert_eq!(level_count(256, 256), 9);
        assert_eq!(level_count(257, 3), 9);
        assert_eq!(level_count(511, 512), 10);
        assert_eq!(level_count(1, 1024), 11);
        assert_eq!(
            level_count(0, 0),
            1,
            "an empty image is one level, not zero"
        );
    }

    #[test]
    fn every_level_is_half_the_one_above_rounded_down_never_below_one() {
        assert_eq!(level_size(256, 64, 0), (256, 64));
        assert_eq!(level_size(256, 64, 1), (128, 32));
        assert_eq!(level_size(256, 64, 6), (4, 1));
        assert_eq!(level_size(256, 64, 8), (1, 1));
        assert_eq!(level_size(5, 3, 1), (2, 1));
        assert_eq!(level_size(5, 3, 2), (1, 1));
        // A shift past the width of a u32 is one texel, not a panic.
        assert_eq!(level_size(7, 7, 40), (1, 1));
        // The last level of a full chain is one texel along the longer side.
        for (w, h) in [(1, 1), (3, 5), (640, 480), (1000, 7), (4096, 4096)] {
            let last = level_size(w, h, level_count(w, h) - 1);
            assert_eq!(last.0.max(last.1), 1, "{w}x{h}");
        }
    }

    #[test]
    fn a_chain_costs_a_third_more_memory_at_most() {
        for (w, h) in [(1024, 1024), (640, 480), (1000, 7), (3, 3)] {
            let texels = |level| {
                let (lw, lh) = level_size(w, h, level);
                u64::from(lw) * u64::from(lh)
            };
            let base = texels(0);
            let chain: u64 = (0..level_count(w, h)).map(texels).sum();
            // A third for a square power of two; a few texels more where sizes round to one.
            assert!(
                chain * 3 <= base * 4 + 3 * u64::from(level_count(w, h)),
                "{w}x{h}"
            );
        }
    }

    /// Every source texel is counted, and counted as much as every other: the weights
    /// laid over the source row add up to the same amount everywhere.
    #[test]
    fn the_taps_cover_every_source_texel_equally() {
        for span in 1..40u32 {
            let next = (span / 2).max(1);
            let mut coverage = vec![0.0f32; span as usize];
            for x in 0..next {
                let (first, weights) = axis_taps(span, x);
                let total: f32 = weights.iter().sum();
                assert!(
                    (total - 1.0).abs() < 1e-5,
                    "span {span}, x {x}: sum {total}"
                );
                for (i, w) in weights.iter().enumerate() {
                    if *w > 0.0 {
                        coverage[(first as usize + i).min(span as usize - 1)] += w;
                    }
                }
            }
            let each = next as f32 / span as f32;
            for (t, c) in coverage.iter().enumerate() {
                assert!((c - each).abs() < 1e-5, "span {span}: texel {t} counts {c}");
            }
        }
    }

    #[test]
    fn only_a_draw_smaller_than_the_pixels_needs_the_chain() {
        let whole = Rect::new(0.0, 0.0, 1.0, 1.0);
        // At its own size, or larger: no.
        assert!(!minifies(
            Rect::new(0.0, 0.0, 256.0, 256.0),
            whole,
            256,
            256
        ));
        assert!(!minifies(
            Rect::new(0.0, 0.0, 512.0, 300.0),
            whole,
            256,
            256
        ));
        // A hair small from rounding: no.
        assert!(!minifies(
            Rect::new(0.0, 0.0, 255.9, 256.0),
            whole,
            256,
            256
        ));
        // Smaller along either axis: yes.
        assert!(minifies(Rect::new(0.0, 0.0, 32.0, 32.0), whole, 256, 256));
        assert!(minifies(Rect::new(0.0, 0.0, 256.0, 200.0), whole, 256, 256));
        // A crop is measured in the texels it shows: half the picture into half the
        // size is its own size.
        let half = Rect::new(0.25, 0.25, 0.5, 0.5);
        assert!(!minifies(Rect::new(0.0, 0.0, 128.0, 128.0), half, 256, 256));
        assert!(minifies(Rect::new(0.0, 0.0, 64.0, 64.0), half, 256, 256));
        // A mirrored draw has a negative uv width.
        let mirrored = Rect::new(1.0, 0.0, -1.0, 1.0);
        assert!(minifies(
            Rect::new(0.0, 0.0, 32.0, 256.0),
            mirrored,
            256,
            256
        ));
        assert!(!minifies(
            Rect::new(0.0, 0.0, 256.0, 256.0),
            mirrored,
            256,
            256
        ));
    }
}
