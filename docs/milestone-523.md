# Milestone 523 — A sheet the application can move

Milestone 515 left this on its list: *nothing moves a sheet from the application.* A sheet
followed a finger, settled, coasted and handed a throw on to its list, and an application could
do none of it. There was no "raise the sheet" button, no sheet opened at full height by a search,
no way back to where it started.

## What the reference does

`DraggableScrollableController` is attached to one sheet and has three verbs. `jumpTo(size)` puts
it at a height at once, "the nearest valid size" when asked outside its bounds. `animateTo(size,
duration, curve)` gets there over a duration along a curve, clamped between the sheet's minimum
and maximum. `reset()` puts it back at its initial size. All three cancel whatever was moving the
sheet, a user's fling included, and **neither `jumpTo` nor `animateTo` snaps afterwards**, even on
a sheet that snaps: "snapping only occurs after user drags". A user touching the sheet interrupts
an animation.

It also reads: `size`, `pixels`, and the conversions between them, with listeners told as the size
changes.

## The shape here

There is no controller object to attach. An application is a function of its state, and "raise
the sheet" happens once — the same sheet, the same height, and yet it is an event — so it is an
**effect**, and frus already has the shape for an effect that moves a retained position:
`Command::scroll(key, ScrollTo)`. The sheet request is the same thing:

```rust
Command::sheet("places", SheetTo::size(1.0).animate(0.3, Curve::ease()))
```

- **`SheetTo::size(s)`** is `jumpTo`, **`.animate(duration, curve)`** is `animateTo`, and
  **`SheetTo::initial()`** is `reset`. A duration of nought or less is no animation.
- **Named by key**, like a scroll or a focus request: the sheet, or anything around it, wrapped in
  `keyed(k, …)`. One step more is needed than for a scroll — a sheet's state belongs to its
  *panel*, which the application never builds — so `find_sheet_by_key` finds the keyed widget and
  goes down to the first sheet under it.
- **Resolved on a scroll request's terms**: against the frame that follows, once more against the
  frame after it for a sheet shown by the same message, and then dropped.
- **Before the sheets are stepped**, so an animated request starts in the frame it arrives in.

The rules are the reference's, and one is the house's:

- **Kept within the sheet's bounds** — between its floor and `max`, as a finger's height is.
- **Whatever was carrying it stops**: a settle, a coast, an earlier request.
- **No snap afterwards.** A request goes where it was told and stays: 0.8 on a sheet whose stops
  are a quarter, half and all is 0.8.
- **A finger outranks it.** A finger landing on a sheet a request is moving stops it where it is,
  as in the reference. And a request that arrives while a finger is on the sheet is **refused** —
  the rule `Runtime::scroll_to` already keeps, for the reason it gives: a panel that moved out
  from under a finger because a timer fired would be the framework overruling the reader.

**One difference, deliberately.** The reference clamps to `minSize`. A frus sheet can be
dismissible, which the reference's cannot, and its floor is then nothing; asked for nothing, it
is put away exactly as a finger would put it away — once it has arrived there, with its message —
and asked back up, it is open again and may be put away again. An application closing its sheet
with an animation is the case this is for.

**The read side is not here.** The reference's `size` and its listeners answer "how tall is it
now"; that is a message from the sheet, not an effect on it, and it is left for its own
milestone.

## In the demo

The sheet screen's page has a **Raise it** button beside **Show the sheet**. It returns
`Command::sheet(PLACES_SHEET, SheetTo::size(1.0).animate(0.3, Curve::ease()))`, and the sheet is
wrapped in `keyed(PLACES_SHEET, …)`.

## Verification

**Tests.** On the sheet's state: a request moves it at once, no higher than `max` and no lower
than `min`, and `initial` puts it back; an animated one is half-way along a straight line at half
its duration, somewhere else along another curve, at its height when it is done and still there
a few seconds later on a sheet whose stops it is between; a finger refuses one and stops one under
way where it is, and a request takes over from a settle; a dismissible sheet asked for nothing is
not put away while the request is under way — a curve that arrives by half-way checks that — and
is once it is done, and asked back it may be put away again. In the shell: a request reaches the
sheet the view named through a wrapper, and another key, or a frame without the sheet, hands it
back and moves nothing; a batch keeps its sheet requests. On the demo's real page: the page's key
names the sheet the frame reports, and the request "Raise it" makes is under way after nine frames
and at full height once done.

**On the device** — the Huawei STK-L21, a release build of the demo. On the sheet screen, with
the sheet resting at half height, a tap on **Raise it** brought it to full height, and a second
capture a moment later found it still there rather than back on the half-way stop. The motion
itself, 0.3 s long, is shorter than a capture takes, so the device shows where it arrives and the
tests show how it got there.

**Mutations** — fifteen, fourteen killed: the finger's refusal, the settle left running, the
clamp, `initial`, the animation, the curve, the finger stopping a request, the dismissal waiting
for it, the dismissed flag, a zero duration, the key's own node taken for the sheet, a batch
dropping requests, an unplaced request dropped, and a placed request moving nothing. The first
run could not apply the unplaced-request mutation — its line appears twice in the shell, once for
scroll requests and once for sheet requests — so it was run again on its own with a pattern that
names the sheet's; the shell's test caught it. **One survived**: requests never reaching the
shell from a returned command. No test drives the shell's event loop, so nothing can kill it
there; the device run above is what covers that path, since **Raise it** is exactly a command
returned from `update` and resolved by the shell.

## What is left

- **The read side**: a message when a sheet's size changes, the reference's controller listeners.
- **Keyboard and accessibility actions** that raise and lower a sheet.
- **`BottomSheet`**, the modal one, built on this sheet.
