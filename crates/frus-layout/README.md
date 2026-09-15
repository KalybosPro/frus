# frus-layout

frus's layout engine: it turns a tree of styled nodes into positioned rectangles.

**Layer:** Foundations. Flexbox comes from [`taffy`](https://github.com/DioxusLabs/taffy),
kept behind frus's own types so it can be replaced without breaking the API. It does not
know what a widget is. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What you reach for

- `Layout`: the node tree. Add leaves and containers, `compute` it, read the rects back.
- `Style`, with `Dimension`, `FlexDirection`, `Justify` and `Align`: what a node asks for.
- `Overflowing`: what `Layout::overflows` reports when children do not fit their parent.

## Example

```rust
use frus_layout::{Dimension, FlexDirection, Layout, Size, Style};

let fixed = |width| Style {
    width: Dimension::Length(width),
    height: Dimension::Length(20.0),
    ..Style::default()
};

let mut layout: Layout<'_, &str> = Layout::new();
let a = layout.leaf(fixed(80.0), "a");
let b = layout.leaf(fixed(80.0), "b");
let row = layout.container(
    Style { flex_direction: FlexDirection::Row, gap: 10.0, ..Style::default() },
    &[a, b],
);
layout.compute(row, Size::new(400.0, 100.0));

let rects = layout.absolute_rects(row);
let (b_rect, _) = rects.iter().find(|(_, data)| *data == Some(&"b")).unwrap();
assert_eq!(b_rect.x, 90.0);
```

## Part of frus

Widgets build their layout through this crate; an application describes layout with
widgets instead. See the [workspace README](https://github.com/KalybosPro/frus#readme).
Licensed under MIT or Apache-2.0.
