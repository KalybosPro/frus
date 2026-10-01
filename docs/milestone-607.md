# Milestone 607 — A menu bar in the demonstration, and what it found

Issue #33 is done when a menu bar is in an application and works there. Milestones 603 and 605
built it and tested it as a widget. This puts it in the demonstration, which found two defects
on the way: one in the widgets, one in the tool that draws the README's pictures.

## The bar

On a **wide window** (the `Expanded` size class, 840 logical pixels and up), the home screen
has a menu bar above its header:

- **File**: Save, Load, a rule, Clear completed… (which asks first, as the button does).
- **View**:
  - Dark theme and Right to left, ticked when on;
  - Next colour;
  - a rule;
  - **Text size ▸** Larger, Smaller;
  - **Language ▸** System and the three languages, the one in force ticked.
- **Go**: Tasks, Stats and About, the one on show ticked; a rule; Log and Settings.

A phone and a medium window do not get it: their header folds the same actions into its "⋯"
menu. The bar is what a desktop application has, and the header is what a phone has. The size
class decides, as it decides elsewhere on this screen.

The screen keeps which menu is open as a `MenuPath`, beside the other things only it cares
about (the filter, the drawer). Every row does its work and then closes the menus, as a popup
menu's row does. The language submenu needed a way to pick a language directly, where the
header's button cycles through them: `Demo::set_lang`.

## A row with its submenu open was narrow

With a submenu open, its row's chevron was drawn **off the menu's start edge**, at the far left
of the window. The row was as narrow as its label, and a press near its end missed it.

A row with its submenu open is wrapped in the portal that floats the submenu. A portal lays its
anchor out as the only child of a container, and that container was a **row**: it stretched the
anchor to its height and left it at its own width. The menu stretched the portal across, and
the portal did not pass it on.

The portal now lays its anchor out in a **column**, which stretches it across. An anchor that
fills only the height keeps the row. Nothing else changed: every golden, and every widget test,
passed unchanged.

## The README's pictures were empty

`cargo run -p frus-demo --features shots --bin shots` draws the README's pictures. Every one came
out **empty**: the background, and an overflow band down the left.

The tool called `view` inside the surface's description, and laid the tree out and painted it
outside. Since the demonstration became components (milestone 556), the screens are built during
the layout, and they ask the surface how big it is then. Asked outside it, they were told
nothing. The tool also drew the first frame's tree, from before the router had started.

The still pictures and the moving one now build the whole frame inside the surface, and draw the
tree built after the router's first frame. The pictures in `docs/media` were not regenerated
here. The next time they are, they show the menu bar, at 900 pixels wide.

## Verification

- **Through the shell**, on the demonstration at 1000 × 700:
  - the bar shows File, View and Go;
  - pressing View opens its menu and its first row takes the focus;
  - the pointer moving onto Go opens Go and closes View, with no press;
  - Escape closes Go;
  - in View, Language opens its submenu, and Escape closes the submenu and leaves View open;
  - Dark theme switches the theme and closes the menus;
  - Français picks French.

  At 400 × 800 there is no bar.
- A row with its submenu open: a press near its end reaches it, the row below reaches as far,
  and its chevron is past its label.
- Rendered with View and Language open: both panels as wide as their widest row, the chevron of
  every row at its end, the language ticked.
- Mutations, each failing a test:
  - the portal's anchor laid out in a row again;
  - the bar on a phone instead;
  - a row that does not close the menus;
  - a language that is not set.
- The arrow keys are not driven through the shell here: its test driver has no way to press a
  key yet. Milestone 605's tests drive them on the widgets, and through the shell's focus.
