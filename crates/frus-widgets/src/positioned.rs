//! [`Positioned`]: places a layer of a [`crate::Stack`] against the stack's own edges,
//! instead of filling it.
//!
//! A badge on a corner, a caption over an image, a button floating above content — all of
//! them are a stack with one layer pinned somewhere. Without this a layer can only fill
//! the box, and the only way to put something in a corner was to give it a transparent
//! wrapper the size of the stack and align inside it.
//!
//! Each edge is optional, and what is given decides both the **size** and the **place**,
//! exactly as in the reference:
//!
//! - `left` **and** `right` → the width is what is between them;
//! - one of them plus [`Positioned::width`] → that width, at that edge;
//! - neither → the child's own width, placed by the stack's alignment.
//!
//! The vertical axis works the same way with `top`, `bottom` and [`Positioned::height`].
//! Giving all three of a triple is a contradiction, and the extra one is ignored: the two
//! edges win, since they are what says where the box is.
//!
//! ```ignore
//! Stack::new()
//!     .layer(photo)
//!     .layer(Positioned::new(badge).top(8.0).right(8.0))
//! ```

use frus_layout::Style;

use crate::widget::Widget;

/// What a [`Positioned`] pins, in logical pixels from the stack's edges. `None` on an
/// edge means "not pinned there".
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Positioning {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl Positioning {
    /// The width this asks for in a stack `available` wide, and `None` when it asks for
    /// the child's own.
    pub fn resolved_width(&self, available: f32) -> Option<f32> {
        match (self.left, self.right) {
            (Some(l), Some(r)) => Some((available - l - r).max(0.0)),
            _ => self.width,
        }
    }

    /// The height this asks for in a stack `available` tall, and `None` when it asks for
    /// the child's own.
    pub fn resolved_height(&self, available: f32) -> Option<f32> {
        match (self.top, self.bottom) {
            (Some(t), Some(b)) => Some((available - t - b).max(0.0)),
            _ => self.height,
        }
    }

    /// Where the box goes on one axis, given the stack's extent, the child's own and the
    /// share of the free space the stack's alignment asks for.
    ///
    /// The near edge wins if it is pinned; failing that the far edge places the box by
    /// its own size; failing both, the stack aligns it.
    pub fn place(
        near: Option<f32>,
        far: Option<f32>,
        available: f32,
        own: f32,
        fraction: f32,
    ) -> f32 {
        match (near, far) {
            (Some(n), _) => n,
            (None, Some(f)) => available - f - own,
            (None, None) => (available - own).max(0.0) * fraction,
        }
    }
}

/// A layer pinned against a [`crate::Stack`]'s edges. See the module documentation.
pub struct Positioned<Msg> {
    inner: Box<dyn Widget<Msg>>,
    spec: Positioning,
}

impl<Msg> Positioned<Msg> {
    /// Wraps a layer, pinned nowhere yet — which on its own is the stack's alignment at
    /// the child's natural size.
    pub fn new(inner: impl Widget<Msg> + 'static) -> Self {
        Self {
            inner: Box::new(inner),
            spec: Positioning::default(),
        }
    }

    /// Distance from the stack's left edge.
    pub fn left(mut self, px: f32) -> Self {
        self.spec.left = Some(px);
        self
    }

    /// Distance from the stack's top edge.
    pub fn top(mut self, px: f32) -> Self {
        self.spec.top = Some(px);
        self
    }

    /// Distance from the stack's right edge.
    pub fn right(mut self, px: f32) -> Self {
        self.spec.right = Some(px);
        self
    }

    /// Distance from the stack's bottom edge.
    pub fn bottom(mut self, px: f32) -> Self {
        self.spec.bottom = Some(px);
        self
    }

    /// An explicit width, used when only one horizontal edge is pinned.
    pub fn width(mut self, px: f32) -> Self {
        self.spec.width = Some(px);
        self
    }

    /// An explicit height, used when only one vertical edge is pinned.
    pub fn height(mut self, px: f32) -> Self {
        self.spec.height = Some(px);
        self
    }

    /// Pins all four edges: the layer fills the stack inset by `px` on every side.
    pub fn inset(self, px: f32) -> Self {
        self.left(px).top(px).right(px).bottom(px)
    }

    /// It does not touch the box — the stack reads the pins and lays the layer out
    /// against them, which a style cannot express.
    fn restyle(&self, base: Style) -> Style {
        base
    }
}

crate::transparent::forward_transparent!(Positioned {
    /// Claimed: the pins are the whole of what this wrapper adds.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        Some(self.spec)
    }

    /// Forwarded: pinning a widget says nothing about which widget it is.
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }

    /// Forwarded too: a place is not a palette.
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
});
