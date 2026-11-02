# Milestone 735 — A canvas as small as the screen

## Objective

The shell asks a new window for a minimum of 360 by 280, so that nobody drags a desktop window to
something absurd. It asked on the web too, where winit carries the minimum to the canvas as
`min-width` and `min-height`. On a screen or in a frame narrower than 360 px, the canvas stayed
360 wide, and everything past the edge was cut off: a small phone, a phone held in a frame of a
page, a foldable's outer screen. `MediaQuery` reported the canvas's size, not the screen's, so an
application could not even adapt to it.

## What was done

The minimum is now asked for everywhere but the web, where nobody drags anything: the page decides
the canvas's size.

## Checks

`scripts/web-small-screen-check.py`, in a real browser: the demo at 320 by 240 and in a frame of 300
by 200. Without the change the canvas stays 360 by 280 in both; with it, it is as wide as the screen
or the frame, and ends inside it. Web-only code has no native test host, so this script is the test.

## Not done

What an application looks like at that size is its own: a layout that needs 360 px now overflows
inside the canvas, where its own scrolling or `MediaQuery` can deal with it, rather than past it.
