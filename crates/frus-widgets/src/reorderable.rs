//! [`ReorderableList`]: **a list whose rows can be dragged into a new order** — a
//! playlist, a set of favourites, a to-do list.
//!
//! Reordering already existed here, but only as a board: [`crate::Kanban`] moves cards
//! between columns and [`crate::Table`] moves columns, and an application that wanted the
//! ordinary case had either to take a board's assumptions or to build the gesture itself
//! out of [`crate::Draggable`]. What it had to build is the part that is hard: a gap that
//! opens where the row would land, a list that keeps scrolling while a row is carried past
//! its edge, and a grip that does not take the gesture the scroll needs.
//!
//! **What is new here is thinner than it looks**, and that is the point. The shell has
//! carried a vertical reordering drag since milestone 252 — the ghost, the insertion line,
//! the neighbours sliding out of the way, the drop routed back as `on_reorder` — and it
//! carries it for any widget that answers [`Widget::reorder_index`] with
//! [`ReorderAxis::Vertical`]. A list's rows answer it. So this module is a row wrapper, a
//! grip, and the two decisions that gesture needs: **who may be grabbed**, and **what index
//! comes back**.
//!
//! ## Who may be grabbed
//!
//! A row that could be lifted by a press anywhere on it would stop its own list scrolling,
//! because the press that starts a drag is the press that would have started the scroll.
//! The reference splits the answer into two widgets — a listener that grabs immediately,
//! for a handle, and one that waits for a long press, for the whole row — and leaves the
//! choice to the application, because it depends on what else the row is for.
//!
//! Here that is [`ReorderGrab`], defaulting the way the reference defaults it: a **grip**
//! on a desktop, where there is a pointer and a small target is easy, and a **hold** on a
//! phone, where the finger is coarse and the list scrolls under it.
//!
//! ## What index comes back
//!
//! Dropping a row on the lower half of the third row means *after the third row*, which is
//! raw index 3 — but the row being carried is no longer in the list, so where it lands is
//! index 2. The reference passed the raw number and told applications to subtract one
//! themselves; it has since deprecated that callback in favour of one that does the
//! subtraction for it. This framework starts where the reference ended up:
//! `on_reorder(from, to)` gives **the index the row ends up at**, and
//! [`settled_index`] is the one line that makes it so.

use std::cell::RefCell;
use std::rc::Rc;

use frus_core::{Color, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::icons::{IconData, Icons};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{ReorderAxis, Widget};
use crate::{Container, Expanded, Icon, ThemeBuilder};

/// The width of the grip's box, and so the size of its tap target.
const HANDLE_WIDTH: f32 = 40.0;
/// The glyph inside it.
const HANDLE_ICON: f32 = 20.0;

/// How a row of a [`ReorderableList`] is picked up.
///
/// Not a decoration: it decides whether the list can still be scrolled with a finger, so
/// it is the first thing to get right and the reason the reference has two widgets where
/// it could have had one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReorderGrab {
    /// A **grip** at the row's trailing edge, which lifts on the press. Everywhere else on
    /// the row keeps meaning what it meant — a tap, a swipe, a scroll.
    Handle,
    /// **The whole row**, after a hold. Nothing is added to the row, and the list still
    /// scrolls: a finger that holds still is not scrolling, which is what makes the hold
    /// a gesture the scroll cannot claim.
    LongPress,
}

impl Default for ReorderGrab {
    /// The reference's own rule, read from the platform rather than from a flag: a grip
    /// where there is a pointer, a hold where there is a finger.
    fn default() -> Self {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            ReorderGrab::LongPress
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            ReorderGrab::Handle
        }
    }
}

/// Where a row **ends up**, given where it came from and the raw slot it was dropped on.
///
/// The raw slot counts the rows as they are on screen, with the carried row still among
/// them. Once it is taken out, everything after it has shifted up by one — so a drop aimed
/// past the row's own position lands one earlier than the number said.
///
/// It is a subtraction every application writes once and gets wrong once, which is why it
/// is here and tested rather than in a doc comment telling callers to do it.
pub fn settled_index(from: usize, raw: usize) -> usize {
    if raw > from {
        raw - 1
    } else {
        raw
    }
}

type ReorderFn<Msg> = Rc<dyn Fn(usize, usize) -> Msg>;
type HandleFn<Msg> = Rc<dyn Fn(&Theme) -> Box<dyn Widget<Msg>>>;

/// Everything a row needs to know that belongs to the **list**, shared with the rows so
/// that a setter called after a row was added still reaches it.
///
/// The alternative — settling each row when it is added — makes the order of the builder
/// calls part of the API, and a list whose `.grab()` silently does nothing because it came
/// after `.row()` is a bug no compiler catches.
struct Spec<Msg> {
    grab: ReorderGrab,
    enabled: bool,
    on_reorder: Option<ReorderFn<Msg>>,
    handle: Option<HandleFn<Msg>>,
    handle_icon: Option<IconData>,
    handle_color: Option<Color>,
    handle_icon_size: Option<f32>,
    handle_width: f32,
    /// How many rows there are, for what is said out loud on the drop. Live, because the
    /// last row is added after the first one was.
    count: usize,
}

impl<Msg> Spec<Msg> {
    /// Is the grip the way in, this frame?
    fn gripped(&self) -> bool {
        self.enabled && self.grab == ReorderGrab::Handle
    }

    /// Is the whole row the way in, this frame?
    fn held(&self) -> bool {
        self.enabled && self.grab == ReorderGrab::LongPress
    }
}

type Shared<Msg> = Rc<RefCell<Spec<Msg>>>;

/// The message for a move, or `None` when the row would not actually move.
///
/// A drop on the lower half of a row's **own** box asks for the slot after it, which is
/// where it already is. The shell guards the same case by identity, but it cannot when the
/// grip and the row are two widgets: what was grabbed is the grip, and what was dropped on
/// is the row, so they do not compare equal. The arithmetic knows, and says nothing.
fn move_message<Msg>(spec: &Spec<Msg>, from: usize, raw: usize) -> Option<Msg> {
    let to = settled_index(from, raw);
    if to == from {
        return None;
    }
    spec.on_reorder.as_ref().map(|f| f(from, to))
}

/// What the screen reader says once the row has landed: its new position, one-based, of
/// however many rows there are.
fn move_announcement<Msg>(spec: &Spec<Msg>, from: usize, raw: usize) -> Option<String> {
    let to = settled_index(from, raw);
    (to != from).then(|| format!("Moved to position {} of {}", to + 1, spec.count.max(to + 1)))
}

/// One row: the caller's widget, and the grip beside it.
///
/// It is the drop **target** in both modes — the whole row is what a carried row is aimed
/// at — and the drag **source** only when the mode is a hold.
struct ReorderRow<Msg> {
    index: usize,
    key: Option<u64>,
    spec: Shared<Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Widget<Msg> for ReorderRow<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_direction: FlexDirection::Row,
            align: Align::Center,
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

    fn key(&self) -> Option<u64> {
        self.key
    }

    fn reorder_index(&self) -> Option<usize> {
        self.spec.borrow().enabled.then_some(self.index)
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        move_message(&self.spec.borrow(), self.index, to)
    }

    fn reorder_announcement(&self, to: usize) -> Option<String> {
        move_announcement(&self.spec.borrow(), self.index, to)
    }

    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Vertical
    }

    fn reorder_draggable(&self) -> bool {
        self.spec.borrow().held()
    }

    fn drag_needs_long_press(&self) -> bool {
        self.spec.borrow().held()
    }
}

/// The grip: a source that is not a target.
///
/// It is always in the row, and takes no room at all in the modes that do not use it —
/// which is what lets `.grab()` be read at build time instead of at the moment the row was
/// added.
struct ReorderHandle<Msg> {
    index: usize,
    spec: Shared<Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Widget<Msg> for ReorderHandle<Msg> {
    fn style(&self) -> Style {
        let spec = self.spec.borrow();
        if !spec.gripped() {
            return Style {
                width: Dimension::Length(0.0),
                height: Dimension::Length(0.0),
                ..Default::default()
            };
        }
        Style {
            width: Dimension::Length(spec.handle_width),
            height: Dimension::Auto,
            // A grip that shrinks is a grip that misses: the row beside it is what gives
            // way when the line is tight.
            flex_shrink: 0.0,
            align: Align::Center,
            justify: Justify::Center,
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

    fn reorder_index(&self) -> Option<usize> {
        self.spec.borrow().gripped().then_some(self.index)
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        move_message(&self.spec.borrow(), self.index, to)
    }

    fn reorder_announcement(&self, to: usize) -> Option<String> {
        move_announcement(&self.spec.borrow(), self.index, to)
    }

    fn reorder_axis(&self) -> ReorderAxis {
        ReorderAxis::Vertical
    }

    fn reorder_droppable(&self) -> bool {
        false
    }
}

/// A list whose rows can be **dragged into a new order**.
///
/// The order stays the application's, as everywhere else here: the list emits *this row
/// moved from `from` to `to`* and the application permutes its own data. Nothing moves on
/// screen until it does.
///
/// ```ignore
/// ReorderableList::new(|from, to| Msg::MoveTrack(from, to))
///     .gap(8.0)
///     .keyed_row(track.id, track_row(track))
/// ```
///
/// **Each row should carry a key** — `keyed_row`, or a [`crate::keyed`] wrapper of your
/// own. Rows are told apart by position otherwise, and a list whose rows are about to swap
/// positions is exactly where that goes wrong: the hover and the animations of the row
/// that moved would stay behind with its index.
pub struct ReorderableList<Msg = ()> {
    spec: Shared<Msg>,
    rows: Vec<Box<dyn Widget<Msg>>>,
    gap: f32,
    width: Dimension,
    height: Dimension,
    flex_grow: f32,
}

impl<Msg: Clone + 'static> ReorderableList<Msg> {
    /// A list whose rows move; `on_reorder(from, to)` carries **the index the row ends up
    /// at**, both counted in this list's own rows.
    pub fn new(on_reorder: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        Self {
            spec: Rc::new(RefCell::new(Spec {
                grab: ReorderGrab::default(),
                enabled: true,
                on_reorder: Some(Rc::new(on_reorder)),
                handle: None,
                handle_icon: None,
                handle_color: None,
                handle_icon_size: None,
                handle_width: HANDLE_WIDTH,
                count: 0,
            })),
            rows: Vec::new(),
            gap: 0.0,
            width: Dimension::Auto,
            height: Dimension::Auto,
            flex_grow: 0.0,
        }
    }

    /// Adds a row, identified by `key`.
    #[must_use]
    pub fn keyed_row(self, key: u64, row: impl Widget<Msg> + 'static) -> Self {
        self.push(Some(key), Box::new(row))
    }

    /// Adds a row with no key of its own — for a list that never changes length.
    #[must_use]
    pub fn row(self, row: impl Widget<Msg> + 'static) -> Self {
        self.push(None, Box::new(row))
    }

    fn push(mut self, key: Option<u64>, row: Box<dyn Widget<Msg>>) -> Self {
        let index = self.rows.len();
        self.spec.borrow_mut().count = index + 1;
        let spec = Rc::clone(&self.spec);
        let glyph = Rc::clone(&self.spec);
        // The grip's own appearance is deferred until the theme is known, so that every
        // one of its parts — the glyph, its colour, its size, or a widget of the
        // application's entirely — can be set after this row was added and still be the
        // one that is drawn.
        let handle = ReorderHandle {
            index,
            spec: Rc::clone(&self.spec),
            children: vec![Box::new(ThemeBuilder::new(move |theme: &Theme| {
                grip(&glyph.borrow(), theme)
            }))],
        };
        self.rows.push(Box::new(ReorderRow {
            index,
            key,
            spec,
            children: vec![Box::new(Expanded::new(row)), Box::new(handle)],
        }));
        self
    }

    /// How a row is picked up. Defaults to the platform's habit — see [`ReorderGrab`].
    #[must_use]
    pub fn grab(self, grab: ReorderGrab) -> Self {
        self.spec.borrow_mut().grab = grab;
        self
    }

    /// Turns reordering off without changing the list: no grip, no hold, no drop target.
    /// A list that is briefly read-only stays the same widget, so its rows keep their
    /// state.
    #[must_use]
    pub fn enabled(self, enabled: bool) -> Self {
        self.spec.borrow_mut().enabled = enabled;
        self
    }

    /// The whole grip, replaced by a widget of the application's. It is built with the
    /// ambient theme, and what it is made of is nobody's business but the caller's — this
    /// widget only says where it goes and what grabbing it means.
    #[must_use]
    pub fn handle<W: Widget<Msg> + 'static>(self, handle: impl Fn(&Theme) -> W + 'static) -> Self {
        self.spec.borrow_mut().handle = Some(Rc::new(move |theme| Box::new(handle(theme))));
        self
    }

    /// The grip's glyph, over the theme's and the framework's.
    #[must_use]
    pub fn handle_icon(self, icon: IconData) -> Self {
        self.spec.borrow_mut().handle_icon = Some(icon);
        self
    }

    /// The grip's colour, over the theme's.
    #[must_use]
    pub fn handle_color(self, color: Color) -> Self {
        self.spec.borrow_mut().handle_color = Some(color);
        self
    }

    /// The grip's glyph size.
    #[must_use]
    pub fn handle_icon_size(self, size: f32) -> Self {
        self.spec.borrow_mut().handle_icon_size = Some(size);
        self
    }

    /// The width of the grip's box — its tap target, which is not the size of the glyph
    /// inside it.
    #[must_use]
    pub fn handle_width(self, width: f32) -> Self {
        self.spec.borrow_mut().handle_width = width;
        self
    }

    /// Space between rows.
    #[must_use]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// An explicit width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// A width given as a fraction of what the parent offers.
    #[must_use]
    pub fn width_fraction(mut self, fraction: f32) -> Self {
        self.width = Dimension::Percent(fraction);
        self
    }

    /// An explicit height.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Dimension::Length(height);
        self
    }

    /// The share of a parent flex's spare room this list takes.
    #[must_use]
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }
}

/// The grip's contents: the application's widget, or the default glyph.
fn grip<Msg: Clone + 'static>(spec: &Spec<Msg>, theme: &Theme) -> Box<dyn Widget<Msg>> {
    if !spec.gripped() {
        // Not this mode: an empty box, so that nothing is measured, drawn or hit.
        return Box::new(Container::new().width(0.0).height(0.0));
    }
    if let Some(handle) = &spec.handle {
        return handle(theme);
    }
    let icon = spec
        .handle_icon
        .or(theme.widgets.reorderable.handle_icon)
        .unwrap_or(Icons::DRAG_HANDLE);
    let size = spec
        .handle_icon_size
        .or(theme.widgets.reorderable.handle_icon_size)
        .unwrap_or(HANDLE_ICON);
    let color = spec
        .handle_color
        .or(theme.widgets.reorderable.handle_color)
        .unwrap_or(theme.muted);
    Box::new(Icon::new(icon).size(size).color(color))
}

impl<Msg: Clone + 'static> Widget<Msg> for ReorderableList<Msg> {
    fn style(&self) -> Style {
        Style {
            width: self.width,
            height: self.height,
            flex_grow: self.flex_grow,
            flex_direction: FlexDirection::Column,
            gap: self.gap,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.rows
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::InspectorNode;
    use crate::widget::Widget;
    use crate::{text, Container, Runtime, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Moved(usize, usize),
    }

    fn list(grab: ReorderGrab) -> ReorderableList<Msg> {
        ReorderableList::new(Msg::Moved)
            .grab(grab)
            .width(300.0)
            .keyed_row(1, Container::new().height(40.0).child(text("one")))
            .keyed_row(2, Container::new().height(40.0).child(text("two")))
            .keyed_row(3, Container::new().height(40.0).child(text("three")))
    }

    /// Walks the built tree and hands back every widget, so the hooks the shell reads can
    /// be read the same way.
    ///
    /// The grip's contents are deferred until the theme is known, so the tree is built
    /// first — a walk that reaches a deferred node before any layout pass has is exactly
    /// what milestone 415 made say so out loud.
    fn nodes_of(root: &dyn Widget<Msg>) -> Vec<&dyn Widget<Msg>> {
        crate::build_deferred(root, &Theme::dark(), &crate::Runtime::default());
        let mut out: Vec<&dyn Widget<Msg>> = vec![root];
        let mut i = 0;
        while i < out.len() {
            let node = out[i];
            for child in node.children() {
                out.push(child.as_ref());
            }
            i += 1;
        }
        out
    }

    /// The inspector's boxes for a list laid out in a 300 x 400 window.
    fn inspected(root: &dyn Widget<Msg>) -> Vec<InspectorNode> {
        crate::build_ui_inspected(
            root,
            Size::new(300.0, 400.0),
            &Runtime::default(),
            &Theme::dark(),
        )
        .1
    }

    /// Every reorderable in the tree, as `(index, draggable, droppable)`.
    fn slots(root: &dyn Widget<Msg>) -> Vec<(usize, bool, bool)> {
        nodes_of(root)
            .into_iter()
            .filter_map(|w| {
                w.reorder_index()
                    .map(|i| (i, w.reorder_draggable(), w.reorder_droppable()))
            })
            .collect()
    }

    #[test]
    fn a_settled_index_is_the_raw_one_minus_the_row_that_left() {
        // Downwards: the row is taken out first, so everything after it has shifted up.
        assert_eq!(settled_index(0, 3), 2);
        // Upwards: nothing between the row and its destination has moved.
        assert_eq!(settled_index(3, 0), 0);
        // Onto itself, from either half: it is already there.
        assert_eq!(settled_index(2, 2), 2);
        assert_eq!(settled_index(2, 3), 2);
    }

    #[test]
    fn a_gripped_row_is_a_target_and_its_grip_is_the_source() {
        let list = list(ReorderGrab::Handle);
        let slots = slots(&list);
        // Three rows and three grips, each pair sharing an index.
        assert_eq!(slots.len(), 6, "three rows and three grips: {slots:?}");
        for i in 0..3 {
            assert!(
                slots.contains(&(i, false, true)),
                "row {i} is a target and not a source: {slots:?}"
            );
            assert!(
                slots.contains(&(i, true, false)),
                "grip {i} is a source and not a target: {slots:?}"
            );
        }
    }

    #[test]
    fn a_held_row_is_both_and_waits_for_the_hold() {
        let list = list(ReorderGrab::LongPress);
        // The grips withdraw entirely: no index, so nothing is registered for them.
        assert_eq!(
            slots(&list),
            vec![(0, true, true), (1, true, true), (2, true, true)]
        );
        // And what makes the list still scrollable: the row does not take the press.
        let held: Vec<bool> = nodes_of(&list)
            .into_iter()
            .filter(|w| w.reorder_index().is_some())
            .map(|w| w.drag_needs_long_press())
            .collect();
        assert_eq!(held, vec![true, true, true]);
    }

    #[test]
    fn the_message_carries_where_the_row_ends_up() {
        let list = list(ReorderGrab::Handle);
        let row = nodes_of(&list)
            .into_iter()
            .find(|w| w.reorder_index() == Some(0) && !w.reorder_draggable())
            .expect("the first row");
        // Dropped past the last row: raw 3, and the list is three long, so it lands at 2.
        assert_eq!(row.on_reorder(3), Some(Msg::Moved(0, 2)));
        // Dropped on its own lower half: raw 1, which is where it already is.
        assert_eq!(row.on_reorder(1), None, "a move of nothing sends nothing");
        assert_eq!(
            row.reorder_announcement(3).as_deref(),
            Some("Moved to position 3 of 3")
        );
        assert_eq!(row.reorder_announcement(1), None);
    }

    #[test]
    fn a_setter_after_a_row_still_reaches_it() {
        // The order of the builder calls is not part of the API: `.grab()` last is the
        // same list as `.grab()` first.
        let late = ReorderableList::new(Msg::Moved)
            .width(300.0)
            .keyed_row(1, Container::new().height(40.0).child(text("one")))
            .grab(ReorderGrab::LongPress);
        assert_eq!(slots(&late), vec![(0, true, true)]);
    }

    #[test]
    fn a_disabled_list_offers_nothing_to_grab() {
        let list = list(ReorderGrab::Handle).enabled(false);
        assert!(slots(&list).is_empty(), "no source and no target");
    }

    /// The grip is beside the row, not on top of it: the row keeps the width it is left,
    /// and the grip keeps its own however narrow the list gets.
    #[test]
    fn the_grip_takes_its_own_room_at_the_trailing_edge() {
        let list = list(ReorderGrab::Handle);
        let nodes = inspected(&list);
        let grips: Vec<&InspectorNode> =
            nodes.iter().filter(|n| n.name == "ReorderHandle").collect();
        assert_eq!(grips.len(), 3, "one grip per row");
        for grip in grips {
            assert!(
                (grip.rect.width - HANDLE_WIDTH).abs() < 0.5,
                "the grip keeps its 40 px: {:?}",
                grip.rect
            );
            assert!(
                (grip.rect.x + grip.rect.width - 300.0).abs() < 0.5,
                "at the trailing edge: {:?}",
                grip.rect
            );
        }
    }

    /// **The grip's middle is inside its row**, and the two share an index.
    ///
    /// That pair is not decoration: it is how the shell finds what a grab actually moves.
    /// What is grabbed is the grip, and everything the drag is made of is the row's — the
    /// ghost, the height of the gap that closes behind it, the band its neighbours are
    /// matched against — so the shell looks for the droppable reorderable of the same
    /// index under the grip's own middle. A grip that sat outside its row, or carried a
    /// different index, would lift a 40-pixel icon and open a slot to match.
    #[test]
    fn the_grip_sits_inside_the_row_it_moves() {
        let list = list(ReorderGrab::Handle);
        let nodes = inspected(&list);
        let rows: Vec<&InspectorNode> = nodes.iter().filter(|n| n.name == "ReorderRow").collect();
        let grips: Vec<&InspectorNode> =
            nodes.iter().filter(|n| n.name == "ReorderHandle").collect();
        assert_eq!((rows.len(), grips.len()), (3, 3));
        for (row, grip) in rows.iter().zip(&grips) {
            let middle = frus_core::Point::new(
                grip.rect.x + grip.rect.width * 0.5,
                grip.rect.y + grip.rect.height * 0.5,
            );
            assert!(
                row.rect.contains(middle),
                "the grip's middle {middle:?} is inside its row {:?}",
                row.rect
            );
        }
        // And the index they share, which is what tells the shell which row is which.
        let indices: Vec<(usize, bool)> = nodes_of(&list)
            .into_iter()
            .filter_map(|w| w.reorder_index().map(|i| (i, w.reorder_droppable())))
            .collect();
        for i in 0..3 {
            assert!(indices.contains(&(i, true)) && indices.contains(&(i, false)));
        }
    }

    /// And in the mode that does not use it, it costs nothing at all.
    #[test]
    fn a_held_list_has_no_grip_taking_room() {
        let list = list(ReorderGrab::LongPress);
        let nodes = inspected(&list);
        for grip in nodes.iter().filter(|n| n.name == "ReorderHandle") {
            assert_eq!(grip.rect.width, 0.0, "no box: {:?}", grip.rect);
        }
        // The rows still fill the list's width, grip or no grip.
        let rows: Vec<&InspectorNode> = nodes.iter().filter(|n| n.name == "ReorderRow").collect();
        assert_eq!(rows.len(), 3);
        for row in rows {
            assert!((row.rect.width - 300.0).abs() < 0.5, "{:?}", row.rect);
        }
    }
}
