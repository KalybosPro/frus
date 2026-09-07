//! **The crossing from one theme to the next.**
//!
//! An application names its themes and says which one it wants ([`Application::theme`],
//! [`dark_theme`], [`theme_mode`], the high-contrast pair). Everything after that —
//! reading the platform's brightness, picking one of the four, and fading from whatever
//! was on screen to whatever is now asked for — happens here, once a frame, for every
//! application.
//!
//! Before milestone 452 none of it did. `Application` had a single `theme()`, so an
//! application that wanted to follow the system's dark mode had to read
//! `MediaQuery::platform_brightness` itself, and an application that wanted the change to
//! be anything but instantaneous had to keep the outgoing theme and a progress value in
//! its own state and interpolate them in `theme()`. This repo's own demonstration did
//! exactly that, which is how the gap was found.
//!
//! [`Application::theme`]: crate::Application::theme
//! [`dark_theme`]: crate::Application::dark_theme
//! [`theme_mode`]: crate::Application::theme_mode

use frus_widgets::{Brightness, Theme};

use crate::application::Application;

/// **The theme on display, and the fade toward the one now asked for.**
///
/// The reference's `AnimatedTheme` (`app.dart:1057`), which its app widget wraps every
/// application in. It watches the *resolved* theme rather than any one of the properties
/// that produced it, so every way of changing the look animates the same way: a
/// light/dark switch, a new seed colour, a different radius, a mode moved off `System`.
#[derive(Default)]
pub(crate) struct ThemeFade {
    /// The blend on display. `None` until the first frame has resolved one.
    current: Option<Theme>,
    /// What the application last asked for — the fade's destination.
    target: Option<Theme>,
    /// Where the running fade started; `None` when nothing is moving.
    from: Option<Theme>,
    /// `0 → 1` across the application's animation duration.
    progress: f32,
}

impl ThemeFade {
    /// Is a fade still running, so the frame should ask for another?
    pub(crate) fn animating(&self) -> bool {
        self.from.is_some()
    }

    /// **The theme to build, lay out and paint with this frame.**
    ///
    /// `app` is asked only as a last resort: before the first frame — a gesture can
    /// arrive that early, and the shell reads the direction off a theme — there is no
    /// resolved one yet, and the application's own is the honest answer.
    pub(crate) fn displayed<A: Application>(&self, app: &A) -> Theme {
        match &self.current {
            Some(theme) => theme.clone(),
            None => app.theme(),
        }
    }

    /// Resolves what the application wants and moves the blend `dt` seconds toward it.
    /// Returns **whether the theme on display moved**, which is a rebuild: the view is a
    /// pure function of `(state, theme, size)`.
    ///
    /// `still` is the reader's *reduce motion* setting. It ends the crossing at once
    /// rather than skipping it — the theme still changes, it just stops moving, which is
    /// what the setting asks for and what the framework does with every implicit
    /// animation it runs.
    pub(crate) fn advance<A: Application>(
        &mut self,
        app: &A,
        brightness: Brightness,
        high_contrast: bool,
        still: bool,
        dt: f32,
    ) -> bool {
        let target = app.resolved_theme(brightness, high_contrast);

        // The very first frame has nothing to cross from: the application opens on the
        // theme it asked for, rather than fading in from one it never showed.
        let Some(current) = self.current.clone() else {
            self.current = Some(target.clone());
            self.target = Some(target);
            return true;
        };

        if self.target.as_ref() != Some(&target) {
            self.from = Some(current.clone());
            self.target = Some(target);
            self.progress = 0.0;
        }

        if self.from.is_none() {
            return false;
        }

        let duration = if still {
            0.0
        } else {
            app.theme_animation_duration().max(0.0)
        };
        self.progress = if duration <= 0.0 {
            1.0
        } else {
            (self.progress + dt / duration).min(1.0)
        };

        let destination = self.target.clone().unwrap_or_else(|| app.theme());
        let next = if self.progress >= 1.0 {
            self.from = None;
            destination
        } else {
            let t = app.theme_animation_curve().transform(self.progress);
            self.from
                .as_ref()
                .map(|from| from.lerp(&destination, t))
                .unwrap_or(destination)
        };
        let moved = next != current;
        self.current = Some(next);
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeFade;
    use crate::application::Application;
    use frus_widgets::{Brightness, Theme, ThemeMode, Widget};

    /// An application with a light theme, a dark one, and a mode it can be told.
    ///
    /// The themes are told apart by their `radius`, which `Theme::lerp` interpolates —
    /// so it also reads the blend mid-crossing, which no equality check on whole themes
    /// could.
    struct Two {
        mode: ThemeMode,
        dark: bool,
        high_contrast: Option<f32>,
        high_contrast_dark: Option<f32>,
        duration: f32,
    }

    impl Default for Two {
        fn default() -> Self {
            Two {
                mode: ThemeMode::System,
                dark: true,
                high_contrast: None,
                high_contrast_dark: None,
                duration: 0.2,
            }
        }
    }

    fn marked(radius: f32) -> Theme {
        let mut theme = Theme::light();
        theme.radius = radius;
        theme
    }

    impl Application for Two {
        type Message = ();

        fn update(&mut self, _message: ()) -> crate::Command<()> {
            crate::Command::none()
        }

        fn view(&self, _theme: &Theme) -> Box<dyn Widget<()>> {
            Box::new(frus_widgets::Flex::<()>::column())
        }

        fn theme(&self) -> Theme {
            marked(0.0)
        }

        fn dark_theme(&self) -> Option<Theme> {
            self.dark.then(|| marked(100.0))
        }

        fn high_contrast_theme(&self) -> Option<Theme> {
            self.high_contrast.map(marked)
        }

        fn high_contrast_dark_theme(&self) -> Option<Theme> {
            self.high_contrast_dark.map(marked)
        }

        fn theme_mode(&self) -> ThemeMode {
            self.mode
        }

        fn theme_animation_duration(&self) -> f32 {
            self.duration
        }
    }

    /// **The platform's dark mode reaches an application that never reads it.**
    ///
    /// The brightness has been reported since milestone 380 and nothing could act on it:
    /// `Application` had one theme, so following the system meant reading
    /// `MediaQuery::platform_brightness` by hand. Naming a second theme is now the whole
    /// of it — `ThemeMode::System` is the default, and the framework does the rest.
    #[test]
    fn a_platform_s_dark_mode_reaches_an_application_that_never_asked() {
        let app = Two::default();
        assert_eq!(
            app.resolved_theme(Brightness::Dark, false).radius,
            100.0,
            "a dark platform, and an application that only supplied a dark theme"
        );
        assert_eq!(app.resolved_theme(Brightness::Light, false).radius, 0.0);

        // And a mode pins it, whatever the platform says.
        let pinned = Two {
            mode: ThemeMode::Light,
            ..Two::default()
        };
        assert_eq!(pinned.resolved_theme(Brightness::Dark, false).radius, 0.0);
        let pinned = Two {
            mode: ThemeMode::Dark,
            ..Two::default()
        };
        assert_eq!(
            pinned.resolved_theme(Brightness::Light, false).radius,
            100.0
        );

        // A mode asking for dark with nothing dark to give falls back rather than
        // failing: an application is never left with no theme at all.
        let unlit = Two {
            mode: ThemeMode::Dark,
            dark: false,
            ..Two::default()
        };
        assert_eq!(unlit.resolved_theme(Brightness::Dark, false).radius, 0.0);
    }

    /// **The rungs of a high-contrast interface** (`app.dart:1004`), including the one
    /// that is easy to get wrong: dark **and** high contrast with no high-contrast dark
    /// theme takes the plain dark one — being right about the brightness matters more
    /// than being right about the contrast — and only falls to the high-contrast light
    /// theme when there is no dark theme at all.
    #[test]
    fn the_rungs_of_a_high_contrast_interface() {
        let all = Two {
            high_contrast: Some(1.0),
            high_contrast_dark: Some(2.0),
            ..Two::default()
        };
        assert_eq!(all.resolved_theme(Brightness::Dark, true).radius, 2.0);
        assert_eq!(all.resolved_theme(Brightness::Light, true).radius, 1.0);
        assert_eq!(all.resolved_theme(Brightness::Dark, false).radius, 100.0);
        assert_eq!(all.resolved_theme(Brightness::Light, false).radius, 0.0);

        let no_hc_dark = Two {
            high_contrast: Some(1.0),
            ..Two::default()
        };
        assert_eq!(
            no_hc_dark.resolved_theme(Brightness::Dark, true).radius,
            100.0,
            "the dark theme, not the high-contrast light one"
        );

        let nothing_dark = Two {
            dark: false,
            high_contrast: Some(1.0),
            ..Two::default()
        };
        assert_eq!(
            nothing_dark.resolved_theme(Brightness::Dark, true).radius,
            1.0,
            "and with no dark theme at all, the high-contrast one"
        );
    }

    /// **The framework crosses from one theme to the next**, which every application used
    /// to have to do for itself: hold the outgoing theme and a progress value in its own
    /// state, advance it in `tick`, and interpolate in `theme()`. This repo's own
    /// demonstration did exactly that until milestone 452.
    #[test]
    fn the_framework_crosses_from_one_theme_to_the_next() {
        let mut app = Two {
            mode: ThemeMode::Light,
            ..Two::default()
        };
        let mut fade = ThemeFade::default();

        // The first frame opens on the theme asked for, with no crossing from a theme
        // that was never on screen.
        assert!(fade.advance(&app, Brightness::Light, false, false, 0.016));
        assert_eq!(fade.displayed(&app).radius, 0.0);
        assert!(!fade.animating(), "nothing to cross on the first frame");

        // A frame that changes nothing changes nothing.
        assert!(!fade.advance(&app, Brightness::Light, false, false, 0.016));

        // The application switches. The **next** frame is already on its way there, and
        // is neither theme.
        app.mode = ThemeMode::Dark;
        assert!(fade.advance(&app, Brightness::Light, false, false, 0.1));
        assert!(fade.animating(), "and it asks for another frame");
        let midway = fade.displayed(&app).radius;
        assert!(
            midway > 0.0 && midway < 100.0,
            "halfway across a 0.2s crossing: {midway}"
        );

        // And it arrives, once, on the theme asked for.
        assert!(fade.advance(&app, Brightness::Light, false, false, 0.1));
        assert_eq!(fade.displayed(&app).radius, 100.0);
        assert!(!fade.animating());
        assert!(
            !fade.advance(&app, Brightness::Light, false, false, 0.1),
            "an arrived theme stops asking for frames"
        );
    }

    /// **Anything that changes the resolved theme is crossed**, not only a light/dark
    /// switch: what is watched is the theme the application resolves to, so a new palette
    /// on one and the same mode fades the same way. That is what the demonstration's
    /// *seed* cycle needs, and it is why watching `theme_mode` instead would have been a
    /// bug waiting to be found.
    #[test]
    fn a_new_palette_crosses_the_same_way_as_a_light_dark_switch() {
        struct Seeded(std::cell::Cell<f32>);

        impl Application for Seeded {
            type Message = ();

            fn update(&mut self, _message: ()) -> crate::Command<()> {
                crate::Command::none()
            }

            fn view(&self, _theme: &Theme) -> Box<dyn Widget<()>> {
                Box::new(frus_widgets::Flex::<()>::column())
            }

            fn theme(&self) -> Theme {
                marked(self.0.get())
            }
        }

        let app = Seeded(std::cell::Cell::new(0.0));
        let mut fade = ThemeFade::default();
        fade.advance(&app, Brightness::Light, false, false, 0.016);
        assert!(!fade.animating());

        app.0.set(100.0);
        assert!(fade.advance(&app, Brightness::Light, false, false, 0.1));
        assert!(fade.animating(), "the same crossing, on the same one theme");
        let midway = fade.displayed(&app).radius;
        assert!(midway > 0.0 && midway < 100.0, "{midway}");
    }

    /// **A reader who asked for less motion gets the change at once** — the change still
    /// happens, it stops moving. That is what the setting asks for, and what the framework
    /// does with every implicit animation it runs.
    ///
    /// A duration of zero says the same thing, and an application may say it.
    #[test]
    fn stillness_and_a_zero_duration_switch_at_once() {
        let mut app = Two {
            mode: ThemeMode::Light,
            ..Two::default()
        };
        let mut fade = ThemeFade::default();
        fade.advance(&app, Brightness::Light, false, true, 0.016);

        app.mode = ThemeMode::Dark;
        assert!(fade.advance(&app, Brightness::Light, false, true, 0.016));
        assert_eq!(
            fade.displayed(&app).radius,
            100.0,
            "the whole way in one frame"
        );
        assert!(!fade.animating(), "and nothing left to animate");

        let mut app = Two {
            mode: ThemeMode::Light,
            duration: 0.0,
            ..Two::default()
        };
        let mut fade = ThemeFade::default();
        fade.advance(&app, Brightness::Light, false, false, 0.016);
        app.mode = ThemeMode::Dark;
        fade.advance(&app, Brightness::Light, false, false, 0.016);
        assert_eq!(fade.displayed(&app).radius, 100.0);
        assert!(!fade.animating());
    }

    /// **Before the first frame there is still a theme to answer with.** The shell reads
    /// the layout direction off one while handling a gesture, which can arrive before
    /// anything has been resolved.
    #[test]
    fn a_theme_can_be_asked_for_before_the_first_frame() {
        let fade = ThemeFade::default();
        assert_eq!(fade.displayed(&Two::default()).radius, marked(0.0).radius);
    }
}
