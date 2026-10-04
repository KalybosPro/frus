# Milestone 624 — The reference's type scale, in full

## Objective

frus's `TextTheme::M3` had the reference's sizes and weights and nothing else. The
reference's Material 3 scale (`typography.dart:2097`, `englishLike … 2021`) also gives each of
its fifteen steps a **letter spacing** and a **line height**: body text sits 0.25–0.5 px
apart on lines 1.33–1.5 of its size, labels 0.1–0.5 px apart, display type −0.25 px. frus set
every step at no spacing on its single default line height of 1.2, so its text was tighter,
and its lines shorter, than the reference's everywhere. Milestone 623 made the spacing
expressible; this milestone writes the scale's numbers in.

## What was done

`TextTheme::M3`, step by step:

| step | size | weight | letter spacing | line height |
|---|---|---|---|---|
| display large / medium / small | 57 / 45 / 36 | regular | −0.25 / 0 / 0 | 1.12 / 1.16 / 1.22 |
| headline large / medium / small | 32 / 28 / 24 | regular | 0 | 1.25 / 1.29 / 1.33 |
| title large | 22 | regular | 0 | 1.27 |
| title medium / small | 16 / 14 | medium | 0.15 / 0.1 | 1.50 / 1.43 |
| body large / medium / small | 16 / 14 / 12 | regular | 0.5 / 0.25 / 0.4 | 1.50 / 1.43 / 1.33 |
| label large / medium / small | 14 / 12 / 11 | medium | 0.1 / 0.5 / 0.5 | 1.43 / 1.33 / 1.45 |

Every widget that takes its type from the scale — which is most of them — now sets it as the
reference does: a little wider, on taller lines.

## What it changes on screen

Text is visibly more open, and a line box is taller; a row that is as tall as its text grows
with it. 81 pictures were recorded again — 57 goldens, 22 widget snapshots, 2 motion
snapshots — after looking at them side by side with the old ones: the same layouts, the words
wider and the lines taller, nothing out of place. One effect worth naming: where an icon and a
text start at the same top (the alert, for one), the text now begins a little lower, because
its taller line adds room above the letters as well as below. The reference's lines share
their leading the same way.

Every unit test passed unchanged: the scale's values reach the widgets, and none of the
widgets' own arithmetic depended on the old ones.

## Tests

- `the_type_scale_is_the_reference_s`: the table above, step by step — size, weight, letter
  spacing and line height of each of the fifteen.
- The 81 recorded pictures.
