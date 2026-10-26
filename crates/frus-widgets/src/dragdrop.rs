//! [`Draggable`] and [`DragTarget`]: **picking a thing up and dropping it
//! somewhere else** — the general pair, underneath the reordering that
//! [`crate::Table`] and [`crate::Kanban`] already do for themselves.
//!
//! Reordering answers a narrower question: *where in this list?* Its answer is an
//! index, and both ends of the gesture belong to the same widget. Drag and drop
//! answers a wider one: *which thing, onto which other thing?* The two ends are
//! unrelated widgets that need not know of each other, so what travels between them
//! is a **payload** the application chooses — a `u64` it can map back to whatever it
//! means.
//!
//! ```ignore
//! Draggable::new(card).payload(task.id)
//! DragTarget::new(column).on_drop(|payload| Msg::MoveTask(payload, column_id))
//! ```
//!
//! **A draggable yields to a scrollable underneath it.** A widget that took every
//! drag inside a list would silently stop that list scrolling, which is a worse
//! failure than not lifting: the gesture that broke is the one the user makes a
//! hundred times an hour. So inside a scrollable, the way to lift is
//! [`Draggable::long_press`] — a gesture the scroll cannot claim, because a finger
//! that holds still is not scrolling. Outside one, a plain drag lifts straight away.
//!
//! **Files from outside the application land on the same targets.** On the desktop,
//! files dragged onto the window from the system's file manager are offered to the
//! target under the pointer, which lights up as it does for an item from inside and
//! receives their paths on the drop — see [`DragTarget::on_files`].

use std::path::PathBuf;

use frus_core::{Color, Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Status, WidgetId};
use crate::theme::Theme;
use crate::widget::{sizing_of, Widget};

/// The opacity left behind where a lifted item was.
const GHOSTED_OPACITY: f32 = 0.35;
/// The opacity of the highlight painted over a target that would accept the drop.
const TARGET_ALPHA: f32 = 0.12;
/// The thickness of that highlight's outline, in px.
const TARGET_OUTLINE: f32 = 2.0;

/// A widget that can be picked up, as the shell sees it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DragSource {
    /// The widget's identity.
    pub id: WidgetId,
    /// Where it sits, in absolute coordinates.
    pub rect: Rect,
    /// What it carries.
    pub payload: u64,
}

/// A widget that can be dropped onto, as the shell sees it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DropZone {
    /// The widget's identity.
    pub id: WidgetId,
    /// Where it sits, in absolute coordinates.
    pub rect: Rect,
}

/// A widget that can be **picked up** and dropped on a [`DragTarget`].
///
/// It carries a `payload`: a number the application chooses and gets back on the
/// drop, so the two ends of the gesture never have to know each other's types. What
/// floats under the pointer is the widget's own appearance, lifted out of the frame
/// — nothing to build twice, and nothing that can drift from what is on screen.
pub struct Draggable<Msg = crate::callback::Callback> {
    payload: u64,
    long_press: bool,
    ghost_opacity: f32,
    on_dropped: Option<Box<dyn Fn(bool) -> Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Draggable<Msg> {
    /// Makes `child` draggable, carrying `0` until [`Draggable::payload`] says
    /// otherwise.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            payload: 0,
            long_press: false,
            ghost_opacity: GHOSTED_OPACITY,
            on_dropped: None,
            children: vec![Box::new(child)],
        }
    }

    /// What this item carries — an id, an index, anything the application can map
    /// back. It is handed to the target's `on_drop` untouched.
    pub fn payload(mut self, payload: u64) -> Self {
        self.payload = payload;
        self
    }

    /// Lift on a **long press** rather than on the first movement.
    ///
    /// This is what a draggable inside a list needs. Both gestures start with a finger
    /// going down on the same row, and only one of them can win; resolving it by "who
    /// is on top" would hand every drag to the item and leave the list unscrollable.
    /// A hold is the one signal the scroll cannot claim — a finger that stays still is
    /// not scrolling — so it is the hold that lifts.
    pub fn long_press(mut self) -> Self {
        self.long_press = true;
        self
    }

    /// How solid the item left behind looks while its copy is being dragged. `1.0`
    /// leaves it untouched, `0.0` makes the lift look like a removal.
    pub fn ghost_opacity(mut self, opacity: f32) -> Self {
        self.ghost_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// The message sent when the drag ends, `true` if a target took it.
    ///
    /// A refused drop is worth knowing about: it is the moment to say why, and the
    /// only difference between "nothing happened" and "that is not allowed here".
    pub fn on_dropped<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        message: impl Fn(bool) -> R + 'static,
    ) -> Self {
        self.on_dropped = Some(Box::new(crate::callback::handler1(message)));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Draggable<Msg> {
    fn style(&self) -> Style {
        sizing_of(
            self.children
                .first()
                .map(|child| child.style())
                .unwrap_or_default(),
        )
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Nothing of its own: the child is the item.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    // The **structural** questions, forwarded from the child. A wrapper that answered
    // them for itself would change how its content is laid out: a `Dismissible` is a
    // layout leaf, so wrapping it in an ordinary container leaves it with no content
    // size and it collapses to nothing on the wrapper's main axis — silently, since
    // neither draws a box of its own. Found on a device: a task row wrapped for
    // dragging vanished while its checkbox still counted. The same lesson as `Keyed`,
    // which had it the other way round.
    fn stack(&self) -> bool {
        self.children.first().is_some_and(|child| child.stack())
    }

    fn continuous(&self) -> bool {
        self.children
            .first()
            .is_some_and(|child| child.continuous())
    }

    fn drag_payload(&self) -> Option<u64> {
        Some(self.payload)
    }

    fn drag_needs_long_press(&self) -> bool {
        self.long_press
    }

    fn drag_ghost_opacity(&self) -> f32 {
        self.ghost_opacity
    }

    fn on_dropped(&self, accepted: bool) -> Option<Msg> {
        self.on_dropped.as_ref().map(|make| make(accepted))
    }
}

/// Which sets of dropped files a target takes; see [`DragTarget::accepts_files`].
type FilesPredicate = Box<dyn Fn(&[PathBuf]) -> bool>;

/// A widget that can be **dropped onto**.
///
/// By default it takes anything. `accepts` narrows that — a column that only takes
/// tasks of its own project, say — and a target that refuses is not highlighted and
/// not offered the drop, so the answer is visible before the finger lifts rather
/// than after.
///
/// It can also take **files dragged onto the window** from outside the application —
/// from the system's file manager, say — once it is given [`DragTarget::on_files`]:
///
/// ```ignore
/// DragTarget::new(drop_area)
///     .on_files(Msg::Attach)
///     .accepts_files(|paths| paths.iter().all(|p| p.extension().is_some_and(|e| e == "png")))
/// ```
pub struct DragTarget<Msg = crate::callback::Callback> {
    accepts: Option<Box<dyn Fn(u64) -> bool>>,
    on_drop: Option<Box<dyn Fn(u64) -> Msg>>,
    accepts_files: Option<FilesPredicate>,
    on_files: Option<Box<dyn Fn(Vec<PathBuf>) -> Msg>>,
    highlight: Option<Color>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> DragTarget<Msg> {
    /// Makes `child` a drop target.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            accepts: None,
            on_drop: None,
            accepts_files: None,
            on_files: None,
            highlight: None,
            children: vec![Box::new(child)],
        }
    }

    /// The message sent when an accepted item is dropped here, given its payload.
    pub fn on_drop<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        message: impl Fn(u64) -> R + 'static,
    ) -> Self {
        self.on_drop = Some(Box::new(crate::callback::handler1(message)));
        self
    }

    /// Which payloads this target takes; everything, by default.
    pub fn accepts(mut self, predicate: impl Fn(u64) -> bool + 'static) -> Self {
        self.accepts = Some(Box::new(predicate));
        self
    }

    /// The message sent when **files** dragged onto the window from outside the
    /// application are dropped here, given their paths, in the order the system
    /// listed them.
    ///
    /// While the files are held over this target it shows the same "drop it here"
    /// highlight as for an item from inside. A target given this and neither
    /// [`DragTarget::on_drop`] nor [`DragTarget::accepts`] takes **only** files: it is
    /// a place for files, and lighting up under a card being moved would say
    /// otherwise.
    ///
    /// Files are delivered on the desktop. On the web and on mobile, where nothing is
    /// dragged onto the window this way, the message is never sent.
    pub fn on_files<R: crate::callback::IntoMsg<Msg>>(
        mut self,
        message: impl Fn(Vec<PathBuf>) -> R + 'static,
    ) -> Self {
        self.on_files = Some(Box::new(crate::callback::handler1(message)));
        self
    }

    /// Which sets of files this target takes; every set, by default, once it has
    /// [`DragTarget::on_files`]. The predicate sees every path being dragged, so a
    /// target that wants images can refuse a drag that holds anything else — and is
    /// then not highlighted, so the answer shows before the button is let go.
    pub fn accepts_files(mut self, predicate: impl Fn(&[PathBuf]) -> bool + 'static) -> Self {
        self.accepts_files = Some(Box::new(predicate));
        self
    }

    /// The colour of the "drop it here" highlight. Unset, it is the theme's accent;
    /// set it to [`Color::TRANSPARENT`] to draw none and paint your own from the
    /// child.
    pub fn highlight(mut self, color: Color) -> Self {
        self.highlight = Some(color);
        self
    }
}

impl<Msg: Clone> Widget<Msg> for DragTarget<Msg> {
    fn style(&self) -> Style {
        sizing_of(
            self.children
                .first()
                .map(|child| child.style())
                .unwrap_or_default(),
        )
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Painted **under** the child, since `paint` runs before the children are
        // walked: a wash over the target's own background rather than over its text.
        if !status.drag_over {
            return;
        }
        let color = self.highlight.unwrap_or(theme.primary);
        if color.a <= 0.0 {
            return;
        }
        scene.draw_rect(
            bounds,
            color.fade(TARGET_ALPHA),
            theme.radius,
            TARGET_OUTLINE,
            color,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    // The **structural** questions, forwarded from the child. A wrapper that answered
    // them for itself would change how its content is laid out: a `Dismissible` is a
    // layout leaf, so wrapping it in an ordinary container leaves it with no content
    // size and it collapses to nothing on the wrapper's main axis — silently, since
    // neither draws a box of its own. Found on a device: a task row wrapped for
    // dragging vanished while its checkbox still counted. The same lesson as `Keyed`,
    // which had it the other way round.
    fn stack(&self) -> bool {
        self.children.first().is_some_and(|child| child.stack())
    }

    fn continuous(&self) -> bool {
        self.children
            .first()
            .is_some_and(|child| child.continuous())
    }

    fn drop_zone(&self) -> bool {
        true
    }

    fn accepts_drag(&self, payload: u64) -> bool {
        match &self.accepts {
            Some(predicate) => predicate(payload),
            // A place for files only, unless it said it takes items too.
            None => self.on_drop.is_some() || self.on_files.is_none(),
        }
    }

    fn on_drop(&self, payload: u64) -> Option<Msg> {
        self.on_drop.as_ref().map(|make| make(payload))
    }

    fn accepts_files(&self, paths: &[PathBuf]) -> bool {
        if self.on_files.is_none() || paths.is_empty() {
            return false;
        }
        match &self.accepts_files {
            Some(predicate) => predicate(paths),
            None => true,
        }
    }

    fn on_files_dropped(&self, paths: Vec<PathBuf>) -> Option<Msg> {
        self.on_files.as_ref().map(|make| make(paths))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Flex, Runtime};
    use frus_core::{Point, Primitive, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Dropped(u64),
        Ended(bool),
        Files(Vec<PathBuf>),
    }

    fn tree() -> Flex<Msg> {
        Flex::column()
            .width(200.0)
            .child(
                Draggable::new(Container::new().width(100.0).height(40.0))
                    .payload(7)
                    .on_dropped(Msg::Ended),
            )
            .child(
                DragTarget::new(Container::new().width(200.0).height(60.0))
                    .on_drop(Msg::Dropped)
                    .accepts(|payload| payload % 2 == 1),
            )
    }

    fn ui(runtime: &Runtime) -> crate::Ui<Msg> {
        build_ui(&tree(), Size::new(200.0, 200.0), runtime, &Theme::dark())
    }

    #[test]
    fn a_draggable_registers_what_it_carries_and_where_it_is() {
        let runtime = Runtime::default();
        let ui = ui(&runtime);
        let source = ui
            .drag_source_at(Point::new(50.0, 20.0))
            .expect("a source under the pointer");
        assert_eq!(source.payload, 7);
        assert!((source.rect.height - 40.0).abs() < 0.5);
        // Below the item there is no source.
        assert!(ui.drag_source_at(Point::new(50.0, 70.0)).is_none());
    }

    #[test]
    fn a_draggable_says_how_it_wants_to_be_lifted() {
        let plain = Draggable::<Msg>::new(Container::new());
        assert!(!Widget::<Msg>::drag_needs_long_press(&plain));
        let held = Draggable::<Msg>::new(Container::new()).long_press();
        assert!(Widget::<Msg>::drag_needs_long_press(&held));
    }

    #[test]
    fn a_draggable_row_inside_a_column_still_paints() {
        use crate::{keyed, Dismissible, Flex};
        let red = Color::rgb(1.0, 0.0, 0.0);
        let row = Dismissible::<Msg>::new(Container::new().color(red).height(62.0))
            .height(62.0)
            .on_dismiss(Msg::Dropped(1));
        let root = Flex::<Msg>::column()
            .width(300.0)
            .child(keyed(1u64, Draggable::new(row).payload(1).long_press()));
        let runtime = Runtime::default();
        let ui = build_ui(&root, Size::new(300.0, 400.0), &runtime, &Theme::dark());
        let painted: Vec<Rect> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 && color.g < 0.5 => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        assert!(
            !painted.is_empty(),
            "the row vanished ({} primitives in all)",
            ui.scene().primitives().len()
        );
        assert!((painted[0].height - 62.0).abs() < 1.0, "{:?}", painted[0]);
    }

    /// A target wrapping ordinary content keeps that content's height — it must not
    /// become a layout leaf just because some other child would need it to be.
    #[test]
    fn a_target_around_ordinary_content_is_as_tall_as_that_content() {
        use crate::Flex;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let root = Flex::<Msg>::column().width(300.0).child(DragTarget::new(
            Container::new()
                .padding(12.0)
                .color(red)
                .child(Container::new().height(20.0)),
        ));
        let runtime = Runtime::default();
        let ui = build_ui(&root, Size::new(300.0, 400.0), &runtime, &Theme::dark());
        let painted = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 && color.g < 0.5 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the target's content");
        assert!(
            (painted.height - 44.0).abs() < 1.0,
            "20 px of content plus 12 either side: {painted:?}"
        );
    }

    #[test]
    fn a_target_is_found_by_where_it_is() {
        let runtime = Runtime::default();
        let ui = ui(&runtime);
        assert!(ui.drop_zone_at(Point::new(100.0, 60.0)).is_some());
        assert!(ui.drop_zone_at(Point::new(100.0, 10.0)).is_none());
    }

    #[test]
    fn a_target_refuses_what_it_says_it_refuses() {
        let target = DragTarget::<Msg>::new(Container::new()).accepts(|payload| payload % 2 == 1);
        assert!(Widget::<Msg>::accepts_drag(&target, 7));
        assert!(!Widget::<Msg>::accepts_drag(&target, 8));
        // With no predicate, everything is welcome.
        let open = DragTarget::<Msg>::new(Container::new());
        assert!(Widget::<Msg>::accepts_drag(&open, 8));
    }

    #[test]
    fn the_payload_reaches_the_drop_untouched() {
        // Spelled through the trait: the builder of the same name would win otherwise.
        let target = DragTarget::new(Container::new()).on_drop(Msg::Dropped);
        assert_eq!(Widget::<Msg>::on_drop(&target, 7), Some(Msg::Dropped(7)));
        // A target with no handler is still a target; it just has nothing to say.
        let silent = DragTarget::<Msg>::new(Container::new());
        assert_eq!(Widget::<Msg>::on_drop(&silent, 7), None);
    }

    #[test]
    fn a_refused_drop_is_reported_as_such() {
        let source = Draggable::new(Container::new()).on_dropped(Msg::Ended);
        assert_eq!(
            Widget::<Msg>::on_dropped(&source, true),
            Some(Msg::Ended(true))
        );
        assert_eq!(
            Widget::<Msg>::on_dropped(&source, false),
            Some(Msg::Ended(false))
        );
    }

    #[test]
    fn a_target_is_only_highlighted_while_a_drag_is_over_it() {
        let quiet = Runtime::default();
        let plain = ui(&quiet).scene().primitives().len();

        let mut hovered = Runtime::default();
        let zone = ui(&quiet)
            .drop_zone_at(Point::new(100.0, 60.0))
            .expect("the target");
        hovered.drag_over = Some(zone.id);
        let lit = ui(&hovered);
        assert_eq!(
            lit.scene().primitives().len(),
            plain + 1,
            "exactly one highlight, and only when hovered"
        );
        // And it covers the target, rather than being a stray mark somewhere.
        let highlight = lit
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } if (rect.height - 60.0).abs() < 0.5 => Some(*rect),
                _ => None,
            })
            .next()
            .expect("the highlight");
        assert!((highlight.width - 200.0).abs() < 0.5, "{highlight:?}");
    }
    #[test]
    fn a_target_takes_no_files_unless_it_says_so() {
        let paths = vec![PathBuf::from("a.txt")];
        let plain = DragTarget::new(Container::new()).on_drop(Msg::Dropped);
        assert!(!Widget::<Msg>::accepts_files(&plain, &paths));
        assert_eq!(Widget::<Msg>::on_files_dropped(&plain, paths.clone()), None);
        let files = DragTarget::new(Container::new()).on_files(Msg::Files);
        assert!(Widget::<Msg>::accepts_files(&files, &paths));
        // Nothing dragged is nothing to take.
        assert!(!Widget::<Msg>::accepts_files(&files, &[]));
    }

    #[test]
    fn the_paths_reach_the_drop_untouched_and_in_order() {
        let paths = vec![PathBuf::from("b.png"), PathBuf::from("a.png")];
        let target = DragTarget::new(Container::new()).on_files(Msg::Files);
        assert_eq!(
            Widget::<Msg>::on_files_dropped(&target, paths.clone()),
            Some(Msg::Files(paths))
        );
    }

    #[test]
    fn a_target_refuses_the_files_it_says_it_refuses() {
        let images = DragTarget::new(Container::new())
            .on_files(Msg::Files)
            .accepts_files(|paths| {
                paths
                    .iter()
                    .all(|p| p.extension().is_some_and(|e| e == "png"))
            });
        let png = [PathBuf::from("a.png"), PathBuf::from("b.png")];
        let mixed = [PathBuf::from("a.png"), PathBuf::from("b.txt")];
        assert!(Widget::<Msg>::accepts_files(&images, &png));
        assert!(!Widget::<Msg>::accepts_files(&images, &mixed));
    }

    #[test]
    fn a_target_for_files_only_turns_items_away() {
        let files = DragTarget::new(Container::new()).on_files(Msg::Files);
        assert!(!Widget::<Msg>::accepts_drag(&files, 7));
        // Unless it takes items as well, or says which.
        let both = DragTarget::new(Container::new())
            .on_files(Msg::Files)
            .on_drop(Msg::Dropped);
        assert!(Widget::<Msg>::accepts_drag(&both, 7));
        let picky = DragTarget::new(Container::new())
            .on_files(Msg::Files)
            .accepts(|payload| payload == 7);
        assert!(Widget::<Msg>::accepts_drag(&picky, 7));
        // And a target with neither handler stays what it was: open to anything.
        let open = DragTarget::<Msg>::new(Container::new());
        assert!(Widget::<Msg>::accepts_drag(&open, 7));
    }

    #[test]
    fn a_wrapped_target_still_takes_files() {
        let paths = vec![PathBuf::from("a.txt")];
        let wrapped = crate::keyed(1u64, DragTarget::new(Container::new()).on_files(Msg::Files));
        assert!(Widget::<Msg>::accepts_files(&wrapped, &paths));
        assert_eq!(
            Widget::<Msg>::on_files_dropped(&wrapped, paths.clone()),
            Some(Msg::Files(paths))
        );
    }
}
