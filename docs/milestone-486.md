# Milestone 486 — Three panels that are one card

Closes [#40](https://github.com/KalybosPro/frus/issues/40).

`ExpansionTile` opens one row on its own, and a column of them is already an accordion as
far as the *state* goes — each asks to change and the application answers, which is what
the tile's own documentation says lets a column of them behave as one.

What a column of them is not is **one object**. It is a stack of independent rows, each
with its own corners, and nothing about it says the shut ones belong together. That is the
half this milestone adds, and it is the reference's `MergeableMaterial` rather than its
`ExpansionPanelList`.

## The surface

Adjacent shut panels are **one card**, divided by a hairline. An open one is lifted out of
the run: a gap either side, and four corners of its own.

The rule is one line — *there is a gap between two panels when either of them is open* —
and everything else follows from it. A panel at the head of a run rounds its top, one at
the foot rounds its bottom, and a panel that is both, which is what an open one always is,
is a whole card.

## Two modes, two messages

The state is the application's, and the two modes want two different states — so they say
two different things rather than one thing twice:

- `ExpansionPanelList::new(open, on_toggle)` is the free one: any number open,
  `on_toggle(index, now_open)`, and the application keeps a set.
- `ExpansionPanelList::radio(open, on_open)` is the exclusive one. It takes an
  `Option<usize>`, and its message **carries the resulting state** rather than the index
  that was pressed: opening the third while the second is open sends `Some(2)`.

That second choice is the whole design. *Only one at a time* becomes a property of what the
widget sends, which an application cannot get wrong by writing its reducer the obvious way
— `Msg::Open(next) => self.open = next`. Had the message been the index pressed, the
invariant would have lived in a paragraph of documentation and been re-implemented, once
per application, in a reducer nobody tests.

**Pressing the open one shuts it**, which the reference's radio list cannot do. A panel a
reader has opened and read is one they may want out of the way, and there is no other
control for it.

## `ExpansionTile::title_child`

A panel's header can be a widget — the reference's `headerBuilder`, for a name with a badge
beside it or a row of chips. The tile's row is a `ListTile`, which has taken a widget for
its first line all along; the tile just never passed one on. Two builders and a slot, and
the chevron, the tap target and the row stay the tile's.

## What is not animated

**The split is not tweened**, and this is the documented decision the issue asks for rather
than an omission. A panel's body appears in one frame, as `ExpansionTile`'s always has, and
the gap and the corners follow it in the same frame — so the surface is never wrong, but it
does not grow into place the way the reference's does.

The tween belongs to `ExpansionTile` first. A list that animated its gaps around a body
that snapped would look worse than one that does neither, and the tile's own animation is
[#30](https://github.com/KalybosPro/frus/issues/30)'s territory.

## The picture

`expansion_panels`: four panels with the second open. Four rather than three, so the
picture carries both halves of the claim — the open one lifted out with its own corners,
and the two shut ones at the foot as a single card with a hairline between them. A column
of `ExpansionTile`s draws the same words and reads as four unrelated rows.
