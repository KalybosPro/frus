# Milestone 552 — Listenables and the text controller

## Objective

A text field draws the text it is given, and in this framework the text lived wherever the
application put it — a `String` in the state, written back by an `on_input` message. Fine for
an application with one state struct; awkward for a component that wants a field of its own
and to read what was typed when a button is pressed, or to clear it from code.

A **controller** is that place, made once and handed to the field: what the person types lands
in it, what the program writes appears in the field.

## What was added

- **`ChangeNotifier` and `ValueNotifier<T>`** — something that changes and tells whoever is
  listening. `add_listener` returns a `Subscription` that stops the notifications when it is
  dropped; `forget()` keeps listening for the notifier's lifetime. Every notification also
  asks the shell for a rebuild, so a tree that *reads* a notifier while it is built needs no
  listener to show a change made from a timer or another component.
- **`Listenable`** — the trait both implement, so a router (or anything else) can watch either.
- **`TextEditingController`** — `new`, `with_text`, `text`, `set_text`, `clear`, `is_empty`,
  `len`, and it is a `Listenable`. `TextField::controller(&c)` and `SearchBar::controller(&c)`
  drive a field from it. `BuildContext::use_text_controller("")` keeps one for the life of a
  component.

## Decisions

**A controller is a `ValueNotifier<String>`, not a second kind of thing.** It holds the text
and nothing else. A caret and a selection are not in it: they live in the runtime beside the
field, keyed by its identity, and a controller that claimed to own them would have had to
reach into that store from outside a frame. That is a real difference from a controller that
owns its selection, and the honest name for what is missing is *selection control*, not a
half of one. It can come later without changing this.

**`controller` sets the field's text and its `on_input`, and says so.** The alternatives were
to compose with an `on_input` the caller had already set, or to refuse the combination. Either
hides a rule in an order of calls. Replacing is the rule with the fewest cases, and listening
to the controller is how to do more when the text changes.

**The write happens when the message is delivered.** `on_input` is a value handler, and a
value handler's work is deferred to delivery (milestone 551): a field may ask for its message
more than once for one keystroke, and the controller must change once.

**Setting the text a controller already holds notifies nobody.** `ValueNotifier::set` compares,
as any value that means "changed" must; `update` is there for a value that is changed in place.

## Verification

- 6 tests on the notifiers: order of notification, a dropped and a forgotten subscription, a
  listener dropping another during a notification, changes-only for values, and that a change
  asks for a rebuild.
- 5 tests on the controller: text operations, that a clone is the same text, that a listener
  hears exactly the changes, that a field shows its controller and writes back to it once the
  message is delivered (and not before), and that a hook returns the same controller on every
  build.
- Doctests for both entry points.

## Left out

Selection and composing-range control, and a controller for a field that is not a `TextField`
or `SearchBar` (a `SearchView`, an `Autocomplete`).
