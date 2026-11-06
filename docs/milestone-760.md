# Milestone 760 — A selection's highlight sits under its word on a dense screen

## Objective

On the phone (Huawei STK-L21, Android 10, 1080×2340 at 2.625×), a long press on the word
"framework" in the About page's rich paragraph selected the word, but the highlight and its
handles sat to the right of it: the box started on the "r" and ended past the "k", about 10
physical pixels off. The paragraph is a `RichText` inside a `SelectionArea`, styled with the
theme's `body_medium`, which has a letter spacing of 0.25.

## Root cause

The scene is built in logical pixels and scaled to the screen by `Primitive::scaled_xy` just
before it is drawn. For a plain `Primitive::Text` that scales the size **and** the letter
spacing. For a `Primitive::RichText` it scaled each run's size and left its `letter_spacing`
logical.

The shaper takes letter spacing as a fraction of the size (`spacing / size`), so at 2.625× a
0.25 px spacing became 0.25 / 36.75 of an em instead of 0.656 / 36.75: each character was drawn
0.41 physical pixels closer to the previous one than the layout placed it. The highlight, the
handles, hits and the toolbar's anchor all come from the layout in logical pixels (scaled with
the rest of the scene), so they were right; the words were not. After the 25 characters before
"framework" the gap was 10.2 physical pixels, which is what the photo shows. On a 1× screen
the two agree, which is why no test and no golden saw it.

It was neither the selection code, the alignment, nor the bidi change of milestone 747: the
selection rectangles and the glyphs were computed from the same runs in the same box. Only the
copy that went to the renderer differed.

`Text::selectable()` and text fields draw `Primitive::Text`, whose spacing was already scaled,
so they were never affected. Any rich paragraph with a non-zero letter spacing was drawn too
tight on any screen above 1×, selected or not; the selection made it visible.

## Change

`crates/frus-core/src/scene.rs`, `Primitive::scaled_xy`: a rich run's `letter_spacing` is
multiplied by `sx`, as a plain text's is (spacing runs along the line).

## How it was tested

- `frus-widgets` `richtext::tests::the_highlight_sits_under_its_word_on_a_dense_screen`: the
  About paragraph (same spans, `body_medium`), painted with "framework" selected, the scene
  scaled by 2.625 as the shell scales it, and the scaled rich primitive laid out as the
  renderer shapes it. The word's drawn box must match the highlight to half a physical pixel.
  Before the fix: "the highlight starts at 579.83 where the word is drawn from 569.68" (10.2 px,
  the photo's offset). After: passes.
- `frus-core` `scene::tests::scaling_scales_a_rich_runs_letter_spacing`: uniform and per-axis
  scaling of a rich run's spacing.
- `frus-core` and `frus-widgets` unit tests, the goldens (`frus-test`, WSL).
- On the phone: see below.

## On the phone

A release build of `frus-demo` with the fix, the same long press on "framework" on the About
tab: see the device section of the pull request for the result.

## Not done

- Other screen densities than 2.625× were checked only by the unit test (any `sx` is scaled the
  same way).
