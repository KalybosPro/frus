//! **Every public item of `frus-core` is reachable from `frus-widgets`**, and so from `frus`,
//! which re-exports all of it.
//!
//! An application depends on `frus` and nothing else. A type the core makes public but this
//! crate does not re-export cannot be named there — `BoxShape` shipped that way, so a
//! `BoxDecoration` could be built with a circle only by depending on `frus-core` directly.
//! The list is read from the two `lib.rs` files, so a type added to the core and forgotten
//! here fails this test rather than a user.

use std::collections::BTreeSet;

/// Kept out on purpose, each for its reason.
const INTERNAL: &[(&str, &str)] = &[
    // Re-exported under another name, so as not to shadow the widget crate's own.
    (
        "Status",
        "re-exported as `AnimationStatus`; `Status` is the interaction state",
    ),
    (
        "ShaderMask",
        "re-exported as `LayerShaderMask`; `ShaderMask` is the widget",
    ),
    // Read once by `TextStyle::resolved`; a second reader could scale a size twice.
    (
        "text_scale",
        "the ambient scale is read by `TextStyle::resolved` alone",
    ),
    (
        "with_text_scale",
        "set by the media scope, not by an application",
    ),
    // Painter helpers.
    ("fits", "a layout rounding helper"),
    (
        "tile",
        "the gradient shader's tiling, mirrored on the CPU for tests",
    ),
    ("__set_detected_target_platform", "set by the shell"),
];

/// The names a `lib.rs` exports with `pub use krate::path::{..}` or `pub use krate::Name`,
/// from `prefix` (the crate being exported from, as it is written there).
fn exported(source: &str, prefix: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let source: String = source
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut rest = source.as_str();
    while let Some(at) = rest.find("pub use ") {
        rest = &rest[at + "pub use ".len()..];
        let end = rest.find(';').unwrap_or(rest.len());
        let statement = &rest[..end];
        rest = &rest[end..];
        if !statement.starts_with(prefix) {
            continue;
        }
        let listed = match (statement.find('{'), statement.rfind('}')) {
            (Some(open), Some(close)) => &statement[open + 1..close],
            _ => statement.rsplit("::").next().unwrap_or(""),
        };
        for item in listed.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            // `Name as Alias` exports the name under the alias; the original is what the
            // core calls it.
            let original = item.split(" as ").next().unwrap_or(item).trim();
            let last = original.rsplit("::").next().unwrap_or(original);
            names.insert(last.to_string());
        }
    }
    names
}

#[test]
fn every_core_item_is_reachable_from_the_widget_crate() {
    let here = env!("CARGO_MANIFEST_DIR");
    let core = std::fs::read_to_string(format!("{here}/../frus-core/src/lib.rs"))
        .expect("frus-core's lib.rs");
    let widgets = std::fs::read_to_string(format!("{here}/src/lib.rs")).expect("our lib.rs");
    // The core's own modules export from `self` paths: `pub use decoration::{..}`.
    let core_items = exported(&core, "");
    let ours = exported(&widgets, "frus_core");
    let internal: BTreeSet<&str> = INTERNAL.iter().map(|(name, _)| *name).collect();
    let missing: Vec<&String> = core_items
        .iter()
        .filter(|name| !ours.contains(*name) && !internal.contains(name.as_str()))
        // A module re-export (`pub use frus_core::animation`) is a path, not an item.
        .filter(|name| name.chars().next().is_some_and(|c| c.is_alphabetic()))
        .collect();
    assert!(
        missing.is_empty(),
        "frus-core exports these, and frus cannot name them: {missing:?}\n\
         Re-export them in crates/frus-widgets/src/lib.rs, or list them in INTERNAL with a reason."
    );
    // And the list of exceptions holds nothing the core no longer has.
    let stale: Vec<&str> = internal
        .iter()
        .filter(|name| !core_items.contains(**name))
        .copied()
        .collect();
    assert!(
        stale.is_empty(),
        "INTERNAL names what the core no longer exports: {stale:?}"
    );
}

/// The ones that started this, named from the outside as an application names them.
#[test]
fn the_painting_types_can_be_named() {
    use frus_widgets::{
        BlurStyle, BorderSide, BoxBorder, BoxDecoration, BoxShape, DecorationImage, Gradient,
        ImageRepeat, LayerFilter, RadialGradient, ShapeBorder, SweepGradient, TileMode,
    };
    let _ = BoxDecoration::default()
        .shape(BoxShape::Circle)
        .gradient(Gradient::from(RadialGradient::new(vec![])))
        .image(DecorationImage::new(None).repeat(ImageRepeat::Repeat));
    let _ = (
        BlurStyle::Outer,
        BorderSide::NONE,
        TileMode::Mirror,
        LayerFilter::NONE,
        SweepGradient::new(vec![]),
    );
    let _: Option<BoxBorder> = None;
    let _: Option<ShapeBorder> = None;
}
