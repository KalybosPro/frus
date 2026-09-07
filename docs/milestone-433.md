# Milestone 433 — A rail is a column, and a column can do three things a row cannot

A rail and a bottom bar have shared everything since they were written: the same
destinations, the same builders, the same items. But one of them is a **column the height
of the screen** and the other is a single row along the bottom of it, and the reference
gives the column three properties it gives the row none of.

| | what it is | the reference |
|---|---|---|
| `extended` | 256 across instead of 80, labels **beside** the glyphs | `navigation_rail.dart:131`, `:1241` |
| `groupAlignment` | where the destinations sit between the rail's two ends, −1 to 1 | `:205`, `:1237` |
| `leading` / `trailing` | a slot above the destinations and a slot below them | `:145`, `:156` |

None of the three existed here. A rail was a column of destinations against its top, and
nothing else could be put in it.

## The extended form widens the rail and moves nothing

The obvious reading of "the labels move beside the glyphs" is that the whole row is laid
out again — glyph, gap, label, all of it centred in a wider box. The reference does
something more careful: the glyph keeps the **80-pixel column** it had (`:753`), and the
label starts where that column ends (`:796`).

That is why a rail can extend without its destinations appearing to jump sideways: the
glyphs stay on the line they were on, and the words open out to the side of them. The
indicator follows the same rule — it stays around the glyph alone (`:756`) rather than
growing to swallow the label, which is what makes an extended rail still read as a rail
rather than as a list of buttons.

So `NavItem` gained one flag, and the paint gained one number: `col`, the column the glyph
lives in, which is the whole row unextended and 80 when extended. Every horizontal
measurement in the item — the glyph, the indicator pill, the badge anchored to the glyph's
corner — is taken from `col` instead of from the row, and they all move together for free.

**And an extended row is wider, not taller.** The label left the column, so the row is as
tall as the taller of the glyph and the label rather than as tall as both. `item_height`
takes a `beside` flag for it. With the default type this is invisible — the row's constant
floor of 58 wins either way — which is why the test asks with a 40-pixel label, where the
two numbers actually differ.

**An extended rail labels every destination**, whatever `RailLabels` says (`:219`). The
reference goes further and forbids the combination outright (`:121`: an assert). Here
`extended` simply wins, and says so in its doc: the two are different label *layouts*
rather than modes of one another, and the label has room of its own on an extended rail,
so there is nothing left for a mode to trade away.

## The alignment is continuous, which is why it is not a `justify`

`groupAlignment` is a double from −1 to 1, not three names. A rail whose destinations sit a
third of the way down is a thing an application asks for, and a three-valued enum would
have to be replaced the first time one did.

A `justify` on the rail's column would have given exactly three positions. So the rail
assembles itself the way the reference does (`:559`) — the destinations in a **group**,
with a flexible box above it and a flexible box below:

```text
leading      if it is pinned to the top
spacer       (1 + alignment) / 2
group        an unpinned leading slot, the destinations, an unpinned trailing one
spacer       (1 - alignment) / 2
trailing     if it is pinned to the bottom
```

The two grow factors always add up to one, so the free space is split between them in
whatever proportion the alignment names, and the ends of the range are the same three
positions a `justify` would have offered.

**The destinations' spacing had to move into the group.** The rail's column used to carry
`gap: 4`. Left there, it would also have spaced the two flexible boxes away from the group
— which puts the top-aligned default, the one every existing rail uses, four pixels lower
than it has always been. The gap belongs to the thing whose children it separates.

## The two slots start on opposite sides of the group

The reference pins the leading slot to the top and lets the trailing one travel with the
destinations (`:112`, `:113`). That asymmetry looks arbitrary until you name what each slot
is for: a leading slot is **chrome at the top of the rail** — a floating action button, a
menu button — and a trailing one is the **tail of the list of destinations**. An account
switcher below three destinations that have moved to the middle of the rail belongs in the
middle with them, not stranded at the bottom.

Both defaults are the caller's to flip, with `leading_at_top` and `trailing_at_bottom`.

## Building it once, because assembling consumes the slots

Every other builder on this widget could rebuild eagerly: `item()` and `labels()` and the
rest own their inputs and can rebuild the destinations as often as they like. A slot is a
`Box<dyn Widget>`, which cannot be cloned, so assembling the subtree **takes** it — and a
builder can still arrive after the one that set it (`.leading(fab).item("H", "Home")`).

So the rail follows the idiom `ListTile` already uses: the slots are `RefCell<Option<…>>`,
the assembled children are a `OnceCell`, and every builder throws that cell away rather
than filling it. The subtree is built on the first read, by which time the builders have
all run. It is safe for the reason `Widget::build_themed` is documented on: a widget tree
is rebuilt from `view` rather than mutated, so once per instance and once per frame are the
same thing.

## The tests

- `an_extended_rail_puts_the_label_beside_the_glyph` — the label starts past the glyph's
  column, the two stand on one line, and the glyph is at the same x it was unextended.
- `an_extended_row_is_wider_and_not_taller` — asked with a 40-pixel label, where the row's
  constant floor no longer hides the difference.
- `an_extended_rail_labels_every_destination` — under `RailLabels::None`, which is the
  rail's own default.
- `the_group_travels_between_the_rail_s_ends` — four alignments, laid out in a real 600-pixel
  window, in order; and `-0.5` lands between the top and the middle rather than at one of
  them, which is the part a three-valued enum could not have done. It also checks the group
  did not change **shape** on the way.
- `the_leading_slot_stays_where_the_trailing_one_travels` — the two defaults.
- `and_which_of_them_is_pinned_can_be_swapped` — both flags, both ways round.

Each of the six was run against the code without the change first: the three alignment and
slot tests fail with the alignment forced to −1, the paint test fails with `col` forced back
to the row's width, the height test fails with the `beside` branch removed, and the label
test fails without `extended` overriding the mode.

## Still open

**The extended rail does not animate.** The reference drives the width with a controller
and fades the labels in over the first quarter of it (`:605`, `:781`); here it switches.

**`scrollable`** (`:347`), for a rail too short for its slots and its destinations at once,
and **`mainAxisAlignment`** (`:362`), which the reference lets override `groupAlignment`
entirely.

**The shell cannot ask for any of it.** `Scaffold` builds the rail itself from
`.destination(…)`, so an application that uses the shell — which is most of them — can
reach neither this milestone's three properties nor milestone 432's label modes. That is one
pass-through, and it is the next thing worth doing to this widget.

And the rest of a destination: `selectedIcon`, per-destination `padding`, `disabled`,
`indicatorShape`, `useIndicator`, and the rail's `elevation`.
