//! `frus-gpu` — contexte GPU et moteur de rendu 2D minimal.
//!
//! Ce crate ne dépend d'aucune bibliothèque de fenêtrage : il reçoit une
//! [`wgpu::SurfaceTarget`] (fournie par la couche plateforme, p. ex. `frus-shell`)
//! et se charge de tout le reste (device, queue, pipeline, présentation).
//!
//! Objectif du Jalon 0 : dessiner un quad coloré à l'écran.

// Ré-export pour que les couches supérieures manipulent les types wgpu
// (ex. `SurfaceError`) sans avoir à dépendre directement de `wgpu`.
pub use wgpu;

mod renderer;

pub use renderer::Renderer;
