# Jalon 59 — Generalising the Material state layers

Milestone 58 introduced `Theme::state_layer` (the baked-in state rule) and
adopted it in `Button`. This milestone **propagates** that rule to the widgets
that each reinvented their own hover with an arbitrary percentage
(`surface.lerp(on_surface, 0.05..0.08 · hover)`), with no response to focus or
press.

## Widgets migrated

Six hover surfaces move to `theme.state_layer(theme.surface, theme.on_surface,
&status)`:

- **menu** (action row) — was 7%
- **dropdown** (header) — was 6%
- **collapsible** (header) — was 5%
- **autocomplete** (field) — was 7%
- **datepicker** (unselected day cell) — was 8%
- **tree** (clickable row, under its hover guard) — was 5%

The benefit: a **unified 8% hover**, and above all the **free** addition of
**focus (10%)** and **press (12%)** responses everywhere, where those widgets did
not react to them at all. At rest (`hover_progress = 0`, no focus or press),
`state_layer` returns the base colour **unchanged** — so the resting appearance
is identical and there is no regression.

## Left as they were (different semantics)

- **breadcrumb**, **chip**: they interpolate a **text colour** (`muted →
  on_surface`) on hover, not a background — that is not a state layer.
- **switch**: the track colour follows the **value** (the position), not the
  interaction.
- **navrail**: the hover/selection pill is drawn as a tinted `fill_rect` — a
  later migration is possible, but it is structured differently.

## Validation

- `frus-widgets` **130 tests**, `frus-demo` **15**, green — the resting
  appearance is unchanged (the state layer is neutral at rest), so the
  structure/rendering tests pass without modification.
- `cargo build --workspace` with no warnings.

## What's next

- **navrail** and other structured hovers (pills) to be unified as we go.
- The **typography** system (`TextStyle`/`TextSpan`/`TextTheme`) — the other half
  of a premium default (the next milestone).
