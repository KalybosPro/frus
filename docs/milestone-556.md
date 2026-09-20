# Milestone 556 — The demo is made of components

## Objective

`frus-demo` is the largest application in the repository — twelve screens, a task list, a
widget gallery — and it was the one written the old way: one struct of eighty fields, a
`Msg` enum of a hundred and thirty variants, a `reduce` function and a `view`. Milestones 551
to 555 gave the framework the way an application is now written; this step writes the demo
that way, so that the example people read to learn the framework is written in it.

## What changed

- **Every screen is a widget with the state it keeps.** The sign-up wizard, the editable grid,
  the chart dashboard, the data table, the Kanban board, the log, the tour, the sheet, the
  settings and the licences are `StatefulWidget`s whose `State` holds exactly what only they
  care about — the wizard's step, the grid's sort, the value of each control — with the rules
  that change it as plain methods (`WizardState::submit`, `DataState::delete_checked`,
  `BoardState::move_label`). The task's own screen is a `StatelessWidget`. The home screen is a
  `StatefulWidget` that keeps its filter, which overlay is open and which section is on show.
- **What more than one screen needs is one object, `Demo`.** The task list, how the demo is
  dressed (light or dark, seed, direction, language, zoom) and the notification queue. It is plain
  Rust with no widget in it, handed to the screens that ask for it as part of their
  configuration, and every method that changes it asks for a rebuild.
- **The router says where the reader is.** Home is `/`; the other screens are its sub-routes
  (`/settings`, `/wizard`, `/task/:id`, …), so going back from any of them lands on the list. The
  hand-written screen stack, the transition controller, the back-gesture record and the
  `nav_from` bookkeeping — four fields and a hundred lines — are the router's.
- **Text is held by `TextEditingController`s.** The home screen's draft and the wizard's four
  fields are controllers: the field is driven by one, and the buttons read it.
- **What is not a widget goes through `host`.** The light switch and the language menu set the
  window's themes, mode and locale; the grid's "Next error" and the wizard's summary ask for a
  focus; the log's "Top" and the sheet's "Raise it" ask for a scroll and a sheet height by
  name; saving and loading run on another thread and use their answer; a notification
  schedules its own exit.
- **`Msg`, `reduce`, `TodoApp` and the `Route` enum are gone.** About 1,100 lines of them.

## What the demo asked of the framework

Writing a real application against it found what an example that small could not:

- **`GoRouterState::entering()`**: how far into view a page is. The task screen moves its own
  words as it arrives, and the router already had the number; it now hands it to the page it
  builds (the arriving page as it is, the leaving one the other way).
- **`Driver::texts`, `Driver::tap_text` and `Driver::frame_parts`**, in the shell's test driver:
  the words a frame painted and where, a tap on a word, and the interface and the tree of the
  last frame — what a test needs to press a control by what it says and to ask the registries
  the questions the shell asks.
- **`frus_test::Stage` builds components** the way the shell does — the states marked as
  reached, the components built before anything asks for their children — so a tree with
  components in it renders in a golden and in the pictures the README shows.
- **`Msg = Callback` is the default for five more widgets**: `BarChart`, `LineChart`,
  `DataTable`, `Kanban` and `ReorderableList` still defaulted to `()`, which is not a message
  an application made of components has; `CellFn` had no default at all.
- **`cx.handler(..)` answers with a `Callback`**, not with nothing. A handler whose closure
  returns nothing could stand for a message of any type, and where nothing else in an
  expression says which, the compiler chose `()` and reported a widget that does not implement
  `Widget<()>`. Naming the callback is what says it, so the demo does, everywhere a closure
  would otherwise be ambiguous (`on(..)`, `on_value(..)`, `on_values(..)`).

## Decisions

**The shared object is not a store with a subscription.** A change to it asks for a rebuild of
the whole tree, which is what any change does in this framework; a screen does not subscribe to
the part it reads. The alternative — a `ChangeNotifier` per field and a `watch` hook — would be
more code for a saving nobody has measured, and would put back the question the rebuild model
answers by having none: who needs to hear about this?

**Live reload keeps less.** The snapshot the development loop restores across a recompilation
held the filter, the section on show, the typed draft and the screen stack as well as the tasks
and the theme; the first three are now the state of a screen, and the stack is the router's.
What survives is what the shared object holds: the tasks and how the demo is dressed.

**Routes are addresses, not an enum.** A task's own screen is `/task/:id`, and its page reads
`:id` from the location. The old `Route::Task(u64)` carried the same number, but the router
could not have been asked for the location of a screen the application had no name for.

**The direction of the layout is told to the host from a build.** A device set to Arabic mirrors
the layout before anything is picked, and the device's language is only known while a tree is
being built (`locale::of()` reads the surface description). The home screen tells the host when
that answer differs from what the host was last told, which is nearly never and costs a
comparison.

## Findings on the way

- **The journal's list was taller than the safe area.** It sized itself to the window less a
  hand-counted 196 pixels and forgot the system's bars, so on a phone with a status bar and a
  navigation bar its last rows sat under the bar. The old test that was written to catch
  exactly this never did: it pushed the screen and did not wait for the slide, so it checked a
  page a whole width off the glass. The new one lets the transition finish, and the list is
  sized to what is left.
- **A scroll region inside another one is clipped, not overflowing.** The bars test says so:
  what the outer region holds beyond its viewport is emitted and cut away, and the region
  itself is what is checked against the bars.

## Verification

- Seventy-four tests of the demo: the rules of `Demo` (adding, moving under a filter, the language
  cycle and the French table, the notification queue, a live reload's round trip), the state of
  every screen as plain Rust, and the whole application through the router at a stated size —
  the overflow test on every screen at a phone's width and a desktop's, the safe area, the
  reorder registries, the sheet's gestures and its raise by name, the wizard tapped through
  the shell, the board's strip carried through the shell, the scroll kept through a pop, and
  the last frame of a push.
- The tests that were about a widget, not the demo, were kept as they were: nothing in the
  framework's own suite changed but the chart tests that named `()` where the default now says `Callback`.

## Left out

`frus-fetch-example` and `frus-transforms` remain on the typed-message model, which is what
they are there to show. The wizard's typed flow (a password of eight characters and a
confirmation that matches) is tested on its pure form and its state, not through the shell:
the test driver has no keyboard.

The demo does not build for the web target: `frus_shell::main!` expands to code that names
`wasm_bindgen`, which only the web example depends on. That was so before this step and is left
as it was.
