# Milestone 306 — Ink, and a title that knows where it is

Two things the reference does that this framework did not.

## The ink

A material surface answers a tap with a circle of ink that grows from under the finger,
drifts towards the middle of the surface and fades. There was none: `grep -ri
"ripple\|splash"` across the widgets and the core came back empty. Pressing anything
changed a colour and nothing moved.

It is now transcribed rather than approximated. From the reference's `InkRipple`:

| | |
|---|---|
| fade-in | 75 ms |
| radius, finger still down | 1 s |
| radius, once the tap is confirmed | 225 ms |
| fade-out, confirmed | 375 ms |
| fade-out, cancelled | 75 ms |
| the fade-out's dead zone | the first 225 ms of its 375 |
| starting radius | 30 % of the target (a diameter of 60 %) |
| final radius | the target **+ 5 px** |
| target radius | half the box's diagonal |
| curve, radius **and** drift | `ease` — `cubic-bezier(0.25, 0.1, 0.25, 1)` |

Three timelines running independently, which is the point of copying it rather than
inventing something: the alpha can still be rising while the radius is most of the way
out, and a *held* press swells slowly for a whole second before the release makes it
hurry. That is why a long press on a button feels different from a tap, and no single
duration reproduces it.

One of the reference's own oddities came along with it, deliberately. Cancelling a
splash mid fade-in **raises** its opacity for a moment: the fade-out timeline it is
dropped into does nothing before three fifths of the way through, so a ripple at half
alpha is handed a value that maps to *full*. Over 75 ms it reads as a flick. It is
written down in `Ripple::cancel` rather than quietly smoothed, because a difference from
the reference should be a decision, not a drift.

### Where the pieces live

A ripple outlives the frame that started it, so it cannot live in the widget — the tree
is rebuilt every frame. The three parties:

- the **shell** knows a finger went down, and where;
- the **runtime** holds the ripples (`Runtime::ink`), advances them, and sweeps up;
- the **paint walk** knows where the box is, and paints them.

A widget opts in through `Widget::ink(&Theme) -> Option<InkStyle>` — the theme is handed
over so a coloured surface can splash in its own `on` colour rather than the one a plain
surface would use. `InkWell` is the ready-made wrapper, and `Button` takes ink now.

The splash is painted **over the widget's own paint and under its children**, which is
where a material surface puts it. For a widget that paints its own content — `Button`
draws its label itself, it has no child — the ink therefore lands *over* the label. At
16 % alpha that is a tint, not a veil, but it is a difference from the reference and it
is written in the trait's documentation rather than left to be discovered.

### Two things that would have been silently broken

**The box.** A splash needs the surface's *whole* box: where inside it the finger landed,
and how far the circle has to travel to cover it. The click registry records a target's
**visible** part, so a half-scrolled row would have splashed at the wrong size, from the
wrong point. `inks` is its own registry, and — this is the part that is easy to get
wrong — wired through every place the others are: the repaint-boundary capture and
replay, the barrier truncation, and the transform remap. A registry filled in the walk
but absent from the boundary cache would work perfectly until the first cached frame,
then quietly stop starting splashes.

**The cache.** A repaint boundary replays its primitives when its fingerprint is
unchanged, and a ripple is not part of any widget's configuration — so a splash inside a
cached subtree would have been painted once and then frozen. The ink's motion is folded
into `hash_statuses`. While a splash is alive the boundary misses every frame and
repaints; once the ink dries the hash returns to what it was and the boundary starts
hitting again, which is the behaviour worth having and not merely a disabled cache.

### The sweep

The shell confirms a tap that lands on the widget it started on. Everything else — a
finger that slid off, a widget unmounted mid-press, a gesture taken over by a scroll —
leaves a splash that nothing will ever confirm. `advance_ink` cancels any ripple whose
widget is no longer the pressed one, so the ink leaves in 75 ms instead of sitting there
for the life of the application. There is a test for exactly that, because it is the
kind of leak that never shows up in a screenshot.

## The title

`AppBar` centred its title only when asked, on every platform, and its own documentation
argued the case: *"which of the two reads better is a platform convention and a house
style, not something a bar can work out for itself, so it is asked for rather than
guessed."*

The reference disagrees, and it is right. Its bar resolves
`centerTitle ?? theme.centerTitle ?? platformCenter()`, where `platformCenter()` is
`true` on Apple's platforms **while there are fewer than two actions**, and `false` on
Android, Fuchsia, Linux and Windows. A framework that ships an application to iOS with
a left-flush title has produced something that does not look like the platform it is
running on — and that *is* something a bar can work out for itself.

`AppBar::center_title` now overrides a default that follows the platform, resolved at
compile time from the target like `ScrollPhysics::platform_default`. The clause about
two actions matters as much as the platform one: a centred title squeezed between a
leading and three buttons is neither centred nor readable.

## Verification

- Ten unit tests on the ripple itself, each pinned to a number from the reference rather
  than to whatever the implementation happened to produce: the 30 % start, the half
  diagonal, the second-long held swell, the 375 ms fade, the dead zone before it, the
  drift to the centre, the second tap that does not cancel the first.
- Two through the **walk**: that the ink is a shape-clipped layer painted before the
  child on top of it, and that the registry hands back the whole box and not the visible
  part.
- One on the sweep: held for a fifth of a second, then abandoned, and gone within
  75 ms.
- One on the title, which asserts the platform rule in both directions and holds
  whichever platform it is compiled for.
- 991 workspace tests, `clippy` silent on every target, `rustdoc` clean under
  `--all-features`, and 117 pixel tests with **no golden changed** — which is expected
  rather than lucky: a golden is rendered with no finger on it, and dry ink paints
  nothing.
- **On the device**, and not by eye. Two screenshots taken while a finger was held on a
  button look nearly the same to a person: a splash at full radius covers the whole
  surface, and so does the press state layer that was already there. Eyeballing it would
  have "confirmed" a ripple that never moved.

  So the frames were **subtracted** instead. Three captures taken *on the phone* — one
  `adb shell` round trip, not three — with the press at `x = 600` on a button spanning
  `551..968`:

  | | changed region |
  |---|---|
  | idle → first frame | the whole button, `x 550..968` |
  | first → second frame | `x 836..968`, centred at 906 |
  | second → third frame | nothing |

  The band that lights up between the first and second frame is at the **far edge from
  the finger**, and the last thing to be reached. That is a circle growing outward from
  where it was touched, clipped to the button — which is the claim, and the only reading
  of those numbers. The third frame is identical to the second because the swell was
  already over: a held radius takes one second, and an `adb` round trip is not fast.

## Owed, and left

The **device check for milestone 305 is done**: the release APK, on the phone that
reported it, in landscape — the navigation stays at the bottom. It is the one thing that
note was still waiting for.

Along the way, a documentation bug worth more than it looks. `cargo apk build` panics
with `Bin is not compatible with Cdylib` on any crate that carries both a `cdylib`
library and a binary — which every frus application does — and it panics **after**
writing and signing the APK. Exit code 101, and a finished-looking file on disk: a
script that checks the status fails, a person who checks the folder does not.

The cure is `--lib`, which CI has always passed. Every page a newcomer actually follows
did not: `docs/getting-started.md` printed `cargo apk build --release` bare, and the
README, its French translation and `CONTRIBUTING.md` all printed `cargo apk run -p
frus-demo`. All four fixed, with the reason written next to the flag rather than left as
something to copy.

The chase before that was mine, and it is worth recording: three rebuilds went into
suspecting a `[[bin]]` section added two milestones earlier, on the strength of an old
log that *ended* at the signing line. It ended there because it had been truncated, not
because the build had succeeded. Reproducing against the original configuration took one
command and would have settled it first: **a log that stops where you hoped it would is
not evidence that it stopped there.**

Left for the ink: the reference's other two ink features. `InkHighlight` — the steady
wash under a hovered or held surface, distinct from the splash — and `InkSplash`, the
older feature that expands less aggressively and is what `ThemeData.splashFactory` can
still be set to. Neither is needed to make a tap feel right; both are needed before
`splashFactory` means anything.
