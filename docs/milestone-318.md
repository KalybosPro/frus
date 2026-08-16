# Milestone 318 — Chrome that changed shape, and the middle term that could not be added

The sweep's next entry was `AppBar`, carried on the roadmap as one line: `center_title`
resolves `caller ?? platform`, missing the theme in between. It turned out to be two
findings of very different sizes, and only one of them could be acted on here.

## The bar had no height of its own

`AppBar` sized itself to whatever was in it. A bar with a title measured one height; the
same bar with a leading button and two actions measured another; a longer title that
wrapped measured a third.

That is chrome behaving like content. The consequences are not subtle once named: every
screen in an application is a slightly different shape, and the page **moves** the moment
an action appears or disappears — a button that shows up when a selection is made pushes
the whole body down a few pixels.

The reference gives the bar a fixed `toolbarHeight`, **64** in Material 3, and lets the
content sit inside it. So does this now, with `AppBar::height` still overriding, and a test
that measures a bare bar against a busy one and insists they are the same.

Two type sizes came with it, both read rather than remembered: the title is `title_large`
(22, was 20), and the actions are `label_large` (14, was 16) — the reference's app-bar
actions are text buttons, and that is a text button's label.

## The middle term could not be added, and the reason is worth writing down

`center_title` should resolve `caller ?? theme ?? platform`. It cannot, and the obstacle is
not `AppBar`.

`AppBar` is a **builder**: `AppBar::new(…).action(…).build()` hands back a finished
`Box<dyn Widget>`, and `build()` never sees a `Theme`. Milestone 309's chain works for
painted properties because `paint` is handed the theme, and milestone 310 extended it to
layout through `style_themed`. Neither helps here, because `center_title` is not a property
that gets painted — it decides **which children exist and in what order**. By the time a
theme is in reach, the composition has already been made.

Nothing in the framework builds *from* the ambient theme:

- `Themed` pushes a theme **down** to a subtree. The wrong direction.
- `LayoutBuilder` builds lazily, but takes a `Size` and nothing else, and its own
  documentation says it has **no retained state** — an app bar built inside one would lose
  its focus and its overflow menu.
- `Widget::children` returns a **borrowed slice**, so the children have to exist before the
  walk arrives.

So the general fix is a primitive that does not exist: a widget that builds its subtree
from the ambient theme during the themed walk, cached per frame. Unlike `LayoutBuilder` it
could keep retained state, because the theme is stable frame to frame while a box is not —
identity stays positional and stable. It would unblock `AppBar`, and with it `Scaffold`,
`NavBar`, the pickers, and every other composition that is assembled before a theme is in
hand. It is how the reference works: everything reads `Theme.of(context)`.

That is a milestone about the walk, not about a bar, and it is on the roadmap as one.
Doing it inside an app-bar pass would have been the wrong shape of change.

## Verification

1065 tests (1 new), clippy silent. The goldens that moved were read.

## Left

- **`center_title`, the background and the foreground still have no theme term** — the
  above.
- **No scrolled-under elevation.** The reference raises the bar to elevation 3 when content
  scrolls beneath it, and keeps it flat otherwise. Here elevation is a fixed number the
  caller gives, and nothing knows whether anything is scrolled under.
- **The bar does not tint with the surface.** The reference's `surfaceTintColor` is
  transparent in Material 3, so this matches by accident rather than by decision.
