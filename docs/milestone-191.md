# Milestone 191 — Button: disabled state

## Analysis

The wizard (milestone 190) wanted to block "Next" while a step is invalid, but `Button` had
**only** two useful states: present (clickable) or absent. Material's **unavailable control** was
missing — visible but greyed out and inert — which *shows* that an action exists without allowing
it yet. It is a prerequisite for per-step validation (milestone 192).

## Technical decisions

- **An `enabled` flag, inert end to end.** `Button::enabled(false)` greys the button out (a
  neutral `surface.lerp(muted)` fill, muted text, **no shadow**), and above all makes it **inert
  everywhere**: `on_click` returns `None`, `focusable` is `false` (out of the keyboard tab order),
  and the semantics do **not** announce a clickable action (screen readers). A disabled button is
  therefore never actionable, by any route (mouse, keyboard, a11y) — not merely greyed out on
  screen.

- **The default unchanged.** `enabled` is `true` by default; every existing button keeps its
  behaviour. No rendering changes unless `enabled(false)` is called.

## Implementation

- `button.rs`: the `enabled` field (+ builder); a "disabled" branch at the top of `paint` (the
  fill + muted text, `return`ing before the shadow); `on_click`/`focusable`/`semantics` made
  conditional.

## Verification

- **Unit**: `disabled_button_is_inert_and_unfocusable` (no message, not focusable, non-clickable
  semantics; re-enabled → the click comes back); `disabled_button_paints_no_shadow` (a shadow when
  active, none when disabled). `on_click_returns_message` stays **green**.
- **Golden** `button_disabled` **inspected**: an active "Next" (accent + shadow) next to the
  disabled one (greyed, a thin border, no shadow).
- `cargo test -p frus-widgets button::` **green**.

## What's left

- A **"why is this disabled" tooltip** on hover — composing with the existing `Popover`/`Tooltip`.
- A **"loading" variant** (a spinner in place of the label) — a distinct state, another milestone.
