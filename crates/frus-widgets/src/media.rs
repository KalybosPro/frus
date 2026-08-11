//! [`MediaQuery`] — the **ambient** description of the surface being drawn on: its
//! size, its pixel density, and the parts of it the system has already taken (bars,
//! notch, soft keyboard).
//!
//! Until now this information reached an application only through
//! `Application::on_insets`, which means every app that wanted to avoid a notch had to
//! store the insets in its own state and thread them down by hand to whichever widget
//! needed them. That is a lot of plumbing for a value that is the same everywhere in
//! the tree.
//!
//! `MediaQuery` makes it **ambient** instead: the framework installs it around the
//! call to `view`, and any widget built during that call reads it with
//! [`MediaQuery::of`] — no argument to pass, no field to carry.
//!
//! ```ignore
//! let mq = MediaQuery::of();
//! if mq.orientation() == Orientation::Landscape { … }
//! ```
//!
//! A subtree can be built against a **different** description with
//! [`MediaQuery::scope`] — which is how [`crate::SafeArea`] stops its descendants from
//! padding for the same notch twice.
//!
//! ## Why a scoped ambient, and not a `BuildContext`
//!
//! Widgets here are ordinary Rust values, built eagerly by `view`; there is no build
//! context to look an inherited value up from. The scope is therefore **dynamic** —
//! it covers whatever runs inside the closure — and it is restored on the way out even
//! if that closure panics. Reading it is a thread-local read: no lock, no allocation.
//!
//! Outside a scope (a unit test constructing a widget directly, say) `of` returns
//! [`MediaQuery::UNSET`], a zero-size surface with no insets, rather than panicking —
//! a widget that reads it still builds, it simply has nothing to avoid.

use std::cell::Cell;

use frus_core::{Insets, Orientation, Size, SizeClass, WindowInsets};

/// A set of edges, as four independent flags — which sides of a box an operation
/// applies to. Used by [`crate::SafeArea`] to pick the edges it insets, and by
/// [`MediaQuery::remove_padding`] to pick the ones it consumes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Edges {
    /// The top edge.
    pub top: bool,
    /// The right edge.
    pub right: bool,
    /// The bottom edge.
    pub bottom: bool,
    /// The left edge.
    pub left: bool,
}

impl Edges {
    /// All four edges.
    pub const ALL: Self = Self {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };

    /// No edge at all.
    pub const NONE: Self = Self {
        top: false,
        right: false,
        bottom: false,
        left: false,
    };

    /// Top and bottom only.
    pub const VERTICAL: Self = Self {
        top: true,
        right: false,
        bottom: true,
        left: false,
    };

    /// Left and right only.
    pub const HORIZONTAL: Self = Self {
        top: false,
        right: true,
        bottom: false,
        left: true,
    };

    /// The same set without its top edge.
    pub const fn without_top(mut self) -> Self {
        self.top = false;
        self
    }

    /// The same set without its right edge.
    pub const fn without_right(mut self) -> Self {
        self.right = false;
        self
    }

    /// The same set without its bottom edge.
    pub const fn without_bottom(mut self) -> Self {
        self.bottom = false;
        self
    }

    /// The same set without its left edge.
    pub const fn without_left(mut self) -> Self {
        self.left = false;
        self
    }

    /// Keeps `insets` on the selected edges and zeroes the others.
    pub fn select(self, insets: Insets) -> Insets {
        Insets::new(
            if self.top { insets.top } else { 0.0 },
            if self.right { insets.right } else { 0.0 },
            if self.bottom { insets.bottom } else { 0.0 },
            if self.left { insets.left } else { 0.0 },
        )
    }
}

impl Default for Edges {
    fn default() -> Self {
        Self::ALL
    }
}

/// The surface an interface is being built for.
///
/// Every length is in **logical** pixels, the same unit widget styles use, so a value
/// read here can be handed straight to a padding or a size.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MediaQuery {
    /// The drawable surface, in logical px — the window's client area, or the whole
    /// screen on a phone.
    pub size: Size,
    /// Physical pixels per logical pixel: the system's DPI scale.
    pub device_pixel_ratio: f32,
    /// The application's own zoom factor on top of the system scale (see
    /// `Application::density`). `1.0` is neutral.
    pub density: f32,
    /// Permanently occupied edges: system bars, the notch, the gesture handle.
    pub padding: Insets,
    /// Transiently occupied edges — in practice the soft keyboard, at the bottom.
    /// Measured from the window edge, so it already includes whatever bar it covers.
    pub view_insets: Insets,
}

impl MediaQuery {
    /// The value [`of`](Self::of) returns outside any scope: a zero-size surface, no
    /// scaling, nothing to avoid.
    pub const UNSET: Self = Self {
        size: Size {
            width: 0.0,
            height: 0.0,
        },
        device_pixel_ratio: 1.0,
        density: 1.0,
        padding: Insets::ZERO,
        view_insets: Insets::ZERO,
    };

    /// A description of a bare surface of `size`, with no insets — the desktop case,
    /// and the starting point for building one by hand in a test.
    pub fn new(size: Size) -> Self {
        Self {
            size,
            ..Self::UNSET
        }
    }

    /// Sets the DPI scale.
    pub fn with_device_pixel_ratio(mut self, ratio: f32) -> Self {
        self.device_pixel_ratio = ratio;
        self
    }

    /// Sets the application zoom factor.
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Sets both kinds of inset at once, from what the shell reports.
    pub fn with_insets(mut self, insets: WindowInsets) -> Self {
        self.padding = insets.padding;
        self.view_insets = insets.view_insets;
        self
    }

    /// The area to keep content out of: the per-side **maximum** of the two kinds of
    /// inset. The keyboard covers the navigation bar rather than stacking on top of
    /// it, so they are combined with `max` and not with `+`.
    pub fn safe(&self) -> Insets {
        WindowInsets {
            padding: self.padding,
            view_insets: self.view_insets,
        }
        .safe()
    }

    /// Which way round the surface is.
    pub fn orientation(&self) -> Orientation {
        Orientation::from_size(self.size.width, self.size.height)
    }

    /// The width breakpoint the surface falls into.
    pub fn size_class(&self) -> SizeClass {
        SizeClass::from_width(self.size.width)
    }

    /// The same description with the selected edges' [`padding`](Self::padding)
    /// zeroed — what a widget that has just *consumed* that padding hands to its
    /// descendants, so they do not inset for the same notch a second time.
    pub fn remove_padding(mut self, edges: Edges) -> Self {
        let consumed = edges.select(self.padding);
        self.padding = Insets::new(
            self.padding.top - consumed.top,
            self.padding.right - consumed.right,
            self.padding.bottom - consumed.bottom,
            self.padding.left - consumed.left,
        );
        self
    }

    /// The same description with the selected edges' [`view_insets`](Self::view_insets)
    /// zeroed — the keyboard equivalent of [`remove_padding`](Self::remove_padding).
    pub fn remove_view_insets(mut self, edges: Edges) -> Self {
        let consumed = edges.select(self.view_insets);
        self.view_insets = Insets::new(
            self.view_insets.top - consumed.top,
            self.view_insets.right - consumed.right,
            self.view_insets.bottom - consumed.bottom,
            self.view_insets.left - consumed.left,
        );
        self
    }

    /// The description currently in force, or [`UNSET`](Self::UNSET) outside any
    /// scope.
    pub fn of() -> MediaQuery {
        AMBIENT.with(|a| a.get())
    }

    /// Runs `f` with `self` as the ambient description, and restores whatever was in
    /// force before — including when `f` panics, so one bad frame cannot leave a
    /// stale surface installed for every frame after it.
    ///
    /// This is what the framework wraps `view` in, and what a widget uses to change
    /// the description its own subtree sees.
    pub fn scope<R>(self, f: impl FnOnce() -> R) -> R {
        let previous = AMBIENT.with(|a| a.replace(self));
        let guard = Restore(previous);
        let out = f();
        drop(guard);
        out
    }
}

thread_local! {
    /// The description in force on this thread. `Cell`, not `RefCell`: the value is
    /// `Copy` and every access is a whole get or a whole set, so there is no borrow to
    /// get wrong.
    static AMBIENT: Cell<MediaQuery> = const { Cell::new(MediaQuery::UNSET) };
}

/// Puts back the previous ambient description when dropped — including while a panic
/// unwinds.
struct Restore(MediaQuery);

impl Drop for Restore {
    fn drop(&mut self) {
        AMBIENT.with(|a| a.set(self.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone() -> MediaQuery {
        MediaQuery::new(Size::new(360.0, 780.0))
            .with_device_pixel_ratio(3.0)
            .with_insets(WindowInsets {
                padding: Insets::new(28.0, 0.0, 16.0, 0.0),
                view_insets: Insets::ZERO,
            })
    }

    #[test]
    fn outside_a_scope_the_surface_is_unset_rather_than_a_panic() {
        assert_eq!(MediaQuery::of(), MediaQuery::UNSET);
    }

    #[test]
    fn a_scope_is_visible_inside_and_gone_outside() {
        let seen = phone().scope(MediaQuery::of);
        assert_eq!(seen.size.width, 360.0);
        assert_eq!(seen.padding.top, 28.0);
        assert_eq!(MediaQuery::of(), MediaQuery::UNSET);
    }

    #[test]
    fn scopes_nest_and_unwind_in_order() {
        phone().scope(|| {
            assert_eq!(MediaQuery::of().size.width, 360.0);
            MediaQuery::new(Size::new(1200.0, 800.0)).scope(|| {
                assert_eq!(MediaQuery::of().size.width, 1200.0);
            });
            // The inner scope put the outer one back, not `UNSET`.
            assert_eq!(MediaQuery::of().size.width, 360.0);
        });
    }

    #[test]
    fn a_panicking_scope_still_restores_the_previous_surface() {
        let caught = std::panic::catch_unwind(|| {
            phone().scope(|| panic!("the view exploded"));
        });
        assert!(caught.is_err());
        assert_eq!(
            MediaQuery::of(),
            MediaQuery::UNSET,
            "a panic inside a scope must not leave its surface installed"
        );
    }

    #[test]
    fn removing_padding_leaves_the_other_edges_alone() {
        let mq = phone().remove_padding(Edges::ALL.without_bottom());
        assert_eq!(mq.padding.top, 0.0);
        assert_eq!(mq.padding.bottom, 16.0, "the bottom was not consumed");
    }

    #[test]
    fn the_keyboard_and_the_navigation_bar_do_not_stack() {
        let mq = phone().with_insets(WindowInsets {
            padding: Insets::new(28.0, 0.0, 16.0, 0.0),
            // The keyboard is measured from the window edge, bar included.
            view_insets: Insets::new(0.0, 0.0, 320.0, 0.0),
        });
        assert_eq!(mq.safe().bottom, 320.0, "not 336");
        assert_eq!(mq.safe().top, 28.0);
    }

    #[test]
    fn orientation_and_size_class_come_from_the_surface() {
        assert_eq!(phone().orientation(), Orientation::Portrait);
        assert_eq!(phone().size_class(), SizeClass::Compact);
        let desk = MediaQuery::new(Size::new(1440.0, 900.0));
        assert_eq!(desk.orientation(), Orientation::Landscape);
        assert_eq!(desk.size_class(), SizeClass::Expanded);
    }

    #[test]
    fn a_square_surface_counts_as_portrait() {
        let square = MediaQuery::new(Size::new(500.0, 500.0));
        assert_eq!(square.orientation(), Orientation::Portrait);
    }
}
