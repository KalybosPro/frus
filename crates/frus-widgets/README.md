# frus-widgets

frus's widget library and interaction model: the widget tree, theming, and the build that
turns a tree into a scene plus everything needed to hit-test, focus and scroll it.

**Layer:** Widgets, above the foundations and below the shell. It is platform-independent:
the shell feeds it input, and it never opens a window. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Widget<Msg>`: the trait every widget implements, generic over the application's
  message type.
- `column!`, `row!`, `text`, `button`, `Container`, `Scaffold`, `Theme`: what a `view`
  is written with.
- `build_ui`, `Ui` and `Runtime`: a tree built for one frame, against the state kept
  between frames. The shell and `frus-test` call these; an application rarely does.

## Example

```rust
use frus_widgets::{build_ui, column, text, Runtime, Size, Theme, Widget};

let root: Box<dyn Widget<()>> = Box::new(column![text("Hello"), text("world")]);
let (runtime, theme) = (Runtime::default(), Theme::dark());

let ui = build_ui(root.as_ref(), Size::new(320.0, 240.0), &runtime, &theme);
assert!(!ui.scene().is_empty());
```

## Features

`images` (default) decodes embedded PNG and JPEG; the `bundled-*` features choose the
fonts compiled in; `icons-outlined`, `icons-rounded` and `icons-sharp` add icon styles
beyond the filled set.

## Part of frus

Applications get all of this through the `frus` facade. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
