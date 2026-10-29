# Milestone 707 — The badge, as the reference has it

## Objective

A review of [`Badge`] against the reference, for what it *does* rather than what it is called,
asked for after milestone 706. Its colours, its two sizes, its label's style and what a reader
hears already matched. Six things did not.

## What the review found, and what was done

1. **A hidden badge was a dot.** `label_visible(false)` fell back to the six-pixel dot, so a
   count of zero still said *something is here*. The reference takes the badge away. Now a
   hidden badge takes no room, draws nothing and is not read out; on a child, only the child
   is left.
2. **A badge could not be pinned to a child**, which is what the reference's is mostly for:
   the count on a mail icon, the dot on a bell. **`Badge::on(child)`** now pins it, as a
   [`Badged`]: the child is laid out and takes input exactly as it would alone, and the badge
   is painted over it after its subtree, past its edges if need be. Where:
   - **`alignment`** — the child's top-end corner by default, an `AlignmentDirectional` that
     follows the reading direction;
   - **`offset`** — `(4, -4)` for a badge with a label, mirrored for right-to-left; none for a
     dot, which sits in the corner;
   - the reference's placement exactly (`_RenderBadge`): the alignment along the child less the
     badge's height across, the offset, the 8 pixels the reference adds so as not to move the
     badges laid out before its own placement changed, and a pill raised by half its height.
     On a 24-pixel glyph the pill's left edge is 12 pixels in and its top 4 above.
   Both come from the theme otherwise (`BadgeTheme::alignment`, `BadgeTheme::offset`).
3. **No count.** **`Badge::count(n)`** writes the number, and `999+` past
   **`max_count`** (999 by default) — the reference's `Badge.count`.
4. **The navigation's badges were badges of their own.** The rail and the bottom bar drew their
   count with a geometry of their own — as tall as the text plus four pixels, hung at 40 % of
   their width from the glyph's end — and stopped at `99+`, which the reference's navigation
   does not do. They are now a `Badge`, pinned to the glyph through the same code as
   `Badge::on`: the same height, the same corner, the same count (`150`, then `999+`). The
   drawer, which writes its count as text at the end of the row as the reference's does,
   writes it the same way.
5. **No dot on a destination.** `badge_dot()` on a destination, a bottom bar's item, a rail's,
   a drawer's and the scaffolds' puts the dot there — the commonest mark of all.

## Left

- **A label that is any widget.** The reference's label is a widget; here it is text. An icon
  in a badge is rare, and the text covers counts and words.
- **Padding as insets.** The reference's padding is `EdgeInsets` with only the horizontal side
  set by default; here it is one horizontal value, which is everything the default uses.

## Tests

- `badge.rs`: a hidden badge takes no room, draws nothing and is not read out (a label, a dot,
  a count); a count is capped (`999`, `999+`, a smaller cap); a pinned badge sits where the
  reference puts it, the pill and the dot, left-to-right and right-to-left; alignment and
  offset are the instance's, then the theme's; on a child, the child is the one child and the
  badge is painted over it, past its corner, and a hidden one leaves the child alone; a count
  past the cap is read out as written.
- `navrail.rs`: a destination's count is written as the badge writes it (`150`, `999+`,
  nothing for `0`); a destination can carry a dot, a badge's size, with no text; the pill is a
  badge's, 16 pixels tall and round for one digit. `badge_over_99_is_capped`, which held the
  old `99+`, is gone with it.

## Goldens

Two of the widget goldens show navigation badges, and moved: `navigation_chrome` (325 pixels)
and `one_navigation_two_chromes` (619) — the badge, and only the badge, now the reference's
16-pixel pill on the glyph's top-end corner instead of a larger one hung further out. They were
compared with the committed images and blessed under lavapipe, as the others are; the other 48
are unchanged.

## Mutation testing

Eight hand mutants, eight killed: a badge never hidden, the cap written one early, the
reference's 8 left out, a pill not raised by half its height, the offset not mirrored, a dot
moved like a pill, a hidden pinned badge still painted, a count of 0 shown on a destination.
