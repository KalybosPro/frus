# Milestone 394 — Four properties the shell did not have

Milestone 393 took the screen's size out of the `Scaffold`'s constructor. This one goes
through what the reference's `Scaffold` carries that ours did not, and adds the four that
mean something for a shell whose state the application owns.

## `primary`

```dart
final double topPadding = widget.primary ? MediaQuery.paddingOf(context).top : 0.0;
_appBarMaxHeight = AppBar.preferredHeightFor(context, widget.appBar!.preferredSize) + topPadding;
```
— `scaffold.dart:3049`

Whether the app bar's height is the bar's own or the bar's **plus the status bar**. It is
`true` by default because a shell is usually the screen; `false` is for one nested in a
page, or beside another, where something else is above it and adding the notch would inset
for it twice.

Ours added `insets.top` to the bar unconditionally. `Scaffold::primary(false)` stops it,
and the test asserts the bar moves by *exactly* the intrusion — not merely that it moves.

## The footer's rule

```dart
decoration: widget.persistentFooterDecoration ??
    BoxDecoration(border: Border(top: Divider.createBorderSide(context, width: 1.0))),
child: SafeArea(top: false, child: …)
```
— `scaffold.dart:3134`

Two things at once, and we had neither.

**A line along the top, by default.** The reference's footer is ruled off from the body
unless the caller replaces the decoration; ours simply sat there. `persistent_footer_divider`
is on by default and drawn as a `Divider`, so its colour and thickness follow the theme like
every other line in the framework rather than being a number in the shell's code.

**The decoration outside, the safe area inside.** The border and the background belong to
the outer container; the `SafeArea` is its child. So the rule and the background run the
full width of the screen and the *content* is what keeps clear of the side intrusions — a
border inset by the notch would be a rule that stops short of the edge it rules off. Ours
inset both, and getting that right was the point of restructuring rather than just adding a
line.

`persistent_footer_color` is the other half of the decoration.

## The drawers' scrim

`drawer_scrim_color` and `drawer_barrier_dismissible` — the reference's `drawerScrimColor`
and `drawerBarrierDismissible`, both applying to the leading drawer and the trailing one, as
its single pair does.

`Drawer` already had a `scrim_color`; the shell had no way to reach it. The barrier is new:
a drawer holding something that has to be answered wants the way out to be a control inside
the panel, and the screen behind unreachable. The test clicks the scrim and asserts the
message that comes back — `Some(Drawer)` one way, `None` the other.

## The half of `primary` we cannot have, and why

The reference splits this job across two widgets, and it took reading both to see it.

```dart
// The padding applies to the toolbar and tabbar, not the flexible space.
if (widget.primary) {
  appBar = SafeArea(bottom: false, child: appBar);
}
```
— `app_bar.dart:1190`

`AppBar.primary` is where the padding actually happens: the bar wraps **itself** in a
`SafeArea`. `Scaffold.primary` only makes the slot tall enough for it to. Two halves of one
thing, both `true` by default, both set `false` for a nested shell.

That split works there because the shell builds its slots **lazily**, under a `MediaQuery`
it controls: it hands the bar a description and the bar decides what to do with it. Ours
are eager — `Scaffold::app_bar(widget)` takes a widget that has already been built — so the
shell cannot hand its slot a description at all. It insets the slot from outside instead,
which is why our one `Scaffold::primary` does the whole job.

So **`AppBar::primary` is deliberately absent**. Adding it with the reference's default of
`true` would pad the top twice for every bar inside a shell; adding it defaulting to `false`
would be the same name meaning the opposite thing. Neither is worth having.

What would earn it is a **builder-based slot** — `app_bar_with(|mq| …)`, the shape
`SafeArea::build` already uses — so the shell can scope its slot the way the reference does
and hand the padding decision back to the bar. That is a milestone about how slots are
filled, not about the app bar, and it is on the roadmap as such.

An `AppBar` used **outside** a shell still draws under the status bar. That is the gap this
leaves, and it is the same gap the builder-based slot closes.

## What was deliberately not added

Three of the reference's properties **cannot mean anything here**, and saying so is worth
more than a no-op builder:

- **`onDrawerChanged` / `onEndDrawerChanged`.** The reference's `Scaffold` owns whether its
  drawer is open, so it has to tell the application when that changes. Ours is controlled —
  the application holds the flag and passes it in. It already knows.
- **`restorationId`.** For a retained tree restored across a process death. Ours is rebuilt
  from the application's own state every frame, which is the state that would be restored.
- **`floatingActionButtonAnimator`.** It animates the button *between* locations. Ours does
  not move between locations yet, so there is nothing to animate; the animator arrives with
  the movement, not before it.

And three wait on a capability rather than on a decision: **`drawerEdgeDragWidth`**,
**`drawerEnableOpenDragGesture`** and **`endDrawerEnableOpenDragGesture`** all configure
opening a drawer by dragging from the screen's edge, which we do not do at all. They belong
to the milestone that adds it — a switch for a gesture that does not exist would be a
property that reads as supported and is not.

## Still open on the shell

`drawerDragStartBehavior` (with the drag gesture), `bottomSheetScrimBuilder`, and the
`AppBar` and `Navigator` property lists recorded at the end of milestone 393.
