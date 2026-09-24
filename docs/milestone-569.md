# Milestone 569 — The web demo fills its window, sits under its own header, and the drawer opens from its button

Not part of an issue: two faults found by using the demo across its pages, on a phone and in a
browser, while checking the selection bar (milestone 568). Both are older than it — the same
behaviour was reproduced on `master` — and neither is the bar's.

## The canvas did not follow the window

On the Web the demo's canvas was 900 by 680 CSS px whatever the window: in a 420 px browser it
was cropped, the right of every screen out of reach. The page's own stylesheet asks for the
opposite (`canvas { width: 100vw; height: 100vh }`), and says so in a comment.

**Cause.** The shell passed the application's `window_size()` to winit with `with_inner_size`,
and on the Web winit implements a requested size by writing it **inline** on the canvas — where it
outranks any stylesheet. So the application's 900 by 680, meant as a desktop window's first size,
became the canvas's size for good. Without a requested size winit leaves the canvas to the page's
CSS and follows it.

**Fix.** The size is a request for a desktop window; on the Web it is not made, and `window_size`
says so. Measured in headless Edge with the demo built for the web: a 420 by 800 viewport gives a
420 by 800 canvas (it was 900 by 680), and resizing the viewport to 900 by 600 and then 380 by 700
resized the canvas with it. The same page at 1000 by 760 still has a canvas 900 by 680 on `master`.

## The page's header drew on top of the application's app bar

The demo's page carried a `<header>` — the title, a sentence, a link to the source — positioned
`fixed` over the top of the canvas. The application's own app bar is at the top of the canvas, so
on the Web the two titles and the row of buttons under them were drawn on top of each other, and
a canvas that filled the window (above) would have been a canvas taller than what was visible
under a floating strip.

**Fix.** The page is a column: the header in the flow, one slim row, then the canvas taking
everything that is left (`flex: 1 1 0`, `min-height: 0`), which winit follows through the
canvas's own box. On a window under 640 px the sentence goes and the title and the link stay.
Measured in headless Edge at three sizes, the canvas is the window less a 36-px header, and its
backing store the same: 1270 by 590 gives 1270 by 553, 420 by 800 gives 420 by 764, 900 by 600
gives 900 by 563. The bottom navigation, which the fixed 900 by 680 canvas had cut off in a window
shorter than 680 px, is in view at each.

## The drawer came from the wrong side

The demo's hamburger is at the **start** of the app bar, and the drawer it opens slid in from the
**end**: on the phone and in the browser alike, the panel covered the right of the screen and left
the button that had asked for it in the dimmed part. It was an `end_drawer` (since the scaffold
was first written, in milestone 52), and `Scaffold` says that `drawer` is "the usual home of the
navigation on a phone". It is the start drawer now.

It was noticed on the Web, where at 420 px the cropped canvas hid the panel altogether: the menu
button turned to a cross and the page dimmed, and there was no drawer to see — until the phone's
screenshots were looked at again, and the panel was on the right there too.

## Verification

The demo's own tests, 75, pass. The rest is what only a browser shows: at 420 by 800 the drawer opens from the left, whole,
with every destination; before, at the same size, none of it was visible. On the phone (Huawei STK-L21,
Android 10, the demo built from this branch): the menu button opens the drawer **from the
left**, with every destination, and a tap on the dimmed page beside it closes it and returns
to the page. Not run: a browser at a size larger than 900 by 680, where the canvas now fills
the window rather than stopping at 900.

## Left

- The page's header is now above the canvas, and so no longer over the drawer's own title.
- At a device pixel ratio above 1 the canvas's backing store was measured equal to its CSS size
  in the emulated browser, which is not what a real high-density screen should give. Whether that
  is the emulation or a blur on a real one was not looked into.
