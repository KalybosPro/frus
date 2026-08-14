//! [`Table`]: a text data grid built from **`Flex` rows** (columns of **fixed or flexible**
//! width). A header that **sorts** on click (with a direction indicator), **selectable**
//! rows, and optional **multi-selection** through a checkbox column topped by a
//! "check all" box.
//!
//! As everywhere else in frus, sorting and selection are **decided by the application**: the
//! table emits a message on click (`on_sort`, `on_select_row`, `on_check`, `on_check_all`)
//! and displays nothing but the state it is handed (`sorted`, `selected`).

use std::rc::Rc;

use frus_core::{Color, Insets, Path, Point, Rect, Scene};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::flex::Flex;
use crate::icons::IconName;
use crate::interaction::{Key, KeyResponse, Status};
use crate::list::List;
use crate::scroll::{Axis, Scroll};
use crate::stack::Stack;
use crate::theme::Theme;
use crate::widget::{CellFn, Widget};

/// A column reordering: `(from, to, callback)`. The two indices bound keyboard
/// reordering; the callback turns a move into a message.
type ReorderSpec<Msg> = (usize, usize, Rc<dyn Fn(usize, usize) -> Msg>);

/// A row of widget cells, built on demand from the row index.
type RowWidgets<Msg> = Rc<dyn Fn(usize) -> Vec<Box<dyn Widget<Msg>>>>;

const ROW_H: f32 = 34.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 15.0;
/// Gap between rows and between cells (must match the geometry of the handles).
const ROW_GAP: f32 = 2.0;
/// Width of the checkbox column (multi-selection).
const CHECK_W: f32 = 40.0;
/// Side of the drawn checkbox.
const BOX: f32 = 18.0;
/// Width of a resize handle's **grab zone**.
const HANDLE_W: f32 = 8.0;

/// Shared background of a cell, by its role and the interaction (a shared factor).
fn cell_background(
    header: bool,
    selected: bool,
    clickable: bool,
    theme: &Theme,
    status: &Status,
) -> Color {
    let base = if header {
        theme.surface.lerp(theme.on_surface, 0.06)
    } else if selected {
        theme.surface.lerp(theme.primary, 0.16)
    } else {
        theme.surface
    };
    if clickable {
        theme.state_layer(base, theme.on_surface, status)
    } else {
        base
    }
}

/// Style of a cell: the column width (fixed or flexible), and an **adaptive** height — a
/// `ROW_H` floor (a minimum comfort) that grows with the content. Since the row aligns its
/// cells with `Stretch`, they all follow the tallest one (nothing is clipped).
fn cell_style(width: Dimension) -> Style {
    let flex_grow = if matches!(width, Dimension::Length(_)) {
        0.0
    } else {
        1.0
    };
    Style {
        width,
        height: Dimension::Auto,
        min_height: Dimension::Length(ROW_H),
        flex_grow,
        ..Default::default()
    }
}

/// Side of a header icon (a `24×24` grid, scaled).
const ICON: f32 = 16.0;
/// Gap between a header icon and its label.
const ICON_GAP: f32 = 6.0;

/// A text cell (a header or a data cell), themed at render time.
struct Cell<Msg> {
    label: String,
    width: Dimension,
    header: bool,
    selected: bool,
    /// Row index (on a data cell): used to announce "Row N selected" to the screen
    /// reader. `None` on a header.
    row: Option<usize>,
    /// **Leading** icon (headers only): painted before the label (icon + text).
    icon: Option<IconName>,
    /// The header's sort indicator: `Some(true)` = ▲, `Some(false)` = ▼.
    sort: Option<bool>,
    message: Option<Msg>,
    /// **Reorderable** header: `(column index, column count, the on_reorder(from, to)
    /// callback)`. The column count bounds keyboard reordering.
    reorder: Option<ReorderSpec<Msg>>,
    /// The header's **action widget** (0 or 1): a button (a filter, a menu…) placed on the
    /// right, a **child** of the cell. It captures its own click (deepest-first hit-test)
    /// while the rest of the header goes on sorting — hence a `Vec`, to expose it through
    /// [`children`](Widget::children).
    action: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for Cell<Msg> {
    fn style(&self) -> Style {
        let base = cell_style(self.width);
        // With an action widget, it sits on the **right** (the label goes on being painted
        // on the left) and is vertically centred.
        if self.action.is_empty() {
            base
        } else {
            Style {
                justify: Justify::End,
                align: Align::Center,
                padding: Insets::new(0.0, PAD_X, 0.0, 0.0),
                ..base
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.action
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        let bg = cell_background(self.header, self.selected, clickable, theme, &status);
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }

        let color = if self.header {
            theme.muted
        } else {
            theme.on_surface
        };
        let ty = bounds.y + (bounds.height - frus_text::line_height(SIZE)) * 0.5;

        // Leading icon (headers): painted on the left, the label shifted after it.
        let mut text_x = bounds.x + PAD_X;
        if let Some(icon) = self.icon {
            let iy = bounds.y + (bounds.height - ICON) * 0.5;
            let path = icon.path().scaled(ICON / 24.0).translated(text_x, iy);
            scene.fill_path(&path, color.fade(o));
            text_x += ICON + ICON_GAP;
        }
        scene.text(
            Point::new(text_x, ty),
            self.label.clone(),
            SIZE,
            color.fade(o),
        );

        if let (true, Some(ascending)) = (self.header, self.sort) {
            let lw = frus_text::measure(&self.label, SIZE).width;
            let cx = text_x + lw + 8.0;
            let cy = bounds.y + bounds.height * 0.5;
            let (w, h) = (4.0, 4.0);
            let tri = if ascending {
                Path::new()
                    .move_to(Point::new(cx, cy - h))
                    .line_to(Point::new(cx - w, cy + h))
                    .line_to(Point::new(cx + w, cy + h))
                    .close()
            } else {
                Path::new()
                    .move_to(Point::new(cx, cy + h))
                    .line_to(Point::new(cx - w, cy - h))
                    .line_to(Point::new(cx + w, cy - h))
                    .close()
            };
            scene.fill_path(&tri, theme.on_surface.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        // Only sortable headers take keyboard focus (Enter/Space = sort); data cells stay
        // clickable with the mouse without cluttering the Tab cycle.
        self.header && self.message.is_some()
    }

    fn reorder_index(&self) -> Option<usize> {
        self.reorder.as_ref().map(|(col, _, _)| *col)
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        self.reorder.as_ref().map(|(col, _, cb)| cb(*col, to))
    }

    fn announce(&self) -> Option<String> {
        // A sortable header: announces the **resulting** sort to the screen reader (it flips
        // the current direction: ascending by default, otherwise inverted — the usual pattern).
        if self.header && self.message.is_some() {
            let ascending = !matches!(self.sort, Some(true));
            let dir = if ascending { "ascending" } else { "descending" };
            return Some(format!("Sorted by {} {}", self.label, dir));
        }
        // A selectable data cell: announces the row's **resulting** state.
        if let (false, Some(row), true) = (self.header, self.row, self.message.is_some()) {
            let verb = if self.selected {
                "deselected"
            } else {
                "selected"
            };
            return Some(format!("Row {} {}", row + 1, verb));
        }
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // A header: announced to screen readers (label + position when it is reorderable, so
        // that a user perceives a column move when going through them again).
        if !self.header {
            return None;
        }
        let mut sem = frus_core::Semantics::new(frus_core::Role::Button).label(self.label.clone());
        if let Some((col, columns, _)) = &self.reorder {
            sem = sem.value(format!("column {} of {}", col + 1, columns));
        }
        Some(sem)
    }

    fn on_key(&self, key: &Key) -> KeyResponse<Msg> {
        // Ctrl+Arrows (Left/Right with `word`) on a focused header: moves the column by one
        // step (clamped). Bare arrows let the focus navigate instead.
        let Some((col, columns, cb)) = self.reorder.as_ref() else {
            return KeyResponse::Ignored;
        };
        let to = match key {
            Key::Left { word: true, .. } => col.checked_sub(1),
            Key::Right { word: true, .. } if col + 1 < *columns => Some(col + 1),
            _ => return KeyResponse::Ignored,
        };
        match to {
            Some(to) => KeyResponse::Handled(Some(cb(*col, to))),
            None => KeyResponse::Ignored, // at the edge: let the focus navigate
        }
    }
}

/// A checkbox cell (the multi-selection column).
struct CheckCell<Msg> {
    checked: bool,
    /// The **indeterminate** state (some rows checked, not all) — the "check all" box.
    /// It takes precedence over the unchecked look; ignored when `checked`.
    indeterminate: bool,
    header: bool,
    selected: bool,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for CheckCell<Msg> {
    fn style(&self) -> Style {
        cell_style(Dimension::Length(CHECK_W))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        let bg = cell_background(self.header, self.selected, clickable, theme, &status);
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }

        let bx = bounds.x + (bounds.width - BOX) * 0.5;
        let by = bounds.y + (bounds.height - BOX) * 0.5;
        let box_rect = Rect::new(bx, by, BOX, BOX);
        if self.checked {
            scene.draw_rect(
                box_rect,
                theme.primary.fade(o),
                4.0,
                0.0,
                Color::TRANSPARENT,
            );
            // The tick: the filled Check icon, centred in the box.
            let scale = (BOX - 4.0) / 24.0;
            let inset = (BOX - 24.0 * scale) * 0.5;
            let path = IconName::Check
                .path()
                .scaled(scale)
                .translated(bx + inset, by + inset);
            scene.fill_path(&path, theme.on_primary.fade(o));
        } else if self.indeterminate {
            // Indeterminate: a filled box crossed by a dash.
            scene.draw_rect(
                box_rect,
                theme.primary.fade(o),
                4.0,
                0.0,
                Color::TRANSPARENT,
            );
            let dash = Rect::new(bx + 4.0, by + BOX * 0.5 - 1.0, BOX - 8.0, 2.0);
            scene.draw_rect(dash, theme.on_primary.fade(o), 1.0, 0.0, Color::TRANSPARENT);
        } else {
            scene.draw_rect(box_rect, Color::TRANSPARENT, 4.0, 1.5, theme.muted.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }

    fn announce(&self) -> Option<String> {
        // A checkbox: announces the **resulting** state of the toggle (checked → it will be
        // unchecked). The header's box acts on **every** row.
        self.message.as_ref()?;
        let selecting = !self.checked;
        Some(match (self.header, selecting) {
            (true, true) => "All rows selected".into(),
            (true, false) => "All rows deselected".into(),
            (false, true) => "Row selected".into(),
            (false, false) => "Row deselected".into(),
        })
    }
}

/// Factory of a cell widget: **called again on every rebuild** (the table rebuilds itself
/// after each setting), so it produces a **fresh** widget every time. This is what allows
/// rich cells (avatars, chips, action buttons) beyond plain text.
pub type CellFactory<Msg> = std::rc::Rc<dyn Fn() -> Box<dyn Widget<Msg>>>;

/// Content of a data row: plain text, or widgets (one per column).
enum RowKind<Msg> {
    Text(Vec<String>),
    Widgets(Vec<CellFactory<Msg>>),
}

/// Factory for the content of a **virtualised** row, by index: texts (one value per column)
/// or widgets (one per column). Shared (`Rc`) so it can be captured in the virtualised
/// list's `'static` factory.
#[derive(Clone)]
enum VirtualBuild<Msg> {
    Text(std::rc::Rc<dyn Fn(usize) -> Vec<String>>),
    Widgets(RowWidgets<Msg>),
}

/// A **widget** cell: arbitrary content (centred, with the cell background and, when the row
/// is selectable, the selection click). The content paints on top.
struct WidgetCell<Msg> {
    width: Dimension,
    selected: bool,
    /// A **header** cell (with the header background) rather than a data cell.
    header: bool,
    /// Row index: to announce "Row N selected" to the screen reader.
    row: usize,
    message: Option<Msg>,
    content: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for WidgetCell<Msg> {
    fn style(&self) -> Style {
        Style {
            align: Align::Center,
            padding: Insets::new(0.0, PAD_X, 0.0, PAD_X),
            ..cell_style(self.width)
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        let bg = cell_background(self.header, self.selected, clickable, theme, &status);
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn announce(&self) -> Option<String> {
        // A selectable widget row: announces the selection's **resulting** state.
        self.message.as_ref()?;
        let verb = if self.selected {
            "deselected"
        } else {
            "selected"
        };
        Some(format!("Row {} {}", self.row + 1, verb))
    }
}

/// Width of a frozen column's **separation shadow** gradient.
const FROZEN_SHADOW_W: f32 = 8.0;

/// The frozen columns' **separation shadow** layer: a gradient (scrim → transparent) placed
/// at the inner edge of the pinned block, over the scrolling area — the visual cue of the
/// freeze. **Inert** (it captures no click): the cells beneath stay clickable.
struct FrozenShadow {
    /// Size of the layer (it fills the stack) — otherwise `Auto` would shrink it to 0×0.
    width: f32,
    height: f32,
    /// x of the right edge of the **left** pinned block (the shadow runs rightwards).
    left: Option<f32>,
    /// x of the left edge of the **right** pinned block (the shadow runs leftwards).
    right: Option<f32>,
}

impl<Msg: Clone> Widget<Msg> for FrozenShadow {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let dark = theme.scheme.scrim.with_alpha(0.28 * o);
        let clear = theme.scheme.scrim.with_alpha(0.0);
        if let Some(x) = self.left {
            scene.gradient_rect(
                Rect::new(bounds.x + x, bounds.y, FROZEN_SHADOW_W, bounds.height),
                dark,
                clear,
                [1.0, 0.0],
                0.0,
                0.0,
                Color::TRANSPARENT,
            );
        }
        if let Some(x) = self.right {
            scene.gradient_rect(
                Rect::new(
                    bounds.x + x - FROZEN_SHADOW_W,
                    bounds.y,
                    FROZEN_SHADOW_W,
                    bounds.height,
                ),
                clear,
                dark,
                [1.0, 0.0],
                0.0,
                0.0,
                Color::TRANSPARENT,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A transparent, **inert** space (neither clickable nor draggable): it wedges the resize
/// handles onto the column edges without blocking clicks (the handle layer floats above
/// the grid).
struct Spacer {
    width: f32,
    height: f32,
}

impl<Msg: Clone> Widget<Msg> for Spacer {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A column **resize handle**: a thin vertical bar at a column's right edge. Dragged
/// horizontally, it emits `on_resize(col, dx)` where `dx` is the movement (px) since the last
/// event — the application **accumulates** the width. The handle layer floats above the grid.
struct ResizeHandle<Msg> {
    col: usize,
    height: f32,
    on_resize: Rc<dyn Fn(usize, f32) -> Msg>,
}

impl<Msg: Clone> Widget<Msg> for ResizeHandle<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(HANDLE_W),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A centred grab line: discreet at rest, tinted `primary` and thickened on hover /
        // while being dragged.
        let t = status.hover_progress;
        let color = theme.border.lerp(theme.primary, t);
        let lw = 1.0 + t; // 1 px → 2 px
        let x = bounds.x + (HANDLE_W - lw) * 0.5;
        scene.draw_rect(
            Rect::new(x, bounds.y, lw, bounds.height),
            color.fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn draggable(&self) -> bool {
        true
    }

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        // A zero delta (a press, or purely vertical movement) changes nothing.
        if dx == 0.0 {
            None
        } else {
            Some((self.on_resize)(self.col, dx))
        }
    }
}

/// A data grid with fixed or flexible columns (see the module).
pub struct Table<Msg> {
    columns: usize,
    headers: Vec<String>,
    /// Leading icon per header column (icon + label). Missing = none.
    header_icons: Vec<Option<IconName>>,
    /// A **fully widget** header (per column): it replaces the text header row. Empty = the
    /// ordinary text header. Automatic sorting/reordering does not apply to these headers —
    /// the application wires the behaviour into its own widgets.
    header_widgets: Vec<CellFactory<Msg>>,
    /// Action widget per header column (a filter/menu button), built again on every rebuild.
    /// `None` = none. Placed on the right of the header, it captures its own click.
    header_actions: Vec<Option<CellFactory<Msg>>>,
    rows: Vec<RowKind<Msg>>,
    /// Width per column: `> 0` = fixed (px), `<= 0` = flexible (an equal share).
    widths: Vec<f32>,
    total_width: Option<f32>,
    sort: Option<(usize, bool)>,
    selected: Vec<usize>,
    on_sort: Option<Box<dyn Fn(usize) -> Msg>>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    on_check: Option<Rc<dyn Fn(usize) -> Msg>>,
    on_check_all: Option<Msg>,
    /// Resize callback (column, delta px). Only active when every column has a **fixed**
    /// width (the geometry of the edges is then known).
    on_resize: Option<Rc<dyn Fn(usize, f32) -> Msg>>,
    /// Reorder callback (`on_reorder(from, to)`): dragging a header onto another moves the
    /// column. Also requires `on_sort` (clickable headers).
    on_reorder: Option<Rc<dyn Fn(usize, usize) -> Msg>>,
    /// **Virtualised** mode: `(row count, viewport height, row factory by index — texts or
    /// widgets)`. Only the **visible** rows are built (internal scrolling), for grids of
    /// thousands of rows. Excludes `rows`.
    virtual_data: Option<(usize, f32, VirtualBuild<Msg>)>,
    /// **Frozen** columns `(left, right)`: pinned at both edges while the middle scrolls
    /// horizontally. Requires a total width and columns that are all **fixed**.
    frozen: (usize, usize),
    root: Box<dyn Widget<Msg>>,
}

/// Dimension of a column from the widths: fixed when `> 0`, flexible otherwise (a factor
/// shared by the direct build and the virtualised one).
fn col_dimension(widths: &[f32], c: usize) -> Dimension {
    match widths.get(c).copied().unwrap_or(0.0) {
        w if w > 0.0 => Dimension::Length(w),
        _ => Dimension::Auto,
    }
}

impl<Msg: Clone + 'static> Table<Msg> {
    /// Creates a table of `columns` columns (flexible, of equal width by default).
    pub fn new(columns: usize) -> Self {
        let columns = columns.max(1);
        Self {
            columns,
            headers: Vec::new(),
            header_icons: Vec::new(),
            header_widgets: Vec::new(),
            header_actions: Vec::new(),
            rows: Vec::new(),
            widths: vec![0.0; columns],
            total_width: None,
            sort: None,
            selected: Vec::new(),
            on_sort: None,
            on_select: None,
            on_check: None,
            on_check_all: None,
            on_resize: None,
            on_reorder: None,
            virtual_data: None,
            frozen: (0, 0),
            root: Box::new(Flex::<Msg>::column().gap(ROW_GAP)),
        }
    }

    /// Sets the header row (one label per column).
    pub fn header(mut self, labels: &[&str]) -> Self {
        self.headers = labels.iter().map(|s| s.to_string()).collect();
        self.rebuild();
        self
    }

    /// Replaces the header row with **fully widget headers** (one per column) — for heavily
    /// customised grids (a bespoke sort button, an embedded filter, a two-line title…).
    /// **Automatic** sorting and reordering do not apply here: the application wires the
    /// behaviour into the widgets it supplies (e.g. a button emitting its own sort message).
    /// Each factory is called again on every rebuild (a fresh widget). Excludes
    /// [`header`](Self::header) (the last one called wins).
    pub fn widget_header(mut self, cells: Vec<CellFn<Msg>>) -> Self {
        self.header_widgets = cells.into_iter().map(std::rc::Rc::from).collect();
        self.headers.clear();
        self.rebuild();
        self
    }

    /// Gives header columns a **leading icon** (icon + label): `None` leaves the column
    /// without one. The header stays **sortable** and **reorderable** just like a text
    /// header (the icon is purely decorative).
    pub fn header_icons(mut self, icons: &[Option<IconName>]) -> Self {
        self.header_icons = icons.to_vec();
        self.rebuild();
        self
    }

    /// Places an **action widget** (a filter button, a menu…) on the right of column `col`'s
    /// header. It is a **child** of the cell: it captures its own click (deepest-first
    /// hit-test), while the rest of the header **sorts** and **reorders** as usual. The
    /// factory is called again on every rebuild (a fresh widget).
    ///
    /// **Column menu**: pass a [`Menu`](crate::Menu) or a [`Dropdown`](crate::Dropdown) here.
    /// Its **floating** menu is rendered even when **nested** inside the header (overlays are
    /// collected at any depth), reachable with **Tab** and driven by the arrows/Enter, and
    /// closed by Escape / an outside click — with no table-specific code (the application
    /// drives the menu's open/closed state).
    pub fn header_action(
        mut self,
        col: usize,
        make: impl Fn() -> Box<dyn Widget<Msg>> + 'static,
    ) -> Self {
        if col < self.columns {
            if self.header_actions.len() <= col {
                self.header_actions.resize_with(col + 1, || None);
            }
            self.header_actions[col] = Some(std::rc::Rc::new(make));
        }
        self.rebuild();
        self
    }

    /// Adds a data row (one **text** value per column).
    pub fn row(mut self, cells: &[&str]) -> Self {
        self.rows
            .push(RowKind::Text(cells.iter().map(|s| s.to_string()).collect()));
        self.rebuild();
        self
    }

    /// Adds a row whose every cell is a **widget** (an avatar, a chip, an action button…),
    /// supplied by a **factory** called again on every rebuild. The row stays selectable
    /// (a background on click, outside the internal clickable zones).
    ///
    /// **Sorting a widget column**: the table cannot compare widgets — it only emits the
    /// column that was clicked (`on_sort`). The **application** supplies the key: on the sort
    /// message it orders its own data by the field matching that column (e.g. the name behind
    /// an avatar), then hands the already-sorted rows back.
    pub fn widget_row(mut self, cells: Vec<CellFn<Msg>>) -> Self {
        self.rows.push(RowKind::Widgets(
            cells.into_iter().map(std::rc::Rc::from).collect(),
        ));
        self.rebuild();
        self
    }

    /// Sets the table's total width, in logical pixels (the flexible columns share what is
    /// left).
    pub fn width(mut self, width: f32) -> Self {
        self.total_width = Some(width);
        self.rebuild();
        self
    }

    /// **Fixed** width of each column, in pixels (`0` or less = a flexible column). Missing
    /// entries leave the column flexible.
    pub fn column_widths(mut self, widths: &[f32]) -> Self {
        for (i, w) in widths.iter().enumerate().take(self.columns) {
            self.widths[i] = *w;
        }
        self.rebuild();
        self
    }

    /// Makes the headers **clickable**: `on_sort(column)` on a click on a header.
    pub fn on_sort(mut self, on_sort: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_sort = Some(Box::new(on_sort));
        self.rebuild();
        self
    }

    /// States the sorted column and its direction (`true` = ascending) → shows the indicator.
    pub fn sorted(mut self, column: usize, ascending: bool) -> Self {
        self.sort = Some((column, ascending));
        self.rebuild();
        self
    }

    /// Makes the rows **clickable**: `on_select_row(row)` on a click on a row.
    pub fn on_select_row(mut self, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self.rebuild();
        self
    }

    /// Turns on **multi-selection**: a checkbox column (on the left) topped by a "check all"
    /// box. `on_check(row)` toggles one row, `on_check_all` toggles every row. The checked
    /// state mirrors [`selected`](Self::selected).
    pub fn checkboxes(
        mut self,
        on_check: impl Fn(usize) -> Msg + 'static,
        on_check_all: Msg,
    ) -> Self {
        self.on_check = Some(Rc::new(on_check));
        self.on_check_all = Some(on_check_all);
        self.rebuild();
        self
    }

    /// **Freezes** the first `n` columns: they stay **pinned** on the left while the rest
    /// **scrolls horizontally** (the scrolling columns' header follows its columns), with a
    /// **separation shadow** at the freeze edge. For wide grids where an identifier must stay
    /// in view. Requires a total [`width`](Self::width) and columns that are **all fixed**
    /// ([`column_widths`](Self::column_widths)) — otherwise it does nothing. **Text** tables
    /// only; not combined with virtualisation / checkboxes / resizing / reordering.
    pub fn frozen_columns(mut self, n: usize) -> Self {
        self.frozen.0 = n;
        self.rebuild();
        self
    }

    /// **Freezes** the **last** `m` columns (pinned on the **right** — action columns,
    /// totals…), the middle scrolling. Combines with
    /// [`frozen_columns`](Self::frozen_columns) (both edges frozen). Same conditions and
    /// exclusions.
    pub fn frozen_columns_right(mut self, m: usize) -> Self {
        self.frozen.1 = m;
        self.rebuild();
        self
    }

    /// Switches the table to **virtualised** mode: `count` data rows, of which only the
    /// **visible** ones are built/laid out/painted (internal vertical scrolling inside a
    /// viewport of `viewport_height` px). `build(index)` supplies the row's **texts** (one
    /// value per column). Indispensable for large grids (thousands of rows): the per-frame
    /// cost is ∝ the visible rows, not the total. The header stays **pinned**.
    ///
    /// Selection: [`on_select_row`](Self::on_select_row) and [`selected`](Self::selected) work
    /// on the visible rows; **multi-selection** ([`checkboxes`](Self::checkboxes)) is
    /// supported too (a checkbox column + a pinned "check all"). Not combined with resizing /
    /// reordering — they are ignored in virtualised mode. Excludes [`row`](Self::row).
    pub fn virtual_rows(
        mut self,
        count: usize,
        viewport_height: f32,
        build: impl Fn(usize) -> Vec<String> + 'static,
    ) -> Self {
        self.virtual_data = Some((count, viewport_height, VirtualBuild::Text(Rc::new(build))));
        self.rows.clear();
        self.rebuild();
        self
    }

    /// Like [`virtual_rows`](Self::virtual_rows), but every row is made of **widgets**
    /// (avatars, chips, buttons…): `build(index)` returns one widget per column. Only the
    /// visible rows are built. Click selection **and** checkboxes; the header stays pinned.
    /// Same exclusions (resizing / reordering).
    pub fn virtual_widget_rows(
        mut self,
        count: usize,
        viewport_height: f32,
        build: impl Fn(usize) -> Vec<Box<dyn Widget<Msg>>> + 'static,
    ) -> Self {
        self.virtual_data = Some((
            count,
            viewport_height,
            VirtualBuild::Widgets(Rc::new(build)),
        ));
        self.rows.clear();
        self.rebuild();
        self
    }

    /// Makes the columns **resizable** with the mouse: a thin handle at each column's right
    /// edge (except the last) emits `on_resize(column, delta_px)` when dragged. The
    /// application **accumulates** the width, e.g.
    /// `widths[col] = (widths[col] + delta).max(MIN)`, and hands it back through
    /// [`column_widths`](Self::column_widths). Only has an effect when **every** column has a
    /// fixed width (so the edges are known).
    pub fn on_resize(mut self, on_resize: impl Fn(usize, f32) -> Msg + 'static) -> Self {
        self.on_resize = Some(Rc::new(on_resize));
        self.rebuild();
        self
    }

    /// Makes the columns **reorderable**: dragging a header (past the threshold) and dropping
    /// it on another emits `on_reorder(from, to)`; the application permutes its own column
    /// order. A plain **click** still sorts (`on_sort`). Does nothing when the headers are
    /// not clickable.
    pub fn on_reorder(mut self, on_reorder: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.on_reorder = Some(Rc::new(on_reorder));
        self.rebuild();
        self
    }

    /// States the selected rows (highlighted, their boxes checked).
    pub fn selected(mut self, rows: &[usize]) -> Self {
        self.selected = rows.to_vec();
        self.rebuild();
        self
    }

    /// Dimension of column `c`: fixed when `widths[c] > 0`, flexible otherwise.
    fn col_width(&self, c: usize) -> Dimension {
        col_dimension(&self.widths, c)
    }

    /// One `Flex` row, at the table's total width when one is set.
    fn new_row(&self) -> Flex<Msg> {
        let row = Flex::row().gap(ROW_GAP);
        match self.total_width {
            Some(w) => row.width(w),
            None => row,
        }
    }

    /// Number of data rows: the **virtualised** count when one is set, otherwise the
    /// materialised rows. (In virtualised mode, `rows` is empty.)
    fn row_count(&self) -> usize {
        self.virtual_data
            .as_ref()
            .map(|(count, _, _)| *count)
            .unwrap_or(self.rows.len())
    }

    /// Number of selected rows **within range** (indices `< row_count`). Assumes unique, valid
    /// indices ([`selected`](Self::selected)'s contract) — O(selection), so it stays viable
    /// even for a virtualised grid of millions of rows.
    fn selected_count(&self) -> usize {
        let n = self.row_count();
        self.selected.iter().filter(|&&r| r < n).count()
    }

    /// True when every row is selected (for the "check all" box).
    fn all_selected(&self) -> bool {
        let n = self.row_count();
        n > 0 && self.selected_count() == n
    }

    /// True when **some** rows (not all) are selected → an indeterminate "check all".
    fn some_selected(&self) -> bool {
        let s = self.selected_count();
        s > 0 && s < self.row_count()
    }

    /// The handle layer (one per column edge, except the last), to be laid over the grid.
    /// `None` when the table is not resizable, or when the columns are not all fixed.
    fn resize_overlay(&self, total_h: f32) -> Option<Flex<Msg>> {
        let on_resize = self.on_resize.as_ref()?;
        // The edges are only known when every column is fixed.
        if !(0..self.columns).all(|c| self.widths.get(c).copied().unwrap_or(0.0) > 0.0) {
            return None;
        }
        let base = if self.on_check.is_some() {
            CHECK_W + ROW_GAP
        } else {
            0.0
        };
        // A row with no gap: the spacing is materialised by spacers.
        let mut row = Flex::row();
        let mut consumed = 0.0f32;
        let mut edge = base;
        for c in 0..self.columns {
            edge += self.widths[c]; // right edge of column c
            if c + 1 < self.columns {
                let handle_left = edge - HANDLE_W * 0.5;
                row = row
                    .child(Spacer {
                        width: (handle_left - consumed).max(0.0),
                        height: total_h,
                    })
                    .child(ResizeHandle {
                        col: c,
                        height: total_h,
                        on_resize: on_resize.clone(),
                    });
                consumed = handle_left + HANDLE_W;
            }
            edge += ROW_GAP; // the inter-column gap
        }
        Some(row)
    }

    /// Header cell for **frozen columns** mode (label + icon + sorting, with no action and
    /// no reordering).
    fn frozen_header_cell(&self, c: usize) -> Cell<Msg> {
        let sort = self.sort.filter(|(col, _)| *col == c).map(|(_, asc)| asc);
        Cell {
            label: self.headers.get(c).cloned().unwrap_or_default(),
            width: self.col_width(c),
            header: true,
            selected: false,
            row: None,
            icon: self.header_icons.get(c).copied().flatten(),
            sort,
            message: self.on_sort.as_ref().map(|f| f(c)),
            reorder: None,
            action: Vec::new(),
        }
    }

    /// Builds a **block of columns** (header + text rows) for the columns `cols`, at width
    /// `w`. Assumes **text** rows (validated upstream).
    fn frozen_block(
        &self,
        cols: std::ops::Range<usize>,
        w: f32,
        header_present: bool,
    ) -> Flex<Msg> {
        let mut col = Flex::column().gap(ROW_GAP).width(w);
        if header_present {
            let mut hr = Flex::row().gap(ROW_GAP).width(w);
            for c in cols.clone() {
                hr = hr.child(self.frozen_header_cell(c));
            }
            col = col.child(hr);
        }
        for (r, row) in self.rows.iter().enumerate() {
            let RowKind::Text(cells) = row else { continue };
            let selected = self.selected.contains(&r);
            let mut rr = Flex::row().gap(ROW_GAP).width(w);
            for c in cols.clone() {
                rr = rr.child(Cell {
                    label: cells.get(c).cloned().unwrap_or_default(),
                    width: self.col_width(c),
                    header: false,
                    selected,
                    row: Some(r),
                    icon: None,
                    sort: None,
                    message: self.on_select.as_ref().map(|f| f(r)),
                    reorder: None,
                    action: Vec::new(),
                });
            }
            col = col.child(rr);
        }
        col
    }

    /// Builds the **frozen columns** layout: a pinned block on the left and/or the right, the
    /// middle inside a horizontal scroll (header included), with a **separation shadow** at
    /// each freeze edge. `None` when the conditions are not met (no total width, columns not
    /// all fixed, counts out of range, or an unsupported combination: virtualisation /
    /// checkboxes / widget rows) → it falls back to the ordinary layout.
    fn build_frozen(&self) -> Option<Box<dyn Widget<Msg>>> {
        let (left, right) = self.frozen;
        if (left == 0 && right == 0) || self.virtual_data.is_some() || self.on_check.is_some() {
            return None;
        }
        if left + right >= self.columns {
            return None;
        }
        let total_width = self.total_width?;
        if !(0..self.columns).all(|c| self.widths.get(c).copied().unwrap_or(0.0) > 0.0) {
            return None;
        }
        // Frozen columns: text only (off-screen content is not retained).
        if self.rows.iter().any(|r| !matches!(r, RowKind::Text(_))) {
            return None;
        }

        let span = |a: usize, b: usize| -> f32 {
            if b <= a {
                return 0.0;
            }
            let cols: f32 = (a..b).map(|c| self.widths[c]).sum();
            cols + ROW_GAP * ((b - a) - 1) as f32
        };
        let mid_end = self.columns - right;
        let (left_w, mid_w, right_w) = (
            span(0, left),
            span(left, mid_end),
            span(mid_end, self.columns),
        );

        let header_present = !self.headers.is_empty();
        let n_rows = header_present as usize + self.rows.len();
        let total_h = if n_rows > 0 {
            n_rows as f32 * ROW_H + (n_rows as f32 - 1.0) * ROW_GAP
        } else {
            0.0
        };

        // Gaps between the blocks that are present (the scrolling middle is always there).
        let gaps = (left > 0) as usize + (right > 0) as usize;
        let viewport_w = (total_width - left_w - right_w - gaps as f32 * ROW_GAP).max(0.0);

        let mut row = Flex::row().gap(ROW_GAP).width(total_width);
        if left > 0 {
            row = row.child(self.frozen_block(0..left, left_w, header_present));
        }
        row = row.child(
            Scroll::new()
                .axis(Axis::Horizontal)
                .width(viewport_w)
                .height(total_h)
                .child(self.frozen_block(left..mid_end, mid_w, header_present)),
        );
        if right > 0 {
            row = row.child(self.frozen_block(mid_end..self.columns, right_w, header_present));
        }

        // A separation shadow at each pinned block's inner edge (over the scrolling part).
        let shadow = FrozenShadow {
            width: total_width,
            height: total_h,
            left: (left > 0).then_some(left_w),
            right: (right > 0).then_some(total_width - right_w),
        };
        Some(Box::new(
            Stack::new()
                .width(total_width)
                .height(total_h)
                .layer(row)
                .layer(shadow),
        ))
    }

    fn rebuild(&mut self) {
        // Frozen columns: a dedicated layout (a pinned block + horizontal scrolling for the rest).
        if let Some(root) = self.build_frozen() {
            self.root = root;
            return;
        }

        let checks = self.on_check.is_some();
        let mut col = Flex::column().gap(ROW_GAP);

        // The header row (when there are text labels, widget headers, or checkboxes).
        let widget_headers = !self.header_widgets.is_empty();
        if !self.headers.is_empty() || widget_headers || checks {
            let mut hrow = self.new_row();
            if checks {
                hrow = hrow.child(CheckCell {
                    checked: self.all_selected(),
                    indeterminate: self.some_selected(),
                    header: true,
                    selected: false,
                    message: self.on_check_all.clone(),
                });
            }
            if widget_headers {
                // Fully widget headers: each cell hosts the widget supplied (header
                // background, centred content). Sorting/reordering are wired by the app.
                for (c, make) in self.header_widgets.iter().enumerate() {
                    hrow = hrow.child(WidgetCell {
                        width: self.col_width(c),
                        selected: false,
                        header: true,
                        row: 0,
                        message: None,
                        content: vec![make()],
                    });
                }
            } else {
                for (c, label) in self.headers.iter().enumerate() {
                    let sort = self.sort.filter(|(col, _)| *col == c).map(|(_, asc)| asc);
                    let message = self.on_sort.as_ref().map(|f| f(c));
                    let reorder = self
                        .on_reorder
                        .as_ref()
                        .map(|cb| (c, self.columns, cb.clone()));
                    let action = self
                        .header_actions
                        .get(c)
                        .and_then(|a| a.as_ref())
                        .map(|make| vec![make()])
                        .unwrap_or_default();
                    hrow = hrow.child(Cell {
                        label: label.clone(),
                        width: self.col_width(c),
                        header: true,
                        selected: false,
                        row: None,
                        icon: self.header_icons.get(c).copied().flatten(),
                        sort,
                        message,
                        reorder,
                        action,
                    });
                }
            }
            col = col.child(hrow);
        }

        // Virtualised mode: the pinned header above a **virtualised list** of data rows (only
        // the visible ones are built). Every parameter a row needs is **captured** (cloned)
        // into the list's factory, which stays `'static` — so the closure cannot reach `self`.
        if let Some((count, viewport_height, build)) = self.virtual_data.clone() {
            let columns = self.columns;
            let widths = self.widths.clone();
            let total_width = self.total_width;
            let selected = self.selected.clone();
            let on_select = self.on_select.clone();
            let on_check = self.on_check.clone();
            let list = List::new(count, ROW_H, move |i| {
                let is_selected = selected.contains(&i);
                let message = on_select.as_ref().map(|f| f(i));
                let mut row = Flex::row().gap(ROW_GAP);
                if let Some(w) = total_width {
                    row = row.width(w);
                }
                // The multi-selection column (like the pinned "check all" header).
                if let Some(check) = &on_check {
                    row = row.child(CheckCell {
                        checked: is_selected,
                        indeterminate: false,
                        header: false,
                        selected: is_selected,
                        message: Some(check(i)),
                    });
                }
                match &build {
                    VirtualBuild::Text(f) => {
                        let cells = f(i);
                        for c in 0..columns {
                            row = row.child(Cell {
                                label: cells.get(c).cloned().unwrap_or_default(),
                                width: col_dimension(&widths, c),
                                header: false,
                                selected: is_selected,
                                row: Some(i),
                                icon: None,
                                sort: None,
                                message: message.clone(),
                                reorder: None,
                                action: Vec::new(),
                            });
                        }
                    }
                    VirtualBuild::Widgets(f) => {
                        for (c, widget) in f(i).into_iter().enumerate().take(columns) {
                            row = row.child(WidgetCell {
                                width: col_dimension(&widths, c),
                                selected: is_selected,
                                header: false,
                                row: i,
                                message: message.clone(),
                                content: vec![widget],
                            });
                        }
                    }
                }
                row
            });
            let list = match total_width {
                Some(w) => list.width(w).height(viewport_height),
                None => list.height(viewport_height),
            };
            col = col.child(list);
            self.root = Box::new(col);
            return;
        }

        // The data rows.
        for (r, row) in self.rows.iter().enumerate() {
            let selected = self.selected.contains(&r);
            let mut drow = self.new_row();
            if checks {
                drow = drow.child(CheckCell {
                    checked: selected,
                    indeterminate: false,
                    header: false,
                    selected,
                    message: self.on_check.as_ref().map(|f| f(r)),
                });
            }
            match row {
                RowKind::Text(cells) => {
                    for (c, label) in cells.iter().enumerate() {
                        let message = self.on_select.as_ref().map(|f| f(r));
                        drow = drow.child(Cell {
                            label: label.clone(),
                            width: self.col_width(c),
                            header: false,
                            selected,
                            row: Some(r),
                            icon: None,
                            sort: None,
                            message,
                            reorder: None,
                            action: Vec::new(),
                        });
                    }
                }
                RowKind::Widgets(factories) => {
                    for (c, make) in factories.iter().enumerate() {
                        drow = drow.child(WidgetCell {
                            width: self.col_width(c),
                            selected,
                            header: false,
                            row: r,
                            message: self.on_select.as_ref().map(|f| f(r)),
                            content: vec![make()],
                        });
                    }
                }
            }
            col = col.child(drow);
        }

        // Total height of the grid (rows + gaps): to size the handle layer, which runs the
        // whole height. Based on `ROW_H` (the adaptive floor): exact for a text table — which
        // is the resizing case, where every column is fixed; a taller widget row makes it an
        // under-estimate.
        let header_present = !self.headers.is_empty() || widget_headers || checks;
        let n = header_present as usize + self.rows.len();
        let total_h = if n > 0 {
            n as f32 * ROW_H + (n as f32 - 1.0) * ROW_GAP
        } else {
            0.0
        };

        self.root = match self.resize_overlay(total_h) {
            Some(handles) => {
                let base = if checks { CHECK_W + ROW_GAP } else { 0.0 };
                let cols_w: f32 = (0..self.columns).map(|c| self.widths[c]).sum();
                let computed = base + cols_w + ROW_GAP * self.columns.saturating_sub(1) as f32;
                let total_w = self.total_width.unwrap_or(computed);
                Box::new(
                    Stack::new()
                        .width(total_w)
                        .height(total_h)
                        .layer(col)
                        .layer(handles),
                )
            }
            None => Box::new(col),
        };
    }
}

impl<Msg: Clone> Widget<Msg> for Table<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style(&self.root)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        Widget::<Msg>::children(&self.root)
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        // When the table is resizable the root is a stack (the grid + the handle layer): the
        // flag is relayed so the layers superimpose instead of lining up.
        Widget::<Msg>::stack(&self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::{Point, Primitive};

    #[test]
    fn header_and_rows_produce_rows_of_cells() {
        let table = Table::<()>::new(2)
            .header(&["Nom", "Note"])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // 1 header + 2 data rows.
        assert_eq!(Widget::<()>::children(&table).len(), 3);
        // Each row has 2 cells.
        assert_eq!(Widget::<()>::children(&table)[0].children().len(), 2);

        let ui = build_ui(
            &table,
            Size::new(240.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Nom") && has("Ada") && has("Bob"));
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Sort(usize),
        Select(usize),
        Check(usize),
        CheckAll,
        Resize(usize, f32),
        Reorder(usize, usize),
        Filter,
    }

    #[test]
    fn header_click_sorts_and_row_click_selects() {
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .on_select_row(Msg::Select)
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        let ui = build_ui(
            &table,
            Size::new(240.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        assert_eq!(click(180.0, ROW_H * 0.5), Some(Msg::Sort(1)));
        assert_eq!(click(30.0, ROW_H * 2.5), Some(Msg::Select(1)));
    }

    #[test]
    fn checkbox_column_toggles_rows_and_all() {
        let table = Table::<Msg>::new(2)
            .width(280.0)
            .header(&["Name", "Score"])
            .checkboxes(Msg::Check, Msg::CheckAll)
            .selected(&[0])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // Each row has 3 cells: the checkbox + 2 columns.
        assert_eq!(Widget::<Msg>::children(&table)[0].children().len(), 3);

        let ui = build_ui(
            &table,
            Size::new(280.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        // The "check all" box in the header (the left column).
        assert_eq!(click(CHECK_W * 0.5, ROW_H * 0.5), Some(Msg::CheckAll));
        // The box on the 2nd data row (r=1).
        assert_eq!(click(CHECK_W * 0.5, ROW_H * 2.5), Some(Msg::Check(1)));
    }

    #[test]
    fn select_all_is_indeterminate_on_partial_selection() {
        // The header's "check all" box = the 1st cell of the 1st row.
        let header_check = |sel: &[usize]| {
            let table = Table::<Msg>::new(2)
                .header(&["A", "B"])
                .checkboxes(Msg::Check, Msg::CheckAll)
                .selected(sel)
                .row(&["x", "1"])
                .row(&["y", "2"]);
            let row0 = &Widget::<Msg>::children(&table)[0];
            // Painting the cell to read its state from the primitives would be heavy; the
            // helpers are tested directly instead.
            let _ = row0;
            (table.all_selected(), table.some_selected())
        };
        assert_eq!(header_check(&[]), (false, false), "nothing checked");
        assert_eq!(header_check(&[0]), (false, true), "partial → indeterminate");
        assert_eq!(header_check(&[0, 1]), (true, false), "all checked");
    }

    #[test]
    fn only_headers_take_keyboard_focus() {
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .on_select_row(Msg::Select)
            .row(&["Ada", "5"]);
        let ui = build_ui(
            &table,
            Size::new(240.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Two sortable headers are focusable; the data cells are not. The focusables are
        // counted by walking the Tab cycle.
        let first = ui.focus_next(None, true);
        let mut count = 0;
        let mut cur = first;
        while let Some(id) = cur {
            count += 1;
            let next = ui.focus_next(Some(id), true);
            if next == first || count > 10 {
                break;
            }
            cur = next;
        }
        assert_eq!(count, 2, "only the 2 headers take focus (got {count})");
    }

    #[test]
    fn resize_handle_emits_accumulating_delta() {
        let table = Table::<Msg>::new(2)
            .column_widths(&[100.0, 100.0])
            .header(&["A", "B"])
            .on_resize(Msg::Resize)
            .row(&["x", "y"]);
        // Root = a stack; layer 1 is the row of handles.
        assert_eq!(Widget::<Msg>::children(&table).len(), 2);
        let overlay = &Widget::<Msg>::children(&table)[1];
        // A single handle (the edge between the 2 columns) among the spacers.
        let handle = overlay
            .children()
            .iter()
            .find(|c| c.draggable())
            .expect("a resize handle");
        // Dragging emits an accumulable delta; a zero delta does nothing.
        assert_eq!(handle.on_drag_delta(12.0), Some(Msg::Resize(0, 12.0)));
        assert_eq!(handle.on_drag_delta(0.0), None);

        // The handle is reachable as a draggable at the 1st column's edge (x≈100).
        let ui = build_ui(
            &table,
            Size::new(220.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(
            ui.draggable_at(Point::new(100.0, 20.0)).is_some(),
            "the handle can be grabbed at the edge"
        );
        // Outside fixed columns there is no overlay: one flexible width turns it off.
        let flex = Table::<Msg>::new(2)
            .header(&["A", "B"])
            .on_resize(Msg::Resize)
            .row(&["x", "y"]);
        // A bare grid (header + 1 row), with no handle layer over it.
        assert_eq!(
            Widget::<Msg>::children(&flex).len(),
            2,
            "flexible columns: no handle layer"
        );
        assert!(
            Widget::<Msg>::children(&flex)
                .iter()
                .all(|r| r.children().iter().all(|c| !c.draggable())),
            "no handle without fixed widths",
        );
    }

    #[test]
    fn reorderable_headers_expose_index_and_message() {
        let table = Table::<Msg>::new(3)
            .width(300.0)
            .header(&["A", "B", "C"])
            .on_sort(Msg::Sort)
            .on_reorder(Msg::Reorder)
            .row(&["x", "y", "z"]);
        let hrow = &Widget::<Msg>::children(&table)[0];
        let cells = hrow.children();
        // Each header knows its column (source/target) and produces Reorder(from, to).
        assert_eq!(cells[0].reorder_index(), Some(0));
        assert_eq!(cells[2].reorder_index(), Some(2));
        assert_eq!(cells[0].on_reorder(2), Some(Msg::Reorder(0, 2)));
        // A click still sorts (tap = sort, drag = reorder).
        assert_eq!(cells[1].on_click(), Some(Msg::Sort(1)));
        // Data cells are not reorderable.
        let drow = &Widget::<Msg>::children(&table)[1];
        assert_eq!(drow.children()[0].reorder_index(), None);
    }

    #[test]
    fn ctrl_arrows_reorder_focused_header() {
        use crate::interaction::{Key, KeyResponse};
        let table = Table::<Msg>::new(3)
            .width(300.0)
            .header(&["A", "B", "C"])
            .on_sort(Msg::Sort)
            .on_reorder(Msg::Reorder)
            .row(&["x", "y", "z"]);
        let cells = Widget::<Msg>::children(&table)[0].children();
        let ctrl_left = Key::Left {
            shift: false,
            word: true,
        };
        let ctrl_right = Key::Right {
            shift: false,
            word: true,
        };
        // The middle column (1): Ctrl+Left → 0, Ctrl+Right → 2.
        assert_eq!(
            cells[1].on_key(&ctrl_left),
            KeyResponse::Handled(Some(Msg::Reorder(1, 0)))
        );
        assert_eq!(
            cells[1].on_key(&ctrl_right),
            KeyResponse::Handled(Some(Msg::Reorder(1, 2)))
        );
        // At the edges: ignored (the focus navigates). Col 0 leftwards, col 2 rightwards.
        assert_eq!(cells[0].on_key(&ctrl_left), KeyResponse::Ignored);
        assert_eq!(cells[2].on_key(&ctrl_right), KeyResponse::Ignored);
        // A bare arrow (no Ctrl): ignored → focus navigation.
        assert_eq!(
            cells[1].on_key(&Key::Left {
                shift: false,
                word: false
            }),
            KeyResponse::Ignored
        );
    }

    #[test]
    fn reorderable_header_is_announced_with_position() {
        let table = Table::<Msg>::new(3)
            .width(300.0)
            .header(&["A", "B", "C"])
            .on_sort(Msg::Sort)
            .on_reorder(Msg::Reorder)
            .row(&["x", "y", "z"]);
        let sem = Widget::<Msg>::children(&table)[0].children()[1]
            .semantics()
            .expect("an announced header");
        assert_eq!(sem.label.as_deref(), Some("B"));
        assert_eq!(sem.value.as_deref(), Some("column 2 of 3"));
        // Data cells carry no header semantics.
        assert!(Widget::<Msg>::children(&table)[1].children()[0]
            .semantics()
            .is_none());
    }

    #[test]
    fn widget_rows_embed_arbitrary_widgets() {
        use crate::Text;
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Tag"])
            .on_select_row(Msg::Select)
            .widget_row(vec![
                Box::new(|| Box::new(Text::new("Ada"))),
                Box::new(|| Box::new(Text::new("admin"))),
            ]);
        // A header + 1 row of 2 widget cells, each holding its own widget.
        let rows = Widget::<Msg>::children(&table);
        assert_eq!(rows.len(), 2);
        let drow = &rows[1];
        assert_eq!(drow.children().len(), 2);
        assert_eq!(
            drow.children()[0].children().len(),
            1,
            "the cell holds a widget"
        );

        // The content ("admin") is indeed rendered, and the row stays selectable.
        let ui = build_ui(
            &table,
            Size::new(240.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has_admin = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "admin"));
        assert!(has_admin, "the cell widget is painted");
        let click = ui
            .hit(Point::new(30.0, ROW_H * 1.5))
            .and_then(|id| ui.msg_for(id));
        assert_eq!(click, Some(Msg::Select(0)), "the widget row is selectable");
    }

    #[test]
    fn header_icon_shifts_label_and_paints() {
        // A leading icon pushes the header's label back (icon + text).
        let name_x = |icons: bool| {
            let mut t = Table::<Msg>::new(2).width(240.0).header(&["Name", "Score"]);
            if icons {
                t = t.header_icons(&[Some(IconName::Star), None]);
            }
            let ui = build_ui(
                &t,
                Size::new(240.0, 100.0),
                &Runtime::default(),
                &Theme::default(),
            );
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Text { text, position, .. } if text == "Name" => Some(position.x),
                    _ => None,
                })
                .unwrap()
        };
        let (plain, iconed) = (name_x(false), name_x(true));
        assert!(
            iconed >= plain + ICON,
            "the label is shifted behind the icon: {iconed} vs {plain}"
        );
        // The column without an icon ("Score") is not shifted.
    }

    #[test]
    fn sort_and_selection_are_announced() {
        // A sortable header: the resulting sort announces the flipped direction.
        let unsorted = Table::<Msg>::new(2)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort);
        let head =
            |t: &Table<Msg>, c: usize| Widget::<Msg>::children(t)[0].children()[c].announce();
        assert_eq!(
            head(&unsorted, 0).as_deref(),
            Some("Sorted by Name ascending")
        );
        // Already ascending → a click switches to descending.
        let asc = Table::<Msg>::new(2)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .sorted(0, true);
        assert_eq!(head(&asc, 0).as_deref(), Some("Sorted by Name descending"));

        // Checkboxes: the resulting state of the toggle.
        let table = Table::<Msg>::new(2)
            .header(&["Name", "Score"])
            .checkboxes(Msg::Check, Msg::CheckAll)
            .selected(&[0])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // The "check all" box (header, partial) → check them all.
        let all = Widget::<Msg>::children(&table)[0].children()[0].announce();
        assert_eq!(all.as_deref(), Some("All rows selected"));
        // Row 0 (checked) → uncheck; row 1 (unchecked) → check.
        let row0 = Widget::<Msg>::children(&table)[1].children()[0].announce();
        let row1 = Widget::<Msg>::children(&table)[2].children()[0].announce();
        assert_eq!(row0.as_deref(), Some("Row deselected"));
        assert_eq!(row1.as_deref(), Some("Row selected"));
    }

    #[test]
    fn header_action_widget_captures_its_click() {
        use frus_core::Color;
        // An action button placed in the header, clickable independently of the sorting.
        struct Btn;
        impl Widget<Msg> for Btn {
            fn style(&self) -> Style {
                Style {
                    width: Dimension::Length(24.0),
                    height: Dimension::Length(24.0),
                    ..Default::default()
                }
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                &[]
            }
            fn paint(&self, bounds: Rect, _s: Status, t: &Theme, scene: &mut Scene) {
                scene.draw_rect(bounds, t.primary, 4.0, 0.0, Color::TRANSPARENT);
            }
            fn on_click(&self) -> Option<Msg> {
                Some(Msg::Filter)
            }
        }
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .header_action(1, || Box::new(Btn));
        // Column 1's header cell carries the action widget.
        assert_eq!(
            Widget::<Msg>::children(&table)[0].children()[1]
                .children()
                .len(),
            1,
            "column 1's header carries the action",
        );
        let ui = build_ui(
            &table,
            Size::new(240.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let click = |x: f32| {
            ui.hit(Point::new(x, ROW_H * 0.5))
                .and_then(|id| ui.msg_for(id))
        };
        // A click on the right (on the button, centred ~218) → the action; elsewhere → sorting.
        assert_eq!(
            click(218.0),
            Some(Msg::Filter),
            "the button captures its click"
        );
        assert_eq!(
            click(130.0),
            Some(Msg::Sort(1)),
            "the rest of the header sorts"
        );
    }

    #[test]
    fn row_click_selection_is_announced() {
        // Clicking a row (text or widget) announces the resulting state, with its number.
        let table = Table::<Msg>::new(2)
            .header(&["Name", "Tag"])
            .on_select_row(Msg::Select)
            .selected(&[1])
            .row(&["Ada", "admin"])
            .widget_row(vec![
                Box::new(|| Box::new(crate::Text::new("Bob"))),
                Box::new(|| Box::new(crate::Text::new("editor"))),
            ]);
        let rows = Widget::<Msg>::children(&table);
        // Text row 0 (not selected) → "selected"; any cell of the row will do.
        assert_eq!(
            rows[1].children()[0].announce().as_deref(),
            Some("Row 1 selected")
        );
        // Widget row 1 (selected) → "deselected".
        assert_eq!(
            rows[2].children()[0].announce().as_deref(),
            Some("Row 2 deselected")
        );
        // With no selection possible, there is no announcement.
        let plain = Table::<Msg>::new(1).header(&["N"]).row(&["x"]);
        assert_eq!(
            Widget::<Msg>::children(&plain)[1].children()[0].announce(),
            None
        );
    }

    #[test]
    fn widget_row_grows_to_tall_content() {
        use frus_core::Color;
        // A widget taller than ROW_H: the row adapts instead of clipping it.
        struct Tall(f32);
        impl Widget<Msg> for Tall {
            fn style(&self) -> Style {
                Style {
                    width: Dimension::Length(40.0),
                    height: Dimension::Length(self.0),
                    ..Default::default()
                }
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                &[]
            }
            fn paint(&self, bounds: Rect, _s: Status, _t: &Theme, scene: &mut Scene) {
                scene.fill_rect(bounds, Color::rgb(1.0, 0.0, 0.0));
            }
            fn on_click(&self) -> Option<Msg> {
                None
            }
        }
        let tall = 60.0;
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["A", "B"])
            .widget_row(vec![
                Box::new(move || Box::new(Tall(tall))),
                Box::new(|| Box::new(crate::Text::new("x"))),
            ]);
        let ui = build_ui(
            &table,
            Size::new(240.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The red bar (the tall widget) is painted at its full height: the cell, and so the
        // row, followed it (no clipping to ROW_H).
        let bar_h = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => {
                Some(rect.height)
            }
            _ => None,
        });
        assert!(
            bar_h.unwrap_or(0.0) >= tall - 1.0,
            "the row follows the tall content: {bar_h:?}"
        );
        assert!(
            tall - 1.0 > ROW_H,
            "the content does exceed the nominal height"
        );
    }

    #[test]
    fn header_action_menu_opens_as_column_menu() {
        use crate::{Button, Menu};
        // A Menu dropped in as a header action widget = a column menu: its floating overlay is
        // collected **even when nested** in the header, and it closes (Escape / an outside
        // click). The items are focusable → navigable with the keyboard.
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .header_action(1, || {
                Box::new(
                    Menu::new(
                        Button::new("...").on_press(Msg::Filter),
                        true,
                        Msg::CheckAll,
                    )
                    .item("Sort ascending", Msg::Sort(1))
                    .item("Sort descending", Msg::Sort(1)),
                )
            });
        let ui = build_ui(
            &table,
            Size::new(240.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The menu's (nested) overlay is indeed collected → dismissible.
        assert_eq!(
            ui.top_dismiss(),
            Some(Msg::CheckAll),
            "the column menu's overlay is collected"
        );
        // The menu floats and paints (its items are rendered over the grid).
        let painted = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            painted("Sort ascending"),
            "the column menu floats and paints"
        );
    }

    #[test]
    fn widget_header_hosts_arbitrary_header_widgets() {
        use crate::{Button, Text};
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .widget_header(vec![
                Box::new(|| Box::new(Text::new("Name"))),
                Box::new(|| Box::new(Button::new("Sort").on_press(Msg::Sort(1)))),
            ])
            .row(&["Ada", "5"]);
        // Header = the 1st row: 2 widget cells, each hosting its own widget.
        let rows = Widget::<Msg>::children(&table);
        assert_eq!(rows[0].children().len(), 2);
        assert_eq!(
            rows[0].children()[0].children().len(),
            1,
            "the widget header holds its widget"
        );

        let ui = build_ui(
            &table,
            Size::new(240.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The "Name" label (a header widget) is painted.
        let painted = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            painted("Name") && painted("Sort"),
            "the header widgets are painted"
        );
        // The bespoke header button emits **its** message (no automatic sorting).
        let click = ui
            .hit(Point::new(180.0, ROW_H * 0.5))
            .and_then(|id| ui.msg_for(id));
        assert_eq!(
            click,
            Some(Msg::Sort(1)),
            "the app wires the sorting into its own header widget"
        );
    }

    #[test]
    fn virtual_table_builds_only_visible_rows() {
        use std::cell::Cell as StdCell;
        use std::rc::Rc as StdRc;
        let built = StdRc::new(StdCell::new(0usize));
        let counter = built.clone();
        let table = Table::<Msg>::new(2)
            .width(200.0)
            .header(&["Name", "Score"])
            .on_select_row(Msg::Select)
            .virtual_rows(5000, 200.0, move |i| {
                counter.set(counter.get() + 1);
                vec![format!("R{i}"), format!("{i}")]
            });
        let ui = build_ui(
            &table,
            Size::new(200.0, 260.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Viewport 200 / ROW_H ≈ 6 visible rows (+ a margin) — never 5000.
        assert!(
            built.get() < 20,
            "only the visible rows are built: {}",
            built.get()
        );
        assert!(
            built.get() >= 5,
            "at least the visible window: {}",
            built.get()
        );
        // The **pinned** header + the first visible row are painted.
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Name"), "the header is pinned above the list");
        assert!(has("R0"), "the first virtualised row is built");
        // The scroll spans the whole content (5000 × ROW_H − the viewport).
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1, "one scrollable viewport");
        assert_eq!(
            maxes[0].2,
            5000.0 * ROW_H - 200.0,
            "the scroll bound = the total content"
        );
        // A visible row stays clickable (selection).
        let click = ui
            .hit(Point::new(20.0, ROW_H + 15.0))
            .and_then(|id| ui.msg_for(id));
        assert!(
            matches!(click, Some(Msg::Select(_))),
            "a virtualised row is clickable: {click:?}"
        );
    }

    #[test]
    fn frozen_columns_split_into_pinned_and_scrolling_blocks() {
        let table = Table::<Msg>::new(3)
            .width(240.0)
            .column_widths(&[80.0, 120.0, 120.0])
            .header(&["Name", "A", "B"])
            .on_sort(Msg::Sort)
            .on_select_row(Msg::Select)
            .frozen_columns(1)
            .row(&["Ada", "1", "2"])
            .row(&["Bob", "3", "4"]);
        // Root = a stack [row of blocks, shadow]; the row carries the pinned block + the scrolling one.
        let ui = build_ui(
            &table,
            Size::new(240.0, 160.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // **Horizontal** scrolling: the content (120+120) is wider than the remaining viewport.
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1, "one scrollable area");
        assert!(maxes[0].1 > 0.0, "max horizontal > 0 : {:?}", maxes[0]);
        // A **frozen** cell is clickable (selection); a **scrolling** header sorts — the
        // shadow at the freeze edge does not block clicks.
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        assert_eq!(
            click(40.0, ROW_H * 1.5),
            Some(Msg::Select(0)),
            "a frozen cell selects"
        );
        assert_eq!(
            click(90.0, ROW_H * 0.5),
            Some(Msg::Sort(1)),
            "a scrolling header sorts"
        );
    }

    #[test]
    fn freezing_both_edges_pins_left_and_right_columns() {
        // 4 columns: 1 frozen on the left, 1 on the right → column 2 (the middle) scrolls.
        let table = Table::<Msg>::new(4)
            .width(260.0)
            .column_widths(&[70.0, 120.0, 120.0, 70.0])
            .header(&["Name", "A", "B", "Act"])
            .on_sort(Msg::Sort)
            .on_select_row(Msg::Select)
            .frozen_columns(1)
            .frozen_columns_right(1)
            .row(&["Ada", "1", "2", "x"]);
        let ui = build_ui(
            &table,
            Size::new(260.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The middle (A + B = 240) overflows the remaining viewport → horizontal scrolling.
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1);
        assert!(maxes[0].1 > 0.0, "the middle scrolls: {:?}", maxes[0]);
        // The header pinned on the right ("Act", column 3) sorts, at the far right of the table.
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        let right = click(225.0, ROW_H * 0.5);
        assert_eq!(
            right,
            Some(Msg::Sort(3)),
            "the column frozen on the right: {right:?}"
        );
        // The header pinned on the left ("Name", column 0) sorts.
        assert_eq!(
            click(30.0, ROW_H * 0.5),
            Some(Msg::Sort(0)),
            "the column frozen on the left"
        );
    }

    #[test]
    fn virtual_table_supports_checkboxes() {
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .checkboxes(Msg::Check, Msg::CheckAll)
            .selected(&[0])
            .virtual_rows(1000, 200.0, |i| vec![format!("R{i}"), format!("{i}")]);
        let ui = build_ui(
            &table,
            Size::new(240.0, 260.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        // "Check all" in the pinned header.
        assert_eq!(click(CHECK_W * 0.5, ROW_H * 0.5), Some(Msg::CheckAll));
        // The box on the first visible row (the left column, below the header).
        let row_check = click(CHECK_W * 0.5, ROW_H + 15.0);
        assert!(
            matches!(row_check, Some(Msg::Check(_))),
            "a virtualised row's box: {row_check:?}"
        );
    }

    #[test]
    fn virtual_widget_rows_builds_only_visible() {
        use crate::Text;
        use std::cell::Cell as StdCell;
        use std::rc::Rc as StdRc;
        let built = StdRc::new(StdCell::new(0usize));
        let counter = built.clone();
        let table = Table::<Msg>::new(1)
            .width(200.0)
            .header(&["Item"])
            .on_select_row(Msg::Select)
            .virtual_widget_rows(3000, 200.0, move |i| {
                counter.set(counter.get() + 1);
                vec![Box::new(Text::new(format!("W{i}"))) as Box<dyn Widget<Msg>>]
            });
        let ui = build_ui(
            &table,
            Size::new(200.0, 260.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(
            built.get() < 20 && built.get() >= 5,
            "only the visible ones: {}",
            built.get()
        );
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Item"), "the header is pinned");
        assert!(has("W0"), "the first row's widget is built");
        // The virtualised widget row stays selectable (the cell captures the click).
        let click = ui
            .hit(Point::new(20.0, ROW_H + 15.0))
            .and_then(|id| ui.msg_for(id));
        assert!(
            matches!(click, Some(Msg::Select(_))),
            "the widget row is clickable: {click:?}"
        );
    }

    #[test]
    fn fixed_column_width_is_applied() {
        let table = Table::<()>::new(2)
            .width(300.0)
            .column_widths(&[80.0]) // 1st column fixed at 80, 2nd flexible
            .header(&["A", "B"])
            .row(&["x", "y"]);
        let ui = build_ui(
            &table,
            Size::new(300.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The 1st header column ("A") takes 80 px: "B" starts beyond 80 + the gap.
        let text_x = |t: &str| {
            ui.scene().primitives().iter().find_map(|p| match p {
                Primitive::Text { text, position, .. } if text == t => Some(position.x),
                _ => None,
            })
        };
        let (ax, bx) = (text_x("A").unwrap(), text_x("B").unwrap());
        assert!(bx >= ax + 80.0, "a fixed column of 80: bx={bx} ax={ax}");
    }
}
