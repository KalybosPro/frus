# Jalon 77 — `frus-test`: headless rendering, snapshots and goldens (opening §13)

## Analysis

§13 of the ideas book identifies **testing DX** as an adoption factor. The
infrastructure already existed in pieces: every GPU test copied ~90 lines of
offscreen harness (headless device, texture, readback). Mature toolkits package
this as a test crate with a golden-file matcher — we follow that step.

## Architecture

- **`frus-gpu::render_offscreen(scene, w, h, clear) -> Option<OffscreenFrame>`**
  (a new public `offscreen.rs` module): THE window's pipeline — quads +
  decoration quads + glyphs, targeting **sRGB** (the bytes read back = what a
  screenshot would give), with a readback padded to 256 bytes (arbitrary widths).
  `None` when there is no GPU adapter. The duplicated harness in `text.rs`'s
  tests is replaced by one call (−90 lines).
- **`frus-test`** (a new crate, outside the production pyramid: ← gpu + widgets):
  - [`Snapshot`]: `pixel(x,y)`, `lit_pixels(threshold)`, `diff_count(other,
    tol)`;
  - [`render_scene`] and **[`render_widget`]** — the latter does what the shell
    would do: `build_ui` (taffy layout + themed painting) inside a virtual
    window, with neutral retained state;
  - **goldens**: `assert_golden(path)` compares against a reference PNG. Absent →
    created (to be reviewed and then committed); `FRUS_UPDATE_GOLDENS=1` →
    regenerated; a difference → a panic, writing `<name>.actual.png` alongside.

## Decisions

- Goldens depend on the rasteriser (text AA): generate and compare **in the same
  environment** (here llvmpipe/WSL, deterministic — verified: two successive runs,
  0 diff). The tolerance is configurable (`assert_golden_with(path, channel_tol,
  max_pixels)`).
- The `png` dependency (pure Rust) lives **in the test crate only** — the
  production pyramid is unchanged.
- §13's tier 1 ("a pure `update` makes tests trivial") needs no tooling at all:
  it is the Elm architecture itself; documented at the top of the crate.
- `*.actual.png` gitignored (failure artefacts).

## Tests (266 → 269)

- `renders_rect_and_reads_back_srgb` (gpu): the public offscreen path, at a
  non-aligned width (70 px) → the padding is exercised, exact pixels.
- `scene_matches_golden`: a rounded rect + **underlined** text → a committed
  golden (double visual proof of milestone 75).
- `widget_tree_matches_golden`: a Container/Flex/Text tree (including a
  `strikethrough`) rendered through `build_ui` + the dark theme → a committed
  golden.
- `diff_count_is_exact`: 0 on identical renders, 1 on one corrupted pixel,
  absorbed by the maximum tolerance.

## The rest of the §13 work (in order of value)

1. A runtime inspector (the diagnostic dump as an overlay);
2. state-preserving hot reload (`subsecond`/`hot-lib-reloader`, the Elm state
   being a single serialisable struct);
3. a `cargo new` template (`cargo generate`) to start a frus app.
