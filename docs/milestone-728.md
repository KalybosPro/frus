# Milestone 728 — The app bar beside a cutout, the status bar's clock, the README

## Objective

Two things the simulated devices showed (milestones 725 and 727), and the README, which did not
yet say that frus can show an application on a phone's screen from any computer.

## What was done

- **An app bar beside a cutout folds what it cannot fit.** A primary `AppBar` keeps its row
  clear of the side intrusions (its `SafeArea`), as a phone on its side needs. But it decided how
  many actions fit against the whole width, so on an iPhone on its side the demo's header ran
  77 px off its end, ending in an overflow stripe. The bar now reads the side intrusions with the
  width (`side_insets`, from `MediaQuery::of()` in `AppBar::new`) and counts them out of the room
  for actions. The demo's header folds its last actions into "⋯".
- **The simulated status bar's clock is each system's.** iOS's is 17 points, semibold;
  Android's 14, medium. It was 15, semibold, everywhere. Milestone 727 noted it as "a serif face"
  on the counter example. A probe in the renderer showed the face was the bundled DejaVu Sans
  Bold in every case. At about 14.8 px, the size a zoomed Android phone gave it, its stems were
  rasterised unevenly and looked like another typeface; at 15.7 px they did not. That was a wrong
  diagnosis, corrected here. The clock now has each system's size and weight, which also draws
  cleanly.
- **The README** (English and French) has a section on phones and tablets from any computer:
  `frus run -d iphone-15`, `--web`, the keys, the built-in devices, a project's own
  `[[package.metadata.frus.devices]]`, and what a preview is not. `frus devices` and the keys of
  `frus run` say so in their tables too.

## Tests

- `appbar::a_bar_beside_a_cutout_folds_against_what_is_left`: the narrowest surface where three
  actions fit with nothing at the sides folds one once 59 is taken a side, and fits all three
  given that room back. With the side insets left out of the budget again, it fails.
- The simulated device's tests (12) and the app bar's (42) pass.
