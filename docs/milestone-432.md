# Milestone 432 — When a destination says what it is

The rail and the bottom bar always showed every label. The reference lets each of them
choose, and — the part worth knowing — **starts them from opposite defaults**.

| | the reference | why |
|---|---|---|
| `NavigationRail` | no labels (`navigation_rail.dart:1238`) | it stands beside a page it does not own, and glyphs alone keep it narrow |
| `NavigationBar` | all labels (`navigation_bar.dart:1388`) | it owns the bottom of the screen and has the room to say what its destinations are |

The reference keeps two names for the one idea — `NavigationRailLabelType` and
`NavigationDestinationLabelBehavior` — with the same three values. Here they are one enum,
`RailLabels`, with `None`, `Selected` and `All`, and each widget's `new()` starts it where
the reference does.

**This changes what an existing rail looks like.** A `NavigationRail` that showed its labels
now shows glyphs alone until it is told `.labels(RailLabels::All)`. That is a behaviour
change rather than a fix, and it is deliberate: the default is the reference's, and a
framework whose defaults quietly differ from the thing it is modelled on is a framework
whose defaults have to be memorised one widget at a time.

## Two things the shape of it made necessary

**A row does not shrink when its label goes.** Under `RailLabels::Selected` exactly one
destination is labelled at a time. If a row's height followed its own label, selecting a
different destination would move every row in the rail — the selection would appear to
shove the list around. `item_height` keeps the floor whether there is a label or not, and
there is a test for it.

The same reasoning one level up: `BottomBar::sizing` asks for the height a *labelled*
destination needs as soon as **any** mode but `None` is in play, because a bar that resized
as the selection moved would shift the whole page under it.

**A silent destination centres its glyph.** With no label below it, the glyph would
otherwise stay where it sat when there was one, leaving a gap under it. The total content
height is measured from what is actually drawn, so the glyph centres on its own.

## Two numbers, while here

`RAIL_WIDTH` was 76 and the reference's `minWidth` is 80 (`navigation_rail.dart:1240`). And
`item_height` still reserved the old 2-pixel gap under the glyph after milestone 431 moved
the painted one to the reference's 4 — the layout and the paint disagreed by two pixels,
hidden because the constant floor won in every case tried. It reads `LABEL_GAP` now.

## The tests

- `a_rail_and_a_bar_start_from_opposite_defaults` — the three modes' selections, and then
  the two defaults, which is the asymmetry the whole milestone is about.
- `a_silent_destination_centres_its_glyph` — one text primitive instead of two, and the
  glyph **moved down** into the room the label was using.
- `a_row_does_not_shrink_when_its_label_goes` — the height with and without.

## Still open

`extended`: the reference can widen a rail to 256 and put the labels **beside** the glyphs
rather than under them (`minExtendedWidth`). That is a different item layout, not a mode of
this one.

`groupAlignment` (−1 to 1, continuous) positions the destinations as a group between the
top and the bottom of the rail; `leading` and `trailing` are slots above and below them,
where an application puts a floating action button or an account switcher. Neither exists
here.
