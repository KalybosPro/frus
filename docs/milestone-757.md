# Milestone 757 — The floating button with the keyboard up: placed from the content's bottom, docked or not

## Objective

Milestone 752 left four things open on the phone (Huawei STK-L21, Android 10):

1. In landscape with the on-screen keyboard up, the floating action button stopped below the
   status bar and covered the right end of the app bar.
2. The landscape page kept about 80 px with SwiftKey, and the app bar gave no room.
3. A **docked** button with no bottom bar under it was placed from the window's edge, and
   nothing tested it.
4. Selectable text, which now waits on a finger the way a field does, was not tried on the
   phone.

This milestone settles what a `Scaffold` should do in each case and makes it do that.

## What a scaffold does with the button

The button's place is computed from three numbers, and nothing else:

- **The content's bottom**: where the body stops. That is the higher of the widgets below the
  body (bottom bar, footer) and the keyboard, when the scaffold resizes for it.
- **The keyboard's inset**: the keyboard's height when the scaffold resizes for it, otherwise 0.
- **The bottom safe area for the button**: the gesture bar's height (`view_padding.bottom`).
  It is 0 while a keyboard the scaffold resizes for is up, because the keyboard covers it.

From those:

- **Floating** (`StartFloat`, `CenterFloat`, `EndFloat`): the button's bottom is its margin
  (16) above the higher of the content's bottom and the safe area. **Nothing holds it from
  above.** When the keyboard leaves less room than the button needs, the button moves over the
  app bar, then over the status bar, then past the window's top edge. It is not stopped below
  the status bar, and it is not hidden. An application that wants the button out of the way
  while the keyboard is up removes it itself (the keyboard's height is in `MediaQuery`).
- **Docked** (`StartDocked`, `CenterDocked`, `EndDocked`):
  - With a bar tall enough to show half the button above what the keyboard covers, the
    button's centre sits on the bar's top edge, as before.
  - Otherwise, with no keyboard, the button is kept **whole** above the gesture bar. With no
    bar this is the case the window's edge used to decide: half the button hung off the window.
  - Otherwise, with a keyboard, the button stands its margin above the keyboard, like a floating
    one. It used to straddle the keyboard's top edge, half behind the keyboard.

**The page in landscape** is what the keyboard leaves below the app bar. The app bar is not
collapsed and does not shrink for the keyboard: a scaffold leaves that to the application (a
collapsing app bar in a scroll view, or a page without one while editing). Nothing changed
there: `the_keyboard_covers_the_bottom_bar_instead_of_lifting_it` (milestone 752) already pins
the landscape body to the room between the app bar and the keyboard.

## What changed

`scaffold.rs`, the floating button's layer:

- The clamp milestone 752 added (`fab_bottom.min(height - insets.top - fab_size)`, "never above
  the status bar") is gone. The button is placed from the content's bottom alone. The layer is
  a column justified to its end, so a button lifted past the top runs off the window's top edge
  rather than squeezing the column.
- The floating placement reads the safe area as above (`view_padding.bottom`, 0 under a
  keyboard the scaffold resizes for), not `padding.bottom`. The two differ only when a keyboard
  is shorter than the navigation bar.
- The docked placement has the three cases above.

The bottom bar's notch is still cut around the button's centre on the bar's edge. A docked
button lifted off the bar by the keyboard leaves the notch where it was, under the keyboard.

## How it was tested

New `scaffold.rs` tests, which read the painted rectangles. Each failed before the change, with
the old code put back and the rest of the file kept:

- `a_tall_keyboard_lifts_the_button_over_the_app_bar`: an 846 × 412 window with keyboards of
  300, 340 and 380 px. The button's bottom is 16 px above the keyboard's top every time; at 380
  its top is 40 px above the window. Before, the 380 px case stopped the button at the status
  bar.
- `a_docked_button_without_a_bar_sits_above_the_gesture_bar`: with no bottom bar the docked
  button's bottom is on the gesture bar's top (48 px up), on the window's edge with no gesture
  bar, and still 48 px up under a keyboard the shell does not resize for. Before, it hung half
  off the window.
- `a_docked_button_floats_clear_of_the_keyboard`: portrait and landscape, with and without a
  bar, a 220 px keyboard puts the button's bottom 16 px above it. With the keyboard down and a
  bar, the centre is still on the bar's top edge.

`the_button_stays_below_the_status_bar` (milestone 752) asserted the clamp that was removed,
and was replaced by the first test above. The other milestone-752 tests pass unchanged.

After the change, the whole `scaffold` test module passes (56 tests). `cargo fmt --all
--check` and `cargo clippy --workspace --all-targets -D warnings` pass. The `frus-test` suites,
goldens included, were run under WSL (19 test binaries, `--no-fail-fast`): all pass, and no golden changed.

## Checked on the phone

frus-demo was built with `cargo apk build --release` (7.1 MB, debug keystore) and installed over
the 0.2.1 already on the phone. Screenshots were taken after every step.

1. **Landscape, keyboard up** (`user_rotation` 1, SwiftKey). A tap on "New task" focused it
   and opened the keyboard (`mInputShown=true`). The keyboard's top is at about 314 px of 1080.
   The floating button's bottom is about 42 px (16 dp at this density) above it: the button
   stands over the right end of the app bar, under the overflow button and "Settings →", below
   the status bar. This is the placement the new rule gives for this room (the content's bottom
   less the margin); the room here was large enough that the removed clamp did not apply.
2. **A hold on selectable text selects.** On the About page, in portrait, a finger held for
   1.2 s on "framework" in the `SelectionArea` selected the word, with both handles and the
   Copy / Select all bar.
3. **A swipe on selectable text scrolls.** On the same page in landscape (where it scrolls), an
   `input swipe` starting on "Responsive primitives", inside the `SelectionArea`, scrolled the
   page by the swipe's length. Nothing was selected.

After the session the 0.2.1 that was on the phone was installed back (`install -r -d`, since the
build carries a higher version), the rotation settings were put back as found
(`accelerometer_rotation=1`, `user_rotation=0`), and no `adb` forward or reverse mapping was
made.

## Not done

- **The other placements the leading toolkit has** (top ones, mini ones, the "contained" one
  centred in a bottom app bar) do not exist in `FabLocation`, and were not added.
- **A persistent footer does not lift the floating button.** The button is placed above the
  bottom bar and the keyboard, but the footer's height is not measured before the button is
  placed, so a button over a tall footer can cover it.
- **A snack bar and a bottom sheet** do not move the button either. Neither is part of this
  layout in frus (both are overlays).
- **The notch** of a bottom app bar is cut under a docked button even when the keyboard has
  lifted the button off the bar. The bar is under the keyboard then, so nothing shows it.
- **A docked button with no bar, on the phone**: the demo has no such screen, so only the layout
  tests cover it.
- **The selection highlight on the About page** sat a little right of the word "framework" (it
  began after the "f"). The selected text itself was not checked (no paste). Not investigated.
- **The Web** was not run.
