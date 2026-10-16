//! `GridView` — cells in equal columns, placed row by row: a fixed number of columns,
//! as many as fit a maximum width, or a virtualised grid that only builds the visible
//! rows.

use frus::{Center, Color, Colors, Container, GridView, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // A fixed number of columns. `aspect` makes every tile the same shape
        // (width / height); without it a row is as tall as its tallest cell.
        Box::new(section(
            "Three columns, square tiles",
            Container::new().width(320.0).child((0..9).fold(
                GridView::new(3).width(320.0).gap(8.0).aspect(1.0),
                |grid, i| grid.cell(tile(i)),
            )),
        )),
        // `extent` takes a maximum tile width and works out how many columns fit — more
        // on a wide window, fewer on a phone, with no breakpoints. Its cells are built
        // from their index once the width is known.
        Box::new(section(
            "As many columns as fit (tiles ≤ 72 px)",
            GridView::extent(72.0, 12, tile).gap(6.0).aspect(1.0),
        )),
        // `builder` virtualises: a thousand tiles, only the visible rows built. It
        // scrolls by itself, so it needs a height here.
        Box::new(section(
            "1 000 tiles, virtualised",
            GridView::builder(4, 1000, tile)
                .gap(4.0)
                .tile_height(56.0)
                .width(320.0)
                .height(240.0),
        )),
    ])
}

/// A numbered, coloured tile. A plain function of the index, which is what the grids
/// that build their cells late (`extent`, `builder`) need.
fn tile(i: usize) -> Container {
    const HUES: [&str; 5] = ["#6366F1", "#0EA5E9", "#10B981", "#F59E0B", "#EF4444"];
    Container::new()
        .radius(6.0)
        .color(Color::hex(HUES[i % HUES.len()]))
        .child(Center::new(
            Text::new(format!("{}", i + 1)).color(Colors::WHITE),
        ))
}
