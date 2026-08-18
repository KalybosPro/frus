//! `frus-layout` — frus's layout engine.
//!
//! It turns a **tree of styled nodes** into **positioned rectangles**, in absolute
//! coordinates, ready for `frus-gpu` to draw.
//!
//! The implementation rests on [`taffy`](https://docs.rs/taffy) for flexbox, but
//! taffy is **entirely hidden** behind frus's own API, so it can be replaced later
//! without breaking anything public.

mod style;
mod tree;

pub use style::{Align, Dimension, FlexDirection, Justify, Style};
pub use tree::{Layout, MeasureFn, NodeId, Overflowing, Side};

// Re-export of the core geometry types, as a convenience for callers.
pub use frus_core::{Rect, Size};
