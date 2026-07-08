//! `frus-layout` — moteur de mise en page de frus.
//!
//! Il transforme un **arbre de nœuds stylés** en **rectangles positionnés** (en
//! coordonnées absolues), prêts à être dessinés par `frus-gpu`.
//!
//! L'implémentation repose sur [`taffy`](https://docs.rs/taffy) (flexbox), mais
//! celui-ci est **entièrement caché** derrière l'API de frus : on pourra le
//! remplacer plus tard sans casser l'API publique.

mod style;
mod tree;

pub use style::{Dimension, FlexDirection, Style};
pub use tree::{Layout, NodeId};

// Ré-export des types géométriques du socle, par commodité pour les appelants.
pub use frus_core::{Rect, Size};
