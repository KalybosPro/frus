# Jalon 108 — `AlignmentGeometry`: unified anchoring

## Analysis

J106–J107 delivered two kinds of anchoring — physical ([`Alignment`]) and
directional ([`AlignmentDirectional`]) — but exposed them **twice**: two builders
on `Container` (`.alignment` / `.alignment_directional`), **two** trait methods
(`alignment` / `alignment_directional`), and a resolution that filtered both. The
established shape puts the two types under a common abstraction,
`AlignmentGeometry`, which the container's `alignment` accepts either way. This
milestone adopts that shape: **one** entry point, resolved in one place.

## Technical decisions

- **`AlignmentGeometry` (frus-core).** An enum `Physical(Alignment) |
  Directional(AlignmentDirectional)` with `resolve(direction) -> Alignment` (the
  physical one is returned as-is). `From<Alignment>` and
  `From<AlignmentDirectional>` → so any anchor converts implicitly.

- **A single builder.** `Container::alignment(impl Into<AlignmentGeometry>)`
  accepts physical **or** directional — `.alignment(Alignment::CENTER)` just as
  much as `.alignment(AlignmentDirectional::CENTER_START)`. The
  `.alignment_directional` builder disappears (redundant).

- **A single trait method.** `Widget::alignment_geometry() ->
  Option<AlignmentGeometry>` replaces the previous two; `align_offset` resolves
  once by `self.rtl` and then applies the unchanged physical mechanics (RTL
  correction included). A reduced trait surface, and lighter forwarders
  (`Box`/`Keyed`/`Responsive`/named: one line instead of two).

## Implementation

- `frus-core`: the `AlignmentGeometry` enum + `resolve` + two `From` impls
  (geometry.rs), re-export.
- `frus-widgets`: the `alignment_geometry()` trait method (replacing `alignment` +
  `alignment_directional`) + forwarders; `Container` (a single
  `alignment: Option<AlignmentGeometry>` field, an `impl Into` builder);
  `align_offset` resolves the unified geometry.

## Tests

- `alignment_geometry_unifies_physical_and_directional` (core): a physical anchor
  is direction-invariant; a directional one follows the reading direction (LTR →
  left, RTL → right), both built through `Into`.
- The existing anchoring tests (centring, corner, fractional, RTL flipping) are
  unchanged — the directional one now goes through `.alignment(...)`, proving the
  single builder accepts both.
- Suites green: frus-core 88, frus-widgets 198; the whole workspace green.

## What's left

- A shell idiom / demo animating `align_tween.animate(&ctrl).value()`.
- Anchoring with **multiple children** (today: a single child).
