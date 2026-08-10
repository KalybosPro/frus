//! `frus-core` — the fundamental types shared by the whole framework.
//!
//! This crate holds **no logic**: only data types (geometry, colour) with no
//! dependency on rendering or on any platform. It is the common foundation for
//! `frus-gpu`, `frus-layout` and the rest, which keeps those layers from
//! duplicating types or coupling to one another.

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
    Animatable, Animation, AnimationController, BouncingScrollSimulation, ClampedSimulation,
    ClampingScrollSimulation, Curve, Curved, FrictionSimulation, Lerp, Simulation,
    SpringDescription, SpringSimulation, Status, Tolerance, Tween, TweenSequence, BOUNCING_DRAG,
    CLAMPING_FRICTION, MAX_SPRING_TRANSFER_VELOCITY,
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
pub use scene::{ClipShape, LayerTransform, Primitive, Scene};
pub use semantics::{Role, Semantics, Toggled};
pub use text_style::{FontWeight, TextDecoration, TextRun, TextSpan, TextStyle};
