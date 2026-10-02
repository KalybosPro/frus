# Milestone 608 — Pressing a key in a test, and what it found

Milestone 607 left a gap: the menu bar's arrow keys were tested on the widgets, but not through
the shell, because the shell's test driver had no way to press a key. Winit's key event cannot be
made outside winit, and the shell's handling of a key read it directly.

## A key the shell can be handed

- **`KeyDown`**, inside the shell, is what it reads of a key going down: the key the layout makes
  of it, where it is on the keyboard, the text it types, and whether it is repeating. It is made
  from winit's event in a window.
- **`App::key_down`** is everything the shell does with a key, moved unchanged out of the event
  loop's match arm:
  - the system's back key and the inspector;
  - shortcuts;
  - Tab;
  - Escape;
  - the arrows;
  - activation;
  - the clipboard;
  - typing.

  The arm now makes a `KeyDown` and calls it. The system's back key quits through the event loop
  as a last resort; handed none, that last resort does nothing.
- **`Driver::key(KeyStroke)`** presses a key in a test, in the vocabulary shortcuts are already
  bound in. A stroke's modifiers are held down for that key and let go after it. A letter is also
  given its place on a QWERTY keyboard, which is where the clipboard's shortcuts are read from.
  The system's back key is not offered: what it does last is quit, and a driver has no loop to
  quit.

The 264 shell tests passed unchanged after the move.

## What it found: the focus could not find its way back

Driven through the shell, the menu was worked from the keyboard (down to a row that opens a
submenu, right into it, down, left back out), and **the focus did not go back to the row.** It
went to the menu's first row.

A widget's identity is its place in the tree. The menu bar wrapped a row in the portal that
floats its submenu only while the submenu was open, so opening the submenu moved the row to a
different place. The shell keeps a history of where the focus has been, so that it can go back
when what has it disappears. Here the row it would have gone back to no longer existed once the submenu had
opened, so it was never recorded. The word on the bar had the same fault: closing its menu with
Escape did not put the focus back on it.

Milestone 605's test of Escape had passed by coincidence: the row it expected the focus on was
the menu's first, which is where the focus fell anyway.

**Now the portal is there, open or shut.** A row that opens a submenu, like a word on the bar,
is in its portal whether or not anything floats from it. The region that opens a submenu on
hover still comes and goes: a mutation that made it permanent changed nothing, because a region
is transparent and takes no place of its own. A test now keeps the focus on the menu's last
row, so that a coincidence cannot pass for a return.

A portal with nothing floating is laid out like any container, and its default row would have
shrunk the anchor: the fault milestone 607 mended for an open portal. Its style is now a column.

## Verification

- **Through the shell**, on a menu of New, Open recent ▸ (notes.txt, todo.md) and Quit:
  - Down from New reaches Open recent;
  - Right opens the submenu, and notes.txt has the focus;
  - Down reaches todo.md;
  - Left closes the submenu, and the focus is back on Open recent;
  - Up reaches New;
  - Enter picks it, and the menus close.
- With the focus on Quit, a submenu opened by the pointer and closed with Escape hands the
  focus back to Quit.
- Tab onto File, then Enter, opens the menu with the focus inside it; Escape closes it, and the
  focus is back on File.
- A stroke with Ctrl held lets Ctrl go after it.
- A right-click on a field opens its selection bar, showing only what applies: Select all, with
  nothing selected and nothing to paste. Milestone 568 had built that path and not run it.
- A row with its submenu shut is as wide as the menu.
- Every widget test and every golden, unchanged.
- Mutations, each failing a test:
  - the word's portal only while its menu is open;
  - a row's portal only while its submenu is open;
  - the portal laid out in a row;
  - the modifiers not let go after a stroke.
