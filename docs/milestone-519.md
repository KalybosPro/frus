# Milestone 519 — The sheet that was said to lose its finger, and did not

Milestone 515 closed its notes on `DraggableScrollableSheet` with a list of what was left, and
the first line of it was a defect: *a sheet lowered to nothing through its list cannot be
brought back without lifting the finger — at nought its list leaves the frame, and the gesture
with it.* This milestone set out to fix that. There was nothing to fix, and this note is here so
that nobody sets out again.

## What was claimed, and how it was checked

The claim has two parts, a mechanism and a consequence, and each was checked on its own.

**The mechanism: the list leaves the frame at nought.** The shell finds the sheet a finger on a
list belongs to through the list — `sheet_holding`, which looks the list up among the scroll
areas walked inside each panel — on every movement, on the release and on a cancel. If the list
were not walked at nought, all three would lose the sheet. It is walked. A panel with no height
still lays out its subtree, the list is still registered inside it, still found as the sheet's,
and the height the sheet's shares are taken of is still known, carried over from the last frame
that had one. Checked twice: on a minimal sheet holding a scroll view, and on the demo's real
page — handle, heading, a rounded surface and twenty places.

**The consequence: the gesture is lost.** On a phone (the same Huawei STK-L21, the finger held
with `motionevent`), a finger on the demo's first place carried the sheet down from half height
to far below its lowest stop — only the handle, the heading and the top of the first place
showing — and then, without lifting, back up. The sheet followed it down and all the way back to
half height, and a still release left it there, on a stop.

**And the exact case cannot be reached by a finger.** A press on the list is below the panel's
handle and heading, so with the finger at the bottom edge of the screen the panel is still at
least that tall: carried by its list, the demo's sheet cannot go lower than about a ninth of the
window. Only the handle, a few pixels under the panel's top, comes close to nothing — and 515
itself saw a sheet brought back that way.

## Where the claim came from

The likeliest source is an inference that was never tested: a panel with no height *looks* as if
it should have nothing inside it to hit or to register, and the note says what that would imply.
Registration does not depend on height, and hit testing is not what a gesture already under way
consults.

## What this milestone leaves behind

- **Two tests that pin what was found**, so the claim cannot quietly become true: on a minimal
  sheet, the list still shares the finger at nought; on the demo's page, the list is still walked
  inside the lowered panel, still the sheet's, the available height kept — and the next movement
  up, split as the shell splits it, grows the sheet back from nothing.
- **Milestone 515's note corrected** where it made the claim, pointing here.

No library code changed.

## What is left

The rest of 515's list stands: a throw that reaches full height does not carry on into the list;
no controller or actuator; no keyboard or accessibility action; `BottomSheet` not yet built on
this sheet.
