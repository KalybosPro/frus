# frus-test

Test tooling for frus: widgets rendered offscreen, snapshots queried pixel by pixel, and
golden images compared on disk.

**Layer:** Application, as a consumer of the widgets and the renderer; it belongs in
`[dev-dependencies]`. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `render_widget` and `render_scene`: one settled frame through the same pipeline as the
  window. Both return `None` when there is no GPU adapter, so a test can skip itself.
- `Snapshot`: `pixel`, `diff_count`, and `assert_golden`, which writes the reference PNG
  the first time and compares against it after that. `FRUS_UPDATE_GOLDENS=1` rewrites it.
- `Stage`: the retained state stepped frame by frame, for a gesture caught in flight.

Pure `update` logic needs none of this: the application's state is a plain struct.

## Example

```rust,no_run
use frus_test::render_widget;
use frus_widgets::{text, Theme, Widget};

let root: Box<dyn Widget<()>> = Box::new(text("Hello"));
if let Some(shot) = render_widget(root.as_ref(), 200, 80, &Theme::dark()) {
    shot.assert_golden("tests/goldens/hello.png");
}
```

Goldens depend on the rasteriser: generate and compare them in the same environment.

## Part of frus

See the [workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT
or Apache-2.0.
