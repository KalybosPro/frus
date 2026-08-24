//! [`GridView`]: a **grid** container of equal columns. Cells place themselves
//! automatically, row by row.
//!
//! Unlike the composites, `GridView` is a **plain container**: the layout is
//! done by the layout engine (taffy's CSS Grid), so
//! `cell()` is nothing more than adding a child.
//!
//! **How tall is a row?** By default the tallest cell in it, which is what a grid of
//! forms or of labels wants. A grid of *tiles* — photos, cards, a colour swatch board —
//! wants every tile the same shape instead, and says so with [`GridView::aspect`] (a
//! `width / height` ratio, `1.0` for squares) or [`GridView::tile_height`] (an exact number
//! of pixels). The reference draws the same distinction and puts the choice on the grid,
//! not on the tile: a tile cannot know how wide its column came out.
//!
//! **How many columns?** [`GridView::new`] takes the number. [`GridView::extent`] takes a
//! *maximum tile width* instead and works the count out from the room there is, which is
//! what a photo grid actually wants: four columns on a phone, nine on a desktop, without
//! the application computing breakpoints. That one cannot be a plain container — the
//! count depends on a width nobody has until the layout runs — so it builds its cells
//! late, from a factory, and the caveats of building late apply (see [`GridView::extent`]).

use std::cell::OnceCell;
use std::rc::Rc;

use frus_core::{Rect, Scene, Size};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;
use frus_layout::Align;

/// A cell factory: the widget at an index, boxed. Shared rather than owned because the
/// composition below has to outlive the borrow that reads it.
type CellAt<Msg> = Rc<dyn Fn(usize) -> Box<dyn Widget<Msg>>>;

/// The `size → widget` composition a late-building grid hands the walk: given the box it
/// was allotted, the grid it puts its cells in.
type Composed<Msg> = Box<dyn Fn(Size) -> Box<dyn Widget<Msg>>>;
/// Builds the widget for one **row** of a windowed grid.
type RowAt<Msg> = Box<dyn Fn(usize) -> Box<dyn Widget<Msg>>>;

/// Where a grid's cells come from: given, or built once the width is known.
enum Cells<Msg> {
    /// Added one at a time with [`GridView::cell`]. The grid is a plain container and the
    /// layout engine places them.
    Given(Vec<Box<dyn Widget<Msg>>>),
    /// Built from an index, with the column count derived from `max` and the width the
    /// grid is actually given ([`GridView::extent`]).
    Built {
        max: f32,
        count: usize,
        build: CellAt<Msg>,
    },
    /// Built from an index like `Built`, but only the **visible rows**
    /// ([`GridView::builder`]).
    Windowed {
        columns: usize,
        count: usize,
        build: CellAt<Msg>,
    },
}

/// A grid of `columns` equal columns.
pub struct GridView<Msg> {
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
    /// The `size → widget` closure a [`GridView::extent`] hands the walk, composed on first
    /// use: the builder methods may still be changing the spacing and the shape when the
    /// grid is constructed, so it cannot be composed before it is asked for.
    composed: OnceCell<Composed<Msg>>,
    /// The `row → widget` closure a windowed grid hands the list machinery, composed on
    /// first use for the same reason as `composed` above.
    rows: OnceCell<RowAt<Msg>>,
}

impl<Msg> GridView<Msg> {
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
            rows: OnceCell::new(),
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
    /// GridView::extent(160.0, photos.len(), move |i| photo_tile(&photos[i]))
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
    ///   fine; a grid of text fields is not, and wants [`GridView::new`] with a count the
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
            rows: OnceCell::new(),
        }
    }

    /// A grid of `count` tiles in `columns` columns, of which only the **visible rows**
    /// are built, laid out and painted.
    ///
    /// The form a photo grid wants. [`GridView::new`] and [`GridView::extent`] both build
    /// every cell every frame — fine for a dozen swatches, and the wrong shape entirely
    /// for two thousand photographs, which is the same argument milestone 375 made about
    /// lists and which does not stop being true because the cells are in rows.
    ///
    /// ```ignore
    /// GridView::builder(3, photos.len(), |i| photo_tile(&photos[i]))
    ///     .aspect(1.0)
    ///     .gap(8.0)
    ///     .flex(1.0)
    /// ```
    ///
    /// **A virtualised grid is a list of rows**, and that is how this is built: the row
    /// height comes out of the tile shape and the width the grid was given, and from
    /// there it is a [`crate::ListView`] whose item is a row of cells. Nothing new
    /// windows, measures or scrolls — it is the list's machinery, which is why a
    /// windowed grid scrolls, flings, reverses and shows a scrollbar without any of that
    /// being written a second time.
    ///
    /// The shape of a tile is [`GridView::aspect`] or [`GridView::tile_height`], as in
    /// the other two forms. With neither, tiles are **square**: the window is found by
    /// division and division needs a number, so there has to be a default, and a square
    /// is the one a grid of photographs means when it says nothing.
    ///
    /// The caveats of building late apply, and here they are the list's: cells have **no
    /// retained state**, since a cell that does not exist off screen cannot keep any.
    /// Clicks and hover work; persistent keyboard focus and deferred overlays do not. A
    /// grid of images or buttons is right; a grid of text fields wants
    /// [`GridView::new`].
    pub fn builder<W: Widget<Msg> + 'static>(
        columns: usize,
        count: usize,
        build: impl Fn(usize) -> W + 'static,
    ) -> Self {
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
            cells: Cells::Windowed {
                columns: columns.max(1),
                count,
                build: Rc::new(move |i| Box::new(build(i)) as Box<dyn Widget<Msg>>),
            },
            composed: OnceCell::new(),
            rows: OnceCell::new(),
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

    /// Spacing **between rows** only, in logical pixels. Overrides [`GridView::gap`] down
    /// the grid and leaves it alone across.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = Some(gap.max(0.0));
        self
    }

    /// Spacing **between columns** only, in logical pixels. Overrides [`GridView::gap`]
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
    /// [`GridView::tile_height`] wins over this, as the exact number wins over the ratio in
    /// the reference.
    pub fn aspect(mut self, ratio: f32) -> Self {
        self.aspect = Some(ratio.max(f32::MIN_POSITIVE));
        self
    }

    /// Gives every row the same **height**, in logical pixels, whatever its content —
    /// the reference's per-tile main-axis extent.
    ///
    /// Unlike [`GridView::aspect`] this is a fixed number rather than one derived from the
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
    /// A grid built by [`GridView::extent`] gets its cells from its factory and ignores this.
    pub fn cell(mut self, cell: impl Widget<Msg> + 'static) -> Self {
        if let Cells::Given(cells) = &mut self.cells {
            cells.push(Box::new(cell));
        }
        self
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for GridView<Msg> {
    fn style(&self) -> Style {
        // A grid that builds late is a **leaf** to the layout: its cells live in a tree
        // of their own, laid out inside the box this style asks for. Declaring the grid
        // properties here as well would make the engine place children it does not have.
        if matches!(self.cells, Cells::Built { .. } | Cells::Windowed { .. }) {
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
        if self.tile_height.is_some()
            || matches!(self.cells, Cells::Built { .. } | Cells::Windowed { .. })
        {
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
                let mut grid = GridView::new(columns).gap(gap);
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

    fn virtual_list(&self, viewport: Size) -> Option<crate::list::VirtualList<'_, Msg>> {
        let Cells::Windowed {
            columns,
            count,
            build,
        } = &self.cells
        else {
            return None;
        };
        let (columns, count) = (*columns, *count);
        let across = self.column_gap.unwrap_or(self.gap);
        let down = self.row_gap.unwrap_or(self.gap);
        // What one tile gets across, once the gaps between the columns are taken out.
        // The same arithmetic the layout engine would do -- done here because the row
        // height comes from it, and the **window** comes from the row height. That is
        // the whole reason this hook is handed the box it was given: a grid cannot say
        // how many rows fit until it knows how wide its columns came out.
        let tile_w = ((viewport.width - across * columns.saturating_sub(1) as f32)
            / columns as f32)
            .max(0.0);
        let tile_h = self.tile_height.unwrap_or_else(|| match self.aspect {
            Some(ratio) if ratio > 0.0 => tile_w / ratio,
            _ => tile_w,
        });

        // The row factory needs the column count, the spacing and the cells -- none of
        // which depend on the width -- so it is composed **once**. The row height does
        // depend on the width, and it is not in here: it is the item extent above, and
        // the gap below the row is the row's own bottom padding, which is what keeps
        // the two apart. A factory that had to know the height would have to be rebuilt
        // on every resize, and a `OnceCell` cannot be.
        let rows = self.rows.get_or_init(|| {
            let cells = build.clone();
            Box::new(move |row: usize| {
                let mut strip = crate::Flex::row()
                    .align(Align::Stretch)
                    .gap(across)
                    .padding_each(0.0, 0.0, down, 0.0);
                for column in 0..columns {
                    let index = row * columns + column;
                    // The last row can be short. Its tiles keep the width every other
                    // row's have -- a half-full row of double-width photographs is not
                    // what "three columns" meant -- so the missing ones are empties
                    // that take their share and draw nothing.
                    strip = match index < count {
                        true => strip.child(crate::Expanded::new(cells(index))),
                        false => strip.child(crate::Expanded::new(crate::Flex::<Msg>::row())),
                    };
                }
                Box::new(strip) as Box<dyn Widget<Msg>>
            })
        });

        Some(crate::list::VirtualList {
            count: count.div_ceil(columns.max(1)),
            item_extent: tile_h + down,
            axis: crate::scroll::Axis::Vertical,
            build: &**rows,
        })
    }

    fn scroll_physics(&self) -> Option<crate::physics::ScrollPhysics> {
        None
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        match &self.cells {
            Cells::Given(cells) => cells,
            Cells::Built { .. } | Cells::Windowed { .. } => &[],
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

    /// **A cell could widen its own column, and take the grid off the screen with it.**
    ///
    /// A grid track written `1fr` is `minmax(auto, 1fr)`, and that `auto` floor is the
    /// item's min-content width: one card holding a long unbreakable title widened its
    /// own column, the others followed, and the grid came out wider than the box it was
    /// given. Reported from a real application whose product grid ran off the side of a
    /// phone — two columns of 315 in a window of 492.
    ///
    /// A share of a width is not a negotiation with the contents. The floor is zero.
    #[test]
    fn a_long_title_cannot_widen_its_column() {
        // Two columns in 300 px, one of them holding a title far too long for its share.
        let marker = Color::rgb(0.5, 0.25, 0.75);
        let grid: GridView<()> =
            GridView::new(2)
                .width(300.0)
                .gap(12.0)
                .cell(Container::new().height(40.0).color(marker).child(
                    crate::text("Espresso Creme de la Maison, Torrefaction Lente").size(15.0),
                ))
                .cell(Container::new().height(40.0));
        let marker_width = |ui: &crate::Ui<()>| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Rect { color, rect, .. } if *color == marker => Some(rect.width),
                    _ => None,
                })
                .expect("the cell is drawn")
        };

        let ui = build_ui(
            &grid,
            Size::new(300.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Every box the frame drew, and none of them may leave the grid.
        let widest = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } => Some(rect.x + rect.width),
                Primitive::Text { position, .. } => Some(position.x),
                _ => None,
            })
            .fold(0.0_f32, f32::max);
        assert!(widest <= 300.5, "nothing runs off the grid: {widest}");
        // And the cell holding it keeps to its share: (300 - 12) / 2.
        let w = marker_width(&ui);
        assert!(
            (w - 144.0).abs() < 1.0,
            "a cell takes its share and no more: {w}"
        );
    }

    #[test]
    fn cells_flow_into_rows_and_columns() {
        let a = Color::rgb(1.0, 0.0, 0.0);
        let b = Color::rgb(0.0, 1.0, 0.0);
        let c = Color::rgb(0.0, 0.0, 1.0);
        let d = Color::rgb(1.0, 1.0, 0.0);
        // A 2-column grid: [a b] / [c d].
        let grid = GridView::<()>::new(2)
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
    fn first_cell(grid: GridView<()>) -> Rect {
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
            GridView::<()>::new(2)
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
            GridView::<()>::new(2)
                .width(220.0)
                .aspect(1.0)
                .cell(filled())
                .cell(filled()),
        );
        assert_eq!((square.width, square.height), (110.0, 110.0));

        // Twice as wide as it is tall, and the gap comes out of the column first.
        let wide = first_cell(
            GridView::<()>::new(2)
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
            GridView::<()>::new(2)
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
            GridView::<()>::new(2)
                .width(220.0)
                .aspect(1.0)
                .cell(filled().height(20.0))
                .cell(filled().height(20.0)),
        );
        assert_eq!(rect.height, 20.0);
    }

    /// Every tile a `GridView::extent` painted, in a grid laid out `width` across.
    fn extent_tiles(width: f32, grid: GridView<()>) -> Vec<Rect> {
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
        let tiles = extent_tiles(500.0, GridView::extent(150.0, 4, |_| filled().height(20.0)));
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
            let tiles = extent_tiles(
                width,
                GridView::extent(150.0, 12, |_| filled().height(20.0)),
            );
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
            GridView::extent(150.0, 4, |_| filled().height(20.0)).gap(10.0),
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
        let tiles = extent_tiles(500.0, GridView::extent(150.0, 4, |_| filled()).aspect(1.0));
        assert_eq!((tiles[0].width, tiles[0].height), (125.0, 125.0));
    }

    /// A grid that builds late is as tall as what it built (milestone 355), so a sibling
    /// below it starts where the tiles end rather than on top of them.
    #[test]
    fn it_is_as_tall_as_the_tiles_it_built() {
        let root = crate::Flex::<()>::column()
            .width(500.0)
            .height(2000.0)
            .child(GridView::extent(150.0, 8, |_| filled()).aspect(1.0))
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
        let grid = GridView::<()>::new(2)
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

/// The windowed form: a grid that builds only the rows you can see.
#[cfg(test)]
mod windowed_tests {
    use super::*;
    use crate::{build_ui, Container, Runtime};
    use frus_core::{Color, Primitive, Rect};
    use std::cell::RefCell;
    use std::rc::Rc;

    const TILE: Color = Color::rgb(1.0, 0.0, 0.0);

    /// The tiles painted, and which indices the factory was asked for.
    fn painted(grid: GridView<()>, size: Size, runtime: &Runtime) -> Vec<Rect> {
        build_ui(&grid, size, runtime, &Theme::default())
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if *color == TILE => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// A grid of `count` square tiles, 3 across, in a 300x200 box.
    fn grid_of(count: usize, asked: Rc<RefCell<Vec<usize>>>) -> GridView<()> {
        GridView::<()>::builder(3, count, move |i| {
            asked.borrow_mut().push(i);
            Container::<()>::new().color(TILE)
        })
        .width(300.0)
        .height(200.0)
    }

    /// The point of the whole thing: two thousand tiles, and the factory is asked for
    /// the handful that show. `GridView::new` and `GridView::extent` both build every
    /// cell every frame, which is the shape this form exists to avoid.
    #[test]
    fn only_the_visible_rows_are_built() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let grid = grid_of(2000, asked.clone());
        let _ = painted(grid, Size::new(300.0, 200.0), &Runtime::default());
        let count = asked.borrow().len();
        assert!(
            count > 0 && count <= 12,
            "asked for {count} of 2000 tiles; a 300x200 box at 100px square holds nine"
        );
        assert_eq!(asked.borrow()[0], 0, "and it starts at the beginning");
    }

    /// Square by default, three across a 300 wide box: 100 each, and the second row a
    /// hundred down.
    #[test]
    fn tiles_are_square_and_rows_follow_them() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let tiles = painted(
            grid_of(9, asked),
            Size::new(300.0, 200.0),
            &Runtime::default(),
        );
        assert_eq!((tiles[0].x, tiles[0].y), (0.0, 0.0));
        assert_eq!((tiles[0].width, tiles[0].height), (100.0, 100.0));
        assert_eq!(tiles[1].x, 100.0, "the second is beside the first");
        assert_eq!(tiles[3].y, 100.0, "and the fourth is on the next row");
    }

    /// `aspect` reshapes the row, because the row height is *derived* from the tile
    /// shape and the width the grid was given -- which is why this form composes once
    /// the width is known rather than at construction.
    #[test]
    fn the_aspect_ratio_sets_the_row_height() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let tiles = painted(
            grid_of(6, asked).aspect(2.0),
            Size::new(300.0, 200.0),
            &Runtime::default(),
        );
        assert_eq!(tiles[0].height, 50.0, "twice as wide as tall");
        assert_eq!(tiles[3].y, 50.0, "so the rows are half as far apart");
    }

    /// The gap goes **between** the tiles across, and below the row down. Getting the
    /// second one wrong is invisible in a single row and wrong in every grid.
    #[test]
    fn the_gap_separates_columns_and_rows() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let tiles = painted(
            grid_of(6, asked).gap(10.0),
            Size::new(300.0, 200.0),
            &Runtime::default(),
        );
        // Three tiles and two 10px gaps across 300 leaves 280/3 each.
        let tile_w = 280.0 / 3.0;
        assert!((tiles[0].width - tile_w).abs() < 0.5, "{}", tiles[0].width);
        assert!((tiles[1].x - (tile_w + 10.0)).abs() < 0.5, "{}", tiles[1].x);
        assert!(
            (tiles[3].y - (tiles[0].height + 10.0)).abs() < 0.5,
            "the row below clears the gap: {}",
            tiles[3].y
        );
    }

    /// A short last row keeps its columns. Four tiles across three columns is a full row
    /// and one lonely tile -- and that tile is a third of the width, not the whole of it,
    /// because "three columns" did not stop meaning three.
    #[test]
    fn a_short_last_row_keeps_the_column_width() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let tiles = painted(
            grid_of(4, asked),
            Size::new(300.0, 200.0),
            &Runtime::default(),
        );
        assert_eq!(
            tiles.len(),
            4,
            "four tiles, not four plus two empties drawn"
        );
        assert_eq!(tiles[3].width, 100.0, "the lonely one keeps its column");
        assert_eq!(tiles[3].x, 0.0, "at the start of its row");
    }

    /// And it **scrolls**, without a line of scrolling being written here: a windowed
    /// grid is a list of rows, so it is the list that windows, measures and moves.
    #[test]
    fn it_scrolls_like_the_list_it_is_made_of() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let ui = build_ui(
            &grid_of(30, asked.clone()),
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Ten rows of 100 is 1000, in a 200 box.
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1, "one scrollable, and it is the grid");
        assert_eq!(maxes[0].2, 800.0);

        let mut scrolled = Runtime::default();
        scrolled
            .scroll
            .insert(crate::interaction::WidgetId::ROOT, (0.0, 250.0));
        asked.borrow_mut().clear();
        let tiles = painted(
            grid_of(30, asked.clone()),
            Size::new(300.0, 200.0),
            &scrolled,
        );
        assert_eq!(tiles[0].y, -50.0, "row 2 starts 50px above the window");
        assert_eq!(
            asked.borrow()[0],
            6,
            "and the rows before it were never built"
        );
    }
}
