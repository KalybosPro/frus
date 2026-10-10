# Milestone 642 — The menu bar is the window's, not a page's

## Objective

J640 put the menu bar on the title bar's line with `Scaffold::menu_bar(MenuBar)`. The
maintainer's question afterwards: a desktop application has **one window**, and its pages
take turns inside it, so is a page's scaffold the right owner for what sits on the window's
title bar? It is not, and this milestone moves the bar to where the window's things go.

## What was wrong with a page's scaffold

Read from the code, then confirmed in the demo:

- **The title bar changed with the page.** The shell shares the line while the frame has a
  menu-bar row. The demo's home screen had one; its other screens have scaffolds of their
  own without one. Opening a task gave the line back to the system, and coming back took it
  again: the frame changed and the content jumped by the caption's height at every page.
- **A transition showed the bar moving.** During a push both pages are on screen, so two
  rows were laid out, and the bar slid out with the page leaving.
- **A page without a scaffold had no bar.**
- **An open menu was the page's state**, though the menus are the window's.

## What the reference does

- **The bar is an ordinary widget** that "typically resides above the main body of an
  application (but can go anywhere)" (`material/menu_anchor.dart`). Its examples put it in
  the home page's scaffold body, and it has no notion of a window. The one app-wide menu is
  the platform menu bar, which draws nothing: the system draws it.
- **What goes above the pages** goes in the app's `builder`, "for inserting widgets above
  the Navigator" (`widgets/app.dart`).
- **The back swipe is a page's**: a strip on the start edge of each pushed page, inside the
  page's transition (`cupertino/route.dart`), so it cannot reach anything above the
  navigator.
- **The reference sets the native title bar dark or light** with the system's apps, and
  nothing more. It does not draw on the title bar.

frus does what the reference leaves to plugins: the menu bar on the title bar's line. So
the bar follows the reference's shape for what belongs to the window: **above the pages,
put there by the app's builder**.

## What was done

- **`WindowMenuBar::new(bar, pages)`** (in `titlebar.rs`): the bar's row, then the pages
  under it. The pages are built for the window less the row: their `MediaQuery` is shorter
  by it, its top intrusion is consumed, and it has no title bar line. `None` for the bar (a
  narrow window) gives the pages the whole window and the line back to the system. **The
  tree keeps its shape** either way, so the pages keep their state when the bar comes and
  goes.
- **`FrusApp::builder(|cx, pages| ..)`**: what the window shows around its pages. The pages
  are their own component, built where the builder puts them, so a scope the builder sets
  reaches them. The builder may call hooks, and reach the router from the first frame.
- **`Scaffold::menu_bar` is gone.** It was never released.
- **The demo** builds its menu bar in the app's builder (`screens/menu.rs`). What the menus
  act on that was the home screen's own state is now in `Demo`, the shared state: the
  section on show and whether the "clear completed" confirmation is up. From another page,
  "Go → Stats" and "Clear completed…" go home first. "Clear completed…" also shows the
  task list, whose footer the confirmation hangs from. Asked from the Stats section, it was
  never shown, under J640 too.

## Two bugs it brought to light

**A component inside a scope was built for the scope around it.** A wrapper and the
component inside it are one node, and the walks built the component (`Widget::expand`)
*before* installing the node's scopes. So `MediaScope::tweak(.., page)` built the page for
the whole window, and `cx.theme()` under `Themed` read the outer theme. Both contradict
their own documentation. The walks now build a node under the scopes it introduces before it
is built (`expand_scoped` in `ui.rs`): the surface, the theme, the shell and the scrolling.
An unbuilt component answers for none of them, so this is exactly the wrapper's, and the
walk asks again once the component is built.

**The back gesture started on the menu bar.** The shell arms the back gesture on a press in
the window's 24 px leading edge whenever the app can go back. "File" sits in that edge, so
on any page but the first, a click on "File" started a back swipe instead of opening the
menu. "View" and "Go" worked. This was the case under J640 too. In the reference the strip
is the page's, so it never covers what is above the pages. The first fix only kept the
gesture off the bar's row, and the end-to-end test caught the next case: "Clear completed…"
in the open File menu is in the same edge, and the gesture took that press too. So the rule
is now the reference's: the gesture starts only on the pages. It does not start on the
window bar's row (`Ui::title_bar_row`), nor on an overlay: a menu's or a dialog's box, or
the whole window under an overlay that a press outside dismisses (`Ui::over_overlay`).

## The line's colours

The maintainer asked for the system's colours by default, and for them to be customizable.
On the title bar's line, by default:

- the **plain caption**, measured on Windows 11's own captions on this machine:

  | | active | inactive |
  |---|---|---|
  | light | surface `#F4F1F9`, words `#000000` | surface `#F3F3F3`, words `#919191` |
  | dark | surface `#221E29`, words `#FFFFFF` | surface `#202020`, words `#797979` |

  An active caption is tinted by the wallpaper (Mica). An opaque surface cannot follow that
  tint, so the untinted `#F3F3F3` / `#202020` is used (`#FFFFFF` before Windows 11). Light
  or dark follows the window, which follows the system's apps;
- the **accent colour** behind an active window's caption where the person asked for it on
  title bars (`DWM\ColorPrevalence`, `AccentColor`, `AccentColorInactive`), with white or
  black words, whichever contrasts more;
- under **high contrast**, the scheme's caption colours (`GetSysColor`).

They are read again when the system says they changed (`WM_SETTINGCHANGE`,
`WM_DWMCOLORIZATIONCOLORCHANGED`, `WM_THEMECHANGED`).

**Customizable.** `WindowMenuBar::background` / `foreground` / `style(TitleBarTheme)` for
one bar, and `theme.widgets.title_bar` (`TitleBarTheme`, with inactive variants) for every
bar. The bar's own setting outranks the theme, which outranks the system. Everything on the
line takes them: the row, the bar's words, the window's glyphs and their washes. The menu
bar's own `background` and `foreground_color` still outrank them. The bar's hover highlight
now leans toward the bar's words rather than `on_surface`. That changes nothing by default,
and keeps a highlight visible on an accent caption with white words.

These painted colours are the **fallback**. The maintainer chose the line painted **by the
system** (Mica, its own buttons), which needs a transparent swapchain (DirectComposition).
That is milestone 643.

## Tests

- `the_menu_bar_is_the_window_s_first_line_and_the_page_gets_the_rest`: the bar in the first
  30 px; the page is built for 1000×670 with no title bar line; its footer and floating
  button are at the window's bottom.
- `on_the_title_bar_line_the_system_s_parts_are_painted_where_it_has_them`: the line taken
  off the page, the icon and the buttons where the system has them.
- `under_a_status_bar_the_menu_bar_starts_below_it`: a tablet's status bar is consumed.
- `with_no_bar_the_page_has_the_window_and_keeps_its_state`: a hook's value survives the bar
  going and coming back, and the line is the system's in between.
- `the_line_takes_the_system_s_caption_colours`, `the_bar_and_the_theme_say_otherwise`,
  `off_the_line_the_row_is_the_menu_bar_s_own`: the precedence, for active and inactive
  windows, on the surface, the words and the glyphs.
- `a_component_inside_a_scope_is_built_for_it` (`mediascope.rs`) and
  `a_component_inside_a_themed_subtree_is_built_with_its_theme` (`themed.rs`).
- Demo: `the_menu_bar_stays_put_while_the_pages_change_under_it` drives a push and a pop
  frame by frame (60 frames each) through the shell. In every frame the bar is drawn once,
  in the same place, nothing is drawn over it, and the line is asked for.
  `the_window_s_menu_works_from_any_page` covers "Go → Stats" and "Clear completed…" from
  Settings, including the click on "File" in the back gesture's edge.

## Mutation testing

Eighteen mutants, eighteen killed. The first run left four alive, and each one pointed to a
missing test:

- the highlight leaning toward the words → `the_highlight_leans_toward_the_words`;
- the overlay's barrier and the overlay's box as places the back gesture does not start →
  `an_overlay_says_where_presses_are_its_own`. The demo's test covered both at once, and
  the File menu has both, so neither mutant alone changed anything;
- the router provided before the app's builder runs →
  `the_builder_reaches_the_router_from_the_first_frame`.

The others were the scopes in force while a component builds (the surface, the theme), the
page's surface (its height, its line, its top intrusion), every step of the colours'
precedence, the glyphs' ink, the back gesture's two conditions, and the builder itself.

## Checked on Windows 11

Run on the maintainer's machine with real clicks: the bar on the title bar's line on the
home screen and on the screens pushed over it, the menus opening, and the line in the
dark caption's colours.
