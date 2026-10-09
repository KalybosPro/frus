# Milestone 640 — The menu bar on the title bar's line

## Objective

A desktop application's menu bar shares the window's title bar line, as a code editor's
does: the window's icon, then File, Edit, View…, then the minimize, maximize and close
buttons. The request was specific about two things:

- frus does **not** manage the window: the system keeps its icon and its three buttons;
- the API is `Scaffold::menu_bar(MenuBar)`.

## What the system allows

- **Windows.** The caption is drawn in the non-client area. The documented way to share it
  ("Custom Window Frame Using DWM") extends the frame into the client area. The desktop
  compositor then keeps its three buttons there and acts on them: it answers their hit test
  (`DwmDefWindowProc`), and it minimizes, maximizes, closes and shows its snap layouts.
- **But the compositor's buttons are drawn under the window's content.** A prototype showed
  them hidden by frus's GPU-rendered content. Keeping them visible over GPU content needs
  the Windows App SDK, a separate runtime. Code editors and browsers paint their own.
- So, as agreed: **frus paints, Windows acts.** frus paints the icon and the three buttons
  exactly where the compositor has them (`DWMWA_CAPTION_BUTTON_BOUNDS`), in the state the
  compositor reports (hover, press), and Windows does the rest.
- **macOS** allows the same with its own buttons kept by the system (a transparent title
  bar). It is not done here: it could not be tried on a Mac. There, as on **Linux**, where
  the window manager draws the bar, the menu bar is the first line under the system's bar.

## What was done

- **`Scaffold::menu_bar(MenuBar)`**: the menu bar at the very top of the window, above the
  app bar. The scaffold takes the row's height off everything else.
- **`MediaQuery::title_bar: Option<TitleBar>`** says when the line is shared, how tall it is,
  and what the system keeps on it: where its buttons are, the window's icon, which button
  is hovered or pressed, whether the window is maximized or active.
- **On the shared line** the row is the system's height, with the application's icon
  (`AppIcon`, the frus logo by default) at its start, then the menu bar, then an empty stretch,
  then the three buttons painted where Windows has them. The middle button shows the restore
  glyph when the window is maximized, the close button turns red under the pointer, and an
  inactive window's glyphs are quieter.
- **What the system does:**
  - a press on the icon opens the window menu, and a double press closes the window
    (`HTSYSMENU`);
  - the empty stretch moves the window and maximizes it on a double click (`HTCAPTION`);
  - the three buttons are the compositor's;
  - the top edge resizes the window;
  - a menu bar's words are frus's (`HTCLIENT`).
- **The shell** (`title_bar.rs`, Windows): it shares the line when the frame has a menu-bar
  row (`Widget::wants_title_bar`) and gives it back when it no longer does, for example when
  a narrow window drops the menu bar. Each frame it tells the window procedure what of the
  line is the application's (the boxes a press can land on) and where the icon is
  (`Widget::title_bar_role`).
- **`CaptionButtonsTheme`** (`theme.widgets.caption_buttons`): every colour of the buttons is
  the caller's to say.
- **The demo** puts its menu bar in `Scaffold::menu_bar` on a wide window.

## Also fixed

A plain build of `frus-shell` warned that `Clipboard::Memory` is never constructed. The
in-memory clipboard exists for the test driver and is only made under `cfg(test)` or the
`testing` feature, but the variant itself was always compiled. CI never saw it, because its
clippy builds with the tests. The variant and its two arms are now behind the same `cfg`.
Clippy on Windows is clean for the library, with the tests, and with `testing`.

## Checked on Windows 11

Run on the maintainer's machine, driven by real clicks:

- the icon, File, View and Go, and the three buttons on one line, with the app bar under it;
- a press on File opens its menu;
- dragging the empty stretch moved the window by exactly the pointer's travel (80, 50);
- the painted maximize button maximized the window, and close closed it;
- maximized, the line stays on the screen and the middle glyph is the restore glyph;
- the close button turns red under the pointer, and minimize takes a wash;
- narrowed until the menu bar goes, the system's own title bar comes back.

## Tests

- `a_scaffold_s_menu_bar_is_its_first_line`: off the shared line, 30 px, above the app bar,
  asking for the line.
- `on_the_title_bar_line_the_system_s_parts_are_painted_where_it_has_them`: the system's
  height, the icon at the start, the buttons where the system has them.
- `the_buttons_take_the_system_s_states`: three glyphs at rest, red on close, a wash on the
  others, and the restore glyph when maximized.
- The Windows module runs only on Windows, and CI is Linux; it was exercised on the
  maintainer's machine as above, and linted there with clippy.

Mutation testing: six mutants, five killed: the close button's red, the restore glyph, the
row's height on the shared line, the window's icon, and the walk asking for the line. The
survivor takes the menu row's height off the rest of the scaffold. Without it, the column the
scaffold is laid out in still shrinks the scaffold to the room left, and its layers are laid
out against that box, so the footer and the floating action button land in the same places;
the test checks both. The subtraction is kept so that the numbers the scaffold computes with
are the box it gets, rather than leaving it to the column to shrink.
