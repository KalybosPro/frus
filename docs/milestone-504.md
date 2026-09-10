# Milestone 504 — A navigation bar behind the clock

Found while photographing milestone 503's system bars: at the head of the demo's task
screen, `NavigationBar` drew its title and its back button **under the status bar**, the
clock and the battery on top of them. It is older than 503 — it had always done it.

## Why only this bar

A [`Scaffold`] tells its app-bar slot what the status bar takes: it hands the slot a
description of the surface with the top intrusion in it (milestone 417), and the bar is
meant to consume it. `AppBar` does — it wraps its toolbar in a safe area that leaves the
bottom edge alone, so its colour runs behind the status bar while its title sits below.
`NavigationBar` never read the description at all. Its height was a constant, its padding
was a constant, and it centred its title in whatever box it was handed.

## The bar clears what is above it and beside it

The bar now reads the ambient top and side intrusions **when it is asked for its size and
when it paints**, not when it is built — the same moment `SafeArea` and `DrawerHeader`
read them, and the one that lets a scaffold, a scope or a safe area above it change the
answer.

The intrusion is **added to the bar, not taken out of it**. The bar grows by the status
bar's height and its padding by the same amount, so:

- the background runs up behind the status bar, as an app bar's does;
- the title and the back button keep their full 56 pixels below it, and the button is
  still a 48-pixel target;
- the hairline stays the bar's last pixel.

That is how the reference sizes its persistent navigation bar (its height plus the top
padding) and how a drawer header is sized here already. The side intrusions — a cutout in
landscape — are added to the padding the same way. The bottom is not the bar's: something
is under it by definition.

## Which made `SafeArea::new` the real problem

Most of the demo's screens do not use a scaffold. They are a column with the bar at its
head, inside a `SafeArea::new`, inside a coloured container — the background under the
bars, the content clear of them. With the bar reading the surface, every one of them
would have cleared the status bar **twice**: once by the safe area's padding, once by the
bar.

Because `SafeArea::new` **padded without consuming**. Only `SafeArea::build` told its
subtree that the edges were gone, and `new`'s documentation said why: its child is already
built by the time the widget exists, so it could not affect what the child saw. That was
true when the description was read at construction. It stopped being true at milestone
417, when the walk began resolving the surface on the way down, and `MediaScope` began
using exactly that to tell an already-built slot something new.

So `SafeArea::new` does the same: it wraps its child in a scope that removes the edges it
pads, which is what the reference's safe area does with `MediaQuery.removePadding`. Two
details:

- **The scope wraps the child, not the safe area.** The walk installs a node's surface
  before it asks that node for its style, so a safe area that consumed its own edges would
  read its own padding back as zero.
- **The builders still work after the wrap.** `edges` and `avoid_keyboard` change what is
  consumed, and are called on the finished widget; the scope reads a cell they share. Only
  the selected edges are consumed, so one left free is still there for a descendant.

`SafeArea::build` keeps handing the consumed description to its closure, and its subtree is
now told the same thing again during the walk, so the two cannot disagree.

## On the device

Huawei STK-L21, Android 10:

- **task screen** (the bar in a scaffold's app-bar slot) — the title and the back arrow
  below the status bar, the bar's colour running behind the clock;
- **guided tour** (the bar at the head of a column in a `SafeArea::new`) — the bar at
  exactly the same height, one status bar down and not two;
- the home screen, whose `AppBar` was right already, unchanged.

## Found on the way, and not fixed here

On the task screen a **"Back" tooltip** stood over the status bar after the screen
opened, with nothing pressed. A tooltip shows while its anchor is hovered; the shell sets
the hover from a finger as from a mouse and never clears it when the finger lifts, and the
hover is a widget id — which is a position in the tree. The avatar tapped on the home
screen and the back button on the next one share it. Its own fix, in the next milestone.

## Verification

- Three tests on the bar: at the head of a screen under a status bar — the background from
  the top, the bar grown by the intrusion, the title centred below it, the button answering
  below the status bar and not behind it; in a safe area, cleared once; in a scaffold's
  app-bar slot, cleared as at the head of a column.
- Two tests on the safe area: nested `new`s avoiding the notch once, and an edge left free
  reaching a descendant.
- The workspace's tests and the goldens unchanged; `frus-shell` checked for Android; the
  APK built, installed and looked at.
- Six mutations, each failing the test meant for it: the bar ignoring the surface, at the
  head of a screen and in a scaffold slot; the title painted as if nothing were taken;
  `SafeArea::new` not consuming, for nested safe areas and for a bar inside one; `edges`
  not reaching the scope.

[`Scaffold`]: ../crates/frus-widgets/src/scaffold.rs
