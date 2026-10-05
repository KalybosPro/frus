//! The colours of Apple's platforms, and the colours that **adapt**: one name, up to eight
//! values, picked by the brightness, the contrast and the level of the interface it is used
//! in (milestone 627).

use frus_core::Color;

use crate::media::{Brightness, MediaQuery};
use crate::Theme;

/// **How high a piece of interface sits**: on the window's own surface, or raised above it —
/// a sheet, a dialog, a popover.
///
/// An adaptive colour has a variant for each, and in a dark interface they differ: the
/// background of a raised sheet is a little lighter than the page under it, which is how a
/// dark interface shows depth without shadows. In a light one they are nearly all the same.
///
/// A subtree says which it is with [`around`](Self::around); the theme carries the answer
/// ([`Theme::user_interface_level`]), and [`CupertinoDynamicColor::resolve`] reads it there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CupertinoUserInterfaceLevelData {
    /// The window's own surface. What everything is, until something says otherwise.
    #[default]
    Base,
    /// Raised above it: a sheet, a dialog, a popover.
    Elevated,
}

impl CupertinoUserInterfaceLevelData {
    /// Wraps `child` in this level: the adaptive colours inside it resolve to this level's
    /// variants.
    pub fn around<Msg: 'static>(
        self,
        child: impl crate::Widget<Msg> + 'static,
    ) -> crate::Themed<Msg> {
        crate::Themed::tweak(move |t| t.user_interface_level = self, child)
    }

    /// The level `theme` says its subtree is at.
    pub fn of(theme: &Theme) -> Self {
        theme.user_interface_level
    }
}

/// A colour that **adapts**: up to eight values for one name, one for each combination of a
/// light or dark interface, normal or high contrast, and the base or the
/// [elevated](CupertinoUserInterfaceLevelData::Elevated) level.
///
/// [`resolve`](Self::resolve) picks the one that applies where it is used;
/// [`resolve_with`](Self::resolve_with) picks it for conditions given outright. Used as a
/// plain [`Color`] without being resolved (`Color::from`), it is its light, normal-contrast,
/// base value, as the reference's is.
///
/// Most colours vary along fewer than three of the axes, and two constructors say so:
/// [`with_brightness`](Self::with_brightness) and
/// [`with_brightness_and_contrast`](Self::with_brightness_and_contrast). The palette is
/// [`CupertinoColors`].
///
/// ```
/// use frus_widgets::{Brightness, Color, CupertinoDynamicColor, CupertinoUserInterfaceLevelData};
///
/// let ink = CupertinoDynamicColor::with_brightness(Color::rgb8(20, 20, 20), Color::rgb8(240, 240, 240));
/// let dark = ink.resolve_with(Brightness::Dark, CupertinoUserInterfaceLevelData::Base, false);
/// assert_eq!(dark, Color::rgb8(240, 240, 240));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct CupertinoDynamicColor {
    /// Light, normal contrast, base level.
    pub color: Color,
    /// Dark, normal contrast, base level.
    pub dark_color: Color,
    /// Light, high contrast, base level.
    pub high_contrast_color: Color,
    /// Dark, high contrast, base level.
    pub dark_high_contrast_color: Color,
    /// Light, normal contrast, elevated.
    pub elevated_color: Color,
    /// Dark, normal contrast, elevated.
    pub dark_elevated_color: Color,
    /// Light, high contrast, elevated.
    pub high_contrast_elevated_color: Color,
    /// Dark, high contrast, elevated.
    pub dark_high_contrast_elevated_color: Color,
    /// A name to tell it by when it is printed; it takes no part in equality.
    pub debug_label: Option<&'static str>,
}

impl CupertinoDynamicColor {
    /// A colour that varies along all three axes: all eight values, given outright.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        color: Color,
        dark_color: Color,
        high_contrast_color: Color,
        dark_high_contrast_color: Color,
        elevated_color: Color,
        dark_elevated_color: Color,
        high_contrast_elevated_color: Color,
        dark_high_contrast_elevated_color: Color,
    ) -> Self {
        Self {
            color,
            dark_color,
            high_contrast_color,
            dark_high_contrast_color,
            elevated_color,
            dark_elevated_color,
            high_contrast_elevated_color,
            dark_high_contrast_elevated_color,
            debug_label: None,
        }
    }

    /// A colour that varies with the brightness and the contrast, and is the same at either
    /// level.
    pub const fn with_brightness_and_contrast(
        color: Color,
        dark_color: Color,
        high_contrast_color: Color,
        dark_high_contrast_color: Color,
    ) -> Self {
        Self::new(
            color,
            dark_color,
            high_contrast_color,
            dark_high_contrast_color,
            color,
            dark_color,
            high_contrast_color,
            dark_high_contrast_color,
        )
    }

    /// A colour that varies with the brightness alone.
    pub const fn with_brightness(color: Color, dark_color: Color) -> Self {
        Self::new(
            color, dark_color, color, dark_color, color, dark_color, color, dark_color,
        )
    }

    /// The same colour, printed as `label`.
    #[must_use]
    pub const fn labelled(mut self, label: &'static str) -> Self {
        self.debug_label = Some(label);
        self
    }

    /// **The value for these conditions**: a `brightness`, a `level`, and whether the
    /// reader has asked for `high_contrast`.
    pub fn resolve_with(
        &self,
        brightness: Brightness,
        level: CupertinoUserInterfaceLevelData,
        high_contrast: bool,
    ) -> Color {
        use CupertinoUserInterfaceLevelData::{Base, Elevated};
        match (brightness, level, high_contrast) {
            (Brightness::Light, Base, false) => self.color,
            (Brightness::Light, Base, true) => self.high_contrast_color,
            (Brightness::Light, Elevated, false) => self.elevated_color,
            (Brightness::Light, Elevated, true) => self.high_contrast_elevated_color,
            (Brightness::Dark, Base, false) => self.dark_color,
            (Brightness::Dark, Base, true) => self.dark_high_contrast_color,
            (Brightness::Dark, Elevated, false) => self.dark_elevated_color,
            (Brightness::Dark, Elevated, true) => self.dark_high_contrast_elevated_color,
        }
    }

    /// **The value where it is used**: in `theme`'s brightness, at the level `theme` says its
    /// subtree is at, and in high contrast if the reader has asked for it
    /// ([`Accessibility::high_contrast`](crate::Accessibility::high_contrast), read from the
    /// ambient [`MediaQuery`]).
    ///
    /// The theme's brightness, not the system's: an application that keeps a light theme on
    /// a dark system gets the light values, as the reference's adaptive colours under its
    /// main theme do.
    pub fn resolve(&self, theme: &Theme) -> Color {
        self.resolve_with(
            theme.brightness(),
            theme.user_interface_level,
            MediaQuery::of().accessibility.high_contrast,
        )
    }
}

/// Equal when all eight values are: the label is only a name.
impl PartialEq for CupertinoDynamicColor {
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color
            && self.dark_color == other.dark_color
            && self.high_contrast_color == other.high_contrast_color
            && self.dark_high_contrast_color == other.dark_high_contrast_color
            && self.elevated_color == other.elevated_color
            && self.dark_elevated_color == other.dark_elevated_color
            && self.high_contrast_elevated_color == other.high_contrast_elevated_color
            && self.dark_high_contrast_elevated_color == other.dark_high_contrast_elevated_color
    }
}

/// An adaptive colour used without being resolved: its light, normal-contrast, base value.
impl From<CupertinoDynamicColor> for Color {
    fn from(c: CupertinoDynamicColor) -> Color {
        c.color
    }
}

/// **The palette of Apple's platforms**: their system colours, the colours of text, fills,
/// backgrounds and separators, and a few plain ones — the reference's table, value for value.
///
/// Most are [adaptive](CupertinoDynamicColor), and are resolved where they are used:
///
/// ```
/// use frus_widgets::{CupertinoColors, Theme};
///
/// let theme = Theme::dark();
/// let background = CupertinoColors::SYSTEM_BACKGROUND.resolve(&theme);
/// assert_eq!(background, CupertinoColors::BLACK);
/// ```
pub struct CupertinoColors;

impl CupertinoColors {
    /// The blue of active elements — a button's label, a selected tab. The same as [`SYSTEM_BLUE`](Self::SYSTEM_BLUE).
    pub const ACTIVE_BLUE: CupertinoDynamicColor = Self::SYSTEM_BLUE;

    /// The green of active accents — a switch that is on. The same as [`SYSTEM_GREEN`](Self::SYSTEM_GREEN).
    pub const ACTIVE_GREEN: CupertinoDynamicColor = Self::SYSTEM_GREEN;

    /// The orange of active accents. The same as [`SYSTEM_ORANGE`](Self::SYSTEM_ORANGE).
    pub const ACTIVE_ORANGE: CupertinoDynamicColor = Self::SYSTEM_ORANGE;

    /// Opaque white, for backgrounds, and for text against dark ones.
    pub const WHITE: Color = Color::from_argb_u32(0xFFFFFFFF);

    /// Opaque black, for text against light backgrounds.
    pub const BLACK: Color = Color::from_argb_u32(0xFF000000);

    /// Fully transparent: a colour that paints nothing.
    pub const TRANSPARENT: Color = Color::from_argb_u32(0x00000000);

    /// A light background fill, such as a chat bubble's.
    pub const LIGHT_BACKGROUND_GRAY: Color = Color::from_argb_u32(0xFFE5E5EA);

    /// A very light background fill, such as the gap between a table's groups of cells.
    pub const EXTRA_LIGHT_BACKGROUND_GRAY: Color = Color::from_argb_u32(0xFFEFEFF4);

    /// A very dark background fill, such as the gap between a dark table's groups of cells.
    pub const DARK_BACKGROUND_GRAY: Color = Color::from_argb_u32(0xFF171717);

    /// Controls that are not selected, such as a tab bar's other items.
    pub const INACTIVE_GRAY: CupertinoDynamicColor = CupertinoDynamicColor::with_brightness(
        Color::from_argb_u32(0xFF999999),
        Color::from_argb_u32(0xFF757575),
    )
    .labelled("inactiveGray");

    /// Destructive actions, such as a list's delete. The same as [`SYSTEM_RED`](Self::SYSTEM_RED).
    pub const DESTRUCTIVE_RED: CupertinoDynamicColor = Self::SYSTEM_RED;

    /// The system's blue.
    pub const SYSTEM_BLUE: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(0, 122, 255, 255),
            Color::rgba8(10, 132, 255, 255),
            Color::rgba8(0, 64, 221, 255),
            Color::rgba8(64, 156, 255, 255),
        )
        .labelled("systemBlue");

    /// The system's green.
    pub const SYSTEM_GREEN: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(52, 199, 89, 255),
            Color::rgba8(48, 209, 88, 255),
            Color::rgba8(36, 138, 61, 255),
            Color::rgba8(48, 219, 91, 255),
        )
        .labelled("systemGreen");

    /// The system's mint.
    pub const SYSTEM_MINT: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(0, 199, 190, 255),
            Color::rgba8(99, 230, 226, 255),
            Color::rgba8(12, 129, 123, 255),
            Color::rgba8(102, 212, 207, 255),
        )
        .labelled("systemMint");

    /// The system's indigo.
    pub const SYSTEM_INDIGO: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(88, 86, 214, 255),
            Color::rgba8(94, 92, 230, 255),
            Color::rgba8(54, 52, 163, 255),
            Color::rgba8(125, 122, 255, 255),
        )
        .labelled("systemIndigo");

    /// The system's orange.
    pub const SYSTEM_ORANGE: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(255, 149, 0, 255),
            Color::rgba8(255, 159, 10, 255),
            Color::rgba8(201, 52, 0, 255),
            Color::rgba8(255, 179, 64, 255),
        )
        .labelled("systemOrange");

    /// The system's pink.
    pub const SYSTEM_PINK: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(255, 45, 85, 255),
            Color::rgba8(255, 55, 95, 255),
            Color::rgba8(211, 15, 69, 255),
            Color::rgba8(255, 100, 130, 255),
        )
        .labelled("systemPink");

    /// The system's brown.
    pub const SYSTEM_BROWN: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(162, 132, 94, 255),
            Color::rgba8(172, 142, 104, 255),
            Color::rgba8(127, 101, 69, 255),
            Color::rgba8(181, 148, 105, 255),
        )
        .labelled("systemBrown");

    /// The system's purple.
    pub const SYSTEM_PURPLE: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(175, 82, 222, 255),
            Color::rgba8(191, 90, 242, 255),
            Color::rgba8(137, 68, 171, 255),
            Color::rgba8(218, 143, 255, 255),
        )
        .labelled("systemPurple");

    /// The system's red.
    pub const SYSTEM_RED: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(255, 59, 48, 255),
            Color::rgba8(255, 69, 58, 255),
            Color::rgba8(215, 0, 21, 255),
            Color::rgba8(255, 105, 97, 255),
        )
        .labelled("systemRed");

    /// The system's teal.
    pub const SYSTEM_TEAL: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(90, 200, 250, 255),
            Color::rgba8(100, 210, 255, 255),
            Color::rgba8(0, 113, 164, 255),
            Color::rgba8(112, 215, 255, 255),
        )
        .labelled("systemTeal");

    /// The system's cyan.
    pub const SYSTEM_CYAN: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(50, 173, 230, 255),
            Color::rgba8(100, 210, 255, 255),
            Color::rgba8(0, 113, 164, 255),
            Color::rgba8(112, 215, 255, 255),
        )
        .labelled("systemCyan");

    /// The system's yellow.
    pub const SYSTEM_YELLOW: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(255, 204, 0, 255),
            Color::rgba8(255, 214, 10, 255),
            Color::rgba8(160, 90, 0, 255),
            Color::rgba8(255, 212, 38, 255),
        )
        .labelled("systemYellow");

    /// The system's base grey.
    pub const SYSTEM_GREY: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(142, 142, 147, 255),
            Color::rgba8(142, 142, 147, 255),
            Color::rgba8(108, 108, 112, 255),
            Color::rgba8(174, 174, 178, 255),
        )
        .labelled("systemGrey");

    /// The system's second grey, a step lighter in the light and darker in the dark.
    pub const SYSTEM_GREY_2: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(174, 174, 178, 255),
            Color::rgba8(99, 99, 102, 255),
            Color::rgba8(142, 142, 147, 255),
            Color::rgba8(124, 124, 128, 255),
        )
        .labelled("systemGrey2");

    /// The system's third grey.
    pub const SYSTEM_GREY_3: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(199, 199, 204, 255),
            Color::rgba8(72, 72, 74, 255),
            Color::rgba8(174, 174, 178, 255),
            Color::rgba8(84, 84, 86, 255),
        )
        .labelled("systemGrey3");

    /// The system's fourth grey.
    pub const SYSTEM_GREY_4: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(209, 209, 214, 255),
            Color::rgba8(58, 58, 60, 255),
            Color::rgba8(188, 188, 192, 255),
            Color::rgba8(68, 68, 70, 255),
        )
        .labelled("systemGrey4");

    /// The system's fifth grey.
    pub const SYSTEM_GREY_5: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(229, 229, 234, 255),
            Color::rgba8(44, 44, 46, 255),
            Color::rgba8(216, 216, 220, 255),
            Color::rgba8(54, 54, 56, 255),
        )
        .labelled("systemGrey5");

    /// The system's sixth grey, the nearest to the background.
    pub const SYSTEM_GREY_6: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::rgba8(242, 242, 247, 255),
            Color::rgba8(28, 28, 30, 255),
            Color::rgba8(235, 235, 240, 255),
            Color::rgba8(36, 36, 38, 255),
        )
        .labelled("systemGrey6");

    /// Text that holds the primary content.
    pub const LABEL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(255, 255, 255, 255),
    )
    .labelled("label");

    /// Text that holds secondary content.
    pub const SECONDARY_LABEL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(60, 60, 67, 153),
        Color::rgba8(235, 235, 245, 153),
        Color::rgba8(60, 60, 67, 173),
        Color::rgba8(235, 235, 245, 173),
        Color::rgba8(60, 60, 67, 153),
        Color::rgba8(235, 235, 245, 153),
        Color::rgba8(60, 60, 67, 173),
        Color::rgba8(235, 235, 245, 173),
    )
    .labelled("secondaryLabel");

    /// Text that holds tertiary content.
    pub const TERTIARY_LABEL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(60, 60, 67, 76),
        Color::rgba8(235, 235, 245, 76),
        Color::rgba8(60, 60, 67, 96),
        Color::rgba8(235, 235, 245, 96),
        Color::rgba8(60, 60, 67, 76),
        Color::rgba8(235, 235, 245, 76),
        Color::rgba8(60, 60, 67, 96),
        Color::rgba8(235, 235, 245, 96),
    )
    .labelled("tertiaryLabel");

    /// Text that holds quaternary content.
    pub const QUATERNARY_LABEL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(60, 60, 67, 45),
        Color::rgba8(235, 235, 245, 40),
        Color::rgba8(60, 60, 67, 66),
        Color::rgba8(235, 235, 245, 61),
        Color::rgba8(60, 60, 67, 45),
        Color::rgba8(235, 235, 245, 40),
        Color::rgba8(60, 60, 67, 66),
        Color::rgba8(235, 235, 245, 61),
    )
    .labelled("quaternaryLabel");

    /// An overlay fill for thin and small shapes.
    pub const SYSTEM_FILL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(120, 120, 128, 51),
        Color::rgba8(120, 120, 128, 91),
        Color::rgba8(120, 120, 128, 71),
        Color::rgba8(120, 120, 128, 112),
        Color::rgba8(120, 120, 128, 51),
        Color::rgba8(120, 120, 128, 91),
        Color::rgba8(120, 120, 128, 71),
        Color::rgba8(120, 120, 128, 112),
    )
    .labelled("systemFill");

    /// An overlay fill for medium-sized shapes.
    pub const SECONDARY_SYSTEM_FILL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(120, 120, 128, 40),
        Color::rgba8(120, 120, 128, 81),
        Color::rgba8(120, 120, 128, 61),
        Color::rgba8(120, 120, 128, 102),
        Color::rgba8(120, 120, 128, 40),
        Color::rgba8(120, 120, 128, 81),
        Color::rgba8(120, 120, 128, 61),
        Color::rgba8(120, 120, 128, 102),
    )
    .labelled("secondarySystemFill");

    /// An overlay fill for large shapes.
    pub const TERTIARY_SYSTEM_FILL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(118, 118, 128, 30),
        Color::rgba8(118, 118, 128, 61),
        Color::rgba8(118, 118, 128, 51),
        Color::rgba8(118, 118, 128, 81),
        Color::rgba8(118, 118, 128, 30),
        Color::rgba8(118, 118, 128, 61),
        Color::rgba8(118, 118, 128, 51),
        Color::rgba8(118, 118, 128, 81),
    )
    .labelled("tertiarySystemFill");

    /// An overlay fill for large areas holding complex content.
    pub const QUATERNARY_SYSTEM_FILL: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(116, 116, 128, 20),
        Color::rgba8(118, 118, 128, 45),
        Color::rgba8(116, 116, 128, 40),
        Color::rgba8(118, 118, 128, 66),
        Color::rgba8(116, 116, 128, 20),
        Color::rgba8(118, 118, 128, 45),
        Color::rgba8(116, 116, 128, 40),
        Color::rgba8(118, 118, 128, 66),
    )
    .labelled("quaternarySystemFill");

    /// A field's placeholder text.
    pub const PLACEHOLDER_TEXT: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(60, 60, 67, 76),
        Color::rgba8(235, 235, 245, 76),
        Color::rgba8(60, 60, 67, 96),
        Color::rgba8(235, 235, 245, 96),
        Color::rgba8(60, 60, 67, 76),
        Color::rgba8(235, 235, 245, 76),
        Color::rgba8(60, 60, 67, 96),
        Color::rgba8(235, 235, 245, 96),
    )
    .labelled("placeholderText");

    /// The interface's main background.
    pub const SYSTEM_BACKGROUND: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(28, 28, 30, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(36, 36, 38, 255),
    )
    .labelled("systemBackground");

    /// Content layered on the main background.
    pub const SECONDARY_SYSTEM_BACKGROUND: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(242, 242, 247, 255),
        Color::rgba8(28, 28, 30, 255),
        Color::rgba8(235, 235, 240, 255),
        Color::rgba8(36, 36, 38, 255),
        Color::rgba8(242, 242, 247, 255),
        Color::rgba8(44, 44, 46, 255),
        Color::rgba8(235, 235, 240, 255),
        Color::rgba8(54, 54, 56, 255),
    )
    .labelled("secondarySystemBackground");

    /// Content layered on a secondary background.
    pub const TERTIARY_SYSTEM_BACKGROUND: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(44, 44, 46, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(54, 54, 56, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(58, 58, 60, 255),
        Color::rgba8(255, 255, 255, 255),
        Color::rgba8(68, 68, 70, 255),
    )
    .labelled("tertiarySystemBackground");

    /// The main background of a grouped interface.
    pub const SYSTEM_GROUPED_BACKGROUND: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(242, 242, 247, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(235, 235, 240, 255),
        Color::rgba8(0, 0, 0, 255),
        Color::rgba8(242, 242, 247, 255),
        Color::rgba8(28, 28, 30, 255),
        Color::rgba8(235, 235, 240, 255),
        Color::rgba8(36, 36, 38, 255),
    )
    .labelled("systemGroupedBackground");

    /// Content layered on a grouped interface's main background.
    pub const SECONDARY_SYSTEM_GROUPED_BACKGROUND: CupertinoDynamicColor =
        CupertinoDynamicColor::new(
            Color::rgba8(255, 255, 255, 255),
            Color::rgba8(28, 28, 30, 255),
            Color::rgba8(255, 255, 255, 255),
            Color::rgba8(36, 36, 38, 255),
            Color::rgba8(255, 255, 255, 255),
            Color::rgba8(44, 44, 46, 255),
            Color::rgba8(255, 255, 255, 255),
            Color::rgba8(54, 54, 56, 255),
        )
        .labelled("secondarySystemGroupedBackground");

    /// Content layered on a grouped interface's secondary background.
    pub const TERTIARY_SYSTEM_GROUPED_BACKGROUND: CupertinoDynamicColor =
        CupertinoDynamicColor::new(
            Color::rgba8(242, 242, 247, 255),
            Color::rgba8(44, 44, 46, 255),
            Color::rgba8(235, 235, 240, 255),
            Color::rgba8(54, 54, 56, 255),
            Color::rgba8(242, 242, 247, 255),
            Color::rgba8(58, 58, 60, 255),
            Color::rgba8(235, 235, 240, 255),
            Color::rgba8(68, 68, 70, 255),
        )
        .labelled("tertiarySystemGroupedBackground");

    /// Thin borders and dividers that let some of what is under them show.
    pub const SEPARATOR: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(60, 60, 67, 73),
        Color::rgba8(84, 84, 88, 153),
        Color::rgba8(60, 60, 67, 94),
        Color::rgba8(84, 84, 88, 173),
        Color::rgba8(60, 60, 67, 73),
        Color::rgba8(210, 210, 210, 153),
        Color::rgba8(60, 60, 67, 94),
        Color::rgba8(84, 84, 88, 173),
    )
    .labelled("separator");

    /// Borders and dividers that hide what is under them.
    pub const OPAQUE_SEPARATOR: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(198, 198, 200, 255),
        Color::rgba8(56, 56, 58, 255),
        Color::rgba8(198, 198, 200, 255),
        Color::rgba8(56, 56, 58, 255),
        Color::rgba8(198, 198, 200, 255),
        Color::rgba8(56, 56, 58, 255),
        Color::rgba8(198, 198, 200, 255),
        Color::rgba8(56, 56, 58, 255),
    )
    .labelled("opaqueSeparator");

    /// Links.
    pub const LINK: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::rgba8(0, 122, 255, 255),
        Color::rgba8(9, 132, 255, 255),
        Color::rgba8(0, 122, 255, 255),
        Color::rgba8(9, 132, 255, 255),
        Color::rgba8(0, 122, 255, 255),
        Color::rgba8(9, 132, 255, 255),
        Color::rgba8(0, 122, 255, 255),
        Color::rgba8(9, 132, 255, 255),
    )
    .labelled("link");
}

#[cfg(test)]
mod tests {
    use super::{CupertinoColors as C, CupertinoDynamicColor, CupertinoUserInterfaceLevelData};
    use crate::media::{Accessibility, Brightness, MediaQuery};
    use crate::{build_ui, Container, Flex, Runtime, Theme, ThemeBuilder, Widget};
    use frus_core::{Color, Primitive, Size};
    use CupertinoUserInterfaceLevelData::{Base, Elevated};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {}

    /// **Each of the eight conditions picks its own value** — the reference's table of
    /// brightness × level × contrast, cell by cell.
    #[test]
    fn each_condition_picks_its_value() {
        let v: Vec<Color> = (0..8).map(|i| Color::rgb8(i * 10, 0, 0)).collect();
        let c = CupertinoDynamicColor::new(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
        let light = Brightness::Light;
        let dark = Brightness::Dark;
        assert_eq!(c.resolve_with(light, Base, false), v[0]);
        assert_eq!(c.resolve_with(dark, Base, false), v[1]);
        assert_eq!(c.resolve_with(light, Base, true), v[2]);
        assert_eq!(c.resolve_with(dark, Base, true), v[3]);
        assert_eq!(c.resolve_with(light, Elevated, false), v[4]);
        assert_eq!(c.resolve_with(dark, Elevated, false), v[5]);
        assert_eq!(c.resolve_with(light, Elevated, true), v[6]);
        assert_eq!(c.resolve_with(dark, Elevated, true), v[7]);
        // Unresolved, it is its first value.
        assert_eq!(Color::from(c), v[0]);
    }

    /// **The shorter constructors fill the axes they do not vary along**, as the reference's
    /// do.
    #[test]
    fn the_shorter_constructors_fill_the_other_axes() {
        let [a, b, c, d] = [1, 2, 3, 4].map(|i| Color::rgb8(i, 0, 0));
        assert_eq!(
            CupertinoDynamicColor::with_brightness_and_contrast(a, b, c, d),
            CupertinoDynamicColor::new(a, b, c, d, a, b, c, d)
        );
        assert_eq!(
            CupertinoDynamicColor::with_brightness(a, b),
            CupertinoDynamicColor::new(a, b, a, b, a, b, a, b)
        );
        // The label is a name, not a value.
        assert_eq!(
            CupertinoDynamicColor::with_brightness(a, b).labelled("ink"),
            CupertinoDynamicColor::with_brightness(a, b)
        );
    }

    /// **The palette is the reference's**, checked at the values that are easiest to get
    /// wrong: a system colour's four, a label's translucency, a background that lightens
    /// when raised in the dark, and the aliases.
    #[test]
    fn the_palette_is_the_reference_s() {
        let rgba = Color::rgba8;
        assert_eq!(
            C::SYSTEM_BLUE,
            CupertinoDynamicColor::with_brightness_and_contrast(
                rgba(0, 122, 255, 255),
                rgba(10, 132, 255, 255),
                rgba(0, 64, 221, 255),
                rgba(64, 156, 255, 255),
            )
        );
        assert_eq!(C::SYSTEM_BLUE.debug_label, Some("systemBlue"));
        assert_eq!(C::ACTIVE_BLUE, C::SYSTEM_BLUE);
        assert_eq!(C::ACTIVE_GREEN, C::SYSTEM_GREEN);
        assert_eq!(C::ACTIVE_ORANGE, C::SYSTEM_ORANGE);
        assert_eq!(C::DESTRUCTIVE_RED, C::SYSTEM_RED);
        assert_eq!(C::SECONDARY_LABEL.color, rgba(60, 60, 67, 153));
        assert_eq!(
            C::SECONDARY_LABEL.dark_high_contrast_color,
            rgba(235, 235, 245, 173)
        );
        assert_eq!(C::SYSTEM_BACKGROUND.dark_color, rgba(0, 0, 0, 255));
        assert_eq!(
            C::SYSTEM_BACKGROUND.dark_elevated_color,
            rgba(28, 28, 30, 255)
        );
        assert_eq!(
            C::SYSTEM_BACKGROUND.dark_high_contrast_elevated_color,
            rgba(36, 36, 38, 255)
        );
        assert_eq!(C::SEPARATOR.dark_elevated_color, rgba(210, 210, 210, 153));
        assert_eq!(C::SYSTEM_GREY_6.color, rgba(242, 242, 247, 255));
        assert_eq!(
            C::INACTIVE_GRAY.dark_color,
            Color::from_argb_u32(0xFF757575)
        );
        assert_eq!(
            C::EXTRA_LIGHT_BACKGROUND_GRAY,
            Color::from_argb_u32(0xFFEFEFF4)
        );
    }

    /// **Where it is used decides**: the theme's brightness, the level the theme carries, and
    /// the reader's high-contrast setting from the ambient surface.
    #[test]
    fn resolve_reads_the_theme_and_the_surface() {
        let bg = C::SYSTEM_BACKGROUND;
        assert_eq!(bg.resolve(&Theme::light()), bg.color);
        assert_eq!(bg.resolve(&Theme::dark()), bg.dark_color);
        let mut raised = Theme::dark();
        raised.user_interface_level = Elevated;
        assert_eq!(bg.resolve(&raised), bg.dark_elevated_color);
        let mut surface = MediaQuery::new(Size::new(100.0, 100.0));
        surface.accessibility = Accessibility {
            high_contrast: true,
            ..Accessibility::NONE
        };
        surface.scope(|| {
            assert_eq!(bg.resolve(&raised), bg.dark_high_contrast_elevated_color);
            assert_eq!(
                C::LABEL.resolve(&Theme::light()),
                C::LABEL.high_contrast_color
            );
        });
    }

    /// **A theme starts at the base level, keeps it through a fade, and a subtree raised with
    /// `around` resolves its colours raised** — while its sibling, outside, does not.
    #[test]
    fn a_raised_subtree_resolves_raised() {
        assert_eq!(Theme::default().user_interface_level, Base);
        let mut raised = Theme::dark();
        raised.user_interface_level = Elevated;
        assert_eq!(raised.lerp(&raised, 0.3).user_interface_level, Elevated);
        assert_eq!(
            Theme::dark().lerp(&raised, 0.7).user_interface_level,
            Elevated
        );
        assert_eq!(Theme::dark().lerp(&raised, 0.3).user_interface_level, Base);

        let swatch = || {
            ThemeBuilder::new(|t: &Theme| {
                Container::new()
                    .width(50.0)
                    .height(50.0)
                    .color(C::SYSTEM_BACKGROUND.resolve(t))
            })
        };
        let tree = Flex::<Msg>::column()
            .width(200.0)
            .height(200.0)
            .child(swatch())
            .child(Elevated.around(swatch()));
        let painted: Vec<Color> = build_ui(
            &tree as &dyn Widget<Msg>,
            Size::new(200.0, 200.0),
            &Runtime::default(),
            &Theme::dark(),
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
                C::SYSTEM_BACKGROUND.dark_color,
                C::SYSTEM_BACKGROUND.dark_elevated_color
            ]
        );
        assert_eq!(CupertinoUserInterfaceLevelData::of(&raised), Elevated);
    }
}
