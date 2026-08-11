# Milestone 281 — Pull to refresh, from a measurement already being thrown away

## The goal

Milestone 277 established that `ScrollPhysics::apply_boundary_conditions` returns the
movement the physics **refused** — precisely the distance the finger asked for and did
not get. Milestone 279 spent that measurement on the overscroll glow, which
*acknowledges* the refusal.

Pull-to-refresh spends the same measurement on something that *leads somewhere*: it
accumulates the refusal, and past a threshold turns it into a message.

So this milestone adds no new instrumentation to the gesture path. It adds a consumer.

## The widget

```rust
Refresh::new(list)
    .on_refresh(Msg::Reload)
    .refreshing(self.loading)
```

Any scrollable anywhere inside a `Refresh` feeds it — the child does not have to be a
`Scroll` directly, which is what lets a screen keep its own layout around the list. The
walk names the enclosing area on every `Scrollable` it registers, so the shell knows
where to send the refusal without searching by geometry.

### Who decides when it is over

The framework reports that the user asked; the application decides when the answer has
arrived. There is no future to await and no callback to complete: the indicator spins
for exactly as long as the tree is rebuilt with `refreshing(true)`.

That is not a simplification of the usual pattern, it is the Elm shape of it — the flag
in the application's own state is the single source of truth, and it is what makes the
whole state machine testable without a clock or an executor.

Two consequences worth stating:

- **The message fires on release**, not when the indicator finishes settling. The snap
  takes 150 ms, and there is no reason to make a network request wait for an animation.
- **A refresh that needed no work still goes away.** If, by the end of the snap, the
  application has not raised its flag, the indicator plays snap → done rather than
  spinning forever. A cache hit reads as "already up to date", which is honest, instead
  of hanging.

### The thresholds

| | |
|---|---|
| a full drag | 25 % of the scrollable's own extent |
| armed at | two thirds of that |
| overshoot allowed | one and a half times the resting displacement |
| snap back | 150 ms |
| scale away | 200 ms |

The threshold is **proportional**, not a pixel count: a tall list and a short one ask
for the same *gesture*, not the same number of pixels.

An armed pull does not disarm when the finger eases off — only letting go ends it.
Otherwise the indicator would flicker in and out of "release me now" as the finger
wavers around the line.

## The two physics put the overscroll in different places

This is the part that had to be got right, and it is not symmetric:

- **Clamping** refuses the movement and pins the offset at the edge. The refused amount
  is the only trace the gesture leaves, and it is incremental. The physics returns
  nothing at all for a move back towards the content, so an eased-off pull *holds*
  rather than retracting — which is right, because the finger has not let go.
- **Bouncing** lets the offset travel past the edge, so the **depth** it reached is the
  signal, and the change in that depth is signed. The indicator therefore follows the
  rubber band back in as the finger returns — also right, because there the content
  itself is already saying so.

Reading only one of the two would have produced a feature that worked on one platform
and did nothing at all on the other. The bouncing case in particular never produces a
"refused" amount, so a naive port would have been silently dead there.

Leaving the top edge at all ends the pull: the gesture has become an ordinary scroll,
and an indicator still hanging there would be promising something the release is no
longer going to deliver.

Where a `Refresh` is listening, the **top glow stands down**. Both answer "there is
nothing more that way", and giving both would say it twice.

## The indicator

It is the framework's own activity ring — the one `Spinner` already draws — not a
second, subtly different idea of "busy". `paint_activity_ring` was factored out of
`Spinner` so the two cannot drift apart, and it grew a second mode:

- **Filling** while dragging: the ring fills to three quarters, with a partial dot at
  the leading edge so the growth is smooth rather than clicking round one eighth at a
  time. A completed drag is visibly *not* a completed circle.
- **Spinning** while working: the full ring with a bright head. The circle is what
  spinning means, which is why the drag never completes it.

Colour and disc are the theme's primary and surface, overridable per area
(`.color(…)`, `.background(…)`, `.displacement(…)`, `.size(…)`).

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **729 tests, 0
  failures** (709 at milestone 280); 20 new, covering the state machine (arming,
  proportional threshold, no disarm on ease-off, the drag limit, cancel, the spin
  lasting exactly as long as the flag, an area leaving the tree) and the frame
  (the host recorded on the scrollable, the indicator drawn and moving with the pull,
  nothing drawn and no frames asked for when nothing is pulled).
- `cargo build --workspace --all-targets` — OK, no new warning.

**On a physical device** (Huawei STK-L21, Android 10), which is where a gesture is
actually judged:

| | clamping (the platform default there) | bouncing |
|---|---|---|
| mid-drag, finger down | content still, ring part-filled below the edge | content pulled ~260 px away **and** the ring above it |
| on release | ring snaps up, completes, spins | band springs home, ring spins |
| after | indicator gone, "Reloaded 1×" | indicator gone, "Reloaded 1×" |

The demo's log screen is the fixture: 5000 rows, a `Switch` button for the physics, and
a counter so a completed pull leaves a trace.

### One thing that was not a bug

Early device runs showed the counter advancing by **two** per gesture. Instrumenting the
dispatch with a unique per-call id showed one dispatch and one completion for an intact
gesture — the doubling appeared only when the gesture was split across separate
`adb shell input motionevent` invocations, minutes apart, which is a driving artefact
and not something a finger can do. Worth recording because the obvious conclusion from
the screenshots was the wrong one, and the log was what settled it.

## What's left

- **Only the top edge.** Pulling *up* from the bottom to load more is the same
  machinery with the other edge and a different threshold, and is the natural follow-on.
- **No programmatic show.** An application cannot start the indicator itself — for a
  refresh triggered by a button rather than a gesture. That wants a way to address a
  widget's retained state by identity, which is a broader question than this widget.
- **The pull does not resume after release.** A second finger arriving mid-animation
  starts nothing; it waits for the current cycle to end.
