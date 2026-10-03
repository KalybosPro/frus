# Milestone 618 — The selection bar's desktop form

## Objective

The bar over a selection — Cut, Copy, Paste, Select all — was the same pill on every
platform, and on a desktop it opened over the selection even when a right-click had asked
for it somewhere else. The reference's adaptive toolbar takes its form from the theme's
platform (`adaptive_text_selection_toolbar.dart:308`): the pill on Android, a menu on
Fuchsia, Linux and Windows, and Apple's own bars on iOS and macOS. And a context menu opened
by a secondary tap goes where the tap was (`editable_text.dart:3211`).

## What was done

- **The desktop menu** (`desktop_text_selection_toolbar.dart`), under a theme whose platform
  is Fuchsia, Linux or Windows: a column 222 px wide, its corners rounded at 7, one step off
  the page on the scheme's surface; a row per item, at least 36 px, the words 20 px from its
  start and 3 px clear of its bottom, 14 px at the regular weight, white on a dark scheme
  and black at 87 % on a light one, in a square button with the theme's state layer.
- **Where it goes** (`desktop_text_selection_toolbar_layout_delegate.dart`): its top-left
  at the anchor, moved back by as much as it would hang past the window's right or bottom
  edge, less 8 px.
- **The anchor**: a right-click records where the pointer was, and the bar — menu or pill —
  goes there. Opened any other way (a hold, a handle let go, the context-menu key), a menu
  hangs from the top of the selection at its middle, the reference's primary anchor
  (`text_selection_toolbar_anchors.dart:45`), and the pill stays centred over the
  selection.
- One `SelectionToolbar` holds both forms and lays out the one the theme it is laid out
  under asks for. Android, iOS and macOS keep the pill: the reference's Apple bars have their
  own look, which frus has no widgets for yet.

## A bug the menu found

A press is offered to the selection handles before anything else, and the bar came after
them. The pill never covers the handles — it goes above the selection, or below the handles
— so nobody could tell. The menu hangs from the selection's top, over its handles, and a
press on its rows took a handle instead. The bar is drawn over everything, the handles
included, so it now has the first say.

## Not done

- **Letter spacing.** The reference's rows are set at −0.15 px; frus's `TextStyle` has no
  letter spacing.
- **Apple's bars**, for iOS and macOS.
- **The context menu after Select all.** On the desktops the reference hides it; frus keeps
  it open, as it does on a phone.

## Tests

- `the_desktops_get_the_menu`, `the_menu_hangs_from_its_anchor_inside_the_window`,
  `on_the_desktops_it_is_a_menu_of_rows`: the platform table, the placement arithmetic,
  and the menu's form under a Linux theme whatever the test runs on.
- `a_desktop_menu_hangs_from_the_pointer_or_the_selection`: at the pointer, pushed back
  inside the window, and from the selection's top when no pointer opened it.
- `a_right_click_opens_the_menu_at_the_pointer` (shell): a real right-click puts it there.
- `the_desktop_menus_rows_act_on_the_area` (shell): its rows act, over the handles.
- The pill's tests now say they are about the pill: they lay out under a theme that
  follows Android, where they used to follow whatever machine ran them.

Mutation testing: five mutants, all killed — Fuchsia given the pill, the menu let hang past
the window's edge, a selection's anchor taken from its bottom, the pointer ignored by the
layout, and a right-click that does not record it.
