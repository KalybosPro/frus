# Milestone 559 — `frus-widgets` denies a missing doc comment

Issue #14, and the last of the fifteen crates. `frus-widgets` is the largest of them — the widget
tree, the theme, the runtime and the driver that turns one into a `Ui` — and almost all of what it
holds is documented already: `#![warn(missing_docs)]` found 155 locations in twenty-seven files,
and everything else it exports already had its comment.

## What was found

- **The per-widget theme (`widgettheme.rs`, 42) and the colour and text scales (`theme.rs`, 35).**
  The fields of `WidgetThemes` each carry the defaults of one widget, and the struct's own comment
  said so in general; each field now says which widget — `Defaults for [`Card`]` — and links it,
  so a reader on the field sees where it leads. The fifteen steps of `TextTheme` say which group
  they belong to and what that group is for, and the roles of `ColorScheme` say what they colour
  or what is legible on them.
- **The three central types with no comment at all.** `Widget`, the trait every node of the tree
  implements, now says what the four required methods are and that everything else has a default
  that does nothing; `Ui`, the interface of one frame, says what it holds and who reads it; and
  `Kanban` says what the board is and that it is controlled.
- **Variants and fields on documented types** (the rest): `AlertKind`, `SnackBarKind`,
  `SnackBarPosition`, `GlowEdge`, `Axis`, `FocusDirection`, `TimeField`, `IntrinsicAxis`; the
  fields of `Positioning`, `ScrollMetrics`, `Scrollbar`, `Scrollable`, `Anim`, `ScrollBallistic`,
  `InspectorNode`, `Status`, the four key-movement variants of `Key`; and a handful of methods —
  `Card::radius`, `Container::decoration`, `Text::wrap`, `TextField::filled`,
  `MediaQuery::remove_padding`, `Runtime::hold_scroll`, `Ui::scene`, `find_widget`.

## Three doc comments that were on the wrong item

The lint found what a count of comments never would: three items that had no comment because
theirs had slid onto a neighbour.

- `Theme::state_layer` had none, and `Theme::brightness` carried both: the state-layer paragraphs
  ran straight into "Whether this theme is a light one or a dark one".
- `Divider::color` had none, and its sentence sat on `Divider::radius`, above the radius's own.
- `TabBar::enabled` had none, and its paragraph about a disabled bar sat on `TabBar::scrollable`,
  above the scrolling text.

Each comment is back on its own item; none was rewritten.

## How it was checked

The lint was run on the host with every feature, and on `wasm32-unknown-unknown` and
`aarch64-linux-android`, because the crate has code that only exists on each.

One link in the first draft did not resolve: `Form` is not exported (`ErrorSummary` is), and
rustdoc under `-D warnings` said so. The field links to the type that is exported.

## Verification

- `cargo clippy -p frus-widgets --all-targets [--all-features] -- -D warnings` — clean.
- `cargo check -p frus-widgets` for `wasm32-unknown-unknown` (all features) and
  `aarch64-linux-android` — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p frus-widgets --no-deps --all-features` — clean.
- `cargo fmt -p frus-widgets -- --check` — clean.

Nothing but comments, and one attribute, changed, and the tests were compiled by clippy's
`--all-targets`; they were not run again.

## What is left

- Nothing that issue #14 names. The workspace's fifteen crates all deny a missing doc comment
  once the pull requests for `frus-gpu` (557), `frus-shell` (558) and this one are merged.
