//! Size classes (breakpoints) — the foundation of responsive behaviour.
//!
//! A `SizeClass` sorts a width (in **logical** pixels) into one of three bands, in
//! the Material 3 style. Applications and responsive widgets use it to adapt their
//! layout without hand-coding thresholds.

/// A display-width band.
///
/// Thresholds (logical px): `Compact` < 600, `Medium` 600–840, `Expanded` ≥ 840.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeClass {
    /// Phone / narrow window (< 600).
    Compact,
    /// Portrait tablet / medium window (600–840).
    Medium,
    /// Desktop / wide window (≥ 840).
    Expanded,
}

impl SizeClass {
    /// Lower bound (inclusive) of the `Medium` band, in logical px.
    pub const MEDIUM: f32 = 600.0;
    /// Lower bound (inclusive) of the `Expanded` band, in logical px.
    pub const EXPANDED: f32 = 840.0;

    /// The class matching a width (logical px).
    pub fn from_width(width: f32) -> SizeClass {
        if width >= Self::EXPANDED {
            SizeClass::Expanded
        } else if width >= Self::MEDIUM {
            SizeClass::Medium
        } else {
            SizeClass::Compact
        }
    }

    /// The class matching a **height** (logical px), using the same thresholds.
    ///
    /// Useful on the vertical axis: a short window (< 600) is `Compact` in height,
    /// which is a cue to hide labels, shrink margins, and so on.
    pub fn from_height(height: f32) -> SizeClass {
        Self::from_width(height)
    }

    /// Ordinal rank (0 = Compact … 2 = Expanded), handy for comparing bands.
    pub fn rank(self) -> u8 {
        match self {
            SizeClass::Compact => 0,
            SizeClass::Medium => 1,
            SizeClass::Expanded => 2,
        }
    }
}

/// Display orientation, derived from the width-to-height ratio.
///
/// A second **axis** of responsiveness, independent of the size class: the same
/// width can be portrait (phone held upright) or landscape (phone on its side),
/// which sometimes calls for a different layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// Taller than wide, or square: `height >= width`.
    Portrait,
    /// Wider than tall: `width > height`.
    Landscape,
}

impl Orientation {
    /// The orientation of a window of the given dimensions (logical px).
    pub fn from_size(width: f32, height: f32) -> Orientation {
        if width > height {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }

    /// `true` when portrait (taller than wide).
    pub fn is_portrait(self) -> bool {
        self == Orientation::Portrait
    }

    /// `true` when landscape (wider than tall).
    pub fn is_landscape(self) -> bool {
        self == Orientation::Landscape
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds() {
        assert_eq!(SizeClass::from_width(0.0), SizeClass::Compact);
        assert_eq!(SizeClass::from_width(599.0), SizeClass::Compact);
        assert_eq!(SizeClass::from_width(600.0), SizeClass::Medium);
        assert_eq!(SizeClass::from_width(839.0), SizeClass::Medium);
        assert_eq!(SizeClass::from_width(840.0), SizeClass::Expanded);
        assert_eq!(SizeClass::from_width(1920.0), SizeClass::Expanded);
    }

    #[test]
    fn ordering_by_rank() {
        assert!(SizeClass::Compact < SizeClass::Medium);
        assert!(SizeClass::Medium < SizeClass::Expanded);
        assert_eq!(SizeClass::Expanded.rank(), 2);
    }

    #[test]
    fn height_uses_same_thresholds() {
        assert_eq!(SizeClass::from_height(500.0), SizeClass::Compact);
        assert_eq!(SizeClass::from_height(700.0), SizeClass::Medium);
        assert_eq!(SizeClass::from_height(900.0), SizeClass::Expanded);
    }

    #[test]
    fn orientation_from_size() {
        assert_eq!(Orientation::from_size(400.0, 800.0), Orientation::Portrait);
        assert_eq!(Orientation::from_size(800.0, 400.0), Orientation::Landscape);
        // Square → portrait (the `height >= width` convention).
        assert_eq!(Orientation::from_size(500.0, 500.0), Orientation::Portrait);
        assert!(Orientation::from_size(400.0, 800.0).is_portrait());
        assert!(Orientation::from_size(800.0, 400.0).is_landscape());
    }
}
