# Milestone 297 — A harness that can run the clock

Milestone 296 gave 75 of the 86 widget modules a pixel test and left eleven, for one
stated reason: what they draw is not a function of their arguments. A swipe half
done, a pull past the top edge, a glow where a list hit its end, a page between two
pages — none of that is in the widget. It is in the `Runtime`, and the shell is what
puts it there.

So the harness learned to be a shell.

## `Stage`

```rust
let mut stage = Stage::new(260, 170);
stage.settle(&root);
let region = stage.build(&root).scroll_regions()[0].clone();
stage.runtime.glow_pull(region.id, GlowEdge::Top, 90.0, 150.0, 0.0, 240.0);
stage.advance(&root, 1.0 / 60.0);
stage.render(&root)
```

Three things make it honest rather than a set of pokes at a struct:

- **`advance` is the shell's own list, in the shell's own order.** It builds the tree,
  takes the scroll regions, refresh areas, dismissables and interactive bounds off the
  frame, then steps every family: values, colours, sizes, radii, paddings, scroll,
  glow, refresh, dismiss, interactive, hover/focus, leaving. Joined with `|` and not
  `||`, because every family must be stepped whatever an earlier one answered.
- **The gestures go in through the entry points the shell uses** — `refresh_pull`,
  `dismiss_drag`, `glow_pull`, `dismiss_release` — not by writing to a map. A test
  that lies about how state arrives will pass over a widget that no real gesture can
  reach.
- **Identity comes from the frame.** `stage.build(&root).dismissables()` hands back the
  swipeable with its id and its extent, so nothing counts nodes. `multiline_scroll.rs`
  had to say "the field is the second node: Container is the root, its child is the
  field"; that comment is the thing this replaces.

`Stage::advance(root, 0.0)` is not a no-op — it is the **first** frame, where a widget
seen for the first time adopts its target instead of sliding in from zero. That is
what milestone 296 found by accident, and it is now a named method: `settle`.
`render_widget` is three lines on top of `Stage`.

One test exists purely to keep the harness honest:
`the_stage_actually_advances_time` pulls a glow, photographs it, runs half a second of
nothing, and asserts the two frames differ by more than 200 pixels. A harness that can
set state but never watch it settle is half a harness, and this fails if `advance`
ever stops advancing.

## The eleven

`crates/frus-test/tests/motion.rs`, twelve tests. Five needed the frame loop:

| golden | what it catches |
|---|---|
| `swiped_half_way` | a row moved 45% of its extent, the red background it was hiding now showing |
| `pulled_past_the_top` | a list pulled 70 px past its top, the indicator out and armed |
| `overscroll_glow` | the glow at the edge a list hit, one frame in, while it is still bright |
| `page_view_mid_swipe` | the seam between page 0 and page 1, mid-viewport |
| `drop_zone_highlighted` | one target washed because something is over it, beside one that is not |

Six only ever needed the right arguments, and are here because they belong with their
neighbours: a navigator mid-push with both screens in the frame, a hero at rest, two
keyed subtrees, `Responsive` at two size classes, a `LayoutBuilder` printing the box
it was handed (`180 × 60` — the assertion *is* the picture), and `NavScaffold` in both
presentations, a bar when compact and a rail when not.

**Every widget module that draws now has a pixel test.** 86 of 86.

## What reading them turned up

All eleven images were looked at before being accepted, and three were wrong the first
time — the tests, not the widgets:

- The swiped rows hugged their text, so the swipe read as a chip sliding rather than a
  row. Given a width.
- The navigator's screens did not fill it, for the same reason.
- **The lit drop zone was not lit.** `DragTarget` washes its own box *before* its
  children paint — deliberately, so the wash lands on the target's background and not
  over its text — which means a child with an opaque background hides it completely.
  The test outlines its zones instead of filling them, and says why. Worth knowing
  before you wonder why your drop target does nothing.

## And a claim that was wrong

The CI job and the roadmap both said the goldens stay advisory because "lavapipe
rasterises differently from hardware". They do not: the goldens are *blessed* under
lavapipe as well — llvmpipe under WSL is the only adapter there — so both sides run
the same rasteriser. What differs is its **version**. Corrected in both places, with
the actual fix named: pin the rasteriser in CI and drop `continue-on-error`. A
tolerance guessed without measuring would hide the drift rather than remove it.

## Verification

- `cargo test -p frus-test` — **127 pixel tests, 0 failures**: 77 screens, 27 widgets,
  12 in motion, and the rest. Every previously blessed image is byte-identical.
- The workspace suite is unchanged.
