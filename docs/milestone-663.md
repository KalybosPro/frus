# Milestone 663 — Reload: a restart that keeps the state

## Objective

Milestone 662 gave `frus run` a restart that brings the new build back on the same page, with
the window in the same place. What the developer was doing on that page was still lost: a
counter back at zero, a form emptied, a step of a wizard started over. The reference's tool
answers this with **hot reload**, `r`: the code changes while the application runs, and its
state stays where it was.

This milestone set out to do the same, found what stands in the way, and ships what can be
shipped now: **`r` reloads the application with the state of every component that keeps
one**.

## What was done

- **`State::save` and `State::restore`.** Two optional methods on a component's state:
  - `save` returns what to keep, as text: a number, a few fields, a JSON document;
  - `restore` takes it back in the new build, right after `create_state` and before
    `init_state`, and only for the component at the same place in the tree, of the same type.
  - Both are optional and default to keeping nothing, so nothing changes for a state that does
    not implement them, and nothing is ever saved outside `frus run`.
- **The state store keeps them** (`StateStore::snapshot`, `StateStore::restore_from`). The
  snapshot is a line per state that saved something: its place, its type and what it saved,
  escaped. A restored value is given to the first component that mounts at its place with its
  type, and only once.
- **`r` and `R` in `frus run`**:
  - `r` **reloads**: the application is built again and comes back where it was, on the same
    page with the window in the same place, and with every kept state;
  - `R` **restarts**: the same, but every state starts afresh.
- **How the states cross the restart:**
  - **desktop**: the channel to `frus run` now carries requests too. Before stopping the
    application, `frus run` sends `save <file>`, the application writes its snapshot at its
    next frame and answers `saved`, and the new build reads it from `FRUS_RESTORE_STATES`. With
    no answer within three seconds, the reload goes on without states;
  - **Android**: nothing reaches the application, since it reports through its log. So
    `frus run` sends it to the background (the HOME key) first. Going there under `frus run`,
    the shell writes the snapshot into the application's own storage, which `adb install -r`
    leaves alone, and the next launch reads it back and deletes it;
  - **web**: unchanged, the page reloads itself; states are not kept there yet.
- **The counter example keeps its count and its step**: change its code, press `r`, and it is
  still where it was.

## Verified

- `cargo test -p frus-widgets component::`: a state saved in one runtime comes back in a fresh
  one before its `init_state`, a state that saves nothing is left out, and a value with tabs,
  line breaks and backslashes survives the snapshot's escaping.
- `cargo test -p frus-cli`: the request is sent to the application and its answer waited for.
- On Windows, a copy of the counter example counting by itself: 6 on screen, then `r` showed
  8 (it had carried on from where it was), then `R` showed 1 (afresh).

## Hot reload proper: what was learned

A true hot reload — the running program's code patched in place, every state where it was,
without declaring what to keep — was tried with **subsecond**, the engine Dioxus's CLI (`dx serve
--hot-patch`) drives. It builds a patch from a change in well under a second, and frus received
and applied it. Three findings stop it here:

1. **subsecond patches the program's own crate only.** A frus application keeps its code in
   its library, which Android needs, and its desktop program only calls `run()`, so a patch saw
   none of the application's code: 195 entries, none of them the changed component.
   - Compiling `src/lib.rs` as a program of its own (a `[[bin]]` with that path) puts the
     application's code in the patch: 23,059 entries.
   - That `[[bin]]` makes cargo warn, on every build, that the file belongs to two targets.
2. **A patch is looked up by the function it replaces.** Wrapping a component's `build` in a
   closure made inside frus-widgets never reaches the patched version, because the closure is
   compiled with frus-widgets' code, which no patch touches. Looking up the method itself
   through a function pointer is what subsecond's table answers for.
3. **The patched code crashed the application with an infinite recursion** as soon as it ran,
   even with a 64 MB stack, and outside the components' own `build` calls. subsecond documents
   that thread-locals are reset in patched code. frus keeps its build state in thread-locals (the
   hooks, the rebuild flags, the depth of a chain of components, the ambient theme and media
   query), and almost every widget is generic, so its code is compiled into the application and
   patched with it. A patched build then works with fresh copies of state the rest of frus holds
   elsewhere.

Making frus patchable would mean reaching every thread-local of the framework only through
functions that are not generic and not inlined, so that no copy of the access is compiled into
the application. That is a refactor across frus-widgets and frus-shell, and its result is not
certain. The experiment's code is not kept; this note is its record.

## Left

- **The web keeps no state across a reload yet**: its page reloads, and the snapshot would have
  to cross that reload (session storage is the natural place).
- **Hooks** (`use_state`) are not kept, only `State`s that implement `save`.
- **Android**: the reload was verified on the desktop; the phone was not connected when it was
  finished.
- **True hot reload**: see above.
