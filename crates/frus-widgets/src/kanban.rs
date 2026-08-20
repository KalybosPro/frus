//! [`Kanban`]: a **columns + cards** board with **drag and drop** between columns.
//!
//! Like the rest of frus, the widget is **controlled**: the application holds the columns and
//! their cards, and reacts to a single `on_move(from_col, from_pos, to_col, to_pos)` message
//! emitted when a card is dropped. Drag and drop reuses the framework's **reordering**
//! mechanism (`reorder_index` / `on_reorder`): every slot carries a **flat index**
//! `col * STRIDE + pos`, and the card grabbed decodes the **target** slot (another card, or
//! the drop zone at the bottom of a column) to work out the destination.

use std::rc::Rc;

use frus_core::{Color, Insets, Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::button::{Button, Variant};
use crate::flex::Flex;
use crate::interaction::Status;
use crate::scroll::{Axis, SingleChildScrollView};
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::{CellFn, ReorderAxis, Widget};

/// A card moving between columns: `(from_col, from_pos, to_col, to_pos)` into a
/// message.
type MoveFn<Msg> = Rc<dyn Fn(usize, usize, usize, usize) -> Msg>;

/// The encoding stride of a `(column, position)` slot into a flat index: it bounds the number
/// of cards per column (amply enough for a board). See [`kanban_slot`].
const STRIDE: usize = 1000;
/// A column's width.
const COL_W: f32 = 220.0;
/// A column panel's (uniform) inner padding.
const COL_PAD: f32 = 12.0;
/// A card's height.
const CARD_H: f32 = 44.0;

/// The **flat** index of a `(col, pos)` slot: `col * STRIDE + pos`. This is a card's
/// [`reorder_index`](Widget::reorder_index) value (both source **and** target). Reusable to
/// test the routing of drag and drop.
pub fn kanban_slot(col: usize, pos: usize) -> usize {
    // Past STRIDE cards in a column, `pos` would overflow into the column field (the flat
    // index would silently target the next column). A debug guard; STRIDE stays generous.
    debug_assert!(
        pos < STRIDE,
        "position {pos} out of bounds (STRIDE = {STRIDE}): column overflow"
    );
    col * STRIDE + pos
}

/// Decodes a flat index into `(col, pos)` (the inverse of [`kanban_slot`]).
fn decode(slot: usize) -> (usize, usize) {
    (slot / STRIDE, slot % STRIDE)
}

/// A card: both **source** and **target** of drag and drop. Painted as a raised tile, it shows
/// either a **label** (a text card) or **widget content** supplied by the application (a rich
/// card: a label + tags + a delete button…), placed as a child.
struct Card<Msg> {
    label: String,
    /// **Rich** content (0 or 1 widget): when present, the card hosts it instead of the label.
    content: Vec<Box<dyn Widget<Msg>>>,
    /// Its own slot (a flat index): serves as `reorder_index`, both grabbed source **and** drop target.
    slot: usize,
    from_col: usize,
    from_pos: usize,
    on_move: Option<MoveFn<Msg>>,
}

impl<Msg: Clone> Widget<Msg> for Card<Msg> {
    fn style(&self) -> Style {
        if self.content.is_empty() {
            // A text card: a fixed height, with the label painted by the card.
            Style {
                width: Dimension::Auto,
                height: Dimension::Length(CARD_H),
                ..Default::default()
            }
        } else {
            // A rich card: the content is a child, the card adapts (floor `CARD_H`), and has padding.
            Style {
                width: Dimension::Auto,
                height: Dimension::Auto,
                min_height: Dimension::Length(CARD_H),
                padding: Insets::uniform(8.0),
                align: Align::Center,
                ..Default::default()
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A raised tile; tinted on hover, as a grab affordance.
        let base = theme.surface.lerp(theme.on_surface, 0.05);
        let fill = theme.state_layer(base, theme.on_surface, &status);
        scene.draw_rect(
            bounds,
            fill.fade(o),
            theme.radius,
            1.0,
            theme.scheme.outline_variant.fade(o),
        );
        // A label only for a **text** card (a rich card paints its own content).
        if self.content.is_empty() {
            let ty = bounds.y + (bounds.height - frus_text::line_height(15.0)) * 0.5;
            scene.text(
                Point::new(bounds.x + 12.0, ty),
                self.label.clone(),
                15.0,
                theme.on_surface.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn reorder_index(&self) -> Option<usize> {
        Some(self.slot)
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        // The target `to` is the flat index of the hovered slot (another card, or a drop zone).
        let (to_col, to_pos) = decode(to);
        self.on_move
            .as_ref()
            .map(|f| f(self.from_col, self.from_pos, to_col, to_pos))
    }

    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Vertical // cards slide vertically
    }
}

/// The drop zone at the bottom of a column: the insertion target **at the end** of the column
/// (and the only target of an empty one). Not a useful source (its `on_reorder` moves nothing).
struct DropZone {
    slot: usize,
}

impl<Msg: Clone> Widget<Msg> for DropZone {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            height: Dimension::Length(CARD_H * 0.8),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // A discreet simulated dashed outline (a faded border): "drop here".
        scene.draw_rect(
            bounds,
            Color::TRANSPARENT,
            theme.radius,
            1.0,
            theme.scheme.outline_variant.fade(status.opacity * 0.5),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn reorder_index(&self) -> Option<usize> {
        Some(self.slot)
    }

    fn on_reorder(&self, _to: usize) -> Option<Msg> {
        None // the drop zone is not a move source
    }

    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Vertical
    }

    fn reorder_draggable(&self) -> bool {
        false // target **only**: you drop onto it, you do not lift it
    }
}

/// A column's **panel**: title + cards + drop zone, on a discreet themed background. It is a
/// vertical `Flex` container that paints its own background (the panel's clip holds its cards).
struct Column<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for Column<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(COL_W),
            flex_direction: FlexDirection::Column,
            gap: 8.0,
            padding: Insets::uniform(COL_PAD),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // The panel's **themed** background (a default overridable through the theme): a discreet veil.
        let bg = theme.surface.lerp(theme.on_surface, 0.04);
        scene.draw_rect(
            bounds,
            bg.fade(status.opacity),
            theme.radius,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A **Kanban** board: titled columns of cards, with drag and drop between columns.
///
/// ```
/// use frus_widgets::Kanban;
/// let board: Kanban = Kanban::new(|fc, fp, tc, tp| { let _ = (fc, fp, tc, tp); })
///     .column("To do", ["Design", "Spec"])
///     .column("Doing", ["Build"])
///     .column("Done", ["Kickoff"]);
/// ```
/// The factory for a card's **rich content**: called again on every rebuild (a fresh widget), for
/// cards beyond plain text (a label + tags + a delete button…).
pub type CardFactory<Msg> = Rc<dyn Fn() -> Box<dyn Widget<Msg>>>;

/// A column's cards: plain **text**, or rich **widgets**, one per card.
enum ColCards<Msg> {
    Text(Vec<String>),
    Widgets(Vec<CardFactory<Msg>>),
}

impl<Msg> ColCards<Msg> {
    fn len(&self) -> usize {
        match self {
            ColCards::Text(v) => v.len(),
            ColCards::Widgets(v) => v.len(),
        }
    }
}

pub struct Kanban<Msg = ()> {
    on_move: Option<MoveFn<Msg>>,
    /// A "+ Add card" button at the bottom of every column (milestone 249): `on_add(col)` on adding.
    on_add: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// An explicit height for each column's **scrollable card area** (milestone 264, Trello
    /// style). `Some(h)`: the cards scroll vertically in a region of height `h` (with the title
    /// fixed above and the "+ Add card" button fixed below). `None`: the column stretches to
    /// the height of its content (the original behaviour). See [`Kanban::card_area_height`].
    card_area_height: Option<f32>,
    /// Per-column vertical scrolling **Trello style, with no explicit height** (milestone 266):
    /// the columns **fill** the board's height (through a stretched `Row`) and each column's
    /// card area is a `flex(1)` `SingleChildScrollView` that takes the rest and then scrolls. It takes
    /// precedence over `card_area_height`. See [`Kanban::scrollable_columns`].
    fill_columns: bool,
    columns: Vec<(String, ColCards<Msg>)>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Kanban<Msg> {
    /// Creates a board; `on_move(from_col, from_pos, to_col, to_pos)` is emitted when a card is
    /// **dropped** onto a slot (another card, or the end of a column).
    pub fn new(on_move: impl Fn(usize, usize, usize, usize) -> Msg + 'static) -> Self {
        Self {
            on_move: Some(Rc::new(on_move)),
            on_add: None,
            card_area_height: None,
            fill_columns: false,
            columns: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Adds a titled **column** with its (text) cards, in order.
    pub fn column(
        mut self,
        title: impl Into<String>,
        cards: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let cards = ColCards::Text(cards.into_iter().map(Into::into).collect());
        self.columns.push((title.into(), cards));
        self.rebuild();
        self
    }

    /// Adds a column in which every card is a **widget** (a rich card: a label + tags + a delete
    /// button…), supplied by a **factory** called again on every rebuild. The card stays a drag
    /// and drop source and target; its clickable elements (a × button…) capture their own click.
    pub fn column_widgets(
        mut self,
        title: impl Into<String>,
        cards: impl IntoIterator<Item = CellFn<Msg>>,
    ) -> Self {
        let cards = ColCards::Widgets(cards.into_iter().map(Rc::from).collect());
        self.columns.push((title.into(), cards));
        self.rebuild();
        self
    }

    /// Adds a **"+ Add card"** button at the bottom of every column; `on_add(col)` on click (the
    /// app adds a card to the column). Without this call, there is no add button.
    pub fn on_add(mut self, on_add: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_add = Some(Rc::new(on_add));
        self.rebuild();
        self
    }

    /// Makes each column's cards **scroll vertically** within a region of height `h` (in logical
    /// pixels), Trello style: the **title** stays fixed above, the "+ Add card" button fixed
    /// below, and only the cards (with the final drop zone) scroll. Without this call, the
    /// column stretches to the height of its content.
    ///
    /// This is a stopgap **controlled by the application**, which supplies the height: a
    /// `flex(1)` `SingleChildScrollView` receives no usable height until its chain of ancestors gives it a
    /// **definite** height (milestone 263), hence an explicit height here.
    pub fn card_area_height(mut self, h: f32) -> Self {
        self.card_area_height = Some(h);
        self.rebuild();
        self
    }

    /// Per-column vertical scrolling **Trello style, with no explicit height** (milestone 266):
    /// the columns **fill** the board's height and each column's card area takes the rest
    /// (below the title, above the "+ Add card" button) and then **scrolls**. Unlike
    /// [`Kanban::card_area_height`], the app has no height to compute: the board sits in an
    /// ancestor of **definite height** (a window, a bounded horizontal `SingleChildScrollView`…) and the flex
    /// fill does the rest. Takes precedence over `card_area_height` when both are set.
    pub fn scrollable_columns(mut self) -> Self {
        self.fill_columns = true;
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        self.children = self
            .columns
            .iter()
            .enumerate()
            .map(|(c, (title, cards))| self.build_column(c, title, cards))
            .collect();
    }

    /// Builds one column: title + cards (each a drop source and target) + the final drop zone +
    /// an optional add button.
    fn build_column(&self, col: usize, title: &str, cards: &ColCards<Msg>) -> Box<dyn Widget<Msg>> {
        let make_card = |pos: usize, content: Vec<Box<dyn Widget<Msg>>>, label: String| Card {
            label,
            content,
            slot: kanban_slot(col, pos),
            from_col: col,
            from_pos: pos,
            on_move: self.on_move.clone(),
        };
        // The cards (each both source **and** target) + the final drop zone: this is the
        // column's **scrollable** part (the title and the add button stay fixed).
        let mut cards_v: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        match cards {
            ColCards::Text(labels) => {
                for (pos, label) in labels.iter().enumerate() {
                    cards_v.push(Box::new(make_card(pos, Vec::new(), label.clone())));
                }
            }
            ColCards::Widgets(factories) => {
                for (pos, make) in factories.iter().enumerate() {
                    cards_v.push(Box::new(make_card(pos, vec![make()], String::new())));
                }
            }
        }
        // The insertion slot at the end of the column (and the target of an empty one).
        cards_v.push(Box::new(DropZone {
            slot: kanban_slot(col, cards.len()),
        }));

        let mut children: Vec<Box<dyn Widget<Msg>>> =
            vec![Box::new(Text::new(title.to_string()).size(16.0))];
        // The column's inner width (the column width minus its padding). The scrollable card
        // area wraps the cards in a vertical `Flex` (a single child for the `SingleChildScrollView`).
        let inner_w = COL_W - 2.0 * COL_PAD;
        let list = |cards: Vec<Box<dyn Widget<Msg>>>| {
            cards
                .into_iter()
                .fold(Flex::column().gap(8.0).width(inner_w), Flex::child_boxed)
        };
        if self.fill_columns {
            // **Filling** (milestone 266): the column fills the board's height (a stretched Row);
            // the card area, a `flex(1)` `SingleChildScrollView`, takes the rest (below the title, above the
            // button) and then scrolls. No explicit height: the flex does the arithmetic.
            children.push(Box::new(
                SingleChildScrollView::new()
                    .axis(Axis::Vertical)
                    .width(inner_w)
                    .flex(1.0)
                    .child(list(cards_v)),
            ));
        } else if let Some(h) = self.card_area_height {
            // An **explicit** height (milestone 264): the fallback when the ancestor has no definite height.
            children.push(Box::new(
                SingleChildScrollView::new()
                    .axis(Axis::Vertical)
                    .width(inner_w)
                    .height(h)
                    .child(list(cards_v)),
            ));
        } else {
            // A column that stretches to the height of its content (the original behaviour, bare cards).
            children.extend(cards_v);
        }
        // The "+ Add card" button (milestone 249), if requested.
        if let Some(on_add) = &self.on_add {
            let on_add = on_add.clone();
            children.push(Box::new(
                Button::new("+ Add card")
                    .variant(Variant::Outlined)
                    .size(13.0)
                    .on_press(on_add(col)),
            ));
        }
        Box::new(Column { children })
    }
}

impl<Msg: Clone> Widget<Msg> for Kanban<Msg> {
    fn style(&self) -> Style {
        // **Filling** mode (milestone 266): the row takes its parent's height (`Percent(1.0)`
        // — the ancestor has a definite height) and **stretches** its columns
        // (`Align::Stretch`) so each fills the board; their `flex(1)` card area then takes
        // the rest and scrolls. Otherwise: the content's height, columns aligned to the top.
        let (align, height) = if self.fill_columns {
            (Align::Stretch, Dimension::Percent(1.0))
        } else {
            (Align::Start, Dimension::Auto)
        };
        Style {
            height,
            flex_direction: FlexDirection::Row,
            gap: 12.0,
            align,
            padding: Insets::ZERO,
            ..Default::default()
        }
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Move(usize, usize, usize, usize),
        Add(usize),
        Del(usize),
    }

    /// Collects, in tree order, every non-null `on_click` message of a subtree.
    fn collect_clicks(w: &dyn Widget<Msg>, out: &mut Vec<Msg>) {
        if let Some(m) = w.on_click() {
            out.push(m);
        }
        for c in w.children() {
            collect_clicks(c.as_ref(), out);
        }
    }

    /// Finds a subtree's first card (a widget with a `reorder_index`) and returns
    /// `(reorder_index, on_reorder(target))`.
    fn first_card(w: &dyn Widget<Msg>, target: usize) -> Option<(usize, Option<Msg>)> {
        if let Some(idx) = w.reorder_index() {
            return Some((idx, w.on_reorder(target)));
        }
        for c in w.children() {
            if let Some(found) = first_card(c.as_ref(), target) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn slot_encoding_roundtrips() {
        assert_eq!(decode(kanban_slot(0, 0)), (0, 0));
        assert_eq!(decode(kanban_slot(2, 5)), (2, 5));
        assert_eq!(decode(kanban_slot(1, 3)), (1, 3));
    }

    #[test]
    fn dropping_a_card_routes_a_cross_column_move() {
        let board = Kanban::new(Msg::Move)
            .column("To do", ["A", "B"])
            .column("Doing", ["C"]);
        // The first card = column 0, position 0 ("A"): its flat index, and the move produced
        // when it is dropped onto slot (1, 0) of the "Doing" column.
        let col0 = &Widget::<Msg>::children(&board)[0];
        let (idx, moved) = first_card(col0.as_ref(), kanban_slot(1, 0)).expect("a card");
        assert_eq!(idx, kanban_slot(0, 0), "the source card's flat index");
        assert_eq!(
            moved,
            Some(Msg::Move(0, 0, 1, 0)),
            "a drop at (1,0): a cross-column move"
        );
    }

    #[test]
    fn cards_declare_vertical_reorder_axis() {
        fn first_axis(w: &dyn Widget<Msg>) -> Option<ReorderAxis> {
            if w.reorder_index().is_some() {
                return Some(w.reorder_axis());
            }
            w.children().iter().find_map(|c| first_axis(c.as_ref()))
        }
        let board = Kanban::new(Msg::Move).column("A", ["x"]);
        let col0 = &Widget::<Msg>::children(&board)[0];
        assert_eq!(
            first_axis(col0.as_ref()),
            Some(ReorderAxis::Vertical),
            "the cards slide vertically"
        );
    }

    #[test]
    fn rich_cards_host_content_and_add_button_is_present() {
        // A **rich** column: each card hosts a × (delete) button; plus an add button.
        let cards: Vec<CellFn<Msg>> = (0..2)
            .map(|i| {
                Box::new(move || {
                    Box::new(Button::new("x").on_press(Msg::Del(i))) as Box<dyn Widget<Msg>>
                }) as CellFn<Msg>
            })
            .collect();
        let board = Kanban::new(Msg::Move)
            .on_add(Msg::Add)
            .column_widgets("Col", cards);
        let col0 = &Widget::<Msg>::children(&board)[0];
        // The rich card stays **reorderable** (dragging wired up) and routes a Move.
        let (idx, moved) = first_card(col0.as_ref(), kanban_slot(0, 1)).expect("a card");
        assert_eq!(idx, kanban_slot(0, 0), "the rich card keeps its flat index");
        assert_eq!(
            moved,
            Some(Msg::Move(0, 0, 0, 1)),
            "and still routes the move"
        );
        // The reachable clicks include deletion (each card's ×) and adding (the column button).
        let mut clicks = Vec::new();
        collect_clicks(col0.as_ref(), &mut clicks);
        assert!(
            clicks.contains(&Msg::Del(0)) && clicks.contains(&Msg::Del(1)),
            "a × delete per card"
        );
        assert!(
            clicks.contains(&Msg::Add(0)),
            "the column's + Add card button"
        );
    }

    #[test]
    fn cards_are_draggable_but_the_drop_zone_is_target_only() {
        /// Returns the `reorder_draggable` of the first reorderable (a card) and looks for a
        /// drop zone (a reorderable that is **not** grabbable) in the subtree.
        fn scan(w: &dyn Widget<Msg>, out: &mut Vec<bool>) {
            if w.reorder_index().is_some() {
                out.push(w.reorder_draggable());
            }
            for c in w.children() {
                scan(c.as_ref(), out);
            }
        }
        let board = Kanban::new(Msg::Move).column("To do", ["A", "B"]);
        let col0 = &Widget::<Msg>::children(&board)[0];
        let mut flags = Vec::new();
        scan(col0.as_ref(), &mut flags);
        // Two grabbable cards + one non-grabbable drop zone.
        assert_eq!(
            flags.iter().filter(|&&d| d).count(),
            2,
            "both cards are grabbable"
        );
        assert_eq!(
            flags.iter().filter(|&&d| !d).count(),
            1,
            "the drop zone is target-only"
        );
    }

    #[test]
    fn board_lays_out_one_widget_per_column() {
        let board = Kanban::new(Msg::Move)
            .column("A", ["x"])
            .column("B", Vec::<String>::new());
        assert_eq!(
            Widget::<Msg>::children(&board).len(),
            2,
            "one widget per column"
        );
    }
}
