# Jalon 76 — `from_seed`: theme generated from a seed colour (HCT)

## Analysis

The `ColorScheme` (milestone 68) was written by hand. Material 3 generates its
own from **a single seed colour** through the **HCT** space (Hue-Chroma-Tone):
CAM16's perceptual hue and chroma + CIELAB's L\* tone. The tone carries the
contrast — two tones 40+ apart guarantee legibility — so a "by tone" scheme has
`X`/`on_X` pairs that are legible **by construction**.

## Architecture

- **`frus-core/hct.rs`** (pure, zero-dependency): a port of
  `material-color-utilities` (Google).
  - Analysis [`Hct::from_color`]: sRGB → XYZ (D65) → CAM16 (standard viewing
    conditions) for hue and chroma, L\* for the tone.
  - Synthesis [`Hct::solve`]: Newton iteration on lightness `J` (5 steps,
    `findResultByJ`); out of gamut, a **bisection on chroma** (precision 0.4 —
    Google's historical solver) instead of an analytic bisection of the gamut
    boundary (~150 lines avoided for a ±2/255 maximum difference).
  - [`TonalPalette`]: one hue/chroma spread across the tone scale.
- **`frus-widgets`**: `ColorScheme::from_seed(seed, dark)` — 5 palettes (primary
  = the seed's chroma with a floor of 48; secondary 16; neutral 4; neutral
  variant 8; error hue 25 chroma 84), each role being an M3 tone.
  `Theme::from_seed` derives focus and selection from the primary.

## Decisions

- **Ground truth**: the constants and the behaviour are pinned against the
  `materialyoucolor` Python port (#4285F4 → H 265.979, C 62.269, T 56.550; greys
  keep a residual chroma of ≈ 1.9 under partial adaptation — that is not a bug).
  A constant mis-copied from the solver (`m[2][2]`) was caught by that
  cross-check — hence the exact-value tests.
- An accepted departure from M3: `surface` detached from `background` (tones 12/6
  dark, 100/98 light) — our cards place a surface on the background, whereas the
  2023 spec conflates them.
- The **tertiary** palette (hue +60°, chroma 24) will wait for a role to consume
  it (there is no tertiary field in the scheme → no dead code).

## Tests (256 → 265)

- `google_blue_analyzes_to_known_hct`, `solve_matches_reference_implementation`
  (exact values from the Python port, ± 1/255 in gamut, ± 3 out of gamut), round
  trips, the palette's monotonicity in luminance, degenerate inputs.
- `from_seed_generates_contrasting_pairs`: **every** `X`/`on_X` pair holds AA
  (≥ 4.5:1) for 3 seeds × 2 modes — including a grey seed (near-zero chroma).
- `from_seed_light_and_dark_share_the_hue`: both modes spread the same hue, with
  dark and light backgrounds respectively.

## Demo

A "Seed: …" action in the AppBar's menu: it cycles through the hand-written
scheme → Blue (#4285F4) → Purple (#9C27B0) → Orange (#E8710A), with the same fade
as the light/dark toggle (the generated theme interpolates role by role like the
others).
