# Milestone 279 — The overscroll glow, and the bug a device found

## The goal

Milestone 277 gave the clamping physics its behaviour: reaching the end of a list
stops the content dead. That is correct, and on its own it is **indistinguishable
from a frozen app**. The finger drags, nothing moves, and nothing says why.

A platform that bounces answers "there is nothing more" with movement — the content
pulls away and springs back, and no other signal is needed. A platform that clamps
refuses the movement, so it owes the user an answer in some other currency. That
answer is the **glow**: an arc of light swelling at the edge the content will not
cross.

Until this milestone, half of `ScrollPhysics::Clamping` was missing.

## The glow

`OverscrollGlow` answers two events, and they look different on purpose:

| | trigger | shape |
|---|---|---|
| `absorb_impact(velocity)` | a **fling** landed on the edge | a brief, bright flash whose strength is the speed that was thrown away |
| `pull(overscroll, …)` | a **finger** is dragging past the edge | swells with the accumulated pull, holds, then decays |

Both then recede. Two quantities animate — opacity and size — each interpolated from
**where it currently is** to where the newest event asks it to go, shaped by a
decelerating curve. That "from where it is" is the whole trick: a second event
arriving mid-flight redirects the glow instead of making it jump.

Three details are worth stating because they are not obvious from the effect:

- **The arc slides towards the finger** rather than jumping to it, halving the
  remaining distance every 1/60 s — frame-rate independent, unlike a fixed step.
- **A pull left unattended decays on its own.** A finger that stops moving without
  lifting must not leave a glow burning for ever, so the pull holds for 167 ms and
  then fades over two seconds — much slower than letting go, because the gesture is
  not actually over.
- **The shape is a wide, shallow ellipse, clipped to a thin band.** The visible arc
  is the cap of an ellipse far larger than the band it shows through; scaling it
  towards the edge is what turns a round cap into a flat, wide sweep. `Path::oval`
  is new in `frus-core` for it, and generalises `Path::circle`.

The colour is the scheme's **secondary**: an acknowledgement, not a call to action,
so it must not compete with whatever primary-coloured control sits nearby.

## What feeds it

The glow needed no new measurement. Every source was already computed and thrown
away:

- **A drag** — `apply_boundary_conditions` returns the movement the physics
  *refused*, which is exactly the distance the user asked for and did not get.
- **A fling** — when a clamping ballistic hits an edge, the runtime already stops
  that axis; it now reports the velocity it stopped at.
- **The wheel** — a notch that would push the target past the end is clamped, and
  the clamped-off amount is the same overscroll. A wheel has no lift-off, so its
  pull is released immediately and simply fades.

`ScrollPhysics::Bouncing` feeds none of them: the bounce *is* the feedback, and a
glow on top of it would say the same thing twice. That is a test.

## The bug the device found

This is the part that could not have come from a test suite.

With the glow working, the clamping side looked right on the phone. Switching the
demo's log list to bouncing physics, the content **did not move either** — no rubber
band at all, where there should have been one.

The cause: `advance_scroll`'s edge spring kept retracting the offset **while the
finger was still holding it**. Each frame it pulled roughly a fifth of the overscroll
back, so a rubber band was dragged home as fast as it was stretched. Between two
slow moves of a finger it vanished entirely. On a fast drag it would merely have felt
stiff and short — the kind of wrongness that never produces a bug report, only a
vague "it doesn't feel right".

The fix is a single invariant, and it is the one mature toolkits state explicitly: a
scroll offset has **one owner at a time**. `Runtime::scroll_held` names the region a
finger is holding, and `advance_scroll` leaves it alone — no spring, no retract,
nothing. `hold_scroll` on the press, `release_scroll` on the release *and on a
cancelled gesture*, or the region would stay frozen under a finger that is no longer
there.

> The lesson generalises beyond this bug: anywhere the framework animates a value the
> user can also drive, the two need an explicit owner, not a hope that they will not
> collide.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **677 tests, 0
  failures** (657 at milestone 278).
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK.
- `cargo build --workspace --all-targets` — OK, no new warning.
- The desktop demo runs and exits cleanly.

**On a physical device** (Huawei STK-L21, Android 10) — which is the point of this
milestone, since a glow cannot be judged from a test:

- **Clamping** (the platform default there): dragging down at the top of the 5000-row
  log leaves the content exactly where it was, and a wide arc appears along the top
  edge, brightest under the finger and fading downwards.
- **Bouncing** (the demo's switch): the same gesture pulls the content ~236 px away
  from the edge and **holds it there** under the finger, with no glow.

The two screenshots side by side are the verification: same gesture, two honest
answers, and neither is silence.

The unit tests cover what the eye cannot check repeatedly: an impact flashes and ends
(no glow outlives its gesture); a harder landing shows more; the opacity is capped; a
longer pull accumulates; letting go fades it; an unattended pull decays; the arc
slides towards the finger; each of the four edges paints inside its own band; a
clamping fling lights the edge it hits and *only* that edge; a bouncing fling lights
nothing; and — the regression test for the bug above — a held offset does not move on
its own while a released one springs home.

## What's left

- **The glow is subtle on a dark theme**, because the secondary role is. It is
  overridable through the theme, but not yet per scroll area; a `Scroll::glow_color`
  would be the natural place.
- **No stretch alternative.** Newer platform versions replace the glow with a
  stretch of the content itself. That needs a render-target effect, which is a
  different kind of work.
- **`ScrollDecelerationRate.fast`** — still unported (milestone 277's leftover).
- **The pan fling has no velocity cap**, since it does not go through the physics
  (milestone 278's leftover).
