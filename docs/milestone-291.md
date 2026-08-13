# Milestone 291 — The bar that receives the button

Milestone 290 gave the floating action button a docked placement and then had to admit
it had nowhere to dock: frus's only bottom bar is the navigation one, whose destinations
a docked button lands on. The device said so plainly, and the demo backed off to
floating. This is the bar that was missing.

## `BottomAppBar`

The other kind of bottom bar. The navigation one answers *which section am I in*; this
one carries the actions belonging to the screen you are already on, and leaves a gap for
the one action that matters most.

```rust
Scaffold::new(width, height)
    .bottom_app_bar(BottomAppBar::new().child(row![delete, bar_spacer()]))
    .fab_location(FabLocation::EndDocked)
    .fab(fab_button("✓", Msg::Done))
    .build()
```

`color`, `height`, `padding`, `notch_margin`, `child` — the reference's parameters that
mean something without a Material surface model behind them. `elevation`,
`surfaceTintColor`, `shadowColor` and `clipBehavior` are left out for the same reason
they were left out of the app bar in milestone 287.

## Who cuts the notch

Not the bar. Where the button sits and how big it is are the **scaffold's** business —
it places both — so the scaffold cuts the notch, and the bar's `notched_at` is
`pub(crate)`. A bar built by hand and dropped somewhere else is simply a bar.

This is why `bottom_app_bar` takes a `BottomAppBar<Msg>` rather than an
`impl Widget<Msg>` like every other slot. Typing the slot is what lets the scaffold
configure it. It is also the one clean way round the limitation recorded in milestone
290 — a widget here cannot ask an opaque child anything — and it is worth being honest
that it is a way *round* rather than a fix: it works because both parties are frus
types that know each other.

## The notch itself

The curve is the reference's `CircularNotchedRectangle`, derivation and all: two
quadratics joining the bar's top edge to the button's circle, and an arc between them,
so the bar meets the button **tangentially** rather than at a corner. The constants
`s1 = 15` and `s2 = 1` are its shoulders and its clearance, and they are named as such
rather than left as magic numbers.

`Path` gained `arc_to` for the middle segment — a circular arc appended to an open
outline, split so that no cubic spans more than a quarter turn, since the familiar
`0.5523` stops being accurate beyond that. The general form is
`k = 4/3 · tan(step/4)`, which gives `0.5523` back at exactly 90°.

## What the device added

The first build put a correct notch around a button that was not the shape of it:
`fab_button` made a rounded *rectangle*, so the bar curved around a circle that was not
there and the two did not meet. `fab_button` is round now — `radius = size / 2` — which
is the convention anyway, but the notch is what made it a requirement rather than a
preference. Nothing in the tests could see it: the notch's geometry was right, and the
button's corner radius is not something the scaffold's assertions look at.

The demo's task screen also changed shape in passing: its content used to be centred by
a flex column, and a `Scaffold` body is positioned at the top-left of what it is given —
which is the reference's documented behaviour, so the screen keeps it.

## Found, not fixed: the renderer does not composite in scene order

The device showed the bar's *Delete* button as bare text, with no fill, though the
scene has the bar's surface first and the button's box after it — and a test says so.
The renderer is why. It draws by **kind**, in one pass per painter:

```
rect → image → path → text → composite
```

So **every path is drawn over every rectangle in the frame**, wherever the two sit in
the scene, and the text survives only because text comes after paths. A notched bar is
the first thing built here that puts a path *behind* something, which is why nothing
had noticed.

This is a renderer defect and not a bar one — it applies to any `Primitive::Path`:
`CustomPaint`, the charts, `ClipPath`, the overscroll glow. It wants the passes
interleaved by scene order (runs of same-kind primitives, one draw call each) rather
than four buffers drained in a fixed order, and that is its own milestone.

Meanwhile the demo's bar carries **unfilled** actions — words and icons, which is what
a bottom app bar carries anyway. That is not a workaround dressed as a design: filled
buttons on a bottom app bar are wrong in the reference too. But it is worth being clear
that it is *also* the only thing that works today.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **810 tests, 0
  failures**; five new: an arc lands where it was told and keeps every node on its
  circle; the notch clears the circle it was cut for; it is symmetric about the
  button's centre; no notch asked for gives a plain rectangle; and, at the scaffold
  level, a docked button and the bar it sits on agree — no node of the bar's outline
  intrudes into the button.
- **On a physical device** (Huawei, Android 10): the demo's task screen, which now has
  a bottom app bar with the screen's own actions and the done button docked into it.
