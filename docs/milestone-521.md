# Milestone 521 — A sheet thrown to its top, and the list that goes on

Milestone 515 left this on its list: *a throw that reaches full height does not carry on into
the list — the throw stops at the top.* On a phone that reads as the list having nothing more to
show. A flick up a half-height sheet's list raised the sheet to the top of the screen and stopped
there, the places where they were, the momentum of the flick simply gone.

## What the reference does

Its sheet and its list are one scroll position. A release goes to the sheet when the sheet can
still move, and while the sheet settles, every tick asks one question: *is it moving up, and has it
reached its top?* The moment it has, what is left of the motion's velocity — plus the tolerance,
so it does not die on the spot — is handed to the list as a fling of its own, and the sheet's
motion stops. A snapping sheet arrives at its stop at the snap's speed, so a sheet flicked to its
top carries its list on at that speed; a coasting one hands over whatever it had left.

It hands over only a throw that was going up in the first place, and one that did not start at the
top: a release at full height goes to the list directly, and a still release gets no hand-over
even when it snaps up the last few pixels.

## The fix

**The release remembers the list, and the settle hands it the throw.** `SheetState::release` now
takes the list the finger was on, and keeps it only for a throw up that starts short of full
height. While the sheet settles, the moment a motion going up is at the top — a snap done on its
stop, or a coast past it — the sheet records a hand-over: that list, and the motion's velocity
plus the tolerance. It stops there; the list takes it from there.

**The runtime keeps the hand-over; the shell flings it.** A fling is started under the *list's*
physics, which are the application's to choose, so the sheet does not start it itself.
`Runtime::advance_sheets` collects hand-overs and `Runtime::take_sheet_handovers` gives them up;
the shell, right after stepping the sheets, flings each list at its velocity. The test harness's
`Stage` does the same, so a golden of a sheet in motion sees what the shell sees.

**A sheet thrown by its handle** has no list under the finger. When it holds exactly one, that list
goes on — as in the reference, whose whole content is the list. With several, nothing does.

## Verification

**Tests.** A snapping sheet flicked up from just above half arrives at its top and hands its list
exactly the snap's speed plus the tolerance; a coasting one thrown hard hands over less than it was
thrown with. Nothing is handed over by a throw that settles lower, a throw down, a release with no
list, a still release at the top, a flick up at the top, or a still release just under it that
snaps up. And on the demo's real page, stepped as the shell steps it, a release up the list reaches
full height, is handed over once, and the list's offset goes on growing.

**On the device** — the Huawei STK-L21, a 200 ms flick up the demo sheet's list from half height,
traced: the release went to the sheet at 2014 px/s, the hand-over went to the list at 2016 px/s,
and the capture after it showed the sheet at full height and the list scrolled on to its twentieth
and last place.

The first attempts showed nothing of the kind, and not because of this milestone: the release was
read at a velocity of nought, so the sheet settled on its nearest stop and had no throw to hand
over. That was the velocity tracker reading a flick as a finger at rest whenever frames were slow,
and milestone 520 is its fix. This one was verified on top of it.

**Mutations** — nine, seven killed: a still release handing over, a sheet already at the top
handing over, a snap handing over nought, no tolerance on the hand-over, a coast handing over
nothing, the sheets' step dropping its hand-overs, and the runtime forgetting them. The two in the
shell's wiring survived — a list's release naming no list, and the list flung the wrong way. No
test drives the shell's event loop, so nothing can kill them there; the device trace above is what
covers that path, and the harness's `Stage` repeats the same wiring under test.

## What is left

- **A controller or actuator** to move a sheet from the application.
- **Keyboard and accessibility actions** that raise and lower it.
- **`BottomSheet`**, the modal one, built on this sheet.
- A throw *down* that reaches the sheet's lowest height is handed to nobody. The reference hands it
  to the list, which is at its top by then and so does nothing with it; nothing is lost.
