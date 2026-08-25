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
//! if mq.orientation() == Orientation::Landscape {
//!     // the landscape arrangement
//! }
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

/// Whether a surface is showing a **light** or a **dark** interface.
///
/// The platform's own preference, which an application may follow or ignore. It is not
/// the same question as which theme this application is using: a light application on a
/// dark system is a choice, not a mistake.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Brightness {
    /// A light interface: dark content on a light surface.
    #[default]
    Light,
    /// A dark interface: light content on a dark surface.
    Dark,
}

/// The **accessibility settings** the platform reports about its user.
///
/// The reference carries these on `MediaQueryData` as separate booleans; they are one
/// struct here because they arrive from one query and are read together, and because a
/// widget that honours one usually honours its neighbours.
///
/// **Honoured by the framework**: [`disable_animations`](Self::disable_animations). The
/// rest are reported for the application to act on — saying so is more useful than a
/// field that looks obeyed and is not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Accessibility {
    /// The user has asked for **heavier** text everywhere.
    pub bold_text: bool,
    /// The user has asked for **higher contrast** than the design's own.
    pub high_contrast: bool,
    /// The user has asked for motion to be reduced or removed.
    ///
    /// The framework honours this for its **implicit** animations — the ones a widget
    /// starts by itself when a value changes — by completing them at once instead of
    /// over time. The change still happens; it stops moving. That is what the setting
    /// asks for, and skipping the change instead would leave the interface wrong rather
    /// than still.
    pub disable_animations: bool,
    /// The user has asked for inverted colours.
    pub invert_colors: bool,
    /// Something is reading the screen aloud, or otherwise driving it without a pointer.
    ///
    /// A hint rather than an instruction: it is how an interface knows that a control
    /// which only appears on hover will never appear.
    pub accessible_navigation: bool,
    /// The user's clock format — `true` for 24-hour, `false` for 12.
    pub always_use_24_hour_format: bool,
}

impl Accessibility {
    /// Nothing asked for: the settings of a user who has changed none of them.
    pub const NONE: Self = Self {
        bold_text: false,
        high_contrast: false,
        disable_animations: false,
        invert_colors: false,
        accessible_navigation: false,
        always_use_24_hour_format: false,
    };

    /// The platform's answer with an application's [`AccessibilityOverrides`] laid over it.
    #[must_use]
    pub fn with_overrides(self, over: AccessibilityOverrides) -> Self {
        Self {
            bold_text: over.bold_text.unwrap_or(self.bold_text),
            high_contrast: over.high_contrast.unwrap_or(self.high_contrast),
            disable_animations: over.disable_animations.unwrap_or(self.disable_animations),
            invert_colors: over.invert_colors.unwrap_or(self.invert_colors),
            accessible_navigation: over
                .accessible_navigation
                .unwrap_or(self.accessible_navigation),
            always_use_24_hour_format: over
                .always_use_24_hour_format
                .unwrap_or(self.always_use_24_hour_format),
        }
    }
}

/// What an application wants to say about [`Accessibility`] **instead of the platform**,
/// one setting at a time.
///
/// Every field is an `Option`, and `None` is an answer: *this application has no opinion,
/// use what the user's system reports*. That distinction is the whole point of the type.
/// A plain `Accessibility` cannot make it — a `false` there is indistinguishable from
/// silence, so an application that only wanted to force *reduce motion* would also be
/// telling the framework that its user does not use a screen reader.
///
/// It is the same shape, and the same reason, as
/// [`TextStyle`](frus_core::TextStyle) against
/// [`ResolvedTextStyle`](frus_core::ResolvedTextStyle): one type asks, the other answers.
///
/// ```ignore
/// fn accessibility(&self) -> AccessibilityOverrides {
///     // A settings screen with a "reduce motion" switch of its own. Everything else
///     // still comes from the platform.
///     AccessibilityOverrides::NONE.disable_animations(self.reduce_motion)
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccessibilityOverrides {
    /// Override [`Accessibility::bold_text`].
    pub bold_text: Option<bool>,
    /// Override [`Accessibility::high_contrast`].
    pub high_contrast: Option<bool>,
    /// Override [`Accessibility::disable_animations`].
    pub disable_animations: Option<bool>,
    /// Override [`Accessibility::invert_colors`].
    pub invert_colors: Option<bool>,
    /// Override [`Accessibility::accessible_navigation`].
    pub accessible_navigation: Option<bool>,
    /// Override [`Accessibility::always_use_24_hour_format`].
    pub always_use_24_hour_format: Option<bool>,
}

impl AccessibilityOverrides {
    /// **Nothing said**: every setting left to the platform.
    pub const NONE: Self = Self {
        bold_text: None,
        high_contrast: None,
        disable_animations: None,
        invert_colors: None,
        accessible_navigation: None,
        always_use_24_hour_format: None,
    };

    /// Speak for [`Accessibility::bold_text`].
    #[must_use]
    pub const fn bold_text(mut self, on: bool) -> Self {
        self.bold_text = Some(on);
        self
    }

    /// Speak for [`Accessibility::high_contrast`].
    #[must_use]
    pub const fn high_contrast(mut self, on: bool) -> Self {
        self.high_contrast = Some(on);
        self
    }

    /// Speak for [`Accessibility::disable_animations`].
    #[must_use]
    pub const fn disable_animations(mut self, on: bool) -> Self {
        self.disable_animations = Some(on);
        self
    }

    /// Speak for [`Accessibility::invert_colors`].
    #[must_use]
    pub const fn invert_colors(mut self, on: bool) -> Self {
        self.invert_colors = Some(on);
        self
    }

    /// Speak for [`Accessibility::accessible_navigation`].
    #[must_use]
    pub const fn accessible_navigation(mut self, on: bool) -> Self {
        self.accessible_navigation = Some(on);
        self
    }

    /// Speak for [`Accessibility::always_use_24_hour_format`].
    #[must_use]
    pub const fn always_use_24_hour_format(mut self, on: bool) -> Self {
        self.always_use_24_hour_format = Some(on);
        self
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
    /// Occupied edges **still worth avoiding**: the intrusions, less whatever
    /// [`view_insets`](Self::view_insets) already covers.
    ///
    /// Zero at the bottom while the keyboard is up, because the navigation bar it hides
    /// is not an edge anything needs to stay clear of any more. This is what a
    /// [`crate::SafeArea`] pads by, and it is why a screen does not keep a strip of
    /// nothing between its content and the keys.
    pub padding: Insets,
    /// Transiently occupied edges — in practice the soft keyboard, at the bottom.
    /// Measured from the window edge, so it already includes whatever bar it covers.
    pub view_insets: Insets,
    /// How much larger the **user** has asked text to be — the system's font-size
    /// setting, not the application's zoom (that is [`density`](Self::density)).
    ///
    /// `1.0` is the platform's normal. It is the accessibility setting people actually
    /// change: a phone's *Font size* slider goes to 1.3 on Android and past 3 with
    /// *Larger Accessibility Sizes* on iOS, and a layout that ignores it is one a great
    /// many people cannot read.
    ///
    /// **The framework scales text by it** since milestone 403: installing this surface
    /// with [`MediaQuery::scope`](Self::scope) puts the factor where
    /// [`TextStyle::resolved`](frus_core::TextStyle::resolved) reads it, and every size in
    /// the framework passes through there. [`scaled`](Self::scaled) applies it to a number
    /// of an application's own.
    ///
    /// Chrome that cannot grow caps it instead of obeying it — see
    /// [`TextStyle::clamp_scale`](frus_core::TextStyle::clamp_scale), which is what an
    /// app bar does with its title.
    pub text_scaler: f32,
    /// Whether the platform is currently showing a **dark** interface, independently of
    /// what this application chose.
    ///
    /// The reference's `platformBrightness`. An application that follows the system
    /// reads it; one that has its own switch ignores it.
    pub platform_brightness: Brightness,
    /// The accessibility settings the platform reports, which the framework and the
    /// application both have a say in honouring.
    pub accessibility: Accessibility,
    /// The edges a **system gesture** starts from — a back swipe, a home swipe — which
    /// a widget with a horizontal drag of its own has to keep clear of, or the two fight
    /// over the same finger.
    ///
    /// The reference's `systemGestureInsets`. Usually wider than
    /// [`padding`](Self::padding) at the sides and taller at the bottom.
    pub system_gesture_insets: Insets,
    /// The intrusions **ignoring** anything transient: what the notch and the bars take
    /// whether or not the keyboard is over them.
    ///
    /// The one that does not move when the keyboard opens. A layout with a flexible
    /// child would otherwise shift the moment the padding under it went to zero, which
    /// is a whole screen twitching because somebody tapped a field — see
    /// [`crate::SafeArea::maintain_bottom_view_padding`].
    pub view_padding: Insets,
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
        view_padding: Insets::ZERO,
        text_scaler: 1.0,
        platform_brightness: Brightness::Light,
        accessibility: Accessibility::NONE,
        system_gesture_insets: Insets::ZERO,
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

    /// Sets the user's text-size setting. Clamped at zero, since a scale that shrank
    /// text to nothing would be a setting nobody could undo.
    pub fn with_text_scaler(mut self, scaler: f32) -> Self {
        self.text_scaler = scaler.max(0.0);
        self
    }

    /// Sets the platform's own light/dark preference.
    pub fn with_platform_brightness(mut self, brightness: Brightness) -> Self {
        self.platform_brightness = brightness;
        self
    }

    /// Sets the accessibility settings the platform reports.
    pub fn with_accessibility(mut self, accessibility: Accessibility) -> Self {
        self.accessibility = accessibility;
        self
    }

    /// Sets the edges a system gesture starts from.
    pub fn with_system_gesture_insets(mut self, insets: Insets) -> Self {
        self.system_gesture_insets = insets;
        self
    }

    /// A font size with the user's text-size setting applied.
    ///
    /// The reference calls this `TextScaler.scale`, and it is a **function** there
    /// rather than a multiplication because a platform may scale non-linearly — large
    /// sizes growing less than small ones, so a heading does not run off the screen when
    /// body text is made readable. Ours is linear; the shape of the call is the one that
    /// can become non-linear without every caller changing.
    pub fn scaled(&self, size: f32) -> f32 {
        size * self.text_scaler
    }

    /// Sets both kinds of inset at once, from what the shell reports.
    pub fn with_insets(mut self, insets: WindowInsets) -> Self {
        self.padding = insets.padding;
        self.view_insets = insets.view_insets;
        self.view_padding = insets.view_padding;
        self
    }

    /// The area to keep content out of: the per-side **maximum** of the two kinds of
    /// inset. The keyboard covers the navigation bar rather than stacking on top of
    /// it, so they are combined with `max` and not with `+`.
    pub fn safe(&self) -> Insets {
        WindowInsets {
            padding: self.padding,
            view_insets: self.view_insets,
            view_padding: self.view_padding,
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
        // The **view** padding loses the same amount, floored at zero. Without this a
        // descendant asking for the intrusion that does not move would be told about
        // one its parent has already dealt with, and would inset for the notch twice
        // on the one path that was meant to be immune to the keyboard.
        self.view_padding = Insets::new(
            (self.view_padding.top - consumed.top).max(0.0),
            (self.view_padding.right - consumed.right).max(0.0),
            (self.view_padding.bottom - consumed.bottom).max(0.0),
            (self.view_padding.left - consumed.left).max(0.0),
        );
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
        // Same reasoning as [`remove_padding`](Self::remove_padding): a keyboard a
        // parent has already made room for is not one its children should hear about,
        // through either inset.
        self.view_padding = Insets::new(
            (self.view_padding.top - consumed.top).max(0.0),
            (self.view_padding.right - consumed.right).max(0.0),
            (self.view_padding.bottom - consumed.bottom).max(0.0),
            (self.view_padding.left - consumed.left).max(0.0),
        );
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

    /// Is a surface actually **described**? False outside every
    /// [`scope`](Self::scope), where [`of`](Self::of) answers
    /// [`UNSET`](Self::UNSET) — a surface of no size.
    ///
    /// Any widget that sizes itself from the ambient description needs this, because
    /// a width of zero is not a narrow screen: it is the absence of one, and the two
    /// want opposite answers. A bar folds everything into its menu at zero and nothing
    /// at all when there is no screen to fold against.
    pub fn is_described(&self) -> bool {
        self.size.width > 0.0 && self.size.height > 0.0
    }

    /// Runs `f` with `self` as the ambient description, and restores whatever was in
    /// force before — including when `f` panics, so one bad frame cannot leave a
    /// stale surface installed for every frame after it.
    ///
    /// **For a subtree**, where a closure is the right shape: a widget changing the
    /// description its children see. A *frame* wants [`install`](Self::install) instead,
    /// because a frame does not fit in a closure — see there for why that matters.
    pub fn scope<R>(self, f: impl FnOnce() -> R) -> R {
        let guard = self.install();
        let out = f();
        drop(guard);
        out
    }

    /// Installs `self` as the ambient description **until the returned guard is dropped**,
    /// and restores whatever was in force before — panic or not.
    ///
    /// # Why this exists as well as [`scope`](Self::scope)
    ///
    /// A closure bounds the description to one call, and a *frame* is not one call. The
    /// widgets are built, then measured and laid out, then painted, and **all three**
    /// resolve sizes. The shell wrapped only the build for four milestones: the reader's
    /// font size reached a real device and moved not one pixel, because the two steps that
    /// decide how big text actually is ran outside the closure (milestone 407).
    ///
    /// The description and the reader's font size are installed **together, by this one
    /// call**, and that is the point. They were briefly two guards that had to agree with
    /// each other, which is the same bug with an extra step: whoever holds one and forgets
    /// the other gets a layout measured at one size and painted at another, with nothing to
    /// say so.
    #[must_use = "the surface is uninstalled the moment the guard is dropped"]
    pub fn install(self) -> SurfaceGuard {
        let previous = AMBIENT.with(|a| a.replace(self));
        // The reader's font size travels with the description, because it **is** part of
        // the description and because installing it anywhere else would be one more thing
        // to remember. It lives in `frus-core` rather than here: the only place a size
        // becomes a number is `TextStyle::resolved`, and that is below this crate.
        SurfaceGuard {
            _scale: frus_core::install_text_scale(self.text_scaler),
            _surface: Restore(previous),
        }
    }
}

/// Holds a surface installed — see [`MediaQuery::install`]. Dropping it puts back the
/// description **and** the reader's font size that were in force before.
///
/// The two live in one guard so that they cannot be held for different lengths of time,
/// which is exactly how they came apart in milestone 407.
#[must_use = "the surface is uninstalled the moment the guard is dropped"]
pub struct SurfaceGuard {
    /// Held for its `Drop` and never read, hence the name. Declared **first**, so it is
    /// dropped first: the reverse of the order the two were installed in.
    _scale: frus_core::TextScaleGuard,
    /// Held for its `Drop` and never read. See `_scale` for the ordering.
    _surface: Restore,
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
            .with_insets(WindowInsets::bars(Insets::new(28.0, 0.0, 16.0, 0.0)))
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
        // Derived the way the shell derives it: a 16 px bar, then 320 px of occlusion
        // measured from the window edge. Hand-writing the three insets would let a test
        // describe a surface no platform can report.
        let mq = phone().with_insets(WindowInsets::from_baseline(
            Insets::new(28.0, 0.0, 16.0, 0.0),
            Insets::new(28.0, 0.0, 320.0, 0.0),
        ));
        assert_eq!(mq.safe().bottom, 320.0, "not 336");
        assert_eq!(mq.safe().top, 28.0);
    }

    /// The three insets, and what each is for.
    ///
    /// `view_padding` is the bar whether or not anything covers it; `view_insets` is the
    /// occlusion from the window edge; `padding` is what is **left** to avoid, which is
    /// nothing at the bottom once the keyboard is over the bar. Padding a screen by the
    /// bar as well would leave a strip of nothing above the keys.
    #[test]
    fn the_padding_is_what_the_keyboard_has_not_already_covered() {
        let bar = Insets::new(28.0, 0.0, 16.0, 0.0);
        let shut = phone().with_insets(WindowInsets::bars(bar));
        assert_eq!(shut.padding.bottom, 16.0);
        assert_eq!(shut.view_padding.bottom, 16.0);
        assert_eq!(shut.view_insets.bottom, 0.0);

        let open = phone().with_insets(WindowInsets::from_baseline(
            bar,
            Insets::new(28.0, 0.0, 320.0, 0.0),
        ));
        assert_eq!(
            open.padding.bottom, 0.0,
            "the bar is covered, so nothing avoids it"
        );
        assert_eq!(open.view_padding.bottom, 16.0, "the bar has not moved");
        assert_eq!(open.view_insets.bottom, 320.0);
        // The top is untouched by any of it: a notch is a notch.
        assert_eq!(open.padding.top, 28.0);
        assert_eq!(open.view_padding.top, 28.0);
    }

    /// Consuming the padding consumes as much of the **view** padding, or a widget
    /// inside a safe area would inset for the notch a second time on the one path that
    /// was meant to be immune to the keyboard.
    #[test]
    fn consuming_the_padding_consumes_the_view_padding_with_it() {
        let mq = phone()
            .with_insets(WindowInsets::bars(Insets::new(28.0, 0.0, 16.0, 0.0)))
            .remove_padding(Edges::ALL);
        assert_eq!(mq.padding, Insets::ZERO);
        assert_eq!(mq.view_padding, Insets::ZERO);
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

    /// **The platform answers, and the application overrides only what it spoke for.**
    ///
    /// The ordering is the point. These settings belong to the person using the device,
    /// so an application that says nothing must not be able to overrule them by accident —
    /// and before this the application was the only source, so silence and "off" were the
    /// same sentence.
    #[test]
    fn an_application_that_says_nothing_leaves_the_user_alone() {
        let user = Accessibility {
            disable_animations: true,
            accessible_navigation: true,
            ..Accessibility::NONE
        };
        assert_eq!(
            user.with_overrides(AccessibilityOverrides::NONE),
            user,
            "an application with no settings screen changes nothing"
        );
    }

    /// And an application that speaks for **one** setting speaks for one setting.
    ///
    /// This is what a plain `Accessibility` could not express: forcing *reduce motion* off
    /// used to mean also declaring that the user runs no screen reader, because a `false`
    /// and a silence were the same value.
    #[test]
    fn speaking_for_one_setting_does_not_speak_for_the_others() {
        let user = Accessibility {
            disable_animations: true,
            accessible_navigation: true,
            always_use_24_hour_format: true,
            ..Accessibility::NONE
        };
        let resolved = user.with_overrides(AccessibilityOverrides::NONE.disable_animations(false));
        assert!(!resolved.disable_animations, "the application had its say");
        assert!(
            resolved.accessible_navigation,
            "and said nothing about the screen reader"
        );
        assert!(resolved.always_use_24_hour_format, "nor about the clock");
    }

    /// A surface installed with [`MediaQuery::scope`] puts the reader's font size where
    /// [`frus_core::TextStyle::resolved`] reads it — the whole point of carrying the
    /// number. Without this the platform could report it and nothing would change.
    #[test]
    fn a_described_surface_hands_the_readers_font_size_to_the_text() {
        let phone = MediaQuery::new(Size::new(400.0, 800.0)).with_text_scaler(1.5);
        let plain = frus_core::TextStyle::new(16.0);
        assert_eq!(
            plain.resolved().size,
            16.0,
            "outside a surface, nothing moves"
        );
        phone.scope(|| {
            assert_eq!(plain.resolved().size, 24.0);
        });
        assert_eq!(plain.resolved().size, 16.0, "and it is put back afterwards");
    }

    /// **The description and the reader's font size are installed by one call.**
    ///
    /// They were briefly two guards that had to agree with each other, which is the same
    /// bug with an extra step: whoever holds one and forgets the other gets a layout
    /// measured at one size and painted at another.
    #[test]
    fn one_guard_installs_the_whole_surface() {
        let phone = MediaQuery::new(Size::new(360.0, 780.0)).with_text_scaler(1.5);
        assert_eq!(frus_core::text_scale(), 1.0);
        assert!(!MediaQuery::of().is_described());
        {
            let _held = phone.install();
            assert_eq!(frus_core::text_scale(), 1.5, "the font size is in force");
            assert!(MediaQuery::of().is_described(), "and so is the description");
        }
        assert_eq!(frus_core::text_scale(), 1.0, "both are put back");
        assert!(!MediaQuery::of().is_described());
    }

    /// A guard **outlives a closure**, which is the whole reason it exists: a frame builds,
    /// then lays out, then paints, and all three resolve sizes.
    #[test]
    fn a_guard_holds_past_the_call_that_made_it() {
        let built = {
            let held = MediaQuery::new(Size::new(400.0, 300.0))
                .with_text_scaler(2.0)
                .install();
            let size = frus_core::TextStyle::new(16.0).resolved().size;
            // The "layout" and the "paint" of this frame: after the widgets were built,
            // and still inside the surface.
            let laid_out = frus_core::TextStyle::new(16.0).resolved().size;
            drop(held);
            (size, laid_out)
        };
        assert_eq!(built, (32.0, 32.0));
        assert_eq!(frus_core::TextStyle::new(16.0).resolved().size, 16.0);
    }

    /// Installing one **inside** another nests and unwinds in order, so a widget can still
    /// change the description its own subtree sees.
    #[test]
    fn surfaces_nest_and_unwind_in_order() {
        let outer = MediaQuery::new(Size::new(400.0, 800.0)).with_text_scaler(1.5);
        let inner = MediaQuery::new(Size::new(200.0, 400.0)).with_text_scaler(3.0);
        let _o = outer.install();
        assert_eq!(frus_core::text_scale(), 1.5);
        {
            let _i = inner.install();
            assert_eq!(frus_core::text_scale(), 3.0);
            assert_eq!(MediaQuery::of().size.width, 200.0);
        }
        assert_eq!(frus_core::text_scale(), 1.5, "the outer surface is back");
        assert_eq!(MediaQuery::of().size.width, 400.0);
    }
}
