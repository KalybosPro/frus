//! Animation: the **physics and kinematics** layer shared by the whole framework,
//! with no dependency on rendering or on any platform.
//!
//! Three decoupled building blocks:
//! - [`Simulation`]: pure `time → value` functions (spring, friction, clamp). The
//!   physical *what*.
//! - [`Curve`] / [`Tween`]: the *shaping* of a `[0,1]` progress value, and its
//!   application to a typed value.
//! - [`AnimationController`]: the *driver*, which samples a simulation frame by
//!   frame and exposes a value plus a [`Status`].
//!
//! The shell instantiates controllers by identity (`child_id`); widgets read the
//! current value at paint time — consistent with "widgets theme themselves at
//! paint".

mod controller;
mod curve;
mod simulation;
mod tween;
mod velocity;

pub use controller::{AnimationController, Status};
pub use curve::Curve;
pub use simulation::{
    BouncingScrollSimulation, ClampedSimulation, ClampingScrollSimulation, FrictionSimulation,
    Simulation, SpringDescription, SpringSimulation, Tolerance, BOUNCING_DRAG, CLAMPING_FRICTION,
    MAX_SPRING_TRANSFER_VELOCITY,
};
pub use tween::{Animatable, Animation, Curved, Lerp, Tween, TweenSequence};
pub use velocity::{
    PolynomialFit, Velocity, VelocityEstimate, VelocityStrategy, VelocityTracker,
    BOUNCING_FLING_WEIGHTS, DESKTOP_FLING_WEIGHTS,
};
