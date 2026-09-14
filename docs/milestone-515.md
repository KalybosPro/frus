# Milestone 515 — A sheet that follows the finger, and shares it with its list

Issue #39. `BottomSheet` showed a panel at the bottom of the screen and did nothing else:
it could not be dragged taller, dragged away, or rested at a peek and a full height. That
gesture is most of what a bottom sheet is — a map's place card, a player's now-playing bar,
a filter panel pulled up over a list.

## The widget

`DraggableScrollableSheet` is the reference's widget of the same name. It fills the box it
is given and draws nothing of its own: its panel lies along that box's bottom edge, as tall
as the share of it the sheet is at, and the content decides what a sheet looks like.

```rust
Stack::new().flex(1.0).layer(page).layer(
    DraggableScrollableSheet::new(places)
        .min(0.25)
        .snap_sizes([0.5])
        .on_dismiss(Msg::PlacesDismissed),
)
```

- **Heights are shares** of the box — `initial` (half), `min` (a quarter), `max` (all of
  it), the reference's defaults — so a sheet means the same thing in a phone held either way.
- **It coasts unless it snaps**, as the reference's does. `snap_sizes` gives it stops between
  `min` and `max`, and turns snapping on.
- **`on_dismiss` lets it go below `min`, to nothing**, and the message goes once it has
  arrived there — never under a finger still holding it at the bottom, which may yet bring it
  back up.
- **Its height is retained by identity and forgotten the first frame it is not in the tree**,
  so an application that removes a dismissed sheet and shows it again gets it back at its
  initial height rather than at nothing.

## The height is a layout question

The panel's height is not built into the tree: the layout reads it from the runtime, in
`effective_style`, as a `Percent` of the sheet's box — the same place an animated fraction
enters layout, and for the same reason: the share is the quantity, and what it is a share of
is the layout's to know. `effective_style` is also what the layout cache fingerprints, so the
cache misses on every frame the sheet moves and hits again once it rests. Nothing rebuilds
the tree for a drag.

## Who owns the finger

The interesting part, as the issue said. A sheet whose content scrolls is two surfaces under
one finger, and nothing at the press says which one it means. Milestone 282 settled a row
against its list **once**, at the threshold, by direction; that cannot work here, because the
answer changes in the middle of the gesture — a finger pulling a half sheet up is moving the
sheet until the sheet is full, and the list from then on, without lifting.

So the shell asks on **every movement**, and splits it (`split_sheet_drag`, pure and tested):

- **Up**: the sheet grows until it is as tall as it goes, and only what is left over scrolls
  the list — unless the list is already scrolled, in which case all of it does. A sheet grows
  only from the top of what it holds.
- **Down**: the list scrolls back to its top first, and only what is left over lowers the
  sheet.

The two parts always add up to the movement, so nothing the finger does is lost: what the
sheet cannot take past its floor goes to the list, whose own physics refuses it at the edge
and lights the glow, as it would for any list. The reference decides per movement as well,
but hands a whole movement to one side or the other; splitting it is what makes the handover
land on the exact pixel where the sheet reaches full height.

Which lists share a finger with a sheet is recorded while the frame is walked: the scroll
areas registered inside the panel's subtree. A subtree's registrations are contiguous, and a
replayed boundary splices its own in the same place, so it is a range — no ambient host
threaded through the walk, and no new field on a registry built in twelve places.

A press on the panel where nothing scrolls — the handle, the heading, a list too short to
move — drags the sheet itself, with a pointer as well as a finger: on a desktop there is no
other way to raise one.

## On release

The velocity is asked the same question (`sheet_takes_release`, the reference's rule): the
list keeps a still finger over a sheet at rest on a stop, a throw down while the list is
scrolled, and a throw up under a sheet already at full height. Everything else is the
sheet's, and the list does not fling as well.

A snapping sheet settles on a stop by `snap_target` — the paged view's rule on uneven stops:
the nearer stop when let go slowly, the next one the way it went on any flick, however short
— at a **constant speed**, the release's or 1600 px/s if that is slower, stopping dead on the
stop. The reference's motion, and not a spring on purpose: a sheet that overshot would uncover
and cover again whatever is behind it. A flick down from just under the lowest open stop is
therefore a flick to nothing, which is what puts a sheet away.

A sheet that does not snap coasts on the platform's clamping fling and stops at its own ends.

## In the demo

**Draggable sheet** on the home screen: a page, and over it a sheet with stops at a quarter,
half and all of the page, holding a handle, a heading and a list of twenty places. A flick
down from the quarter puts it away; a button on the page brings it back.

The list's end is padded by the bottom inset. The sheet reaches the bottom of the window,
under the system's navigation bar, and draws there, as the reference's does; what has to
stay clear of the bar is the content, and the device run below found the last place sitting
under the buttons, out of reach, until it did.

## Verification

**On a device** — a Huawei STK-L21, Android 10 — driven with `adb input`, the finger held
with `motionevent` where the handover had to be seen mid-gesture:

- The screen opens with the sheet at half height, the list at its top.
- A slow drag up on the **list** moved the **sheet**, not the list, and the release carried
  it up to full height.
- At full height, the same drag scrolled the list, and the sheet stayed where it was.
- One continuous drag down from there, the finger **still held**: the list came back to its
  first place and the sheet then came down below full height — the handover in the middle of
  one gesture, which is what this issue asked for.
- A drag by the **handle** to about a third, let go of still, settled on the quarter — the
  panel's top on the pixel the quarter puts it at.
- A flick down from the quarter put the sheet away; **Show the sheet** brought it back at half
  height, its old height forgotten.
- Scrolled to its end at full height, the list's last place sat under the navigation buttons.
  That is the inset fix above; rebuilt and installed again, the same scroll ended with the last
  place clear of the buttons.

**Tests.** Sixteen in `sheet`: the split both ways, the handover inside one movement, who takes
a release, the snap's speed and its stopping dead on the stop, a released sheet arriving on the
nearer or the next stop, a flick down dismissing once and only when it can, a held sheet never
dismissed, a coasting sheet, a sheet that leaves the frame forgetting its height, and the panel
read off the scene at its retained height. Three for `snap_target`. In the demo, the sheet driven
through the registries in the order the shell reads them, and the end of its list clearing a
bottom bar. In the shell, a press on a sheet that never moved still being a tap.

**Mutations**, fifteen, each failing the test meant for it — the fifteenth taking the inset
padding away again — among them a scrolled list giving
up the finger, the list not going home first, a snap overshooting its stop (which survived the
first run: the test only looked a second later, and now looks at the frame that passes the
stop), a layout ignoring the retained height, and the lists inside not being recorded.

**Not covered by a test**: the shell's own wiring — the split applied to a `Drag::Scroll`, the
release routed between the sheet and the fling. It has no harness that drives a real window;
the device run above is its verification.

## What is left

- ~~**A sheet lowered to nothing through its list** cannot be brought back without lifting the
  finger: at nought its list leaves the frame, and the gesture with it. Through the handle it
  can.~~ **Not so — see [milestone 519](milestone-519.md).** The list stays in the frame at
  nought, the gesture comes back, and a finger on the list cannot lower the sheet that far in
  the first place.
- **A throw that reaches full height does not carry on into the list**, as the reference's
  coasting sheet hands its remaining velocity over; the throw stops at the top.
- **No controller**: nothing moves a sheet from the application — the reference's
  `DraggableScrollableController` and `DraggableScrollableActuator`.
- **No keyboard or accessibility action** raises or lowers it.
- `BottomSheet`, the modal one, still does not drag; building it on this sheet is the natural
  next step.
