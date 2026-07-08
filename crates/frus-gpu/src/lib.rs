//! `frus-gpu` — contexte GPU et moteur de rendu 2D.
//!
//! Ce crate ne dépend d'aucune bibliothèque de fenêtrage : il reçoit une
//! [`wgpu::SurfaceTarget`] (fournie par la couche plateforme, p. ex. `frus-shell`)
//! et se charge de tout le reste (device, queue, pipeline, présentation).
//!
//! On décrit ce qu'on veut dessiner dans une [`Scene`] (des [`Rect`] colorés),
//! puis on la remet à [`Renderer::render`].

// Ré-export pour que les couches supérieures manipulent les types wgpu
// (ex. `SurfaceError`) sans avoir à dépendre directement de `wgpu`.
pub use wgpu;

mod color;
mod geometry;
mod painter;
mod renderer;
mod scene;

pub use color::Color;
pub use geometry::Rect;
pub use renderer::Renderer;
pub use scene::Scene;
