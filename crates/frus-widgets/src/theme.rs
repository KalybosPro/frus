//! The theme: *design tokens* (colors, radius, spacing) injected into rendering.
//!
//! The theme is handed to [`crate::build_ui`] and passed on to `Widget::paint`;
//! widgets use it for their default values (text color, text fields, scrollbars…),
//! without preventing an explicit override.

use frus_core::{Color, FontWeight, TextDirection, TextStyle};

use crate::interaction::Status;
use crate::media::Brightness;

/// **Which of an application's themes is on display** (`app.dart:57`).
///
/// An application supplies a light theme and, if it has one, a dark theme; this says
/// which of the two the framework picks. It is a *question about the application*, not
/// about the device: [`Brightness`](crate::Brightness) is what the platform reports, and
/// [`System`](Self::System) is the mode that agrees to follow it.
///
/// The framework resolves this once a frame and fades between the answers, so an
/// application never has to read the platform's brightness or write a crossfade of its
/// own.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeMode {
    /// Follow the platform: the dark theme where the system asks for a dark interface,
    /// the light one otherwise. **The default**, and the answer most applications want.
    #[default]
    System,
    /// The light theme, whatever the platform says.
    Light,
    /// The dark theme, whatever the platform says — falling back to the light one when
    /// the application has no dark theme to give.
    Dark,
}

impl ThemeMode {
    /// Does this mode follow the platform?
    pub const fn is_system(self) -> bool {
        matches!(self, ThemeMode::System)
    }

    /// Does this mode pin the light theme?
    pub const fn is_light(self) -> bool {
        matches!(self, ThemeMode::Light)
    }

    /// Does this mode pin the dark theme?
    pub const fn is_dark(self) -> bool {
        matches!(self, ThemeMode::Dark)
    }

    /// **Does this mode want a dark interface** on a platform reporting `brightness`?
    ///
    /// The one line the whole light/dark decision comes down to (`app.dart:1000`), and
    /// the reason it lives here rather than in the shell: an application that shows a
    /// *theme* setting with a *System* entry needs the same answer to tick the right row.
    pub const fn wants_dark(self, brightness: Brightness) -> bool {
        match self {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => matches!(brightness, Brightness::Dark),
        }
    }
}

/// The **named** typographic scale (Material 3's 15 steps). Widgets pick a step
/// (`theme.text.title_medium`), never a hardcoded size — changing the scale
/// retypesets the whole app. The colors stay inherited (`None` → resolved against
/// the theme at paint time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTheme {
    pub display_large: TextStyle,
    pub display_medium: TextStyle,
    pub display_small: TextStyle,
    pub headline_large: TextStyle,
    pub headline_medium: TextStyle,
    pub headline_small: TextStyle,
    pub title_large: TextStyle,
    pub title_medium: TextStyle,
    pub title_small: TextStyle,
    pub body_large: TextStyle,
    pub body_medium: TextStyle,
    pub body_small: TextStyle,
    pub label_large: TextStyle,
    pub label_medium: TextStyle,
    pub label_small: TextStyle,
}

impl TextTheme {
    /// The reference Material 3 scale (sizes in logical pixels; the title and label
    /// steps carry a medium weight, as the spec has it).
    ///
    /// A **const**, and that matters: a widget measured with no theme in hand — the
    /// un-themed [`Widget::style`](crate::Widget::style) path the transparent wrappers
    /// take — reads its step from *this* rather than from a private constant of its own.
    /// Twelve widgets used to carry their own number, and one of them had drifted two
    /// pixels from the reference without anybody being able to see it.
    pub const M3: Self = Self {
        display_large: TextStyle::new(57.0),
        display_medium: TextStyle::new(45.0),
        display_small: TextStyle::new(36.0),
        headline_large: TextStyle::new(32.0),
        headline_medium: TextStyle::new(28.0),
        headline_small: TextStyle::new(24.0),
        title_large: TextStyle::new(22.0),
        title_medium: TextStyle::new(16.0).weight(FontWeight::Medium),
        title_small: TextStyle::new(14.0).weight(FontWeight::Medium),
        body_large: TextStyle::new(16.0),
        body_medium: TextStyle::new(14.0),
        body_small: TextStyle::new(12.0),
        label_large: TextStyle::new(14.0).weight(FontWeight::Medium),
        label_medium: TextStyle::new(12.0).weight(FontWeight::Medium),
        label_small: TextStyle::new(11.0).weight(FontWeight::Medium),
    };
}

impl Default for TextTheme {
    fn default() -> Self {
        Self::M3
    }
}

/// The type scale a widget measures with: the theme's when it has one, the framework's
/// own when it does not.
///
/// `None` is the un-themed [`Widget::style`](crate::Widget::style) path. It answers from
/// the *same* scale as the themed one — the point of milestone 413 being that a widget
/// never decides its own type, and a fallback constant beside the scale would be that
/// decision taken back.
pub(crate) fn type_scale(theme: Option<&Theme>) -> TextTheme {
    theme.map_or(TextTheme::M3, |t| t.text)
}

/// The **color roles** (Material 3) — the **source of truth** for the theme's
/// colors. Widgets reference roles, never literal colors: changing scheme recolors
/// the whole app and guarantees the contrast of the `X`/`on_X` pairs. Written by
/// hand for light and dark; `from_seed` (HCT) comes after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorScheme {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    /// A **third** accent, for what is neither the app's main colour nor its supporting
    /// one: a highlight that has to stand apart from both. Generated a sixth of the way
    /// round the wheel from the seed, which is what keeps it from reading as either.
    pub tertiary: Color,
    pub on_tertiary: Color,
    pub tertiary_container: Color,
    pub on_tertiary_container: Color,
    pub background: Color,
    pub surface: Color,
    pub on_surface: Color,
    /// A discreet tonal surface (zone backgrounds, tracks).
    pub surface_variant: Color,
    /// Secondary content on surfaces (the historical `muted`).
    pub on_surface_variant: Color,
    /// The container ladder's **lowest** rung: the least emphasis against the
    /// surface. Its neighbour rather than its opposite — in a dark scheme it is
    /// *darker* than the surface, as the reference's is.
    pub surface_container_lowest: Color,
    /// Less emphasis than [`Self::surface_container`]: cards off the page, banners,
    /// drawers, sheets.
    pub surface_container_low: Color,
    /// A distinct area **within** the surface: menus, navigation bars.
    pub surface_container: Color,
    /// More emphasis than [`Self::surface_container`]: dialogs, search views.
    pub surface_container_high: Color,
    /// The **most** emphasis against the surface: filled cards, filled fields.
    pub surface_container_highest: Color,
    /// An inverted surface (toasts and snackbars that stand out from the background).
    pub inverse_surface: Color,
    pub on_inverse_surface: Color,
    /// Outlines at rest.
    pub outline: Color,
    /// Discreet outlines (thin separators).
    pub outline_variant: Color,
    pub error: Color,
    pub on_error: Color,
    /// The **quiet** form of `error`: a field's error surface, a warning that has to be
    /// read rather than shouted. `on_error_container` is what is legible on it — and it
    /// is what an errored field's border, label and helper take, not `error` itself
    /// (`input_decorator.dart:5981`).
    pub error_container: Color,
    pub on_error_container: Color,
    /// The accent as it must be drawn **on `inverse_surface`**: a snack bar's action
    /// (`snack_bar.dart:954`). `primary` on an inverted surface is the pair the scheme
    /// guarantees nothing about, which is exactly why this role exists.
    pub inverse_primary: Color,
    /// What a raised surface is tinted **towards** as it lifts — `primary` in Material 3.
    /// The elevation model there is a tint, not a shadow, and this is the colour it tints
    /// with (`bottom_app_bar.dart:301`).
    pub surface_tint: Color,
    /// The **darkest** surface in either theme, and the **lightest** — the two ends the
    /// container ladder runs between.
    pub surface_dim: Color,
    pub surface_bright: Color,
    /// The scrim for modals and drawers (the alpha is applied at the point of use).
    pub scrim: Color,
    /// The color of drop shadows (the alpha is applied at the point of use).
    pub shadow: Color,
}

impl ColorScheme {
    /// The dark scheme.
    pub fn dark() -> Self {
        Self {
            primary: Color::rgb8(96, 200, 130),
            on_primary: Color::rgb8(16, 28, 20),
            primary_container: Color::rgb8(30, 64, 44),
            on_primary_container: Color::rgb8(178, 240, 200),
            secondary: Color::rgb8(150, 170, 200),
            on_secondary: Color::rgb8(20, 26, 36),
            // Tone 30 of its own hue and chroma, where the reference puts a dark
            // secondary container and where the light scheme's already sat. This one was
            // at tone 22 — under the disabled fill it has to beat, so a slider's rail and
            // a selected segment read as unavailable. Only checkable once milestone 329
            // resolved that fill in sRGB; before, it was 14 tones adrift.
            secondary_container: Color::rgb8(63, 71, 88),
            on_secondary_container: Color::rgb8(205, 220, 240),
            // The tertiary family, a sixth of the wheel from the primary at the chroma
            // the reference's tonal-spot scheme uses (24), read off this crate's own HCT
            // rather than picked by eye.
            tertiary: Color::rgb8(162, 206, 217),
            on_tertiary: Color::rgb8(1, 54, 63),
            tertiary_container: Color::rgb8(32, 77, 86),
            on_tertiary_container: Color::rgb8(190, 234, 246),
            background: Color::rgb8(18, 20, 24),
            surface: Color::rgb8(30, 33, 40),
            on_surface: Color::rgb8(230, 232, 236),
            surface_variant: Color::rgb8(38, 42, 52),
            on_surface_variant: Color::rgb8(150, 156, 168),
            surface_container_lowest: Color::rgb8(26, 29, 35),
            surface_container_low: Color::rgb8(34, 37, 45),
            surface_container: Color::rgb8(36, 40, 48),
            surface_container_high: Color::rgb8(44, 48, 58),
            surface_container_highest: Color::rgb8(52, 56, 68),
            inverse_surface: Color::rgb8(226, 228, 234),
            on_inverse_surface: Color::rgb8(28, 32, 38),
            // Tones 60 and 30 of this palette's neutral-variant family, which is where
            // the reference puts them in a dark scheme. Anything darker collides with a
            // disabled outline — `on_surface` at 12 % over this surface is tone 24.
            outline: Color::rgb8(141, 145, 153),
            outline_variant: Color::rgb8(67, 71, 78),
            error: Color::rgb8(224, 108, 108),
            on_error: Color::rgb8(38, 12, 12),
            error_container: Color::rgb8(130, 37, 41),
            on_error_container: Color::rgb8(255, 218, 216),
            inverse_primary: Color::rgb8(2, 109, 56),
            surface_tint: Color::rgb8(96, 200, 130),
            surface_dim: Color::rgb8(30, 33, 40),
            surface_bright: Color::rgb8(57, 61, 74),
            scrim: Color::BLACK,
            shadow: Color::BLACK,
        }
    }

    /// The light scheme.
    pub fn light() -> Self {
        Self {
            primary: Color::rgb8(46, 160, 96),
            on_primary: Color::rgb8(255, 255, 255),
            primary_container: Color::rgb8(200, 238, 214),
            on_primary_container: Color::rgb8(10, 64, 36),
            secondary: Color::rgb8(90, 110, 150),
            on_secondary: Color::rgb8(255, 255, 255),
            secondary_container: Color::rgb8(220, 228, 244),
            on_secondary_container: Color::rgb8(30, 42, 66),
            tertiary: Color::rgb8(58, 100, 111),
            on_tertiary: Color::rgb8(255, 255, 255),
            tertiary_container: Color::rgb8(190, 234, 246),
            on_tertiary_container: Color::rgb8(1, 31, 38),
            background: Color::rgb8(245, 246, 248),
            surface: Color::rgb8(255, 255, 255),
            on_surface: Color::rgb8(28, 32, 38),
            surface_variant: Color::rgb8(238, 240, 244),
            on_surface_variant: Color::rgb8(110, 116, 126),
            surface_container_lowest: Color::rgb8(255, 255, 255),
            surface_container_low: Color::rgb8(250, 250, 252),
            surface_container: Color::rgb8(244, 245, 248),
            surface_container_high: Color::rgb8(238, 240, 244),
            surface_container_highest: Color::rgb8(232, 235, 240),
            inverse_surface: Color::rgb8(45, 50, 58),
            on_inverse_surface: Color::rgb8(240, 242, 246),
            // Tones 50 and 80, the reference's light-scheme positions. A disabled
            // outline here is tone 91, so both must sit well below it.
            outline: Color::rgb8(115, 119, 127),
            outline_variant: Color::rgb8(195, 198, 207),
            error: Color::rgb8(200, 64, 64),
            on_error: Color::rgb8(255, 255, 255),
            error_container: Color::rgb8(255, 218, 215),
            on_error_container: Color::rgb8(65, 0, 5),
            inverse_primary: Color::rgb8(111, 220, 149),
            surface_tint: Color::rgb8(46, 160, 96),
            surface_dim: Color::rgb8(223, 228, 234),
            surface_bright: Color::rgb8(255, 255, 255),
            scrim: Color::BLACK,
            shadow: Color::BLACK,
        }
    }

    /// Generates a complete scheme from **a seed color** (Material 3 "dynamic
    /// color", through [HCT](frus_core::Hct)). The seed's hue feeds five tonal
    /// palettes (primary, secondary, tertiary — not exposed for now — and the
    /// neutrals); each role is a precise **tone** of its palette, which guarantees
    /// the contrast of the `X`/`on_X` pairs.
    ///
    /// Deliberate departures from the M3 spec: `surface` sits slightly apart from
    /// `background` (our cards lay a surface over the background, tones 12/6 in
    /// dark, 100/98 in light) — the 2023 spec conflates them.
    ///
    /// The **container ladder** is anchored on *that* surface rather than on the
    /// spec's. Its five rungs keep the reference's own tonal steps — going toward
    /// more emphasis, −4, −2, −2, −2 in light and +6, +2, +5, +5 in dark — measured
    /// from `surface_container`, so every rung stands off this scheme's surface by
    /// what it stands off the reference's. In light the top rung lands on tone 100,
    /// which is where this scheme's `surface` already is: the departure showing
    /// through.
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        use frus_core::{Hct, TonalPalette};

        let hct = Hct::from_color(seed);
        // M3 chromas: the primary keeps the seed's chroma (with a floor of 48), and
        // the other palettes are muted variations on the hue.
        let primary = TonalPalette::new(hct.hue, hct.chroma.max(48.0));
        let secondary = TonalPalette::new(hct.hue, 16.0);
        // A sixth of the wheel away, at the chroma the reference's tonal-spot scheme
        // gives a tertiary: far enough from the primary to read as a third thing, close
        // enough to belong to the same palette.
        let tertiary = TonalPalette::new(hct.hue + 60.0, 24.0);
        let neutral = TonalPalette::new(hct.hue, 4.0);
        let neutral_variant = TonalPalette::new(hct.hue, 8.0);
        let error = TonalPalette::new(25.0, 84.0);

        let p = |tone: f64| primary.tone(tone);
        let s = |tone: f64| secondary.tone(tone);
        let ter = |tone: f64| tertiary.tone(tone);
        let n = |tone: f64| neutral.tone(tone);
        let nv = |tone: f64| neutral_variant.tone(tone);
        let e = |tone: f64| error.tone(tone);

        if dark {
            Self {
                primary: p(80.0),
                on_primary: p(20.0),
                primary_container: p(30.0),
                on_primary_container: p(90.0),
                secondary: s(80.0),
                on_secondary: s(20.0),
                secondary_container: s(30.0),
                on_secondary_container: s(90.0),
                tertiary: ter(80.0),
                on_tertiary: ter(20.0),
                tertiary_container: ter(30.0),
                on_tertiary_container: ter(90.0),
                background: n(6.0),
                surface: n(12.0),
                on_surface: n(90.0),
                surface_variant: nv(20.0),
                on_surface_variant: nv(80.0),
                surface_container_lowest: n(9.0),
                surface_container_low: n(15.0),
                surface_container: n(17.0),
                surface_container_high: n(22.0),
                surface_container_highest: n(27.0),
                inverse_surface: n(90.0),
                on_inverse_surface: n(20.0),
                outline: nv(60.0),
                outline_variant: nv(30.0),
                error: e(80.0),
                on_error: e(20.0),
                error_container: e(30.0),
                on_error_container: e(90.0),
                // The accent as it reads on the inverted surface: the *other* theme's
                // tone of the same palette, which is what an inverted surface is.
                inverse_primary: p(40.0),
                surface_tint: p(80.0),
                surface_dim: n(12.0),
                surface_bright: n(30.0),
                scrim: n(0.0),
                shadow: n(0.0),
            }
        } else {
            Self {
                primary: p(40.0),
                on_primary: p(100.0),
                primary_container: p(90.0),
                on_primary_container: p(10.0),
                secondary: s(40.0),
                on_secondary: s(100.0),
                secondary_container: s(90.0),
                on_secondary_container: s(10.0),
                tertiary: ter(40.0),
                on_tertiary: ter(100.0),
                tertiary_container: ter(90.0),
                on_tertiary_container: ter(10.0),
                background: n(98.0),
                surface: n(100.0),
                on_surface: n(10.0),
                surface_variant: nv(94.0),
                on_surface_variant: nv(30.0),
                surface_container_lowest: n(100.0),
                surface_container_low: n(98.0),
                surface_container: n(96.0),
                surface_container_high: n(94.0),
                surface_container_highest: n(92.0),
                inverse_surface: n(20.0),
                on_inverse_surface: n(95.0),
                outline: nv(50.0),
                outline_variant: nv(80.0),
                error: e(40.0),
                on_error: e(100.0),
                error_container: e(90.0),
                on_error_container: e(10.0),
                inverse_primary: p(80.0),
                surface_tint: p(40.0),
                surface_dim: n(89.0),
                surface_bright: n(100.0),
                scrim: n(0.0),
                shadow: n(0.0),
            }
        }
    }

    /// Interpolates role by role toward `other` (the light/dark switch fade).
    pub fn lerp(&self, other: &ColorScheme, t: f32) -> ColorScheme {
        let c = |a: Color, b: Color| a.lerp(b, t);
        ColorScheme {
            primary: c(self.primary, other.primary),
            on_primary: c(self.on_primary, other.on_primary),
            primary_container: c(self.primary_container, other.primary_container),
            on_primary_container: c(self.on_primary_container, other.on_primary_container),
            secondary: c(self.secondary, other.secondary),
            on_secondary: c(self.on_secondary, other.on_secondary),
            secondary_container: c(self.secondary_container, other.secondary_container),
            on_secondary_container: c(self.on_secondary_container, other.on_secondary_container),
            tertiary: c(self.tertiary, other.tertiary),
            on_tertiary: c(self.on_tertiary, other.on_tertiary),
            tertiary_container: c(self.tertiary_container, other.tertiary_container),
            on_tertiary_container: c(self.on_tertiary_container, other.on_tertiary_container),
            background: c(self.background, other.background),
            surface: c(self.surface, other.surface),
            on_surface: c(self.on_surface, other.on_surface),
            surface_variant: c(self.surface_variant, other.surface_variant),
            on_surface_variant: c(self.on_surface_variant, other.on_surface_variant),
            surface_container_lowest: c(
                self.surface_container_lowest,
                other.surface_container_lowest,
            ),
            surface_container_low: c(self.surface_container_low, other.surface_container_low),
            surface_container: c(self.surface_container, other.surface_container),
            surface_container_high: c(self.surface_container_high, other.surface_container_high),
            surface_container_highest: c(
                self.surface_container_highest,
                other.surface_container_highest,
            ),
            inverse_surface: c(self.inverse_surface, other.inverse_surface),
            on_inverse_surface: c(self.on_inverse_surface, other.on_inverse_surface),
            outline: c(self.outline, other.outline),
            outline_variant: c(self.outline_variant, other.outline_variant),
            error: c(self.error, other.error),
            on_error: c(self.on_error, other.on_error),
            error_container: c(self.error_container, other.error_container),
            on_error_container: c(self.on_error_container, other.on_error_container),
            inverse_primary: c(self.inverse_primary, other.inverse_primary),
            surface_tint: c(self.surface_tint, other.surface_tint),
            surface_dim: c(self.surface_dim, other.surface_dim),
            surface_bright: c(self.surface_bright, other.surface_bright),
            scrim: c(self.scrim, other.scrim),
            shadow: c(self.shadow, other.shadow),
        }
    }
}

/// **The smallest box a control that a finger works reserves for it**, in pixels
/// (`constants.dart:27`).
///
/// Not a look: a target. Forty-eight is the number the accessibility scanners on both
/// mobile platforms check for, and it is what the reference reserves by default for
/// every switch, checkbox, radio and icon button it draws — whatever those controls
/// actually paint inside it.
pub const MIN_TAP_TARGET: f32 = 48.0;

/// The same for a control told to reserve only what the specification requires: the
/// minimum less eight (`checkbox.dart:522`, `switch.dart:2090`).
pub const SHRUNK_TAP_TARGET: f32 = 40.0;

/// **How much room a small control reserves for the finger that works it.**
///
/// A switch paints a track 32 pixels tall; a checkbox paints a box of 20. Neither is
/// something a finger can be asked to hit. The reference lays both out inside a
/// 48-pixel square and paints the small thing in the middle of it, and it makes that a
/// theme-wide setting with a per-widget override, because it is the kind of decision an
/// application makes once (`theme_data.dart:172`).
///
/// This is a **layout** answer, not a visual one. What the control paints does not
/// change; the room around it does, and so does the area a click may land in.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TapTarget {
    /// At least [`MIN_TAP_TARGET`] on both sides — the default, as it is over there.
    #[default]
    Padded,
    /// Only [`SHRUNK_TAP_TARGET`]: for the dense interface that has measured its own
    /// reach and decided, which is a decision rather than an oversight.
    ShrinkWrap,
}

impl TapTarget {
    /// The smallest side, in pixels, this answer reserves.
    pub fn min_side(self) -> f32 {
        match self {
            TapTarget::Padded => MIN_TAP_TARGET,
            TapTarget::ShrinkWrap => SHRUNK_TAP_TARGET,
        }
    }
}

/// A set of style tokens.
///
/// The [`ColorScheme`] (`theme.scheme`) is the **source of truth** for the colors;
/// the "flat" fields (`background`, `surface`, `primary`, …) are **convenience
/// views** of the most used roles, derived from the scheme — the widgets'
/// historical API stays intact. `focus`/`selection` are interaction accents
/// specific to frus (outside the M3 roles).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// The color roles (the source of truth).
    pub scheme: ColorScheme,
    /// The application's background (= `scheme.background`).
    pub background: Color,
    /// The surfaces' background (= `scheme.surface`).
    pub surface: Color,
    /// The accent color (= `scheme.primary`).
    pub primary: Color,
    /// Text and content on `primary` (= `scheme.on_primary`).
    pub on_primary: Color,
    /// The default text on surfaces (= `scheme.on_surface`).
    pub on_surface: Color,
    /// Secondary text and discreet elements (= `scheme.on_surface_variant`).
    pub muted: Color,
    /// Borders at rest (= `scheme.outline`).
    pub border: Color,
    /// The focus accent (a frus interaction accent, outside the scheme).
    pub focus: Color,
    /// The text selection highlight (likewise).
    pub selection: Color,
    /// The tonal accent container (= `scheme.primary_container`).
    pub primary_container: Color,
    /// Content on `primary_container` (= `scheme.on_primary_container`).
    pub on_primary_container: Color,
    /// The error or danger color (= `scheme.error`).
    pub error: Color,
    /// Content on `error` (= `scheme.on_error`).
    pub on_error: Color,
    /// The discreet outline variant (= `scheme.outline_variant`).
    pub outline_variant: Color,
    /// The named typographic scale (Material's 15 steps).
    pub text: TextTheme,
    /// The default corner radius.
    pub radius: f32,
    /// The base spacing unit.
    pub spacing: f32,
    /// The ambient **reading and layout direction** (LTR by default). In RTL, the
    /// driver mirrors the layout horizontally. Carried here (an ambient context
    /// threaded down to paint) pending a dedicated `Env` (§2).
    pub direction: TextDirection,
    /// **Per-widget defaults**: what a widget looks like when the caller has not said,
    /// resolved as `caller ?? theme ?? framework`. Empty by default — a theme that sets
    /// nothing behaves exactly as if there were none. See
    /// [`WidgetThemes`](crate::WidgetThemes).
    pub widgets: crate::widgettheme::WidgetThemes,
    /// **How much room the small controls reserve for a finger** ([`TapTarget`]).
    /// [`Padded`](TapTarget::Padded) by default, as it is over there: a switch, a
    /// checkbox, a radio and an icon button each lay out inside at least
    /// [`MIN_TAP_TARGET`] and paint what they paint in the middle of it.
    pub tap_target: TapTarget,
}

impl Theme {
    /// Builds a theme from a scheme: the flat fields are **derived** from the roles,
    /// so there is a single source of truth.
    pub fn from_scheme(scheme: ColorScheme, focus: Color, selection: Color) -> Self {
        Self {
            scheme,
            background: scheme.background,
            surface: scheme.surface,
            primary: scheme.primary,
            on_primary: scheme.on_primary,
            on_surface: scheme.on_surface,
            muted: scheme.on_surface_variant,
            border: scheme.outline,
            focus,
            selection,
            primary_container: scheme.primary_container,
            on_primary_container: scheme.on_primary_container,
            error: scheme.error,
            on_error: scheme.on_error,
            outline_variant: scheme.outline_variant,
            text: TextTheme::default(),
            radius: 10.0,
            spacing: 8.0,
            direction: TextDirection::Ltr,
            widgets: crate::widgettheme::WidgetThemes::default(),
            tap_target: TapTarget::default(),
        }
    }

    /// The same theme in **right-to-left** (Arabic, Hebrew…).
    pub fn rtl(mut self) -> Self {
        self.direction = TextDirection::Rtl;
        self
    }

    /// The dark theme.
    pub fn dark() -> Self {
        Self::from_scheme(
            ColorScheme::dark(),
            Color::rgb8(90, 158, 242),
            Color::rgba(0.35, 0.62, 0.95, 0.40),
        )
    }

    /// The light theme.
    pub fn light() -> Self {
        Self::from_scheme(
            ColorScheme::light(),
            Color::rgb8(40, 120, 220),
            Color::rgba(0.20, 0.50, 0.90, 0.30),
        )
    }

    /// A theme generated from a **seed color** (see [`ColorScheme::from_seed`]). The
    /// focus ring and the selection derive from the scheme's primary (interaction
    /// roles specific to frus, outside the M3 scheme).
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        let scheme = ColorScheme::from_seed(seed, dark);
        let focus = scheme.primary;
        let selection = scheme.primary.with_alpha(if dark { 0.40 } else { 0.30 });
        Self::from_scheme(scheme, focus, selection)
    }

    /// Applies the Material **state layer** over `base`: it overlays the content
    /// color `on` at low opacity according to the interaction state — hover 8%,
    /// focus 10%, press 12% — taking the animated progressions into account
    /// (`hover_progress`/`focus_progress`/`press_progress`). This is the state rule
    /// **baked** into the theme: widgets stay declarative (they pass their base color and
    /// their content color, and the theme decides on the overlay).
    ///
    /// All three terms are **progressions**, the press included. It read the flag until
    /// milestone 441, which meant that term could only ever be 0 or 12%: the layer
    /// arrived whole under a finger and vanished whole when it left. Reading
    /// `press_progress` is what lets it fade, and reading the flag *as well* would defeat
    /// that — the term would reach full on the first frame and the fade would never run.
    pub fn state_layer(&self, base: Color, on: Color, status: &Status) -> Color {
        let overlay = 0.08 * status.hover_progress.clamp(0.0, 1.0)
            + 0.10 * status.focus_progress.clamp(0.0, 1.0)
            + 0.12 * status.press_progress.clamp(0.0, 1.0);
        base.lerp(on, overlay.min(1.0))
    }
}

impl Theme {
    /// Interpolates toward `other` at progress `t` (`0` = `self`, `1` = `other`).
    /// Used for the theme fade when switching light and dark. The **scheme** is
    /// interpolated role by role and the flat fields are re-derived from it (a
    /// single source of truth, even mid-fade).
    pub fn lerp(&self, other: &Theme, t: f32) -> Theme {
        let t = t.clamp(0.0, 1.0);
        let f = |a: f32, b: f32| a + (b - a) * t;
        let mut out = Theme::from_scheme(
            self.scheme.lerp(&other.scheme, t),
            self.focus.lerp(other.focus, t),
            self.selection.lerp(other.selection, t),
        );
        // Typography takes no part in the fade (it is identical light and dark).
        out.text = self.text;
        out.radius = f(self.radius, other.radius);
        out.spacing = f(self.spacing, other.spacing);
        // Direction is discrete: keep the fade target's.
        out.direction = other.direction;
        // So is the tap target — and so are the per-widget defaults, which the fade used
        // to **drop**: `from_scheme` starts from an empty set and nothing put them back,
        // so every override an application had written disappeared for the length of a
        // light/dark crossing and came back when it ended.
        out.tap_target = other.tap_target;
        out.widgets = other.widgets.clone();
        out
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    /// **A theme is eight kilobytes, and it used to be `Copy`.** Every `*theme` in the
    /// crate was a silent eight-kilobyte memcpy — in the layout walk, in every themed
    /// subtree, once per overlay. Milestone 448 dropped `Copy`, which turned nine of
    /// those into `clone()` calls that say what they cost.
    ///
    /// The number itself is not a promise; the assertion is that it is **large**, since
    /// that is the whole argument for the change.
    #[test]
    fn a_theme_is_far_too_big_to_copy_by_accident() {
        assert!(
            std::mem::size_of::<super::Theme>() > 4096,
            "a theme is {} bytes",
            std::mem::size_of::<super::Theme>()
        );
    }

    use super::*;
    use crate::disabled::DISABLED_CONTAINER_OPACITY;

    /// The WCAG contrast between two colors (a ratio ≥ 1).
    fn contrast(a: Color, b: Color) -> f32 {
        let (la, lb) = (a.compute_luminance() + 0.05, b.compute_luminance() + 0.05);
        if la > lb {
            la / lb
        } else {
            lb / la
        }
    }

    #[test]
    fn from_seed_generates_contrasting_pairs() {
        // Every X / on_X pair must stay legible (≥ 4.5:1, the AA requirement), for
        // any seed at all — even a barely chromatic one.
        for seed in [
            Color::rgb8(0x42, 0x85, 0xF4), // Google blue
            Color::rgb8(0x9C, 0x27, 0xB0), // violet
            Color::rgb8(0x80, 0x80, 0x80), // grey — very nearly no chroma at all
        ] {
            for dark in [false, true] {
                let s = ColorScheme::from_seed(seed, dark);
                for (name, base, on) in [
                    ("primary", s.primary, s.on_primary),
                    ("secondary", s.secondary, s.on_secondary),
                    ("surface", s.surface, s.on_surface),
                    ("tertiary", s.tertiary, s.on_tertiary),
                    ("error", s.error, s.on_error),
                    ("inverse", s.inverse_surface, s.on_inverse_surface),
                    // The containers carry text too — an errored field's helper line is
                    // `on_error_container` on `error_container`, and it has to be read.
                    (
                        "primary_container",
                        s.primary_container,
                        s.on_primary_container,
                    ),
                    (
                        "secondary_container",
                        s.secondary_container,
                        s.on_secondary_container,
                    ),
                    (
                        "tertiary_container",
                        s.tertiary_container,
                        s.on_tertiary_container,
                    ),
                    ("error_container", s.error_container, s.on_error_container),
                ] {
                    let ratio = contrast(base, on);
                    assert!(
                        ratio >= 4.5,
                        "{name} does not contrast enough ({ratio:.2}) — seed {seed:?}, \
                         dark={dark}"
                    );
                }
            }
        }
    }

    #[test]
    fn from_seed_light_and_dark_share_the_hue() {
        // Both modes decline the same hue (the dark primary is the tone-80 version
        // of the tone-40 light primary).
        let seed = Color::rgb8(0x42, 0x85, 0xF4);
        let light = ColorScheme::from_seed(seed, false);
        let dark = ColorScheme::from_seed(seed, true);
        let hue = |c: Color| frus_core::Hct::from_color(c).hue;
        let delta = (hue(light.primary) - hue(dark.primary)).abs();
        let delta = delta.min(360.0 - delta);
        assert!(
            delta < 12.0,
            "teintes clair/sombre divergentes ({delta:.1}°)"
        );
        // And the dark one is indeed… dark.
        assert!(dark.background.compute_luminance() < 0.1);
        assert!(light.background.compute_luminance() > 0.85);
    }

    #[test]
    fn an_outline_is_never_the_colour_of_a_disabled_one() {
        // Since milestone 322 a disabled control's outline is `on_surface` at 12 % over
        // the surface. If a scheme puts a *live* outline at the same tone, then every
        // outlined control in the framework looks available whether it is or not — which
        // is what a device found on the dark palette. This is the guard against it.
        //
        // The bar is the reference's own margin at its narrowest: its baseline schemes
        // separate `outline_variant` from a disabled outline by about ten tones, and
        // `outline` by nearly forty.
        let tone = |c: Color| frus_core::Hct::from_color(c).tone;
        let mut checked = 0;
        for (name, s) in [
            ("dark", ColorScheme::dark()),
            ("light", ColorScheme::light()),
            (
                "seeded dark",
                ColorScheme::from_seed(Color::rgb8(0x42, 0x85, 0xF4), true),
            ),
            (
                "seeded light",
                ColorScheme::from_seed(Color::rgb8(0x42, 0x85, 0xF4), false),
            ),
            (
                "grey seeded dark",
                ColorScheme::from_seed(Color::rgb8(0x80, 0x80, 0x80), true),
            ),
        ] {
            // Opaque over opaque: the lerp is the composite `fade(0.12)` resolves to.
            let disabled = tone(s.surface.lerp(s.on_surface, DISABLED_CONTAINER_OPACITY));
            for (role, colour, floor) in [
                ("outline", s.outline, 24.0),
                ("outline_variant", s.outline_variant, 6.0),
            ] {
                let gap = (tone(colour) - disabled).abs();
                assert!(
                    gap >= floor,
                    "{name}: {role} is {gap:.1} tones from a disabled outline \
                     (needs {floor}) — a live control and an unavailable one \
                     would be drawn the same"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 10);
    }

    /// The same question for a control that is **filled** rather than outlined: it must
    /// be tellable from the disabled fill, or the filled half of the disabled rule
    /// inverts. A slider's rail was fainter than its own disabled twin in both schemes,
    /// which is what sent us looking.
    ///
    /// Two ways to be tellable, and a fill needs one of them. **Tone**, measured as a
    /// distance from the surface rather than from each other — what the eye reads is how
    /// far a fill sits from what it lies on, and the disabled fill is by construction a
    /// fixed step from it. Or **chroma**: the disabled fill is `on_surface` at 12 %, so
    /// it is nearly grey, and a fill carrying real colour is told apart from it at any
    /// tone. `primary_container` in the dark scheme is tone 24 against a disabled tone 24
    /// and nobody would confuse them, because one is green.
    ///
    /// What the rail had was neither: a near-neutral of the surface's own hue, sitting
    /// *closer* to the surface than the disabled fill.
    ///
    /// This guard could not be written before milestone 329. It models the disabled fill
    /// as an sRGB blend, which was a fiction while the GPU blended the token in linear
    /// light — 14 tones adrift in the dark scheme. Now [`crate::disabled::over_surface`]
    /// resolves exactly this arithmetic, so the model is the painted truth.
    #[test]
    fn a_live_container_is_never_quieter_than_a_disabled_fill() {
        let tone = |c: Color| frus_core::Hct::from_color(c).tone;
        let chroma = |c: Color| frus_core::Hct::from_color(c).chroma;
        /// A fill this much more colourful than the disabled one is told apart by its
        /// colour whatever its tone. Set where a *saturated* container sits — the two
        /// `primary_container`s clear it by a third — and well above the rail that
        /// failed, which was only 8 more colourful and would have reached for this
        /// escape without deserving it.
        const CHROMA_MARGIN: f64 = 15.0;
        let mut checked = 0;
        for (name, s) in [
            ("dark", ColorScheme::dark()),
            ("light", ColorScheme::light()),
            (
                "seeded dark",
                ColorScheme::from_seed(Color::rgb8(0x42, 0x85, 0xF4), true),
            ),
            (
                "seeded light",
                ColorScheme::from_seed(Color::rgb8(0x42, 0x85, 0xF4), false),
            ),
            (
                "grey seeded dark",
                ColorScheme::from_seed(Color::rgb8(0x80, 0x80, 0x80), true),
            ),
        ] {
            let surface = tone(s.surface);
            let fill = s.surface.lerp(s.on_surface, DISABLED_CONTAINER_OPACITY);
            let dead = (tone(fill) - surface).abs();
            // The roles a live control is filled with. `secondary_container` carries a
            // slider's rail, a selected chip, a selected segment and a tonal button;
            // `primary_container` the louder selections.
            for (role, colour) in [
                ("secondary_container", s.secondary_container),
                ("primary_container", s.primary_container),
            ] {
                let live = (tone(colour) - surface).abs();
                let colourful = chroma(colour) - chroma(fill) >= CHROMA_MARGIN;
                assert!(
                    live >= dead || colourful,
                    "{name}: {role} is {live:.1} tones off the surface where a disabled \
                     fill is {dead:.1}, and only {:.1} more colourful — a live control \
                     would read as an unavailable one",
                    chroma(colour) - chroma(fill)
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 10);
    }

    /// The five container rungs are a **ladder**: each stands further from the
    /// surface than the one below it, in whichever direction the scheme's own
    /// brightness sends them. A rung out of order — or two on the same tone — is two
    /// widgets that cannot be told apart while their roles say they should be.
    #[test]
    fn the_container_ladder_climbs_in_one_direction() {
        let tone = |c: Color| frus_core::Hct::from_color(c).tone;
        /// Two rungs closer than this read as the same colour.
        const RUNG: f64 = 1.0;
        let mut checked = 0;
        for (name, s) in [
            ("dark", ColorScheme::dark()),
            ("light", ColorScheme::light()),
            (
                "seeded dark",
                ColorScheme::from_seed(Color::rgb8(0x42, 0x85, 0xF4), true),
            ),
            (
                "seeded light",
                ColorScheme::from_seed(Color::rgb8(0x42, 0x85, 0xF4), false),
            ),
        ] {
            // A dark scheme's containers grow lighter as they take emphasis, a light
            // scheme's darker.
            let up = if tone(s.surface) < 50.0 { 1.0 } else { -1.0 };
            let rungs = [
                ("lowest", s.surface_container_lowest),
                ("low", s.surface_container_low),
                ("container", s.surface_container),
                ("high", s.surface_container_high),
                ("highest", s.surface_container_highest),
            ];
            // `surface_dim` and `surface_bright` bracket the **surface**, in either
            // theme — "always the darkest" and "always the lightest"
            // (`color_scheme.dart:1236`, `:1241`).
            //
            // They do not bracket the *containers*, and the first draft of this test
            // asserted that they did. The reference's own dark scheme puts
            // `surfaceContainerLowest` at tone 4 and `surfaceDim` at 6, so the ladder's
            // bottom rung is darker than the darkest surface: the two are separate
            // families, and dim/bright are a claim about the surface alone.
            assert!(
                tone(s.surface_dim) <= tone(s.surface) + 1e-6,
                "surface_dim is not the darker end"
            );
            assert!(
                tone(s.surface) <= tone(s.surface_bright) + 1e-6,
                "surface_bright is not the lighter one"
            );
            checked += 2;
            for pair in rungs.windows(2) {
                let ((below, b), (above, a)) = (pair[0], pair[1]);
                let step = (tone(a) - tone(b)) * up;
                assert!(
                    step >= RUNG,
                    "{name}: the {above} rung is {step:.1} tones of emphasis above the \
                     {below} one — a ladder cannot go back down"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 24);
    }

    #[test]
    fn dark_and_light_differ() {
        assert_ne!(Theme::dark().background, Theme::light().background);
        assert_ne!(Theme::dark().on_surface, Theme::light().on_surface);
    }

    #[test]
    fn flat_fields_mirror_the_scheme() {
        // The flat fields are views derived from the scheme — including mid-fade
        // (the lerp goes through the scheme).
        for theme in [
            Theme::dark(),
            Theme::light(),
            Theme::dark().lerp(&Theme::light(), 0.37),
        ] {
            assert_eq!(theme.background, theme.scheme.background);
            assert_eq!(theme.surface, theme.scheme.surface);
            assert_eq!(theme.primary, theme.scheme.primary);
            assert_eq!(theme.on_primary, theme.scheme.on_primary);
            assert_eq!(theme.on_surface, theme.scheme.on_surface);
            assert_eq!(theme.muted, theme.scheme.on_surface_variant);
            assert_eq!(theme.border, theme.scheme.outline);
            assert_eq!(theme.primary_container, theme.scheme.primary_container);
            assert_eq!(theme.error, theme.scheme.error);
            assert_eq!(theme.outline_variant, theme.scheme.outline_variant);
        }
    }

    #[test]
    fn state_layer_darkens_toward_content_on_interaction() {
        let theme = Theme::dark();
        let base = Color::rgb(0.4, 0.4, 0.4);
        let on = Color::BLACK;

        // At rest: no overlay.
        let idle = Status::default();
        assert_eq!(theme.state_layer(base, on, &idle), base);

        // Fully hovered: the base is pulled 8% toward `on` (darker here).
        let hovered = Status {
            hover_progress: 1.0,
            ..Default::default()
        };
        let h = theme.state_layer(base, on, &hovered);
        assert!(h.r < base.r && (base.r - h.r - 0.4 * 0.08).abs() < 1e-4);

        // Pressed: a stronger overlay than hover alone.
        let pressed = Status {
            press_progress: 1.0,
            ..Default::default()
        };
        assert!(theme.state_layer(base, on, &pressed).r < h.r);
    }

    /// **A press lights by degrees** (milestone 441). The term read
    /// `Interaction::Pressed`, a flag, so it could only ever be 0 or 12 %: the layer
    /// arrived whole under a finger and left whole with it.
    #[test]
    fn a_press_lights_by_degrees() {
        let theme = Theme::dark();
        let base = Color::rgb(0.4, 0.4, 0.4);
        let on = Color::BLACK;
        let at = |p: f32| {
            theme.state_layer(
                base,
                on,
                &Status {
                    press_progress: p,
                    ..Default::default()
                },
            )
        };
        let (rest, half, full) = (at(0.0), at(0.5), at(1.0));
        assert_eq!(rest, base, "nothing at rest");
        assert!(full.r < half.r && half.r < rest.r, "half way is half way");
        assert!(
            (half.r - (rest.r + full.r) * 0.5).abs() < 1e-4,
            "and it is the overlay that is halved, not the colour"
        );

        // And the flag alone no longer lights it. If it were read as well the term would
        // reach full on the very first frame, and the fade above could never run.
        let flagged = Status {
            interaction: crate::interaction::Interaction::Pressed,
            ..Default::default()
        };
        assert_eq!(theme.state_layer(base, on, &flagged), base);
    }

    /// **A tap target is a theme-wide answer with a per-widget override** (milestone
    /// 442), and the default is the reference's: at least 48 pixels for anything a
    /// finger works.
    #[test]
    fn a_theme_reserves_a_tap_target_by_default() {
        assert_eq!(Theme::dark().tap_target, TapTarget::Padded);
        assert_eq!(Theme::light().tap_target, TapTarget::Padded);
        // Read through the enum, which is what a widget asking would get.
        let sides: Vec<f32> = [TapTarget::Padded, TapTarget::ShrinkWrap]
            .iter()
            .map(|t| t.min_side())
            .collect();
        assert_eq!(sides, vec![MIN_TAP_TARGET, SHRUNK_TAP_TARGET]);
        assert!(
            sides[1] < sides[0],
            "shrink-wrapping is a smaller answer, not a different one"
        );
    }

    /// **A fade used to drop every per-widget default it crossed** (milestone 442).
    ///
    /// `lerp` rebuilds the theme from the interpolated scheme, and `from_scheme` starts
    /// from an empty set of widget defaults — so every override an application had
    /// written disappeared for the length of a light/dark crossing and came back when it
    /// ended. Discrete, like the direction beside it: a corner radius is not a colour and
    /// has no half-way.
    #[test]
    fn a_fade_keeps_what_it_cannot_interpolate() {
        let mut target = Theme::light();
        target.widgets.checkbox.radius = Some(3.0);
        target.tap_target = TapTarget::ShrinkWrap;

        let mid = Theme::dark().lerp(&target, 0.5);
        assert_eq!(
            mid.widgets.checkbox.radius,
            Some(3.0),
            "the defaults survive"
        );
        assert_eq!(mid.tap_target, TapTarget::ShrinkWrap);
        // And the colours are still half way across, which is what the fade is for.
        assert_ne!(mid.background, Theme::dark().background);
        assert_ne!(mid.background, target.background);
    }

    #[test]
    fn lerp_hits_endpoints() {
        let d = Theme::dark();
        let l = Theme::light();
        assert_eq!(d.lerp(&l, 0.0).background, d.background);
        assert_eq!(d.lerp(&l, 1.0).background, l.background);
        // In the middle: neither one nor the other.
        let mid = d.lerp(&l, 0.5).background;
        assert_ne!(mid, d.background);
        assert_ne!(mid, l.background);
    }
}
