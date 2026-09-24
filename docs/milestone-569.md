# Milestone 569 — The web demo fills its window, and the drawer opens from its button

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

- The demo's page header (the HTML title and the *Source on GitHub* link) is fixed over the top
  of the canvas and sits over the drawer's own title. It is page chrome and was left alone.
