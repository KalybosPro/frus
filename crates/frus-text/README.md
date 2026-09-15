# frus-text

Text shaping and measurement for frus, over
[`cosmic-text`](https://github.com/pop-os/cosmic-text), with the fonts it bundles.

**Layer:** Foundations. Given a string, a style and a width constraint, it answers how big
the text is and where its lines break. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `measure`, `measure_style`, `measure_wrapped`: the size a piece of text needs.
- `add_font`, `set_default_family`, `set_monospace_family`: registering an application's
  own faces.
- The `bundled-sans`, `bundled-italic`, `bundled-mono` and `bundled-arabic` features, all
  on by default: the faces compiled in. Dropping one never panics; see *Fonts, and what
  they weigh* in
  [the getting-started guide](https://github.com/KalybosPro/frus/blob/master/docs/getting-started.md).

## Example

```rust
let size = frus_text::measure("Hello, world", 16.0);
assert!(size.width > 0.0 && size.height > 0.0);
```

## Part of frus

An application reaches the font registration through `frus::fonts` and rarely needs
anything else here. See the [workspace README](https://github.com/KalybosPro/frus#readme).
Licensed under MIT or Apache-2.0.
