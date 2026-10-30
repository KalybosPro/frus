# Milestone 722 — Turning, typing and reading on a simulated device

## Objective

Step 3 of simulated devices (milestone 720 has the plan, 721 the screen): what a developer
does to a phone to see how an application copes. Turn it on its side, bring the keyboard up,
switch to dark, make the text larger.

## What was done

- **The keyboard comes up with a field's focus**, as a phone's does: at the end of a frame
  the shell asks the same `wants_keyboard` Android asks (a focused text field that takes
  typing; not a read-only one, not a selectable paragraph), and the simulated device's keyboard
  follows. The application is told through `view_insets` exactly as on a device. The shell
  draws the keyboard in the device's brightness: a panel, three rows of letters, shift and
  delete, the space bar and its neighbours. It is a picture: typing still comes from the
  desktop's keyboard. The home indicator then reads against the keyboard.
- **`K` holds the keyboard up**, focus or not, to see a screen under it without a field to
  focus. A second press lets it go.
- **`l` turns the device** on its side and back. The window turns with it (its room is the
  same, transposed); the application gets the landscape size and insets, the island moves to
  the left, an iPhone's status bar goes.
- **`T` steps the text size** through 100%, 130%, 200% and 85%, the sizes a reader's settings
  offer, into `MediaQuery`'s text scaler. It works on any window, simulated or not.
- **`b`** (milestone 668) already switched light and dark. The status bar's ink and the
  keyboard follow it.

The keys are `frus run`'s: `rotate`, `keyboard` and `text-scale` commands on its channel. On a
window that is not simulating, `l` and `K` say so in the log.

## Checked in the real application

The demo was driven as `iphone-15` through `frus run`'s channel, a screenshot after each step:

- **`K`**: the keyboard drawn at the bottom of the screen, the demo's navigation bar raised
  above it. The demo reads `view_insets`.
- **`T` at 130%**: every text larger. The demo shows a real overflow
  (*RIGHT OVERFLOWED BY 5.0*) in its row of chips.
- **`l`**: the window turned, the island on the left, the buttons on the long edges, no status
  bar.

That run also found two things in the demo, not in the simulator. The demo picks its desktop
menu bar by width alone (`SizeClass::Expanded`, 840 and up), so an iPhone on its side (852)
gets *File View Go*. And `WindowMenuBar` does not keep clear of the left safe area, so the bar
starts under the screen's corner. A real iPhone on its side would show both. They are left for
their own milestone.

## Tests

- `simulated_keyboard_tests::the_keyboard_follows_a_field_s_focus`: through the test driver, a
  tap on a field brings the simulated keyboard up, and a tap on plain text takes it away.
- `simulated_keyboard_tests::the_text_size_cycles`.
- `simulated::tests::the_keyboard_is_drawn_where_the_insets_say`: the keyboard's height is the
  profile's and the one `view_insets` reports, at the bottom of the screen, light or dark; held
  up without a field.
- `frus-cli`: `l`, `K` and `T` mean what they say, and the help lists them.

`clippy` also passes for `aarch64-linux-android`, since the Android keyboard path changed shape.
