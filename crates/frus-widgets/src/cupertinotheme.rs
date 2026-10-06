//! The theme of the Apple-style widgets: their primary colour, their bars' and pages'
//! backgrounds, and their type (milestone 628).
//!
//! It is **part of the main theme**, as in the reference: unless something says otherwise,
//! the Apple-style widgets take their primary colour, the colour on it and the page
//! background from the main [`Theme`], and only what the main theme has no word for — a
//! bar's translucent background, the type — from Apple's own defaults. An application
//! changes any of it with [`Theme::with_cupertino_override_theme`]; a subtree replaces all of
//! it with [`CupertinoTheme::around`].

use frus_core::{Color, FontWeight, TextDecoration, TextStyle};

use crate::media::{Brightness, MediaQuery};
use crate::{CupertinoColors, CupertinoDynamicColor, Theme};

/// The type of the Apple-style widgets **as asked for**: each style an application sets, or
/// `None` for the default.
///
/// The defaults are the reference's (`cupertino/text_theme.dart`): body text at 17 px, a
/// navigation bar's title semibold at 17, its large title bold at 34, a tab's label at 10,
/// and the letter spacing Apple's guidelines give each. Their colours adapt: the label colour
/// for text, the primary colour for actions, the inactive grey for tab labels.
///
/// The reference names Apple's system faces, which exist only on Apple's platforms; the
/// defaults here name no family, so they are set in the application's own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CupertinoTextThemeData {
    /// The colour of the actions' text. Unset, the system blue — or, for the defaults the
    /// theme makes when no text theme is given, the theme's primary colour.
    pub primary_color: Option<CupertinoDynamicColor>,
    /// Body text.
    pub text_style: Option<TextStyle>,
    /// The text of a button.
    pub action_text_style: Option<TextStyle>,
    /// The text of a small button.
    pub action_small_text_style: Option<TextStyle>,
    /// A tab bar's labels.
    pub tab_label_text_style: Option<TextStyle>,
    /// A navigation bar's title.
    pub nav_title_text_style: Option<TextStyle>,
    /// A navigation bar's large title.
    pub nav_large_title_text_style: Option<TextStyle>,
    /// A navigation bar's buttons.
    pub nav_action_text_style: Option<TextStyle>,
    /// A picker's items.
    pub picker_text_style: Option<TextStyle>,
    /// A date and time picker's items.
    pub date_time_picker_text_style: Option<TextStyle>,
}

/// The type of the Apple-style widgets **resolved**: every style, its colour picked for where
/// it is used. [`CupertinoTheme::of`] makes one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CupertinoTextTheme {
    /// Body text.
    pub text_style: TextStyle,
    /// The text of a button.
    pub action_text_style: TextStyle,
    /// The text of a small button.
    pub action_small_text_style: TextStyle,
    /// A tab bar's labels.
    pub tab_label_text_style: TextStyle,
    /// A navigation bar's title.
    pub nav_title_text_style: TextStyle,
    /// A navigation bar's large title.
    pub nav_large_title_text_style: TextStyle,
    /// A navigation bar's buttons.
    pub nav_action_text_style: TextStyle,
    /// A picker's items.
    pub picker_text_style: TextStyle,
    /// A date and time picker's items.
    pub date_time_picker_text_style: TextStyle,
}

/// Body text: 17 px, −0.41 apart, undecorated (`text_theme.dart:19`).
const TEXT: TextStyle = TextStyle::new(17.0)
    .letter_spacing(-0.41)
    .decoration(TextDecoration::NONE);
/// A button's text: as body text, in the primary colour (`text_theme.dart:34`).
const ACTION: TextStyle = TEXT;
/// A small button's text: 15 px, −0.23 apart (`text_theme.dart:49`).
const ACTION_SMALL: TextStyle = TextStyle::new(15.0)
    .letter_spacing(-0.23)
    .decoration(TextDecoration::NONE);
/// A tab's label: 10 px, medium, −0.24 apart (`text_theme.dart:63`).
const TAB_LABEL: TextStyle = TextStyle::new(10.0)
    .weight(FontWeight::Medium)
    .letter_spacing(-0.24);
/// A navigation bar's title: 17 px, semibold, −0.41 apart (`text_theme.dart:72`).
const NAV_TITLE: TextStyle = TextStyle::new(17.0)
    .weight(FontWeight::SemiBold)
    .letter_spacing(-0.41);
/// A navigation bar's large title: 34 px, bold, 0.38 apart (`text_theme.dart:81`).
const NAV_LARGE_TITLE: TextStyle = TextStyle::new(34.0)
    .weight(FontWeight::Bold)
    .letter_spacing(0.38);
/// A picker's items: 21 px, regular, −0.6 apart (`text_theme.dart:101`).
const PICKER: TextStyle = TextStyle::new(21.0)
    .weight(FontWeight::Regular)
    .letter_spacing(-0.6);
/// A date and time picker's items: 21 px, regular, 0.4 apart (`text_theme.dart:116`).
const DATE_TIME_PICKER: TextStyle = TextStyle::new(21.0)
    .weight(FontWeight::Regular)
    .letter_spacing(0.4);

impl CupertinoTextThemeData {
    /// Nothing asked for: every style the default.
    pub const NONE: Self = Self {
        primary_color: None,
        text_style: None,
        action_text_style: None,
        action_small_text_style: None,
        tab_label_text_style: None,
        nav_title_text_style: None,
        nav_large_title_text_style: None,
        nav_action_text_style: None,
        picker_text_style: None,
        date_time_picker_text_style: None,
    };

    /// The same, its actions in `color`.
    #[must_use]
    pub fn primary_color(mut self, color: impl Into<CupertinoDynamicColor>) -> Self {
        self.primary_color = Some(color.into());
        self
    }

    /// The same, its body text in `style`.
    #[must_use]
    pub const fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// The same, its buttons' text in `style`.
    #[must_use]
    pub const fn action_text_style(mut self, style: TextStyle) -> Self {
        self.action_text_style = Some(style);
        self
    }

    /// The same, its navigation bar's title in `style`.
    #[must_use]
    pub const fn nav_title_text_style(mut self, style: TextStyle) -> Self {
        self.nav_title_text_style = Some(style);
        self
    }

    /// The same, its navigation bar's large title in `style`.
    #[must_use]
    pub const fn nav_large_title_text_style(mut self, style: TextStyle) -> Self {
        self.nav_large_title_text_style = Some(style);
        self
    }

    /// **Every style, for where it is used**: a style asked for as it was asked, a default in
    /// the colour `theme` resolves its role to.
    pub fn resolve(&self, theme: &Theme) -> CupertinoTextTheme {
        let label = CupertinoColors::LABEL.resolve(theme);
        let inactive = CupertinoColors::INACTIVE_GRAY.resolve(theme);
        let primary = self
            .primary_color
            .unwrap_or(CupertinoColors::SYSTEM_BLUE)
            .resolve(theme);
        let action = self.action_text_style.unwrap_or(ACTION.color(primary));
        CupertinoTextTheme {
            text_style: self.text_style.unwrap_or(TEXT.color(label)),
            action_text_style: action,
            action_small_text_style: self
                .action_small_text_style
                .unwrap_or(ACTION_SMALL.color(primary)),
            tab_label_text_style: self
                .tab_label_text_style
                .unwrap_or(TAB_LABEL.color(inactive)),
            nav_title_text_style: self.nav_title_text_style.unwrap_or(NAV_TITLE.color(label)),
            nav_large_title_text_style: self
                .nav_large_title_text_style
                .unwrap_or(NAV_LARGE_TITLE.color(label)),
            // A navigation bar's buttons are buttons: unset, the default button text, in
            // the primary colour — not the action style an application set.
            nav_action_text_style: self.nav_action_text_style.unwrap_or(ACTION.color(primary)),
            picker_text_style: self.picker_text_style.unwrap_or(PICKER.color(label)),
            date_time_picker_text_style: self
                .date_time_picker_text_style
                .unwrap_or(DATE_TIME_PICKER.color(label)),
        }
    }
}

/// The theme of the Apple-style widgets **as asked for**: each value an application sets, or
/// `None` for the default — the reference's `CupertinoThemeData`.
///
/// Where it applies decides what the defaults are. As the main theme's override
/// ([`Theme::with_cupertino_override_theme`]) the primary colour, the colour on it, the page
/// background and the brightness default to the main theme's; as a subtree's own theme
/// ([`CupertinoTheme::around`]) they default to Apple's — the system blue, white, the
/// system background, and the system's brightness.
///
/// A colour may be [adaptive](CupertinoDynamicColor) or a plain [`Color`]; it is resolved
/// where it is used.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CupertinoThemeData {
    /// Light or dark. It decides how every adaptive colour under it resolves.
    pub brightness: Option<Brightness>,
    /// The colour of the controls: a button's label, a switch that is on, a selected tab.
    pub primary_color: Option<CupertinoDynamicColor>,
    /// The colour of what sits on the primary colour.
    pub primary_contrasting_color: Option<CupertinoDynamicColor>,
    /// The type.
    pub text_theme: Option<Box<CupertinoTextThemeData>>,
    /// The background of the bars: navigation, tab, toolbar.
    pub bar_background_color: Option<CupertinoDynamicColor>,
    /// The background of a page.
    pub scaffold_background_color: Option<CupertinoDynamicColor>,
    /// The handles of a text selection.
    pub selection_handle_color: Option<CupertinoDynamicColor>,
    /// Whether the widgets that adapt to the platform take this theme's colours on every
    /// platform, or only on Apple's.
    pub apply_theme_to_all: Option<bool>,
}

impl CupertinoThemeData {
    /// Nothing asked for: every value the default.
    pub fn new() -> Self {
        Self::default()
    }

    /// The same, light or dark.
    #[must_use]
    pub fn brightness(mut self, brightness: Brightness) -> Self {
        self.brightness = Some(brightness);
        self
    }

    /// The same, its controls in `color`.
    #[must_use]
    pub fn primary_color(mut self, color: impl Into<CupertinoDynamicColor>) -> Self {
        self.primary_color = Some(color.into());
        self
    }

    /// The same, what sits on the primary colour in `color`.
    #[must_use]
    pub fn primary_contrasting_color(mut self, color: impl Into<CupertinoDynamicColor>) -> Self {
        self.primary_contrasting_color = Some(color.into());
        self
    }

    /// The same, set in `text_theme`.
    #[must_use]
    pub fn text_theme(mut self, text_theme: CupertinoTextThemeData) -> Self {
        self.text_theme = Some(Box::new(text_theme));
        self
    }

    /// The same, its bars on `color`.
    #[must_use]
    pub fn bar_background_color(mut self, color: impl Into<CupertinoDynamicColor>) -> Self {
        self.bar_background_color = Some(color.into());
        self
    }

    /// The same, its pages on `color`.
    #[must_use]
    pub fn scaffold_background_color(mut self, color: impl Into<CupertinoDynamicColor>) -> Self {
        self.scaffold_background_color = Some(color.into());
        self
    }

    /// The same, its selection handles in `color`.
    #[must_use]
    pub fn selection_handle_color(mut self, color: impl Into<CupertinoDynamicColor>) -> Self {
        self.selection_handle_color = Some(color.into());
        self
    }

    /// The same, its colours taken on every platform or only on Apple's.
    #[must_use]
    pub fn apply_theme_to_all(mut self, all: bool) -> Self {
        self.apply_theme_to_all = Some(all);
        self
    }
}

/// The theme of the Apple-style widgets **resolved** for where it is used: every value
/// concrete, every adaptive colour picked. [`CupertinoTheme::of`] makes one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CupertinoTheme {
    /// Light or dark.
    pub brightness: Brightness,
    /// The colour of the controls.
    pub primary_color: Color,
    /// The colour of what sits on the primary colour.
    pub primary_contrasting_color: Color,
    /// The background of the bars.
    pub bar_background_color: Color,
    /// The background of a page.
    pub scaffold_background_color: Color,
    /// The handles of a text selection.
    pub selection_handle_color: Color,
    /// Whether the adaptive widgets take these colours on every platform.
    pub apply_theme_to_all: bool,
    /// The type.
    pub text_theme: CupertinoTextTheme,
}

impl CupertinoTheme {
    /// The bars' default background: nearly opaque, nearly white in the light and nearly
    /// black in the dark, so that what scrolls under a bar shows through a little
    /// (`cupertino/theme.dart:26`).
    pub const DEFAULT_BAR_BACKGROUND: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness(
            Color::from_argb_u32(0xF0F9F9F9),
            Color::from_argb_u32(0xF01D1D1D),
        );

    /// **The Apple-style theme in force under `theme`**, resolved.
    ///
    /// A subtree's own ([`around`](Self::around)) if it has one, with Apple's defaults
    /// under it; otherwise the main theme's, with the main theme's colours as the defaults
    /// its override does not replace — the reference's `MaterialBasedCupertinoThemeData`.
    pub fn of(theme: &Theme) -> Self {
        let own = theme.cupertino_theme.as_deref();
        let data = own
            .or(theme.cupertino_override_theme.as_deref())
            .cloned()
            .unwrap_or_default();
        let main = own.is_none();
        let resolve = |c: CupertinoDynamicColor| c.resolve(theme);
        let primary = data.primary_color.unwrap_or(if main {
            theme.scheme.primary.into()
        } else {
            CupertinoColors::SYSTEM_BLUE
        });
        let text_theme = match data.text_theme.as_deref() {
            Some(asked) => asked.resolve(theme),
            None => CupertinoTextThemeData::NONE
                .primary_color(primary)
                .resolve(theme),
        };
        // The main theme's selection handles, if it names them, before Apple's blue.
        let main_handles = if main {
            theme.widgets.text_selection.handle_color.map(Into::into)
        } else {
            None
        };
        Self {
            brightness: Self::brightness_of(theme),
            primary_color: resolve(primary),
            primary_contrasting_color: resolve(data.primary_contrasting_color.unwrap_or(if main {
                theme.scheme.on_primary.into()
            } else {
                CupertinoColors::WHITE.into()
            })),
            bar_background_color: resolve(
                data.bar_background_color
                    .unwrap_or(Self::DEFAULT_BAR_BACKGROUND),
            ),
            scaffold_background_color: resolve(data.scaffold_background_color.unwrap_or(if main {
                theme.background.into()
            } else {
                CupertinoColors::SYSTEM_BACKGROUND
            })),
            selection_handle_color: resolve(
                data.selection_handle_color
                    .or(main_handles)
                    .unwrap_or(CupertinoColors::SYSTEM_BLUE),
            ),
            apply_theme_to_all: data.apply_theme_to_all.unwrap_or(false),
            text_theme,
        }
    }

    /// **Light or dark, for the Apple-style widgets under `theme`** — what every adaptive
    /// colour resolves by.
    ///
    /// A subtree's own theme's brightness, else the system's; under the main theme, its
    /// override's, else the main theme's own (the reference's `maybeBrightnessOf`).
    pub fn brightness_of(theme: &Theme) -> Brightness {
        match theme.cupertino_theme.as_deref() {
            Some(own) => own
                .brightness
                .unwrap_or_else(|| MediaQuery::of().platform_brightness),
            None => theme
                .cupertino_override_theme
                .as_deref()
                .and_then(|o| o.brightness)
                .unwrap_or_else(|| theme.brightness()),
        }
    }

    /// Wraps `child` in `data`: the Apple-style widgets inside it take this theme, with
    /// Apple's defaults for what it does not set, and the icons inside it take its primary
    /// colour, as under the reference's `CupertinoTheme`.
    pub fn around<Msg: 'static>(
        data: CupertinoThemeData,
        child: impl crate::Widget<Msg> + 'static,
    ) -> crate::Themed<Msg> {
        crate::Themed::tweak(
            move |t| {
                t.cupertino_theme = Some(Box::new(data.clone()));
                t.widgets.icon.color = Some(CupertinoTheme::of(t).primary_color);
            },
            child,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{CupertinoTextThemeData, CupertinoTheme, CupertinoThemeData};
    use crate::media::{Brightness, MediaQuery};
    use crate::{
        build_ui, Container, CupertinoColors as C, Flex, Runtime, Theme, ThemeBuilder, Widget,
    };
    use frus_core::{Color, FontWeight, Primitive, Size, TextStyle};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    /// **Under the main theme, its colours are the defaults**: its primary, the colour on
    /// it, its background and its brightness — and Apple's only for what it has no word for.
    #[test]
    fn under_the_main_theme_its_colours_are_the_defaults() {
        let theme = Theme::dark();
        let c = CupertinoTheme::of(&theme);
        assert_eq!(c.brightness, Brightness::Dark);
        assert_eq!(c.primary_color, theme.scheme.primary);
        assert_eq!(c.primary_contrasting_color, theme.scheme.on_primary);
        assert_eq!(c.scaffold_background_color, theme.background);
        assert_eq!(
            c.bar_background_color,
            CupertinoTheme::DEFAULT_BAR_BACKGROUND.dark_color
        );
        assert_eq!(c.selection_handle_color, C::SYSTEM_BLUE.dark_color);
        assert!(!c.apply_theme_to_all);
        // The actions' type is in the primary colour; text in the label colour.
        assert_eq!(
            c.text_theme.action_text_style.color,
            Some(theme.scheme.primary)
        );
        assert_eq!(c.text_theme.text_style.color, Some(C::LABEL.dark_color));
        // The main theme's own selection handles come before Apple's blue.
        let mut named = Theme::light();
        named.widgets.text_selection.handle_color = Some(Color::rgb8(1, 2, 3));
        assert_eq!(
            CupertinoTheme::of(&named).selection_handle_color,
            Color::rgb8(1, 2, 3)
        );
    }

    /// **The override replaces what it sets, and its brightness rules every adaptive
    /// colour** — the system background turns black under a dark override of a light theme.
    #[test]
    fn the_override_replaces_what_it_sets() {
        let theme = Theme::light().with_cupertino_override_theme(
            CupertinoThemeData::new()
                .brightness(Brightness::Dark)
                .primary_color(C::SYSTEM_GREEN)
                .scaffold_background_color(C::SYSTEM_BACKGROUND),
        );
        let c = CupertinoTheme::of(&theme);
        assert_eq!(c.brightness, Brightness::Dark);
        assert_eq!(c.primary_color, C::SYSTEM_GREEN.dark_color);
        assert_eq!(c.scaffold_background_color, C::SYSTEM_BACKGROUND.dark_color);
        // Still the main theme's: the colour on the primary.
        assert_eq!(c.primary_contrasting_color, theme.scheme.on_primary);
        assert_eq!(C::LABEL.resolve(&theme), C::LABEL.dark_color);
        assert_eq!(
            c.text_theme.action_text_style.color,
            Some(C::SYSTEM_GREEN.dark_color)
        );
    }

    /// **A subtree's own theme has Apple's defaults** — the system blue, white, the system
    /// background — and, unless it says, the system's brightness.
    #[test]
    fn a_subtree_s_own_theme_has_apple_s_defaults() {
        let mut theme = Theme::light();
        theme.cupertino_theme = Some(Box::new(CupertinoThemeData::new()));
        let surface = MediaQuery {
            platform_brightness: Brightness::Dark,
            ..MediaQuery::new(Size::new(100.0, 100.0))
        };
        surface.scope(|| {
            let c = CupertinoTheme::of(&theme);
            assert_eq!(c.brightness, Brightness::Dark);
            assert_eq!(c.primary_color, C::SYSTEM_BLUE.dark_color);
            assert_eq!(c.primary_contrasting_color, C::WHITE);
            assert_eq!(c.scaffold_background_color, C::SYSTEM_BACKGROUND.dark_color);
        });
        // Its own brightness wins over the system's.
        theme.cupertino_theme = Some(Box::new(
            CupertinoThemeData::new().brightness(Brightness::Light),
        ));
        surface.scope(|| {
            assert_eq!(
                CupertinoTheme::of(&theme).scaffold_background_color,
                C::SYSTEM_BACKGROUND.color
            )
        });
    }

    /// **The default type is the reference's**, size, weight and spacing; a style asked for
    /// is taken as it is, and a text theme given without a primary colour puts its actions in
    /// the system blue.
    #[test]
    fn the_default_type_is_the_reference_s() {
        let t = CupertinoTheme::of(&Theme::light()).text_theme;
        let spec = |s: TextStyle| (s.size, s.weight, s.letter_spacing);
        assert_eq!(spec(t.text_style), (Some(17.0), None, Some(-0.41)));
        assert_eq!(spec(t.action_text_style), (Some(17.0), None, Some(-0.41)));
        assert_eq!(
            spec(t.action_small_text_style),
            (Some(15.0), None, Some(-0.23))
        );
        assert_eq!(
            spec(t.tab_label_text_style),
            (Some(10.0), Some(FontWeight::Medium), Some(-0.24))
        );
        assert_eq!(
            spec(t.nav_title_text_style),
            (Some(17.0), Some(FontWeight::SemiBold), Some(-0.41))
        );
        assert_eq!(
            spec(t.nav_large_title_text_style),
            (Some(34.0), Some(FontWeight::Bold), Some(0.38))
        );
        assert_eq!(
            spec(t.picker_text_style),
            (Some(21.0), Some(FontWeight::Regular), Some(-0.6))
        );
        assert_eq!(
            spec(t.date_time_picker_text_style),
            (Some(21.0), Some(FontWeight::Regular), Some(0.4))
        );
        assert_eq!(t.tab_label_text_style.color, Some(C::INACTIVE_GRAY.color));
        assert_eq!(t.nav_title_text_style.color, Some(C::LABEL.color));

        let mine = TextStyle::new(20.0);
        let theme = Theme::light().with_cupertino_override_theme(
            CupertinoThemeData::new()
                .text_theme(CupertinoTextThemeData::NONE.nav_title_text_style(mine)),
        );
        let t = CupertinoTheme::of(&theme).text_theme;
        assert_eq!(t.nav_title_text_style, mine);
        assert_eq!(t.action_text_style.color, Some(C::SYSTEM_BLUE.color));
    }

    /// **A theme keeps its Apple-style parts through a fade, switching halfway, and a subtree
    /// wrapped in its own theme resolves by it — its icons in its primary colour.**
    #[test]
    fn a_subtree_takes_its_own_theme() {
        let green = Theme::light().with_cupertino_override_theme(
            CupertinoThemeData::new().primary_color(C::SYSTEM_GREEN),
        );
        let plain = Theme::light();
        assert_eq!(
            plain.lerp(&green, 0.7).cupertino_override_theme,
            green.cupertino_override_theme
        );
        assert_eq!(plain.lerp(&green, 0.3).cupertino_override_theme, None);

        let swatch = || {
            ThemeBuilder::new(|t: &Theme| {
                Container::new()
                    .width(50.0)
                    .height(50.0)
                    .color(CupertinoTheme::of(t).primary_color)
            })
        };
        let icon = || {
            ThemeBuilder::new(|t: &Theme| {
                Container::new()
                    .width(50.0)
                    .height(50.0)
                    .color(t.widgets.icon.color.unwrap_or(Color::TRANSPARENT))
            })
        };
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(swatch())
            .child(CupertinoTheme::around(CupertinoThemeData::new(), swatch()))
            .child(CupertinoTheme::around(
                CupertinoThemeData::new().primary_color(C::SYSTEM_PINK),
                icon(),
            ));
        let painted: Vec<Color> = build_ui(
            &tree as &dyn Widget<Msg>,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::light(),
        )
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(*color),
            _ => None,
        })
        .collect();
        assert_eq!(
            painted,
            vec![
                Theme::light().scheme.primary,
                C::SYSTEM_BLUE.color,
                C::SYSTEM_PINK.color
            ]
        );
    }
}
