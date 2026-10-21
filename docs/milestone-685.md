# Milestone 685 — The pipelines kept between launches

## Objective

Milestone 683 measured the first frame (#211): on the test phone, 1.2–1.4 s from a cold start,
of which 0.9–1.2 s is Vulkan building the renderer's pipelines — again at every launch, since
nothing kept them (#217).

## What was done

- **The renderer builds its pipelines through a pipeline cache** where the device has one
  (`wgpu::Features::PIPELINE_CACHE`: Vulkan) and the shell has said where to keep it. Before
  the pipelines, the cache is filled from the file a previous launch wrote for this adapter and
  driver (`wgpu::util::pipeline_cache_key`); after the warm-up frame it is written back when it
  holds more than was read — aside first, then moved over, so a launch stopped halfway leaves
  no half a cache. wgpu checks the data's header against the device and starts empty when it
  does not match (a new driver).
- **Where**: on Android the application's own storage (`frus-pipelines/`); on Linux
  `$XDG_CACHE_HOME/frus/<program>` (or `~/.cache/…`). Windows draws through DirectX 12, which
  has no pipeline cache in wgpu (#298 makes its compiler faster instead); the web's browser
  keeps compiled shaders itself.
- Every pipeline of the painters takes the cache (`Painters::new(…, cache)`); glyphon's text
  pipelines are built inside glyphon and are not cached yet.
- The renderer says what it did: `pipeline cache: 61 KB read from the previous launch`, or that
  there is none yet.

## Measured

The counter, release, on the Huawei STK-L21 (Mali-G51, Vulkan), started cold three times after
installing:

| start | pipeline cache | pipelines | first frame |
|---|---|---|---|
| 1 (just installed) | written, 61 KB | 708 ms | 1153 ms |
| 2 | read | 49 ms | 333 ms |
| 3 | read | 46 ms | 323 ms |

From the second start the first frame comes about three and a half times sooner. The frame
drawn from cached pipelines was checked on the screen.

## Verified

The phone as above; clippy for `frus-gpu` and `frus-shell` on Windows, Android and Linux (WSL),
and `frus-gpu` for wasm32.
