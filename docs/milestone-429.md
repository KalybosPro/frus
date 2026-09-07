# Milestone 429 — The families the scheme was without

`ColorScheme` carried primary, secondary, the surfaces, the outlines, error and the
inverted pair. The reference carries four more families, and each of them is the answer to
a question a widget here has already had to fudge.

| role | what it is for | the reference |
|---|---|---|
| `tertiary` + its three | a third accent that reads as neither of the other two | `time_picker.dart:3664` |
| `error_container` / `on_error_container` | error said quietly rather than shouted | `input_decorator.dart:5981` |
| `inverse_primary` | the accent **on** an inverted surface | `snack_bar.dart:954` |
| `surface_tint` | what a raised surface is tinted *towards* | `bottom_app_bar.dart:301` |
| `surface_dim` / `surface_bright` | the darkest and lightest surface, either theme | `color_scheme.dart:1236` |

Ten roles. The `*Fixed` set is deliberately not among them — see the end.

## Where the values come from

The seeded scheme generates them the way the reference's tonal-spot scheme does. The
tertiary palette is **a sixth of the wheel from the seed at chroma 24** — far enough from
the primary to read as a third thing, close enough to belong to the same palette — and each
role is a tone of it, 80/20/30/90 in dark and 40/100/90/10 in light, the same ladder every
other family uses. `error_container` is a tone of the error palette that already existed.
`inverse_primary` is the *other* theme's tone of the primary palette, which is what an
inverted surface is. `surface_tint` is `primary`, as Material 3 defines it.

The two hand-written schemes needed literals, and those were **read off this crate's own
HCT** rather than picked by eye: a throwaway test printed `TonalPalette::new(hue + 60, 24)`
at each tone for the two schemes' actual primaries, and the numbers went in as written. The
alternative was choosing eight colours by intuition and hoping they contrasted.

`surface_dim` and `surface_bright` follow this scheme's own surface rather than the spec's
tones, for the reason milestone 426 gives at length: the surface here sits apart from the
spec's, so a family anchored on the spec's would land in the wrong place. In light, bright
is the surface (both tone 100) and dim is eleven tones below it; in dark, dim is the surface
and bright is eighteen above — the same two offsets the reference uses from its own surface.

## What the test found

The ladder test gained the dim/bright pair, and the first version of that assertion was
wrong. It asserted that `surface_dim` is darker than **every** container rung and
`surface_bright` lighter than every one — which reads naturally from "always the darkest"
and "always the lightest" (`color_scheme.dart:1236`, `:1241`).

It failed on the ladder's bottom rung, and the reference agrees with the failure: its dark
scheme puts `surfaceContainerLowest` at tone 4 and `surfaceDim` at 6, so the container
family's bottom rung is *below* the darkest surface. The two are separate families, and
dim/bright are a claim about the **surface**, not about the ladder. The assertion is now
`dim ≤ surface ≤ bright` with that written beside it.

The contrast test gained five pairs — tertiary and all four containers — because a
container carries text too, and an errored field's helper line is `on_error_container` on
`error_container` and has to be readable. All five clear 4.5:1 on every seed the test
tries, including the nearly-achromatic grey.

Two lines of French left in that test since it was written are now English, which is the
repo's rule.

## What this makes possible, and what it turned out not to be

Three widgets can now be corrected, and checking each against the reference was worth doing
before writing any of them down as a plan:

- **The snack bar is the big one.** The reference's is an **inverted** bar —
  `inverseSurface`, `onInverseSurface` text, an `inversePrimary` action, elevation 6,
  radius 4 (`snack_bar.dart:949`, `:965`, `:980`, `:983`). This crate's is a card on
  `surface` with a border and a coloured stripe down its left edge, and its three kinds
  carry **literal** colours — `Color::rgb8(70, 190, 120)` and friends — where the scheme's
  own documentation says widgets reference roles and never literals. The inverted pair has
  been in this scheme all along, documented as being *for toasts and snackbars*, and the
  toast has never used it.
- **`BottomAppBar` and `AppBar` want `surface_tint`** (`bottom_app_bar.dart:301`,
  `app_bar.dart:1243`). Material 3's elevation is a tint rather than a shadow, and neither
  bar tints at all here.
- **`TimePicker`'s selected day period is `tertiaryContainer`** (`time_picker.dart:3664`).

And one that looked like a fourth and is not. **The text field is already right.** The
`onErrorContainer` at `input_decorator.dart:5981` and `:6035` is guarded by
`states.contains(WidgetState.hovered)`; the resting error border, label and helper are
`_colors.error`, which is what this crate paints. What is missing is only the *hover*
deepening of an errored field, which is a smaller and different thing than the role list
made it look.

## Still open

The `*Fixed` set — `primaryFixed`, `primaryFixedDim`, `onPrimaryFixed`,
`onPrimaryFixedVariant` and the secondary and tertiary equivalents. They are a different
idea from everything above: colours that stay **the same in light and dark**, for a surface
that must not change when the theme does. Nothing here has that requirement yet, and unlike
`surface_dim` they have no relationship to the rest of the scheme that a test could hold
them to.
