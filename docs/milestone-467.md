# Milestone 467 — The third navigation form, and the two that nobody could hear

Material has three presentations of the primary navigation and this framework had two: a
bar across the bottom, a rail down the side, and no drawer. `NavScaffold` mapped its three
size classes onto those two by giving the widest band an *extended* rail — which is a real
answer, and not the only one.

An application with more than five or six destinations has nowhere to put them. A rail's
column is 80 pixels wide; an extended one is 256 and still gives each destination a single
line with no room for a heading above the group or a footer below it. `NavigationDrawer` is
the form that has all of that.

## What a drawer is here

The reference's `NavigationDrawer` *is* a `Drawer` — it returns one, wrapping its
children in a `ListView` inside a `Column` inside `Drawer`
(`navigation_drawer.dart:184`). This crate already has a `Drawer`, and it is the shell:
the thing that slides, scrims, docks and dismisses. So `NavigationDrawer` here is the
**content** of that panel rather than a second panel:

```rust
Drawer::new(app.menu_open)
    .on_dismiss(Msg::CloseMenu)
    .panel(
        NavigationDrawer::new(app.tab, Msg::Go)
            .header(text("Mailbox"))
            .item("✉", "Inbox").badge(12)
            .item("★", "Starred")
            .child(Divider::new())
            .item("✗", "Trash"),
    )
    .body(page)
```

Splitting it that way is what lets the same panel be modal on a tablet and docked on a
desktop without the destinations knowing which.

## A destination's index counts destinations

The reference walks its child list twice and increments a counter only on
`NavigationDrawerDestination` (`navigation_drawer.dart:180`). That is not a detail — it is
the only thing standing between a divider and every destination below it answering with the
wrong screen. The picture would look perfect.

So the entries are one list of two kinds rather than two lists:

```rust
enum Entry<Msg> {
    Stop(Destination),
    Other(Box<dyn Widget<Msg>>),
}
```

`.child(…)` puts anything at the current position and takes no number.
`a_rule_between_destinations_takes_no_number` holds it, and
`a_reader_is_told_which_of_how_many` holds that the announced position counts the same way.

## The indicator's width is a ceiling

`indicatorSize` is `Size(336, 56)` (`navigation_drawer.dart:732`) and `Drawer`'s width is
304. Those numbers do not fit, and in the reference nothing goes wrong: the indicator is a
child of a `Stack` under the panel's constraints, so 336 is what it asks for and 280 is
what it gets.

Read as a width, the pill hangs 56 pixels out past the panel's edge in every application
that never changed the drawer's width — which is all of them. So it is `want_w.min(inner.width)`,
and `the_pill_never_grows_past_the_room_the_tile_has` pins both halves: 280 at 304 wide, 336
at 600.

A number lifted out of the reference carries the constraint system it was written under.
`336` is not a width there; it is a request, and the widget it belongs to never sees it
granted at its default size. Copying it as a width would have been faithful to the digits
and wrong about the widget.

## One colour for both halves

A rail's indicator is a pill **around the glyph** and its label sits below it on the rail's
own surface, so the two take different colours when selected — `onSecondaryContainer` for
the glyph, `onSurface` for the label (`navigation_rail.dart:1251`). A drawer's indicator is
the whole row, so the label stands *on* it and takes its content colour like the glyph does
(`navigation_drawer.dart:759` and `:773`).

That follows from the indicator's size rather than from a separate decision, which is why
`DrawerTile` is written out rather than being `NavItem` with a third mode: the two rows
differ in where the pill goes, and everything else differs downstream of that.

`a_selected_row_paints_its_glyph_and_its_label_the_same` is the assertion; the golden
`navigation_drawer.png` is what actually shows it.

## The rail and the bar were never announced

Adding the drawer's `semantics()` meant reading the reference's, which meant noticing that
`NavItem` — the row both the rail and the bottom bar are made of — **had no `semantics()`
at all**.

The primary navigation of every application built on this framework announced nothing. Not
the destination's name, not which one was live, not how many there were. A screen reader
walking the shell found a row of unlabelled boxes. Every visible part of it was correct,
every test was green, and there was no picture that could have shown it.

It now says what the reference says: the name, then where it sits — "Home, Tab 1 of 3"
(`navigation_rail.dart:533`, `navigation_bar.dart:1020`, `navigation_drawer.dart:464`). Two things about that sentence
are decisions:

- **The label is announced whether or not it is drawn.** `RailLabels::None` is a decision
  about width, and a reader has no width.
- **Which one is live survives being disabled.** Read-only is not invisible.

`tab_label(index, count)` joins `Localizations`. It is the first entry there that takes
arguments and so returns an owned `String` rather than a borrowed `&str` — the numbers are
the caller's, and a table cannot have written the sentence in advance.

## A fourth row in the shell's table

`NavScaffold::nav_drawer(|drawer| …)` gives the `Expanded` band a drawer instead of an
extended rail, and says what to do to it — the door `NavScaffold::rail` opens on the other
two forms.

It is **opt-in**, because the reference gives two answers for the widest band and they are
both right: Material's adaptive guidance puts a navigation drawer there, and the framework's
own adaptive study puts an extended rail (`reply/adaptive_nav.dart:97`). The rail is the
safer default — 256 pixels against 304, and it looks finished with no header — so it stays
the default and an application that has outgrown it says so.

It is silent below `Expanded`. A drawer takes 304 pixels of a window that has 600, and a
shell that handed them over would be answering a question about taste with the body's width.

## `NavDrawerTheme`

Thirteen fields, and a struct of its own rather than more fields on `NavRailTheme` — which
already carries two widgets, with `bar_background_color` beside `background_color` as the
scar. The reference keeps `NavigationDrawerThemeData` separate
(`navigation_drawer_theme.dart`) and the defaults are why: the surface is a rung higher,
the type is a step larger, the indicator has a size where a rail's has only a shape, and a
tile has a height. A shared struct would have needed a second copy of every one of them.

## The guard that never looked

`every_control_with_an_enabled_flag_honours_all_four` (milestone 322) scans for the literal
`enabled: bool,`. The navigation family spells it `disabled` — `Destination::disabled`,
`NavItem::disabled`, and now `DrawerTile::disabled` — so **none of the three has ever been
in the guard's scope**, and neither is `navdrawer.rs`.

The four hooks are honoured here regardless, and there are tests that say so
(`a_destination_that_cannot_be_reached_answers_nowhere`,
`a_destination_that_cannot_be_reached_does_not_light_up`). But that is this module being
careful, not the framework checking. Two milestones ago the guard was defeated by a macro;
this time it was defeated by a synonym. Both are the same finding — a check that reads
source text is checking spelling — and it is on the roadmap as one item now.

## Verification

`cargo fmt`, clippy across the workspace with all targets and all features: silent.
`RUSTDOCFLAGS='-D warnings' cargo doc`: silent. **1256 unit tests**, all green — thirteen of
them new. Goldens **91 + 30 + 14**, with one picture added (`navigation_drawer.png`) and
none moved.
