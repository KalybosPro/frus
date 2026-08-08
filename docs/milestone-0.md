# Milestone 0 — A window + a coloured quad

The first milestone of **frus**. It establishes the fundamental loop of any UI
framework: `system event → update → GPU frame → present`.

## What ships

- Cargo workspace (`crates/*`).
- `frus-gpu`: GPU context + minimal renderer (wgpu), **with no dependency on
  windowing**. Draws a coloured quad through a WGSL pipeline.
- `frus-shell`: platform layer (winit 0.30, `ApplicationHandler`). Creates the
  window and drives the renderer.
- `frus-demo`: demonstration binary.

## Architecture

```
        winit (event loop)                 wgpu (GPU)
      ┌───────────────────┐          ┌──────────────────────┐
OS ─► │  frus-shell        │ ─frame─► │  frus-gpu (Renderer) │ ─► screen
event │  ApplicationHandler│ ─resize► │  pipeline + quad     │
      └───────────────────┘          └──────────────────────┘
                    └──── frus-demo (bin: cargo run) ────┘
```

The key seam: `frus-gpu::Renderer::new` takes a `wgpu::SurfaceTarget`, not a
winit type. That makes the platform replaceable (Web/mobile later) without
touching the renderer.

## Dev environment: WSL2

**Important**: on the Windows dev machine, **Smart App Control** is enabled and
blocks execution of the *build scripts* Cargo compiles (system error 4551). SAC
can only be turned off once, irreversibly; so we chose to develop inside
**WSL2 (Ubuntu 24.04)**, where Linux binaries are not subject to that policy.
Display goes through **WSLg**.

Setup (already done):

```sh
wsl --install -d Ubuntu-24.04
# inside Ubuntu:
sudo apt-get install -y build-essential curl pkg-config \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 wayland-protocols \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libvulkan1 mesa-vulkan-drivers vulkan-tools libgl1-mesa-dri libegl1
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

## Running

From WSL2, at the project root:

```sh
bash scripts/wsl-run.sh
```

The script forces winit's **X11** backend (under WSLg, Wayland as root is
unstable; X11 via `DISPLAY=:0` is reliable) and puts the build directory on the
Linux filesystem.

Expected result: a window titled "frus — Milestone 0" with a gradient square
(red/green/blue/yellow) on a midnight-blue background.

> GPU note: under WSLg without hardware passthrough, wgpu falls back to the
> software Vulkan backend `llvmpipe` (Mesa). The rendering is correct, simply
> not accelerated. On a native Linux machine a real GPU will be used.

## Tests

```sh
cargo test
```

`frus-gpu` includes a test that enumerates the GPU adapters without crashing
(runnable in CI without a display).

## Version notes

`wgpu` moves fast: the versions are pinned in `Cargo.toml` (`wgpu = "22"`,
`winit = "0.30"`). The first `cargo build` (once Rust is installed) will
validate the API and adjust any minor details.
