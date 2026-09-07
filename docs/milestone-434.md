# Milestone 434 — A door into the widget the shell builds for you

The shell builds the navigation itself. That is what makes it a shell: an application says
`.destination("✔", "Tasks")` and never sees a `NavigationRail`, never decides where it
goes, never measures it.

The cost showed up the moment the rail grew properties of its own. Milestone 432 gave it
label modes and milestone 433 gave it an extended form, a group alignment and two slots —
and **none of them could be reached from the shell**. An application that wanted an
extended rail had to give up `Scaffold` and assemble the screen by hand, which is a very
high price for one boolean.

## One door, not five pass-throughs

The obvious fix is a builder per property: `rail_extended`, `rail_group_alignment`,
`rail_leading`, `rail_trailing`. That is five methods for today's rail and a sixth the next
time the rail learns something, each one a duplicated doc comment, each one able to drift
from the property it forwards.

So the shell takes a function instead:

```rust
Scaffold::new()
    .nav(app.section, Msg::SetSection)
    .nav_placement(NavPlacement::Rail)
    .destination("✔", "Tasks")
    .rail(|rail| rail.extended(true).group_alignment(0.0).leading(fab))
```

The shell builds the rail from the destinations, hands it over, and takes back whatever
comes out. Everything the rail can do today is reachable, and everything it learns later is
reachable the day it learns it, with no change here.

It runs **last** — after the destinations and after `nav_labels` — so the caller has the
final word on both. It is silent when the navigation is a bottom bar, which has none of
these properties to set.

## The label mode is not the rail's alone

`RailLabels` is the one property of the three-and-a-half that **both** navigation widgets
have, so it gets a real builder: `Scaffold::nav_labels`, applied to whichever of the two the
placement chose.

Its field is an `Option`, and that matters. Left unsaid, each widget keeps the default the
reference gives it — a rail labels nothing, a bar labels everything — which is the
asymmetry milestone 432 was about. A shell-level default of its own would have quietly
collapsed the two onto one answer.

## The bug the door opened

The persistent footer's row is **given** its width rather than hugging its content — an
alignment is a claim on free space, so whoever aligns must first be told how much there is,
which is milestone 288's finding. That width was:

```rust
let rail = if rail_nav { RAIL_WIDTH } else { 0.0 };
let row_width = (width - insets.left - insets.right - rail - FOOTER_PAD * 2.0).max(0.0);
```

Read off the constant. Which is right until a caller extends the rail — and then the row is
**176 pixels wider than the room it sits in**, and an end-aligned footer is pushed clean off
the right of the screen.

So the shell asks the rail how wide it came out. `NavigationRail::declared_width` joins
`BottomAppBar::declared_height`, which the shell already had to ask for the same reason, and
the measurement is taken **after** the caller's function has run rather than before.

This is the general shape of the thing: a shell that lays out around a widget cannot read
that widget's size off a constant, because a constant is a guess about a widget the caller
has not finished configuring yet.

## The tests

- `the_shell_can_ask_for_an_extended_rail` — the body starts at 80 unextended and at 256
  extended, and the door is shut when the navigation is a bar.
- `a_footer_beside_an_extended_rail_stays_on_the_screen` — it is on the screen, and it
  lands in the **same place** either way: the footer's row ends where the window does,
  whatever the rail took off the front of it.
- `the_shell_hands_the_label_mode_on` — through both widgets, and saying nothing leaves each
  of them on its own default.

Each was run against the code without the change: the footer test fails with the row width
back on the constant, the extended test fails without the function being applied, and the
label test fails without `nav_labels` being forwarded.

## Still open

`NavScaffold` — the shell that *does* swap a rail for a bar by size class — builds its
navigation the same way and has no door of its own. It is the same three lines, with one
question this milestone did not have to answer: what an `extended` rail means on the width
where that shell has already decided to show a bottom bar.
