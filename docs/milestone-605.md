# Milestone 605 — The menu bar from the keyboard, and `Widget::autofocus`

Issue #33 asks for a menu bar that is "operable entirely from the keyboard". Milestone 603 did
the pointer. This is the keyboard, and the one general thing it needed.

## What it does

With a menu bar's menus:

- **Opening a menu gives its first row the focus.** That is the first row that can be used:
  not a rule, not a disabled row. The same goes for a submenu. From there:
  - **Up and down** move between the rows, as the arrows move the focus anywhere.
  - **Right** on a row that opens a submenu opens it. On any other row, it opens the next menu
    along the bar.
  - **Left** in a submenu goes back to the menu it came from. In a menu off the bar, it opens
    the menu before. Both go round the ends of the bar.
  - **Enter** or **Space** on a row presses it, as they press anything with the focus.
  - **Escape closes one level**: a submenu, then its menu. The focus goes back to where it was
    before, in the menu the submenu opened from.
- **On a word of the bar with the focus**, while a menu is open, left and right open the menu
  beside it. With every menu closed, they move the focus as they do anywhere else, and Enter
  opens the word's menu.

The application does nothing new: every move is still a `MenuPath` in the one message it
already handles.

## `Widget::autofocus`

A menu that opens with the focus somewhere else cannot be worked from the keyboard: the arrows
move a focus that is not in it. Nothing in the framework could put the focus somewhere when it
appeared, so this adds the hook:

- **`Widget::autofocus`**: a focusable widget that says yes **takes the focus when it
  appears**. The reference has the same flag on its focus nodes, and a field on a screen made to
  be typed into wants it as much as a menu.
- The walk records it on the frame's focus stops, and the shell, after each frame, gives the
  focus to one that **was not there on the frame before**.
- **Where the focus came from is remembered** by the history the shell already keeps for an
  overlay closing (it hands the focus back to the trigger). When the widget goes, the focus goes
  back. This is how Escape out of a submenu lands in the menu again.
- Wrappers forward it.

The test harness now does what the shell does after each frame, and gives the focus back and
hands it to what takes it. It was left out before, so no test of the harness could see a focus
return.

## How the keys reach the menus

- **Left and right** are offered only to the widget with the focus. The row itself answers
  them: a popup menu's row learnt to carry what they send (`RowKeys`), and a menu bar fills it
  in. The word on the bar answers them the same way.
- **Escape** travels up from the focus. The row with the focus answers it first, with the path
  that closes its own menu and nothing more, so the innermost menu closes first.
- A row that cannot be used, and a word that cannot, answer no key, as they answer no press.
  The repository's own check, that every hook of a control with an `enabled` flag consults it,
  caught the first version: Escape was on the panel, which has no such flag, and the row and
  the word answered the arrows even when disabled.
- A bar is built whole again each time a menu is added, because a word's and a row's arrows
  name the menus beside them, and the last menu's next is the first.

## Not yet

- A screen reader hears each row as a button, a ticked row as a checkbox, and a word as a
  button. There is no menu role in the framework's vocabulary yet, nor an "expanded" state.
- The underlined accelerator letter, the delay before a submenu opens on hover, and the demo.

## Verification

- The first usable row of an open menu is the frame's one widget that takes the focus. A rule
  and a disabled row are passed over. An open submenu's first row takes it too.
- Right on "Open recent" opens it. Right on "New" opens the next menu, and left on "New" opens
  the one before, round the end. From "notes.txt", left goes back to `[0]` and right to the next
  menu.
- On the bar with a menu open, left and right on a word open the menu beside it. With all
  menus closed, they send nothing and leave the focus to move.
- Escape, travelling up from "notes.txt", closes the submenu only. From "Quit", it closes the
  menu.
- Through the shell and its frames: opening the menu focuses "New", opening the submenu
  focuses "notes.txt", and Escape closes the submenu and puts the focus back on "New". A second
  Escape closes the menu.
- Mutations, each failing a test:
  - a row that never takes the focus;
  - the first row taken whether it can be used or not;
  - left and right the wrong way round;
  - the bar's arrows switching menus with none open;
  - no menu counted beside another;
  - no Escape on a row, whether the row does not answer it or the bar gives it nothing;
  - Escape closing everything;
  - left from a submenu going to the menu before;
  - the shell not giving the focus to what appeared;
  - the harness not reconciling the focus after a frame.
