# Milestone 332 — A mark, not a container's edge

The roadmap said an unselected checkbox's ring should be `on_surface` rather than
`outline`, and deferred it: milestone 325 had just raised the `outline` token, which
changed every checkbox in the framework, and doing both at once would have made the
pictures unreadable.

The entry was half right, and the half it got wrong is the interesting part.

## What the reference actually resolves

Not a colour — a **state machine**:

```dart
if (disabled)  return selected ? transparent : onSurface.withOpacity(0.38);
if (selected)  return transparent;
if (error)     return error;
if (pressed)   return onSurface;
if (hovered)   return onSurface;
if (focused)   return onSurface;
return onSurfaceVariant;
```

So `on_surface` is right only **under a finger, a pointer or focus**. At rest it is
`on_surface_variant`, which is neither what we had nor what the roadmap said. Ours was a
single colour for every state, and the wrong one.

The disabled branch we already matched exactly — `disabled_content` is `on_surface` at
38 %, and it has been since milestone 322.

## Why it is not `outline`

`outline` is the role for the edge of a **container**. An unselected checkbox is not a
container with something inside it; it *is* the mark, drawn in an *on* colour, the same as
the tick that replaces it.

This project had already written that down. `disabled.rs`, on which token a disabled
control takes:

> a checkbox's tick box and a radio's dot take `disabled_content` even though they look
> like containers — they are the mark itself, with nothing behind them

The disabled path had the rule right and the enabled path did not. Two branches of one
control, disagreeing about what the control *is*.

## The radio too

Which is how the radio came into it. The reference resolves its `fillColor` — the ring and
the dot together — through the same ladder, `onSurfaceVariant` at rest and `onSurface`
interacted, `primary` in every state when the option is the chosen one. Ours had the same
`theme.border` in the same place for the same wrong reason.

`RadioOption` is a widget in its own right, so it receives its own `Status`: hovering one
option lifts that ring and not its neighbours', which is the behaviour the per-state
resolution is for. A group painting itself would have had to fake it.

## The pictures

Two goldens, and the numbers are exact: the dark checkbox side and radio ring both move
from `outline` `(141, 145, 153)` to `on_surface_variant` `(150, 156, 168)`. A modest lift
and a slight turn towards blue.

In light the two roles sit five values apart — `(115, 119, 127)` against
`(110, 116, 126)` — so only a 20 × 20 region moved at all. The change there is about which
role is being asked for, not about what anyone will see, and that is worth saying plainly
rather than dressing up.

## Left

- **`error` and the interacted states of the *selected* box.** The reference returns
  `transparent` for a selected side and a red one under an error; neither is expressible
  here, because a checkbox has no error state at all yet.
- **Neither control has a theme entry**, so this is a hardcoded default rather than an
  overridable one — the standing rule is that every widget's styling is overridable, and
  `Checkbox`, `Switch` and `RadioGroup` are on the roadmap's list of controls still
  missing theirs.
- **Hover and focus are resolved, press is not distinguished from hover.** The reference
  separates the three; all three land on `on_surface`, so nothing is visibly wrong, but the
  ladder is flatter than the one it copies.
