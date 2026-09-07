# Milestone 474 — A mark that changes its mind

Closes [#49](https://github.com/KalybosPro/frus/issues/49).

An icon either was one thing or was replaced by another. It could not **become** another:
the hamburger that turns into a cross as a drawer opens, the play that folds into a pause,
the plus that leans over into a dismiss. A swap tells the reader that something was
replaced. A morph tells them that the same control changed its mind, which is usually what
actually happened.

## The decision the issue said was the whole issue

Interpolating between two arbitrary outlines is a general path-morphing problem — matching
sub-paths, matching point counts, matching winding — and it has no good answer for two
drawings that were never made to correspond. Three bars and a cross do not have the same
number of anything. Solving it in general would be a large, fragile piece of geometry in
service of eight drawings.

So a pair is **authored as a function of `t`**, which is the same door
[`IconData::custom`] already opens for a static mark with one more parameter on it:

```rust
pub struct AnimatedIconData { draw: fn(f32) -> Path, directional: bool }
```

A function pointer and not a closure, so a pair is still a `const` and is declared exactly
where a bundled icon would be. An application that needs a pair this framework does not
ship authors it the same way the framework authors its own — there is no privileged set.

## The part that made every widget work for free

`AnimatedIconData::at(t)` returns an ordinary [`IconData`]. Not a new kind of thing: the
same value every widget in the crate already paints, with a third case behind it.

```rust
enum Source {
    Bundled(IconStyle, u16),
    Custom(fn() -> Path),
    Morph(fn(f32) -> Path, f32),   // ← the drawing, and where in it
}
```

Which means a morph is paintable by an [`Icon`], an [`IconButton`], a floating action
button, a chip, a table header and a navigation destination **without one of them being
told that morphs exist**. Milestone 472 centralised icon placement into
`IconData::placed`; this is what that bought.

It costs one thing, and it is worth writing down: `IconData` is `Eq` and `Hash`, and now
one of its cases holds an `f32`. Position is compared and hashed **by bits**, and
`AnimatedIconData::at` reads a non-finite `t` as `0.0` — because an icon that compares
unequal to itself would break `Eq`'s reflexivity, and an animation driver dividing by a
zero duration is exactly how a NaN gets in.

## Where the `t` comes from, and where it does not

`Icon::animated(pair, on)` and `IconButton::animated(pair, on)` do **not** time anything.
They declare the end they are heading for through `Widget::anim_target`, and the runtime
drives the value there — the same machinery a switch's knob and a drawer's slide are
already on, with the same default duration and the same curve.

That matters for the issue's acceptance criterion. The demo's drawer button and the drawer
it opens both read `app.drawer_open`, and both hand the runtime the same `0 ↔ 1` under the
same rule, so on any given frame they are at the same place — not because two timers were
set to the same number, but because it is one rule computed twice. They are still two
entries in the runtime; a *literally* shared value would need the button to be able to name
the drawer's widget id, and the scaffold does not hand that out. The distinction is
invisible on screen and worth being honest about anyway.

The button was `IconButton::glyph("☰")` until today — a **text character**, in the demo of
a framework that has shipped 2 233 drawn icons since milestone 472.

## The four pairs

- **`MENU_CLOSE`** — three bars into a cross. The outer two swing and lengthen; the middle
  one **shortens to nothing** rather than fading, because an icon is one filled path and
  half of a path cannot be given its own opacity.
- **`PLAY_PAUSE`** — a triangle into two bars, as two quadrilaterals that begin as the two
  halves of the triangle. The right half starts with both of its right-hand corners on the
  tip, which is how a three-cornered shape and a four-cornered one manage to be the same
  shape all the way across. It carries a direction: a play arrow points the way the reader
  reads.
- **`ADD_CLOSE`** — a plus into a cross. The two shapes are congruent, so this is a
  rotation and nothing else; lerping the twelve corners onto each other would pinch the
  arms in the middle.
- **`EXPAND_COLLAPSE`** — a chevron over. Half a turn, so it passes through sideways
  rather than collapsing into a line, which is what lerping a chevron onto its own
  reflection does.

`Path::rotated` is new in `frus-core` for the last two. It keeps each contour's winding —
unlike a reflection — so a shape's holes stay holes with nothing else done about it, and
it turns control points with their curves, which is exact: an affine map of a Bézier's
control polygon is the same Bézier mapped.

## The bug the tests found in the artwork

The first draft dropped the middle bar of `MENU_CLOSE` entirely at `t = 1`, on the
reasoning that a zero-length bar is four points of nothing. Two tests then contradicted
each other — one pinning the point count across the crossing, one asserting the bar was
gone — and the code was what was wrong, not either test. A path whose **shape** depends on
which frame you asked for is precisely the thing a morph exists not to be. The bar stays a
contour and loses its ink instead, and the test now says so in the terms that matter:
three contours at every `t`, and the middle one's area running out exactly at the end.

## What the tests hold

Every pair is pinned at `t = 0`, `0.5` and `1` — the issue's own criterion — and then:

- **nothing jumps**: across eleven steps, no point moves more than four grid units and the
  point count never changes, which is the difference between a morph and a dissolve;
- **a directional pair is mirrored at every `t`**, not only at its ends;
- **two positions of one pair are different icons**, since `IconData` is used as a key;
- **a `t` outside the range is clamped**, and a NaN is read as the start.

And a golden, `animated_icons`: four pairs across five positions each. The three middle
columns are what no amount of testing the two ends can say anything about — a morph that
pinches, turns inside out, or passes through nothing at all is obvious in a picture and
invisible in a number.

## Not here

- **An `AnimatedIcon` widget.** The reference needs one because its icon is a widget; here
  an icon is a **value**, so `Icon::animated` is a constructor rather than a type, and
  every other widget that takes an icon got the feature without being touched.
- **`ExpansionTile` on `EXPAND_COLLAPSE`.** Its current pair is a chevron down when open
  and a chevron along the reading direction when shut — a quarter turn, not a half, and a
  deliberate choice about what "shut" points at. Changing it is a design decision, not a
  consequence of this.
- **Morphing between two *bundled* icons.** That is the general problem this milestone
  decided not to solve, and the reason is above.
