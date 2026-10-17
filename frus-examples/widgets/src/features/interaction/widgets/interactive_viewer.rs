//! `InteractiveViewer` — a viewport whose content can be panned (drag) and zoomed (mouse
//! wheel, trackpad or pinch): a map, a diagram, a photo.
//!
//! The pan and the zoom are kept by the framework between frames; you only say how big
//! the viewport is and how far the zoom may go.

use frus::{
    component, Alignment, Column, Container, InteractiveViewer, LayoutBuilder, Row, Text, Widget,
};

use crate::common::shared::{section, sections};

/// The content is a 6 × 6 board of tiles.
const CELLS: usize = 6;
/// The viewport is square, as wide as the page allows up to this.
const MAX_SIDE: f32 = 360.0;

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let scheme = cx.theme().scheme;
        let muted = scheme.on_surface_variant;

        // At scale 1 the content **fills the viewport exactly**: the viewer lays its
        // child out at the viewport's size. So the board is sized from the room the
        // `LayoutBuilder` reports, and zooming in is what reveals detail and makes room
        // to pan.
        let viewer = LayoutBuilder::new(move |size| {
            // Less the 1 px border drawn round it on each side.
            let side = (size.width - 2.0).min(MAX_SIDE);
            let cell = side / CELLS as f32;
            let board = (0..CELLS).fold(Column::new(), |column, row| {
                column.child((0..CELLS).fold(Row::new(), |line, col| {
                    // A checkerboard of two tones, each tile named by its coordinates.
                    let dark = (row + col) % 2 == 0;
                    line.child(
                        Container::new()
                            .width(cell)
                            .height(cell)
                            .color(if dark {
                                scheme.primary_container
                            } else {
                                scheme.surface_container_highest
                            })
                            .alignment(Alignment::CENTER)
                            .child(
                                Text::new(format!("{}{}", (b'A' + row as u8) as char, col + 1))
                                    .size(13.0)
                                    .color(if dark {
                                        scheme.on_primary_container
                                    } else {
                                        scheme.on_surface
                                    }),
                            ),
                    )
                }))
            });

            // Sized, or the frame would stretch to the whole width the builder was given.
            Container::new()
                .width(side + 2.0)
                .height(side + 2.0)
                .radius(12.0)
                .clip()
                .border(1.0, scheme.outline_variant)
                .child(
                    InteractiveViewer::new()
                        .width(side)
                        .height(side)
                        // From half size to four times the natural size.
                        .min_scale(0.5)
                        .max_scale(4.0)
                        .child(board),
                )
        });

        sections(vec![
            Box::new(section("Wheel or pinch to zoom, then drag to pan", viewer)),
            Box::new(
                Text::new(
                    "At scale 1 the board fills the viewport and cannot move; zoomed in, \
                     it can be dragged until an edge meets the viewport's.",
                )
                .wrap()
                .size(13.0)
                .color(muted),
            ),
        ])
    }))
}
