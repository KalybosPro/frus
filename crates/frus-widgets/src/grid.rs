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
//!
//! **How many columns?** [`Grid::new`] takes the number. [`Grid::extent`] takes a
//! *maximum tile width* instead and works the count out from the room there is, which is
//! what a photo grid actually wants: four columns on a phone, nine on a desktop, without
//! the application computing breakpoints. That one cannot be a plain container — the
//! count depends on a width nobody has until the layout runs — so it builds its cells
//! late, from a factory, and the caveats of building late apply (see [`Grid::extent`]).

use std::cell::OnceCell;
use std::rc::Rc;

use frus_core::{Rect, Scene, Size};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A cell factory: the widget at an index, boxed. Shared rather than owned because the
/// composition below has to outlive the borrow that reads it.
type CellAt<Msg> = Rc<dyn Fn(usize) -> Box<dyn Widget<Msg>>>;

/// The `size → widget` composition a late-building grid hands the walk: given the box it
/// was allotted, the grid it puts its cells in.
type Composed<Msg> = Box<dyn Fn(Size) -> Box<dyn Widget<Msg>>>;

/// Where a grid's cells come from: given, or built once the width is known.
enum Cells<Msg> {
    /// Added one at a time with [`Grid::cell`]. The grid is a plain container and the
    /// layout engine places them.
    Given(Vec<Box<dyn Widget<Msg>>>),
    /// Built from an index, with the column count derived from `max` and the width the
    /// grid is actually given ([`Grid::extent`]).
    Built {
        max: f32,
        count: usize,
        build: CellAt<Msg>,
    },
}

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
    cells: Cells<Msg>,
    /// The `size → widget` closure a [`Grid::extent`] hands the walk, composed on first
    /// use: the builder methods may still be changing the spacing and the shape when the
    /// grid is constructed, so it cannot be composed before it is asked for.
    composed: OnceCell<Composed<Msg>>,
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
            cells: Cells::Given(Vec::new()),
            composed: OnceCell::new(),
        }
    }

    /// Creates a grid whose columns are **as many as fit**, each at most
    /// `max_tile_width` logical pixels across — the reference's max-cross-axis-extent
    /// delegate, and the one a responsive photo or card grid wants.
    ///
    /// The count is the reference's arithmetic exactly: `ceil(width / (max + column
    /// gap))`, at least one, and the room left over is then divided equally, so the
    /// tiles come out *at most* `max_tile_width` and never leave a ragged edge. A 500 px
    /// grid at `extent(150.0)` is four columns of 125, not three of 166.
    ///
    /// The cells are **built from their index**, because there is no grid to put them in
    /// until the width is known:
    ///
    /// ```ignore
    /// Grid::extent(160.0, photos.len(), move |i| photo_tile(&photos[i]))
    ///     .gap(8.0)
    ///     .aspect(1.0)
    /// ```
    ///
    /// Two consequences of building late, both shared with [`crate::LayoutBuilder`],
    /// which is the mechanism underneath:
    ///
    /// - the factory runs **more than once a frame**, so it must be cheap and free of
    ///   side effects;
    /// - the cells have **no retained state** — hover and clicks work, persistent
    ///   keyboard focus and deferred overlays do not. A grid of images or of buttons is
    ///   fine; a grid of text fields is not, and wants [`Grid::new`] with a count the
    ///   application chose.
    pub fn extent<W: Widget<Msg> + 'static>(
        max_tile_width: f32,
        count: usize,
        build: impl Fn(usize) -> W + 'static,
    ) -> Self {
        Self {
            columns: 1,
            gap: 0.0,
            row_gap: None,
            column_gap: None,
            aspect: None,
            tile_height: None,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
            cells: Cells::Built {
                max: max_tile_width.max(f32::MIN_POSITIVE),
                count,
                build: Rc::new(move |i| Box::new(build(i)) as Box<dyn Widget<Msg>>),
            },
            composed: OnceCell::new(),
        }
    }

    /// How many columns of at most `max` fit across `width`, and what is left for each —
    /// the reference's delegate, transcribed.
    fn columns_across(max: f32, cross_gap: f32, width: f32) -> usize {
        let count = (width / (max + cross_gap)).ceil();
        if count.is_finite() && count >= 1.0 {
            count as usize
        } else {
            1
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
    ///
    /// A grid built by [`Grid::extent`] gets its cells from its factory and ignores this.
    pub fn cell(mut self, cell: impl Widget<Msg> + 'static) -> Self {
        if let Cells::Given(cells) = &mut self.cells {
            cells.push(Box::new(cell));
        }
        self
    }
}

impl<Msg: 'static> Widget<Msg> for Grid<Msg> {
    fn style(&self) -> Style {
        // A grid that builds late is a **leaf** to the layout: its cells live in a tree
        // of their own, laid out inside the box this style asks for. Declaring the grid
        // properties here as well would make the engine place children it does not have.
        if matches!(self.cells, Cells::Built { .. }) {
            return Style {
                width: self.width,
                height: self.height,
                flex_grow: self.flex_grow,
                ..Default::default()
            };
        }
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
        // A grid that builds late says neither: the grid it builds says both.
        if self.tile_height.is_some() || matches!(self.cells, Cells::Built { .. }) {
            return None;
        }
        self.aspect
    }

    fn layout_builder(&self) -> Option<&dyn Fn(Size) -> Box<dyn Widget<Msg>>> {
        let Cells::Built { max, count, build } = &self.cells else {
            return None;
        };
        let (max, count, build) = (*max, *count, build.clone());
        let (gap, row_gap, column_gap) = (self.gap, self.row_gap, self.column_gap);
        let (aspect, tile_height) = (self.aspect, self.tile_height);
        Some(&**self.composed.get_or_init(|| {
            Box::new(move |size: Size| {
                let columns = Self::columns_across(max, column_gap.unwrap_or(gap), size.width);
                let mut grid = Grid::new(columns).gap(gap);
                if let Some(g) = row_gap {
                    grid = grid.row_gap(g);
                }
                if let Some(g) = column_gap {
                    grid = grid.column_gap(g);
                }
                if let Some(r) = aspect {
                    grid = grid.aspect(r);
                }
                if let Some(h) = tile_height {
                    grid = grid.tile_height(h);
                }
                if let Cells::Given(cells) = &mut grid.cells {
                    cells.extend((0..count).map(|i| build(i)));
                }
                Box::new(grid) as Box<dyn Widget<Msg>>
            })
        }))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        match &self.cells {
            Cells::Given(cells) => cells,
            Cells::Built { .. } => &[],
        }
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

    /// Every tile a `Grid::extent` painted, in a grid laid out `width` across.
    fn extent_tiles(width: f32, grid: Grid<()>) -> Vec<Rect> {
        let root = crate::Flex::<()>::column()
            .width(width)
            .height(2000.0)
            .child(grid);
        let ui = build_ui(
            &root,
            Size::new(width, 2000.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { color, rect, .. } if color.r > 0.9 && color.g < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect()
    }

    /// The reference's example, transcribed: a 500 px grid at a maximum tile of 150 is
    /// **four columns of 125**, not three of 166. That is the difference between the
    /// reference's arithmetic — as many columns as it takes for none to exceed the
    /// maximum — and the CSS `auto-fill` that looks like it and is not.
    #[test]
    fn the_column_count_comes_from_the_tile_width() {
        let tiles = extent_tiles(500.0, Grid::extent(150.0, 4, |_| filled().height(20.0)));
        assert_eq!(tiles.len(), 4);
        for t in &tiles {
            assert_eq!(t.width, 125.0, "four equal columns of 125: {tiles:?}");
        }
        // All four on one row.
        assert!(tiles.iter().all(|t| t.y == tiles[0].y), "{tiles:?}");
    }

    /// The same grid at three widths: the count follows the room, which is the whole
    /// point of the delegate.
    #[test]
    fn the_count_follows_the_room() {
        let columns_at = |width: f32| {
            let tiles = extent_tiles(width, Grid::extent(150.0, 12, |_| filled().height(20.0)));
            let top = tiles[0].y;
            tiles.iter().filter(|t| t.y == top).count()
        };
        assert_eq!(columns_at(300.0), 2);
        assert_eq!(columns_at(500.0), 4);
        assert_eq!(columns_at(900.0), 6);
    }

    /// The column gap comes out of the arithmetic, exactly as it does in the reference:
    /// the count divides `max + gap`, then the gaps come out of the room before the
    /// columns share it.
    #[test]
    fn the_gap_is_part_of_the_arithmetic() {
        let tiles = extent_tiles(
            500.0,
            Grid::extent(150.0, 4, |_| filled().height(20.0)).gap(10.0),
        );
        // ceil(500 / 160) = 4 columns; (500 - 3×10) / 4 = 117.5 — which the layout then
        // rounds to whole pixels, alternating 118 and 117 so that the row still ends
        // exactly on the edge rather than drifting half a pixel per column.
        let top = tiles[0].y;
        assert_eq!(tiles.iter().filter(|t| t.y == top).count(), 4);
        for t in &tiles {
            assert!(
                (t.width - 117.5).abs() <= 0.5,
                "each column is 117.5 to the pixel: {tiles:?}"
            );
        }
        let last = tiles.last().expect("four tiles");
        assert_eq!(last.x + last.width, 500.0, "and the row ends on the edge");
    }

    /// The shape and the spacings travel into the grid it builds — they are set on the
    /// outer one, which never lays anything out itself.
    #[test]
    fn the_tile_shape_reaches_the_built_grid() {
        let tiles = extent_tiles(500.0, Grid::extent(150.0, 4, |_| filled()).aspect(1.0));
        assert_eq!((tiles[0].width, tiles[0].height), (125.0, 125.0));
    }

    /// A grid that builds late is as tall as what it built (milestone 355), so a sibling
    /// below it starts where the tiles end rather than on top of them.
    #[test]
    fn it_is_as_tall_as_the_tiles_it_built() {
        let root = crate::Flex::<()>::column()
            .width(500.0)
            .height(2000.0)
            .child(Grid::extent(150.0, 8, |_| filled()).aspect(1.0))
            .child(
                Container::<()>::new()
                    .height(10.0)
                    .color(Color::rgb(0.0, 1.0, 0.0)),
            );
        let ui = build_ui(
            &root,
            Size::new(500.0, 2000.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let sibling = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.g > 0.9 && color.r < 0.1 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the sibling is painted");
        // Eight tiles, four to a row, each 125 square: two rows.
        assert_eq!(sibling.y, 250.0, "below both rows of tiles");
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
