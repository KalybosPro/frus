//! The **material palette**: every named colour of the material design system, as
//! plain [`Color`] constants ([`Colors`]) and as whole ramps ([`MaterialColor`]).
//!
//! Two shapes, because there are two jobs:
//!
//! * [`Colors`] holds one constant per colour — `Colors::RED`, `Colors::RED_300`,
//!   `Colors::BLUE_ACCENT`, `Colors::BLACK54`. They are `Color`s, so they go straight
//!   into any API that takes one, with nothing to unwrap.
//! * [`MaterialColor`] holds the **ramp**: the ten (or four, for an accent) steps of one
//!   hue, indexable by step. That is what a theme wants when it derives a whole scheme
//!   from one family rather than picking a single tone.
//!
//! `Colors::RED` and `MaterialColor::RED.primary()` are the same colour; the ramp type
//! exists so the other nine are reachable *as a set*.
//!
//! The values are sRGB, the way the design system states them, stated as `0xAARRGGBB`
//! so that a constant can be checked against the specification by eye.

use crate::Color;

/// A **colour ramp**: one hue in numbered steps, from the palest (`50`) to the darkest
/// (`900`), plus the `primary` step the family is named after.
///
/// Accent ramps carry the four steps `100`, `200`, `400`, `700`; the grey ramp carries
/// two extra half-steps (`350`, `850`). That is why the steps are a slice rather than a
/// fixed array — a ramp states which steps it has, and [`MaterialColor::shade`] answers
/// `None` for one it does not.
///
/// ```
/// use frus_core::MaterialColor;
///
/// assert_eq!(MaterialColor::RED.shade(300), Some(frus_core::Colors::RED_300));
/// assert_eq!(MaterialColor::RED.shade(350), None); // only grey has the half-steps
/// assert_eq!(MaterialColor::GREY.shade(350), Some(frus_core::Colors::GREY_350));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialColor {
    primary: Color,
    shades: &'static [(u16, Color)],
}

impl MaterialColor {
    /// Builds a ramp from its primary step and its numbered steps, which must be in
    /// ascending order.
    pub const fn new(primary: Color, shades: &'static [(u16, Color)]) -> Self {
        Self { primary, shades }
    }

    /// The step the family is named after — `500` for a primary ramp, `200` for an accent.
    pub const fn primary(self) -> Color {
        self.primary
    }

    /// The colour at `step`, or `None` if this ramp has no such step.
    pub fn shade(self, step: u16) -> Option<Color> {
        self.shades
            .iter()
            .find(|(s, _)| *s == step)
            .map(|(_, c)| *c)
    }

    /// The colour at `step`, falling back to the primary if the ramp has no such step.
    pub fn shade_or_primary(self, step: u16) -> Color {
        self.shade(step).unwrap_or(self.primary)
    }

    /// Every `(step, colour)` pair, in ascending order.
    pub fn steps(self) -> impl Iterator<Item = (u16, Color)> {
        self.shades.iter().copied()
    }

    /// The number of steps in the ramp.
    pub fn len(self) -> usize {
        self.shades.len()
    }

    /// Whether the ramp has no steps at all — no bundled family is empty.
    pub fn is_empty(self) -> bool {
        self.shades.is_empty()
    }
}

impl From<MaterialColor> for Color {
    /// A ramp used where a single colour is expected is its primary step.
    fn from(ramp: MaterialColor) -> Color {
        ramp.primary
    }
}

// The step tables. Private: a ramp is reached through `MaterialColor`, and a single
// tone through `Colors`.
#[rustfmt::skip]
const RED_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFFEBEE)),
    (100, Color::from_argb_u32(0xFFFFCDD2)),
    (200, Color::from_argb_u32(0xFFEF9A9A)),
    (300, Color::from_argb_u32(0xFFE57373)),
    (400, Color::from_argb_u32(0xFFEF5350)),
    (500, Color::from_argb_u32(0xFFF44336)),
    (600, Color::from_argb_u32(0xFFE53935)),
    (700, Color::from_argb_u32(0xFFD32F2F)),
    (800, Color::from_argb_u32(0xFFC62828)),
    (900, Color::from_argb_u32(0xFFB71C1C)),
];
#[rustfmt::skip]
const PINK_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFCE4EC)),
    (100, Color::from_argb_u32(0xFFF8BBD0)),
    (200, Color::from_argb_u32(0xFFF48FB1)),
    (300, Color::from_argb_u32(0xFFF06292)),
    (400, Color::from_argb_u32(0xFFEC407A)),
    (500, Color::from_argb_u32(0xFFE91E63)),
    (600, Color::from_argb_u32(0xFFD81B60)),
    (700, Color::from_argb_u32(0xFFC2185B)),
    (800, Color::from_argb_u32(0xFFAD1457)),
    (900, Color::from_argb_u32(0xFF880E4F)),
];
#[rustfmt::skip]
const PURPLE_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFF3E5F5)),
    (100, Color::from_argb_u32(0xFFE1BEE7)),
    (200, Color::from_argb_u32(0xFFCE93D8)),
    (300, Color::from_argb_u32(0xFFBA68C8)),
    (400, Color::from_argb_u32(0xFFAB47BC)),
    (500, Color::from_argb_u32(0xFF9C27B0)),
    (600, Color::from_argb_u32(0xFF8E24AA)),
    (700, Color::from_argb_u32(0xFF7B1FA2)),
    (800, Color::from_argb_u32(0xFF6A1B9A)),
    (900, Color::from_argb_u32(0xFF4A148C)),
];
#[rustfmt::skip]
const DEEP_PURPLE_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFEDE7F6)),
    (100, Color::from_argb_u32(0xFFD1C4E9)),
    (200, Color::from_argb_u32(0xFFB39DDB)),
    (300, Color::from_argb_u32(0xFF9575CD)),
    (400, Color::from_argb_u32(0xFF7E57C2)),
    (500, Color::from_argb_u32(0xFF673AB7)),
    (600, Color::from_argb_u32(0xFF5E35B1)),
    (700, Color::from_argb_u32(0xFF512DA8)),
    (800, Color::from_argb_u32(0xFF4527A0)),
    (900, Color::from_argb_u32(0xFF311B92)),
];
#[rustfmt::skip]
const INDIGO_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFE8EAF6)),
    (100, Color::from_argb_u32(0xFFC5CAE9)),
    (200, Color::from_argb_u32(0xFF9FA8DA)),
    (300, Color::from_argb_u32(0xFF7986CB)),
    (400, Color::from_argb_u32(0xFF5C6BC0)),
    (500, Color::from_argb_u32(0xFF3F51B5)),
    (600, Color::from_argb_u32(0xFF3949AB)),
    (700, Color::from_argb_u32(0xFF303F9F)),
    (800, Color::from_argb_u32(0xFF283593)),
    (900, Color::from_argb_u32(0xFF1A237E)),
];
#[rustfmt::skip]
const BLUE_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFE3F2FD)),
    (100, Color::from_argb_u32(0xFFBBDEFB)),
    (200, Color::from_argb_u32(0xFF90CAF9)),
    (300, Color::from_argb_u32(0xFF64B5F6)),
    (400, Color::from_argb_u32(0xFF42A5F5)),
    (500, Color::from_argb_u32(0xFF2196F3)),
    (600, Color::from_argb_u32(0xFF1E88E5)),
    (700, Color::from_argb_u32(0xFF1976D2)),
    (800, Color::from_argb_u32(0xFF1565C0)),
    (900, Color::from_argb_u32(0xFF0D47A1)),
];
#[rustfmt::skip]
const LIGHT_BLUE_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFE1F5FE)),
    (100, Color::from_argb_u32(0xFFB3E5FC)),
    (200, Color::from_argb_u32(0xFF81D4FA)),
    (300, Color::from_argb_u32(0xFF4FC3F7)),
    (400, Color::from_argb_u32(0xFF29B6F6)),
    (500, Color::from_argb_u32(0xFF03A9F4)),
    (600, Color::from_argb_u32(0xFF039BE5)),
    (700, Color::from_argb_u32(0xFF0288D1)),
    (800, Color::from_argb_u32(0xFF0277BD)),
    (900, Color::from_argb_u32(0xFF01579B)),
];
#[rustfmt::skip]
const CYAN_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFE0F7FA)),
    (100, Color::from_argb_u32(0xFFB2EBF2)),
    (200, Color::from_argb_u32(0xFF80DEEA)),
    (300, Color::from_argb_u32(0xFF4DD0E1)),
    (400, Color::from_argb_u32(0xFF26C6DA)),
    (500, Color::from_argb_u32(0xFF00BCD4)),
    (600, Color::from_argb_u32(0xFF00ACC1)),
    (700, Color::from_argb_u32(0xFF0097A7)),
    (800, Color::from_argb_u32(0xFF00838F)),
    (900, Color::from_argb_u32(0xFF006064)),
];
#[rustfmt::skip]
const TEAL_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFE0F2F1)),
    (100, Color::from_argb_u32(0xFFB2DFDB)),
    (200, Color::from_argb_u32(0xFF80CBC4)),
    (300, Color::from_argb_u32(0xFF4DB6AC)),
    (400, Color::from_argb_u32(0xFF26A69A)),
    (500, Color::from_argb_u32(0xFF009688)),
    (600, Color::from_argb_u32(0xFF00897B)),
    (700, Color::from_argb_u32(0xFF00796B)),
    (800, Color::from_argb_u32(0xFF00695C)),
    (900, Color::from_argb_u32(0xFF004D40)),
];
#[rustfmt::skip]
const GREEN_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFE8F5E9)),
    (100, Color::from_argb_u32(0xFFC8E6C9)),
    (200, Color::from_argb_u32(0xFFA5D6A7)),
    (300, Color::from_argb_u32(0xFF81C784)),
    (400, Color::from_argb_u32(0xFF66BB6A)),
    (500, Color::from_argb_u32(0xFF4CAF50)),
    (600, Color::from_argb_u32(0xFF43A047)),
    (700, Color::from_argb_u32(0xFF388E3C)),
    (800, Color::from_argb_u32(0xFF2E7D32)),
    (900, Color::from_argb_u32(0xFF1B5E20)),
];
#[rustfmt::skip]
const LIGHT_GREEN_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFF1F8E9)),
    (100, Color::from_argb_u32(0xFFDCEDC8)),
    (200, Color::from_argb_u32(0xFFC5E1A5)),
    (300, Color::from_argb_u32(0xFFAED581)),
    (400, Color::from_argb_u32(0xFF9CCC65)),
    (500, Color::from_argb_u32(0xFF8BC34A)),
    (600, Color::from_argb_u32(0xFF7CB342)),
    (700, Color::from_argb_u32(0xFF689F38)),
    (800, Color::from_argb_u32(0xFF558B2F)),
    (900, Color::from_argb_u32(0xFF33691E)),
];
#[rustfmt::skip]
const LIME_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFF9FBE7)),
    (100, Color::from_argb_u32(0xFFF0F4C3)),
    (200, Color::from_argb_u32(0xFFE6EE9C)),
    (300, Color::from_argb_u32(0xFFDCE775)),
    (400, Color::from_argb_u32(0xFFD4E157)),
    (500, Color::from_argb_u32(0xFFCDDC39)),
    (600, Color::from_argb_u32(0xFFC0CA33)),
    (700, Color::from_argb_u32(0xFFAFB42B)),
    (800, Color::from_argb_u32(0xFF9E9D24)),
    (900, Color::from_argb_u32(0xFF827717)),
];
#[rustfmt::skip]
const YELLOW_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFFFDE7)),
    (100, Color::from_argb_u32(0xFFFFF9C4)),
    (200, Color::from_argb_u32(0xFFFFF59D)),
    (300, Color::from_argb_u32(0xFFFFF176)),
    (400, Color::from_argb_u32(0xFFFFEE58)),
    (500, Color::from_argb_u32(0xFFFFEB3B)),
    (600, Color::from_argb_u32(0xFFFDD835)),
    (700, Color::from_argb_u32(0xFFFBC02D)),
    (800, Color::from_argb_u32(0xFFF9A825)),
    (900, Color::from_argb_u32(0xFFF57F17)),
];
#[rustfmt::skip]
const AMBER_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFFF8E1)),
    (100, Color::from_argb_u32(0xFFFFECB3)),
    (200, Color::from_argb_u32(0xFFFFE082)),
    (300, Color::from_argb_u32(0xFFFFD54F)),
    (400, Color::from_argb_u32(0xFFFFCA28)),
    (500, Color::from_argb_u32(0xFFFFC107)),
    (600, Color::from_argb_u32(0xFFFFB300)),
    (700, Color::from_argb_u32(0xFFFFA000)),
    (800, Color::from_argb_u32(0xFFFF8F00)),
    (900, Color::from_argb_u32(0xFFFF6F00)),
];
#[rustfmt::skip]
const ORANGE_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFFF3E0)),
    (100, Color::from_argb_u32(0xFFFFE0B2)),
    (200, Color::from_argb_u32(0xFFFFCC80)),
    (300, Color::from_argb_u32(0xFFFFB74D)),
    (400, Color::from_argb_u32(0xFFFFA726)),
    (500, Color::from_argb_u32(0xFFFF9800)),
    (600, Color::from_argb_u32(0xFFFB8C00)),
    (700, Color::from_argb_u32(0xFFF57C00)),
    (800, Color::from_argb_u32(0xFFEF6C00)),
    (900, Color::from_argb_u32(0xFFE65100)),
];
#[rustfmt::skip]
const DEEP_ORANGE_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFBE9E7)),
    (100, Color::from_argb_u32(0xFFFFCCBC)),
    (200, Color::from_argb_u32(0xFFFFAB91)),
    (300, Color::from_argb_u32(0xFFFF8A65)),
    (400, Color::from_argb_u32(0xFFFF7043)),
    (500, Color::from_argb_u32(0xFFFF5722)),
    (600, Color::from_argb_u32(0xFFF4511E)),
    (700, Color::from_argb_u32(0xFFE64A19)),
    (800, Color::from_argb_u32(0xFFD84315)),
    (900, Color::from_argb_u32(0xFFBF360C)),
];
#[rustfmt::skip]
const BROWN_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFEFEBE9)),
    (100, Color::from_argb_u32(0xFFD7CCC8)),
    (200, Color::from_argb_u32(0xFFBCAAA4)),
    (300, Color::from_argb_u32(0xFFA1887F)),
    (400, Color::from_argb_u32(0xFF8D6E63)),
    (500, Color::from_argb_u32(0xFF795548)),
    (600, Color::from_argb_u32(0xFF6D4C41)),
    (700, Color::from_argb_u32(0xFF5D4037)),
    (800, Color::from_argb_u32(0xFF4E342E)),
    (900, Color::from_argb_u32(0xFF3E2723)),
];
#[rustfmt::skip]
const GREY_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFFAFAFA)),
    (100, Color::from_argb_u32(0xFFF5F5F5)),
    (200, Color::from_argb_u32(0xFFEEEEEE)),
    (300, Color::from_argb_u32(0xFFE0E0E0)),
    (350, Color::from_argb_u32(0xFFD6D6D6)),
    (400, Color::from_argb_u32(0xFFBDBDBD)),
    (500, Color::from_argb_u32(0xFF9E9E9E)),
    (600, Color::from_argb_u32(0xFF757575)),
    (700, Color::from_argb_u32(0xFF616161)),
    (800, Color::from_argb_u32(0xFF424242)),
    (850, Color::from_argb_u32(0xFF303030)),
    (900, Color::from_argb_u32(0xFF212121)),
];
#[rustfmt::skip]
const BLUE_GREY_SHADES: &[(u16, Color)] = &[
    (50, Color::from_argb_u32(0xFFECEFF1)),
    (100, Color::from_argb_u32(0xFFCFD8DC)),
    (200, Color::from_argb_u32(0xFFB0BEC5)),
    (300, Color::from_argb_u32(0xFF90A4AE)),
    (400, Color::from_argb_u32(0xFF78909C)),
    (500, Color::from_argb_u32(0xFF607D8B)),
    (600, Color::from_argb_u32(0xFF546E7A)),
    (700, Color::from_argb_u32(0xFF455A64)),
    (800, Color::from_argb_u32(0xFF37474F)),
    (900, Color::from_argb_u32(0xFF263238)),
];
#[rustfmt::skip]
const RED_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFFF8A80)),
    (200, Color::from_argb_u32(0xFFFF5252)),
    (400, Color::from_argb_u32(0xFFFF1744)),
    (700, Color::from_argb_u32(0xFFD50000)),
];
#[rustfmt::skip]
const PINK_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFFF80AB)),
    (200, Color::from_argb_u32(0xFFFF4081)),
    (400, Color::from_argb_u32(0xFFF50057)),
    (700, Color::from_argb_u32(0xFFC51162)),
];
#[rustfmt::skip]
const PURPLE_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFEA80FC)),
    (200, Color::from_argb_u32(0xFFE040FB)),
    (400, Color::from_argb_u32(0xFFD500F9)),
    (700, Color::from_argb_u32(0xFFAA00FF)),
];
#[rustfmt::skip]
const DEEP_PURPLE_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFB388FF)),
    (200, Color::from_argb_u32(0xFF7C4DFF)),
    (400, Color::from_argb_u32(0xFF651FFF)),
    (700, Color::from_argb_u32(0xFF6200EA)),
];
#[rustfmt::skip]
const INDIGO_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFF8C9EFF)),
    (200, Color::from_argb_u32(0xFF536DFE)),
    (400, Color::from_argb_u32(0xFF3D5AFE)),
    (700, Color::from_argb_u32(0xFF304FFE)),
];
#[rustfmt::skip]
const BLUE_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFF82B1FF)),
    (200, Color::from_argb_u32(0xFF448AFF)),
    (400, Color::from_argb_u32(0xFF2979FF)),
    (700, Color::from_argb_u32(0xFF2962FF)),
];
#[rustfmt::skip]
const LIGHT_BLUE_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFF80D8FF)),
    (200, Color::from_argb_u32(0xFF40C4FF)),
    (400, Color::from_argb_u32(0xFF00B0FF)),
    (700, Color::from_argb_u32(0xFF0091EA)),
];
#[rustfmt::skip]
const CYAN_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFF84FFFF)),
    (200, Color::from_argb_u32(0xFF18FFFF)),
    (400, Color::from_argb_u32(0xFF00E5FF)),
    (700, Color::from_argb_u32(0xFF00B8D4)),
];
#[rustfmt::skip]
const TEAL_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFA7FFEB)),
    (200, Color::from_argb_u32(0xFF64FFDA)),
    (400, Color::from_argb_u32(0xFF1DE9B6)),
    (700, Color::from_argb_u32(0xFF00BFA5)),
];
#[rustfmt::skip]
const GREEN_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFB9F6CA)),
    (200, Color::from_argb_u32(0xFF69F0AE)),
    (400, Color::from_argb_u32(0xFF00E676)),
    (700, Color::from_argb_u32(0xFF00C853)),
];
#[rustfmt::skip]
const LIGHT_GREEN_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFCCFF90)),
    (200, Color::from_argb_u32(0xFFB2FF59)),
    (400, Color::from_argb_u32(0xFF76FF03)),
    (700, Color::from_argb_u32(0xFF64DD17)),
];
#[rustfmt::skip]
const LIME_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFF4FF81)),
    (200, Color::from_argb_u32(0xFFEEFF41)),
    (400, Color::from_argb_u32(0xFFC6FF00)),
    (700, Color::from_argb_u32(0xFFAEEA00)),
];
#[rustfmt::skip]
const YELLOW_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFFFFF8D)),
    (200, Color::from_argb_u32(0xFFFFFF00)),
    (400, Color::from_argb_u32(0xFFFFEA00)),
    (700, Color::from_argb_u32(0xFFFFD600)),
];
#[rustfmt::skip]
const AMBER_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFFFE57F)),
    (200, Color::from_argb_u32(0xFFFFD740)),
    (400, Color::from_argb_u32(0xFFFFC400)),
    (700, Color::from_argb_u32(0xFFFFAB00)),
];
#[rustfmt::skip]
const ORANGE_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFFFD180)),
    (200, Color::from_argb_u32(0xFFFFAB40)),
    (400, Color::from_argb_u32(0xFFFF9100)),
    (700, Color::from_argb_u32(0xFFFF6D00)),
];
#[rustfmt::skip]
const DEEP_ORANGE_ACCENT_SHADES: &[(u16, Color)] = &[
    (100, Color::from_argb_u32(0xFFFF9E80)),
    (200, Color::from_argb_u32(0xFFFF6E40)),
    (400, Color::from_argb_u32(0xFFFF3D00)),
    (700, Color::from_argb_u32(0xFFDD2C00)),
];

impl MaterialColor {
    /// The **red** ramp: 10 steps, `50` to `900`.
    pub const RED: MaterialColor = MaterialColor::new(Color::from_argb_u32(0xFFF44336), RED_SHADES);
    /// The **pink** ramp: 10 steps, `50` to `900`.
    pub const PINK: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFE91E63), PINK_SHADES);
    /// The **purple** ramp: 10 steps, `50` to `900`.
    pub const PURPLE: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF9C27B0), PURPLE_SHADES);
    /// The **deep purple** ramp: 10 steps, `50` to `900`.
    pub const DEEP_PURPLE: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF673AB7), DEEP_PURPLE_SHADES);
    /// The **indigo** ramp: 10 steps, `50` to `900`.
    pub const INDIGO: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF3F51B5), INDIGO_SHADES);
    /// The **blue** ramp: 10 steps, `50` to `900`.
    pub const BLUE: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF2196F3), BLUE_SHADES);
    /// The **light blue** ramp: 10 steps, `50` to `900`.
    pub const LIGHT_BLUE: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF03A9F4), LIGHT_BLUE_SHADES);
    /// The **cyan** ramp: 10 steps, `50` to `900`.
    pub const CYAN: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF00BCD4), CYAN_SHADES);
    /// The **teal** ramp: 10 steps, `50` to `900`.
    pub const TEAL: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF009688), TEAL_SHADES);
    /// The **green** ramp: 10 steps, `50` to `900`.
    pub const GREEN: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF4CAF50), GREEN_SHADES);
    /// The **light green** ramp: 10 steps, `50` to `900`.
    pub const LIGHT_GREEN: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF8BC34A), LIGHT_GREEN_SHADES);
    /// The **lime** ramp: 10 steps, `50` to `900`.
    pub const LIME: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFCDDC39), LIME_SHADES);
    /// The **yellow** ramp: 10 steps, `50` to `900`.
    pub const YELLOW: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFFEB3B), YELLOW_SHADES);
    /// The **amber** ramp: 10 steps, `50` to `900`.
    pub const AMBER: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFFC107), AMBER_SHADES);
    /// The **orange** ramp: 10 steps, `50` to `900`.
    pub const ORANGE: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFF9800), ORANGE_SHADES);
    /// The **deep orange** ramp: 10 steps, `50` to `900`.
    pub const DEEP_ORANGE: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFF5722), DEEP_ORANGE_SHADES);
    /// The **brown** ramp: 10 steps, `50` to `900`.
    pub const BROWN: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF795548), BROWN_SHADES);
    /// The **grey** ramp: 12 steps, `50` to `900`.
    pub const GREY: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF9E9E9E), GREY_SHADES);
    /// The **blue grey** ramp: 10 steps, `50` to `900`.
    pub const BLUE_GREY: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF607D8B), BLUE_GREY_SHADES);
    /// The **red** accent ramp: 4 steps, `100` to `700`.
    pub const RED_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFF5252), RED_ACCENT_SHADES);
    /// The **pink** accent ramp: 4 steps, `100` to `700`.
    pub const PINK_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFF4081), PINK_ACCENT_SHADES);
    /// The **purple** accent ramp: 4 steps, `100` to `700`.
    pub const PURPLE_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFE040FB), PURPLE_ACCENT_SHADES);
    /// The **deep purple** accent ramp: 4 steps, `100` to `700`.
    pub const DEEP_PURPLE_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF7C4DFF), DEEP_PURPLE_ACCENT_SHADES);
    /// The **indigo** accent ramp: 4 steps, `100` to `700`.
    pub const INDIGO_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF536DFE), INDIGO_ACCENT_SHADES);
    /// The **blue** accent ramp: 4 steps, `100` to `700`.
    pub const BLUE_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF448AFF), BLUE_ACCENT_SHADES);
    /// The **light blue** accent ramp: 4 steps, `100` to `700`.
    pub const LIGHT_BLUE_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF40C4FF), LIGHT_BLUE_ACCENT_SHADES);
    /// The **cyan** accent ramp: 4 steps, `100` to `700`.
    pub const CYAN_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF18FFFF), CYAN_ACCENT_SHADES);
    /// The **teal** accent ramp: 4 steps, `100` to `700`.
    pub const TEAL_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF64FFDA), TEAL_ACCENT_SHADES);
    /// The **green** accent ramp: 4 steps, `100` to `700`.
    pub const GREEN_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFF69F0AE), GREEN_ACCENT_SHADES);
    /// The **light green** accent ramp: 4 steps, `100` to `700`.
    pub const LIGHT_GREEN_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFB2FF59), LIGHT_GREEN_ACCENT_SHADES);
    /// The **lime** accent ramp: 4 steps, `100` to `700`.
    pub const LIME_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFEEFF41), LIME_ACCENT_SHADES);
    /// The **yellow** accent ramp: 4 steps, `100` to `700`.
    pub const YELLOW_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFFFF00), YELLOW_ACCENT_SHADES);
    /// The **amber** accent ramp: 4 steps, `100` to `700`.
    pub const AMBER_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFFD740), AMBER_ACCENT_SHADES);
    /// The **orange** accent ramp: 4 steps, `100` to `700`.
    pub const ORANGE_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFFAB40), ORANGE_ACCENT_SHADES);
    /// The **deep orange** accent ramp: 4 steps, `100` to `700`.
    pub const DEEP_ORANGE_ACCENT: MaterialColor =
        MaterialColor::new(Color::from_argb_u32(0xFFFF6E40), DEEP_ORANGE_ACCENT_SHADES);
}

/// The **named colours** of the material palette, as plain [`Color`] constants.
///
/// A family's bare name is its primary step: `Colors::RED` is `Colors::RED_500`, and
/// `Colors::RED_ACCENT` is the accent family's `200`. Every other step is spelled out —
/// `Colors::RED_50` through `Colors::RED_900`, `Colors::RED_ACCENT_100` through
/// `Colors::RED_ACCENT_700`.
///
/// The neutral constants carry the palette's standard opacities: `Colors::BLACK54` is
/// black at 54% — the value the design system uses for secondary text, not a number to
/// re-derive by hand.
///
/// ```
/// use frus_core::Colors;
///
/// assert_eq!(Colors::RED, Colors::RED_500);
/// assert_eq!(Colors::BLACK.a, 1.0);
/// assert!(Colors::BLACK54.a < 1.0);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Colors;

impl Colors {
    /// Fully transparent — a colour that paints nothing.
    pub const TRANSPARENT: Color = Color::from_argb_u32(0x00000000);

    /// Opaque black.
    pub const BLACK: Color = Color::from_argb_u32(0xFF000000);

    /// Black at 87% opacity.
    pub const BLACK87: Color = Color::from_argb_u32(0xDD000000);

    /// Black at 54% opacity.
    pub const BLACK54: Color = Color::from_argb_u32(0x8A000000);

    /// Black at 45% opacity.
    pub const BLACK45: Color = Color::from_argb_u32(0x73000000);

    /// Black at 38% opacity.
    pub const BLACK38: Color = Color::from_argb_u32(0x61000000);

    /// Black at 26% opacity.
    pub const BLACK26: Color = Color::from_argb_u32(0x42000000);

    /// Black at 12% opacity.
    pub const BLACK12: Color = Color::from_argb_u32(0x1F000000);

    /// Opaque white.
    pub const WHITE: Color = Color::from_argb_u32(0xFFFFFFFF);

    /// White at 70% opacity.
    pub const WHITE70: Color = Color::from_argb_u32(0xB3FFFFFF);

    /// White at 60% opacity.
    pub const WHITE60: Color = Color::from_argb_u32(0x99FFFFFF);

    /// White at 54% opacity.
    pub const WHITE54: Color = Color::from_argb_u32(0x8AFFFFFF);

    /// White at 38% opacity.
    pub const WHITE38: Color = Color::from_argb_u32(0x62FFFFFF);

    /// White at 30% opacity.
    pub const WHITE30: Color = Color::from_argb_u32(0x4DFFFFFF);

    /// White at 24% opacity.
    pub const WHITE24: Color = Color::from_argb_u32(0x3DFFFFFF);

    /// White at 12% opacity.
    pub const WHITE12: Color = Color::from_argb_u32(0x1FFFFFFF);

    /// White at 10% opacity.
    pub const WHITE10: Color = Color::from_argb_u32(0x1AFFFFFF);

    /// **Red** — the family's primary step (`500`).
    pub const RED: Color = Color::from_argb_u32(0xFFF44336);
    /// Red, step `50`.
    pub const RED_50: Color = Color::from_argb_u32(0xFFFFEBEE);
    /// Red, step `100`.
    pub const RED_100: Color = Color::from_argb_u32(0xFFFFCDD2);
    /// Red, step `200`.
    pub const RED_200: Color = Color::from_argb_u32(0xFFEF9A9A);
    /// Red, step `300`.
    pub const RED_300: Color = Color::from_argb_u32(0xFFE57373);
    /// Red, step `400`.
    pub const RED_400: Color = Color::from_argb_u32(0xFFEF5350);
    /// Red, step `500`.
    pub const RED_500: Color = Color::from_argb_u32(0xFFF44336);
    /// Red, step `600`.
    pub const RED_600: Color = Color::from_argb_u32(0xFFE53935);
    /// Red, step `700`.
    pub const RED_700: Color = Color::from_argb_u32(0xFFD32F2F);
    /// Red, step `800`.
    pub const RED_800: Color = Color::from_argb_u32(0xFFC62828);
    /// Red, step `900`.
    pub const RED_900: Color = Color::from_argb_u32(0xFFB71C1C);

    /// **Pink** — the family's primary step (`500`).
    pub const PINK: Color = Color::from_argb_u32(0xFFE91E63);
    /// Pink, step `50`.
    pub const PINK_50: Color = Color::from_argb_u32(0xFFFCE4EC);
    /// Pink, step `100`.
    pub const PINK_100: Color = Color::from_argb_u32(0xFFF8BBD0);
    /// Pink, step `200`.
    pub const PINK_200: Color = Color::from_argb_u32(0xFFF48FB1);
    /// Pink, step `300`.
    pub const PINK_300: Color = Color::from_argb_u32(0xFFF06292);
    /// Pink, step `400`.
    pub const PINK_400: Color = Color::from_argb_u32(0xFFEC407A);
    /// Pink, step `500`.
    pub const PINK_500: Color = Color::from_argb_u32(0xFFE91E63);
    /// Pink, step `600`.
    pub const PINK_600: Color = Color::from_argb_u32(0xFFD81B60);
    /// Pink, step `700`.
    pub const PINK_700: Color = Color::from_argb_u32(0xFFC2185B);
    /// Pink, step `800`.
    pub const PINK_800: Color = Color::from_argb_u32(0xFFAD1457);
    /// Pink, step `900`.
    pub const PINK_900: Color = Color::from_argb_u32(0xFF880E4F);

    /// **Purple** — the family's primary step (`500`).
    pub const PURPLE: Color = Color::from_argb_u32(0xFF9C27B0);
    /// Purple, step `50`.
    pub const PURPLE_50: Color = Color::from_argb_u32(0xFFF3E5F5);
    /// Purple, step `100`.
    pub const PURPLE_100: Color = Color::from_argb_u32(0xFFE1BEE7);
    /// Purple, step `200`.
    pub const PURPLE_200: Color = Color::from_argb_u32(0xFFCE93D8);
    /// Purple, step `300`.
    pub const PURPLE_300: Color = Color::from_argb_u32(0xFFBA68C8);
    /// Purple, step `400`.
    pub const PURPLE_400: Color = Color::from_argb_u32(0xFFAB47BC);
    /// Purple, step `500`.
    pub const PURPLE_500: Color = Color::from_argb_u32(0xFF9C27B0);
    /// Purple, step `600`.
    pub const PURPLE_600: Color = Color::from_argb_u32(0xFF8E24AA);
    /// Purple, step `700`.
    pub const PURPLE_700: Color = Color::from_argb_u32(0xFF7B1FA2);
    /// Purple, step `800`.
    pub const PURPLE_800: Color = Color::from_argb_u32(0xFF6A1B9A);
    /// Purple, step `900`.
    pub const PURPLE_900: Color = Color::from_argb_u32(0xFF4A148C);

    /// **Deep purple** — the family's primary step (`500`).
    pub const DEEP_PURPLE: Color = Color::from_argb_u32(0xFF673AB7);
    /// Deep purple, step `50`.
    pub const DEEP_PURPLE_50: Color = Color::from_argb_u32(0xFFEDE7F6);
    /// Deep purple, step `100`.
    pub const DEEP_PURPLE_100: Color = Color::from_argb_u32(0xFFD1C4E9);
    /// Deep purple, step `200`.
    pub const DEEP_PURPLE_200: Color = Color::from_argb_u32(0xFFB39DDB);
    /// Deep purple, step `300`.
    pub const DEEP_PURPLE_300: Color = Color::from_argb_u32(0xFF9575CD);
    /// Deep purple, step `400`.
    pub const DEEP_PURPLE_400: Color = Color::from_argb_u32(0xFF7E57C2);
    /// Deep purple, step `500`.
    pub const DEEP_PURPLE_500: Color = Color::from_argb_u32(0xFF673AB7);
    /// Deep purple, step `600`.
    pub const DEEP_PURPLE_600: Color = Color::from_argb_u32(0xFF5E35B1);
    /// Deep purple, step `700`.
    pub const DEEP_PURPLE_700: Color = Color::from_argb_u32(0xFF512DA8);
    /// Deep purple, step `800`.
    pub const DEEP_PURPLE_800: Color = Color::from_argb_u32(0xFF4527A0);
    /// Deep purple, step `900`.
    pub const DEEP_PURPLE_900: Color = Color::from_argb_u32(0xFF311B92);

    /// **Indigo** — the family's primary step (`500`).
    pub const INDIGO: Color = Color::from_argb_u32(0xFF3F51B5);
    /// Indigo, step `50`.
    pub const INDIGO_50: Color = Color::from_argb_u32(0xFFE8EAF6);
    /// Indigo, step `100`.
    pub const INDIGO_100: Color = Color::from_argb_u32(0xFFC5CAE9);
    /// Indigo, step `200`.
    pub const INDIGO_200: Color = Color::from_argb_u32(0xFF9FA8DA);
    /// Indigo, step `300`.
    pub const INDIGO_300: Color = Color::from_argb_u32(0xFF7986CB);
    /// Indigo, step `400`.
    pub const INDIGO_400: Color = Color::from_argb_u32(0xFF5C6BC0);
    /// Indigo, step `500`.
    pub const INDIGO_500: Color = Color::from_argb_u32(0xFF3F51B5);
    /// Indigo, step `600`.
    pub const INDIGO_600: Color = Color::from_argb_u32(0xFF3949AB);
    /// Indigo, step `700`.
    pub const INDIGO_700: Color = Color::from_argb_u32(0xFF303F9F);
    /// Indigo, step `800`.
    pub const INDIGO_800: Color = Color::from_argb_u32(0xFF283593);
    /// Indigo, step `900`.
    pub const INDIGO_900: Color = Color::from_argb_u32(0xFF1A237E);

    /// **Blue** — the family's primary step (`500`).
    pub const BLUE: Color = Color::from_argb_u32(0xFF2196F3);
    /// Blue, step `50`.
    pub const BLUE_50: Color = Color::from_argb_u32(0xFFE3F2FD);
    /// Blue, step `100`.
    pub const BLUE_100: Color = Color::from_argb_u32(0xFFBBDEFB);
    /// Blue, step `200`.
    pub const BLUE_200: Color = Color::from_argb_u32(0xFF90CAF9);
    /// Blue, step `300`.
    pub const BLUE_300: Color = Color::from_argb_u32(0xFF64B5F6);
    /// Blue, step `400`.
    pub const BLUE_400: Color = Color::from_argb_u32(0xFF42A5F5);
    /// Blue, step `500`.
    pub const BLUE_500: Color = Color::from_argb_u32(0xFF2196F3);
    /// Blue, step `600`.
    pub const BLUE_600: Color = Color::from_argb_u32(0xFF1E88E5);
    /// Blue, step `700`.
    pub const BLUE_700: Color = Color::from_argb_u32(0xFF1976D2);
    /// Blue, step `800`.
    pub const BLUE_800: Color = Color::from_argb_u32(0xFF1565C0);
    /// Blue, step `900`.
    pub const BLUE_900: Color = Color::from_argb_u32(0xFF0D47A1);

    /// **Light blue** — the family's primary step (`500`).
    pub const LIGHT_BLUE: Color = Color::from_argb_u32(0xFF03A9F4);
    /// Light blue, step `50`.
    pub const LIGHT_BLUE_50: Color = Color::from_argb_u32(0xFFE1F5FE);
    /// Light blue, step `100`.
    pub const LIGHT_BLUE_100: Color = Color::from_argb_u32(0xFFB3E5FC);
    /// Light blue, step `200`.
    pub const LIGHT_BLUE_200: Color = Color::from_argb_u32(0xFF81D4FA);
    /// Light blue, step `300`.
    pub const LIGHT_BLUE_300: Color = Color::from_argb_u32(0xFF4FC3F7);
    /// Light blue, step `400`.
    pub const LIGHT_BLUE_400: Color = Color::from_argb_u32(0xFF29B6F6);
    /// Light blue, step `500`.
    pub const LIGHT_BLUE_500: Color = Color::from_argb_u32(0xFF03A9F4);
    /// Light blue, step `600`.
    pub const LIGHT_BLUE_600: Color = Color::from_argb_u32(0xFF039BE5);
    /// Light blue, step `700`.
    pub const LIGHT_BLUE_700: Color = Color::from_argb_u32(0xFF0288D1);
    /// Light blue, step `800`.
    pub const LIGHT_BLUE_800: Color = Color::from_argb_u32(0xFF0277BD);
    /// Light blue, step `900`.
    pub const LIGHT_BLUE_900: Color = Color::from_argb_u32(0xFF01579B);

    /// **Cyan** — the family's primary step (`500`).
    pub const CYAN: Color = Color::from_argb_u32(0xFF00BCD4);
    /// Cyan, step `50`.
    pub const CYAN_50: Color = Color::from_argb_u32(0xFFE0F7FA);
    /// Cyan, step `100`.
    pub const CYAN_100: Color = Color::from_argb_u32(0xFFB2EBF2);
    /// Cyan, step `200`.
    pub const CYAN_200: Color = Color::from_argb_u32(0xFF80DEEA);
    /// Cyan, step `300`.
    pub const CYAN_300: Color = Color::from_argb_u32(0xFF4DD0E1);
    /// Cyan, step `400`.
    pub const CYAN_400: Color = Color::from_argb_u32(0xFF26C6DA);
    /// Cyan, step `500`.
    pub const CYAN_500: Color = Color::from_argb_u32(0xFF00BCD4);
    /// Cyan, step `600`.
    pub const CYAN_600: Color = Color::from_argb_u32(0xFF00ACC1);
    /// Cyan, step `700`.
    pub const CYAN_700: Color = Color::from_argb_u32(0xFF0097A7);
    /// Cyan, step `800`.
    pub const CYAN_800: Color = Color::from_argb_u32(0xFF00838F);
    /// Cyan, step `900`.
    pub const CYAN_900: Color = Color::from_argb_u32(0xFF006064);

    /// **Teal** — the family's primary step (`500`).
    pub const TEAL: Color = Color::from_argb_u32(0xFF009688);
    /// Teal, step `50`.
    pub const TEAL_50: Color = Color::from_argb_u32(0xFFE0F2F1);
    /// Teal, step `100`.
    pub const TEAL_100: Color = Color::from_argb_u32(0xFFB2DFDB);
    /// Teal, step `200`.
    pub const TEAL_200: Color = Color::from_argb_u32(0xFF80CBC4);
    /// Teal, step `300`.
    pub const TEAL_300: Color = Color::from_argb_u32(0xFF4DB6AC);
    /// Teal, step `400`.
    pub const TEAL_400: Color = Color::from_argb_u32(0xFF26A69A);
    /// Teal, step `500`.
    pub const TEAL_500: Color = Color::from_argb_u32(0xFF009688);
    /// Teal, step `600`.
    pub const TEAL_600: Color = Color::from_argb_u32(0xFF00897B);
    /// Teal, step `700`.
    pub const TEAL_700: Color = Color::from_argb_u32(0xFF00796B);
    /// Teal, step `800`.
    pub const TEAL_800: Color = Color::from_argb_u32(0xFF00695C);
    /// Teal, step `900`.
    pub const TEAL_900: Color = Color::from_argb_u32(0xFF004D40);

    /// **Green** — the family's primary step (`500`).
    pub const GREEN: Color = Color::from_argb_u32(0xFF4CAF50);
    /// Green, step `50`.
    pub const GREEN_50: Color = Color::from_argb_u32(0xFFE8F5E9);
    /// Green, step `100`.
    pub const GREEN_100: Color = Color::from_argb_u32(0xFFC8E6C9);
    /// Green, step `200`.
    pub const GREEN_200: Color = Color::from_argb_u32(0xFFA5D6A7);
    /// Green, step `300`.
    pub const GREEN_300: Color = Color::from_argb_u32(0xFF81C784);
    /// Green, step `400`.
    pub const GREEN_400: Color = Color::from_argb_u32(0xFF66BB6A);
    /// Green, step `500`.
    pub const GREEN_500: Color = Color::from_argb_u32(0xFF4CAF50);
    /// Green, step `600`.
    pub const GREEN_600: Color = Color::from_argb_u32(0xFF43A047);
    /// Green, step `700`.
    pub const GREEN_700: Color = Color::from_argb_u32(0xFF388E3C);
    /// Green, step `800`.
    pub const GREEN_800: Color = Color::from_argb_u32(0xFF2E7D32);
    /// Green, step `900`.
    pub const GREEN_900: Color = Color::from_argb_u32(0xFF1B5E20);

    /// **Light green** — the family's primary step (`500`).
    pub const LIGHT_GREEN: Color = Color::from_argb_u32(0xFF8BC34A);
    /// Light green, step `50`.
    pub const LIGHT_GREEN_50: Color = Color::from_argb_u32(0xFFF1F8E9);
    /// Light green, step `100`.
    pub const LIGHT_GREEN_100: Color = Color::from_argb_u32(0xFFDCEDC8);
    /// Light green, step `200`.
    pub const LIGHT_GREEN_200: Color = Color::from_argb_u32(0xFFC5E1A5);
    /// Light green, step `300`.
    pub const LIGHT_GREEN_300: Color = Color::from_argb_u32(0xFFAED581);
    /// Light green, step `400`.
    pub const LIGHT_GREEN_400: Color = Color::from_argb_u32(0xFF9CCC65);
    /// Light green, step `500`.
    pub const LIGHT_GREEN_500: Color = Color::from_argb_u32(0xFF8BC34A);
    /// Light green, step `600`.
    pub const LIGHT_GREEN_600: Color = Color::from_argb_u32(0xFF7CB342);
    /// Light green, step `700`.
    pub const LIGHT_GREEN_700: Color = Color::from_argb_u32(0xFF689F38);
    /// Light green, step `800`.
    pub const LIGHT_GREEN_800: Color = Color::from_argb_u32(0xFF558B2F);
    /// Light green, step `900`.
    pub const LIGHT_GREEN_900: Color = Color::from_argb_u32(0xFF33691E);

    /// **Lime** — the family's primary step (`500`).
    pub const LIME: Color = Color::from_argb_u32(0xFFCDDC39);
    /// Lime, step `50`.
    pub const LIME_50: Color = Color::from_argb_u32(0xFFF9FBE7);
    /// Lime, step `100`.
    pub const LIME_100: Color = Color::from_argb_u32(0xFFF0F4C3);
    /// Lime, step `200`.
    pub const LIME_200: Color = Color::from_argb_u32(0xFFE6EE9C);
    /// Lime, step `300`.
    pub const LIME_300: Color = Color::from_argb_u32(0xFFDCE775);
    /// Lime, step `400`.
    pub const LIME_400: Color = Color::from_argb_u32(0xFFD4E157);
    /// Lime, step `500`.
    pub const LIME_500: Color = Color::from_argb_u32(0xFFCDDC39);
    /// Lime, step `600`.
    pub const LIME_600: Color = Color::from_argb_u32(0xFFC0CA33);
    /// Lime, step `700`.
    pub const LIME_700: Color = Color::from_argb_u32(0xFFAFB42B);
    /// Lime, step `800`.
    pub const LIME_800: Color = Color::from_argb_u32(0xFF9E9D24);
    /// Lime, step `900`.
    pub const LIME_900: Color = Color::from_argb_u32(0xFF827717);

    /// **Yellow** — the family's primary step (`500`).
    pub const YELLOW: Color = Color::from_argb_u32(0xFFFFEB3B);
    /// Yellow, step `50`.
    pub const YELLOW_50: Color = Color::from_argb_u32(0xFFFFFDE7);
    /// Yellow, step `100`.
    pub const YELLOW_100: Color = Color::from_argb_u32(0xFFFFF9C4);
    /// Yellow, step `200`.
    pub const YELLOW_200: Color = Color::from_argb_u32(0xFFFFF59D);
    /// Yellow, step `300`.
    pub const YELLOW_300: Color = Color::from_argb_u32(0xFFFFF176);
    /// Yellow, step `400`.
    pub const YELLOW_400: Color = Color::from_argb_u32(0xFFFFEE58);
    /// Yellow, step `500`.
    pub const YELLOW_500: Color = Color::from_argb_u32(0xFFFFEB3B);
    /// Yellow, step `600`.
    pub const YELLOW_600: Color = Color::from_argb_u32(0xFFFDD835);
    /// Yellow, step `700`.
    pub const YELLOW_700: Color = Color::from_argb_u32(0xFFFBC02D);
    /// Yellow, step `800`.
    pub const YELLOW_800: Color = Color::from_argb_u32(0xFFF9A825);
    /// Yellow, step `900`.
    pub const YELLOW_900: Color = Color::from_argb_u32(0xFFF57F17);

    /// **Amber** — the family's primary step (`500`).
    pub const AMBER: Color = Color::from_argb_u32(0xFFFFC107);
    /// Amber, step `50`.
    pub const AMBER_50: Color = Color::from_argb_u32(0xFFFFF8E1);
    /// Amber, step `100`.
    pub const AMBER_100: Color = Color::from_argb_u32(0xFFFFECB3);
    /// Amber, step `200`.
    pub const AMBER_200: Color = Color::from_argb_u32(0xFFFFE082);
    /// Amber, step `300`.
    pub const AMBER_300: Color = Color::from_argb_u32(0xFFFFD54F);
    /// Amber, step `400`.
    pub const AMBER_400: Color = Color::from_argb_u32(0xFFFFCA28);
    /// Amber, step `500`.
    pub const AMBER_500: Color = Color::from_argb_u32(0xFFFFC107);
    /// Amber, step `600`.
    pub const AMBER_600: Color = Color::from_argb_u32(0xFFFFB300);
    /// Amber, step `700`.
    pub const AMBER_700: Color = Color::from_argb_u32(0xFFFFA000);
    /// Amber, step `800`.
    pub const AMBER_800: Color = Color::from_argb_u32(0xFFFF8F00);
    /// Amber, step `900`.
    pub const AMBER_900: Color = Color::from_argb_u32(0xFFFF6F00);

    /// **Orange** — the family's primary step (`500`).
    pub const ORANGE: Color = Color::from_argb_u32(0xFFFF9800);
    /// Orange, step `50`.
    pub const ORANGE_50: Color = Color::from_argb_u32(0xFFFFF3E0);
    /// Orange, step `100`.
    pub const ORANGE_100: Color = Color::from_argb_u32(0xFFFFE0B2);
    /// Orange, step `200`.
    pub const ORANGE_200: Color = Color::from_argb_u32(0xFFFFCC80);
    /// Orange, step `300`.
    pub const ORANGE_300: Color = Color::from_argb_u32(0xFFFFB74D);
    /// Orange, step `400`.
    pub const ORANGE_400: Color = Color::from_argb_u32(0xFFFFA726);
    /// Orange, step `500`.
    pub const ORANGE_500: Color = Color::from_argb_u32(0xFFFF9800);
    /// Orange, step `600`.
    pub const ORANGE_600: Color = Color::from_argb_u32(0xFFFB8C00);
    /// Orange, step `700`.
    pub const ORANGE_700: Color = Color::from_argb_u32(0xFFF57C00);
    /// Orange, step `800`.
    pub const ORANGE_800: Color = Color::from_argb_u32(0xFFEF6C00);
    /// Orange, step `900`.
    pub const ORANGE_900: Color = Color::from_argb_u32(0xFFE65100);

    /// **Deep orange** — the family's primary step (`500`).
    pub const DEEP_ORANGE: Color = Color::from_argb_u32(0xFFFF5722);
    /// Deep orange, step `50`.
    pub const DEEP_ORANGE_50: Color = Color::from_argb_u32(0xFFFBE9E7);
    /// Deep orange, step `100`.
    pub const DEEP_ORANGE_100: Color = Color::from_argb_u32(0xFFFFCCBC);
    /// Deep orange, step `200`.
    pub const DEEP_ORANGE_200: Color = Color::from_argb_u32(0xFFFFAB91);
    /// Deep orange, step `300`.
    pub const DEEP_ORANGE_300: Color = Color::from_argb_u32(0xFFFF8A65);
    /// Deep orange, step `400`.
    pub const DEEP_ORANGE_400: Color = Color::from_argb_u32(0xFFFF7043);
    /// Deep orange, step `500`.
    pub const DEEP_ORANGE_500: Color = Color::from_argb_u32(0xFFFF5722);
    /// Deep orange, step `600`.
    pub const DEEP_ORANGE_600: Color = Color::from_argb_u32(0xFFF4511E);
    /// Deep orange, step `700`.
    pub const DEEP_ORANGE_700: Color = Color::from_argb_u32(0xFFE64A19);
    /// Deep orange, step `800`.
    pub const DEEP_ORANGE_800: Color = Color::from_argb_u32(0xFFD84315);
    /// Deep orange, step `900`.
    pub const DEEP_ORANGE_900: Color = Color::from_argb_u32(0xFFBF360C);

    /// **Brown** — the family's primary step (`500`).
    pub const BROWN: Color = Color::from_argb_u32(0xFF795548);
    /// Brown, step `50`.
    pub const BROWN_50: Color = Color::from_argb_u32(0xFFEFEBE9);
    /// Brown, step `100`.
    pub const BROWN_100: Color = Color::from_argb_u32(0xFFD7CCC8);
    /// Brown, step `200`.
    pub const BROWN_200: Color = Color::from_argb_u32(0xFFBCAAA4);
    /// Brown, step `300`.
    pub const BROWN_300: Color = Color::from_argb_u32(0xFFA1887F);
    /// Brown, step `400`.
    pub const BROWN_400: Color = Color::from_argb_u32(0xFF8D6E63);
    /// Brown, step `500`.
    pub const BROWN_500: Color = Color::from_argb_u32(0xFF795548);
    /// Brown, step `600`.
    pub const BROWN_600: Color = Color::from_argb_u32(0xFF6D4C41);
    /// Brown, step `700`.
    pub const BROWN_700: Color = Color::from_argb_u32(0xFF5D4037);
    /// Brown, step `800`.
    pub const BROWN_800: Color = Color::from_argb_u32(0xFF4E342E);
    /// Brown, step `900`.
    pub const BROWN_900: Color = Color::from_argb_u32(0xFF3E2723);

    /// **Grey** — the family's primary step (`500`).
    pub const GREY: Color = Color::from_argb_u32(0xFF9E9E9E);
    /// Grey, step `50`.
    pub const GREY_50: Color = Color::from_argb_u32(0xFFFAFAFA);
    /// Grey, step `100`.
    pub const GREY_100: Color = Color::from_argb_u32(0xFFF5F5F5);
    /// Grey, step `200`.
    pub const GREY_200: Color = Color::from_argb_u32(0xFFEEEEEE);
    /// Grey, step `300`.
    pub const GREY_300: Color = Color::from_argb_u32(0xFFE0E0E0);
    /// Grey, step `350`.
    pub const GREY_350: Color = Color::from_argb_u32(0xFFD6D6D6);
    /// Grey, step `400`.
    pub const GREY_400: Color = Color::from_argb_u32(0xFFBDBDBD);
    /// Grey, step `500`.
    pub const GREY_500: Color = Color::from_argb_u32(0xFF9E9E9E);
    /// Grey, step `600`.
    pub const GREY_600: Color = Color::from_argb_u32(0xFF757575);
    /// Grey, step `700`.
    pub const GREY_700: Color = Color::from_argb_u32(0xFF616161);
    /// Grey, step `800`.
    pub const GREY_800: Color = Color::from_argb_u32(0xFF424242);
    /// Grey, step `850`.
    pub const GREY_850: Color = Color::from_argb_u32(0xFF303030);
    /// Grey, step `900`.
    pub const GREY_900: Color = Color::from_argb_u32(0xFF212121);

    /// **Blue grey** — the family's primary step (`500`).
    pub const BLUE_GREY: Color = Color::from_argb_u32(0xFF607D8B);
    /// Blue grey, step `50`.
    pub const BLUE_GREY_50: Color = Color::from_argb_u32(0xFFECEFF1);
    /// Blue grey, step `100`.
    pub const BLUE_GREY_100: Color = Color::from_argb_u32(0xFFCFD8DC);
    /// Blue grey, step `200`.
    pub const BLUE_GREY_200: Color = Color::from_argb_u32(0xFFB0BEC5);
    /// Blue grey, step `300`.
    pub const BLUE_GREY_300: Color = Color::from_argb_u32(0xFF90A4AE);
    /// Blue grey, step `400`.
    pub const BLUE_GREY_400: Color = Color::from_argb_u32(0xFF78909C);
    /// Blue grey, step `500`.
    pub const BLUE_GREY_500: Color = Color::from_argb_u32(0xFF607D8B);
    /// Blue grey, step `600`.
    pub const BLUE_GREY_600: Color = Color::from_argb_u32(0xFF546E7A);
    /// Blue grey, step `700`.
    pub const BLUE_GREY_700: Color = Color::from_argb_u32(0xFF455A64);
    /// Blue grey, step `800`.
    pub const BLUE_GREY_800: Color = Color::from_argb_u32(0xFF37474F);
    /// Blue grey, step `900`.
    pub const BLUE_GREY_900: Color = Color::from_argb_u32(0xFF263238);

    /// **Red accent** — the family's primary step (`200`).
    pub const RED_ACCENT: Color = Color::from_argb_u32(0xFFFF5252);
    /// Red accent, step `100`.
    pub const RED_ACCENT_100: Color = Color::from_argb_u32(0xFFFF8A80);
    /// Red accent, step `200`.
    pub const RED_ACCENT_200: Color = Color::from_argb_u32(0xFFFF5252);
    /// Red accent, step `400`.
    pub const RED_ACCENT_400: Color = Color::from_argb_u32(0xFFFF1744);
    /// Red accent, step `700`.
    pub const RED_ACCENT_700: Color = Color::from_argb_u32(0xFFD50000);

    /// **Pink accent** — the family's primary step (`200`).
    pub const PINK_ACCENT: Color = Color::from_argb_u32(0xFFFF4081);
    /// Pink accent, step `100`.
    pub const PINK_ACCENT_100: Color = Color::from_argb_u32(0xFFFF80AB);
    /// Pink accent, step `200`.
    pub const PINK_ACCENT_200: Color = Color::from_argb_u32(0xFFFF4081);
    /// Pink accent, step `400`.
    pub const PINK_ACCENT_400: Color = Color::from_argb_u32(0xFFF50057);
    /// Pink accent, step `700`.
    pub const PINK_ACCENT_700: Color = Color::from_argb_u32(0xFFC51162);

    /// **Purple accent** — the family's primary step (`200`).
    pub const PURPLE_ACCENT: Color = Color::from_argb_u32(0xFFE040FB);
    /// Purple accent, step `100`.
    pub const PURPLE_ACCENT_100: Color = Color::from_argb_u32(0xFFEA80FC);
    /// Purple accent, step `200`.
    pub const PURPLE_ACCENT_200: Color = Color::from_argb_u32(0xFFE040FB);
    /// Purple accent, step `400`.
    pub const PURPLE_ACCENT_400: Color = Color::from_argb_u32(0xFFD500F9);
    /// Purple accent, step `700`.
    pub const PURPLE_ACCENT_700: Color = Color::from_argb_u32(0xFFAA00FF);

    /// **Deep purple accent** — the family's primary step (`200`).
    pub const DEEP_PURPLE_ACCENT: Color = Color::from_argb_u32(0xFF7C4DFF);
    /// Deep purple accent, step `100`.
    pub const DEEP_PURPLE_ACCENT_100: Color = Color::from_argb_u32(0xFFB388FF);
    /// Deep purple accent, step `200`.
    pub const DEEP_PURPLE_ACCENT_200: Color = Color::from_argb_u32(0xFF7C4DFF);
    /// Deep purple accent, step `400`.
    pub const DEEP_PURPLE_ACCENT_400: Color = Color::from_argb_u32(0xFF651FFF);
    /// Deep purple accent, step `700`.
    pub const DEEP_PURPLE_ACCENT_700: Color = Color::from_argb_u32(0xFF6200EA);

    /// **Indigo accent** — the family's primary step (`200`).
    pub const INDIGO_ACCENT: Color = Color::from_argb_u32(0xFF536DFE);
    /// Indigo accent, step `100`.
    pub const INDIGO_ACCENT_100: Color = Color::from_argb_u32(0xFF8C9EFF);
    /// Indigo accent, step `200`.
    pub const INDIGO_ACCENT_200: Color = Color::from_argb_u32(0xFF536DFE);
    /// Indigo accent, step `400`.
    pub const INDIGO_ACCENT_400: Color = Color::from_argb_u32(0xFF3D5AFE);
    /// Indigo accent, step `700`.
    pub const INDIGO_ACCENT_700: Color = Color::from_argb_u32(0xFF304FFE);

    /// **Blue accent** — the family's primary step (`200`).
    pub const BLUE_ACCENT: Color = Color::from_argb_u32(0xFF448AFF);
    /// Blue accent, step `100`.
    pub const BLUE_ACCENT_100: Color = Color::from_argb_u32(0xFF82B1FF);
    /// Blue accent, step `200`.
    pub const BLUE_ACCENT_200: Color = Color::from_argb_u32(0xFF448AFF);
    /// Blue accent, step `400`.
    pub const BLUE_ACCENT_400: Color = Color::from_argb_u32(0xFF2979FF);
    /// Blue accent, step `700`.
    pub const BLUE_ACCENT_700: Color = Color::from_argb_u32(0xFF2962FF);

    /// **Light blue accent** — the family's primary step (`200`).
    pub const LIGHT_BLUE_ACCENT: Color = Color::from_argb_u32(0xFF40C4FF);
    /// Light blue accent, step `100`.
    pub const LIGHT_BLUE_ACCENT_100: Color = Color::from_argb_u32(0xFF80D8FF);
    /// Light blue accent, step `200`.
    pub const LIGHT_BLUE_ACCENT_200: Color = Color::from_argb_u32(0xFF40C4FF);
    /// Light blue accent, step `400`.
    pub const LIGHT_BLUE_ACCENT_400: Color = Color::from_argb_u32(0xFF00B0FF);
    /// Light blue accent, step `700`.
    pub const LIGHT_BLUE_ACCENT_700: Color = Color::from_argb_u32(0xFF0091EA);

    /// **Cyan accent** — the family's primary step (`200`).
    pub const CYAN_ACCENT: Color = Color::from_argb_u32(0xFF18FFFF);
    /// Cyan accent, step `100`.
    pub const CYAN_ACCENT_100: Color = Color::from_argb_u32(0xFF84FFFF);
    /// Cyan accent, step `200`.
    pub const CYAN_ACCENT_200: Color = Color::from_argb_u32(0xFF18FFFF);
    /// Cyan accent, step `400`.
    pub const CYAN_ACCENT_400: Color = Color::from_argb_u32(0xFF00E5FF);
    /// Cyan accent, step `700`.
    pub const CYAN_ACCENT_700: Color = Color::from_argb_u32(0xFF00B8D4);

    /// **Teal accent** — the family's primary step (`200`).
    pub const TEAL_ACCENT: Color = Color::from_argb_u32(0xFF64FFDA);
    /// Teal accent, step `100`.
    pub const TEAL_ACCENT_100: Color = Color::from_argb_u32(0xFFA7FFEB);
    /// Teal accent, step `200`.
    pub const TEAL_ACCENT_200: Color = Color::from_argb_u32(0xFF64FFDA);
    /// Teal accent, step `400`.
    pub const TEAL_ACCENT_400: Color = Color::from_argb_u32(0xFF1DE9B6);
    /// Teal accent, step `700`.
    pub const TEAL_ACCENT_700: Color = Color::from_argb_u32(0xFF00BFA5);

    /// **Green accent** — the family's primary step (`200`).
    pub const GREEN_ACCENT: Color = Color::from_argb_u32(0xFF69F0AE);
    /// Green accent, step `100`.
    pub const GREEN_ACCENT_100: Color = Color::from_argb_u32(0xFFB9F6CA);
    /// Green accent, step `200`.
    pub const GREEN_ACCENT_200: Color = Color::from_argb_u32(0xFF69F0AE);
    /// Green accent, step `400`.
    pub const GREEN_ACCENT_400: Color = Color::from_argb_u32(0xFF00E676);
    /// Green accent, step `700`.
    pub const GREEN_ACCENT_700: Color = Color::from_argb_u32(0xFF00C853);

    /// **Light green accent** — the family's primary step (`200`).
    pub const LIGHT_GREEN_ACCENT: Color = Color::from_argb_u32(0xFFB2FF59);
    /// Light green accent, step `100`.
    pub const LIGHT_GREEN_ACCENT_100: Color = Color::from_argb_u32(0xFFCCFF90);
    /// Light green accent, step `200`.
    pub const LIGHT_GREEN_ACCENT_200: Color = Color::from_argb_u32(0xFFB2FF59);
    /// Light green accent, step `400`.
    pub const LIGHT_GREEN_ACCENT_400: Color = Color::from_argb_u32(0xFF76FF03);
    /// Light green accent, step `700`.
    pub const LIGHT_GREEN_ACCENT_700: Color = Color::from_argb_u32(0xFF64DD17);

    /// **Lime accent** — the family's primary step (`200`).
    pub const LIME_ACCENT: Color = Color::from_argb_u32(0xFFEEFF41);
    /// Lime accent, step `100`.
    pub const LIME_ACCENT_100: Color = Color::from_argb_u32(0xFFF4FF81);
    /// Lime accent, step `200`.
    pub const LIME_ACCENT_200: Color = Color::from_argb_u32(0xFFEEFF41);
    /// Lime accent, step `400`.
    pub const LIME_ACCENT_400: Color = Color::from_argb_u32(0xFFC6FF00);
    /// Lime accent, step `700`.
    pub const LIME_ACCENT_700: Color = Color::from_argb_u32(0xFFAEEA00);

    /// **Yellow accent** — the family's primary step (`200`).
    pub const YELLOW_ACCENT: Color = Color::from_argb_u32(0xFFFFFF00);
    /// Yellow accent, step `100`.
    pub const YELLOW_ACCENT_100: Color = Color::from_argb_u32(0xFFFFFF8D);
    /// Yellow accent, step `200`.
    pub const YELLOW_ACCENT_200: Color = Color::from_argb_u32(0xFFFFFF00);
    /// Yellow accent, step `400`.
    pub const YELLOW_ACCENT_400: Color = Color::from_argb_u32(0xFFFFEA00);
    /// Yellow accent, step `700`.
    pub const YELLOW_ACCENT_700: Color = Color::from_argb_u32(0xFFFFD600);

    /// **Amber accent** — the family's primary step (`200`).
    pub const AMBER_ACCENT: Color = Color::from_argb_u32(0xFFFFD740);
    /// Amber accent, step `100`.
    pub const AMBER_ACCENT_100: Color = Color::from_argb_u32(0xFFFFE57F);
    /// Amber accent, step `200`.
    pub const AMBER_ACCENT_200: Color = Color::from_argb_u32(0xFFFFD740);
    /// Amber accent, step `400`.
    pub const AMBER_ACCENT_400: Color = Color::from_argb_u32(0xFFFFC400);
    /// Amber accent, step `700`.
    pub const AMBER_ACCENT_700: Color = Color::from_argb_u32(0xFFFFAB00);

    /// **Orange accent** — the family's primary step (`200`).
    pub const ORANGE_ACCENT: Color = Color::from_argb_u32(0xFFFFAB40);
    /// Orange accent, step `100`.
    pub const ORANGE_ACCENT_100: Color = Color::from_argb_u32(0xFFFFD180);
    /// Orange accent, step `200`.
    pub const ORANGE_ACCENT_200: Color = Color::from_argb_u32(0xFFFFAB40);
    /// Orange accent, step `400`.
    pub const ORANGE_ACCENT_400: Color = Color::from_argb_u32(0xFFFF9100);
    /// Orange accent, step `700`.
    pub const ORANGE_ACCENT_700: Color = Color::from_argb_u32(0xFFFF6D00);

    /// **Deep orange accent** — the family's primary step (`200`).
    pub const DEEP_ORANGE_ACCENT: Color = Color::from_argb_u32(0xFFFF6E40);
    /// Deep orange accent, step `100`.
    pub const DEEP_ORANGE_ACCENT_100: Color = Color::from_argb_u32(0xFFFF9E80);
    /// Deep orange accent, step `200`.
    pub const DEEP_ORANGE_ACCENT_200: Color = Color::from_argb_u32(0xFFFF6E40);
    /// Deep orange accent, step `400`.
    pub const DEEP_ORANGE_ACCENT_400: Color = Color::from_argb_u32(0xFFFF3D00);
    /// Deep orange accent, step `700`.
    pub const DEEP_ORANGE_ACCENT_700: Color = Color::from_argb_u32(0xFFDD2C00);

    /// Every primary ramp, in palette order — the families a colour picker offers.
    pub const PRIMARIES: [MaterialColor; 19] = [
        MaterialColor::RED,
        MaterialColor::PINK,
        MaterialColor::PURPLE,
        MaterialColor::DEEP_PURPLE,
        MaterialColor::INDIGO,
        MaterialColor::BLUE,
        MaterialColor::LIGHT_BLUE,
        MaterialColor::CYAN,
        MaterialColor::TEAL,
        MaterialColor::GREEN,
        MaterialColor::LIGHT_GREEN,
        MaterialColor::LIME,
        MaterialColor::YELLOW,
        MaterialColor::AMBER,
        MaterialColor::ORANGE,
        MaterialColor::DEEP_ORANGE,
        MaterialColor::BROWN,
        MaterialColor::GREY,
        MaterialColor::BLUE_GREY,
    ];

    /// Every accent ramp, in palette order.
    pub const ACCENTS: [MaterialColor; 16] = [
        MaterialColor::RED_ACCENT,
        MaterialColor::PINK_ACCENT,
        MaterialColor::PURPLE_ACCENT,
        MaterialColor::DEEP_PURPLE_ACCENT,
        MaterialColor::INDIGO_ACCENT,
        MaterialColor::BLUE_ACCENT,
        MaterialColor::LIGHT_BLUE_ACCENT,
        MaterialColor::CYAN_ACCENT,
        MaterialColor::TEAL_ACCENT,
        MaterialColor::GREEN_ACCENT,
        MaterialColor::LIGHT_GREEN_ACCENT,
        MaterialColor::LIME_ACCENT,
        MaterialColor::YELLOW_ACCENT,
        MaterialColor::AMBER_ACCENT,
        MaterialColor::ORANGE_ACCENT,
        MaterialColor::DEEP_ORANGE_ACCENT,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every ramp names itself after one of its own steps — `500` for a family, `200`
    /// for an accent. A primary that is *not* in the ramp is the tell of a
    /// transcription slip, and it is the one error a palette can carry silently.
    #[test]
    fn every_primary_is_a_step_of_its_own_ramp() {
        for ramp in Colors::PRIMARIES {
            assert_eq!(ramp.shade(500), Some(ramp.primary()));
        }
        for ramp in Colors::ACCENTS {
            assert_eq!(ramp.shade(200), Some(ramp.primary()));
        }
    }

    /// The bare constant and the `500` constant are one colour, so `Colors::RED` may be
    /// written wherever `Colors::RED_500` is meant, and the other way round.
    #[test]
    fn the_bare_name_is_the_primary_step() {
        assert_eq!(Colors::RED, Colors::RED_500);
        assert_eq!(Colors::BLUE, Colors::BLUE_500);
        assert_eq!(Colors::GREY, Colors::GREY_500);
        assert_eq!(Colors::BLUE_GREY, Colors::BLUE_GREY_500);
        assert_eq!(Colors::RED_ACCENT, Colors::RED_ACCENT_200);
        assert_eq!(Colors::DEEP_ORANGE_ACCENT, Colors::DEEP_ORANGE_ACCENT_200);
    }

    #[test]
    fn ramps_are_ordered_and_have_the_expected_lengths() {
        for ramp in Colors::PRIMARIES.iter().chain(Colors::ACCENTS.iter()) {
            let steps: Vec<u16> = ramp.steps().map(|(s, _)| s).collect();
            assert!(
                steps.windows(2).all(|w| w[0] < w[1]),
                "steps should ascend: {steps:?}"
            );
        }
        assert_eq!(Colors::PRIMARIES.len(), 19);
        assert_eq!(Colors::ACCENTS.len(), 16);
        // Ten steps everywhere but grey, which carries the two half-steps as well.
        assert_eq!(MaterialColor::RED.len(), 10);
        assert_eq!(MaterialColor::GREY.len(), 12);
        assert_eq!(MaterialColor::RED_ACCENT.len(), 4);
    }

    #[test]
    fn a_missing_step_answers_none_rather_than_the_nearest() {
        assert_eq!(MaterialColor::RED.shade(350), None);
        assert_eq!(MaterialColor::RED.shade(0), None);
        assert_eq!(MaterialColor::RED_ACCENT.shade(300), None);
        // …but the caller can ask for a fallback explicitly.
        assert_eq!(MaterialColor::RED.shade_or_primary(350), Colors::RED);
    }

    /// Spot-checks against the specification's own hex codes. Sampling the table is what
    /// catches a whole family shifted by one step, which the structural tests cannot see.
    #[test]
    fn the_values_match_the_specification() {
        assert_eq!(Colors::RED_500, Color::hex("#F44336"));
        assert_eq!(Colors::BLUE_500, Color::hex("#2196F3"));
        assert_eq!(Colors::GREEN_500, Color::hex("#4CAF50"));
        assert_eq!(Colors::AMBER_500, Color::hex("#FFC107"));
        assert_eq!(Colors::BLUE_GREY_900, Color::hex("#263238"));
        assert_eq!(Colors::TEAL_ACCENT_400, Color::hex("#1DE9B6"));
        assert_eq!(Colors::YELLOW_ACCENT_400, Color::hex("#FFEA00"));
        assert_eq!(Colors::YELLOW_ACCENT_700, Color::hex("#FFD600"));
    }

    /// The neutrals carry the palette's opacities. `BLACK54` is *black at 54%*, which is
    /// an alpha and not a grey: painted over anything but white it must let it through.
    #[test]
    fn the_neutrals_are_alphas_not_greys() {
        assert_eq!(Colors::BLACK, Color::rgba(0.0, 0.0, 0.0, 1.0));
        assert_eq!(Colors::WHITE, Color::rgba(1.0, 1.0, 1.0, 1.0));
        assert_eq!(Colors::TRANSPARENT.a, 0.0);
        for (c, pct) in [
            (Colors::BLACK87, 0.87),
            (Colors::BLACK54, 0.54),
            (Colors::BLACK38, 0.38),
            (Colors::BLACK12, 0.12),
        ] {
            assert_eq!((c.r, c.g, c.b), (0.0, 0.0, 0.0), "the RGB stays black");
            assert!((c.a - pct).abs() < 0.01, "{c:?} should be about {pct}");
        }
        for (c, pct) in [
            (Colors::WHITE70, 0.70),
            (Colors::WHITE54, 0.54),
            (Colors::WHITE24, 0.24),
            (Colors::WHITE10, 0.10),
        ] {
            assert_eq!((c.r, c.g, c.b), (1.0, 1.0, 1.0), "the RGB stays white");
            assert!((c.a - pct).abs() < 0.01, "{c:?} should be about {pct}");
        }
    }

    /// A ramp stands in for a single colour without a cast, which is what lets a theme
    /// take either.
    #[test]
    fn a_ramp_converts_to_its_primary() {
        let color: Color = MaterialColor::INDIGO.into();
        assert_eq!(color, Colors::INDIGO);
    }

    /// Within a family the steps darken monotonically: `50` is the palest and `900` the
    /// darkest. This is the property a caller relies on when picking a step for contrast.
    #[test]
    fn steps_darken_as_they_climb() {
        for ramp in Colors::PRIMARIES {
            let lum: Vec<f32> = ramp.steps().map(|(_, c)| c.compute_luminance()).collect();
            for pair in lum.windows(2) {
                assert!(
                    pair[0] >= pair[1] - 0.005,
                    "a higher step should not be lighter: {lum:?}"
                );
            }
        }
    }
}
