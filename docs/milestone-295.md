# Milestone 295 — Text is not above the frame any more

Milestone 294 gave the renderer a batch planner and left text out of it, for a stated
reason: a `Primitive::Text` records where the text *starts*, not the box it fills, so
the planner could not know what it covered. Text kept a pass above everything, and the
milestone wrote that down as a rule — "covering text needs a layer".

Then the device showed what the rule costs. Opening the demo's overflow menu, the
labels underneath read straight through the panel. Every menu, dropdown, dialog and
sheet does it, because no widget in frus uses `scene.layer`: an overlay is an ordinary
container drawn late, and the text beneath it is drawn later still. The golden
`table_column_menu.png` had been committed with the Score column's "5" and "3" showing
through the menu.

So the rule was not a design. This is the fix.

## The box, from the only place that knows it

`Primitive::Text` and `Primitive::RichText` gained a `bounds: Rect`. It is not passed
at each of the twenty-odd call sites: the widget walk sets it once, beside the clip and
the owner it already sets, from the rectangle it is about to hand the widget.

```rust
self.scene.set_clip(clip);
self.scene.set_owner(id.as_u64());
self.scene.set_bounds(draw_rect);
widget.paint(draw_rect, status, self.theme, &mut self.scene);
```

That box is the *widget's*, so it is wider than the glyphs — an over-estimate, which is
the safe direction, and tight enough to be useful because a `Text` widget's box is
close to its text.

A scene built by hand rather than by the walk leaves it `UNBOUNDED`. Unbounded overlaps
everything, so such text is ordered strictly by where it sits in the scene and batches
with nothing: the conservative reading of "no idea what this covers".

The field travels like `clip` through every transform — scaled, translated, re-clipped —
and the destructures in `Scene` are exhaustive, so the compiler found all nineteen
sites rather than leaving a silently wrong box somewhere.

## One glyphon renderer per batch

A `glyphon::TextRenderer` draws everything it was prepared with, in one call, so
interleaving text needs one per text batch. `TextPainter` keeps a pool, grown as needed
and reused between frames, and prepares each with only its batch's buffers.

The decoration quads (underline, strikethrough) are rectangles drawn by the rectangle
pipeline, and they belong *beneath the glyphs they decorate* — not in a rectangle batch
of their own. They are laid out grouped by text batch, and the compositor draws a text
batch as its decorations followed by its glyphs.

## What it cost, and what it caught

Nothing. The twelve-row list in `a_real_screen_still_batches` is **3 draw calls** with
text planned, exactly as it was without. Labels do not cover one another, so they land
on one level and share one call.

Getting there needed one correction and turned up one bug.

**The correction.** Milestone 294 grew a rectangle's footprint by its `blur`, on the
theory that a shadow spreads past the rectangle casting it. It does not: the shader
softens the edge *inside* the quad — `smoothstep(-softness, softness, d)` over the
instance's own geometry — and a widget casting a shadow passes an already-widened
rectangle. Double-counting it made every button's shadow reach into the row above, and
with text in the plan that cascaded two levels per row: **27 draw calls** for the same
list. The footprint is now the rectangle, and the milestone-294 test that claimed
otherwise said so in its name; it now covers the stroke, where half the line width
genuinely does fall outside the outline.

**The bug.** `decorated_form`'s golden went blank where the second field's label should
be. `TextInput` painted its floating label *before* the box, with a comment explaining
that the label "lives above" the border — true when it has floated, false at rest,
where it sits inside the box over an opaque surface. It only ever worked because the
renderer drew all text above everything. The box goes down first now.

That is the second widget in five milestones whose paint order was wrong and invisible
(the first being the bottom app bar in 291). Both were found by making the renderer
honest, which is the argument for doing it.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **812 tests, 0
  failures**.
- `cargo test -p frus-gpu` — 24 passing, three new in the planner: a panel dropped over
  a label covers it (the defect, in miniature); labels that sit on separate rows still
  share one draw call; and text with no declared box is ordered strictly by its place
  in the scene.
- **The goldens: 77 pass, and exactly two changed** — `table_column_menu.png`, where
  the column values no longer show through the menu, and `inspector_overlay.png`, where
  the inspected label no longer shows through the overlay panel. Both are the fix, and
  both were looked at before being accepted. Every other golden is byte-identical,
  `decorated_form.png` included once `TextInput` was corrected.

  These two are better evidence than a photograph would be: a pixel-exact before and
  after of the exact defect.

- **On the device** (2026-08-14, STK-L21, signed release APK): the demo's overflow menu
  — the thing that started this — opens with every item opaque. Nothing from the page
  beneath reads through a single one of them. What is still visible *between* the items
  is the page in the gaps: the menu is a column of separate rounded cards with no solid
  panel behind the list, so that is its shape, not the defect. Closed the check this
  note recorded as owed.
