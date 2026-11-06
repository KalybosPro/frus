# Milestone 766 — Every floating button placement, placed from what the shell measured

## Objective

Milestones 752 and 757 left four things open in `Scaffold`'s handling of the floating
action button:

1. `FabLocation` had six placements (start, centre, end; floating or docked). The placements
   at the top (astride the app bar's bottom edge), the mini ones, and the one contained inside
   the bottom bar did not exist, and an application could not supply a placement of its own.
2. A persistent footer did not lift the button: its height was not known when the button was
   placed, so a button over a tall footer covered it.
3. A snack bar and a bottom sheet did not move the button. Both were overlays.
4. The bottom app bar's notch was cut under a docked button even when the keyboard had
   lifted the button off the bar.

## Root cause

The four have one cause. The shell was a column (app bar, body, footer, bottom bar) with the
button in a layer above it, and the button's place was computed **while the tree was being
built**, from declared numbers: the bottom bar's declared height, a declared button size
(`fab_size`, 56 by default), the keyboard. Nothing that has to be laid out to know its size
could reach that computation: the footer, a snack bar, a sheet, the button itself. The notch
was cut from the same build-time numbers, so it could not follow a button that moved.

## What was done

**The shell is one layout that measures its slots** (`scaffold.rs`, `ScaffoldLayout`). It is a
`CustomMultiChildLayout` whose function places each slot in the order the leading toolkit's
scaffold does:

1. the rail, if the navigation is one, the whole height on the leading side;
2. the app bar at the top (its height is the content's top);
3. from the bottom up, the bottom bar at the window's edge and the persistent footer on it;
4. the content's bottom: `height - max(keyboard, bottom widgets)`, never the two added up
   (milestone 752's rule, now with the footer's measured height in it);
5. the body, in what is left (under the bottom widgets when `extend_body` says so, never under
   the keyboard);
6. a persistent bottom sheet standing on the bottom widgets;
7. a snack bar;
8. the button, from a `FabGeometry` holding everything measured.

The children are given in the order the old column and its layers painted them, so what the
body runs under is still drawn over it. `KeyboardFloor` and the separate button layer are gone.
`ChildLayout` gained `theme()`, so a layout function can read the reading direction and the
theme (the shell reads the snack bar theme's placing).

**Every placement** (`FabLocation`, 19 variants): `StartTop`, `CenterTop`, `EndTop`,
`StartFloat`, `CenterFloat`, `EndFloat`, `StartDocked`, `CenterDocked`, `EndDocked`, the mini
form of each of these, and `EndContained`.

- **x**: the start is 16 px from the edge plus the safe area on that side, and it is the right
  in a right-to-left script; the end mirrors it; centred is `(width - button) / 2`. A mini
  placement is `MINI_FAB_OFFSET_ADJUSTMENT` (4 px) nearer the edge.
- **Top**: the button's centre on the app bar's bottom edge, or the button just below the
  status bar when there is no app bar.
- **Float**: 16 px above the content's bottom and clear of the gesture bar; a mini one 4 px
  lower.
- **Docked**: the three cases of milestone 757 (on the bar's edge; whole above the gesture
  bar; its margin above the keyboard).
- **Contained**: centred in the bottom bar's room above the gesture bar.
- Floating and docked buttons rise above a fixed snack bar, and sit astride a persistent
  sheet's top edge.

**A placement of one's own.** `Scaffold::fab_location` takes any `FabLocator`. `FabLocation`
is one, and so is any `Fn(&FabGeometry) -> Point`. `FabGeometry` carries the measured sizes
(shell, button, sheet, fixed snack bar), the content's top and bottom, the safe area with the
keyboard, the keyboard-free intrusions and the direction. Its methods (`start_x`, `center_x`,
`end_x`, `top_y`, `float_y`, `docked_y`, `contained_y`) are the halves the standard
placements are made of, so a custom placement can start from one. A locator also says whether
a floating snack bar goes above the button (`snack_bar_above`, false for the top placements).

**The button is measured.** `Scaffold::fab_size` is deprecated and does nothing: a docked
button of any size now sits on the bar's edge without being declared.

**Snack bars in the shell.**

- `Scaffold::snack_bar(SnackBar)` puts a bar in the shell's own layout. A bar fixed to the
  bottom (the default) stands on the bottom bar and the footer, held off the gesture bar when
  nothing below the body does, and the button rises 16 px above it. A floating bar goes above
  the button (and the bottom bar), and the button stays put. The bar's own placing decides,
  then the theme's.
- `Scaffold::snack_bar_widget(widget, behavior)` takes a bar of any make, a bar in a fade for
  instance, with the placing said there.
- **The application's bar** (`host::show_snack_bar`) is now shown by the shell at the top of
  the screen: a `primary` shell whose page put no bar of its own takes it into its layout, and
  the root's overlay along the window's bottom edge is then skipped for that frame (a flag the
  shell sets while it lays the bar out, read when the overlays are placed). Without such a
  shell the bar is shown where it was. `Scaffold::host_snack_bars(false)` declines. Until now
  this bar was drawn over the window's bottom edge, on top of the bottom navigation bar, with
  the button under it unmoved.

**Bottom sheets.** `Scaffold::bottom_sheet` is a modal sheet: its scrim covers the button,
which is what a modal sheet does to it, so nothing changes there. The new
`Scaffold::persistent_bottom_sheet(widget)` is the non-modal kind: it stands on the bottom
widgets, as wide as the body, at most as tall as the room between the app bar and the bottom
widgets, and a floating or docked button sits astride its top edge.

**The notch follows the button** (`bottomappbar.rs`). The scaffold hands the bar a shared cell
that its layout fills with the button's box, in the bar's coordinates, before the bar is
painted. The bar cuts the notch only when that box, grown by the notch margin, is astride the
bar's top edge. A button lifted off the bar by the keyboard or a snack bar, a floating one,
and a contained one cut nothing.

**The demo.** The task list's "Saved" notification is now the shell's snack bar
(`snack_bar_widget`, with its fade-in and fade-out kept) rather than a `ScaffoldMessenger`
layer stacked over the screen, so it stands on the navigation bar and the floating button
rises above it.

## How it was tested

New `scaffold.rs` tests, reading the painted rectangles:

- `every_location_lands_where_it_belongs`: all 19 placements, left to right and
  right to left, on a 400 × 800 phone with a 56 px app bar, a bottom bar 104 px up from the
  edge and a 48 px gesture bar, against the numbers worked out by hand in the test's comment.
- `a_location_of_ones_own_is_honoured`: a fixed point, and one built from `end_x` and
  `float_y`.
- `a_persistent_footer_lifts_the_button`.
- `a_snack_bar_lifts_the_button_or_goes_above_it`: a fixed bar on the bottom bar with the
  button 16 px above it; a floating bar just above an unmoved button.
- `a_persistent_sheet_carries_the_button_on_its_edge`.
- `the_notch_follows_the_button`: notched when docked; not when the keyboard or a snack bar
  lifted the button, not for a contained or a floating button.
- `the_application_s_snack_bar_is_shown_in_the_shell`: `host::show_snack_bar`, then the pages
  under the root's notification overlay as the application's root builds them. The bar is
  painted once, on the bottom bar, and the button rises above it.

**Failing before.** The three tests that need nothing new from the API were run against the
previous `scaffold.rs` and `bottomappbar.rs` (the rest of the tree kept): the footer test
failed (the button's bottom was 736, inside the footer, which starts at 672), the notch test
failed on the keyboard case (the bar was still cut), and the application's snack bar test
failed (the bar was painted at y 707 to 752, over the bottom bar, which starts at 696,
rather than standing on it). The other new tests use placements and slots that did not exist.

All the earlier `scaffold` tests pass unchanged except for the removal of their `fab_size`
calls (68 tests in the module).

RESULTS_FULL

## Checked on the phone

PHONE

## Not done

- **The button does not animate** between places. The leading toolkit moves it with a short
  scale-and-move transition when its location changes, and a fixed snack bar grows in, so the
  button rises with it. Here it jumps to its new place on the frame the bar appears.
- **A fixed snack bar's background stops above the gesture bar** when nothing is below the
  body: the shell holds the whole bar off the edge rather than padding inside it, as the bar
  has no safe area of its own.
- **Only one shell hosts the application's bar at a time in spirit, but every primary shell on
  screen shows it**: during a page transition both pages' shells show it, as the leading
  toolkit's do. A nested shell must say `primary(false)` or `host_snack_bars(false)`.
- **The material banner** slot of the leading toolkit's scaffold (between the app bar and the
  body, moving `content_top`) does not exist in frus, and is not in `FabGeometry`.
- **The body is still told the bottom bar's declared height**, not its measured one, when it
  runs under the bottom widgets (`extend_body`): the description is decided before layout.
- **The modal sheet** keeps covering the button, as intended; a persistent sheet that can be
  dragged or dismissed is the application's to build.
