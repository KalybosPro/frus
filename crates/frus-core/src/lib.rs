//! `frus-core` — types fondamentaux partagés par tout le framework.
//!
//! Ce crate ne contient **aucune logique** : uniquement des types de données
//! (géométrie, couleur) sans dépendance au rendu ni à la plateforme. Il sert de
//! socle commun à `frus-gpu`, `frus-layout`, etc., pour éviter la duplication et
//! le couplage entre les couches.

mod color;
mod geometry;
mod scene;

pub use color::Color;
pub use geometry::{Insets, Point, Rect, Size};
pub use scene::{Primitive, Scene};
