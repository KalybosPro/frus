//! Animation : la couche **physique et cinématique** partagée par tout le
//! framework, sans dépendance au rendu ni à la plateforme.
//!
//! Trois briques découplées, sur le modèle éprouvé de Flutter :
//! - [`Simulation`] : des fonctions pures `temps → valeur` (ressort, friction,
//!   clamp). Le *quoi* physique.
//! - [`Curve`] / [`Tween`] : le *façonnage* d'une progression `[0,1]` et son
//!   application à une valeur typée.
//! - [`AnimationController`] : le *pilote* qui échantillonne une simulation frame
//!   par frame et expose valeur + [`Status`].
//!
//! Le shell instancie les contrôleurs par identité (`child_id`) ; les widgets
//! lisent la valeur courante au paint — cohérent avec « les widgets se thèment au
//! paint ».

mod controller;
mod curve;
mod simulation;
mod tween;

pub use controller::{AnimationController, Status};
pub use curve::Curve;
pub use simulation::{
    ClampedSimulation, FrictionSimulation, Simulation, SpringDescription, SpringSimulation,
    Tolerance,
};
pub use tween::{Lerp, Tween};
