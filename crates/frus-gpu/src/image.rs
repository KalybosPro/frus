// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! The `ImagePainter`: image rendering ([`Primitive::Image`]) and the **texture
//! cache**.
//!
//! Each image is uploaded to a GPU texture **once** (in sRGB), cached by
//! [`frus_core::ImageData::id`] and reused from frame to frame. Textures left
//! unused during a frame are evicted. Drawing issues one instanced quad per image,
//! with the texture bound in group 1; UV, tint and clip all ride on the instance.
//!
//! The sampler follows the draw's [`FilterQuality`] (milestone 743). An image drawn
//! smaller than its pixels at `Medium` or `High` gets a **mip chain** the first time it
//! is: its texture is made again with every level, rendered by [`MipGenerator`], and
//! sampled trilinearly from then on. An image only ever drawn at its size or larger
//! keeps one level and the memory a chain would cost.

use std::ops::Range;

use crate::batch::{Batch, Kind};
use crate::mipmap::{self, MipGenerator};
use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use frus_core::{FilterQuality, Primitive, Rect, Scene};
use wgpu::util::DeviceExt;

/// A vertex of the unit quad.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

impl QuadVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];
const QUAD_VERTEX_COUNT: u32 = 6;

/// An instance: destination rectangle, UV, tint, clip, and the region sampling stays in.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    rect: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
    clip: [f32; 4],
    /// Low and high corners, in `0..1`, of what sampling may reach: the `uv` region
    /// itself for a strict draw, the whole texture otherwise (milestone 645).
    region: [f32; 4],
}

/// The corners sampling may reach for a draw of `uv`.
fn region(uv: Rect, strict: bool) -> [f32; 4] {
    if !strict {
        return [0.0, 0.0, 1.0, 1.0];
    }
    // A mirrored region has a negative width.
    let (x0, x1) = (uv.x.min(uv.x + uv.width), uv.x.max(uv.x + uv.width));
    let (y0, y1) = (uv.y.min(uv.y + uv.height), uv.y.max(uv.y + uv.height));
    [x0, y0, x1, y1]
}

impl Instance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Viewport {
    size: [f32; 2],
    _pad: [f32; 2],
}

/// The samplers, one per way of filtering; see [`sampler_index`].
const SAMPLERS: usize = 3;

/// Which of the painter's samplers a [`FilterQuality`] reads through.
fn sampler_index(quality: FilterQuality) -> usize {
    match quality {
        FilterQuality::None => 0,
        FilterQuality::Low => 1,
        FilterQuality::Medium | FilterQuality::High => 2,
    }
}

/// A cached texture, with a bind group per sampler made the first time it is drawn so.
struct CachedTexture {
    view: wgpu::TextureView,
    /// How many mip levels it holds: one, or the full chain.
    levels: u32,
    bind_groups: [Option<wgpu::BindGroup>; SAMPLERS],
    /// A use marker for the current frame, which drives eviction.
    used: bool,
}

const INITIAL_INSTANCE_CAPACITY: usize = 32;

/// Holds the pipeline, the texture cache and the instance buffer.
pub(crate) struct ImagePainter {
    pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    /// Nearest, bilinear on the first level, trilinear: see [`sampler_index`].
    samplers: [wgpu::Sampler; SAMPLERS],
    /// Made the first time an image needs a chain.
    mipmaps: Option<MipGenerator>,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    /// The texture cache, keyed by image id.
    cache: HashMap<u64, CachedTexture>,
    /// The CPU-side instance buffer, reused across frames.
    instances: Vec<Instance>,
    /// Each instance's image id and sampler, in order, so the right texture is bound
    /// at draw time.
    frame_ids: Vec<(u64, usize)>,
}

impl ImagePainter {
    /// Builds the painter for a given target format.
    /// `sample_count` is the MSAA sample count; 1 means no multisampling.
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frus.image.shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
        });

        let viewport_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.image.viewport"),
            size: std::mem::size_of::<Viewport>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let viewport_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.image.viewport.bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frus.image.viewport.bind_group"),
            layout: &viewport_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frus.image.texture.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = |label, filter, mipmap_filter, lod_max_clamp| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter,
                lod_min_clamp: 0.0,
                lod_max_clamp,
                ..Default::default()
            })
        };
        // `None` and `Low` never leave the first level, even of a texture that a
        // `Medium` draw gave a chain.
        let samplers = [
            sampler(
                "frus.image.sampler.nearest",
                wgpu::FilterMode::Nearest,
                wgpu::MipmapFilterMode::Nearest,
                0.0,
            ),
            sampler(
                "frus.image.sampler.linear",
                wgpu::FilterMode::Linear,
                wgpu::MipmapFilterMode::Nearest,
                0.0,
            ),
            sampler(
                "frus.image.sampler.trilinear",
                wgpu::FilterMode::Linear,
                wgpu::MipmapFilterMode::Linear,
                32.0,
            ),
        ];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frus.image.pipeline_layout"),
            bind_group_layouts: &[Some(&viewport_layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frus.image.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(QuadVertex::layout()), Some(Instance::layout())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache,
        });

        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frus.image.quad_vertex_buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frus.image.instance_buffer"),
            size: (INITIAL_INSTANCE_CAPACITY * std::mem::size_of::<Instance>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            viewport_buffer,
            viewport_bind_group,
            texture_layout,
            samplers,
            mipmaps: None,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            cache: HashMap::new(),
            instances: Vec::new(),
            frame_ids: Vec::new(),
        }
    }

    /// Updates the surface size.
    pub(crate) fn set_viewport(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let viewport = Viewport {
            size: [width.max(1.0), height.max(1.0)],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));
    }

    /// Uploads any new textures and builds the instances. Returns each batch's range of
    /// instances to draw.
    ///
    /// Eviction waits for [`ImagePainter::end_frame`]: layers prepare their own images
    /// first, several times a frame, and must not evict each other's.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        batches: &[Batch],
    ) -> Vec<Range<u32>> {
        self.instances.clear();
        self.frame_ids.clear();

        // Filled **in batch order**, so a batch is one contiguous run of instances.
        let mut ranges = Vec::with_capacity(batches.len());
        for batch in batches {
            let start = self.instances.len() as u32;
            if batch.kind != Kind::Image {
                ranges.push(start..start);
                continue;
            }
            for &member in &batch.members {
                if let Primitive::Image {
                    image,
                    rect,
                    uv,
                    tint,
                    clip,
                    strict,
                    filter_quality,
                    ..
                } = &scene.primitives()[member]
                {
                    let id = image.id();
                    let chain = filter_quality.uses_mipmaps()
                        && mipmap::minifies(*rect, *uv, image.width(), image.height());
                    let sampler = sampler_index(*filter_quality);
                    self.ensure_texture(device, queue, image, chain, sampler);
                    self.instances.push(Instance {
                        rect: rect.to_array(),
                        uv: uv.to_array(),
                        tint: tint.to_array(),
                        clip: clip.to_array(),
                        region: region(*uv, *strict),
                    });
                    self.frame_ids.push((id, sampler));
                }
            }
            ranges.push(start..self.instances.len() as u32);
        }

        let count = self.instances.len();
        if count == 0 {
            return ranges;
        }
        if count > self.instance_capacity {
            let new_capacity = count.next_power_of_two();
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frus.image.instance_buffer"),
                size: (new_capacity * std::mem::size_of::<Instance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = new_capacity;
        }
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
        ranges
    }

    /// Ends a frame: evicts the textures no draw of the frame used. Called once a frame,
    /// after the frame's last `prepare_frame`.
    pub(crate) fn end_frame(&mut self) {
        self.cache.retain(|_, c| c.used);
        for cached in self.cache.values_mut() {
            cached.used = false;
        }
    }

    /// Makes sure an image's texture is cached, with its mip chain when `chain` asks for
    /// one, and has a bind group for `sampler`; marks it used.
    fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &frus_core::ImageHandle,
        chain: bool,
        sampler: usize,
    ) {
        let id = image.id();
        let levels = if chain {
            mipmap::level_count(image.width(), image.height())
        } else {
            1
        };
        // A texture with fewer levels than asked for is made again: the chain needs
        // every level allocated up front.
        let stale = self.cache.get(&id).is_none_or(|c| c.levels < levels);
        // The samplers to bind it with: this draw's, and those earlier draws of the frame
        // bound the texture being replaced with, which they still look up by id.
        let mut wanted = [false; SAMPLERS];
        wanted[sampler] = true;
        if stale {
            if let Some(old) = self.cache.get(&id) {
                for (want, bound) in wanted.iter_mut().zip(&old.bind_groups) {
                    *want |= bound.is_some();
                }
            }
            let view = self.upload(device, queue, image, levels);
            self.cache.insert(
                id,
                CachedTexture {
                    view,
                    levels,
                    bind_groups: [None, None, None],
                    used: true,
                },
            );
        }
        let cached = self.cache.get_mut(&id).expect("cached above");
        cached.used = true;
        for (index, want) in wanted.into_iter().enumerate() {
            if !want || cached.bind_groups[index].is_some() {
                continue;
            }
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("frus.image.texture.bind_group"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&cached.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.samplers[index]),
                    },
                ],
            });
            cached.bind_groups[index] = Some(bind_group);
        }
    }

    /// Uploads an image into a new texture of `levels` mip levels, renders the levels
    /// past the first, and answers the view the draws sample.
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &frus_core::ImageHandle,
        levels: u32,
    ) -> wgpu::TextureView {
        let (w, h) = (image.width(), image.height());
        let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        if levels > 1 {
            // The levels past the first are rendered into.
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frus.image.texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: mipmap::FORMAT,
            usage,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image.rgba(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        if levels > 1 {
            self.mipmaps
                .get_or_insert_with(|| MipGenerator::new(device))
                .generate(device, queue, &texture, levels);
        }
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Draws the images: one instanced quad each, with the texture bound per draw.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, range: Range<u32>) {
        if range.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        // One call per image even inside a batch: each carries its own texture.
        for inst in range {
            let (id, sampler) = self.frame_ids[inst as usize];
            let bind_group = self
                .cache
                .get(&id)
                .and_then(|c| c.bind_groups[sampler].as_ref());
            if let Some(bind_group) = bind_group {
                pass.set_bind_group(1, bind_group, &[]);
                pass.draw(0..QUAD_VERTEX_COUNT, inst..inst + 1);
            }
        }
    }
}
