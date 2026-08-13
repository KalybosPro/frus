# Milestone 290 — The button has a place, not a corner

The named leftover of milestone 288: the floating action button was pinned to one
corner, bottom-trailing, and that was the whole of it. The reference gives it a
**location** — which end of the row, and whether it floats clear of the bottom bar or
docks astride its top edge.

## What was taken

`FabLocation`, with the six placements that mean something here:

| | leading | centre | trailing |
|---|---|---|---|
| **float** | `StartFloat` | `CenterFloat` | `EndFloat` (default) |
| **docked** | `StartDocked` | `CenterDocked` | `EndDocked` |

`EndFloat` is what the scaffold did before, so nothing moves unless it is asked to.

The geometry follows the reference exactly. Floating: the button clears the top of the
bottom bar by the standard margin. Docked: its **centre** sits on that edge —
`contentBottom - fabHeight / 2` there, `content_bottom - fab_size / 2.0` here. That is
the placement a notched bottom bar is cut for.

## The mini variants, and why they are not here

The reference has twelve placements, not six: each of ours has a `mini` twin that
shifts the button by a fixed adjustment because a mini FAB is 40 px rather than 56. In
frus a FAB is whatever widget the application passes, so "mini" is not a variant of the
placement — it is a smaller button, and it docks correctly by saying so:

```rust
.fab_size(40.0)
.fab(mini_button)
```

Six placements and a size, instead of twelve placements. Same reachable positions,
one fewer thing to know.

The `*Top` family — the button centred on the *app bar's* bottom edge — is **not**
implemented, and the reason is worth stating rather than hiding: it needs the app bar's
height, and the scaffold is handed the bar as an opaque widget. See below.

## The measurement the scaffold cannot make

Docking needs the button's height. The reference measures it, because it lays the
scaffold out itself and knows every child's size by then. frus assembles the scaffold
out of widgets during `view`, before anything is laid out, so the height has to be
**declared**: `fab_size`, defaulting to 56 — the conventional diameter.

This is a real divergence and it is the same one that keeps `*Top` out, that keeps
`AppBar` a builder rather than a `Widget`, and that stops the scaffold telling an
extended body what it is running under. One cause, three symptoms: **a widget here
cannot ask its children how big they are, nor tell its subtree anything, because the
children are already built by the time it sees them.** The reference builds lazily and
so can do both. That is a framework-level design, not a parameter, and it now has its
own roadmap entry with these three as its evidence.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **812 tests, 0
  failures**; one new for the placement: the three ends are distinct and in order, the
  margins are right at both edges, the centred one is centred, a floating button clears
  the bar and a docked one has its centre on the bar's edge.
- **On a physical device** (Huawei, Android 10): the demo's own screen, with a FAB it
  did not have before. `EndDocked` was tried there first and the geometry was right —
  the button astride the bar's top edge — but the same capture settled what the demo
  should ship: that bar carries **three destinations**, so a docked button lands on
  one of them. The demo floats instead.

  That is not a defect in the placement; it is what docking is *for*. The reference
  pairs a docked FAB with a bottom app bar cut with a notch to receive it. frus has no
  such bar, so docking has nowhere to sit properly yet — a roadmap entry, not a fix.

  The same capture caught something smaller and more embarrassing: the button's glyph
  was `＋` (fullwidth plus), which the bundled font does not carry, so the device drew
  a tofu box. An ordinary `+` now. Worth noting because it is invisible to every test
  in the suite — a missing glyph measures and lays out exactly like a present one.
