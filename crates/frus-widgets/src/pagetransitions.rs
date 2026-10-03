//! **How a page arrives and leaves**, platform by platform (milestone 620).
//!
//! The reference's shape (`page_transitions_theme.dart`): a [`PageTransitionsTheme`] on the
//! theme maps each platform to a [`PageTransitionsBuilder`], and the builder decides how the
//! page being pushed or popped, and the page under it, look at each moment of the
//! transition. By default the desktops zoom, Apple's platforms slide, and Android fades
//! forwards — the reference's own defaults, read from the **theme's** platform so a theme
//! set to iOS slides on any device.
//!
//! Everything here is a pure function of the transition's progress: the navigator asks
//! [`PageTransitionsBuilder::frame`] what to paint, and the router runs the progress over
//! [`PageTransitionsBuilder::transition_duration`].

use frus_core::{Curve, TargetPlatform};

/// One of the reference's page transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageTransitionsBuilder {
    /// The page zooms in from 85 % over a scrim, the one under it grows a little; popping,
    /// the page shrinks away and fades (`ZoomPageTransitionsBuilder`). What the desktops
    /// use by default, and what a platform with no builder falls back to.
    Zoom,
    /// The page slides in from the end edge while the one under it drifts a third of the
    /// way towards the start, as iOS pages do (`CupertinoPageTransitionsBuilder`).
    Cupertino,
    /// The page slides a quarter of the way in as it fades in, and the one under it a
    /// quarter of the way out as it fades out, as Android's own pages do
    /// (`FadeForwardsPageTransitionsBuilder`). What Android uses when no back gesture is
    /// under way.
    FadeForwards,
}

/// How the theme's platform chooses a [`PageTransitionsBuilder`]: the reference's
/// `PageTransitionsTheme`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageTransitionsTheme {
    /// One per platform, in [`TargetPlatform::ALL`]'s order; `None` falls back to
    /// [`PageTransitionsBuilder::Zoom`].
    builders: [Option<PageTransitionsBuilder>; 6],
}

impl Default for PageTransitionsTheme {
    /// The reference's defaults (`page_transitions_theme.dart:764`): Android fades
    /// forwards, iOS and macOS slide, Linux and Windows zoom, and Fuchsia, which has none,
    /// zooms.
    fn default() -> Self {
        use PageTransitionsBuilder as B;
        let mut builders = [None; 6];
        for (platform, builder) in [
            (TargetPlatform::Android, B::FadeForwards),
            (TargetPlatform::Ios, B::Cupertino),
            (TargetPlatform::MacOs, B::Cupertino),
            (TargetPlatform::Windows, B::Zoom),
            (TargetPlatform::Linux, B::Zoom),
        ] {
            builders[index(platform)] = Some(builder);
        }
        Self { builders }
    }
}

fn index(platform: TargetPlatform) -> usize {
    TargetPlatform::ALL
        .iter()
        .position(|p| *p == platform)
        .unwrap_or(0)
}

impl PageTransitionsTheme {
    /// The same theme, with `builder` for `platform`.
    #[must_use]
    pub fn with(mut self, platform: TargetPlatform, builder: PageTransitionsBuilder) -> Self {
        self.builders[index(platform)] = Some(builder);
        self
    }

    /// The builder for `platform`: its own, else the zoom, as the reference falls back
    /// (`page_transitions_theme.dart:881`) — except iOS, which slides whatever.
    pub fn builder_for(&self, platform: TargetPlatform) -> PageTransitionsBuilder {
        self.builders[index(platform)].unwrap_or(match platform {
            TargetPlatform::Ios => PageTransitionsBuilder::Cupertino,
            _ => PageTransitionsBuilder::Zoom,
        })
    }
}

/// How one page looks at a moment of a transition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PagePose {
    /// How far it is moved along the reading direction, in fractions of the page's
    /// width: positive towards the end edge.
    pub dx: f32,
    /// Its scale about its centre.
    pub scale: f32,
    /// Its opacity.
    pub opacity: f32,
}

impl PagePose {
    /// Where a page rests.
    pub const REST: PagePose = PagePose {
        dx: 0.0,
        scale: 1.0,
        opacity: 1.0,
    };
}

/// What a transition paints at one moment: the two pages, and what lies between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionFrame {
    /// The page pushed, or being popped.
    pub top: PagePose,
    /// The page under it.
    pub below: PagePose,
    /// A fill of the scheme's surface **behind both pages**, at this opacity: the fade's
    /// background, what shows where the pages have faded (`page_transitions_theme.dart:541`).
    /// `0` for none.
    pub backdrop: f32,
    /// A fill of the scheme's surface **between the two pages**, at this opacity: the zoom's
    /// scrim, which hides the page under the one arriving (`:1014`). `0` for none.
    pub scrim: f32,
    /// A dimming over the page under the top one, black at this opacity: the iOS route's
    /// barrier. `0` for none.
    pub barrier: f32,
    /// The iOS page's shadow along its start edge, `0..=1` of its strength. `0` for none.
    pub edge_shadow: f32,
    /// Whether `dx` turns round under a right-to-left layout: the iOS slide does, the fade
    /// does not (the reference's `SlideTransition` there is given no direction).
    pub mirrored: bool,
}

/// The iOS route's barrier colour, black at `0x18` (`route.dart:48`).
pub const CUPERTINO_BARRIER_OPACITY: f32 = 0x18 as f32 / 255.0;

impl PageTransitionsBuilder {
    /// How long a push or a pop takes, in seconds: 300 ms for the zoom, 500 for the iOS
    /// slide, 450 for the fade (`page_transitions_builder.dart:66`, `route.dart:155`,
    /// `page_transitions_theme.dart:470`).
    pub fn transition_duration(self) -> f32 {
        match self {
            PageTransitionsBuilder::Zoom => 0.3,
            PageTransitionsBuilder::Cupertino => 0.5,
            PageTransitionsBuilder::FadeForwards => 0.45,
        }
    }

    /// What is painted `progress` of the way through a transition, `0` being the old page
    /// at rest and `1` the new one. `forward` is a push; otherwise a pop, where the top
    /// page is the one leaving. `gesture` is a finger dragging a pop: the motion follows it
    /// linearly, as the reference's does while a pop gesture is under way.
    pub fn frame(self, progress: f32, forward: bool, gesture: bool) -> TransitionFrame {
        let p = progress.clamp(0.0, 1.0);
        match self {
            PageTransitionsBuilder::Zoom => zoom(p, forward),
            PageTransitionsBuilder::Cupertino => cupertino(p, forward, gesture),
            PageTransitionsBuilder::FadeForwards => fade_forwards(p, forward),
        }
    }
}

/// `inner` over `[begin, end]` of `t`, linear inside.
fn interval(begin: f32, end: f32, t: f32) -> f32 {
    Curve::Interval {
        begin,
        end,
        inner: Box::new(Curve::Linear),
    }
    .transform(t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The zoom (`page_transitions_theme.dart:116-133`, `:175-298`). Its scales move on the
/// emphasized curve; its fades and scrim on linear intervals.
fn zoom(p: f32, forward: bool) -> TransitionFrame {
    let scale =
        |from: f32, to: f32| lerp(from, to, Curve::ease_in_out_cubic_emphasized().transform(p));
    if forward {
        TransitionFrame {
            top: PagePose {
                dx: 0.0,
                scale: scale(0.85, 1.0),
                opacity: interval(0.125, 0.25, p),
            },
            below: PagePose {
                scale: scale(1.0, 1.05),
                ..PagePose::REST
            },
            // Drawn behind the page arriving, until it has arrived (`:1014`).
            backdrop: 0.0,
            scrim: if p < 1.0 {
                0.6 * interval(0.2075, 0.4175, p)
            } else {
                0.0
            },
            barrier: 0.0,
            edge_shadow: 0.0,
            mirrored: false,
        }
    } else {
        TransitionFrame {
            top: PagePose {
                dx: 0.0,
                scale: scale(1.0, 0.9),
                opacity: 1.0 - interval(0.0825, 0.2075, p),
            },
            below: PagePose {
                scale: scale(1.1, 1.0),
                ..PagePose::REST
            },
            backdrop: 0.0,
            scrim: 0.0,
            barrier: 0.0,
            edge_shadow: 0.0,
            mirrored: false,
        }
    }
}

/// The iOS slide (`route.dart:531-572`): the top page from the end edge on
/// `fastEaseInToSlowEaseOut` (its mirror on the way back), the one under it a third of the
/// way towards the start on `linearToEaseOut` (`easeInToLinear` on the way back); linear
/// under a finger.
fn cupertino(p: f32, forward: bool, gesture: bool) -> TransitionFrame {
    // The route's own animation, `a`: 0 when the top page is off screen, 1 at rest.
    let a = if forward { p } else { 1.0 - p };
    let (top, below) = if gesture {
        (a, a)
    } else if forward {
        (
            Curve::fast_ease_in_to_slow_ease_out().transform(a),
            Curve::linear_to_ease_out().transform(a),
        )
    } else {
        (
            Curve::Flipped(Box::new(Curve::fast_ease_in_to_slow_ease_out())).transform(a),
            Curve::ease_in_to_linear().transform(a),
        )
    };
    TransitionFrame {
        top: PagePose {
            dx: 1.0 - top,
            ..PagePose::REST
        },
        below: PagePose {
            dx: -below / 3.0,
            ..PagePose::REST
        },
        backdrop: 0.0,
        scrim: 0.0,
        // The barrier eases in with the route (`Curves.ease`, the barrier's default).
        barrier: CUPERTINO_BARRIER_OPACITY * Curve::ease().transform(a),
        edge_shadow: Curve::linear_to_ease_out().transform(a),
        mirrored: true,
    }
}

/// The fade forwards (`page_transitions_theme.dart:396-552`): a quarter of a page's width
/// on the emphasized curve, with the fades on linear intervals, and the surface behind the
/// page leaving.
fn fade_forwards(p: f32, forward: bool) -> TransitionFrame {
    let slide = Curve::ease_in_out_cubic_emphasized().transform(p);
    let (top, below) = if forward {
        (
            PagePose {
                dx: 0.25 * (1.0 - slide),
                scale: 1.0,
                opacity: interval(0.0, 0.75, p),
            },
            PagePose {
                dx: -0.25 * slide,
                scale: 1.0,
                opacity: 1.0 - interval(0.0, 0.25, p),
            },
        )
    } else {
        (
            PagePose {
                dx: 0.25 * slide,
                scale: 1.0,
                opacity: 1.0 - interval(0.0, 0.25, p),
            },
            PagePose {
                dx: -0.25 * (1.0 - slide),
                scale: 1.0,
                opacity: interval(0.0, 0.75, p),
            },
        )
    };
    TransitionFrame {
        top,
        below,
        backdrop: if p < 1.0 { 1.0 } else { 0.0 },
        scrim: 0.0,
        barrier: 0.0,
        edge_shadow: 0.0,
        mirrored: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PageTransitionsBuilder as B;

    /// **The reference's defaults**: Android fades forwards, Apple's platforms slide, the
    /// desktops and Fuchsia zoom; a theme can say otherwise for any one of them.
    #[test]
    fn each_platform_has_the_reference_s_transition() {
        let theme = PageTransitionsTheme::default();
        for platform in TargetPlatform::ALL {
            let want = match platform {
                TargetPlatform::Android => B::FadeForwards,
                TargetPlatform::Ios | TargetPlatform::MacOs => B::Cupertino,
                _ => B::Zoom,
            };
            assert_eq!(theme.builder_for(platform), want, "{platform}");
        }
        let custom = theme.with(TargetPlatform::Linux, B::Cupertino);
        assert_eq!(custom.builder_for(TargetPlatform::Linux), B::Cupertino);
        assert_eq!(custom.builder_for(TargetPlatform::Windows), B::Zoom);
        assert_eq!(B::Zoom.transition_duration(), 0.3);
        assert_eq!(B::Cupertino.transition_duration(), 0.5);
        assert_eq!(B::FadeForwards.transition_duration(), 0.45);
    }

    /// **Every transition starts with the old page at rest and ends with the new one at
    /// rest**, push or pop, with nothing left between them.
    #[test]
    fn every_transition_begins_and_ends_at_rest() {
        for builder in [B::Zoom, B::Cupertino, B::FadeForwards] {
            for forward in [true, false] {
                let start = builder.frame(0.0, forward, false);
                let end = builder.frame(1.0, forward, false);
                // At 0 the page that was showing shows; at 1 the page that will.
                let (was, will) = if forward {
                    (start.below, end.top)
                } else {
                    (start.top, end.below)
                };
                for (pose, what) in [(was, "start"), (will, "end")] {
                    assert!(
                        pose.dx.abs() < 1e-4
                            && (pose.scale - 1.0).abs() < 1e-4
                            && (pose.opacity - 1.0).abs() < 1e-4,
                        "{builder:?} forward={forward} {what}: {pose:?}"
                    );
                }
                assert_eq!(
                    (end.backdrop, end.scrim),
                    (0.0, 0.0),
                    "{builder:?} leaves nothing behind"
                );
            }
        }
    }

    /// **The zoom, push**: the page grows from 85 % and fades in over 12.5–25 % of the
    /// way, over a scrim that rises to 60 % between 20.75 and 41.75 %; the page under it
    /// grows to 105 %.
    #[test]
    fn the_zoom_pushes_from_85_percent_over_a_scrim() {
        let start = B::Zoom.frame(0.0, true, false);
        assert_eq!((start.top.scale, start.top.opacity), (0.85, 0.0));
        let early = B::Zoom.frame(0.1, true, false);
        assert_eq!(early.top.opacity, 0.0, "not yet faded in");
        let quarter = B::Zoom.frame(0.25, true, false);
        assert_eq!(quarter.top.opacity, 1.0, "faded in by a quarter");
        let mid = B::Zoom.frame(0.5, true, false);
        assert!((mid.scrim - 0.6).abs() < 1e-4, "the scrim at 60 %");
        let near = B::Zoom.frame(0.999, true, false);
        assert!((near.below.scale - 1.05).abs() < 1e-2);
    }

    /// **The zoom, pop**: the page shrinks to 90 % and is gone by 20.75 %; the one under it
    /// settles from 110 %.
    #[test]
    fn the_zoom_pops_by_shrinking_away() {
        let gone = B::Zoom.frame(0.21, false, false);
        assert_eq!(gone.top.opacity, 0.0);
        let start = B::Zoom.frame(0.0, false, false);
        assert!((start.below.scale - 1.1).abs() < 1e-4);
        assert_eq!(start.scrim, 0.0, "no scrim on the way back");
    }

    /// **The iOS slide**: in from the end edge, the page under it a third of the way out,
    /// its barrier at 0x18 and its shadow at full strength at rest; under a finger, linear.
    #[test]
    fn the_cupertino_slide_parallaxes_a_third() {
        let start = B::Cupertino.frame(0.0, true, false);
        assert_eq!(start.top.dx, 1.0);
        let end = B::Cupertino.frame(1.0, true, false);
        assert!((end.below.dx + 1.0 / 3.0).abs() < 1e-4);
        assert!((end.barrier - CUPERTINO_BARRIER_OPACITY).abs() < 1e-4);
        assert!(end.mirrored);
        let half = B::Cupertino.frame(0.5, false, true);
        assert!((half.top.dx - 0.5).abs() < 1e-4, "linear under a finger");
        assert!((half.below.dx + 1.0 / 6.0).abs() < 1e-4);
        let eased = B::Cupertino.frame(0.5, false, false);
        assert!(eased.top.dx != 0.5, "eased when no finger drives it");
    }

    /// **The fade forwards**: a quarter of the width each way, the new page faded in over
    /// three quarters of the way, the old one out over the first quarter; not mirrored.
    #[test]
    fn the_fade_forwards_moves_a_quarter() {
        let start = B::FadeForwards.frame(0.0, true, false);
        assert_eq!((start.top.dx, start.top.opacity), (0.25, 0.0));
        let quarter = B::FadeForwards.frame(0.25, true, false);
        assert_eq!(quarter.below.opacity, 0.0);
        // Halfway, the new page is two thirds in: its fade runs over the first 75 %.
        let half = B::FadeForwards.frame(0.5, true, false);
        assert!((half.top.opacity - 2.0 / 3.0).abs() < 1e-4, "{}", half.top.opacity);
        let back = B::FadeForwards.frame(0.5, false, false);
        assert!((back.below.opacity - 2.0 / 3.0).abs() < 1e-4, "and on the way back");
        let end = B::FadeForwards.frame(1.0, true, false);
        assert!((end.below.dx + 0.25).abs() < 1e-4);
        assert!(!end.mirrored);
        let pop = B::FadeForwards.frame(0.0, false, false);
        assert_eq!(pop.below.dx, -0.25);
    }
}
