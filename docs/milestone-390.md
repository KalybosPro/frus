# Milestone 390 — Where the tabs sit in a bar with room to spare

Two tabs in a window meant for six were two tabs each three hundred pixels wide, with
their labels marooned in the middle of a lot of nothing. That was the only answer the bar
had.

`TabAlignment::{Auto, Start, StartOffset, Fill, Center}`.

## `Auto` says what the bar already did

The reference's default is `isScrollable ? start : fill` — a rule stated in the widget's
code and nowhere in its vocabulary. `Auto` is that rule, said out loud, and it means the
default is expressible rather than merely being what happens when you say nothing.

## The reference throws; this does not

`TabAlignment.fill` on a scrollable bar, or `start` on one that is not, is an assertion
failure in the reference.

A layout that throws is a crash on somebody's phone for a combination that has an obvious
reading, so neither throws here:

- **Fill on a scrolling bar** reads as `Start`. Sharing a width is the opposite of
  exceeding it, and there is nothing else it could mean.
- **Start on a bar that shares its width** does exactly what it says: natural-width tabs
  packed against the leading edge. The reference forbids it for reasons that are its own
  history rather than anything about the layout, and it is a perfectly ordinary thing to
  want.

## Centring wants a definite width, and a scroll has none

`Center` works on a bar that does not scroll and reads as `Start` on one that does. That
was measured, not assumed.

A centred strip takes the bar's whole width, so there is something to centre in. Inside a
horizontal scroll there is no such thing: the strip is measured by its own tabs — that is
what gives the scroll something to scroll through — so `Percent(1.0)` resolves against an
indefinite parent and the strip comes out exactly as wide as the tabs, with nothing left
over to share. The tabs did not move a pixel.

This is milestone 368's and 377's trap, paid for a third time. The test asserts the two
alignments produce **identical** output on a scrolling bar, and that the same bar without
the scroll does centre — so the boundary is pinned rather than described.

They were never really alternatives: a bar whose tabs might overflow wants `Start` and a
scroll, and a bar whose tabs certainly fit wants `Center` and no scroll at all.

## Three places, one number

The tab's own width, the strip's box, and `tab_spans` — which the indicator and the
scroll-to-selected both read — have to agree, and the file already carried the warning
about what happens when they do not.

`tab_spans` gained the alignment in place of the `scrolls` flag it took before, so all
three now derive from the same resolved value. A test asserts the indicator moved by
**exactly** as much as the label it marks when the alignment changed, which is the
assertion that would catch the drift.

## Resolved against the theme, not against the default

The first version resolved the alignment once in `rebuild_bar`, against
`Theme::default()`. That is wrong in a way that would have shown up only in an application
that sets `TabBarTheme::alignment`: the tabs are built when the builder runs, long before
the real theme is consulted, so the theme's alignment would have been read for the strip's
box and ignored for the tabs' widths — a row of tabs laid out under one rule with an
indicator drawn under another.

Both ask `TabStyle::alignment(theme, scrolls)` at paint and layout time instead. The
resolution is a function, and nothing caches its answer.

## Left

`TabAlignment::startOffset`'s 52 px is fixed. `TabBarTheme::alignment` is themeable, which
the reference's `TabBarThemeData.tabAlignment` also is.
