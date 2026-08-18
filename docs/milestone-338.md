# Milestone 338 — Shortcuts, actions, and the key that means something

The largest of the three subsystems milestone 336 counted. Six widgets, and the sixth focus
widget that was waiting on them.

## Two steps, not one

The reference splits this deliberately, and the split is the whole design:

- `Shortcuts` maps a **keystroke to an intent** — *Ctrl+S means «save»*. It knows nothing
  about what saving does.
- `Actions` maps an **intent to a message** — *«save» means `Msg::Save`*. It knows nothing
  about which key got here.

The indirection buys independence: a dialog rebinds the keys without touching the handlers,
and a subtree answers «save» differently from the page around it — with the innermost
answer winning, the same rule as focus and resolved the same way. `CallbackShortcuts` binds
a keystroke straight to a message for when naming an intent is ceremony, and
`ActionListener` watches an intent without answering it, so an undo stack or a status line
can know without becoming the thing that performs it.

## A key vocabulary that is not the editing one

`Key` here has always been what text editing needs: `Text(String)`, `Backspace`,
`Left { shift, word }`. It cannot say *Ctrl+S*, and stretching it to would have made the
editing path worse.

So `KeyStroke` is a second, general vocabulary — a `ShortcutKey` plus four modifiers — and
it is deliberately **one type for both sides**: the pattern a shortcut is bound to and the
description of what was pressed. They are only ever compared to each other, and a second
type would be the first with the fields renamed.

Two decisions inside it:

- **Letters match without case.** `Ctrl+S` and `Ctrl+s` are the same shortcut. A caller
  should not have to think about Caps Lock, and neither should a user.
- **Shift alone is not a command.** `Shift+A` is a capital A. That single line is what
  makes the typing rule below expressible.

## Typing beats a bare letter

A stroke with no Ctrl, Alt or Meta goes to a focused field first. Without that rule, a
`Shortcuts` binding on the letter `a` would make every field under it impossible to type
in — the reference reaches the same place through text-editing shortcuts taking priority in
the focused scope.

And the shell's own keys — the system back, F12 — are checked *before* any application
binding, so no binding can take the back gesture or the inspector away.

## Which scope answers, and how it is found without an ancestor test

A binding applies when focus is inside the subtree that declared it. There is no parent
pointer here to walk, and identities are a hash chain rather than a path, so ancestry
cannot be asked directly.

It does not need to be. The walk is depth-first, so a subtree's focus stops are
**contiguous**, and a scope can simply record the range of stops it turned out to contain.
"Focus is inside this subtree" becomes "the focused stop's index is in this range" — one
comparison, exact.

The range is only known on the way *out*, which is where scopes are recorded — and that
gives the ordering for free: innermost first among any that overlap, which is precisely the
order the resolution wants. It is not sorted afterwards because it does not need to be.

## An intent nobody answers does nothing

Deliberately, and it has a test. A key bound to a meaning the current screen has no answer
for should be inert, not an error and not a panic: the binding and the handler are written
by different people at different times, which is the entire reason the indirection exists.

## `FocusableActionDetector`

The sixth focus widget, and the composition the other five could not make: focusable, with
its own shortcut and action tables, so a control answers the keyboard for itself while it
has focus. A menu item that answers Enter, a card that answers Delete, a canvas that
answers arrows.

Disabled, it is **not a focus stop at all** rather than an inert one Tab still lands on —
the same rule the rest of this framework's disabled controls follow.

## Not here, and why: `ShortcutRegistrar`

The reference has a registry a deep widget registers into at runtime, because its tree is
retained and a widget's build is not re-run every frame. Here the view is rebuilt from the
state each frame, so declaring a binding where it applies is the same thing with none of
the bookkeeping. That is an architectural difference, not a missing feature, and it is
written in the module's own documentation rather than left for someone to discover.

## Left

- **Filters** — `BackdropFilter`, `BackdropGroup`, `ColorFiltered`, `ImageFiltered`,
  `ShaderMask`. All GPU work, and the last subsystem.
- **`Baseline` / `IgnoreBaseline`** — taffy has baseline alignment; nothing reaches for it.
- **Repeat and release.** Only key *presses* reach shortcuts; a binding cannot ask to fire
  on release, and a held key repeats through whatever the platform does.
