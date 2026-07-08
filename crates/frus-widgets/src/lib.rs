//! `frus-widgets` — l'arbre de widgets déclaratif de frus.
//!
//! On décrit l'interface avec des widgets composables ([`Container`], [`Flex`])
//! ; [`build_scene`] les traduit en mise en page (via `frus-layout`) puis en
//! [`Scene`] dessinable (via `frus-core`).
//!
//! Ce jalon couvre la **structure déclarative** et le pipeline
//! widget → layout → peinture. L'état et les interactions viendront ensuite.

mod container;
mod flex;
mod widget;

pub use container::Container;
pub use flex::Flex;
pub use widget::{build_scene, Widget};

// Ré-exports de commodité pour les appelants.
pub use frus_core::{Color, Rect, Scene, Size};
