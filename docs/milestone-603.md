# Milestone 603 — A menu bar, with submenus

Issue #33: `MenuAnchor` was the whole of the menu system. There was no bar, no submenu, and
none of the behaviour that makes a menu a menu. This milestone does the bar, the submenus
and the pointer. The keyboard and the demo come next.

## The question the issue asked first

"Which menu is open, and how deep" has to live somewhere: in the application, or in `Runtime`
next to focus and scroll.

**In the application**, as a `MenuPath`:

- **Every other menu here does that.** `PopupMenuButton`, `MenuAnchor`, `DropdownButton` and
  `DropdownMenu` are all told whether they are open. A bar whose state lived somewhere else
  would be the one menu that works differently.
- **`update` stays the one place the interface's state changes**, and a test can drive the
  bar with no window: every move below is a message.
- **The cost is one field and one message.** A bar's state is a path rather than a flag, so the
  path is a type of its own that carries the moves: `opened(level, index)`, `truncated(level)`,
  `up()`, `closed()`. The application stores it and does no arithmetic.
- An application built from components keeps the path in a hook, as it would a flag.

What `Runtime` would have given, keeping the path across a rebuild without the application,
is what the application's own state already gives.

## What it does

```rust
MenuBar::new(&self.menu, Msg::Menu)
    .menu(
        SubmenuButton::new("File")
            .item(MenuItem::new("New", Msg::New).shortcut("Ctrl+N"))
            .divider()
            .submenu(SubmenuButton::new("Open recent").item(MenuItem::new("notes.txt", Msg::Recent(0))))
            .item(MenuItem::new("Quit", Msg::Quit)),
    )
    .menu(SubmenuButton::new("View").item(MenuItem::checked("Word wrap", wrap, Msg::Wrap)))
```

- A press on a menu's word opens it, and a second press closes it. That second press lands on
  the open menu's press-outside, which covers the word, so the word itself only ever opens.
- **Once one is open, moving onto another word opens that one**, with no press.
- Moving onto a row that opens a submenu opens it **beside** the row. Moving onto any other row
  of that menu closes it again. A single open path: opening one submenu closes a sibling's.
- A press outside every open menu closes them all.
- A row sends its own message. The application closes the menus when it handles it, as a popup
  menu's rows do: a press is one message, and it is the row's.
- The rows are `MenuItem`s, so everything a popup menu's row can be, a bar's row can be too:
  ticked, with an icon, with a shortcut shown, disabled, or a rule.
- A disabled menu is shown greyed and does not open.

## How

- **The rows are the popup menu's rows.** `PopupMenuButton`'s panel building moved into
  `menu_panel`, which a menu bar calls too. The panel is the same surface, as wide as its widest
  row. A new `decorate` argument lets the bar wrap each row: in the hover that opens or closes a
  submenu, and in the portal that floats the submenu.
- **A row that opens a submenu ends in a chevron**, pointing to the end side and mirrored in
  right-to-left. Its width is counted when the panel measures its rows.
- **A new placement, `Placement::Beside`**, is on the anchor's end side with the first row level
  with the anchor. With no room there, it opens on the start side rather than over its own row.
  With room on neither side, it is held inside the window, as every anchored overlay is.
- Submenus are overlays inside overlays. The overlay pass already draws what an overlay adds.
- The word on the bar is a tap target tall, lit while its menu is open, and announced as a
  button.

## Not yet

- **The keyboard**: arrows within and across menus, Escape closing one level (`MenuPath::up`
  is there for it), Enter. #33's "done when" asks for it, and it is the next milestone.
- **The delay before a submenu opens**, and the forgiveness for a pointer travelling
  diagonally towards it. Here a submenu opens as soon as the pointer is on its row.
- **The underlined accelerator letter.** The shortcut shown on a row already exists.
- The demo.

## Verification

- `MenuPath`: opening at a level closes what was below it, and `truncated` and `up` close from
  a level.
- Closed: no rows are shown. A press on "File" sends `[0]` and on "View" `[1]`. Moving over a
  word opens nothing.
- `[0]` open: "Quit" is under the bar and sends its message. The word sends `closed`, and so
  does a press elsewhere on the window.
- `[0]` open: moving onto "View" sends `[1]`, and moving onto "File" sends nothing.
- Moving onto "Open recent", or pressing it, sends `[0, 2]`. With `[0, 2]`, "notes.txt" is
  beside the row and level with it, and sends its message. Moving onto "New" sends `[0]`.
- A disabled menu sends nothing when pressed.
- Near the end edge of a 560-px window, a submenu with no room beside its row opens on the
  start side, inside the window.
- The popup menu's 31 tests pass after the panel moved.
- Mutations, each failing a test:
  - the submenu placed under its row;
  - no flip at the window's edge;
  - no switching between words on hover;
  - no submenu on hover;
  - a sibling row not closing the submenu;
  - no press-outside;
  - a disabled word still pressable;
  - a disabled menu still opening.

  A ninth, a word that closes its own open menu, survived. It could not be reached: the
  press-outside is over the word. It was taken out.
