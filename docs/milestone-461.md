# Milestone 461 — A progress indicator was drawn to no specification

Between them, the two progress indicators had **two** builders: a width and a size.
Everything else — the height of the bar, the colour of its track, the colour of its fill,
how thick the ring's dots are — was decided inside `paint`, where nothing could reach it.
Neither read a theme entry, because there was none: `ProgressIndicatorThemeData` is one
of the reference's ordinary tables and this had no equivalent at all.

## What the bar got wrong

Two things, and both were numbers nobody had chosen.

**The height was eight**, which is twice the reference's four
(`progress_indicator.dart:1624`). A progress bar reads as a line; at eight it reads as a
slab.

**The track was `muted` faded to thirty percent.** That is a colour arrived at by
multiplying, not a colour a designer picked and not one a theme could name — you cannot
say "make the track quieter" about an expression. The reference gives it a role:
`secondary_container` (`progress_indicator.dart:1621`).

## The look this takes

The reference draws two linear indicators behind a `year2023` flag. The old one has
square ends, no gap and no stop dot; the new one leaves a **gap** between the fill and
the track and puts a **dot** at the far end saying where the bar is going. The flag still
defaults to the old look — and the reference has already deprecated it, saying in as many
words that it will default to the new one in time.

A framework with no installed base to keep faith with should start where that one is
going. So the gap (four) and the stop dot (radius two) are on by default, and both are
builders: `track_gap(0.0)` and `stop_indicator_radius(0.0)` write the older look back.

The corners stay fully rounded, which is the newer look too — the 2024 default rounds a
four-pixel bar by two, and half of four is two.

Three things fall out of the gap that are worth stating, because they are what the test
holds:

- At the end of a run there **is** no track: the fill has the whole width, so there is
  nothing to leave a gap in.
- And therefore no dot either — the dot is drawn only while the track is wide enough to
  hold it.
- At zero the track is the whole bar and the dot sits at its end, which is the one case
  where the dot is doing the most work: an untouched bar that says where it is going.

## The ring

`CircularProgressIndicator` is a ring of dots, not the reference's arc, and it stays
one — `paint_activity_ring` is shared with pull-to-refresh, so the two cannot drift into
looking like different frameworks' idea of "busy", and that is worth more than the arc.
What it gained is the three rungs it had none of: `color`, `stroke_width`, and
`track_color`.

`track_color` is the reference's `circularTrackColor` (`progress_indicator.dart:1590`):
the whole circle drawn once, quietly, so that the unlit part still reads as part of a
ring. It is **off unless something names a colour**, so a spinner that says nothing draws
exactly what it always drew, and the golden with a spinner in it did not move for that
reason.

`stroke_width` deliberately does **not** take the reference's flat four. Four is right at
24 pixels and a hairline at 96; this keeps the proportional rule as the framework's own
default and lets a caller name a number when the proportion is not what they want. That
is the same argument as the menu's corner in milestone 460, and it is the second time in
two milestones that a per-component absolute in the reference has met a proportional rule
here — worth noticing as a pattern rather than deciding twice.

## Two French comments

`// Piste.` and `// [piste, remplissage]`, left in this file since it was written. Fixed
here rather than filed, since the file was open.

## The tests

- `a_bar_is_drawn_to_a_specification` — the height is four, the track has a role and the
  fill is the accent.
- `a_track_leaves_room_for_the_fill_and_a_dot` — the gap, the dot, and neither of them at
  the end of a run.
- `a_bar_answers_to_its_theme_and_to_its_caller` — six properties through the theme, then
  the caller over the theme.
- `a_ring_answers_to_its_theme_and_to_its_caller` — colour, stroke and track, with the
  track proving it is off by default.

**One golden moved**: `small_indicators`, which is the only picture with a bar in it, and
it moved for the two reasons the milestone is about — the bar is half as tall and its
track has a colour.
