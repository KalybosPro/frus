//! [`VisualDensity`]: how compact the controls are (`theme_data.dart:3183`, milestone 631).

use frus_core::TargetPlatform;

/// **How compact the controls are** — the reference's `VisualDensity`.
///
/// Two numbers from −4 to 4, one per axis. Each step is four logical pixels on a control's
/// base size: a button at the compact density (−2, −2) is 8 px shorter than at the
/// standard one, and its padding loses 8 px across. Zero is the standard density.
///
/// A theme carries one ([`Theme::visual_density`](crate::Theme::visual_density)). Unset,
/// it follows the theme's platform as the reference's does
/// ([`default_for_platform`](Self::default_for_platform)): standard on phones, compact
/// on the desktops, where a pointer is precise and a screen holds more.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VisualDensity {
    /// Across: negative is narrower.
    pub horizontal: f32,
    /// Down: negative is shorter.
    pub vertical: f32,
}

impl VisualDensity {
    /// The smallest a density goes.
    pub const MINIMUM: f32 = -4.0;
    /// The largest a density goes.
    pub const MAXIMUM: f32 = 4.0;
    /// The design's own size.
    pub const STANDARD: Self = Self {
        horizontal: 0.0,
        vertical: 0.0,
    };
    /// A step tighter than standard on both axes.
    pub const COMFORTABLE: Self = Self {
        horizontal: -1.0,
        vertical: -1.0,
    };
    /// Two steps tighter on both axes: the desktops' default.
    pub const COMPACT: Self = Self {
        horizontal: -2.0,
        vertical: -2.0,
    };

    /// A density of `horizontal` and `vertical`, each kept within
    /// [`MINIMUM`](Self::MINIMUM)..=[`MAXIMUM`](Self::MAXIMUM).
    pub fn new(horizontal: f32, vertical: f32) -> Self {
        Self {
            horizontal: horizontal.clamp(Self::MINIMUM, Self::MAXIMUM),
            vertical: vertical.clamp(Self::MINIMUM, Self::MAXIMUM),
        }
    }

    /// **The density a platform's controls have**: standard on Android, iOS and Fuchsia,
    /// compact on Linux, macOS and Windows (`theme_data.dart:3252`).
    pub const fn default_for_platform(platform: TargetPlatform) -> Self {
        match platform {
            TargetPlatform::Android | TargetPlatform::Ios | TargetPlatform::Fuchsia => {
                Self::STANDARD
            }
            TargetPlatform::Linux | TargetPlatform::MacOs | TargetPlatform::Windows => {
                Self::COMPACT
            }
        }
    }

    /// **What it adds to a control's base size**, in logical pixels: four per step, on
    /// each axis (`theme_data.dart:3307`).
    pub fn base_size_adjustment(self) -> (f32, f32) {
        (self.horizontal * 4.0, self.vertical * 4.0)
    }

    /// **A minimum size, adjusted**: `min_width` and `min_height` moved by the adjustment,
    /// never below zero nor above `max_width` and `max_height` (`theme_data.dart:3332`).
    pub fn effective_min_size(
        self,
        (min_width, min_height): (f32, f32),
        (max_width, max_height): (f32, f32),
    ) -> (f32, f32) {
        let (dx, dy) = self.base_size_adjustment();
        (
            (min_width + dx).clamp(0.0, max_width.max(0.0)),
            (min_height + dy).clamp(0.0, max_height.max(0.0)),
        )
    }

    /// Between `a` and `b` at `t`.
    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        Self::new(
            a.horizontal + (b.horizontal - a.horizontal) * t,
            a.vertical + (b.vertical - a.vertical) * t,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::VisualDensity;
    use frus_core::TargetPlatform as P;

    /// **The reference's densities and the platforms that take them.**
    #[test]
    fn the_platforms_take_the_reference_s_densities() {
        for p in [P::Android, P::Ios, P::Fuchsia] {
            assert_eq!(
                VisualDensity::default_for_platform(p),
                VisualDensity::STANDARD
            );
        }
        for p in [P::Linux, P::MacOs, P::Windows] {
            assert_eq!(
                VisualDensity::default_for_platform(p),
                VisualDensity::COMPACT
            );
        }
        assert_eq!(VisualDensity::COMPACT.base_size_adjustment(), (-8.0, -8.0));
        assert_eq!(
            VisualDensity::COMFORTABLE.base_size_adjustment(),
            (-4.0, -4.0)
        );
        assert_eq!(VisualDensity::new(9.0, -9.0), VisualDensity::new(4.0, -4.0));
    }

    /// **A theme's density follows its platform until it is given one**, and a fade between
    /// two densities passes through the ones between (`theme_data.dart:412`).
    #[test]
    fn a_theme_s_density_follows_its_platform() {
        use crate::Theme;
        let phone = Theme::light().with_platform(P::Android);
        let desk = Theme::light().with_platform(P::Windows);
        assert_eq!(phone.visual_density(), VisualDensity::STANDARD);
        assert_eq!(desk.visual_density(), VisualDensity::COMPACT);
        let told = desk.clone().with_visual_density(VisualDensity::COMFORTABLE);
        assert_eq!(told.visual_density(), VisualDensity::COMFORTABLE);
        assert_eq!(
            told.clone().with_platform(P::Ios).visual_density(),
            VisualDensity::COMFORTABLE,
            "a density given stays given"
        );
        assert_eq!(
            phone.lerp(&desk, 0.4).visual_density,
            None,
            "both follow their platform"
        );
        let half = phone.lerp(&told, 0.5).visual_density();
        assert_eq!(half, VisualDensity::new(-0.5, -0.5));
    }

    /// **A minimum size moves by four pixels a step and stays in its bounds**: a 64 × 40
    /// button at the compact density is at least 56 × 32.
    #[test]
    fn a_minimum_size_moves_by_four_a_step() {
        let compact = VisualDensity::COMPACT;
        assert_eq!(
            compact.effective_min_size((64.0, 40.0), (f32::INFINITY, f32::INFINITY)),
            (56.0, 32.0)
        );
        assert_eq!(
            compact.effective_min_size((4.0, 4.0), (f32::INFINITY, f32::INFINITY)),
            (0.0, 0.0),
            "never below zero"
        );
        assert_eq!(
            VisualDensity::new(4.0, 4.0).effective_min_size((40.0, 40.0), (50.0, 50.0)),
            (50.0, 50.0),
            "never above the maximum"
        );
        let half = VisualDensity::lerp(VisualDensity::STANDARD, compact, 0.5);
        assert_eq!(half, VisualDensity::COMFORTABLE);
    }
}
