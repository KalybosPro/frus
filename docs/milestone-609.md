# Milestone 609 — Measuring the frames on a phone, and a screen that drew forever

frus applications have to be fast: smooth on a phone, not just on a laptop. This milestone
starts with measuring where the time goes, on the device that matters, and fixes the first and
largest thing the measurement showed.

## Measuring

**On the laptop**, `cargo bench -p frus-bench` against milestone 299's baseline:

| bench | milestone 299 | now |
|---|---|---|
| a 12-row task list, first frame | 382 µs | 823 µs |
| the same, every string a box | 93 µs | 314 µs |
| 60 rows, every string a box | 433 µs | 1.26 ms |
| a chain of 8 / 64 / 256 containers | — | 96 µs / 3.6 ms / 43 ms |

Two things stand out:

- The walk without text is **three times** what it was. Three hundred milestones have each
  added a little to every node.
- A deep tree costs **more than its depth**. A profile puts 70 % of the deep case in the
  flexbox solver.

But those benches start every frame from an empty runtime, which only the first frame of an
application does. A new group, `frame/steady`, builds the tree again every frame on a runtime
kept between frames, as the shell's is:

| | first frame | every frame after |
|---|---|---|
| 12 rows | 823 µs | 84 µs |
| 60 rows | 2.45 ms | 412 µs |
| 64 containers deep | 3.6 ms | 67 µs |

The caches do their job. **A frame in steady state is cheap on the CPU**, and a phone several
times slower still has room in 16.7 ms. The slow first frame, and the deep case, are left for
later.

**On a phone**, the test device's compositor keeps no frame statistics for a native surface, so
the shell now keeps its own. When asked (`FRUS_FRAME_STATS=1`, or on Android
`adb shell setprop debug.frus.frames 1` before the application starts), every two seconds of
frames drawn without a pause make one line in the log, under `frus::frames`:

- the intervals between frames, and how many missed the display;
- what each stage cost: building the tree, laying it out and painting it into a scene, and
  rendering. Rendering is split into the wait for a surface image and the drawing, which are
  the CPU's; what is left is the CPU held back by a GPU still busy with earlier frames;
- how many layers the scene holds;
- **what asked for the next frame**: the application, a widget that draws continuously, an
  image or a task on its way, the pointer, or the runtime. The runtime's families of retained
  motion are named one by one (a value, a colour, a scroll, the overscroll glow, ink…), so a
  screen that never stops drawing says which motion never settles.

Off, it costs one boolean per frame.

## What it showed: drawing forever

Untouched, the demonstration's home screen was drawing **22 frames a second**: 45 ms each,
with nothing changing on it. Every frame named the runtime's per-widget transitions, then,
narrowed down, an `AnimatedSize`, starting its 0.2 s move over on every frame.

A layout asks the same child several questions in one pass: how wide with this much room, with
that much, at its widest. Only the last answer is the size it ends up at. `AnimatedSize` took
**every** answer for a new target. A child that is given the width on offer answered 297, 481
and 312 px in turn, so its target moved inside every frame and the move began again. That cost
the frames, and the battery.

**Now the answer is noted, the last one winning, and taken up once, when the runtime
advances.** A change starts the move on the frame after it, as it did. A box seen for the first
time still takes its child's size at once, whichever answer was the last.

On the phone, after the fix:

| | before | after |
|---|---|---|
| idle | 22 frames a second | **no frames** |
| rendering, median | 33 ms | 8.5 ms |
| the interval while flicking, median | 45 ms | **16.5 ms** |
| a slow drag | — | 49–52 fps |

## What is left, and is next

Flicking, about a third of the frames still miss the display, at 50–60 ms. The wait for an
image is under a millisecond and the drawing under eight, so it is the **GPU**: it is held up,
and the presentation waits for it.

The scene has two or three **layers** while scrolling. The group-opacity showcase on the home
screen is one; the ink of a touch and the overscroll glow add others. Each layer is rendered
into a texture **the size of the surface**, allocated anew each frame. It is then composited by
a quad **covering the whole screen**. The showcase's layer holds 56 × 32 px of content and costs
two full-screen passes of 1080 × 2340 multisampled pixels. That is the next milestone.

## Verification

- `FrameStats`:
  - two seconds of frames make one line, with the late frame counted;
  - a pause ends a run and is not a late frame;
  - the motion families still moving are named, most frames first;
  - off, nothing is recorded.
- `AnimatedSize`:
  - one frame's three answers are not a change, and ask for no frames;
  - a new last answer is followed;
  - a growing child is followed over its duration, and a page follows a growing section.

  The tests that changed a child within the frame it appeared in now advance between the two
  frames, as the shell does.
- The demonstration, idle on the phone: no frame-statistics line at all, where there was one
  every two seconds before.
- The figures above, measured on the phone with the frame statistics.
