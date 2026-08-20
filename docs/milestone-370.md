# Milestone 370 — A checkbox that can answer "some of them"

`Checkbox` had two answers. The reference's has three, and the third is the one a real
list needs: a "select all" above five rows of which three are ticked.

## The third answer is not a second no

Drawn unticked, that header says *nothing here is selected*, which is false. Ticked, it
says *everything is*, which is also false. There is no honest two-state answer, so the
control has to have a third — the reference calls it `tristate`, the platform's
accessibility vocabulary calls it `mixed`, and both mean the same thing.

`Checkbox::maybe(Option<bool>)` is that control. `None` is partly on, and a click cycles
off → on → partly on → off, which is the reference's order.

## Why it needed a second callback

`on_toggle` takes `Fn(bool) -> Msg`. There is no value of `bool` that means "partly", so a
three-state box wired to it cannot report what it is. `on_change` takes
`Fn(Option<bool>) -> Msg` and wins when both are given.

A tristate box wired only to `on_toggle` is not left silent: it reports the two answers
that type can carry, with partly-on reading as on — which is what a click on it moves away
from. Making that case emit nothing would have been a widget that looks live and is not.

## The mark

Both **on** and **partly on** fill the box; only the mark differs. That is the reference's
drawing and it is the right one: the filled surface is what says *this is not simply off*,
and the mark says which of the two it is.

The tick is a `✓` glyph, which is what it has always been and what it pays for — a font's
own width and weight. The partly-on mark is **drawn**: a bar, two numbers on the same box
the border uses. That is what milestone 368 said about the expansion tile's `▾`, and there
was no reason to add a second glyph that some font may not carry.

## `Toggled::Mixed`

`frus_core::Toggled` gained the variant and the shell maps it to AccessKit's, so a screen
reader is told `mixed` rather than being handed a lie in one of the two directions.
`Semantics::maybe_toggled` is the builder that takes an `Option<bool>` whole, next to the
`toggled` that takes a `bool`.

## Left

The reference draws the tick and the bar with an **animation** between them — the tick
retracting into the bar and back. Ours swaps. That is a `CustomPaint` and a curve rather
than a new concept, and it wants its own step alongside the switch's thumb, which
interpolates already.
