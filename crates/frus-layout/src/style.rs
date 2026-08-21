//! Layout style — a thin frus API, translated to taffy internally.

use frus_core::Insets;

/// The dimension of one axis, width or height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dimension {
    /// A size decided by the content and the layout.
    Auto,
    /// A fixed size, in logical pixels.
    Length(f32),
    /// A percentage of the parent's size (`0.0..=1.0`).
    Percent(f32),
}

impl Dimension {
    fn to_taffy(self) -> taffy::Dimension {
        match self {
            Dimension::Auto => taffy::Dimension::Auto,
            Dimension::Length(v) => taffy::Dimension::Length(v),
            Dimension::Percent(p) => taffy::Dimension::Percent(p),
        }
    }
}

/// The main-axis direction of a flex container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlexDirection {
    /// Children laid out horizontally, first one at the start.
    Row,
    /// Children laid out vertically, first one at the top.
    Column,
    /// Horizontal, **last child first**. It is what a row laid out against the reading
    /// direction amounts to, and the only way to say so per-container: the reading
    /// direction itself is ambient and mirrors the whole frame.
    RowReverse,
    /// Vertical, **last child first** — a column that grows upwards, which is what a
    /// chat transcript or a stack of toasts anchored to the bottom wants.
    ColumnReverse,
}

impl FlexDirection {
    /// Whether the main axis is the horizontal one, whichever way it runs.
    pub fn is_horizontal(self) -> bool {
        matches!(self, FlexDirection::Row | FlexDirection::RowReverse)
    }

    /// Whether the children run backwards along the main axis.
    pub fn is_reversed(self) -> bool {
        matches!(
            self,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        )
    }

    /// The same axis, run backwards (or forwards again).
    pub fn reversed(self) -> Self {
        match self {
            FlexDirection::Row => FlexDirection::RowReverse,
            FlexDirection::RowReverse => FlexDirection::Row,
            FlexDirection::Column => FlexDirection::ColumnReverse,
            FlexDirection::ColumnReverse => FlexDirection::Column,
        }
    }

    fn to_taffy(self) -> taffy::FlexDirection {
        match self {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        }
    }
}

/// How children are distributed along the **main** axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    /// Equal space **everywhere**, the outer gaps included: with `SpaceAround` the end
    /// gaps are half the inner ones, which reads as a mistake when the row is the only
    /// thing on the line.
    SpaceEvenly,
}

impl Justify {
    fn to_taffy(self) -> taffy::JustifyContent {
        match self {
            Justify::Start => taffy::JustifyContent::FlexStart,
            Justify::Center => taffy::JustifyContent::Center,
            Justify::End => taffy::JustifyContent::FlexEnd,
            Justify::SpaceBetween => taffy::JustifyContent::SpaceBetween,
            Justify::SpaceAround => taffy::JustifyContent::SpaceAround,
            Justify::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
        }
    }
}

/// How a wrapping container's **lines** are distributed on the cross axis.
///
/// Not the same question as [`Align`], which places each child *within* its line. This
/// one places the lines themselves, and it only has an answer when there is more than
/// one — that is, when the container wraps and has cross-axis room to spare.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignContent {
    /// The lines fill the cross axis between them. Flexbox's default, and ours.
    #[default]
    Stretch,
    /// Packed at the near edge, keeping their own heights.
    Start,
    /// Packed at the middle.
    Center,
    /// Packed at the far edge.
    End,
    /// The first and last lines touch the edges, the room shared between them.
    SpaceBetween,
    /// Equal space around each line: the outer gaps are half the inner ones.
    SpaceAround,
    /// Equal space everywhere, the outer gaps included.
    SpaceEvenly,
}

impl AlignContent {
    fn to_taffy(self) -> taffy::AlignContent {
        match self {
            AlignContent::Stretch => taffy::AlignContent::Stretch,
            AlignContent::Start => taffy::AlignContent::FlexStart,
            AlignContent::Center => taffy::AlignContent::Center,
            AlignContent::End => taffy::AlignContent::FlexEnd,
            AlignContent::SpaceBetween => taffy::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => taffy::AlignContent::SpaceAround,
            AlignContent::SpaceEvenly => taffy::AlignContent::SpaceEvenly,
        }
    }
}

/// How children are aligned on the **cross** axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Start,
    Center,
    End,
    /// Children stretch to fill the cross axis. This is the default.
    Stretch,
    /// Children are aligned on their **text baselines**: the line their letters sit
    /// on, rather than their tops, their middles or their bottoms.
    ///
    /// It is the only alignment that makes two runs of different sizes look like one
    /// line — a price beside its currency, a heading beside a note. Nothing here
    /// resolves it: the layer above measures each child's baseline and turns the
    /// difference into a top margin, so by the time taffy sees it the children are
    /// already where they belong and it aligns them to the start.
    Baseline,
}

impl Align {
    fn to_taffy(self) -> taffy::AlignItems {
        match self {
            Align::Start => taffy::AlignItems::FlexStart,
            Align::Center => taffy::AlignItems::Center,
            Align::End => taffy::AlignItems::FlexEnd,
            Align::Stretch => taffy::AlignItems::Stretch,
            // Already resolved into per-child margins by then; from here it is a
            // start alignment, and it must be one — stretching would give every child
            // the row's height and there would be no baseline left to align.
            Align::Baseline => taffy::AlignItems::FlexStart,
        }
    }
}

/// A layout node's style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// Width.
    pub width: Dimension,
    /// Height.
    pub height: Dimension,
    /// **Minimum** width: a floor the layout never goes below. Useful for a box that
    /// grows with its content and never squashes (`Auto` = no floor).
    pub min_width: Dimension,
    /// **Minimum** height: a floor the layout never goes below — for instance a row
    /// that grows with tall content and never shrinks (`Auto` = no floor).
    pub min_height: Dimension,
    /// **Maximum** width: a ceiling the layout never goes above, whatever the content
    /// or the space on offer asks for (`Auto` = no ceiling).
    pub max_width: Dimension,
    /// **Maximum** height: the same ceiling on the vertical axis (`Auto` = none).
    pub max_height: Dimension,
    /// The main-axis grow factor (flexbox): the share of **spare** room this item takes.
    pub flex_grow: f32,
    /// The main-axis **shrink** factor: the share of a *deficit* this item absorbs when
    /// the children do not fit. `1.0` — the flexbox default — means everything squashes
    /// together, in proportion to its basis; `0.0` means *not me*, and is what fixed
    /// chrome wants: an icon button at the end of a row keeps its 40 px however long the
    /// label beside it is. See [`Style::flex_basis`] for the other half.
    pub flex_shrink: f32,
    /// The main-axis **basis**: the size an item starts from, before growing or
    /// shrinking. `Auto` — the default — means *use the content*, so a long label starts
    /// long and drags the whole row with it. `Length(0.0)` means *start from nothing and
    /// take what is left*, which is the shape of the reference's expanding child.
    pub flex_basis: Dimension,
    /// The main-axis direction, for a container.
    pub flex_direction: FlexDirection,
    /// Distribution along the main axis.
    pub justify: Justify,
    /// Alignment on the cross axis, for a container: what it does to its children.
    pub align: Align,
    /// How a **wrapping** container's lines are distributed on the cross axis.
    ///
    /// Silent unless the container wraps *and* has cross-axis room to spare, since one
    /// line has nowhere to go.
    pub align_content: AlignContent,
    /// This item's **own** cross-axis alignment, overriding whatever its parent asked
    /// for. `None` — the default — means "do as the others do".
    ///
    /// One box out of step with its row is a common enough shape that the alternative is
    /// a wrapper widget per exception, and flexbox already has the field.
    pub align_self: Option<Align>,
    /// Padding, per side, in logical pixels.
    pub padding: Insets,
    /// **Margin**, per side, in logical pixels: space reserved **around** the box,
    /// outside its decoration, which pushes siblings away.
    pub margin: Insets,
    /// When `Some(r)`, the box **keeps an aspect ratio** of `r` (`width / height`):
    /// the free dimension is derived from the constrained one. `None` means no
    /// ratio constraint.
    pub aspect_ratio: Option<f32>,
    /// Spacing between children, in logical pixels.
    pub gap: f32,
    /// Spacing **between rows**, when it differs from `gap`. `None` means `gap`.
    ///
    /// A grid's two spacings are separate in the reference — one along the axis it
    /// scrolls, one across it — and a tile grid usually wants them different.
    pub row_gap: Option<f32>,
    /// Spacing **between columns**, when it differs from `gap`. `None` means `gap`.
    pub column_gap: Option<f32>,
    /// When `true`, children **wrap** onto the next line (flex-wrap) once they
    /// overflow the main axis — automatic responsive reflow.
    pub flex_wrap: bool,
    /// When `Some(n)`, the container is a **grid** of `n` equal columns, with
    /// children placed automatically, row by row. `None` means flex.
    pub grid_columns: Option<usize>,
    /// When `Some(h)`, every grid row is exactly `h` logical pixels tall instead of
    /// following its content — the reference's per-tile main-axis extent. Ignored
    /// outside a grid.
    pub grid_row_height: Option<f32>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,
            flex_grow: 0.0,
            // **Not** flexbox's default of 1. The reference hands an inflexible child of
            // a row or a column an unbounded main axis: it is never squeezed, and a line
            // that does not fit overflows and says so. Flexbox would quietly take the
            // deficit out of every child in proportion, which is how a 40 px button came
            // to be laid out at 13 and drawn off the card (milestone 333) — silently,
            // for three milestones. A box gives way when it says it will, and the walk
            // grants it to a **lone** child, which is not dividing a line up but being
            // handed its parent's constraints.
            flex_shrink: 0.0,
            flex_basis: Dimension::Auto,
            flex_direction: FlexDirection::Row,
            justify: Justify::Start,
            align_content: AlignContent::default(),
            align: Align::Stretch,
            align_self: None,
            padding: Insets::ZERO,
            margin: Insets::ZERO,
            aspect_ratio: None,
            gap: 0.0,
            row_gap: None,
            column_gap: None,
            flex_wrap: false,
            grid_columns: None,
            grid_row_height: None,
        }
    }
}

impl Style {
    /// Mixes into `hasher` **every field that affects layout geometry**. Two styles
    /// with the same fingerprint produce the same arrangement — which is what makes
    /// a relayout cache possible, skipping taffy when nothing relevant has changed.
    /// The `f32`s are hashed by bit pattern, so equality is exact; colour and text do
    /// not enter into it, since they only affect painting.
    pub fn layout_hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        use std::hash::Hash;
        fn dim<H: std::hash::Hasher>(d: Dimension, h: &mut H) {
            match d {
                Dimension::Auto => 0u8.hash(h),
                Dimension::Length(v) => {
                    1u8.hash(h);
                    v.to_bits().hash(h);
                }
                Dimension::Percent(v) => {
                    2u8.hash(h);
                    v.to_bits().hash(h);
                }
            }
        }
        dim(self.width, hasher);
        dim(self.height, hasher);
        dim(self.min_width, hasher);
        dim(self.min_height, hasher);
        dim(self.max_width, hasher);
        dim(self.max_height, hasher);
        self.flex_grow.to_bits().hash(hasher);
        self.flex_shrink.to_bits().hash(hasher);
        dim(self.flex_basis, hasher);
        (self.flex_direction as u8).hash(hasher);
        (self.justify as u8).hash(hasher);
        (self.align_content as u8).hash(hasher);
        (self.align as u8).hash(hasher);
        match self.align_self {
            None => 0u8.hash(hasher),
            Some(a) => {
                1u8.hash(hasher);
                (a as u8).hash(hasher);
            }
        }
        self.padding.top.to_bits().hash(hasher);
        self.padding.right.to_bits().hash(hasher);
        self.padding.bottom.to_bits().hash(hasher);
        self.padding.left.to_bits().hash(hasher);
        self.margin.top.to_bits().hash(hasher);
        self.margin.right.to_bits().hash(hasher);
        self.margin.bottom.to_bits().hash(hasher);
        self.margin.left.to_bits().hash(hasher);
        match self.aspect_ratio {
            None => 0u8.hash(hasher),
            Some(r) => {
                1u8.hash(hasher);
                r.to_bits().hash(hasher);
            }
        }
        self.gap.to_bits().hash(hasher);
        self.row_gap.map(f32::to_bits).hash(hasher);
        self.column_gap.map(f32::to_bits).hash(hasher);
        self.flex_wrap.hash(hasher);
        self.grid_columns.hash(hasher);
        self.grid_row_height.map(f32::to_bits).hash(hasher);
    }

    pub(crate) fn to_taffy(self) -> taffy::Style {
        let mut style = taffy::Style {
            size: taffy::Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            min_size: taffy::Size {
                width: self.min_width.to_taffy(),
                height: self.min_height.to_taffy(),
            },
            max_size: taffy::Size {
                width: self.max_width.to_taffy(),
                height: self.max_height.to_taffy(),
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.to_taffy(),
            flex_direction: self.flex_direction.to_taffy(),
            flex_wrap: if self.flex_wrap {
                taffy::FlexWrap::Wrap
            } else {
                taffy::FlexWrap::NoWrap
            },
            justify_content: Some(self.justify.to_taffy()),
            align_content: Some(self.align_content.to_taffy()),
            align_items: Some(self.align.to_taffy()),
            align_self: self.align_self.map(Align::to_taffy),
            padding: taffy::Rect {
                left: taffy::LengthPercentage::Length(self.padding.left),
                right: taffy::LengthPercentage::Length(self.padding.right),
                top: taffy::LengthPercentage::Length(self.padding.top),
                bottom: taffy::LengthPercentage::Length(self.padding.bottom),
            },
            margin: taffy::Rect {
                left: taffy::LengthPercentageAuto::Length(self.margin.left),
                right: taffy::LengthPercentageAuto::Length(self.margin.right),
                top: taffy::LengthPercentageAuto::Length(self.margin.top),
                bottom: taffy::LengthPercentageAuto::Length(self.margin.bottom),
            },
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(self.column_gap.unwrap_or(self.gap)),
                height: taffy::LengthPercentage::Length(self.row_gap.unwrap_or(self.gap)),
            },
            aspect_ratio: self.aspect_ratio,
            ..Default::default()
        };

        // Grid: `n` equal columns of 1fr each; children are placed automatically,
        // row by row (auto-flow), with rows sized to their content.
        if let Some(columns) = self.grid_columns {
            use taffy::style_helpers::fr;
            style.display = taffy::Display::Grid;
            // The explicit `1.0_f32`: `fr` is generic, and the silent fallback from
            // `1.0` to `f32` is being withdrawn (future_incompatible, rust#154024).
            // Without the suffix this becomes a hard error.
            style.grid_template_columns = (0..columns).map(|_| fr(1.0_f32)).collect();
            // A fixed **tile height**: the rows the auto-flow creates are all this tall
            // instead of following their content, which is what makes a grid of tiles
            // read as a grid rather than as ragged bands.
            if let Some(height) = self.grid_row_height {
                style.grid_auto_rows = vec![taffy::style_helpers::length(height)];
            }
        }

        style
    }
}
