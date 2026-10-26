# Milestone 696 — The time to the first frame, measured again and again

## Objective

Issue #211. Milestone 683 gave every application one line at launch — how long it took to show
its first frame, and roughly where the time went. Three things were still missing: the line
left whole stages out (the time before the framework's code runs, the surface, the first
build, layout and presentation, each on its own), one launch is not a number — the same
program on the same machine swings by a factor of two — and nothing watched it.

## What was done

- **The line names every stage of a launch**:

  ```text
  first frame at 1376 ms after launch — shell at 37 ms; window at 85 ms; GPU ready at 1165 ms
  (adapter 194 ms, device 115 ms, surface 18 ms, pipelines 618 ms, warm-up 104 ms); fonts
  74 ms (2 loads); built in 0 ms; laid out at 1343 ms (in 66 ms); presented in 33 ms
  ```

  - *shell at* — when the framework's code began to run: what is before it is the system
    loading the program and its libraries.
  - *surface* — new in `frus_gpu::StartupTimes`: the surface's capabilities read and the
    surface configured, which was hidden between the device and the pipelines.
  - *built in*, *laid out at … (in …)*, *presented in* — the first frame's own stages: the
    first tree, its layout and paint into a scene, and the scene drawn and handed to the
    display.

  It stays on for every application, as in 683: a few clock readings, once. The frame timed its
  stages only with the frame statistics on (`FRUS_FRAME_STATS=1`); now it also times them while
  the first frame is still to come, which is one atomic load per frame after that. Before, the
  layout mark took a lock at **every** frame for the life of the program; every mark is now
  an atomic load and a return once the first frame has been reported.
- **`FRUS_EXIT_AFTER_FIRST_FRAME=1`** ends the program right after the line is logged, so a
  launch can be timed by a script without a window to close.
- **`scripts/first-frame.sh`** launches a built program N times (default 7) with that switch,
  reads each launch's line, and prints every launch, then the fewest, the median and the most
  milliseconds of each stage. `-b <ms>` fails when the median time to the first frame is over
  a budget. It runs in any POSIX shell — Linux, macOS, Git Bash on Windows:

  ```sh
  cargo build --release -p frus-hello
  bash scripts/first-frame.sh -n 10 target/release/frus-hello
  ```

- **CI measures it at every push, as advice.** The GPU job's last step builds `frus-hello`
  (the dev profile the job already compiles for its tests; dependencies optimised), launches it
  seven times under Xvfb on lavapipe, writes the table into the job's summary, and compares the
  median with `ci/first-frame-budget.toml` (3000 ms). The step is `continue-on-error`: over
  budget it turns orange and leaves a warning, and the job stays green. Not a gate, because on
  lavapipe every stage of a launch — compiling the pipelines above all — is CPU work on a
  shared runner whose CPU model changes between runs: the measurements below swing by a factor
  of two to three on one machine with nothing changed, and a gate on that would fail changes
  that did nothing. Last in the job, so that installing Xvfb cannot move the pinned mesa
  before the goldens have run.
- CONTRIBUTING's performance section says where the line comes from and how to use the script.

## Measured

All with `scripts/first-frame.sh`, `frus-hello` (the counter), on the maintainer's laptop
(Intel Iris Xe, Windows 11) while other builds were running on it — the absolute figures are
high for that reason, and the spread is the point of taking a median.

**Windows, DirectX 12, release build, DXC beside the program** (as `frus run` and `frus build
desktop` place it since milestone 684), 10 launches:

| stage (ms) | min | median | max |
|---|---|---|---|
| **first frame** | 1094 | **1376** | 2199 |
| shell | 30 | 37 | 43 |
| window | 66 | 85 | 110 |
| GPU ready | 881 | 1165 | 2001 |
| — adapter | 151 | 194 | 217 |
| — device | 103 | 115 | 141 |
| — surface | 16 | 18 | 19 |
| — pipelines | 456 | 618 | 1390 |
| — warm-up | 72 | 104 | 173 |
| fonts | 50 | 74 | 126 |
| first build | 0 | 0 | 0 |
| layout and paint | 40 | 66 | 107 |
| presentation | 21 | 33 | 39 |

**The same program without DXC** (what a bare `cargo build` gives: DirectX 12 falls back to
FXC), 7 launches: median **2998 ms**, pipelines **2347 ms** (1639–2439), the first launch after
the build 11089 ms with 9511 ms in pipelines. DXC is worth 1.6 s at the median here.

**Linux, WSL 2 on the same laptop, lavapipe (Vulkan on the CPU), dev profile** — the CI
configuration, short of Xvfb (WSLg's display here), 7 launches: median **1190 ms** (940–3630);
pipelines 303 ms, warm-up 133 ms, window 360 ms, shell 100 ms. The first launch, with no
pipeline cache yet (milestone 685), took 3630 ms, 2324 ms of it in the warm-up; the next ones
read the cache.

What this says about where to work, in order: on Windows the **pipelines** are still half of
the launch even with DXC (#217: DirectX 12 has no pipeline cache in wgpu); then the **adapter
and device** (300 ms together); on Linux, the **window** (260 ms between the shell's start and
the window: the display connection, the accessibility bridge). The application's own first
build, layout and presentation are under 100 ms together.

Android was not measured this time (no phone attached); milestone 683's method and figures
stand — `am start -W` against the same line in logcat, 1.2–1.4 s cold before the pipeline
cache, 323–333 ms after it (milestone 685).

## Left

- The web has the line (the page's clock counts from the navigation) but no script that loads
  a page N times; milestone 683's figures were taken by hand.
- Android: the script launches a desktop program; a phone is timed with `am start -W` and the
  logcat line, by hand.
- A hard CI gate needs a runner whose timing is stable — a dedicated machine, or a measure
  that counts work instead of time.
- The first build shows `0 ms` for the counter: whole milliseconds are too coarse for it.

## Tests

`startup::tests` (6): the line names each stage in order, in the exact format the script
reads; a stage that did not happen is left out rather than shown as zero; only the first
frame is reported and only the first of each stage counts (a renderer made again after a lost
device, the next frame's layout); after the first frame nothing is recorded; the exit switch
reads set / empty / `0`; `/proc` parsing. The script was run against real launches on Windows
(Git Bash) and Linux (WSL), above. `cargo clippy --workspace --all-targets -D warnings`,
`frus-shell` clippy for wasm32 and Android, `frus-shell` and `frus-gpu` tests, rustdoc with
`-D warnings`. The CI step's YAML was parsed with PyYAML; the step itself first runs on this
change's pull request.

## Mutation testing

Six hand mutants, each killed by a test:

| mutant | killed by |
|---|---|
| a renderer made again replaces the first one's times | `only_the_first_of_each_counts` |
| every frame is reported, not only the first | `after_the_first_frame_nothing_is_kept` |
| one font load is written "1 loads" | `a_stage_not_marked_is_left_out` |
| an empty `FRUS_EXIT_AFTER_FIRST_FRAME` ends the program | `the_exit_switch` |
| the shell mark is recorded after the first frame | `after_the_first_frame_nothing_is_kept` |
| a layout that was not timed is left out of the line | `a_stage_not_marked_is_left_out` |

The script was run against `frus-hello` (dev profile, three launches under WSLg): every
stage parsed, a median of 410 ms, exit 0 under the budget.
