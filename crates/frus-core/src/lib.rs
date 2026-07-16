//! `frus-core` — types fondamentaux partagés par tout le framework.
//!
//! Ce crate ne contient **aucune logique** : uniquement des types de données
//! (géométrie, couleur) sans dépendance au rendu ni à la plateforme. Il sert de
//! socle commun à `frus-gpu`, `frus-layout`, etc., pour éviter la duplication et
//! le couplage entre les couches.

pub mod animation;
mod color;
mod decoration;
mod geometry;
mod responsive;
mod scene;

pub use animation::{
    AnimationController, ClampedSimulation, Curve, FrictionSimulation, Lerp, Simulation,
    SpringDescription, SpringSimulation, Status, Tolerance, Tween,
};
pub use color::Color;
pub use decoration::{Border, BoxDecoration, BoxShadow, LinearGradient};
pub use geometry::{Insets, Point, Rect, Size};
pub use responsive::{Orientation, SizeClass};
pub use scene::{Primitive, Scene};
