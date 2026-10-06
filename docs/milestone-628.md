# Milestone 628 — The Apple-style theme

## Objective

Milestone 627 gave frus the palette of Apple's platforms. The Apple-style widgets need more
than a palette: a primary colour for their controls, a background for their bars and pages,
handles for a selection, and a type scale of their own. In the reference, that is
`CupertinoThemeData` (`cupertino/theme.dart:175`) and `CupertinoTextThemeData`
(`cupertino/text_theme.dart:136`).

The reference has one more property worth keeping: **the Apple-style theme is part of the
main theme.** An application that sets a primary colour once sees it on a Material button
and an Apple-style switch alike, because under the main theme the Apple-style widgets read
`MaterialBasedCupertinoThemeData` (`material/theme_data.dart:2967`). That takes the primary
colour, the colour on it, the page background and the brightness from the main theme, and
Apple's defaults only for the rest. An explicit `CupertinoTheme` in a subtree replaces it
all, with Apple's defaults under it.

## What was done

- **`CupertinoThemeData`**: everything optional, as asked for. It holds the brightness, the
  primary colour, the contrasting colour, a text theme, and the bar, page and
  selection-handle colours, plus `apply_theme_to_all`. Each colour may be adaptive or plain
  (`From<Color> for CupertinoDynamicColor`), and is resolved where it is used.
- **Two places to put one**, as in the reference:
  - `Theme::with_cupertino_override_theme(data)` (the reference's `cupertinoOverrideTheme`).
    It changes what the Apple-style widgets take from the main theme, and leaves the rest
    following the main theme.
  - `CupertinoTheme::around(data, child)`: a subtree's own Apple-style theme, with Apple's
    defaults for what it does not set. It also colours the subtree's icons with its primary
    colour, as the reference's widget does (`theme.dart:126`).
  - Both are `Theme` fields, `cupertino_override_theme` and `cupertino_theme`. They are
    boxed, so the theme, which is copied down a recursive walk, grows by two pointers. A
    fade switches them halfway.
- **`CupertinoTheme::of(&theme)`**: the theme in force, every value concrete:

  | value | under the main theme | under a subtree's own |
  |---|---|---|
  | brightness | the override's, else the main theme's | its own, else the system's |
  | primary | the main theme's primary | system blue |
  | on primary | the main theme's on-primary | white |
  | page background | the main theme's background | system background |
  | bar background | `F0F9F9F9` / `F01D1D1D` | same |
  | selection handles | the main theme's, else system blue | system blue |

- **The brightness rules the adaptive colours.** `CupertinoDynamicColor::resolve` now reads
  `CupertinoTheme::brightness_of(theme)`, the reference's `maybeBrightnessOf`. So a light
  application whose override asks for a dark Apple-style look gets the dark values. Without
  an override nothing changes: the brightness is the main theme's, as in milestone 627.
- **`CupertinoTextThemeData`** (as asked for) and **`CupertinoTextTheme`** (resolved). The
  defaults are the reference's:

  | style | size | weight | letter spacing | colour |
  |---|---|---|---|---|
  | text | 17 | — | −0.41 | label |
  | action / nav action | 17 | — | −0.41 | primary |
  | action small | 15 | — | −0.23 | primary |
  | tab label | 10 | medium | −0.24 | inactive grey |
  | nav title | 17 | semibold | −0.41 | label |
  | nav large title | 34 | bold | 0.38 | label |
  | picker | 21 | regular | −0.6 | label |
  | date and time picker | 21 | regular | 0.4 | label |

  A style an application sets is taken as it is. A text theme given without a primary
  colour puts its actions in system blue, as the reference's constructor does. The text
  theme the theme makes on its own takes the theme's primary colour.

## Deliberate differences

- **No font family.** The reference names Apple's system faces (`CupertinoSystemText`,
  `CupertinoSystemDisplay`), which exist only on Apple's platforms. The defaults here name
  none, so they are set in the application's own face; naming a face that is not there would
  only produce the fallback warning of milestone 626.
- **A style's colour is plain.** frus's `TextStyle` holds a plain colour, so a style an
  application sets is not re-resolved for dark or high contrast. The defaults are, because
  they are built at resolution time from adaptive colours.

## Tests

- `under_the_main_theme_its_colours_are_the_defaults`: the left column of the table, and the
  main theme's selection handles before Apple's blue.
- `the_override_replaces_what_it_sets`: what it sets is replaced and the rest kept, and its
  brightness turns adaptive colours dark under a light theme.
- `a_subtree_s_own_theme_has_apple_s_defaults`: the right column, with the system's
  brightness unless its own is given.
- `the_default_type_is_the_reference_s`: the type table, a style given taken as is, and the
  system-blue default of a given text theme.
- `a_subtree_takes_its_own_theme`: three swatches built from their subtree's theme paint the
  main primary, Apple's blue and a subtree's pink icon colour; the override crosses a fade
  halfway.
