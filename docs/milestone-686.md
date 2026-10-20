# Milestone 686 — The GPU lost, and found again

## Objective

Part of #217: a GPU device can be lost while the application runs — a driver reset or update, a
GPU removed or restarted, a laptop switching graphics. frus did not notice: the renderer went on
asking a dead device for frames, and the window stayed as it was.

## What was done

- **The renderer knows when its device is lost**: wgpu's device-lost callback sets a flag, and
  `Renderer::render` answers the new `RenderOutcome::DeviceLost`. A lost device is reported when
  the device is polled, which the renderer now does — without waiting — at the start of each
  frame.
- **The shell makes a new renderer on the same window** at the next frame's start, of the same
  kind as the old one (the see-through renderer of milestone 643 on Windows), synchronously on
  the desktop and on Android, asynchronously on the web as at the start. The interface is
  rebuilt and drawn again; the application's state is untouched, since it never lived on the
  GPU.
- **A backend can be forced**: `WGPU_BACKEND` (`vulkan`, `gl`, `dx12`, `metal`, `webgpu`,
  comma-separated) decides over frus's own choice — the primary backends, the emulator's
  OpenGL ES — so a device whose driver misbehaves under frus's choice can be given another one
  without rebuilding the application. frus had been setting the backends itself, and the
  variable did nothing.
- **A test hook**: `frus run`'s dev channel takes `lose-gpu`, which destroys the device as a
  driver reset would (`Renderer::lose_device_for_testing`, hidden from the documentation).

## Verified

The counter on Windows, its auto-increment running, driven through its `FRUS_DEV` channel by a
script standing in for `frus run`: `lose-gpu` → `the GPU device was lost (Destroyed)` → `the
GPU device was lost: drawing again with a new one` → a new adapter; the window, captured before
and after, was drawn again correctly with the count still going. `WGPU_BACKEND=vulkan` and
`=dx12` each gave that backend on Windows, said in the log. clippy for `frus-gpu` and
`frus-shell` on Windows, wasm32 and Android.
