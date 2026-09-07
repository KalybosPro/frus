# Milestone 478 — A menu whose rows can be more than a word

Closes [#34](https://github.com/KalybosPro/frus/issues/34).

A menu item was a label and a message, and nothing else:

```rust
pub fn item(mut self, label: impl Into<String>, message: Msg) -> Self
```

So a menu could not have a picture on a row, a tick on the rows that are on, a rule
between two groups, a row that is unavailable, the keys that work an action, or a row of
two lines. Every real menu has at least two of those, and the demo's own overflow menu —
the `⋯` an `AppBar` folds its spare actions into — is one of them.

## The shorthands stay, and a value type carries the rest

`item(label, message)` is unchanged and is still what most call sites write.
`icon_item`, `checked_item`, `item_widget` and `divider` sit beside it for the four other
ordinary rows. Behind all five is `MenuItem`, and `entry(MenuItem)` is the general door.

The type exists because of `enabled` and `shortcut`. Both apply to **any** row, and
folding them into the shorthands means either five more methods or five methods with two
more arguments each — the combinatorial version, which gets worse every time a row gains
another property. As a value, they are written once and compose with all five.

**Not a "modify the last item" builder** — `.item("Paste", Msg::Paste).disabled()` — which
was the smaller-looking API. It has a trap in it: `item` only pushes **when the menu is
open**, so on a closed menu the modifier would find no last item and silently do nothing.
An API whose behaviour depends on a flag set three lines earlier is one people write
correctly and read wrongly.

## The leading column is the menu's, not the row's

If any row has a mark, **every** row keeps the room for one. That is decided once, when
the panel is built, and it is the whole reason the marks line up down a column.

The alternative — each row indenting itself only when it has something to show — is a menu
whose labels shift sideways as things are turned on and off, and whose ticked and unticked
rows do not agree on where the words start. No desktop menu has ever done it that way. It
follows that a tick which is **off** draws nothing and still holds its place, which is one
of the tests.

A menu with no marks at all is not indented at all, which is why no picture already in this
repository moved.

## The width was a constant, and constants overlap

`WIDTH` was 220, flat, for every menu. That is fine for a list of one-word actions and
wrong the moment a row carries a mark, a label and the keys that work it: what it does then
is overlap, and overlapping is a failure no assertion about the tree catches — only a
picture shows it.

It is now a **floor**. The panel measures its rows and takes the widest, or 220, whichever
is more. A floor rather than a fit, so that every menu already drawn keeps the width it
had.

The panel measures rather than the rows, and the words are copied to it to make that
possible: by the time the panel holds its rows they are `dyn Widget`, and a widget cannot be
asked how wide it would like to be. That is [#52](https://github.com/KalybosPro/frus/issues/52),
and this is where it shows — **a row the caller drew contributes nothing to the width** and
takes whatever the words decided.

The issue said the menu "measures its own width from the labels" and asked that it keep
working. It did not measure anything. It does now.

## A press on a menu's own surface used to close the menu

`Panel::on_click` returned `None`, above a comment saying it **traps** the press all the
same — that a click on the gap between two rows cannot reach the page and dismiss the menu.

That was false, and had been for three hundred milestones. A widget with no message is not
registered as a target at all, so a press on the panel's padding fell straight through to
the window-wide region whose press dismisses the overlay. Nothing had noticed because every
row in an open menu had a message: there was no gap big enough to hit. A row that says it is
unavailable is the first one there has ever been, and the first press on it closed the menu.

So `Widget::opaque` is new: a press that lands on this widget and on **nothing inside it**
stops here. It is registered **before** the widget's children, so everything inside still
wins and only the gaps between them are closed. It is not `AbsorbPointer`, which discards
the subtree's targets wholesale; here the subtree keeps every one of them.

Only the menu's panel claims it. A dropdown's list and a tooltip have the same shape and are
left alone deliberately: this milestone is not the place to decide what a press on each of
those means.

### And an identity two different regions were sharing

Making the panel a target then broke the outside click, which is how the second bug came
out. `Ui::hit` returns the topmost target's identity and `Ui::msg_for` resolves that
identity back to a message — a round trip that only works if identities are unique. An
anchored overlay's dismissal barrier was registered under the **overlay root's** identity,
so as soon as the root registered a target of its own there were two different regions
under one name, and the round trip returned the wrong one.

The barrier now derives its own (`WidgetId::barrier`). It is not a widget; it should never
have borrowed one's name.

Both halves are pinned by the same two tests, and each was checked by taking it back out:
without `Panel::opaque`, and equally with `WidgetId::barrier` collapsed to the identity it
used to borrow, the press on the menu's own surface and the press on an unavailable row
both close the menu — and nothing else in 1 356 moves.

## What a caller's own row gets

It is laid out in exactly the room a label would have had — the leading column and the
shortcut column are kept clear with padding rather than with empty siblings — and unlike a
row of words it **grows**: a row of words is a tap target tall, a row of anything else is at
least that and as much more as it needs. A two-line item is half of why the form exists, and
a fixed height would cut it in two.

Holding it needed one small thing. Every builder on `PopupMenuButton` rebuilds the panel, so
that the order they are written in does not matter — the trap milestone 458 found in
`BottomSheet`, where anything said after `body` is silently dropped. Rebuilding means the
rows are built more than once, and `Box<dyn Widget<Msg>>` cannot be cloned. So a caller's row
is held by a shared pointer behind a transparent wrapper, which is four lines of the
`forward_transparent!` macro and keeps the property the test in this file already pins.

## What a reader hears

A row that is on or off is announced as a **checkbox**, with its state. There is no
`menuitemcheckbox` in this framework's vocabulary and inventing a role for one widget is
worse than using the one that carries the thing that matters.

The keys are announced after the label, because a shortcut is the only thing on a row that
someone who cannot see it has no other way to learn.

A rule is not a row: it is sixteen tall rather than a tap target, it takes no focus, and it
answers nothing. A separator that could be focused is a separator that swallows a keystroke.

## Not here

- **A keyboard shortcut that actually works.** `shortcut` draws and announces the keys; it
  does not bind them. The spelling differs by platform (`Ctrl+X`, `⌘X`) and nothing here
  parses it — binding belongs with whoever gives the framework a shortcut table, not with
  the widget that displays one.
- **A submenu.** That is [#33](https://github.com/KalybosPro/frus/issues/33), and it is a
  second overlay anchored to a row rather than anything about what a row can hold.
- **RTL column placement.** The mark is drawn at the row's physical left and the keys at
  its physical right, because that is what every other widget in the crate does with its
  interior — the layout mirrors boxes, not what a widget paints inside one. Making the menu
  the exception would leave it disagreeing with the list tiles beside it.
- **`DropdownButton`'s choices**, which are the same complaint about a different widget and
  are [#35](https://github.com/KalybosPro/frus/issues/35).

## The picture

`popup_menu_rich`: three rows with pictures and keys, one of them unavailable, two rules,
two rows that are on and off, and a row of two lines the caller drew. The three middle
columns are the point — that the marks line up, that the row with nothing to put there keeps
the room, and that the keys end where each other's do.
