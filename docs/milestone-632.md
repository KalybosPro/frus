# Milestone 632 — The buttons, as the reference's

## Objective

The third widget family in the review against the reference (`docs/reference-review.md`).
frus's `Button` drew the reference's five buttons in one widget with a variant, and that
part stays: elevated, filled, filled tonal, outlined and text differ only in their colours
and elevation, and a variant says so. What it did with them differed from the reference
(`material/button_style_button.dart` and the five buttons' defaults):

| | reference | frus before |
|---|---|---|
| content | any widget, plus `.icon` constructors that set an icon beside the label | a text label only |
| style | `ButtonStyle`: every property resolved **per state**, button → theme for that kind of button → defaults | one value per property, and one theme for all kinds |
| size | at least 64 × 40, moved by the visual density; a 48 px touch target around a smaller button (`:473`, `:578`) | 64 × 40 on every platform, no touch target |
| padding | 24 either side (12 for a text button, 8 above and below), shrinking to half at twice the text size and a quarter at three times (`:294`); 16/24 with an icon | 24 (12) at every text size |
| overlays | hovered 8 %, focused 10 %, and a press's ripple 10 %, in the label's colour for filled buttons and the accent for the others (`elevated_button.dart:547`) | the theme's state layer: 8, 10 and **12** %; ripple at 16 % of the label colour |
| outlined | the outline role, the accent while **focused**, faint while disabled (`outlined_button.dart:553`) | the outline role always |
| enabled | only with an `onPressed` or `onLongPress` | enabled unless told otherwise |
| long press | `onLongPress` | none |
| a long label | wraps, as text does | cut with an ellipsis |

## What was done

- **`ButtonStyle`**, with `text_style`, `background_color`, `foreground_color`,
  `overlay_color`, `shadow_color`, `elevation`, `padding`, `minimum_size`, `fixed_size`,
  `maximum_size`, `icon_color`, `icon_size`, `side`, `shape` (each a
  `WidgetStateProperty`), `visual_density`, `tap_target`, `alignment` and
  `icon_alignment`. A button resolves each one as the reference does: its own style, then
  the theme's style for its kind (`ButtonTheme::elevated_style`, `filled_style`, which tonal
  and danger buttons share as the reference's tonal button does, `outlined_style`,
  `text_button_style`), then the reference's defaults for that kind. The icon colour is
  resolved in the reference's order (`:395`).
- **The reference's defaults table** for each kind, Material 3: surfaces and labels enabled
  and disabled, overlays, shadow, elevations, padding with and without an icon, 64 × 40,
  18 px icons, stadium.
- **Content**: `Button::with_child(widget)` takes any widget; `Button::new(label)` is a
  button holding a text label. `.icon(widget)` sets an icon beside either one, 8 px away
  (4 at twice the text size), before it unless `icon_alignment(IconAlignment::End)`. The
  content's text takes the label style in the foreground colour, and its icons the icon
  colour and size, through the theme the button gives its subtree, as the reference does
  through its `Material` and `IconTheme`.
- **Layout**: the padding scaled by the reader's text size and moved by the density
  (across never inwards, down either way, never below nothing, `:501`); the minimum size
  moved by the density; fixed and maximum sizes; the content aligned by `alignment`; the
  touch target as room around the visible button, which paints inside it and takes the
  taps of the whole box.
- **Paint**: the elevation in each state eased by the interaction progress, the focus and
  hover highlights faded over the surface, the outline per state, and the press's overlay
  colour as the ripple.
- **A button with nothing to do is disabled**; `on_long_press` is a thing to do.
- The older builders keep working and set one property of the button's style each:
  `color` and `label_color` (enabled only, so the disabled look stays the reference's),
  `border_color`, `border_width`, `radius`, `shape`, `label_style`, `size`, `padding`,
  `min_width`, `elevation`. `height` is the button's height exactly, the reference's fixed
  height, which the density does not move.
- A labelled button still declares its width (label, padding, icon and gap, within the
  minimum and maximum), so that an app bar can decide which actions fit before it lays them
  out. A button with other content is measured by the layout.

## Found on the way

- **The disabled colours are resolved over the surface**, as everywhere else in frus
  (`disabled.rs`): `on_surface` at 12 % handed to the GPU as an alpha blends in linear light
  and paints like a third. The defaults use `disabled_container` and `disabled_content`.
- **A button replaces the text style around it** rather than merging into it, as the
  reference's `Material` sets its text style outright. Inside an app bar the style around
  is the title's, and a label that kept its spacing was no longer the width the button had
  measured.
- **A layout quirk**: a node whose width and minimum width are both set, and equal, had its
  text measured at no width at all. "Edit" came out one letter a line, and the button 85 px
  tall in a 56 px bar. A button that declares its width no longer also declares a minimum
  width, since the width already holds it. The quirk itself is the layout engine's, and is
  noted for its own milestone.
- **`Tooltip` forwarded its child's padding** as well as its structure, so a tooltip round a
  button padded itself by the button's 24 px and squeezed the button inside. It now forwards
  everything but the padding.
- **The pictures' buttons are given an action.** In the reference a button with nothing to
  do is disabled, and the test buttons that were meant to look enabled had none.

## The pictures

**The reference pictures now pin their platform.** They used the default theme, whose
platform is the machine's, and with the density a desktop's buttons and a phone's differ
by 8 px. Every theme in the picture tests is now Linux's, the platform they were recorded
on.

Twenty pictures were recorded again after a side-by-side look: 15 goldens, four widget
snapshots and one motion snapshot. Every button in them is the compact desktop size,
32 px in a 40 px touch target instead of 40 px. Every text button has the reference's
padding. The disabled buttons keep their look. The carousel's and the kanban's pills keep
their shapes. At a large reader text size (`dialog_actions_stacked`) the buttons' padding
shrinks, as the reference's does.

A note on tooling: the first golden run of this milestone reported every golden unchanged.
It had tested an outdated build of the widgets, through a build directory shared between
working copies. Every picture target is now run with incremental compilation off, after
touching the sources.

## Tests

- `a_button_is_the_reference_s_size`, `a_desktop_s_button_is_compact`,
  `the_padding_follows_the_text_size`, `each_kind_has_the_reference_s_colours`,
  `the_overlays_are_the_reference_s`, `an_outline_follows_the_state`,
  `any_content_takes_the_button_s_colours`,
  `a_style_outranks_the_theme_which_outranks_the_default`, `a_button_is_a_stadium`.

Mutation testing: seven mutants, all killed. They are a hover overlay of 12 %, a button
enabled without an action, the minimum size blind to the density, padding that does not
shrink with the text, an outline that ignores focus, a label not in the foreground colour,
and a tooltip that forwards its child's padding.
