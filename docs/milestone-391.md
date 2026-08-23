# Milestone 391 — The third inset, and the strip of nothing above the keyboard

The reference describes a screen's intrusions with **three** numbers. We had two, and the
two we had were derived wrongly.

## What the three are

- **`view_padding`** — what the notch and the bars take, *whether or not* anything is over
  them. It does not move when the keyboard opens.
- **`view_insets`** — what is transiently covering the surface, measured from the window
  edge, so it already includes any bar underneath it.
- **`padding`** — what is **left** to avoid: `view_padding` less `view_insets`, floored at
  zero per side.

That last line is the one we did not have. Our `padding` reported the navigation bar's
height at the bottom even while the keyboard was covering it, so a `SafeArea` reserved room
for a bar nobody could see: **a strip of nothing between the content and the keys**.

## The one that does not move

`view_padding` was not merely missing — the shell was already computing it and throwing it
away. The inset baseline it keeps per physical size, to tell a keyboard apart from a bar,
*is* the keyboard-free intrusion. `from_baseline` now returns all three.

Having it matters for the case `padding` is wrong for. A screen with a flexible child grows
by exactly the bar's height the moment a field is tapped, and shrinks again when the
keyboard closes — a whole layout twitching because somebody started typing.
`SafeArea::maintain_bottom_view_padding` pads the bottom by the intrusion that does not
move, so nothing shifts. The content still sits behind the keyboard, which is what it was
already doing.

## Consuming one consumes the other

`remove_padding` now takes the same amount off `view_padding`, floored at zero, and
`remove_view_insets` does the same. The reference does both, and the reason is the same
either way: a descendant that asked for the intrusion-that-does-not-move would otherwise be
told about one its parent had already dealt with, and would inset for the notch twice on
the one path that was meant to be immune to the keyboard.

## A constructor, so a test cannot describe an impossible screen

Five tests built `WindowInsets { padding, view_insets }` by hand, and under the new rule
`padding.bottom = 16` beside `view_insets.bottom = 320` is a state no platform can report.
A test asserting against it asserts against nothing.

So `WindowInsets::bars(intrusions)` covers the no-keyboard case, and the keyboard cases now
go through `from_baseline` — the same function the shell uses. The tests exercise the real
derivation instead of hand-assembling its output, which is how the wrong `padding` survived
this long: the test that should have caught it was asserting the bug.

`the_keyboard_is_avoided_only_when_asked` now asserts **zero**, not sixteen, and that one
line is the whole fix.

## Left

`systemGestureInsets`, `displayFeatures`, `textScaler`, `platformBrightness`, and the
accessibility flags — `boldText`, `highContrast`, `disableAnimations`, `invertColors`,
`accessibleNavigation`. The next steps in the same series.
