# Milestone 636 — A desktop's menu bar, and every part of it the caller's

## Objective

The menu bar in the review against the reference (`docs/reference-review.md`). On a desktop,
frus's `MenuBar` looked like a phone's:

| | a desktop application's menu bar | frus before |
|---|---|---|
| bar | about 30 px, 13 px words, 8 px either side | 48 px, `label_large`, 12 px either side |
| open word | a clear rounded highlight, so the eye finds what the menu belongs to | a faint tint |
| menus | 26 px rows in 13 px type, a 1 px outline, a line between groups | 48 px rows, no outline |
| row of an open submenu | stays lit while the pointer is in the submenu | dark |
| styling | — | none of it could be changed |

The reference's own defaults are its design system's (`material/menu_anchor.dart`): rows of
at least 64×48 (`:4140`), moved by the theme's visual density (`:4058`), 4 px at the bar's
ends (`:89`, `:4049`), 12 px either side of a word (`:63`), and a panel of radius 4 at
elevation 3 (`:4021`). Its `MenuBar` takes a `MenuStyle` and every button a `ButtonStyle`,
so all of it can be changed.

## Alternatives weighed

1. **The reference's numbers.** With compact density on a desktop they give a 40 px bar. That
   is still taller than any native desktop bar, and the request was a bar that looks like a
   desktop application's.
2. **Native proportions on a desktop, with every number the caller's.** This is what was
   done. On a phone the bar keeps a finger's 48 px. On Linux, macOS and Windows it takes a
   desktop's proportions. Each value can be set on the bar, then in the theme, and only then
   falls back to the platform default.

## What was done

- **`MenuBarTheme`** (`theme.widgets.menu_bar`) sets `height`, `background`, `padding`,
  `item_padding`, `text_style`, `foreground`, `highlight`, `item_radius`, `item_inset`, and
  `menus`, a `MenuTheme` for the menus the bar opens.
- **Builders on `MenuBar`** set the same things: `height`, `background`, `padding`,
  `item_padding`, `text_style`, `foreground_color`, `highlight_color`, `item_radius`,
  `item_inset`, `menu_style`. They can be called before or after the menus.
- **The order a value is chosen in**: the bar's builder, then `MenuBarTheme`. If neither sets
  it, a desktop theme gets a 30 px bar, 13 px words with 8 px either side, 4 px at the ends,
  and a 4 px rounded highlight inset 3 px. A phone's theme keeps 48 px, `label_large` and
  12 px.
- **Its menus** take the same order: `menu_style`, `MenuBarTheme::menus`, then
  `MenuTheme::desktop` on a desktop, then the application's `MenuTheme`.
  `MenuTheme::desktop` gives 13 px type, 26 px rows with 12 px either side, 4 px above and
  below the rows, radius 6, elevation 2, a 1 px `outline_variant` outline and 9 px rules.
- **`MenuTheme` gains `border` and `divider_height`**, and `MenuTheme::or` fills one theme's
  gaps from another. Popup menus honour both.
- **The highlight**: the open word is drawn 12 % towards `on_surface`, and a word under the
  pointer fades in to 8 %. `highlight_color` replaces both. A row whose submenu is open stays
  lit.
- A menu's rule is its own widget, so its height can come from the theme while it still
  stretches across the menu.

## On screen

Two new pictures, `menu_bar_closed` and `menu_bar_open`, show a code editor's bar (File,
Edit, Selection, View, Go, Run, Help) with File open: shortcuts, rules, a submenu open beside
its lit row, a disabled row and a ticked one. They were recorded after a side-by-side look
with a desktop editor's menu bar.

## Tests

- `a_desktop_bar_is_a_desktops_and_a_phones_a_phones`: the bar's height, the words' type and
  their room, on Windows and on Android.
- `the_open_word_and_the_open_row_keep_their_highlight`: the open word's highlight (inset,
  radius, colour), none under a word at rest, and the open submenu's row lit.
- `every_part_of_the_bar_can_be_said`: every builder, said after the menus, including the
  menus' rows and outline.
- `the_theme_answers_what_the_caller_left_unsaid`: `MenuBarTheme` fills in, and the builder
  outranks it.
- `a_desktop_bars_menus_are_a_desktops`: 26 px rows in 13 px type, the outline, a rule
  drawn across the menu, and a phone's rows still a finger tall.
- `a_desktop_menu_bar` (pictures).

Mutation testing: eight mutants. Seven were killed at once. The survivor dropped the rebuild
in `text_style`: the test said other words after it, and they rebuilt the bar anyway. The
test now also says `text_style` last.
