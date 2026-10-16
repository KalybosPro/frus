# Milestone 659 — The text field's caret, mask and focus

## Objective

The third of the text field's reviews against the reference's `TextField`
(`material/text_field.dart`). frus's caret was a 2-pixel bar in the theme's `on_surface`,
with nothing to change about it. The reference's is different:

- **in the scheme's `primary`**, the colour the focused border and label already take, and
  in the error's colour while an error shows (`text_field.dart:1599`, `:1637`);
- **shaped by the caller**: `cursorColor`, `cursorErrorColor`, `cursorWidth` (2), `cursorHeight`
  (the line's), `cursorRadius` (2 on Apple's platforms, `:1604`, square elsewhere) and
  `showCursor`.

Two more parts of the field were missing: **`obscuringCharacter`**, which a masked field
shows each character as, and **`autofocus`**, which the framework's focus already supports
(milestone 605) but the field did not offer.

## What was done

- **The caret's colour**:
  - its own colour: the field's `cursor_color`, then the field theme's, then the text
    selection theme's (`TextSelectionTheme::cursor_color`, where the reference keeps it too),
    then `primary`;
  - while an error shows: `cursor_error_color`, then the theme's, then the field's error
    colour.

  Both colours are also in `TextFieldStyle` and `TextFieldTheme`.
- **Its shape**:
  - `cursor_width` and `cursor_height`; the height is centred on the line;
  - `cursor_radius`, defaulting to 2 when the theme's platform is Apple's and square
    elsewhere;
  - `show_cursor(false)` hides it, and the field still takes keys and selections.
- **`obscuring_character`** for a masked field. Only a masked field uses it.
- **`autofocus`**, through the widget's own hook; a disabled field never asks for the focus.

## Left

- **`onTapOutside` and `onEditingComplete`**, which need to hear the field lose the focus: a
  milestone of their own (660).

## Tests

- `frus-widgets`:
  - the caret is `primary`, two pixels wide, and the error's colour while one shows;
  - the field's colours, the text selection theme's, and the field theme's over it;
  - the width and height told, the height centred on the line;
  - square on Android, rounded on iOS, a radius told kept;
  - hidden when told;
  - a masked value shows the character told, an unmasked one its text;
  - autofocus asked for when told, never when disabled.
- Goldens: `field_cursor` recorded (a focused field's caret in `primary`); every other golden
  unchanged, none having shown a caret.

Mutation testing: 10 mutants, all killed. They cover:
- the caret's colour from the text selection theme and the field theme, and `primary` as
  the default;
- the error's colour;
- the height centred;
- the Apple radius;
- hiding the caret;
- the masking character;
- a disabled field's autofocus;
- the width told.
