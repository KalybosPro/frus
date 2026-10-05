# Milestone 627 — The colours of Apple's platforms

## Objective

frus follows Apple's platforms in behaviour where the reference does (the back swipe, the page
slide, the scroll physics), but it has none of their look: every Apple-style widget the
reference has (its switch, its slider, its segmented control, its action sheet, the iOS
selection bar) is built from a palette frus did not have. That palette is the first brick.

It is not a list of colours. Most of its entries **adapt**: `systemBackground` is white in a
light interface, black in a dark one, a dark grey in a sheet raised over a dark page, and a
slightly lighter grey again when the reader has asked for more contrast. The reference
models this as one value with up to eight variants, resolved where it is used
(`cupertino/colors.dart:748`, `CupertinoDynamicColor`), from three facts: the brightness, the
high-contrast setting, and the **interface level** (`cupertino/interface_level.dart`: base or
elevated).

## What was done

- **`CupertinoDynamicColor`**: the eight values, the reference's three constructors (`new`
  with all eight, `with_brightness_and_contrast`, `with_brightness`), an optional label, and
  two ways to pick a value:
  - `resolve_with(brightness, level, high_contrast)`: the reference's eight-way table
    (`colors.dart:1024`), for conditions given outright;
  - `resolve(&theme)`: where it is used. The brightness is the **theme's**: under its main
    theme, the reference's adaptive colours take the brightness from that theme
    (`theme_data.dart:2996`), not from the system. The level is the theme's too (below). High
    contrast is the reader's setting, read from the ambient `MediaQuery`.
  - Unresolved, as a plain `Color`, it is its light, normal-contrast, base value, as the
    reference's is.
- **`CupertinoColors`**: the reference's table, all 47 entries, value for value. It was
  generated from the reference's source rather than typed. Names follow Rust
  (`SYSTEM_BLUE`, `SECONDARY_LABEL`, `SYSTEM_GREY_6`), and the aliases stay aliases
  (`ACTIVE_BLUE` is `SYSTEM_BLUE`).
- **`CupertinoUserInterfaceLevelData`** (`Base`, `Elevated`) and **`Theme::user_interface_level`**.
  The reference carries the level in an inherited widget; frus carries a subtree's ambient
  values in its theme. So `Elevated.around(child)` raises a subtree, the way
  `DefaultTextStyle::around` restyles one. A theme starts at `Base`, and a fade switches the
  level halfway, as it does the platform.

## What is not done

- **An Apple-style theme.** The reference also has `CupertinoThemeData`: its own primary
  colour, bar and scaffold backgrounds, and type scale, which can override the brightness.
  Until frus has one, adaptive colours follow the main theme's brightness. That is what the
  reference does when no override is given.
- **Raising sheets and dialogs.** The reference marks its sheets, dialogs and popups
  elevated. Those are Apple-style widgets frus does not have yet; when they come, they will
  wrap their content in `Elevated.around`.

## Tests

- `each_condition_picks_its_value`: the eight cells of the table.
- `the_shorter_constructors_fill_the_other_axes`; the label takes no part in equality.
- `the_palette_is_the_reference_s`: the values easiest to get wrong. These are a system
  colour's four, a label's translucency, the backgrounds that lighten when raised in the dark,
  a separator's raised value, and the aliases.
- `resolve_reads_the_theme_and_the_surface`: the brightness from the theme, the level from
  the theme, high contrast from the ambient surface.
- `a_raised_subtree_resolves_raised`: two swatches built from the subtree's theme, one inside
  `Elevated.around`, paint the base and the raised background; the level crosses a fade
  halfway.
- Two documentation examples.
