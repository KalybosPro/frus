title: Pin the rasteriser so the goldens can become a required check
labels: help wanted, testing, ci

The golden-image job is advisory, and an advisory check went red for five milestones
without anybody noticing (milestone 294). That is the actual cost of leaving it
`continue-on-error`.

The reason recorded in the roadmap for years — "lavapipe rasterises differently from
hardware" — **is not the real one**. The goldens are blessed under lavapipe as well
(llvmpipe under WSL is the only adapter there), so both sides run the same rasteriser.
What differs is its **version**: mesa 25.2 locally against whatever `apt` gives
ubuntu-latest.

### Two ways, and one of them is better

1. **Pin the rasteriser.** A container image, or a mesa PPA at a fixed version, so both
   sides rasterise identically. Then drop `continue-on-error` outright.
2. Measure the version-to-version drift and absorb it through `assert_golden_with`.

The first is the honest one. A tolerance guessed without measuring is a number nobody
can defend, and it will be raised again the first time it fails.

### Where

`.github/workflows/ci.yml`, the `gpu` job. `crates/frus-test/src/lib.rs` for the
comparison itself.
