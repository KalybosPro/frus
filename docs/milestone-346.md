# Milestone 346 — Rich text catches up

Milestones 343 and 344 gave `Text` where its lines sit, how many of them there are, what
becomes of the ones that do not fit, and a default that wraps. `RichText` got none of it,
and milestone 343 wrote that down: *the same three questions, one primitive along.*

They are questions about **text**. That the styles are mixed changes none of them, so
`RichText` now answers all four the same way — `align`, `max_lines`, `overflow`, and
wrapping by default with `no_wrap` for the other case.

## The cut is the only part that is genuinely harder

Everything else is the same code with runs instead of a string. Cutting is not.

A plain text is cut by dropping characters off the end. A rich text has to be cut at a byte
offset in the **concatenation of its runs** — the only coordinate the runs and the shaped
lines share, because the concatenation is what the shaper was given — and then split there,
keeping every whole run before it, half of the run it lands in, and none after.

`frus_text::runs_cut_at` returns that one number. It shapes the runs, walks the layout runs
until one is past the limit, and maps that line's first glyph back through the buffer lines
an explicit newline splits the text into. Splitting the runs at the offset is the easy
half, and it stays in the widget, where the styles live.

The ellipsis takes the style of the run it ends. It is that run's last character as far as
a reader is concerned, and an ellipsis in the base style after a bold word looks like a
mistake.

## What did not have to change

Nothing in the layout. `main_axis_floor`, the fill request behind `align`, the
clamp-to-parent that makes an overflow mode fire, the striped band — all of it came from
milestones 342 to 345 and applies to a paragraph of mixed styles without knowing it is one.
The primitive gained the same two fields as `Primitive::Text` (`soft_wrap`, `align`) and
the renderer sets wrapping and alignment on the rich buffer exactly as it does on the plain
one.

That is the argument for having put those things in the layer they went in rather than in
`Text`.

## Left

- **No fade across a run boundary yet tested.** The mask is a group filter over the whole
  paragraph, so it does not care where the runs meet; there is no golden that proves it.
- **`runs_cut_at` shapes.** Once per frame per limited rich paragraph, and the renderer
  shapes the same runs again immediately afterwards. The same note `visual_lines` carries.
- **A limited plain text is still handed over as lines**, so a justified block with a line
  limit leaves every line ragged rather than only the last. Rich text is cut as a *prefix*
  and does not have that problem; plain text could be cut the same way, and should be.
