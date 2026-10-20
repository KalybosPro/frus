# Milestone 681 — The Android emulator

## Objective

An application built by `frus run` on an Android emulator (an x86_64 system image, the host's
GPU) opened and closed at once. What it takes for frus to run there.

## What was found, in order

1. **The package held an arm64 library only** (`build_targets = ["aarch64-linux-android"]`, as
   `frus create` writes it). On an x86_64 emulator it ran through Android's ARM translation,
   which some system images lack.
2. **The emulator's Vulkan driver crashed on the debug names of GPU objects**
   (`vkSetDebugUtilsObjectNameEXT`, in `vulkan.ranchu.so`), which a debug build of wgpu asks
   for as the device is made.
3. **The same driver (gfxstream) then crashes in its own code at the first submission**
   (`ResourceTracker::on_vkQueueSubmit`, a null pointer), with the library built for x86_64.
4. **Through OpenGL ES instead, the device asked for limits a GPU without compute shaders cannot
   give** (`max_compute_workgroups_per_dimension`).
5. **Then a shader does not compile**: glyphon's text shader marks its position
   `@invariant`, and naga writes `invariant gl_FragCoord` into the fragment shader for OpenGL ES
   (it leaves it out for WebGL only), which the emulator's GLES translator refuses. The fix is in
   naga: gfx-rs/wgpu#10229, open, approved in substance by a maintainer; our case is reported
   there.

## What was done

- **`frus run` builds the package for the device's processor** when the manifest's
  `build_targets` lack it (`getprop ro.product.cpu.abi`; the Rust target is installed first).
  `frus build apk` still builds what the manifest lists.
- **No debug names for GPU objects on Android** (`WGPU_DEBUG=1` asks for them again).
- **The emulator's Vulkan is not used**: when the only Vulkan adapter is gfxstream, the renderer
  goes through OpenGL ES (`WGPU_BACKEND` decides instead when set).
- **Limits for a GPU without compute shaders**: the WebGL 2 ones, at the adapter's resolution.
- clippy's `nonminimal_bool` in the compositor, which the current toolchain denies.

## Where it stands

On the emulator with the host GPU the application now stops at the shader (5), until a wgpu
release has naga's fix. With the emulator's software GPU (`-gpu swiftshader_indirect`) the same
build starts and draws — too slowly to be of use on the machine tried. A phone is unchanged by
all of this: the fallback is taken for gfxstream only, and a debug build there gave the names to
no tool anyone used.

## Verified

On an API 35 x86_64 emulator (Windows 11 host, Intel Iris Xe): the package built for
`x86_64-linux-android` (the target installed by `frus run`); each failure above in turn, read
from the device's log; with SwiftShader, the counter drawn. `cargo test -p frus-cli`: 98 tests;
clippy for `frus-gpu` on Windows and `x86_64-linux-android`.
