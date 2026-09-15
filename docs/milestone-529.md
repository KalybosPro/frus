# Milestone 529 — Shadows the reference does not draw

The question was whether a widget's default shadow here is the reference's. A survey of every
widget that paints one said: in part. Most of the answers matched — an elevated button and an
elevated card at 1, a floating action button at 6 and 8 under a pointer, a popup menu at 3, a
snack bar and a search bar at 6, and no shadow at all under an outlined, text or filled button
at rest, an icon button, a chip, a dialog, a sheet, a navigation bar, a tooltip, a switch or a
segmented button. What did not match fell in two halves. This milestone is the first: **shadows
drawn here that the reference does not draw**, and three places where a caller could not say
what they meant. Milestone 530 is the other half, the shadows the reference draws and this did
not.

## The rule underneath

In the reference's Material 3 a shadow is painted only when a surface has an elevation **and**
a shadow colour that is not transparent. Many of its components keep an elevation — it is what
the surface tint and the container ladder are measured against — and set the shadow colour to
transparent. A height is not a shadow there. Here it was: every widget that drew one drew it
from the elevation alone, in the scheme's shadow at 30 %, whatever its reference said about the
colour. `Dialog` was the exception, since milestone 451, and is the shape followed below.

## What was found, and what the reference does

**The navigation drawer.** Painted a black shadow at 30 % under every panel, from its default
elevation of 1. The reference's navigation drawer defaults keep that elevation (line 729) and
make both the surface tint and the shadow colour transparent (lines 743, 746).

**The drawer and the app bar, given a height.** Neither has a default elevation that shows, so
nothing was wrong until a caller gave one: a `Drawer` told `elevation(3.0)` cast a black shadow
along its inner edge, and an `AppBar` told `elevation(3.0)` a near-black of the bar's own at
22 %. The reference's drawer defaults (lines 781, 792, 795) and its app bar defaults (lines 2524,
2542, 2545) both keep an elevation and make the shadow transparent, and both resolve the colour
as the widget's, then the theme's, then that transparent default (the drawer's build, line 280;
the app bar's, line 1235). A caller who wants the shadow names a colour.

**The banner.** Its default elevation was 1, read from the reference's banner defaults (line
503). That lifted it: ten pixels of margin under it and no rule along its bottom. No shadow was
drawn — the banner already asked for a colour — so what a reader saw was a banner with a gap
under it and nothing to say where it ended. The reference's build never reads the defaults'
elevation: it takes the widget's, then the theme's, then **nought** (line 375), and it is from
that figure that the margin (line 377) and the rule (line 425) are decided. The 1 in its
defaults is a number nothing uses. An untold banner there is flat, ruled and without margin.

**The search bar's colour.** `shadow_color(Color::TRANSPARENT)` did not remove the shadow. The
paint took the caller's colour and then overwrote its alpha with the default's 30 %, so a
transparent shadow came out black at 30 % and a half-strength one at 30 % too. The same line
did it to a theme's colour.

**The floating action button's hover.** The survey's report was that `elevation(0.0)` still
shadows under a pointer. So does the reference's: its build resolves each of the five heights
on its own (lines 512–522) — the widget's, then the theme's, then the defaults' 6, 6, 8 and 6
(lines 778–781) — and a button flattened at rest rises to eight when hovered. What was wrong
here was the rest of that sentence. There was no caller's word for the hovered height at all,
so a button that should never float could not be told so; and the hovered height was the
default's eight *or the resting height if higher*, where the reference's comes down to eight
from a resting twelve.

**The snack bar.** The elevation was the theme's or 6, with no word for the bar itself, where
the reference's is the widget's, then the theme's, then 6 (line 794). And the shadow was drawn
whatever the elevation came to: at nought, an unblurred box of shadow colour the size of the
bar.

## The decisions

- **`NavigationDrawer`, `Drawer` and `AppBar` cast a shadow only in a colour they are given.**
  Each has a `shadow_color` on the widget and on its theme — `NavigationDrawer` and `Drawer`
  gained both, `AppBar` had both and now defaults to transparent. The elevation is untouched:
  it still decides the drop and the blur once there is a colour, and still what an app bar's
  tint is measured by. The app bar's private near-black constant is gone.
- **An untold banner is flat.** `BANNER_ELEVATION` is `0.0`, and its documentation says why it
  is not the reference's defaults' 1. A banner told `elevation(1.0)` behaves as it did.
- **A search bar's shadow colour is used as it is given**, alpha included. Unset it is the
  scheme's shadow at 30 %, exactly the look it had.
- **A floating action button's hovered height is resolved on its own**: `hover_elevation` on the
  widget, over the theme's, over eight. `elevation(0.0)` still rises under a pointer, as the
  reference's does; `elevation(0.0).hover_elevation(0.0)` stays down.
- **`SnackBar::elevation`**, over the theme's, over 6, and no shadow at nought.

Every shadow keeps the geometry it had. How a height becomes a blur and a drop is the house's
approximation, used the same way by the card, the dialog and the menu, and is not what this
milestone is about.

## Verification

**Tests** — one for each widget, each reading what is painted. The navigation drawer paints no
blurred rectangle by default, one in exactly the colour named on it or on its theme, and none
when flat. A drawer given a height alone casts nothing; given a colour as well, it casts along its
inner edge where it always did. An elevated app bar casts nothing, then one in the colour the bar
or its theme names, and nothing when flat. An untold banner draws its rule and sits flush on what
follows it, and a lifted one keeps a margin and no rule. A search bar's default shadow is the
scheme's at 30 % to the value, `TRANSPARENT` casts none, and a half-strength colour on the bar or
on the theme comes out at half strength. A floating action button told nothing rises to eight
under a pointer, told nought still rises to eight, told nought for both stays down, told twelve
comes down to eight, and a theme's hovered height yields to the button's. A snack bar casts at
six, at the three it is told, nothing at nought on the bar or on the theme, and at the bar's two
over the theme's nought. Each fails without the change: the builders do not exist, and the
colours, the margin and the heights asserted are the ones the old code got wrong.

**Goldens** — none changed, and that was checked rather than assumed: every golden passed and no
`.actual.png` was written. The goldens with these widgets in them draw them on their defaults — a
navigation drawer that fills its whole frame, so its shadow always fell outside the picture; an
app bar and drawers without a height; search bars on their default shadow; snack bars at six;
floating action buttons at rest.

**Checks** — `cargo test -p frus-widgets --lib` (1596 passed), `cargo test -p frus-demo --lib`
(59), `cargo test -p frus-test`, `cargo clippy -p frus-core -p frus-widgets -p frus-demo -p
frus-test --all-targets -- -D warnings`, `cargo fmt --all -- --check` and
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p frus-widgets`: all clean.

**Mutations** — twelve, all killed in the end: the navigation drawer's default colour put back to black,
and its shadow drawn at any alpha; the drawer's default put back; the app bar's near-black put
back, and a flat bar casting a colour it was given; the banner lifted to 1 again; the search
bar's alpha overwritten again, and its shadow drawn at any alpha; the floating action button's
hovered height floored at the resting one, and the caller's hovered height ignored; the snack
bar casting at nought, and the bar's own elevation ignored. **One survived the first run**: the
flat app bar. A bar with no height casts its shadow with no blur, and the test looked for
shadows by their blur, so it saw nothing — the hard box of shadow colour was exactly what it
could not find. It now counts a rectangle in the named colour as well, and the mutation, run
again on its own, was killed. Each run was applied, tested against its module and restored, and
the tree was checked against its diff afterwards.

## What is left

- **The shadow's shape.** Every widget here turns a height into one blurred rectangle — the
  blur growing four pixels a step, the drop two — where the reference's Material paints a key
  and an ambient shadow whose sizes come from a table. The shapes differ at every height; this
  milestone only decides *whether* there is one.
- **A banner lifted by its caller keeps twenty pixels of margin, not ten.** Found by this
  milestone's test: the theme builder the banner is assembled in hands the layout its child's
  whole style, margin included, and the child keeps its margin as well. It is the builder's
  bug, older than this and not particular to the banner, and it wants its own look at every
  widget built through one.
- **The floating action button's focused and pressed heights.** The reference has both (6 and
  6) and resolves them the way it resolves the hovered one. With the defaults they equal the
  resting height and nothing shows; a button told a different resting height would.
- **A drawer's default elevation** is still 0 here where the reference's is 1. With both the
  shadow and the tint transparent it paints nothing, so there is nothing to see; it would
  matter to a caller who names a shadow colour and no elevation.
