# Milestone 669 — frus can be hot-patched

## Objective

`frus run`'s `r` restarts the application and hands its state back (milestone 663). The reference
does better: its hot reload replaces the code of the running application, and nothing restarts.
Milestone 663 tried the same with subsecond, the engine of Dioxus's CLI (`dx serve --hot-patch`),
and stopped on a crash it attributed to frus's thread-locals. This milestone looks again, and
finds the crash elsewhere.

## What was found

- **The crash came from the patch engine.** A patch is a library whose calls to the original
  program go through small stubs. On Windows x64, `dx` writes them as `movabs rax, <address>;
  jmp rax`. A function whose stack frame is larger than a page calls `__chkstk` first, with the
  size of the frame in `rax`, and in a patch `__chkstk` is one of those stubs. By the time it
  runs, `rax` holds an address, and it probes a frame of billions of bytes: the stack overflows on
  entry, before the function's first statement.
  - It explains everything 663 saw: small functions patched fine, and a component's `build`,
    with its widget builders, always crashed. Debug builds make frames large.
  - Located by narrowing: markers through the frame stopped inside the counter page's `build`. A
    small `build` that called an `#[inline(never)]` function with the original body printed its
    marker, and the larger function crashed before its first statement.
  - The fix is to jump without touching a register: `jmp [rip+0]` followed by the address,
    which the engine already uses on the other x86_64 systems. With it, the same application
    hot-patches.
  - Reported upstream with the fix: DioxusLabs/dioxus#5966, pull request #5967.
- **Verified** on Windows 11 with a `dx` built with the fix, on a copy of the counter example
  counting by itself on a timer:
  - its label was changed in the source, and the running window showed the new text;
  - the count went on from 9 to 25 across the patch, rather than starting again at 0;
  - the timer kept running.

## What frus needed

Behind the `hot-reload` feature (`frus`, `frus-shell`, `frus-widgets`), which only development
builds turn on:

- **The build methods are looked up by the method itself.** `<W as StatelessWidget>::build` and
  `<S as State>::build` are called through `subsecond::HotFn`. The table answers for the
  application's functions; a closure frus makes around them is frus's code, which no patch
  touches.
- **The application's code is the program's.** subsecond patches the program's own crate, and a
  frus application keeps its code in its library. With the feature, `main!` also writes a `main`,
  so `src/lib.rs` can be compiled as a binary (`[[bin]] path = "src/lib.rs"`) that is the
  application. That is decided inside frus-shell (`__hot_main!`, one definition with the feature
  and an empty one without), so an application's crate never sees an unknown condition, and one
  that has a `main` of its own is left alone.
- **The shell connects to the patch server** at start-up, and builds the interface again when a
  patch arrives (`frus-shell/src/hot.rs`).
- **Every thread-local is reached only through functions compiled once, in frus**: not generic,
  not inlined. A generic function is compiled into the application, and a patch carries its own
  copy of every thread-local such a function touches, apart from the one the rest of frus holds.
  This was not the crash, but it is what keeps a patched build and the framework on the same
  state.
  - Seven accesses were in generic code (`LazyChild::expand`, `use_interval`, `Callback::new`,
    the two `scope`s, `title_bar::on_change`, `web_dev::keep`); they go through small helpers now.
  - Every accessor is `#[inline(never)]`: an optimised build may inline a small function into
    another crate.
  - `frus-widgets/tests/thread_locals.rs` reads the sources of frus-core, frus-widgets and
    frus-shell, and fails on any thread-local touched by a generic or inlinable function.

## Alternatives weighed

- **Waiting for the upstream fix.** The fix is a few bytes and is reported; frus can use a `dx`
  built with it until a release has it (milestone 670).
- **A patch engine of frus's own.** The subsecond crates do the hard part (the jump table, the
  protocol); only the stubs were wrong. Not worth writing again.

## Left

- `frus run --hot`: installing the fixed `dx`, compiling the library as the program without
  changing the application's manifest, and the keys (milestone 670).
- Android and the web, which subsecond supports and which were not tried here.
- What a patch cannot do: change the shape of a state that is alive (a field added). That still
  takes a restart, `R`.
