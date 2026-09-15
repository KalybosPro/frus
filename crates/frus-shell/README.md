# frus-shell

The platform layer of frus: the window, the event loop, and the `Application` model an
application is written against, on desktop, Android and the web.

**Layer:** Shell. It is the only crate that knows which platform it runs on; everything
platform-specific (winit, the Android input and clipboard bridges, AccessKit, the browser)
is gated here and nowhere else. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Application`: `update`, `view`, and optionally `subscription`, `init`, `title` and
  the rest.
- `Command` and `Subscription`: effects, including `perform_async` futures, and
  continuous sources such as `Subscription::every`.
- `main!`: one declaration that generates the desktop `run()`, Android's `android_main`
  and the web's start function.

## Example

```rust,no_run
use frus_shell::{Application, Command};
use frus_widgets::{text, Theme, Widget};

struct Hello;

impl Application for Hello {
    type Message = ();
    fn update(&mut self, _message: ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, _theme: &Theme) -> Box<dyn Widget<()>> {
        Box::new(text("Hello"))
    }
}

fn main() -> frus_shell::anyhow::Result<()> {
    frus_shell::run(Hello)
}
```

## Features

`net` adds the `fetch` helper and `Request` (ureq natively, the browser's `fetch` on the
web); `json` adds typed bodies through serde. The `bundled-*` features forward the font
choice.

## Part of frus

Applications usually depend on the `frus` facade, which re-exports this crate. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
