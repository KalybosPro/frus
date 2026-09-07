# Milestone 471 — Buttons that share their edges, and words that take no room

Three widgets, and each of them exists because of one arrangement that the obvious
composition cannot produce.

## `ToggleButtons` — three buttons, four lines

A row of outlined buttons pushed together has **six** vertical lines: each button draws its
own left and its own right, and every join is two hairlines thick. That is why the
arrangement reads as three controls standing next to each other rather than one control
with three parts, and no amount of spacing or colour fixes it — the doubled line is the
tell.

`ToggleButtons` is the widget that gets it right. Each button draws the edge facing the
button **before** it — its *leading* border — and only the last one draws the edge at the
far end. Three buttons, four lines.

### The seam belongs to two buttons

A shared edge has a button on either side of it, so **either** of them being on colours it
(`toggle_buttons.dart:596`). This is the rule that a row of separate buttons cannot express
at all: there, the line between an on button and an off one is drawn twice in two colours,
and which one shows depends on paint order.

The reference's own defaults make all three border colours the same value
(`toggle_buttons.dart:601`, `:635`, `:651`), so none of this shows until a caller names
`selected_border_color` — which is exactly the point at which getting it wrong becomes
visible, and exactly the reason to get it right first.

### How the corners survive

A rounded rectangle is one call and gets its arcs right. Four strips are four calls and do
not — a corner drawn as two overlapping rectangles is a corner with a notch in it. But a
button that draws a whole outline draws four sides, and one of those sides is not its own.

The way out is to let the neighbour cover it. **Every button but the last grows its outline
by one hairline past the seam**, so that its far edge lands exactly where the next button's
leading border will be drawn, and the next button — painted after — covers it. What comes
out is one hairline per seam, every corner an arc, and no button owning an edge it should
not.

The shared-edge colour is then a second, much smaller draw: a strip along the leading edge,
**only when its colour differs from the one the outline already used**. That condition is
not a shortcut, it is a proof obligation that happens to hold: the two rules give the same
answer for the first button in the list (`isSelected[index - 1]` cannot be read at index
zero), and the first button in the list is the only one whose leading edge is a rounded
outer end. Where they differ, the edge is always a straight inner seam.

### The colour reaches the children through the theme

The reference paints a button's contents by handing it a `foregroundColor` that descendant
`Text` and `Icon` widgets inherit. There is no inheritance here of that kind — but there is
a theme, and `Text` and `Icon` both read it for their colour. So each child is wrapped in a
`Themed::tweak` that sets the text and icon colour for its subtree.

A caller who put a plain `Text` in a button gets the selected colour on it without saying
so, and one who coloured that `Text` themselves still wins. That is the same order of
precedence the reference has, expressed with parts this framework already had.

The tweak runs at **walk time**, not at build time, which is what lets `ToggleButtonsTheme`
have the last word: the theme is not known when a tree is built.

### It is not `SegmentedButton`

Both are rows of touching buttons; they answer different questions. A segmented button is
one control with one answer — labels in, an index out. A bank of toggle buttons hands the
caller the buttons themselves, each an arbitrary widget, and asks a separate yes or no
about every one. Bold, italic and underline is three answers, and no styling turns a
single-selection control into that.

### Disabled, and the part that is easy to miss

The four-part contract as everywhere else — no message, no tab stop, no splash, the
disabled colours. Plus one specific to this widget: a disabled bank **never enters the
selected state at all** (`toggle_buttons.dart:739`), so a button that is on shows no fill.
It is still announced as on, because a disabled control is read-only and not blank.

### A row mirrors; a column has to be told

In a right-to-left reading the frame mirrors as a whole, so a row's first button ends up on
the right and its leading border with it — the layout does that, and paint only has to know
which physical edge the start side landed on. A column has no such mirror, so a bank told
to run upwards swaps the two ends itself, in the padding as well as in the paint.

## `GridTile` and `GridTileBar` — the words take no room

A caption *under* a picture is a column, and nobody needs a widget for a column. A grid of
pictures with their names *on* them is a different thing: the tile is the picture's size,
the strip is laid over it, and the grid's rows stay even because the words took no room at
all.

So the header and the footer are **layers**, not siblings (`grid_tile.dart:49`), and the
child is the layer that is not positioned — which is what makes the child, and only the
child, decide how big the tile is.

A tile with neither is simply its child, and this is the reference's early return
(`grid_tile.dart:45`) rather than an optimisation. A stack answers the walk's structural
questions differently from a box with one child in flow; a tile that claimed to be a stack
while holding one child would lay that child out loose in a box of its own.

### The bar reads light, whatever the application is

A `GridTileBar` stands on a photograph, and a photograph has no brightness the theme knows
about. So the strip takes a dark scheme for its own subtree and white content over it
(`grid_tile_bar.dart:75`) — the one place in this framework where a widget overrules the
application's colours instead of resolving against them. The alternative is a caption that
is legible until somebody switches to the light theme.

What the swap **keeps** is everything that is not a colour: the typographic scale, the
reading direction, the spacing, and the application's per-widget defaults. The reference's
`ThemeData.dark()` throws all of that away too; there is no reason to.

And it is overridable in more ways than the reference's is — `foreground_color`,
`title_style`, `subtitle_style`, the two heights, and `GridTileBarTheme` behind them all.

### Two heights, and the content picks

A bar with a title **and** a subtitle is 68; anything else is 48
(`grid_tile_bar.dart:79`). They are two theme fields and not one, because an application
setting the one-line height has not thereby said anything about the two-line one.

A subtitle on its own takes the **title's** type (`grid_tile_bar.dart:115`), which is the
detail worth keeping: a subtitle with no title above it is not a second line, it is the
only line.

Captions do not wrap and do not grow — two lines of a name would cover the face the tile is
showing — so the bar sets `soft_wrap`, `overflow` and `max_lines` for its subtree rather
than asking each `Text` in it to.

## Recorded, not fixed

The crate now names **five** axes — `Axis`, `ReorderAxis`, `IntrinsicAxis`, `DismissAxis`
and now `ToggleAxis` — because the shared one carries a `Both` that only a scrollable can
mean. That is a type to unify, not a widget to write, and it is on the roadmap.

## Verification

`cargo fmt`, clippy across the workspace with all targets and all features, and
`RUSTDOCFLAGS='-D warnings' cargo doc`: all three silent. **1309 unit tests**, all green
— twenty of them new, three checked by breaking what they guard and watching exactly one
test fail each time: the outlines no longer meeting, the shared edge reading only its own
button, and a tile with nothing over it still claiming to be a stack. Goldens
**91 + 36 + 14**, two pictures added and none moved.
