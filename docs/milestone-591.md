# Milestone 591 — The overscroll stretch

Issue #55 asked for two things. The second bouncing profile was done in milestone 499. The
other was the **stretch**: on current Android, pulling a list past its end does not light up
the edge, it lengthens the content towards it, and the content springs back when the finger
lets go. frus glowed everywhere, which makes an application look older than it is.

## How the reference does it

The reference's `StretchingOverscrollIndicator` has two parts.

- **A controller** ported from the platform's edge effect:
  - A drag past the edge accumulates the refused distance and divides it by the viewport's
    length, capped at one viewport.
  - The stretch is `0.016 · d + 0.016 · (1 − e^(−d · e/0.33))`, signed by the edge.
  - A fling that lands, or a finger that lets go, hands the stretch to a spring. The spring
    runs at 24.657 rad/s with a damping ratio of 0.98, slowed by a factor of 0.8 that the
    reference matched to the platform by eye.
  - A pull during the return carries on from where the spring had got to.
  - Any ordinary scroll releases the stretch.
- **An effect.** Where the engine supports fragment shaders, a shader displaces the content
  unevenly. Everywhere else, the effect is **a scale along the axis, anchored at the edge
  pulled**: `1 + |stretch|`.

## What it does

- **`OverscrollIndicator { Glow, Stretch }`** says what a scroll area shows. The platform
  default is the stretch on Android and the glow elsewhere. `Application::overscroll_indicator`
  overrides it. The shell installs it every frame, like the scrollbars.
- **`OverscrollStretch`** is the controller, one per axis. **`ScrollStretch`** holds both axes
  of a region and gives the transform. The constants are the reference's.
- The runtime's `glow_pull`, `glow_absorb` and `glow_scroll_end` feed the stretch instead of
  the glow when the indicator is the stretch. The shell and the fling code need no change.
  The shell releases the stretch when a drag moves the content again.
- The scroll view, the list and the page view put everything they paint into one layer,
  scaled from the edge pulled. What can be touched in that layer moves with it.

## Alternatives weighed

- **The shader version.** It needs the subtree drawn to a texture and sampled with a
  displacement: a render-target pass like the blur pre-pass of milestone 339. The reference
  uses the scale wherever it has no shaders, so the scale is a faithful first step. The
  uneven displacement can come later without changing the controller or the API.
- **A stretch per edge, like the glow.** A stretch deforms the whole content, so a region has
  one per axis, signed by the edge.

## Verification

- The controller:
  - a whole-viewport pull gives the reference's value exactly;
  - pulls accumulate, up to one viewport;
  - a longer pull stretches more;
  - a held pull stays;
  - letting go settles in under a second;
  - a harder impact throws further, and the stretch returns to zero;
  - a pull mid-return carries on from the spring.
- The transform holds the pulled edge, and it does not scale across the axis.
- A scroll view pulled at the top has one layer, owned by the region. The top stays and the
  bottom moves down. A button inside is touched where it is drawn. No glow is recorded. After
  release the layer is gone.
- A list hit at its end by a fling stretches from the bottom.
- The shell installs the application's choice, and the platform's when the application says
  nothing.
- **On a device**, the demo's task page on an Android phone at 1080 × 2340:
  - A 600-px drag past the top, held, lengthened the page by about 2%, from the top: the
    first row moved 18 px, the last button 31 px. The formula gives 0.020 for that pull.
  - After release the page was back where it was, with no glow.
- Mutations, each failing a test:
  - the pull's direction ignored;
  - the anchor on the wrong edge;
  - the intensity changed;
  - the pull or the impact sent to the glow;
  - the release not reaching the stretch;
  - the spring not reported as moving;
  - the stretch layer dropped from the scroll view or from the list;
  - the hit regions not moved with it;
  - the shell not installing the application's choice.
