# Milestone 548 — `frus-core` denies a missing doc comment

Issue #14, continuing the sweep. The fundamental types shared by the whole
framework — geometry, colour, animation, the scene primitives — 12,600 lines
across fourteen modules, all private except `animation`, with the rest reaching
the public API only through selective `pub use` at the crate root.

Unlike the crates done so far, `#![warn(missing_docs)]` found a real, substantial
list: 92 distinct locations across eleven files, almost all struct fields and
enum-variant fields whose containing type already carried a doc comment but whose
individual members didn't. Following the issue's own rule — say what a field is
*for* or its unit, never restate its name — each one is now documented with its
unit (logical pixels, degrees, a fraction in `[-1, 1]`) or its role:

- `animation/curve.rs`, `animation/velocity.rs`: the named fields inside
  `Curve::Cubic`/`CriticalSpring`/`Interval` and `Velocity`'s `x`/`y`.
- `color.rs`: `Color`'s four channels and its three named constants.
- `decoration.rs`: `BorderRadius`'s four corners.
- `filter.rs`: the sigma/radius pairs on `ImageFilter`'s three variants.
- `geometry.rs`, the largest block: `Point`, `Insets`, `InsetsDirectional`,
  `Size`'s fields and constructors, and the nine-plus-nine named anchor constants
  on `Alignment` and `AlignmentDirectional`.
- `hct.rs`: `TonalPalette`'s `hue`/`chroma`.
- `path.rs`: the control points on `QuadTo`/`CubicTo`, and `Stroke`'s fields.
- `scene.rs`: several `Primitive` variant fields that had docs on their siblings
  but not on themselves, plus `SceneBuilder::draw_rect`, which had none at all.
- `shape.rs`: `BorderSide`'s fields and constructor, and the `side`/`radius` fields
  repeated across `ShapeBorder`'s four variants.
- `text_style.rs`: `TextStyle::resolved` and `TextRun`'s five plain fields.

Promoted to `#![deny(missing_docs)]` once the list was empty; the crate's own 180
unit tests still pass.
