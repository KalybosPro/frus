# Milestone 363 — Room around the content, and which side scrolls with it

The last item on milestone 357's depth audit list, and the one every scroll view in the
reference has: `padding`. `ListView`, `GridView` and `SingleChildScrollView` all take it,
and none of ours did.

## Inside the viewport, not out of it

There are two things "padding on a scroll area" could mean, and only one of them is any
use.

**Room taken out of the viewport** shrinks the window onto the content. A list padded 88
at the bottom would show 88 fewer pixels of itself, permanently, and the last row would
still slide underneath whatever the room was for.

**Room around the content, inside the viewport** — the reference's `SliverPadding`, and
what this is — scrolls *with* what it surrounds. The 88 pixels sit at the end of the
content and are reached by scrolling to them, which is exactly what a floating action
button hovering over the last row needs: the row clears it, and nothing is lost from the
window while you are anywhere else in the list.

Along the cross axis there is no distinction to make: the content is the width of the
viewport less the two sides.

## One hook, two branches

`Widget::scroll_padding` is read by both the scroll branch and the virtualised list, which
is the whole of the change:

- **`Scroll`** lays its content out at the viewport less the insets, adds them back to
  work out what scrolls (which is what makes the far inset reachable rather than
  decorative), and offsets the content by the leading two.
- **`List`** could not have wrapped its items in a padded box even if we wanted to — that
  is the point of a virtualised list, there is no box holding the items. Its window
  arithmetic takes the leading inset off the offset, the extent gains both, and the item
  lands that much further in.

## Which side leads a reversed list

A reversed list starts at the bottom, so the **bottom** inset is the one item 0 clears —
and it is still at the bottom, where it looks. That is not a special case bolted on: it is
`lead = if reverse { pad.bottom } else { pad.top }`, one line, and the rest of the
arithmetic is unchanged.

It is the reference's answer too. `SliverPadding` resolves *before* and *after* in the
scroll axis's direction, which for a reversed vertical list runs upwards — so before is
the bottom.
