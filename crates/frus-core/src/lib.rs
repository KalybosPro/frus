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
mod hct;
mod image;
mod path;
mod responsive;
mod scene;
mod semantics;
mod text_style;

pub use animation::{
    Animatable, Animation, AnimationController, ClampedSimulation, Curve, Curved,
    FrictionSimulation, Lerp, Simulation, SpringDescription, SpringSimulation, Status, Tolerance,
    Tween, TweenSequence,
};
pub use color::Color;
pub use decoration::{Border, BorderRadius, BoxDecoration, BoxShadow, LinearGradient};
pub use geometry::{
    Affine, Alignment, AlignmentDirectional, AlignmentGeometry, Insets, InsetsDirectional, Point,
    Rect, Size, TextDirection, WindowInsets,
};
pub use hct::{Hct, TonalPalette};
pub use image::{BoxFit, ImageData, ImageHandle};
pub use path::{Path, PathVerb, Stroke};
pub use responsive::{Orientation, SizeClass};
pub use semantics::{Role, Semantics, Toggled};
pub use scene::{ClipShape, LayerTransform, Primitive, Scene};
pub use text_style::{FontWeight, TextDecoration, TextRun, TextSpan, TextStyle};
