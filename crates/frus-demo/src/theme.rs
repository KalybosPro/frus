//! The palette: the demonstration seeds, and the theme they produce.

use crate::prelude::*;

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
pub(crate) fn seed_label(app: &TodoApp) -> String {
    match THEME_SEEDS.get(app.seed_index) {
        Some((name, _)) => format!("Seed: {name}"),
        None => "Seed: default".to_string(),
    }
}

/// The "target" theme for the current state (before the fade): the hand-written scheme by
/// default, or one generated from a seed (`from_seed`, HCT).
pub(crate) fn theme_of(app: &TodoApp) -> Theme {
    let theme = match app
        .seed_index
        .checked_sub(1)
        .and_then(|i| THEME_SEEDS.get(i))
    {
        Some((_, seed)) => Theme::from_seed(*seed, !app.light),
        None => {
            if app.light {
                Theme::light()
            } else {
                Theme::dark()
            }
        }
    };
    // The ambient direction: RTL if the user asked for it OR if the current language is
    // written right to left (Arabic). The whole layout mirrors.
    if app.rtl || lang_is_rtl(app.lang) {
        theme.rtl()
    } else {
        theme
    }
}
