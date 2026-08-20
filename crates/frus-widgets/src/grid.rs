//! [`Grid`]: a **grid** container of equal columns. Cells place themselves
//! automatically, row by row.
//!
//! Unlike the composites, `Grid` is a **plain container**: the layout is
//! done by the layout engine (taffy's CSS Grid), so
//! `cell()` is nothing more than adding a child.
//!
//! **How tall is a row?** By default the tallest cell in it, which is what a grid of
//! forms or of labels wants. A grid of *tiles* — photos, cards, a colour swatch board —
//! wants every tile the same shape instead, and says so with [`Grid::aspect`] (a
//! `width / height` ratio, `1.0` for squares) or [`Grid::tile_height`] (an exact number
//! of pixels). The reference draws the same distinction and puts the choice on the grid,
//! not on the tile: a tile cannot know how wide its column came out.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A grid of `columns` equal columns.
pub struct Grid<Msg> {
    columns: usize,
    gap: f32,
    row_gap: Option<f32>,
    column_gap: Option<f32>,
    aspect: Option<f32>,
    tile_height: Option<f32>,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Grid<Msg> {
    /// Creates a grid of `columns` columns, at least one.
    pub fn new(columns: usize) -> Self {
        Self {
            columns: columns.max(1),
            gap: 0.0,
            row_gap: None,
            column_gap: None,
            aspect: None,
            tile_height: None,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            children: Vec::new(),
        }
    }

    /// Spacing between cells (rows and columns), in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Spacing **between rows** only, in logical pixels. Overrides [`Grid::gap`] down
    /// the grid and leaves it alone across.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = Some(gap.max(0.0));
        self
    }

    /// Spacing **between columns** only, in logical pixels. Overrides [`Grid::gap`]
    /// across the grid and leaves it alone down.
    pub fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = Some(gap.max(0.0));
        self
    }

    /// Gives every cell the same **shape**: a `width / height` ratio, so `1.0` is a
    /// square tile, `1.5` a landscape one and `0.75` a portrait one.
    ///
    /// The height follows from the column's width, which means it follows the grid's:
    /// the same board is square on a phone and square on a desktop, with wider tiles.
    /// A cell that has chosen a size of its own keeps it.
    ///
    /// [`Grid::tile_height`] wins over this, as the exact number wins over the ratio in
    /// the reference.
    pub fn aspect(mut self, ratio: f32) -> Self {
        self.aspect = Some(ratio.max(f32::MIN_POSITIVE));
        self
    }

    /// Gives every row the same **height**, in logical pixels, whatever its content —
    /// the reference's per-tile main-axis extent.
    ///
    /// Unlike [`Grid::aspect`] this is a fixed number rather than one derived from the
    /// width, which is what a row of fixed-height cards wants. It wins over `aspect`.
    pub fn tile_height(mut self, height: f32) -> Self {
        self.tile_height = Some(height.max(0.0));
        self
    }

    /// Sets the width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// Sets the height, in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// Flex growth factor along the parent's main axis.
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Adds a cell, placed automatically in the grid.
    pub fn cell(mut self, cell: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(cell));
        self
    }
}

impl<Msg> Widget<Msg> for Grid<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            gap: self.gap,
            row_gap: self.row_gap,
            column_gap: self.column_gap,
            grid_columns: Some(self.columns),
            grid_row_height: self.tile_height,
            ..Default::default()
        }
    }

    fn tile_shape(&self) -> Option<f32> {
        // An exact height is an exact height: a ratio on top of it would be asking for
        // two heights at once, and the reference resolves the same clash the same way.
        if self.tile_height.is_some() {
            return None;
        }
        self.aspect
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Primitive};

    fn tile(c: Color) -> Container<()> {
        Container::<()>::new().height(20.0).color(c)
    }

    #[test]
    fn cells_flow_into_rows_and_columns() {
        let a = Color::rgb(1.0, 0.0, 0.0);
        let b = Color::rgb(0.0, 1.0, 0.0);
        let c = Color::rgb(0.0, 0.0, 1.0);
        let d = Color::rgb(1.0, 1.0, 0.0);
        // A 2-column grid: [a b] / [c d].
        let grid = Grid::<()>::new(2)
            .gap(10.0)
            .width(220.0)
            .cell(tile(a))
            .cell(tile(b))
            .cell(tile(c))
            .cell(tile(d));
        let ui = build_ui(
            &grid,
            Size::new(220.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );

        let rect_of = |col: Color| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == col => Some(*rect),
                    _ => None,
                })
                .expect("the cell is present")
        };
        let (ra, rb, rc, rd) = (rect_of(a), rect_of(b), rect_of(c), rect_of(d));

        // a and b: the same row, same y, in different columns, b right of a.
        assert_eq!(ra.y, rb.y);
        assert!(rb.x > ra.x);
        // c sits under a, on the next row, in the same column, same x.
        assert!(rc.y > ra.y);
        assert_eq!(rc.x, ra.x);
        // d lines up with b in x and with c in y.
        assert_eq!(rd.x, rb.x);
        assert_eq!(rd.y, rc.y);
        // Equal columns: a's width equals b's.
        assert_eq!(ra.width, rb.width);
    }

    /// The rectangle the first (red) cell paints, in a grid laid out 220 wide.
    fn first_cell(grid: Grid<()>) -> Rect {
        let ui = build_ui(
            &grid,
            Size::new(220.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { color, rect, .. } if color.r > 0.9 && color.g < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the first cell is painted")
    }

    fn filled() -> Container<()> {
        Container::<()>::new().color(Color::rgb(1.0, 0.0, 0.0))
    }

    /// Without a shape a row is as tall as its content: two labels make a 19 px band,
    /// not a grid.
    #[test]
    fn a_row_follows_its_content_by_default() {
        let rect = first_cell(
            Grid::<()>::new(2)
                .width(220.0)
                .cell(filled().child(crate::Text::new("x")))
                .cell(filled().child(crate::Text::new("x"))),
        );
        assert_eq!(rect.width, 110.0);
        assert!(rect.height < 40.0, "content height, not a tile: {rect:?}");
    }

    /// `aspect(1.0)`: the tile is as tall as the column is wide, and the column's width
    /// is the grid's — so the same board is square at any size.
    #[test]
    fn a_ratio_derives_the_tile_height_from_the_column() {
        let square = first_cell(
            Grid::<()>::new(2)
                .width(220.0)
                .aspect(1.0)
                .cell(filled())
                .cell(filled()),
        );
        assert_eq!((square.width, square.height), (110.0, 110.0));

        // Twice as wide as it is tall, and the gap comes out of the column first.
        let wide = first_cell(
            Grid::<()>::new(2)
                .width(220.0)
                .gap(10.0)
                .aspect(2.0)
                .cell(filled())
                .cell(filled()),
        );
        assert_eq!(wide.width, 105.0);
        assert!(
            (wide.height - 52.5).abs() <= 0.5,
            "half the width: {wide:?}"
        );
    }

    /// An exact height is an exact height, whatever the grid's width, and it wins over
    /// a ratio rather than fighting it.
    #[test]
    fn an_exact_tile_height_wins_over_a_ratio() {
        let rect = first_cell(
            Grid::<()>::new(2)
                .width(220.0)
                .aspect(1.0)
                .tile_height(60.0)
                .cell(filled())
                .cell(filled()),
        );
        assert_eq!((rect.width, rect.height), (110.0, 60.0));
    }

    /// A cell that has chosen a size of its own keeps it: the grid's shape is a default
    /// for the cells that did not say, not an override of the ones that did.
    #[test]
    fn a_cell_that_chose_its_size_keeps_it() {
        let rect = first_cell(
            Grid::<()>::new(2)
                .width(220.0)
                .aspect(1.0)
                .cell(filled().height(20.0))
                .cell(filled().height(20.0)),
        );
        assert_eq!(rect.height, 20.0);
    }

    /// The two spacings are separate: 4 across, 20 down.
    #[test]
    fn rows_and_columns_space_apart_separately() {
        let grid = Grid::<()>::new(2)
            .width(220.0)
            .column_gap(4.0)
            .row_gap(20.0)
            .tile_height(30.0)
            .cell(filled())
            .cell(filled())
            .cell(filled());
        let ui = build_ui(
            &grid,
            Size::new(220.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let cells: Vec<Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, rect, .. } if color.r > 0.9 && color.g < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(cells.len(), 3);
        // Across: 220 = 108 + 4 + 108.
        assert_eq!(cells[0].width, 108.0);
        assert_eq!(cells[1].x - (cells[0].x + cells[0].width), 4.0);
        // Down: a 30 px row, then 20 px of gap.
        assert_eq!(cells[2].y - (cells[0].y + cells[0].height), 20.0);
    }
}
