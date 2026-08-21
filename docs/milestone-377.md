# Milestone 377 — A tab bar with more tabs than fit

`TabBar` shared its width between its tabs, always. That is the reference's rule for a bar
that is **not** scrollable, and the code said so in a comment — but there was no other
kind, so eight tabs on a phone were eight columns of forty-odd pixels each.

That is not a tab bar with more tabs. It is a tab bar with none you can read.

`TabBar::scrollable(true)` is the other kind: each tab takes the room its label needs, and
the bar scrolls through them.

## One measurement, asked twice

A tab paints its own label rather than holding a `Text` child, so there is nothing for the
layout engine to measure and `width: Auto` would give it nothing. Its width has to be
**stated**.

`TabStyle::scrolled_tab_width` is that number — the measured label plus its padding either
side — and the indicator is placed by the same function. That is the point of there being
one: a tab that measured itself one way and an indicator placed by another would agree on
every label until they did not, and the failure would be an underline creeping away from
its tab as the bar filled up.

`indicator_width` already measured labels, so the measuring was there; what was missing
was anything using it to *place* a tab.

The test checks the spans against a hand-computed prefix sum rather than against
`tab_spans` itself. A test that asks the implementation what it thinks agrees with a
mistake.

## The hairline moved, and had to

The line at the foot of the bar was drawn by the strip. Its own comment said what it is —
*it divides the bar from the panel, and belongs to neither tab* — and once the strip can
scroll, that comment stops describing where it lives: the line would slide away with the
labels.

`min_width: Percent(1.0)` on the strip was the first attempt. It does nothing, for the
reason milestone 368 found on `ExpansionTile`: a percentage resolves against its parent,
and the parent inside a horizontal scroll has no definite width to resolve against. Two
tabs in a scrollable bar ruled an 84 px stub across a 400 px bar.

So the line is `TabBar`'s now, across the bar's own width, at the same offset the strip
used. All 91 goldens are unchanged, which is the check that matters: the line did not move,
it changed hands.

## What did not change

A bar that says nothing is untouched — equal columns, no scrollable registered, the same
pixels. The reference's default is the right one for the two or three tabs most bars have,
and sharing the width in proportion to the labels instead would move every tab whenever
one was renamed.

Everything that makes the bar a bar — the indicator, the ink, the tap, the disabled state —
stays in the strip and does not know it is being scrolled. The scroll is the only
difference, and it is a wrapper.

## Left

`TabAlignment` — where a scrollable bar's tabs sit when they *do* fit: packed at the start,
centred, or spread. The reference has four values and this has one, the start.

`Tab(icon:, text:)` — a tab with a picture above or beside its label. It is a second
measurement rather than a second parameter, since the tab's width and the indicator's both
come from the label alone today.

A scrollable bar does not scroll **to** the selected tab when the selection changes from
outside. The reference does, and it wants the tab's span — which now exists — handed to the
runtime's scroll offset.
