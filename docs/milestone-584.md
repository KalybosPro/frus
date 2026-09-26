# Milestone 584 — A drag test that failed in CI: gestures timed by the driver's frames

`a_free_drag_leaves_the_scroll_its_own_direction` (milestone 582) failed once in CI: a page
dragged down by a finger read an offset of 0. It had failed once locally too, in a run of the
whole group, and passed alone and in 25 runs after. Milestone 582 wrote it down as not
explained.

## Why

The shell times a gesture's samples with the wall clock, for the velocity it flings with at the
release. That is right for an application. The test driver, though, sends its events a few
microseconds apart on the wall, and sends them irregularly on a loaded machine. The fling is
read off a quadratic fitted through those samples, as the reference's is, and its slope at the
last sample can come out at any speed and in either direction. A fling back up, one frame after
the release, put the page back at the top.

Shown, not supposed: with the wall's clock, a pause of 5 to 20 milliseconds slipped into the
middle of the drag failed the test every time. A pause of 60 milliseconds did not: the
estimator drops samples separated by a pause.

## What changed

The driver times gestures by the frames it has run, as it times everything else: the shell's
velocity tracker reads a clock the driver advances by `dt` each frame. An application has no
such clock and still uses the wall. The same pauses, anywhere in the drag and before the
release, from 5 to 200 milliseconds, now change nothing.

## Verification

- A new test drags down a scrolling page with a real 20 ms pause in the middle, and the page
  stays scrolled. With the driver back on the wall's clock it fails, three runs out of three.
- The whole shell suite passes with the driver's clock: no other test depended on the wall's
  timing.
