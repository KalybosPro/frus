# Milestone 715 — A guide, with examples

## Objective

People trying frus 0.5 asked for documentation that shows how to use it, with examples. What
existed was a getting-started page (the smallest application, state, `async` work, routing,
shipping), a page on structuring an application, the API reference, and a gallery of 173
widget demos. Between the first page and the reference there was nothing: someone who had run
the counter and wanted a list, a form or a dialog had to read the gallery's source.

## What was done

`docs/guide/` — one page per topic, each made of examples that are complete pieces of Rust:

| Page | Examples |
| --- | --- |
| `layout.md` | rows and columns, `Expanded`/`Flexible`/`Spacer`, `Container`, the single-purpose boxes, `Stack`, `Wrap`, scrolling, a layout that follows the width |
| `style.md` | text, type roles, colour roles, an application's light and dark themes, `theme.widgets`, one widget's own style, `ButtonStyle`, `Themed`, icons and pictures |
| `input.md` | a field in a function component, a field with an error, a controller, every choice control in one state, `frus::form` checked by assertions, a sign-in form that focuses its first error |
| `lists.md` | a 10 000-row list, the data behind it, pull to refresh, swipe to remove, grids, scrolling from code |
| `navigation.md` | routes and the router's calls, a redirect, a `Scaffold` with an app bar, a FAB and a navigation bar, a dialog held in state |
| `network.md` | typed requests with `serde`, a screen's data with `use_future`, a form sent by an `async` handler |
| `animation.md` | `AnimatedContainer`, `AnimatedOpacity`, `AnimatedSwitcher`, curves, what is animated for you |
| `testing.md` | a state's methods and their test, a form and a redirect tested as functions, a widget rendered offscreen with `frus-test` |

`docs/guide/README.md` is its index; the README (both languages), `docs/README.md` and the
getting-started page point to it. The getting-started page lost two stale passages: "no
in-house CLI" (there has been one since milestone 653) and a testing section that spoke of a
pure `update` — the shape applications had before components.

## Every example is compiled

The pages are included in the `frus` crate under `#[cfg(doctest)]`, as the README already was,
so `cargo test -p frus --doc` compiles every block, and runs the ones with assertions (the
form's errors, the cart's totals, the redirect, the red pixel). The network page needs the
`net` and `json` features, and is included only when both are on — CI's "optional features"
step runs `cargo test -p frus --features "net json"`, which takes it in. `frus` gains two
dev-dependencies for its examples: `serde` and `frus-test`. Neither reaches an application.

Writing the examples against the compiler, rather than from memory, corrected eleven
mistakes before they were published: `TextField::hint` (it is `placeholder`), `Insets::all`
(`uniform`), `SizedBox::new(w, h)` (`SizedBox::empty().height(..)`), `Positioned`'s child (it
is the constructor's argument), `Image::asset` (there is none: `asset!`), `Button::icon` (it
takes a widget), `IconButton::new` (it takes the icon's data), a route's builder (it returns a
widget, not a box), `Scaffold` (it is assembled with `.build()`), `RotationTransition` (there is
none), and `Curve::cubic` (it is the `Cubic` variant).

## What writing it found

The guide is also an audit of how easy frus is to write. These are the places where the
example had to be longer than it should be; they are the next milestone's work, not this one's:

- **Nineteen callbacks insist on a message.** `on_input`, `on_toggle` and `on_change` accept a
  closure that returns nothing, so a function component writes `move |t| name.set(t)`. But
  `Scaffold::nav`, `TabBar`, `SegmentedButton`, `NavigationBar`, `RadioGroup`, `DropdownButton`'s
  options, `Pagination` and twelve others take `impl Fn(usize) -> Msg` only: in a function
  component the closure must build a `Callback` by hand. The guide's scaffold example is a
  `StatefulWidget` for that reason.
- **A snack bar has no one-line form.** Showing "Saved" for three seconds is a queue in the
  state and two timers (the gallery's demo is 60 lines). The reference's
  `ScaffoldMessenger.of(context).showSnackBar(..)` has no counterpart.
- **An app bar under a router does not offer the way back.** It adds a menu button for a
  drawer, but not the back arrow a pushed screen needs; each screen writes its own.
- **`AppBar` and `Scaffold` are builders, not widgets**: forgetting `.build()` is a type error
  that names neither.
- **No list builds rows of different heights lazily.** `ListView` needs one extent for every
  row; a feed of posts of different lengths is a `Column` in a scroll view, built whole.
- **An explicit animation in a function component** is a `use_ref` holding an
  `AnimationController`, a `use_interval` ticking it, and a `use_state` mirroring its value.
  There is no hook that drives a controller from the frame clock.
- **An application cannot drive a page in its tests.** The shell's test driver taps, types
  and reads text through the real event path, but it is hidden behind an internal feature; the
  testing page can only show state tests and rendered pixels.

## Tests

- The guide's own 42 examples, compiled as doctests; 4 of them run assertions.
- The full check (fmt, clippy on CI's rustc, wasm and Android, strict rustdoc, the workspace's
  tests) — see the PR.

## Not done

- **The seven points above**, for milestone 716 (writing frus easily).
- **A page on platforms**: permissions, the camera, files, notifications, the tray. The
  getting-started page and `platform-integration.md` cover how to ship; what each platform
  service looks like from an application deserves its own page once the list settles.
- **The guide in French.** The README is translated; the guide is not yet.
