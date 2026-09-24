# frus

The one dependency a frus application declares: it re-exports the shell, the widgets and
the `main!` entry point.

**Layer:** Application. The facade holds no logic of its own. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `FrusApp`: an application whose whole declaration is a root component.
- `StatelessWidget`, `StatefulWidget` and `State`, and the hooks on `BuildContext`
  (`use_state`, `use_ref`, `use_memo`, `use_effect`, `use_interval`): components, and the
  state they keep between rebuilds.
- `TextEditingController`, `ValueNotifier`, `ChangeNotifier`: state that something outside a
  widget needs to read or write, and to listen to.
- `GoRouter`, `GoRoute`, `GoRouterState`: routes, the stack of pages, redirects, and the page
  transition.
- `Application`, `Command`, `Subscription`: the typed-message model `FrusApp` is built on, from
  `frus-shell`, for an application that wants explicit messages and effects.
- Every widget, `Theme`, and the `column!` and `row!` macros, from `frus-widgets`.
- `main!`: the entry point for every platform. `frus::fonts` registers an application's
  own faces.

## Example

```rust,no_run
use frus::{button, column, text, BuildContext, FrusApp, Widget};

fn counter(cx: &BuildContext) -> Box<dyn Widget> {
    let count = cx.use_state(|| 0);
    let add = count.clone();
    Box::new(column![
        text(count.get().to_string()),
        button("+", move || add.update(|n| *n += 1)),
    ])
}

// Generates `run()` for the desktop, `android_main` for Android, and the web's start.
frus::main!(FrusApp::from_fn(counter));

fn main() -> frus::anyhow::Result<()> {
    run()
}
```

A complete version is [`frus-hello`](https://github.com/KalybosPro/frus/tree/master/crates/frus-hello),
and `cargo generate --path templates/app` starts a project from it.

## Features

`net` and `json` add HTTP and typed JSON. The `bundled-*` features (all on by default)
choose the fonts compiled in, and `icons-outlined`, `icons-rounded` and `icons-sharp` add
icon styles. `images` (default) decodes embedded PNG and JPEG.

## Part of frus

Add it with `cargo add frus`. See the
[workspace README](https://github.com/KalybosPro/frus#readme) and
[the getting-started guide](https://github.com/KalybosPro/frus/blob/master/docs/getting-started.md).
Licensed under MIT or Apache-2.0.
