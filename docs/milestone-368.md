# Milestone 368 — An expansion tile that is a tile

`ExpansionTile`'s whole surface was

```
new(title, open, on_toggle)   content(widget)
```

Two builders. No leading widget, no subtitle, no trailing widget, no colours, no
measurements — a title as a `String` and nothing else. It painted its own header: a
hardcoded 40 px tall, a hardcoded 18 px of text, `▾` and `▸` as *text glyphs*, and colours
taken straight from the scheme with no way to say otherwise.

That is a plain breach of the standing rule — themed defaults are fine, hardcoded-only
never — and it was the last widget in the catalogue still written that way.

## It is a `ListTile`

The reference's `ExpansionTile` **is** a `ListTile` with a chevron and a body underneath,
and ours is now built out of one rather than painting a header of its own.

That single decision brings everything a tile already knows how to do: the Material 3
measurements (56/72/88 by line count, 48/64/76 dense), the leading slot with its minimum
width, the text column that gives way while the slots either side keep their size, the
state layer under a tap, the selected colour. Milestone 336 did that work; there was no
reason to do it a second time, worse.

What the tile adds on top is what the reference adds: `subtitle`, `leading`, `trailing`,
`show_trailing_icon`, `control_affinity`, `dense`, `tile_padding`, `children_padding`, and
four colour pairs — background, text and icon, each with a *collapsed* counterpart, because
an open tile and a shut one are two states a design usually distinguishes.

## Two questions the slots forced

**The chevron and a widget can want the same slot.** `control_affinity(Leading)` puts the
chevron in front, which is what a file tree wants — the chevrons line up down the left and
the indentation reads as a hierarchy. But `leading` may already hold something. The rule is
that the chevron wins and the other is dropped, which is what the reference does: two
widgets in one slot is a bug a row of fixed slots cannot report, and silently stacking them
would be worse than silently dropping one.

`trailing` is the other way round — it **replaces** the chevron, because a tile whose end
carries a badge or a switch is not also carrying an arrow.

**When the palette is read.** The tile is assembled in `build_themed`, on the way down,
under the theme of the subtree it sits in — not at construction. It reaches for
`theme.text.body_large` and `on_surface_variant`, and a tile built at construction time
inside a `Themed` subtree would come out in the wrong palette. `ListTile` learned that
lesson first; this follows it.

## `IconName::ChevronDown`

The old header drew `▾` and `▸` as **text**, which meant the chevron was a font's opinion:
a different size from the icons beside it, a different weight, and missing outright from a
font that does not carry those code points. `ChevronDown` and `ChevronUp` join the icon set
as paths on the same 24×24 grid as the rest, so the arrow is the same arrow everywhere.

## What it found in `ListTile`

The tile's chevron came out against the last letter of the title rather than at the end of
the row. The tile was the right width; the **row inside it** was not.

`ListTile::row()` builds a `Flex::row()` with no width and no grow, so it hugged its slots.
The text column between them is an `Expanded` — which is the whole mechanism for pushing
the trailing slot to the far edge, and it had nothing to push against. Every `ListTile`
with a trailing widget has been drawing it halfway across the row since the tile was
written; nothing caught it because no golden had one until this milestone put a chevron
there.

Fixing the first half turned up the second. A row that only **grows** fills the tile and
then runs straight through it the moment its content is wider — since milestone 349 nothing
is squeezed unless it says so — and a long title pushed the trailing slot **265 px outside
a 200 px tile**. The overflow band said so, loudly and correctly, which is the framework's
contract working; it is still a badge drawn off the side of a list.

`Expanded::new(row)` says both at once: grow into the box, give way rather than run past
it, and no content-sized floor underneath either. Two assertions keep it found — the slot
reaching the end of the row at three widths, and a title too long being cut rather than
carried past the edge.

## Left

`maintainState` — keeping a shut tile's subtree alive — has nothing to keep alive here: the
body is a value the application hands over each frame, and there is no state inside it that
being unbuilt would lose. `expandedAlignment` and `expandedCrossAxisAlignment` are the body
container's business and reachable through it.
