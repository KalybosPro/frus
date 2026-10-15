# Milestone 656 — The text field's decoration, against the reference

## Objective

The first of the text field's reviews against the reference's `TextField` and its
`InputDecoration` (`material/input_decorator.dart`). frus's field already had the floating
label, the notch, the hint, the helper and error line, the counter, and the prefix and
suffix icons. Four parts of the decoration were missing:

- **`floatingLabelBehavior`**: `auto`, `always` (floated over an empty field, so the hint
  shows) or `never` (inside an empty field, gone once it holds something). Also
  `floatingLabelAlignment`: `start` or `center`.
- **`prefixText` and `suffixText`**: text either side of the value in the hint's style,
  such as a currency sign, a country code or a unit. Its room is kept whether it shows or
  not. It shows while the label is floated, or with no label, once the field is focused or
  holds something (`_AffixText`, `input_decorator.dart:1804`).
- **`icon`**: an icon outside the box, 24 pixels (18 dense) with 16 after it. The box, its
  label and the line under it all start after it.
- **`counterText`**: what the counter says instead of the count. An empty text hides it.

## What was done

- **`TextField::floating_label_behavior`** and **`floating_label_alignment`**, with the
  enums `FloatingLabelBehavior` and `FloatingLabelAlignment`; both are also in
  `TextFieldTheme`.
  - The hint shows in an empty field once the label is floated: at once with `Always`, and
    never with a label that never floats, because that label sits where the hint would.
  - A centred label is centred on the box at its floated size. At rest it stays where the
    value starts.
- **`TextField::prefix_text`** and **`suffix_text`**, in the hint's colour, fading in with
  the label.
- **`TextField::icon`**.
- **`TextField::counter_text`**: the line under the box is kept for a counter text even
  without a limit, and not kept for an empty one.
- **One measure of where the value starts.** The outer icon, the padding, the prefix icon
  and the prefix text now feed every consumer of the content geometry: the paint, the click
  (`cursor_at`), the text frame the handles and the selection bar are placed against, and
  the content width. A click is measured from where the glyphs are drawn, affixes or not.

## Tests

- `frus-widgets`:
  - the label floats as told: auto, always (and its hint), never (inside even focused, no
    hint, gone once filled), and the theme's behaviour;
  - a centred label floats to the middle and rests at the start;
  - the affixes either side of the value, in the hint's colour; the value after the prefix;
    a click at the value's start lands at index 0; hidden under a resting label and with no
    label until focused;
  - the outer icon moves the value and the helper line over by 40 (34 dense), the box
    starts after it, and a click is measured from there;
  - the counter says what it is told, an empty text hides it and gives back its line, and a
    counter text with no limit keeps a line.
- Goldens: `field_decoration` recorded; every other golden unchanged.

Mutation testing: 13 mutants, all killed. They cover:
- each label behaviour, the label hidden once a never-floating field is filled, and the
  hint waiting for the label;
- the affixes' visibility with no label, the prefix's room, and the label resting before
  it;
- the outer icon's gap and its dense size;
- the empty counter text;
- the click measured from the lead;
- the centred label's middle;
- the gutter in the lead.
