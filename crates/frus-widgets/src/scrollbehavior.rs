//! [`ScrollBehavior`] and [`ScrollConfiguration`]: **how the scrollables of a subtree
//! behave** — their physics, their scrollbars, what they show when pulled past an edge,
//! how a fling is read — decided in one place, from one platform (milestone 616).
//!
//! The reference's shape (`scroll_configuration.dart`). A scrollable does not decide any
//! of these on its own: it asks the behaviour in force where it stands, and the behaviour
//! answers from the platform it follows. The base behaviour follows the platform the
//! application runs on; the one an application installs by default follows the **theme's**
//! platform, so a theme set to iOS bounces, hides its scrollbars and reads flings the iOS
//! way on any device. A subtree that wants otherwise installs its own with a
//! [`ScrollConfiguration`], usually the inherited one with a change:
//!
//! ```ignore
//! ScrollConfiguration::new(
//!     ScrollConfiguration::of().with_scrollbars(false),
//!     ListView::new(...),
//! )
//! ```

use std::cell::Cell;
use std::hash::{Hash, Hasher};

use frus_core::{TargetPlatform, VelocityStrategy, VelocityTracker};

use crate::physics::{OverscrollIndicator, ScrollDecelerationRate, ScrollPhysics, Scrollbars};
use crate::theme::Theme;
use crate::widget::Widget;

/// Which of the two behaviours a [`ScrollBehavior`] starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Base {
    /// The base behaviour: the platform the application runs on, a glow on Android.
    Plain,
    /// The application's: the theme's platform, the stretch on Android.
    Material,
}

/// **How the scrollables under it behave**, from the platform it follows.
///
/// Two starting points, as in the reference: [`ScrollBehavior::new`], the base, which
/// follows [`default_target_platform`](frus_core::default_target_platform), and
/// [`ScrollBehavior::material`], the one an application installs, which follows
/// [`Theme::platform`]. Each can be changed with the `with_` methods, the reference's
/// `copyWith`: a behaviour with its scrollbars or its overscroll indicator turned off, with
/// physics of its own, or following another platform.
///
/// The answers, platform by platform, are the reference's:
///
/// | | physics | scrollbars | overscroll | fling |
/// |---|---|---|---|---|
/// | Android | clamping | none | stretch (base: glow) | regression |
/// | Fuchsia | clamping | none | glow | regression |
/// | iOS | bouncing | none | none | recent average |
/// | macOS | bouncing, fast | shown | none | recent average, desktop |
/// | Linux, Windows | clamping | shown | none | regression |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScrollBehavior {
    base: Base,
    scrollbars: bool,
    overscroll: bool,
    physics: Option<ScrollPhysics>,
    platform: Option<TargetPlatform>,
}

impl Default for ScrollBehavior {
    /// The base behaviour, [`ScrollBehavior::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollBehavior {
    /// The **base** behaviour: it follows the platform the application runs on
    /// ([`default_target_platform`](frus_core::default_target_platform)), and shows a glow
    /// past the edge on Android and Fuchsia. What [`ScrollConfiguration::of`] answers where
    /// nothing was installed.
    pub const fn new() -> Self {
        Self {
            base: Base::Plain,
            scrollbars: true,
            overscroll: true,
            physics: None,
            platform: None,
        }
    }

    /// The **application's** behaviour: it follows the theme's platform
    /// ([`Theme::platform`]), and stretches the content past the edge on Android. What an
    /// application installs unless it says otherwise
    /// (`Application::scroll_behavior`).
    pub const fn material() -> Self {
        Self {
            base: Base::Material,
            ..Self::new()
        }
    }

    /// The same behaviour, with its scrollbars on or off. Off, none is drawn on any
    /// platform; on, the platform decides.
    #[must_use]
    pub const fn with_scrollbars(mut self, scrollbars: bool) -> Self {
        self.scrollbars = scrollbars;
        self
    }

    /// The same behaviour, with its overscroll indicator on or off. Off, nothing is shown
    /// past an edge on any platform; on, the platform decides.
    #[must_use]
    pub const fn with_overscroll(mut self, overscroll: bool) -> Self {
        self.overscroll = overscroll;
        self
    }

    /// The same behaviour, with these physics on every platform.
    #[must_use]
    pub const fn with_physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    /// The same behaviour, following `platform` instead of the one it would have followed.
    #[must_use]
    pub const fn with_platform(mut self, platform: TargetPlatform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// The platform this behaviour follows under `theme`: the one it was given, else the
    /// theme's for the application's behaviour, else the one the application runs on.
    pub fn platform(&self, theme: &Theme) -> TargetPlatform {
        self.platform.unwrap_or_else(|| match self.base {
            Base::Material => theme.platform,
            Base::Plain => frus_core::default_target_platform(),
        })
    }

    /// The physics a scrollable under this behaviour starts from: bouncing on iOS, bouncing
    /// with a faster stop on macOS, clamping elsewhere — unless the behaviour was given its
    /// own. A scrollable's own physics take precedence over these.
    pub fn scroll_physics(&self, theme: &Theme) -> ScrollPhysics {
        self.physics.unwrap_or(match self.platform(theme) {
            TargetPlatform::Ios => ScrollPhysics::Bouncing(ScrollDecelerationRate::Normal),
            TargetPlatform::MacOs => ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => ScrollPhysics::Clamping,
        })
    }

    /// Whether a vertical scrollable under this behaviour draws a scrollbar: on Linux,
    /// macOS and Windows, where a pointer needs one to find its way; not on a touch screen,
    /// where a finger already knows where it is on the page.
    pub fn scrollbars(&self, theme: &Theme) -> Scrollbars {
        if !self.scrollbars {
            return Scrollbars::Never;
        }
        match self.platform(theme) {
            TargetPlatform::Linux | TargetPlatform::MacOs | TargetPlatform::Windows => {
                Scrollbars::Always
            }
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Ios => {
                Scrollbars::Never
            }
        }
    }

    /// What a scrollable under this behaviour shows when it is pulled past an edge it will
    /// not move beyond, or `None` for nothing: the stretch on Android under the
    /// application's behaviour, a glow on Android under the base one and on Fuchsia under
    /// both, nothing on iOS, Linux, macOS and Windows.
    pub fn overscroll_indicator(&self, theme: &Theme) -> Option<OverscrollIndicator> {
        if !self.overscroll {
            return None;
        }
        match (self.platform(theme), self.base) {
            (TargetPlatform::Android, Base::Material) => Some(OverscrollIndicator::Stretch),
            (TargetPlatform::Android, Base::Plain) | (TargetPlatform::Fuchsia, _) => {
                Some(OverscrollIndicator::Glow)
            }
            (
                TargetPlatform::Ios
                | TargetPlatform::Linux
                | TargetPlatform::MacOs
                | TargetPlatform::Windows,
                _,
            ) => None,
        }
    }

    /// How a fling under this behaviour is read from the finger's last moves: a weighted
    /// average of the most recent on iOS and macOS (each with its own weights), a fitted
    /// curve elsewhere.
    pub fn velocity_tracker(&self, theme: &Theme) -> VelocityTracker {
        VelocityTracker::new(match self.platform(theme) {
            TargetPlatform::Ios => {
                VelocityStrategy::RecentAverage(frus_core::BOUNCING_FLING_WEIGHTS)
            }
            TargetPlatform::MacOs => {
                VelocityStrategy::RecentAverage(frus_core::DESKTOP_FLING_WEIGHTS)
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => VelocityStrategy::Regression,
        })
    }

    /// Hashed into the relayout fingerprint: a subtree under another behaviour may lay out
    /// another scrollbar.
    pub(crate) fn shape_hash<H: Hasher>(&self, hasher: &mut H) {
        self.hash(hasher);
    }
}

thread_local! {
    /// The behaviour in force where the walk is. The shell installs the application's at
    /// the root of every walk; [`ScrollConfiguration`] replaces it for its subtree.
    static AMBIENT: Cell<ScrollBehavior> = const { Cell::new(ScrollBehavior::new()) };
}

/// Puts a [`ScrollBehavior`] in force for a subtree.
pub struct ScrollConfiguration<Msg = crate::callback::Callback> {
    behavior: ScrollBehavior,
    inner: Box<dyn Widget<Msg>>,
}

impl ScrollConfiguration {
    /// The behaviour in force where this is asked — during a build or a layout — or the
    /// base one, [`ScrollBehavior::new`], where nothing installed one.
    pub fn of() -> ScrollBehavior {
        AMBIENT.with(Cell::get)
    }

    /// Makes `behavior` the one in force until the guard is dropped: what the walks do for
    /// each [`ScrollConfiguration`], and what a test does to lay a tree out under a
    /// behaviour.
    pub fn install(behavior: ScrollBehavior) -> ScrollBehaviorGuard {
        ScrollBehaviorGuard(AMBIENT.with(|a| a.replace(behavior)))
    }

    /// Makes `behavior` the one in force at the **root** of the tree, under every
    /// [`ScrollConfiguration`]: the application's, which the shell sets afresh each frame
    /// from `Application::scroll_behavior`, as an application's root installs its own.
    pub fn set_root(behavior: ScrollBehavior) {
        AMBIENT.with(|a| a.set(behavior));
    }
}

impl<Msg> ScrollConfiguration<Msg> {
    /// `child`, and everything under it, scrolling as `behavior` says.
    pub fn new(behavior: ScrollBehavior, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            behavior,
            inner: Box::new(child),
        }
    }

    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

/// Puts the previous behaviour back when dropped.
pub struct ScrollBehaviorGuard(ScrollBehavior);

impl Drop for ScrollBehaviorGuard {
    fn drop(&mut self) {
        AMBIENT.with(|a| a.set(self.0));
    }
}

crate::transparent::forward_transparent!(ScrollConfiguration {
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(&self, inherited: &crate::theme::Theme) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
    fn scroll_behavior_override(&self) -> Option<ScrollBehavior> {
        Some(self.inner.scroll_behavior_override().unwrap_or(self.behavior))
    }
    fn autofill_group(&self) -> bool {
        self.inner.autofill_group()
    }
    fn area_toolbar(
        &self,
        context: crate::ToolbarContext,
    ) -> Option<Option<&dyn crate::widget::Widget<Msg>>> {
        self.inner.area_toolbar(context)
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::TargetPlatform as P;

    fn under(platform: P) -> Theme {
        Theme::default().with_platform(platform)
    }

    /// **The reference's tables**, for every platform, under the application's behaviour.
    #[test]
    fn the_application_s_behaviour_follows_the_theme_s_platform() {
        let behavior = ScrollBehavior::material();
        for platform in P::ALL {
            let theme = under(platform);
            assert_eq!(behavior.platform(&theme), platform);
            let physics = match platform {
                P::Ios => ScrollPhysics::Bouncing(ScrollDecelerationRate::Normal),
                P::MacOs => ScrollPhysics::Bouncing(ScrollDecelerationRate::Fast),
                _ => ScrollPhysics::Clamping,
            };
            assert_eq!(behavior.scroll_physics(&theme), physics, "{platform}");
            let bars = if platform.is_desktop() {
                Scrollbars::Always
            } else {
                Scrollbars::Never
            };
            assert_eq!(behavior.scrollbars(&theme), bars, "{platform}");
            let past = match platform {
                P::Android => Some(OverscrollIndicator::Stretch),
                P::Fuchsia => Some(OverscrollIndicator::Glow),
                _ => None,
            };
            assert_eq!(behavior.overscroll_indicator(&theme), past, "{platform}");
            let fling = match platform {
                P::Ios => VelocityStrategy::RecentAverage(frus_core::BOUNCING_FLING_WEIGHTS),
                P::MacOs => VelocityStrategy::RecentAverage(frus_core::DESKTOP_FLING_WEIGHTS),
                _ => VelocityStrategy::Regression,
            };
            assert_eq!(
                behavior.velocity_tracker(&theme).strategy(),
                fling,
                "{platform}"
            );
        }
    }

    /// **The base behaviour follows the system, not the theme**, and glows on Android
    /// where the application's stretches.
    #[test]
    fn the_base_behaviour_follows_the_system() {
        let base = ScrollBehavior::new();
        let system = frus_core::default_target_platform();
        for platform in P::ALL {
            assert_eq!(base.platform(&under(platform)), system);
        }
        let android = base.with_platform(P::Android);
        assert_eq!(
            android.overscroll_indicator(&under(P::Ios)),
            Some(OverscrollIndicator::Glow)
        );
        assert_eq!(
            ScrollConfiguration::of(),
            ScrollBehavior::new(),
            "nothing installed"
        );
    }

    /// **A change keeps the rest** — the reference's `copyWith`: scrollbars and overscroll
    /// turned off everywhere, physics and platform pinned.
    #[test]
    fn a_changed_behaviour_keeps_what_it_did_not_change() {
        let desktop = under(P::Windows);
        let android = under(P::Android);
        let quiet = ScrollBehavior::material()
            .with_scrollbars(false)
            .with_overscroll(false);
        assert_eq!(quiet.scrollbars(&desktop), Scrollbars::Never);
        assert_eq!(quiet.overscroll_indicator(&android), None);
        assert_eq!(quiet.scroll_physics(&android), ScrollPhysics::Clamping);
        let bouncy = ScrollBehavior::material().with_physics(ScrollPhysics::BOUNCING);
        assert_eq!(bouncy.scroll_physics(&android), ScrollPhysics::BOUNCING);
        assert_eq!(bouncy.scrollbars(&desktop), Scrollbars::Always);
        let ios = ScrollBehavior::material().with_platform(P::Ios);
        assert_eq!(ios.platform(&android), P::Ios);
        assert_eq!(ios.overscroll_indicator(&android), None);
    }

    /// **Installed for a subtree, and put back**: the guard restores what was in force.
    #[test]
    fn an_installed_behaviour_is_put_back() {
        let outer = ScrollBehavior::material();
        let inner = outer.with_scrollbars(false);
        let _a = ScrollConfiguration::install(outer);
        {
            let _b = ScrollConfiguration::install(inner);
            assert_eq!(ScrollConfiguration::of(), inner);
        }
        assert_eq!(ScrollConfiguration::of(), outer);
    }
}
