# Milestone 660 — Hearing the focus move

## Objective

frus had focus and Tab order (milestone 605), but nothing could hear the focus move. The
reference's `Focus` widget takes `onFocusChange`, called with `true` when the focus arrives in
its subtree and `false` when it leaves. Its `TextField` takes `onTapOutside`, called for a
press outside the field while it is focused. Forms use them to validate a field when the user
moves on, save a draft, or close a suggestion list.

## What was done

- **`Widget::on_focus_change(focused)`**, a hook every widget can answer. It is forwarded by
  the boxed widget, the transparent wrappers, `Responsive` and the five focus wrappers.
- **`focus_changes(root, before, after)`** decides who hears a move of the focus:
  - every widget on the path from the root to the old focus that is not on the path to the
    new one hears `false`, deepest first;
  - then every widget on the new path that is not on the old one hears `true`, deepest
    first.

  The widgets around both, the paths' shared start, hear nothing: a move between two fields
  inside a group says nothing to the group. A widget no longer in the tree is not heard from.
- **The shell reports** once a frame, before the frame is built, whenever the focus differs
  from what it last reported. Every way the focus moves (a press, Tab, autofocus, a widget
  leaving) is covered by the one check, rather than by the nine places that set it.
- **`Focus::on_focus_change(f)`**, the reference's `onFocusChange`.
- **`Widget::on_tap_outside()`** and **`TextField::on_tap_outside(msg)`**: a press that does
  not land on the focused widget tells it so, before the press moves the focus.

## Left

- **`onEditingComplete`**: a key answers with one message, and Enter already sends
  `on_submit`.
- **The platform's default for a press outside.** frus moves the focus to whatever a press
  lands on, clearing it on a press on nothing, everywhere. The reference keeps a field focused
  after a mouse click outside it on the desktop and unfocuses only for a touch on a phone.

## Tests

- `frus-widgets`:
  - a group hears the focus arrive and leave, nothing for a move between its own fields,
    nothing for no move, and only its arrival when the old focus is gone from the tree;
  - the one that lost the focus hears first;
  - nested groups hear it deepest first, leaving and entering.
- `frus-shell`, through the windowless driver: a press on the field brings the focus into the
  group; a second press on it says nothing; a press on the other field tells the field it
  was outside and then the group that the focus left; a field without `on_tap_outside` says
  nothing.

Mutation testing: 9 mutants. 8 were killed (one only after the nested groups' test was
added) and 1 is equivalent. They cover:
- the shared start of the two paths;
- the order among the widgets that lose the focus, and between losing and gaining;
- the focus wrappers forwarding the hook, which the first version did not;
- the shell reporting before the build;
- the press counted inside the focused widget;
- the focus remembered once reported.

The equivalent mutant is the shell's own check that nothing moved: without it,
`focus_changes` returns nothing anyway, but walks the tree every frame to find that out. An
early return in `focus_changes` itself, equivalent in the same way, was removed.
