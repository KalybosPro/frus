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

**A fixed bar in a transition spans the shell** (`toast.rs`, found on the emulator). The
shell forces its width on the slot's widget, and a bare `SnackBar` took it; the demo's bar,
in an `AnimatedOpacity`, did not: the wrapper was as wide as the shell and the bar inside kept
its own measured width, at the start edge. A fixed `SnackBar` now asks to fill the width it
is offered (`fill_axes`), its measured width being only a default (`soft_extent`); a floating
bar does not ask, and keeps its own width or the one it was given.

**Where it fills is the reference's answer, not only the shell's.** The reference builds a
fixed bar's content as a row whose message is wrapped in an expanded child, the action and
the close button after it (`snack_bar.dart:767-779`); a row with an expanded child takes the
whole width it is given. So a fixed bar fills the width any parent offers it, in a shell's
slot or in a plain padded box. The `snackbar_action` golden (a fixed bar with an action alone
in a box padded by 20 on a 340 px canvas) changes accordingly: the bar now spans the 300 px
and its action sits at the right end. The new image differs from the old only right of x 200:
the bar's right edge and the "Undo" label both move 42 px to the right, and everything to
their left is identical.

That request also stretched toasts out of their corner: the `toast_host` golden (two fixed
bars at the bottom end of a `ScaffoldMessenger`, a host of frus's own with no counterpart in
the reference) failed, the first bar spanning the host. Two changes keep that golden as it
was:

- **A corner-aligned host keeps its toasts their own width.** A new hook,
  `Widget::grants_cross_fill` (`true` by default, forwarded by the transparent wrappers),
  lets a container refuse its children's requests to fill across its axis; the layout walk
  (`ui.rs`) then neither grants the request nor passes it up. `ScaffoldMessenger` refuses
  unless its alignment is a stretch, which none of its positions is. Nothing in it names
  `SnackBar`. A toast at a corner keeps its own width; the corner alignment wins.
- **A soft size no longer stops a container's request** (`Fills::bounded_by`, `ui.rs`). Of
  the two bars, only the one without an action stretched. A bar with an action or a close
  button lays them out as children, so it is built as a container, and a container whose
  style gives a definite width had its request to fill dropped, even where that width is
  only a default. It now keeps the request on an axis whose size is soft, so a fixed bar with
  an action fills as one without does. This is the change that moves the `snackbar_action`
  golden.

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
- `a_fixed_bar_in_a_transition_spans_the_shell`: a bar in an `AnimatedOpacity`, given with
  `snack_bar_widget`, is painted the shell's whole width. It failed before the `toast.rs`
  change (the bar was 97 px wide at x 0 in a 400 px shell).
- `a_fixed_bar_with_an_action_spans_the_shell`: the same with an Undo action. It passes
  with or without the `bounded_by` change, the shell forcing the slot's width; it guards the
  slot, and `toast.rs`'s column test below is the one that catches the second bug.
- `a_persistent_sheet_carries_the_button_on_its_edge`.
- `the_notch_follows_the_button`: notched when docked; not when the keyboard or a snack bar
  lifted the button, not for a contained or a floating button.
- `the_application_s_snack_bar_is_shown_in_the_shell`: `host::show_snack_bar`, then the pages
  under the root's notification overlay as the application's root builds them. The bar is
  painted once, on the bottom bar, and the button rises above it.

New in `toasthost.rs`: `fixed_bars_at_a_corner_keep_their_own_widths`, two fixed bars (one
with an action) at the bottom end of a 420 px messenger, bare and with `fade_in`, each painted
at its own width against the end edge. Without `grants_cross_fill` it failed: the first bar
was painted 420 px wide at x 0. New in `toast.rs`:
`a_fixed_bar_fills_a_column_with_an_action_or_without`, two fixed bars in a 400 px column
aligned to the start, both painted the column's whole width. Without the `bounded_by` change
it failed: the bar with the action was 258 px wide at x 0.

**Failing before.** The three tests that need nothing new from the API were run against the
previous `scaffold.rs` and `bottomappbar.rs` (the rest of the tree kept): the footer test
failed (the button's bottom was 736, inside the footer, which starts at 672), the notch test
failed on the keyboard case (the bar was still cut), and the application's snack bar test
failed (the bar was painted at y 707 to 752, over the bottom bar, which starts at 696,
rather than standing on it). The other new tests use placements and slots that did not exist.

All the earlier `scaffold` tests pass unchanged except for the removal of their `fab_size`
calls (68 tests in the module).

After merging master: the `frus-widgets` unit tests (2151), the demo's (88), `core_exports`,
`thread_locals` and the doc tests (117, 110 ignored) all pass; the goldens (`frus-test`, all
targets, under WSL with a fresh build: 105 goldens and 132 other rendering tests) all pass.
One golden changed, `snackbar_action`, regenerated after checking the difference was only the
bar spanning its box (see above); `toast_host` failed with the first version of the fix and
passes, unchanged, with the corner rule. `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets -- -D warnings` are clean.

## Checked on the phone

PHONE

## Not done

- **A floating bar without a width of its own** still keeps its measured width. The
  reference's floating bar has the same expanded content row, and without a width it is only
  wrapped in its margin (`snack_bar.dart:815-824`), so it too fills the width it is offered,
  less the margin. Only the fixed bar was changed here.

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
