# Milestone 473 — One navigation, whichever chrome the width called for

Closes [#47](https://github.com/KalybosPro/frus/issues/47).

An application that adapts across widths has three navigation chromes to feed — a bottom
bar when it is narrow, a rail when it is wide, a drawer when it is wider still — and one
navigation to describe. Until now it described it three times, positionally, as
`(glyph, label)` pairs, and the three drifted.

## The destination was already a value. It just was not the caller's.

The interesting part of this milestone is how little of it was new. `Destination` already
existed inside `frus-widgets`, already carried a selected icon, a badge, a disabled flag
and three decorations, and was already what `NavScaffold` forwarded to whichever chrome it
picked. Everything the issue asked for was there — behind `pub(crate)`.

So the work was not to build the type. It was to decide it was **the caller's**:
`NavigationDestination`, public, with a builder of its own, and a `destinations` method on
all five widgets that take destinations — [`BottomBar`], [`NavigationRail`],
[`NavigationDrawer`], [`NavScaffold`] and [`Scaffold`].

```rust
let places = vec![
    NavigationDestination::new(Icons::HOME, "Home"),
    NavigationDestination::new(Icons::FAVORITE_BORDER, "Saved").selected_icon(Icons::FAVORITE),
    NavigationDestination::new(Icons::MAIL_OUTLINE, "Inbox").badge(7).tooltip("Unread"),
];
```

The positional builders stay exactly as they were. `item("★", "Home")` compiles today as
it did yesterday, which was a condition of the issue and is the right condition: the
simple case has to stay simple, and a shell with three destinations should not have to
learn a type to declare them.

## A destination's mark is a drawn icon now, or still a glyph

The module's own documentation used to say, in as many words, that the framework had no
icon font and a destination's icon was therefore a text character. Milestone 472 made that
false. `DestinationIcon` is the type that closes it:

```rust
pub enum DestinationIcon {
    Icon(IconData),
    Glyph(String),
}
```

Both, because both are real. An [`IconData`] scales, takes the theme's colour, and is the
same mark at every size; a glyph is an emoji, which is still the shortest way to put a
flag in a bar. Nothing has to name the type — `From` covers `&str`, `String`, `char` and
`IconData`, so `item` grew from `impl Into<String>` to `impl Into<DestinationIcon>` and
**every existing call site kept compiling**.

The two are measured differently and that is not a detail. A drawn icon is a square of its
own size by definition. A glyph is whatever the font makes of it — wider than it is tall
in one face, narrower in another — so a bar whose selection pill was sized from the icon
grid rather than from the glyph would clip it. `DestinationIcon::measure` asks the mark,
and the pill follows.

## The selected icon, which is the point of having four styles

The convention the icon set is drawn for: an outline at rest, the solid twin when
selected. `Icons::FAVORITE_BORDER` and `Icons::FAVORITE` are the pair the golden uses;
with `icons-outlined` on, so are `Icons::HOME_OUTLINED` and `Icons::HOME`.

It matters beyond taste. A destination that says *this one* with colour alone says nothing
to a reader who cannot tell the two colours apart, and a navigation bar is the one control
in an application that nobody can route around. Two shapes is the accessible answer, and
until the icon set landed there was no way to write it.

## Tooltips, and the wrapper that must not move anything

A rail shows glyphs without labels by default, so the mark is all a destination says about
itself — which is exactly when a hint is worth having. `tooltip` is a destination
property, and the item that has one is **wrapped in [`Tooltip`]** rather than growing a
second implementation of one inside the navigation item.

That is only safe because a tooltip forwards the structure of what it wraps (milestone
425). A bar's destinations share the row through `flex_grow`, and a wrapper that reported
its own box instead of its child's would have collapsed them. There is a test that paints
a bar twice, with hints and without, and asserts the marks land on the same pixels.

## What the tests hold

Six new ones in `navrail.rs`, and two were checked the way this project checks a test —
by breaking what it guards and watching exactly the right ones fail:

- making `glyph()` ignore the selection broke three, including
  `a_selected_destination_shows_its_selected_mark`;
- making a drawn icon paint as text broke two, including
  `a_drawn_icon_is_painted_as_a_path_and_a_glyph_as_text`.

The golden, `one_navigation_two_chromes`, is the issue's own acceptance criterion in a
picture: one `Vec<NavigationDestination>` builds the rail down the side and the bar across
the bottom, the selected destination wears its solid heart where its neighbours wear
outlines, and the third carries a badge — in both chromes, from one declaration.

## In the demo

`sections(active)` in `todo.rs` is that declaration. The bottom bar reads it; so does the
drawer menu beside it, which used to name the same three sections again with its own
glyphs and its own strings. There is now nowhere for the menu to call a section something
the bar does not.

## A cost worth naming

`NavigationDestination` is a wide value — two marks, two strings, three optional colours
and a shape — and a drawer's list holds it beside `Other(Box<dyn Widget>)`, which is one
pointer. Clippy said so, and it was right: the entry is boxed, so a list does not pay a
destination's width for every divider in it.

## Not here

- **`NavigationDrawerDestination` as a distinct type.** The reference has two; this has
  one, because the only thing the drawer's own adds is the full-width background, which
  already lives on the shared value for exactly this reason.
- **A badge that is a dot rather than a count.** `badge(0)` shows nothing, which is the
  behaviour a count wants; an unread *indicator* with no number is a different property
  and belongs with whatever else [`Badge`] grows.

## The two goldens that were red, and were not ours

Milestone 472 left `overflow_band` and `filters_three` failing, by 4 and 12 pixels, and
called it rasteriser noise. That was a guess. This milestone measured it.

Both were rendered in a detached worktree at **`b155332`** and **`63ca41e`** — the commits
that first wrote those two PNGs, 128 and 137 commits back — and both **fail there too**,
on exactly the same 4 and 12 pixels. The renderings are byte-identical at that commit, at
`HEAD`, and in this working tree. So nothing in this framework has moved these pictures in
four months; the bytes in the repository were never what this machine's adapter produces.

What the pixels are, once looked at rather than counted:

- `overflow_band` — a **1×4 sliver at the top-right of the label plate**, where its rounded
  corner resolves one row earlier here. Four pixels out of 48 000, and nothing else in the
  picture differs at all.
- `filters_three` — **twelve pixels on the blur's two vertical edges**, three counts of 255
  darker. A blur edge is a weighted sum of a dozen samples; where that sum falls between
  two integers is the adapter's business.

Neither is re-blessed. Re-blessing would only move the red onto whoever wrote the bytes.
Instead each states what it needs — `assert_golden_with(…, 3, 0)` for the blur, and
`assert_golden_with(…, 2, 4)` for the corner — so both byte sets pass, which is the whole
point of the parameterised variant existing.

A tolerance is worth nothing unstated, so both were checked by breaking the picture: half a
pixel off the blur radius moves **2 410** pixels, and one pixel off the band's box width
moves **1 213**. Four pixels and three counts do not hide a regression; they hide an
adapter. The golden suite is **92 green**.

## And a third, which was neither

`multiline_scrolled` lives in its own test file rather than in `goldens.rs`, was written as
a throwaway preview in milestone 139, and had been red long enough that it was being read
as more of the same noise. It is not. Its 874 pixels are one thing:

- the golden holds a scrollbar **track**, a dim strip the full height of the field. The
  framework stopped painting one on purpose — the reference's track is transparent unless
  a caller asks, and the code says so where it decides not to;
- and its thumb is **six pixels inside an eight-pixel slot**, which is the geometry that
  was corrected to eight pixels held clear of the edge by a margin.

Both changes were deliberate, both landed after milestone 139, and this picture was never
looked at again. So the golden is re-blessed — the render is right and the bytes were
wrong, which is the opposite of the other two and had to be established rather than
assumed.

The test needed one more thing to be worth having. A scrollbar **is not a permanent
fixture**: it arrives when the area moves and fades when it stops, so a `Runtime::default()`
that has never seen this field move draws no bar at all — correctly. The preview injects a
retained scroll offset; it now injects a woken `ScrollbarFade` beside it, because a picture
of a scrolled field with nothing at its edge cannot show where the scroll got to.
