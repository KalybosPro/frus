# Milestone 555 — What a component asks of the application around it

## Objective

The next step is to move `frus-demo`, the largest application in the repository, onto the
component model. Reading what it does turned up five things an application of that size does
and a component could not yet say:

- change the **theme**, the **language** or the **zoom** the whole window is dressed in;
- ask for a **focus** or a **scroll** — "put the caret in the first invalid field", "back to the
  top of the list" — which are events, not state;
- run something on **another thread** and do something with what it made;
- say that an open menu or dialog should take the **back gesture** before the router does;
- **start up** — register data once, before the first frame — and **survive a live reload**.

A component that has to wait for the next release of the framework to do any of these is not
yet a way to write an application. This step adds them.

## What changed

- **`frus_widgets::host`.** `host::app()` is a handle on what the application is dressed in:
  `title`, `theme`, `dark_theme`, `theme_mode`, `density`, `locale`, `supported_locales`,
  `localizations`, each with a setter that asks for a rebuild. Beside it, the effects that are
  not widgets: `host::focus(key)`, `host::scroll_to(key, to)`, `host::sheet_to(key, to)`,
  `host::spawn(work, then)` and `host::after(delay, then)`. They work from a handler and from a
  build alike, because they take no context.
- **`FrusApp` reads its settings from the host.** `.title`, `.theme`, `.dark_theme`,
  `.theme_mode`, `.density` now write there, so the builders and a component change the same
  thing. New builders: `.supported_locales`, `.localizations`, `.on_start` and `.persist`.
  A new application starts from the defaults rather than from whatever the last one left.
- **`Application::effects`.** A defaulted hook the shell calls after every message and at the
  top of every frame; `FrusApp` answers it with the effects components asked for, turned into
  the `Command`s the shell already runs. Nothing about how an effect runs is new.
- **`BuildContext::block_back`.** `cx.block_back(self.menu_open)`: while any component of the
  latest build says so, `FrusApp::can_go_back` is false.
- **`handler`.** `cx.handler(|state, text: String| state.name = text)` is the handler for a
  widget that reports a value, as `cx.callback` is for one that does not.

## Decisions

**Free functions, not methods on the context.** A handler is a closure that outlives the build,
and a `BuildContext` is borrowed for the build only, so a method there could never be called
from the one place effects are usually asked for. `host::focus("name")` can.

**The settings are per thread.** The shell, the widgets and the handlers all run on the
interface's thread, and a test gets a thread of its own, so a thread-local is the smallest
thing that is shared by exactly those. The cost is that there is one application per thread,
which is already true of the shell.

**`spawn` hands its answer over through a callback.** The work runs on another thread and its
answer has to be used on this one, where state may change. The answer is parked in a slot both
sides share, and the message that comes back is a `Callback` — the `Send` handle the model
already has — which takes it out and runs `then`. So a component-model application never needs
a message type to wait for a file.

**Effects are collected, not run.** `host::focus` only appends to a list. The shell takes the
list where it takes everything else a frame produces, so an effect asked for from a build or a
timer is run exactly as one returned from `update` is.

**`block_back` is declared per build, like `use_interval`.** It is not a slot-based hook: it
may be called conditionally, and a build that stops calling it stops blocking. The alternative,
a flag on the router, would have had every page keeping it in step with a menu it owns.

## Verification

- `host`: a setting is read back and asks for a rebuild; the mode chooses between the two
  themes and falls back to light without a dark one; effects are kept in order and taken once;
  spawned work crosses a thread and comes back as a callback that hands its answer over once.
- `FrusApp`: builders and a component write the same settings; a new application starts from
  the defaults; the language a component picks is the one the application answers; what a
  component asks of the shell becomes a command, once; `on_start` runs once; a state can be
  kept across a live reload; an open menu takes the back gesture and gives it back when shut.
- `component`: `handler` carries a value into the state; `block_back` is said afresh by each
  build.

## Left out

The test driver does not resolve scroll requests (only the real frame does), so a scroll asked
for by a component is covered up to the command it becomes, not to the offset it moves.
