# Milestone 611 — The app bar, as the reference lays it out

The app bar did not look like the reference's. Side by side with its Material 3 defaults
(`app_bar.dart`, `_AppBarDefaultsM3`, and the toolbar it builds):

| | the reference | here |
|---|---|---|
| an action with words | a text button: no outline, the bar is its surface | an **outlined pill** |
| the overflow menu | an icon button with three upright dots (`more_vert`) | a `⋯` in an outlined pill |
| between actions | nothing: icon buttons side by side, their 48 px boxes the room | 8 px |
| the margins | none: the leading's 56 px slot starts at the edge, the actions end there | 8 px each side |
| the leading | in a slot of exactly 56 px, so an icon button's glyph sits 16 px in | its own width, at the edge |
| the title | 16 px after the leading's slot (72 from the edge), or 16 from the edge with none | 16 after the leading, 8 from the edge with none |
| the actions' glyphs | `on_surface_variant` | `on_surface`, as the leading's |

Everything in the table is now the reference's. The heights (64), the title's type
(`titleLarge`), the surface and the foreground were already right.

## What changed

- **A labelled action is a text button.** `AppBar::action` makes one, and the bar measures that
  button to decide what folds.
- **The overflow is an icon button** with `Icons::MORE_VERT`, announced as "More". It is
  measured as it is drawn, as the actions are.
- **No margin and no gap**, by default. `AppBar::gap` and `title_spacing` still change them.
- **The leading gets its slot**: a box of the leading width (56 by default, or wider for a wider
  leading), with the leading centred in it. The slot used to be counted in the fold's budget but
  never given, so a glyph touched the edge and the title began 20 px early.
- **Without a leading, the title spacing opens the title**, 16 px from the start edge. A centred
  title is not held off the start.
- **The actions' glyphs are `on_surface_variant`** unless an icon theme says otherwise. The
  reference's default `actionsIconTheme` is a step quieter than the leading's.
- **`actions_padding` insets the actions across, not around.** It was applied on all four sides,
  and a 48 px icon button with 12 px above and below ran 8 px past a 64 px bar. The outlined
  pills, 40 px high, had hidden it. The reference uses it to inset the glyphs from the edge,
  and so does its documentation here.

## Not yet

- **Scrolled under**: the reference raises the bar to elevation 3 and gives it the
  `surface_container` colour while content scrolls beneath it. That needs the scaffold to know
  how far its body has scrolled. It is the next step for the bar.
- The demonstration's header still lists many labelled actions, as text buttons now. An
  application would usually give a bar a few icon actions.

## Verification

- The golden of a bar with a menu leading, a title and two labelled actions, re-recorded:
  - the glyph is centred in its 56 px slot;
  - the title starts at 72;
  - the actions are text, with no outline;
  - nothing runs to the edge but the actions' own boxes.

  No other golden moved.
- The fold, at every width from each configuration's narrowest bar, with no part of the bar
  past its edges:
  - with an icon leading, a labelled leading, or none;
  - with the title centred or not;
  - with and without a title;
  - with a 12 px actions padding.

  The narrowest widths are now the sums of the parts: for an icon leading, the 56 px slot,
  the 16 px spacing, the 64 px title floor and the 48 px overflow button make 184.
- An untitled bar shows its action at exactly the action's width, centred or not, and folds it a
  pixel narrower.
- The same three actions all show on a wide surface and fold on a narrow one, counted by their
  words.
