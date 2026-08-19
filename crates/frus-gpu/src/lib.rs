//! `frus-gpu` — the GPU context and the 2D rendering engine.
//!
//! This crate depends on no windowing library: it is handed a
//! [`wgpu::SurfaceTarget`] by the platform layer — `frus-shell`, say — and takes
//! care of everything else: device, queue, pipeline, presentation.
//!
//! What is to be drawn is described in a [`Scene`] — coloured [`Rect`]s and the
//! rest — which is then handed to [`Renderer::render`].

// Re-exported so the layers above can handle wgpu types (`SurfaceError`, for one)
// without depending on `wgpu` directly.
pub use wgpu;

mod batch;
mod compositor;
mod filter;
mod image;
mod offscreen;
mod painter;
mod path;
mod renderer;
mod text;

/// How many draw calls a scene's rectangles, images and paths will cost.
///
/// The renderer draws them interleaved, in the scene's order wherever they cover one
/// another, and batched wherever they do not — so this number says how well a screen
/// batches. A frame of real interface should be a handful; one draw call per widget
/// means something is breaking every batch, and that is worth noticing before a
/// device does. Text and layers are not counted: they have passes of their own.
pub fn draw_calls(scene: &Scene) -> usize {
    batch::plan(scene).len()
}

// The data types — geometry, colour, scene — come from the shared foundation.
pub use frus_core::{Color, Rect, Scene};
pub use offscreen::{render_offscreen, OffscreenFrame};
pub use renderer::Renderer;
