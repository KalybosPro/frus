# Milestone 530 — Shadows the reference draws

Milestone 529 took away the shadows drawn here that the reference does not draw. This is the
other half of the same survey: **the shadows the reference draws and this did not.** Five were
reported — a button's height under a pointer, the menus' panels, a dropdown button's list, a
slider's thumb and an expansion panel list — and three are done. The menus are not, and why
is most of this note's last section.

## What the reference does

**A button's height follows its state.** Its elevated button defaults resolve the height per
state (lines 570–585): nought disabled, 1 pressed, 3 hovered, 1 focused, 1 otherwise. Its
filled button defaults do the same for the filled button (lines 588–603) and the tonal one
(lines 729–744): nought everywhere except under a pointer, where it is 1. All three cast in the
scheme's shadow colour (lines 562, 580, 721). The resolved height is handed to a `Material`,
which animates a change of height rather than jumping to it. Outlined and text buttons stay at
nought in every state. Here a button held one height whatever the pointer did: 1 for an elevated
button, nought for the rest.

**A slider's thumb stands off its track.** The default thumb is the round one (the slider's
Material 3 defaults, line 2263), and the round thumb rests at 1 and rises to 6 (its slider
parts, lines 680–681), tweened between them by the thumb's activation as it is pressed
(747–749), and cast whether or not the slider is enabled — the shape has no disabled height
(757–767). A range slider's round thumbs are the same, 1 and 6 (its range slider parts, lines
789–790). Here neither thumb cast anything.

**An expansion panel list stands at 2** (the list's constructor, line 195). The panels are
drawn by a mergeable material, which casts a shadow under each *slice* — a run of adjacent
panels with no gap between them, or an open panel on its own — and paints every shadow before
it paints any slice (lines 678–704). Here there was none.

**The menus stand at 3, and a dropdown button's list at 8.** A menu anchor's panel takes its
style property by property, from the widget, then the menu theme, then its Material 3 defaults
(lines 3625–3638), whose elevation is 3 in the scheme's shadow colour (lines 4226, 4249–4251).
A dropdown menu hands its own style to the anchor (the dropdown menu's build, lines 1189 and
1225), and its defaults name no elevation (1719–1724) — so the resolution falls through to the
anchor's 3, which is what the survey said, now checked. A dropdown button's list is painted with
the shadow the elevation table gives 8 (the dropdown's painter, line 66, and its constructor,
line 1002).

## The decisions

**Buttons: a height per state.** Each variant has the reference's four heights — at rest,
hovered, focused, pressed — and a danger button takes the filled one's, being a filled button
in the error colours. A disabled button is flat, as before. `Button::elevation` and
`ButtonTheme::elevation` still name **one** height, and it holds in every state: a caller who
says 2 gets 2 under the pointer too, which is what a height given the reference's button for all
states does.

The height this frame is **blended, not picked**: from the resting height towards the focused
one by the focus progress, then towards the hovered one by the hover progress, then towards the
pressed one by the press progress. That is the reference's order of precedence — a press
outranks a hover, a hover a focus — and it animates with nothing added, because the three
progressions are the ones the state layer is already painted from. So the shadow and the
highlight move together, over the runtime's own durations, rather than on the reference's
separate change of height.

**Slider thumbs: 1 at rest, 6 held.** The press progress carries it, which is what a finger on
the thumb sets: the shell marks the widget pressed when the pointer lands and keeps it until
the release, a drag included. `thumb_elevation` and `pressed_thumb_elevation` are on `Slider`,
on `RangeSlider` — handed down to both its thumbs — and on `SliderTheme`. The shadow is drawn
whether or not the slider is enabled, as the reference's is; the disabled thumb here is opaque,
so what shows is the soft edge around it.

**The expansion panel list: 2, one shadow a card.** `elevation` on the list and on its theme.
The panels are gathered into their runs first and each run casts, so two shut panels are one
shadow rather than two — a shadow under each would lay the lower panel's across the upper one's
surface. The run's shadow is **a node of its own**, because the panels clip to their corners
and a clip takes the clipping widget's own paint with it: a shadow drawn by a clipped container
is cut away at that container's edge. The card is drawn as the list drew it before, one panel
at a time, inside that node.

Every shadow here is cast in the scheme's shadow at 30 %, the house's colour for a height, and
a button keeps the 35 % it had. The buttons and the cards of the list are drawn the way `Card`
draws a height — the blur grows four pixels a step from eight, and the drop is half of that.
**The thumb is not**, and the golden is what said so. Drawn that way, the light-theme picture
showed a square grey patch around each thumb rather than a round shadow. The shadow primitive
fades across its blur on either side of its rectangle's edge and stops at the rectangle, so the
fade is cut off along the border at half strength; on a card the border is the card's and reads
as its edge, but around an eighteen-pixel circle a twelve-pixel blur leaves the rectangle's
corners inside the fade, and the halo is a square. The thumb is drawn the way a search bar
draws its height instead — a blur of two pixels a step from four, a drop of half a pixel a
step — which keeps the rectangle's corners outside the fade at rest, so the halo is a small
soft one with rounded corners. It is still not a circle: the fade is cut along the rectangle's
sides, as it is under every card. Held, at six, the corners begin to show again. Both are the
primitive's limits, and the redesign of shadows is its own topic.

**The menus: not done, for a decision.** A shadow needs something to be the shadow *of*, and
none of the three has it:

- **`MenuAnchor`** floats whatever it is given. It paints nothing of its own — no surface, no
  corner, no padding — where the reference's anchor always wraps its children in a panel.
- **`DropdownButton`** and **`DropdownMenu`** float a column of rows, each drawn as its own
  outlined, rounded box, four pixels apart. There is no panel behind them. A shadow under the
  column would show as grey bands in every gap; a shadow under each row would be six floating
  cards, which is nobody's menu.

The shadow is the easy half. What it waits on is a choice about the surface, which is a visible
change to every application with a dropdown in it, and the reference does not make it for us —
its menus have the panel and these do not:

1. **Give the three the popup menu's panel.** `PopupMenuButton` has had one since milestone
   460: one surface on `surface_container`, the menu corner, vertical padding, rows with no
   outlines of their own, and a shadow. The dropdowns would lose their outlined rows and gain
   the reference's look, at 3 for the menu and the anchor and 8 for the dropdown button, and
   `MenuAnchor` would stop floating bare content. Three goldens change and every dropdown looks
   different.
2. **Shadow what is there.** `MenuAnchor`'s content box casts at 3 and each dropdown row at its
   list's height. The smallest change, and not the reference's look.
3. **Leave the surface to the caller** and say so: `MenuAnchor` documents that its content
   brings its own (a `Card` at 3), and the dropdowns stay as they are.

## Verification

**Tests** — one for each widget, each reading what is painted. A button: an elevated one at 1
at rest, 3 hovered, back at 1 held and focused, and 2 half-way into a hover; a filled, tonal or
danger one flat at rest, 1 hovered, flat again held and focused; an outlined or a text one flat
under a pointer; a disabled one flat; a height named on the button holding under a press, a
theme's nought flattening a hovered button, and the button's word over the theme's. A slider's
thumb at 1 at rest, 6 held and 3.5 half-way, nothing when both heights are nought, the theme's
nought and the slider's 2 over it; each of a range slider's two thumbs at 1 and 6, and at the 3
the range slider was told. An expansion panel list: three shut panels one shadow, the middle one
open three, none at nought on the list or on the theme, and the list's 4 over the theme's
nought. Each fails without the change: there were no such heights, builders or shadows.

Three existing tests read the painted rectangles by position, and the thumb's shadow is now one
of them: the slider's colour tests and its disabled test take the crisp rectangles only, and the
expansion panel list's gap test counts crisp cards, a card's shadow being rounded too. What each
asserts is unchanged.

**Goldens** — six changed, each looked at before it was accepted, and every other golden passed
unchanged. `range_slider`, `range_slider_labels` and `disabled_inputs` gain a faint shadow under
each thumb on the dark theme. `light_outlines` shows it on the light theme, and its first render
is the reason the thumb has its own geometry: drawn as a card's shadow it was a square grey patch
around each thumb, a change for a reason nobody intended, so it was not accepted and the thumb
was moved to the search bar's geometry described above. `tooltip_hovered` hovers a filled button,
which now casts the shadow of its hovered height. `expansion_panels` gains a soft shadow round each
of its three cards.

**Checks** — `cargo test -p frus-widgets --lib` (1599 passed), `cargo test -p frus-demo --lib`
(59), `cargo test -p frus-test`, `cargo clippy -p frus-core -p frus-widgets -p frus-demo -p
frus-test --all-targets -- -D warnings`, `cargo fmt --all -- --check` and
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p frus-widgets`: all clean. One run of the widget
tests failed to link — an unresolved anonymous symbol between two of the crate's own codegen
units — after a build had been stopped part-way through linking; clearing that crate's
incremental cache was the whole of the cure.

**Mutations** — twelve, all killed: a filled button that does not rise under a pointer, an elevated
one that rises no higher than it rests, a hover outranking a press, a height named on the button
ignored, and a disabled button rising; a press that does not raise the thumb, a slider's own
resting height ignored, a range slider's thumbs casting nothing, and a range slider's heights not
handed down to them; every panel of the list made a card of its own, the list's own elevation
ignored, and a card casting a shadow at nought. Each was applied, tested against its module and
restored, and the tree was checked against its diff afterwards.

## What is left

- **The menus' shadows**, waiting on the decision above.
- **The shadow's shape** — one blurred rectangle for a height, where the reference paints a key
  and an ambient shadow from a table. See milestone 529.
- **A floating action button's focused and pressed heights**, left by milestone 529; the
  blending the buttons use here is the shape they would take.
