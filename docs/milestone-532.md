# Milestone 532 — A menu on a panel, as the reference draws one

Milestone 530 drew the shadows the reference draws and this did not, all but three: the
menus. `MenuAnchor`, `DropdownMenu` and `DropdownButton` stand at 3, 3 and 8 in the reference,
and here none of them had anything to cast a shadow *from*. This milestone gives them the
surface first, then the shadow.

## What was wrong

- **`MenuAnchor`** floated whatever it was given. It painted nothing of its own — no surface,
  no corner, no padding — so an application had to bring a surface in its content (the demo's
  was a `Card`), and each one brought a different one.
- **`DropdownButton`** and **`DropdownMenu`** floated a column of rows, each drawn as its own
  filled, outlined, rounded box, four pixels from the next, with nothing behind the column.
  The page showed through every gutter. A shadow under the column would have been grey bands
  in the gaps; a shadow under each row, a stack of floating cards.

`PopupMenuButton` had been through this already. Milestone 460 found its menu drawn the same
way and gave it a `Panel`: one surface, rows with no box of their own, a shadow at 3.

## What the reference does

**A menu anchor's panel** takes each property from the widget's style, then the menu theme,
then its Material 3 menu defaults, resolved property by property in the panel's build, and
hands them to a `Material`. The defaults are:

| Property | Value |
| --- | --- |
| surface | `surfaceContainer` |
| shape | a rounded rectangle, radius 4 |
| padding | 8 above and below, nothing either side |
| elevation | 3 |
| shadow colour | the scheme's `shadow` |
| surface tint | transparent |
| clip | a hard edge, the anchor's default |

The children go inside that padding, in a scroll view, so a menu taller than its room scrolls
between the two strips of padding. Its rows are menu item buttons: a transparent background
and an overlay of `onSurface` at 8 % hovered, 10 % focused and 10 % pressed. No row has a
shape or an outline of its own.

**A dropdown menu** hands its own menu style to that same anchor. Its defaults name only a
minimum and a maximum size, so everything else falls through to the menu theme and the menu
defaults above: `surfaceContainer`, radius 4, 8 above and below, 3 high in the scheme's
shadow. The entry the keyboard is on is given a background of `onSurface` at 12 %.

**A dropdown button's list** is its own thing. It reads neither menu theme:

| Property | Value |
| --- | --- |
| surface | the widget's `dropdownColor`, else the theme's canvas colour — `surface` in Material 3 |
| shape | the widget's `borderRadius`, else a radius of 2 in the painter, unclipped |
| padding | the material list padding, 8 above and below |
| elevation | 8, from its constructor, cast from the elevation shadow table |

The selected item is marked with the theme's focus colour, `onSurface` at 12 %.

## The decision

Milestone 530 set out three ways: give the three the popup menu's panel, shadow what was
there, or leave the surface to the caller. **The owner chose the first**, knowing it is a
visible change to every application with a dropdown in it, because it is what the reference
draws and the other two are not.

## Design

**One panel, not three.** `menu.rs`'s private `Panel` is now shared by all four widgets that
float a list. It had been built for the popup menu alone, with that menu's row-measuring
folded in; the measuring is now optional. The popup menu still hands the panel its rows to
measure its width by, and the other three hand it a panel as wide as what is inside it.

**Whose theme.** A new private `PanelKind` says which theme a panel answers to once its
caller has said nothing, following the reference:

- `Menu` — `PopupMenuButton`, `MenuAnchor` and `DropdownMenu` — reads `MenuTheme`, then 3.
- `Dropdown` — `DropdownButton` — reads new `menu_*` fields on `DropdownTheme`, then 8.

A private `PanelStyle` holds what a caller said: background, shape, elevation, shadow colour,
padding, each an `Option`. It resolves each value as the caller's, then the theme's, then the
framework's.

**The values.** The surface is `surface_container` for all three, the elevation 3, 3 and 8,
and the padding 8 above and below. The corner is `theme.radius`, the framework's one corner,
as the popup menu's has been since milestone 460; the reference's 4 and 2 are one
`radius(..)` away. The shadow colour is the scheme's shadow at 30 %, the framework's colour for
a height. Following milestone 529, **a panel casts only when its height is above nought and
its shadow colour is not transparent**. The colour is used as given, alpha included.

**Two departures, both deliberate:**

- **A dropdown button's surface is `surface_container`, not the reference's `surface`.** The
  owner's choice named the popup menu's panel, and on a dark theme a list in `surface` over a
  page in `surface` shows no edge except its shadow. It is `menu_background` on the widget or
  on `DropdownTheme` for an application that wants the reference's.
- **No panel clips.** A clip takes the clipping widget's own paint with it, and here the
  shadow *is* that paint — milestone 530's expansion panel list found exactly this. The eight
  pixels above and below keep a row's highlight off the corners instead, as they already did
  for the popup menu. A dropdown menu capped by `max_visible` clips its rows in its scroll
  viewport, and that viewport sits inside the panel's padding, where the reference puts its
  scroll view.

**The rows.** A dropdown option no longer draws a box. At rest it paints nothing. Selected,
it paints the tint it always had (the panel's surface moved 14 % towards the primary). Under a
pointer it paints the state layer over that. The tint is now measured from the panel's
*resolved* surface, so a caller's background still tints correctly. The rows are contiguous:
the four-pixel gap is gone, and so is the gap in a capped dropdown menu's viewport height.
The dropdown button's header is a control, not a row of the list, and keeps its outlined box.

**`MenuAnchor` keeps its content** in a shared pointer, as the popup menu keeps a caller's
row, so a builder written after `content` still rebuilds the panel around it. The demo's
anchor content was a `Card`; it is a padded `Container` now, since a card on the panel would
be a surface on a surface.

**New public API.**

- `MenuAnchor::background`, `shape`, `radius`, `elevation`, `shadow_color`, `menu_padding`.
- `PopupMenuButton::shadow_color`.
- `DropdownButton::menu_background`, `menu_shape`, `menu_radius`, `menu_elevation`,
  `menu_shadow_color`, `menu_padding`.
- `DropdownMenu::menu_background`, `menu_shape`, `menu_radius`, `menu_elevation`,
  `menu_shadow_color`, `menu_padding`.
- `MenuTheme::shadow_color`.
- `DropdownTheme::menu_background`, `menu_shape`, `menu_radius`, `menu_elevation`,
  `menu_shadow_color`, `menu_padding`.

Keyboard navigation, selection, focus trapping and dismissal are unchanged. The rows are still
focusable and still carry their messages and semantics. The panel is `opaque`, so a press on
its own padding reaches no page behind it and does not close the menu.

**Two themes that could not be named now can be.** `MenuTheme` and `DropdownTheme` were public
structs in a private module and never re-exported. A caller could set
`theme.widgets.menu.background` but could not write the type's name. The strict doc build
found it, since the new documentation links to both, and both are exported now.

## Verification

**Tests** — sixteen new, each reading what is painted:

- `MenuAnchor` (5):
  - The content floats on one `surface_container` rectangle. It is painted before the content,
    as wide as the content, 16 taller, rounded, and unoutlined. Under it is one shadow of blur
    20, dropped 6, in the scheme's shadow at 30 %.
  - The theme's surface, corner, height, shadow colour and padding are honoured, and the
    anchor's own over them.
  - A transparent shadow colour on the anchor or on the theme casts nothing, and neither does
    a flat panel, not even as an unblurred box.
  - Each look builder, written alone after the content, gives the same frame as written
    before it, and changes something.
  - A press on the panel's padding does not close it.
- `DropdownButton` (5):
  - An open list paints exactly two crisp rectangles: the outlined header and one unoutlined
    panel three rows tall plus 16, with no gutters.
  - One shadow of blur 40, dropped 16, and none when shut.
  - `DropdownTheme`'s `menu_*` fields are honoured, and the widget's over them, including a
    selected tint measured from the resolved surface.
  - A transparent or flat panel casts nothing.
  - The second option still answers a press where it is drawn, the panel's own room
    swallows one, and every option is still focusable.
- `DropdownMenu` (5):
  - Under the field sit only the panel and the selected strip, unrounded.
  - One shadow three high.
  - `MenuTheme` is honoured and the widget over it.
  - A transparent colour casts nothing.
  - A list capped at two is a panel of two rows plus 16, clipped by nothing but the window,
    shadow included. The rows are clipped to a viewport that starts 8 inside the panel, two
    rows tall and within its width, and the last choice is laid out below that viewport.
- `PopupMenuButton` (1): its shadow is the scheme's at 30 %, the caller's, the theme's, the
  caller's over the theme's, and nothing when transparent on either.

One existing test moved with the change: the dropdown menu's index test reads a row at the
field's 56 plus the panel's 8, with the 4-pixel gap between rows gone. Two first drafts were
wrong and were corrected against the scene, not bent to pass. The dropdown panel sits 4 below
the header's box, the gap an overlay placed below its anchor always leaves. The scroll view
clips with the clip rectangle each primitive carries, not with a layer.

**Goldens** — four changed, each looked at before it was accepted. Every other golden passed
unchanged, and no other `.actual.png` was written.

- `dropdown_menu`: the three separately outlined boxes with dark gutters between them are one
  rounded panel under the header. Medium's selected strip runs edge to edge with its tick, and
  a soft shadow shows round the panel on the dark ground.
- `dropdown_menu_filtering`: the same under the field. The two matches sit on one panel with
  room above Dark green, whose selected strip reaches both sides.
- `dropdown_widget_options`: the swatch row, the two-line row, the greyed "Out of stock" row
  and Blue share one panel. The rows keep their heights and their tick column; only the boxes
  and gaps are gone.
- `popover_and_portal`: the anchor's content, a small rounded box of its own, now sits on a
  panel 8 taller above and below, with a soft shadow. The portal below it, which is not a
  menu, is unchanged.

**Checks** — all clean:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p frus-widgets --lib` (1619 passed)
- `cargo test -p frus-demo --lib` (61)
- `cargo test -p frus-test --no-fail-fast` (goldens 102, widgets 47, and the rest)
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p frus-widgets` — clean once the two themes
  were exported, as above

**Mutations** — ten, all killed in the end. Each was applied by a script that checked its
target occurred once, ran the module's tests, wrote the file's original bytes back and compared
the working tree's diff hash with the one it started from. They ran one at a time, and each was
restored.

1. A transparent shadow colour still casts.
2. A dropdown button's list back at 3.
3. The caller's shadow colour ignored.
4. `DropdownTheme::menu_background` ignored.
5. An option outlined again.
6. The anchor floats bare content.
7. The capped viewport keeps the old gutters.
8. A dropdown's padding read from the menu theme.
9. A `MenuAnchor` builder that does not rebuild.
10. The selected tint measured from the default surface.

**One survived the first run: the builder that does not rebuild.** The order test wrote two
builders after the content, and the second rebuilt the panel for both, so the first one's
missing rebuild was hidden. The test now writes each of the five builders alone, and the
mutation, run again on its own, was killed.

## What is left

- **The dropdown button's own values.** Its surface is `surface_container` and not the
  reference's `surface`, and its corner is the framework's and not the reference's unclipped
  2. Both are said above, and both are one builder away.
- **The rows' own values.** A dropdown row is 40 tall where the reference's items are a tap
  target, 48. The selected option is tinted towards the primary where the reference's dropdown
  menu gives the keyboard's entry `onSurface` at 12 %. Both are older than this milestone, and
  both are a change of their own to every dropdown.
- **A menu anchor taller than its room does not scroll.** The reference's panel wraps its
  children in a scroll view; here the content is free, and a caller whose content can be long
  still has to scroll it.
- **The corner and the padding at their defaults.** The framework's corner is 10 and the room
  above the first row is 8, so a highlighted first or last row meets the curve by a fraction of
  a pixel. The popup menu has had the same since milestone 460. A panel that clipped would cut
  its own shadow away.
- **No surface tint.** The reference's menu defaults make it transparent, so there is nothing
  to paint, and it is not exposed here.
- **A dropdown menu reads the menu theme directly.** The reference puts its dropdown menu
  theme's menu style over the menu theme. There is no `DropdownMenuTheme` here yet.
- **The shadow's shape**, one blurred rectangle for a height — see milestone 529.
- **The dropdown button's closed header** is still a menu row and not a field (see the
  roadmap). It keeps its outline here because it is the control.
