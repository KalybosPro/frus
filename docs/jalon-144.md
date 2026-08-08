# Jalon 144 — Label notch (`outlined` style)

## Analysis

The floating label (milestone 134) rose into a **band reserved above** the box. Material
also offers the **outlined** style: the floating label sits **on** the top border, which
**opens** a notch behind it — more compact, very recognisable. It was missing.

## Technical decisions

- **Opt-in, the default unchanged.** A `TextInput::outlined()` constructor enables the
  style. Without it, the original "band" rendering is preserved to the pixel: the existing
  goldens do not move.

- **A notch by fill, not by cutting the stroke.** The border is a single `Primitive::Rect`
  (a rounded rectangle stroked on the GPU): you cannot punch a hole in it. So we paint,
  **after** the border and **under** the label text, a small `surface`-coloured fill that
  hides the border segment crossed — the label then comes on top. That fill only appears as
  the label rises (`fade(o * float_t)`), so the notch "opens" along with the float
  animation.

- **A floated target on the border.** The label's geometry is interpolated between rest
  (inside the box, where the hint sits) and a target that **differs by style**: `outlined` →
  `(field.x + PAD_X, field.y − ½·label_height)`, centred on the top border; otherwise → the
  band's top-left corner. One interpolation path, two targets.

- **Paint order inverted.** In band style, the label is painted **before** the border (it
  lives above it); in outlined style, **after** (it sits on it). The geometry is computed
  once, the label's `scene.text` is simply emitted at the right moment for the style.

- **Reduced vertical reserve.** In `outlined`, `label_block` reserves only the label's **top
  half** (the rest bites into the box, since it overlaps the border) instead of a full band
  — making the field that much more compact.

## Implementation

- `textinput.rs`: the `outlined` field + constructor; `label_block` (½ a label in outlined
  style); `paint` — the label geometry factored out, the target chosen by style, the notch
  fill (`fill_rect` in the surface colour) then the label after the border; the `NOTCH_GAP`
  constant.
- `goldens.rs`: the `outlined_field` golden — a filled field (the notch open) + an empty
  field (the label at rest, the border intact).

## Verification

- **Golden** `outlined_field` rendered and **inspected**: "Full name" sits on the top border
  with a crisp break (the notch), the value inside the box; an empty "Email" keeps its
  border closed, the label at rest. Faithful to Material.
- **No regression**: every existing (band) golden unchanged; `cargo test --workspace` green.

## What's left

- An **animated notch border**: the notch's width could animate with the float (here the
  fill **fades in** rather than opening in width).
- A **configurable corner radius** for the `outlined` border (today `theme.radius`).
