# Milestone 683 — How long the first frame takes

## Objective

Issue #211: nobody knew how long a frus application takes to show its first frame. A toolkit
that draws itself pays at launch — a GPU to set up, pipelines to build, fonts to read, and on
the web a module to download and compile — and nothing measured it, so nothing protected it.

## What was done

- **Every application reports its first frame**, in one log line, which `frus run` shows on
  every platform:

  ```text
  first frame at 1010 ms after launch — window at 34 ms; GPU ready at 948 ms (adapter 109 ms,
  device 36 ms, pipelines 737 ms, warm-up 23 ms); fonts 16 ms (2 loads); laid out at 1000 ms
  ```

  Measured from the process's start where the system says when that was — `/proc/self/stat`
  on Linux and Android, the process's creation time on Windows, the navigation on the web —
  and from the shell's start elsewhere (macOS, iOS). The breakdown comes from where the time
  is spent: the renderer times its own phases (`frus_gpu::StartupTimes`: adapter, device,
  pipelines, warm-up frame), `frus_text::font_loading` the font systems it builds (one for
  measuring text, one for drawing it; each reads the system's fonts), and the shell marks the
  window, the renderer ready, and the first layout.
- **The web bundle has a budget**: `ci/web-size-budget.toml`, checked by a new blocking CI job
  (`Size budget (web)`) the way `ci/size-budget.toml` checks the Android library: `frus-hello`
  built with `web-release`, passed through wasm-bindgen at the lockfile's version, its
  `_bg.wasm` against 8,500,000 bytes. `scripts/check-size.sh` prints brotli's size too where
  the tool is there.

## The numbers

The counter (the template's application), release builds.

**Windows 11, Intel Iris Xe, DirectX 12**, the program started by hand:

| launch | first frame | pipelines | fonts | the rest |
|---|---|---|---|---|
| the very first after a build | 2440 ms | 1983 ms | 794 ms (cold disk cache) | — |
| then, three times | 1083–1489 ms | 784–1148 ms | 17–21 ms | ~250 ms |
| later, three times | 996–1052 ms | 737–751 ms | 15–17 ms | ~250 ms |

**Web, Edge on the same machine (WebGPU), served by `frus run` from this computer**:

| load | first frame | window (module downloaded and compiled) | GPU |
|---|---|---|---|
| first | 984 ms | 275 ms | 611 ms (device 386 ms) |
| reload | 1862 ms | 1726 ms | 110 ms |
| reload | 173 ms | 103 ms | 49 ms |

The module is 7.2 MB as served (`frus run` does not compress); `frus-hello`'s, measured for
the budget, is 6,698,359 bytes raw, 2,892,537 gzip, 1,854,865 brotli.

**Android**: to be measured on a device (`am start -W` against the log line); the phone was not
connected when this was written.

## Where the time goes

- **The pipelines, on the desktop: three quarters of the launch.** Every pipeline is built
  — its shaders compiled by DirectX 12 — at every launch, and nothing is kept between runs:
  issue #217 ("No pipeline cache"). A browser keeps its compiled shaders, and the same step
  takes 6–20 ms there.
- **The fonts, on a cold disk**: the system's fonts are read twice (measuring and drawing):
  794 ms on the first launch after a build, 15–20 ms once the system has them cached.
- **On the web, the module**: downloading and compiling 6.7 MB before the first line of the
  application runs; a reload that finds it compiled starts in under 200 ms.

## Verified

The report's text and `/proc` parsing are unit-tested (`startup::tests`); the numbers above
are this machine's, taken from the log line; the CI job's version extraction run against
`Cargo.lock` (0.2.127). clippy for `frus-shell` on Windows, wasm32 and Android.
