# Milestone 520 — A throw read as a finger at rest, one slow frame at a time

Milestone 278 gave every fling its input: a velocity fitted through the last 100 ms of pointer
positions, with one rule that decides whether there is a fling at all — **40 ms since the last
position means the finger was resting when it let go**. That rule is right, and on a phone it
was wrong: a flick up a sheet's list, fast and clean, raised the sheet and released at a
velocity of exactly nought.

## What the phone showed

The sheet work of the next milestone needed a throw to arrive at a sheet's top, and none ever
did. A trace on the release, and on every movement before it, said why:

- the movements arrived **37 ms apart** — the application was drawing at about 27 frames a
  second while the sheet moved under the finger;
- the finger covered 311 px in 150 ms, about 2000 px/s;
- the release was handled **41 ms after the last movement**;
- and the tracker's estimate was nought, over nought pixels, in nought seconds.

41 is past 40. The tracker concluded the finger had stopped.

## Why

Every sample is stamped with the moment the shell **handles** the event, not the moment the
finger moved. There is no other time to use: the windowing layer hands the shell no timestamp
with a touch, on any platform, and the one underneath it on a phone knows the real time but does
not pass it up. So the stamps inherit the application's frame pace. An application drawing at
60 frames a second is handed its events 16 ms apart and a release a few milliseconds after the
last move, comfortably inside 40 ms. One drawing at 27 is handed them a frame apart — and the
release of a finger still moving can arrive a whole frame after its last movement, which is all
it took.

The reference keeps the same 40 ms rule and never meets this, because its events carry the time
they happened. Here the rule was being applied to a clock that runs in frames.

## The fix

**Allow for the pace the samples arrived at.** The tracker measures it from its own history —
the median of the last three gaps between samples, capped at 50 ms — and takes it off before
comparing with the stop:

- on release, a finger has stopped when the time since its last sample, **less that pace**, is
  past 40 ms;
- inside the history, a gap is a pause when it is past 40 ms **beyond** that pace;
- and the 100 ms horizon widens by the same amount, so frames 51 ms apart still leave the three
  samples a fit needs.

The **median**, so that neither kind of odd gap moves it. One real pause among the newest gaps —
a finger that throws, stops for 60 ms, reports where it stopped and lifts — does not excuse
itself, which the longest gap would; two events handled back to back do not take the allowance
away, which the shortest would.

The **cap**, because the allowance is one frame's worth and not a licence: an application slower
than 50 ms a frame still sees a held finger as held.

An application at 60 frames a second is almost unaffected: its pace is 16 ms, and a finger held
for 56 ms instead of 40 before lifting now reads as stopped.

## Verification

**Tests.** A throw at 2000 px/s delivered every 37, 45 and 51 ms and released 41 ms after its last
sample reads 2000 px/s, under both strategies — the regression and the platforms' weighted
average. The same throw released 120 ms after its last sample reads as a finger at rest, and so
does one delivered every 200 ms. A 5 ms gap among 37 ms ones leaves the pace at 37; a 60 ms pause
just before the release still cuts the throw off. All of milestone 278's tests pass unchanged, and
so do the core, widgets and shell suites.

**On the device** — the same Huawei STK-L21, the same flick up the demo sheet's list, rebuilt with
this fix and traced. The movements still arrived 32 to 40 ms apart and the release 41 ms after the
last one — the frames were no faster — and the release now read **2014 px/s** over 217 px in
118 ms, where it had read nought. The sheet took the throw, arrived at full height, and handed what
was left of it to the list (the next milestone), which carried on to its twentieth place.

One false start is worth recording, because it would have looked like this fix failing: a build of
the traces did not compile, and the build script — which piped the build through `tail` — copied
the APK the previous build had left and installed that. The script now deletes the old APK first
and goes by the build's own exit code.

**Mutations**, seven, in a worktree of their own. Five failed a test at once: the release judged
without the pace, a gap in the history judged without it, the horizon left unwidened, the pace
taken from the shortest gap, and the cap removed. **Two survived, and the tests were wrong**: the
pace taken from the longest gap passed, because the pause test only looked at the velocity and a
quadratic fitted across a throw and a stop happened to come out slow; and no allowance at all for
a history of fewer than three gaps passed, because no test threw that briefly. The pause test now
also requires the history to stop at the pause — none of the throw's travel, no duration — and a
throw three samples long at the slow pace was added. Run again, both fail a test.

## What is left

- **Real event times.** This allowance is the honest best without them, and the reference's
  exactness needs them: a patch to, or a way around, the windowing layer, which does not pass the
  platform's event time up with a touch.
- **The frames themselves.** A sheet moving under a finger is slow to lay out on a phone, and a
  faster frame would narrow the allowance this fix has to make.
