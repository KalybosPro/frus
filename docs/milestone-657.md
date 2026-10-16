# Milestone 657 — A text field that grows with its value

## Objective

The second of the text field's reviews against the reference's `TextField`. A multi-line
field there is sized by `minLines` and `maxLines`:

- `maxLines: 1`, the default, is a single-line field;
- `maxLines: n` starts at one line, or at `minLines`, and grows as the value wraps, up to `n`
  lines, then scrolls;
- `maxLines: null` grows without a limit;
- `minLines` holds the box open at that many lines.

A comment box starts small and becomes the size of what was written. frus's multi-line field
was always `rows` lines tall: three by default, whatever it held.

## What was done

- **`TextField::max_lines(n)`**, taking a number or `None`:
  - `1` is a single-line field;
  - a number or `None` makes the field multi-line and lets it grow;
  - given alone, the box starts at one line, as the reference's does.
- **`TextField::min_lines(n)`**: the fewest lines, in either order with `max_lines`. A most
  below the fewest is the fewest.
- **`rows(n)`** still fixes the box at `n` lines, and **`multiline()`** still gives three.
- **The height from the width.** How many lines a value takes depends on how wide the field
  is, so a growing field is laid out through the layout's measurement rather than a fixed
  height. The measurement wraps the value at the content's width and holds the line count
  between the fewest and the most.
  - The paint, the suffix's click zone, the vertical scroll, the caret's page step, the text
    viewport and the text metrics all ask for the box's height at the width they are given,
    so they agree with the layout.
  - The measurement's key hashes the displayed value, the line bounds and the numbers that
    move the wrap or the bands around the box: the lead, the paddings, the suffix's room,
    and the label and helper bands. A new value, or a prefix that narrows the wrap, is
    measured again.
  - The measured height is whole pixels, rounded up with `frus_core::fits`. The first
    version returned the fractional sum of the label band, the box and the helper line
    (78.4). The layout's guard against a box rounded down past its content caught it in the
    golden; the unit tests had not.

## Left

- **`expands`**, a field that fills the height its parent gives it. It needs the box's height
  in hooks that are only given a width, such as the click and the caret's step.

## Tests

- `frus-widgets`:
  - given only a most, the box starts at one line and grows as the value wraps, held at the
    most; with no most, it keeps growing;
  - the fewest lines hold the box open, in either order, and a most below the fewest is the
    fewest;
  - one line at most is a single-line field; `rows` still fixes the box, which is not
    measured;
  - the layout's measurement, the paint and the suffix's click zone agree on the height; a
    narrower field wraps more; the key changes with the value and with a prefix.
- Goldens: `growing_fields` recorded (an empty field, one with three lines of text, and one
  held open at two); every other golden unchanged.

Mutation testing: 10 mutants, all killed. Three were killed only after their tests were
sharpened:
- the fewest raising a most said first;
- no most meaning no cap, with a value long enough to pass eight lines;
- the value alone changing the measurement key.

They cover:
- when the box grows;
- a most given alone starting at one line;
- one line being a single-line field;
- the fewest and the most kept in order, both ways;
- the unbounded most;
- the suffix zone and the paint following the width;
- the key's value and lead.
