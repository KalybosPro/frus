# frus

The one dependency a frus application declares: it re-exports the shell, the widgets and
the `main!` entry point.

**Layer:** Application. The facade holds no logic of its own. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Application`, `Command`, `Subscription`: the model, from `frus-shell`.
- Every widget, `Theme`, and the `column!` and `row!` macros, from `frus-widgets`.
- `main!`: the entry point for every platform. `frus::fonts` registers an application's
  own faces.

## Example

```rust,no_run
use frus::{button, column, text, Application, Command, Theme, Widget};

#[derive(Default)]
struct Counter {
    count: i32,
}

#[derive(Clone)]
enum Msg {
    Increment,
}

impl Application for Counter {
    type Message = Msg;
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Increment => self.count += 1,
        }
        Command::none()
    }
    fn view(&self, _theme: &Theme) -> Box<dyn Widget<Msg>> {
        Box::new(column![text(self.count.to_string()), button("+", Msg::Increment)])
    }
}

// Generates `run()` for the desktop, `android_main` for Android, and the web's start.
frus::main!(Counter::default());

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

frus is not on crates.io yet; depend on it by `path` or git revision. See the
[workspace README](https://github.com/KalybosPro/frus#readme) and
[the getting-started guide](https://github.com/KalybosPro/frus/blob/master/docs/getting-started.md).
Licensed under MIT or Apache-2.0.
