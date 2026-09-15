# Milestone 531 — A back gesture the page below cannot take

Found on the phone during milestone 528, and left for its own step.

## What was seen

The Huawei STK-L21, a release build of the demo. Home, then the drawer, then **Data table →**. On
the table, a back gesture injected from the left edge — `input motionevent DOWN 8 1400`, then moves
to x = 60, 150, 250, 350, 450 and 540 at the same height, a separate call each, a little over a
tenth of a second apart:

- **At y = 1400 px** (about 509 logical), the height of the home page's "Write code" task row, the
  table **did not slide**. A task row was drawn over it instead, offset to the right, with a green
  outline, and went away when the finger lifted. The build before 528 did the same with an empty
  outline.
- **At y = 1000 px** (about 364 logical), where the home page has its segmented buttons, the
  gesture worked: the table slid out and the home page came in behind it.

## What was happening

Two things, one after the other.

**The frame hit-tested after a push was the push's last.** The shell builds the view again only
when something asks for it or while the application's `tick` says its own animations are moving.
The tick that ends a push takes the page it left out of the state — `nav_from` becomes `None` — and
returns `false`, so that frame repaints the tree it already had: the push's last, with the home
page still in it, parallaxed 30 % to the left and covered by the table. Every frame after it did the
same, and it is that tree and that frame's registries a press is tested against. Covered or not, the
home page's rows were still there to be found.

**The back gesture's press still looked for something to hold.** `pointer_down` starts
`Drag::Back` on the leading edge and returns, and the press handler then asked the frame, as it
does for every press, for a long press under the finger and an item that lifts on a hold. The long
press was gated on the press not being captured by a drag; the lift was not, because a list's
scroll is meant to keep it as a candidate. So the gesture's press armed the home row's lift, the
long-press deadline fired half a second later — the injected moves are further apart than that —
and `new_events` replaced the drag with `Drag::Item`. The page stopped following the finger, the
application's gesture was left at progress nought, and the shell drew the lifted row's ghost card
over everything, moving with the finger. The release ended the lift and put the row back.

That also explains the two pictures. Before 528 a page's identity was its index among the
navigator's children, and the home page left behind by the push was the first child of a
transition; in the frames that followed it was a different widget, so the ghost's owners matched
nothing and only its outline was drawn. Since 528 the page is keyed, and the ghost has its content.

At y = 1000 px there is nothing on the home page that lifts on a hold, so nothing was armed.

## Reproduced first

- **The frame, on the demo.** `the_last_frame_of_a_push_still_holds_the_page_it_left` pushes the
  data table on a 392.7 × 850.9 surface, ticks the way the shell does — a view built only while the
  tick says it is moving — and asks each frame for a drag source at (3, 509). The push's last
  frame has one, the home row at y 461–527; a frame built once the push has settled has none. It
  passes before and after: it is the premise, on the demo's own page, at the phone's coordinates.
- **The gesture, in the shell.** No test drives the shell's event loop, so the two decisions the
  press and the deadline make were taken out as functions, unchanged, and tested against a frame of
  the same shape — a push at 0.999, the page left behind holding a row that lifts on a hold and
  asks for a long press. `a_back_gesture_claims_nothing_under_its_finger` first checks that a press
  there with no gesture finds the row's lift and its long press, then that the gesture's press arms
  neither; before the fix it failed at *"a back gesture's press arms no lift of the row under it"*.
  `a_hold_never_takes_the_pointer_from_a_back_gesture` failed with *"the back gesture is still the
  drag (lift true, reorder false)"*, and
  `a_back_gesture_where_nothing_is_under_it_still_owns_the_pointer` failed too: a deadline with
  nothing to lift used to end whatever drag there was.
- **The frame, in the shell.** `the_frame_an_animation_settles_in_is_built_again` failed on its
  first assertion before its fix.

## What the reference does

A route's pop gesture is a horizontal drag recogniser in the gesture arena that wins the pointer
outright; while it is under way the route below is shown only as a preview and nothing in either
route receives the pointer. Its hit-testing always runs against the current render tree, which a
finished transition has already rebuilt without the page it took away.

## The decision

Both are fixed here: the second is why a row was under the finger, and the first is wrong on its own
— a back gesture over a row that lifts on a hold on the *top* page would have been taken the same
way.

- **A back gesture owns its pointer.** `hold_candidates` answers nothing for a press that started
  `Drag::Back`: no long press, no lift. `drag_after_hold` never replaces `Drag::Back`, whatever the
  press found and even with nothing found; a gesture taken away there would leave the application's
  gesture open with nothing to end it. Every other drag is decided exactly as before — a still
  scroll keeps its long press and its lift, a captured press its lift.
- **The frame an application's animation stops in is built again.** `frame_needs_build` takes
  whether the application was animating at the last frame, which the shell keeps in
  `app_was_animating`. One extra build at the end of each transition, and none while nothing moves.
  It is the shell's rule rather than each application's, since `tick` is documented as "returns
  `true` while something is still moving", and a tick that has just stopped is exactly the one that
  changed the state.

Nothing else in the press was reachable: `pointer_down` returns before it can arm a reorder, a
dismissal or a word, and a frame does not re-hover during a drag.

## Verification

**Tests.** In the shell, five: the gesture's press claims nothing over the row, and the frame
really has the row to claim; a hold's deadline leaves the gesture the drag, with its start, whether
the press found a lift, a reorder, both or nothing; at a height with nothing under it the gesture
still owns the pointer; no regression — under a still scroll, and with no drag at all, a hold on
the row still arms its long press and its lift, the deadline lifts the item where the finger is or
picks the row up for a reorder from where it was pressed, and a hold with nothing to lift ends the
still scroll; and the frame an animation stops in builds the view again, while a still frame after
a still one does not. On the demo, the push's last frame holds a home row under the phone's finger
and the settled frame does not.

The shell's 113 tests pass, the widget crate's 1594 and the demo's 62. `cargo fmt --all -- --check`
and `cargo clippy --workspace --all-targets -- -D warnings` are clean, the widget crate's and the
shell's documentation builds under `-D warnings`, and `cargo clippy -p frus-shell --target
aarch64-linux-android --no-deps -- -D warnings` is clean.

**Mutations** — six, five killed, each applied to the fixed tree alone and restored after, the
hash of the whole diff the same before and after:

- **The gesture's gate in `hold_candidates` removed**: killed by
  `a_back_gesture_claims_nothing_under_its_finger`.
- **`drag_after_hold`'s guard never matching**: killed by
  `a_hold_never_takes_the_pointer_from_a_back_gesture` and
  `a_back_gesture_where_nothing_is_under_it_still_owns_the_pointer`.
- **The guard keeping the gesture but losing its start**: killed by
  `a_hold_never_takes_the_pointer_from_a_back_gesture`.
- **`frame_needs_build` ignoring the frame an animation stops in**: killed by
  `the_frame_an_animation_settles_in_is_built_again`.
- **The gate too broad, a still scroll claiming nothing either**: killed by
  `a_hold_on_a_row_without_a_back_gesture_still_lifts_it`.
- **The frame loop never updating `app_was_animating`**: **survived**. No test drives the shell's
  event loop, and this is the one line that feeds the function from it; the same goes for the press
  handler passing the drag to `hold_candidates`. The device run is what covers those two lines.

## On the device — not run here; left to the lead

The same steps as above, on a release build of this commit:

- Home → drawer → **Data table →**, then the gesture at **y = 1400** with moves that take longer
  than half a second in all: the table should slide with the finger, with no row drawn over it, and
  the home page should come in behind. Released half-way, it should settle back or complete as
  usual.
- The same at **y = 1000**, which should be unchanged.
- On home, a hold on a task row with no gesture should still lift it, and a sideways swipe should
  still dismiss it.
- After the push has settled, a tap on an empty stretch of the table at the height of a home
  button should do nothing.

## What is left

- **A device run**, above.
- **The page below during a gesture is still in the frame's registries.** Nothing reaches it now,
  since the gesture owns the pointer until the finger lifts, and the frame after the gesture settles
  is rebuilt. A second finger during a gesture is not modelled at all: the shell tracks one pointer.
