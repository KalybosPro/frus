# Milestone 612 — What the GPU spends on a frame

## Objective

After milestone 610, about a third of the frames of a flick on the test phone still missed
the refresh. The frame statistics (milestone 609) time the CPU: building, layout and paint,
and the renderer's stages, acquiring an image, drawing, presenting. The late frames were
long in **presenting**, the CPU waiting on a GPU still busy with earlier frames, and nothing
said what the GPU itself was spending. This milestone asks the GPU.

## What was done

- **`GpuTimer`** (`frus-gpu/src/gpu_timer.rs`). An empty compute pass submitted before a
  frame's work writes a timestamp; another, after it, writes a second. The two are resolved
  into a buffer and copied into a mappable one, and the difference, times the queue's
  timestamp period, is the time the GPU took from one to the other.
- **Nothing waits.** A ring of three slots: each frame's timestamps go into the next free
  one, its buffer is mapped when the GPU is done, and the CPU reads it the next time it
  looks, at the start of a later frame. A frame that finds every slot still on its way back
  goes untimed rather than held up.
- **Only where the device can.** The renderer asks for `Features::TIMESTAMP_QUERY` when the
  adapter offers it and for nothing otherwise; without it there is no timer, and nothing is
  asked of the device. At start-up the renderer logs which it got: `GPU clock: frames timed
  on the GPU`, or `none, frames timed on the CPU only`.
- **In the frame statistics.** `RenderTimings::gpu` carries the latest frame the GPU has
  finished, and the two-second line adds `, on the GPU g50/g95/gmax` after the drawing
  figures when any frame was timed.

## Alternatives weighed

- **Timestamps inside the frame's passes** (`timestamp_writes` on the render passes) would
  split the time by pass. It needs every pass to take part, and the compositor's passes
  are built in several places; two marks around the whole frame answer the first question,
  how long, without touching them. Per-pass timing can come later if the total says it is
  worth it.
- **Waiting for the result** (`poll(Wait)` after each frame) would give the frame's own
  figure instead of one a few frames behind, and would stall the CPU on the GPU, which is
  the very thing being measured.
- **`TIMESTAMP_QUERY_INSIDE_ENCODERS`** would let the marks go into the frame's own encoder
  rather than encoders of their own. It is offered by fewer devices than the base feature,
  and two extra submissions a frame cost nothing measurable.

## On the test phone

The Mali-G51 with its r18p0 Vulkan driver does not offer timestamps: the log says `GPU
clock: none`, and the line is as before. The timer is there for the desktop, where the
software adapter used by the tests offers it, and for newer phones. What the test phone
did show again, with milestone 610 in place: flicks at 38–41 frames a second, a median
interval of 17–18 ms, a 95th percentile near 50 ms, and on those frames a render of
~40 ms of which drawing is 5: the rest is presenting.

## Tests

- `a_device_without_a_clock_has_no_timer`: a device created without the feature gets no
  timer.
- `frames_timed_by_the_gpu_come_back`: twenty timed frames on a device with a clock, and a
  duration comes back, never negative and under a second. Skipped on a machine with no
  adapter offering timestamps.

Mutation testing: four mutants of `gpu_timer.rs`, all killed — a slot never marked
pending, the sign of the difference reversed, the feature check removed, and the map
callback that never says the buffer is ready.
