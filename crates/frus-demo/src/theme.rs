//! The palette: the demonstration seeds, and the theme they produce.

use frus_widgets::{Color, Theme};

/// Demonstration seeds for the dynamic theme (`from_seed`, HCT).
pub(crate) const THEME_SEEDS: [(&str, Color); 3] = [
    (
        "Blue",
        Color {
            r: 0x42 as f32 / 255.0,
            g: 0x85 as f32 / 255.0,
            b: 0xF4 as f32 / 255.0,
            a: 1.0,
        },
    ),
    (
        "Purple",
        Color {
            r: 0x9C as f32 / 255.0,
            g: 0x27 as f32 / 255.0,
            b: 0xB0 as f32 / 255.0,
            a: 1.0,
        },
    ),
    (
        "Orange",
        Color {
            r: 0xE8 as f32 / 255.0,
            g: 0x71 as f32 / 255.0,
            b: 0x0A as f32 / 255.0,
            a: 1.0,
        },
    ),
];

/// Label of the menu's "seed" action (the **next** seed of the cycle).
pub(crate) fn seed_label(seed_index: usize) -> String {
    match THEME_SEEDS.get(seed_index) {
        Some((name, _)) => format!("Seed: {name}"),
        None => "Seed: default".to_string(),
    }
}

/// The theme at a given **brightness**: the hand-written scheme by default, or one
/// generated from a seed (`from_seed`, HCT).
///
/// It takes the brightness rather than reading the light switch, because since milestone 452
/// the application no longer chooses between the two: it supplies both — a theme and a dark
/// theme — and the framework picks and crosses between them. `mirrored` is the ambient
/// direction: right to left if the reader asked for it or if the language is written that way
/// (Arabic), and the whole layout mirrors.
pub(crate) fn theme_of(seed_index: usize, mirrored: bool, dark: bool) -> Theme {
    let theme = match seed_index.checked_sub(1).and_then(|i| THEME_SEEDS.get(i)) {
        Some((_, seed)) => Theme::from_seed(*seed, dark),
        None => {
            if dark {
                Theme::dark()
            } else {
                Theme::light()
            }
        }
    };
    if mirrored {
        theme.rtl()
    } else {
        theme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_cycle_names_its_stops() {
        // The label names the stop the action switches **to**: from the hand-written scheme
        // (0) it is Blue, and from the last seed it is back to the default.
        assert_eq!(seed_label(0), "Seed: Blue");
        assert_eq!(seed_label(1), "Seed: Purple");
        assert_eq!(seed_label(2), "Seed: Orange");
        assert_eq!(
            seed_label(3),
            "Seed: default",
            "past the end is the default"
        );
    }

    #[test]
    fn a_mirrored_theme_is_the_same_theme_reading_the_other_way() {
        let plain = theme_of(0, false, true);
        let mirrored = theme_of(0, true, true);
        assert_eq!(plain.background, mirrored.background);
        assert_ne!(plain.direction, mirrored.direction);
    }
}
