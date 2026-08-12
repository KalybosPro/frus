# Milestone 289 — Half a pixel, and the line that was never reserved

A defect found on a device during milestone 286, deferred with a diagnosis that turned
out to be wrong, and fixed here: **a wrapping text laid out at its natural width
reported the height of one line and painted two**, so whatever the layout put below it
sat on the second.

## The wrong diagnosis, and why it survived

The milestone 286 note said the height was settled before the width was, and pointed at
the order of the layout engine's measure passes. Two call-site workarounds had been
tried on the device and neither helped, which seemed to confirm it.

It was wrong, and the way it was shown to be wrong is worth recording: **two attempts
to reproduce it in a test both came out green.** A synthetic measured leaf centred on a
column's cross axis behaved perfectly. So did a widget-level copy of the screen where
the bug appeared. A defect that will not reproduce under the described cause is
evidence against the cause, not evidence that the framework is unreproducible.

What did reproduce it was the screen itself — `Application::view` for the demo's task
route, driven past its transition and read primitive by primitive. That printed the
answer in one line:

```
text "Write code" at Point { x: 139.0, y: 512.0 } max_width=Some(146.0)
text "Still to do" at Point { x: 176.0, y: 558.0 }
```

`max_width = 146` is the box the layout gave the paragraph. 512 → 558 is 46 px, which
is one line of 24 px text plus the column's 18 px gap. And "Write code" **has no
business wrapping at all** at its own natural width.

## The actual cause

`measure_wrapped` returned the shaped width as it came from the shaper: `146.4`. The
layout engine rounds the boxes it hands out to whole pixels, so the paragraph got a box
of **146**. At paint time the text is shaped *again*, at 146 — and 146 < 146.4, so it
wrapped onto a second line that the layout, holding a one-line height from the 146.4
measurement, had never reserved.

Half a pixel. It only shows on a text **sized to fit** — centred on a column's cross
axis, say — because that is the case where the box comes from the measurement itself
rather than from a parent that is wider than either number.

## The fix

The measurement rounds **up**:

```rust
let width = match max_width {
    Some(max) => width.ceil().min(max),
    None => width.ceil(),
};
```

Unconstrained, the ceiling is the point: the box can then only be rounded to something
the text still fits in. Constrained, the ceiling is clamped back to the constraint —
the text did fit that width, and a box a fraction wider than allowed would be a
different bug.

The same correction goes into `measure_runs_wrapped` (rich text), which had the same
latent defect for the same reason. `alert.rs` had already learnt this lesson locally
and ceils its own measurement; now it is the rule rather than one widget's habit.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **803 tests, 0
  failures**, and no collateral damage: rounding every text measurement up by a
  fraction changed no other assertion in the suite.
- Three regression tests, one per level, because the bug lives between them:
  - `frus-text`: the natural width is a whole number, and the text still fits on one
    line when given exactly that width — across four strings and four sizes.
  - `frus-widgets`: a paragraph sized to fit is not squeezed into wrapping, and what
    follows clears the lines it really occupies.
  - `frus-demo`: the task screen where the device found it, with a title long enough to
    genuinely wrap.
- **On a physical device** (Huawei, Android 10): the same screen, before and after.

## What this closes, and what it does not

The demo's task screen goes back to `.wrap()`; the workaround from milestone 286 —
"this screen stays on one line rather than showing the bug off" — is gone, and so is
the roadmap entry.

Not closed: the layout engine still rounds boxes to whole pixels, and the reference
does not. Rounding is defensible — it keeps adjacent boxes from showing hairline seams
— but it means any measurement that is not itself whole can be shaved. Text was the one
that mattered, and text is now whole. Anything else that measures itself should ceil,
and the two functions here are the example to copy.
