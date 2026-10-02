# Milestone 613 — An app bar's actions are widgets

## Objective

The reference's app bar takes `actions` as a list of widgets: icon buttons as a rule, a
popup menu button for an overflow, anything at all. frus's `AppBar::action` took a label
and a message and made a text button, the one kind of action the bar can fold into its
overflow menu; any other widget went through `action_widget`. The common case had the
odd name, and code written against the reference's shape did not read like it.

## What was done

- **`AppBar::action(widget)`** adds any widget at the trailing end, after those already
  added. It is what `action_widget` was.
- **`AppBar::actions(iter)`** adds a list of boxed widgets in one call, as `Row::children`
  does: the reference's `actions: [...]`.
- **`AppBar::foldable_action(label, message)`** is the former `action`: a text button shown
  inline while it fits and folded into the overflow menu when the bar runs out of width.
  It is frus's own addition, kept under a name that says what sets it apart.
- `action_widget` stays one release, deprecated, forwarding to `action`.

Nothing is laid out differently. The actions remain one row, centred across the bar,
inset by `actions_padding` and dressed by `actions_icon_theme`, which is what the
reference builds from its list (`app_bar.dart:1101`). Widget actions never fold, and they
are counted before the foldable ones in the width budget, as before.

## Alternatives weighed

- **Dropping the folding** to match the reference exactly. The reference has no automatic
  overflow: an application that wants one measures and builds a popup menu itself. The
  folding is the reason the bar is called adaptive, and the demonstration's home screen
  depends on it, so it stays, beside the reference's list rather than in place of it.
- **`actions(Vec<Box<dyn Widget>>)` only**, with no single-widget method. Every other
  multi-child widget in frus offers both `child` and `children`; one-at-a-time is what a
  builder chain reads best with.
- **Keeping `action(label, message)`** and adding `actions` beside it. The single-widget
  method would then still be `action_widget`, and the name the reference uses would still
  mean the thing the reference does not have.

## Migration

- `.action("Save", Msg::Save)` → `.foldable_action("Save", Msg::Save)`, or, as the
  reference would write it, `.action(IconButton::new(Icons::SAVE).label("Save").on_press(Msg::Save))`.
- `.action_widget(w)` → `.action(w)`.

## Tests

- `actions_are_widgets_kept_in_order`: actions added one by one and as a list keep their
  order, and none folds at 200 px.
- `a_widget_action_emits_its_own_message`: a widget action's message reaches the
  application untouched.
- The existing folding tests, renamed to `foldable_action`, pass unchanged, and so does
  the `app_bar` golden.

Mutation testing: two mutants of the new methods, both killed — `actions` keeping only
its first widget (which survived a first version of the order test, whose list ended in
an icon button the test could not read; the list is now two texts), and `action`
inserting at the front.
