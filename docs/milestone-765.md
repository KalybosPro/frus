# Milestone 765 — The Web on a real phone

## Objective

Several milestones changed how a frus application behaves in a phone's browser and checked it in
headless Edge only: the page around the canvas (749, 751), the editing element and the keyboard
(711, 750, 752), the selection highlight of rich text (760), the WebGL 2 fallback (745). This
milestone runs all of it in Chrome on a real phone, and fixes what that finds.

## How it was run

- **Phone:** Huawei STK-L21, Android 10, Chrome without WebGPU (frus draws through WebGL 2),
  SwiftKey keyboard, 1080 × 2340 at a device pixel ratio of 2.55.
- **Application:** `frus-demo`, built with `frus build web` from this branch (plus, for the
  rich-text check only, the unmerged branch of milestone 760, merged locally and not committed),
  served from the PC with `python -m http.server` and opened on the phone through
  `adb reverse tcp:8765 tcp:8765`.
- **Driving it:** screenshots with `adb exec-out screencap`; the page read over Chrome's
  DevTools (`adb forward tcp:9222 localabstract:chrome_devtools_remote`), with listeners added
  from DevTools that recorded focus, pointer, viewport and selection events, and stack traces of
  every `focus()`, `blur()` and `remove()` the page made.

**`adb shell input` is not a finger to a web page.** Its taps and swipes reach Chrome with
`pointerType` empty (Android's `input` makes events with an unknown tool type), and winit treats
any pointer that is not `"touch"` as a mouse: a press is a click there, decided on the way
down. The first run, with `input motionevent`, saw the keyboard close at the start of every
scroll — milestone 750's bug, apparently back. It was the injection. The phone refuses raw
`sendevent` (SELinux), so the touches that count were sent with DevTools'
`Input.dispatchTouchEvent`, which Chrome reports as `pointerType: "touch"`, the way a finger is.
The soft keyboard was typed on with real `adb` taps on its keys.

## What the phone showed

| Check | Result |
|---|---|
| The page fills the screen (749) | Yes. Portrait: canvas from 36.18 to 787.84 CSS px, the visual viewport's height, document `scrollHeight` 788 = `innerHeight`. Landscape: 36.18 + 303.03 = 339.2 = `innerHeight`. No band under the canvas. |
| The bars take the application's colours (749, 751) | Yes. Status bar and toolbar `#121418` = the app bar; the system navigation bar `#242830` = the bottom navigation bar, in portrait and in landscape (sampled pixels). `theme-color` `#121418`, root background `#242830`. |
| Safe area (751) | Nothing to see: the phone has no notch; insets stay zero. |
| Tap a field: keyboard, typing (711) | Yes. The keyboard opens, the field stays visible above it; `adb input text` and taps on the keyboard's keys (composition, suggestions) both type into the field. |
| Scroll with the keyboard up (750) | Yes, with touch events: the page scrolls, the field keeps the focus, the keyboard stays. |
| A swipe that starts on a field scrolls (752) | Yes: the page scrolled, the field was not focused, no keyboard. |
| A hold selects, with the selection bar | Yes: "milk" selected, Cut/Copy/Paste/Select all, both handles; the element's selection followed (4–8). |
| The bottom bar under the keyboard (752) | Yes: the keyboard covers the bottom navigation bar. |
| Rich text with letter spacing (760) | Yes, with milestone 760's change: the highlight and the handles cover "framework" exactly at 2.55×. |
| Background and back (745) | The page draws on return (no `webglcontextlost`: Chrome kept the context), the field keeps its text and its focus. |
| Rotation | Both orientations lay out to the new size; the field's draft survives. **In landscape, focusing the field made the keyboard go up and down without end** — below. |

## The bug: the keyboard went up and down in landscape

In landscape the keyboard leaves Chrome's page **38 CSS pixels** — less than the application's
app bar. Tapping the task field opened the keyboard, and then the keyboard closed, opened,
closed — 48 times in the half-minute it was watched — while nothing could be typed.

The recorded events made the cycle plain: the field takes the focus → the editing element is
made and focused → the keyboard comes up and the viewport becomes 38 px → the next frame
**removes the editing element** → the browser's focus goes, and the keyboard with it → the
viewport is 339 px again → the next frame makes the element again and focuses it → the keyboard
comes up …

**Root cause.** `App::sync_web_editing` described the focused field to the editor only if the
frame had a semantics node for it, i.e. had laid it out. With 38 pixels the scaffold's body has
no height and the field is not laid out, so the shell passed "no field" and the editor took the
element away, although the shell's focus had not moved. The keyboard follows the browser's
focus, so it went. Android's own build does not do this: its keyboard follows the focus alone
(`sync_soft_input` never asks where the field is).

**Fix.** The editing element now goes with the focus, not with the layout
(`crates/frus-shell/src/web_editing.rs`, `editing`, `Editor::hold`): when a field that takes
typing has the focus but the frame did not lay it out, the element is left as it is — over the
place the field last was, focused, the keyboard up. An element that is over a *different*
field is still taken away, so typing never goes to a field that lost the focus. When the field
is laid out again the element follows it as before.

## How it was tested

- `frus-shell` `web_editing::tests::a_focused_field_with_no_room_keeps_its_element`: a focused
  typing field with no layout holds its element; one with a layout gets it over itself; no
  focused typing field, no element. With the old rule ("no layout, no element") it fails:
  `left: Gone, right: Hold`.
- `frus-shell` `web_page::tests::a_visual_viewport_with_no_height_is_not_a_keyboard`: the
  0-pixel visual viewport Chrome reported on the phone gives no keyboard cover. With the old
  rule it fails: `left: 221.18, right: 0.0`.
- `frus-shell` tests (454 + 13 pass), `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo clippy -p frus-shell --target wasm32-unknown-unknown -- -D warnings` (the changed code
  is compiled for the Web only).
- On the phone: see "Re-check" below.

## Re-check on the phone

With the fixed build (`frus-demo`, built from this branch at 18:44, loaded with the cache
disabled and checked by its transfer size), touches sent through DevTools:

- **Landscape, tap the task field:** the keyboard came up once and stayed. Over the next
  15 seconds the visual viewport changed size once (to 17 CSS px; Chrome's "No internet
  connection" bar took the rest), the editing element kept the focus, and nothing went up and
  down. `adb input text` typed "Recheck" into the field.
- **Rotate to portrait with the keyboard up:** Chrome reported the visual viewport **0 px
  tall** again, with `innerHeight` still 318. The canvas was laid out in 281.86 px (318 − the
  page header), the field showed "Recheck" with its caret, focused, the keyboard up — not laid
  out in nothing. Between the page's bottom and the keyboard Chrome left a band of its own
  colour: the layout viewport keeps its landscape height until the keyboard goes.
- **Close the keyboard (back):** the viewport went 318 → 767 and the canvas filled the page
  again (730.48 px); the field kept the focus.

## The second fix: a visual viewport with no height

The rotation above showed a second fault: Chrome's 0-pixel visual viewport was read as a
keyboard covering the whole canvas, so the application laid itself out with no room.
`keyboard_cover` (`crates/frus-shell/src/web_page.rs`) now reports no cover for a visual
viewport under one pixel tall, and believes the layout viewport, which Chrome has already
shrunk. Test: `web_page::tests::a_visual_viewport_with_no_height_is_not_a_keyboard`, which
failed before the change (`left: 221.18, right: 0.0`).

## Also seen

- **A selection does not survive going to the home screen.** With "milk" selected, leaving
  Chrome collapsed the selection to a caret after the word. The trace shows the browser itself
  moving the element's selection (a `selectionchange` to 8–8) before the window lost its focus;
  frus followed the element, as it must. Not changed.
- **After coming back, the keyboard is not shown again** although the field keeps the focus;
  Chrome does not reopen it. A tap on a button then shows the keyboard for an instant: the tap
  briefly focuses the canvas, the editing element takes the focus back (milestone 750), and
  Chrome treats that as a reason to show the keyboard before the tap moves the focus to the
  button. Not changed.
- **One frozen page.** Rotating back to portrait while the keyboard was cycling left the canvas
  at its landscape size, with `requestAnimationFrame` never firing; leaving Chrome and coming
  back brought it back at once. That is Chrome's compositor, not frus (no animation frame was
  delivered to the page at all), and it happened only during the cycle fixed here.
- **A task was added once without being asked for.** During the first rotation the field's
  draft became a task. The same steps repeated with DevTools watching every key and input event
  did not do it again, and no submit was seen. The adb server was restarted by another
  program at that moment (an emulator was attached to the same machine and every port mapping
  was dropped), so an input meant for another device is the likeliest cause. Not reproduced.

## Not done

- **Landscape with the keyboard up is still not usable for typing in a field you can see:** the
  page keeps 38 CSS px, under Chrome's toolbar and the page's own header, and the field is not
  in it. The keyboard now stays and typing reaches the field, but blind. Phones solve this with a
  full-screen keyboard (Android's extract mode) that a browser does not use; frus does not
  collapse its app bar for the keyboard either (milestone 752).
- The floating button still pokes over the app bar in landscape with the keyboard up
  (milestone 752's note).
- No notch: the safe-area insets were not seen non-zero on a phone.
- iOS Safari was not run.
