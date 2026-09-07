# Milestone 460 — A menu was a stack of buttons, not a panel

The roadmap had this filed as mechanical: *four widgets still have no shape*, the menu
among them, with a footnote that it "draws per-item rects, not one panel". The footnote
was the milestone. Adding a `shape` to what was there would have rounded the corners of
something that was not a menu.

## What it drew

`rebuild` built a `Flex::column().gap(2.0)` of rows and handed **that** to the overlay.
There was no panel. Each row painted its own filled, outlined, rounded rectangle, so an
open menu was three buttons stacked with two pixels of the page showing between them.

That is not what a menu looks like anywhere, and it is not what the reference draws: a
menu is **one surface**, off the page, with rows inside it (`popup_menu.dart:1837`). The
old golden shows it plainly once you know to look — three separate pills, gutters, and
the table's background running through the middle of the menu.

## What it draws now

A `Panel` widget, which did not exist. One `surface_container` fill, the framework's
corner, a shadow at elevation 3, and eight pixels of room above and below the rows
(`popup_menu.dart:1872`). The rows lost their fill and their outline entirely: a row is
a strip of the panel that lights up under a pointer, and at rest it paints **nothing**.

Three other numbers came along with it, all read off the reference:

- **A row is `MIN_TAP_TARGET` tall**, not 38. The reference gives a menu item
  `kMinInteractiveDimension` (`popup_menu.dart:279`); this had it ten pixels under the
  number the accessibility scanners on both mobile platforms check for, on a widget
  whose entire purpose is to be tapped.
- **The label is `label_large`**, not `title_medium` (`popup_menu.dart:1849`).
- **The row's padding is twelve**, which it already was (`popup_menu.dart:1876`) — the
  one number that was right, now named rather than hard-coded at the call site.

`MenuTheme` grew from one field to eight, and `PopupMenuButton` gained `background`,
`shape`, `radius`, `elevation`, `menu_padding`, `item_padding` and `item_height`.

## Two decisions worth writing down

### The corner is `theme.radius`, not the reference's four

The reference gives every component its own corner: four for a menu, twelve for a card,
twenty-eight for a sheet. This framework collapses all of them into one `theme.radius`
an application sets once. Reaching into that collapse for one widget would make the menu
the only thing on screen that ignores the number the application set — so the framework's
default is `theme.radius`, and `MenuTheme::radius` (or `PopupMenuButton::radius(4.0)`) is
there for anyone who wants the reference's.

**The collapse itself is the real deviation**, and it is now recorded on the roadmap as
one item rather than re-argued once per widget.

### No clip

A row's highlight fills the row's whole box, so on a rounded panel it would paint over
the curve at the top and bottom. The reference clips the panel. This does not need to:
the panel's vertical padding is eight and its corner is ten, and the first row starts
below the second corner's reach anyway — the reference's own numbers make the clip
unnecessary here. Worth saying out loud, because it is the kind of thing that stops being
true if somebody sets `menu_padding` to zero, and the assertion in
`a_menu_is_one_panel_and_not_a_stack_of_buttons` is what would catch it.

## The tests

Four, three of which fail on the old behaviour — checked by reverting the row's paint,
the row's height and the panel wrapping, and watching them go red with the right
messages.

- `a_menu_is_one_panel_and_not_a_stack_of_buttons` — **one** rect the width of the menu
  in `surface_container`, and **not one border anywhere** in an open menu. It was three
  and three.
- `a_row_is_at_least_a_tap_target` — the panel is at least two tap targets plus its own
  room.
- `a_menu_answers_to_its_theme_and_to_its_caller` — surface, corner, room and row height,
  from the theme, then the caller over the theme. None of the four was reachable at all.
- `the_builders_can_be_written_in_any_order` — this one passes on the old behaviour too,
  and is a guard rather than a proof: every builder calls `rebuild`, so unlike
  `BottomSheet` (where milestone 458 found that `body` builds the panel and anything said
  after it is dropped) the order does not matter here. It is exactly the kind of property
  that is true when written and quietly stops being true later.

## The picture

**One golden moved**, `table_column_menu`, and it is the point of the milestone: three
outlined pills with the page between them became one panel with a shadow and three rows.
Its frame went from 340×230 to 340×280, because the taller rows and the panel's own room
no longer fit and a picture that cuts the bottom off the thing it documents is worse than
no picture.
