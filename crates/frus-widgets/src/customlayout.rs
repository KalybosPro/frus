//! [`CustomMultiChildLayout`]: children placed by a function the application writes, for
//! arrangements a row, a column, a stack or a grid cannot express (milestone 601).

use std::hash::{Hash, Hasher};

use frus_core::{Point, Rect, Scene, Size};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{FillAxes, Widget};

/// What a child is given on one axis by a [`ChildLayout::layout_child`] call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChildAxis {
    /// As big as it likes.
    Free,
    /// At most this many pixels: a paragraph wraps there, a block smaller than it keeps its
    /// size.
    AtMost(f32),
    /// This many pixels. A child with a size of its own on this axis keeps it, as a stack's
    /// layer does.
    Exactly(f32),
}

/// What a child is given, per axis, by [`ChildLayout::layout_child`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChildConstraints {
    /// Across.
    pub width: ChildAxis,
    /// Down.
    pub height: ChildAxis,
}

impl ChildConstraints {
    /// As big as it likes, on both axes.
    pub fn free() -> Self {
        Self {
            width: ChildAxis::Free,
            height: ChildAxis::Free,
        }
    }

    /// At most `size`.
    pub fn loose(size: Size) -> Self {
        Self {
            width: ChildAxis::AtMost(size.width),
            height: ChildAxis::AtMost(size.height),
        }
    }

    /// Exactly `size`.
    pub fn tight(size: Size) -> Self {
        Self {
            width: ChildAxis::Exactly(size.width),
            height: ChildAxis::Exactly(size.height),
        }
    }

    /// The same, with `width` across.
    pub fn width(mut self, width: ChildAxis) -> Self {
        self.width = width;
        self
    }

    /// The same, with `height` down.
    pub fn height(mut self, height: ChildAxis) -> Self {
        self.height = height;
        self
    }
}

/// The key a child is named by, from whatever the application named it with.
pub(crate) fn key_of(key: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// What a [`CustomMultiChildLayout`]'s function is handed: its children, by key, to measure
/// and to place.
pub struct ChildLayout<'a> {
    keys: &'a [u64],
    measure: &'a dyn Fn(usize, ChildConstraints) -> Vec<Rect>,
    pub(crate) laid: Vec<Option<Vec<Rect>>>,
    pub(crate) at: Vec<Point>,
}

impl<'a> ChildLayout<'a> {
    pub(crate) fn new(
        keys: &'a [u64],
        measure: &'a dyn Fn(usize, ChildConstraints) -> Vec<Rect>,
    ) -> Self {
        Self {
            keys,
            measure,
            laid: vec![None; keys.len()],
            at: vec![Point::new(0.0, 0.0); keys.len()],
        }
    }

    fn index(&self, key: impl Hash) -> Option<usize> {
        let key = key_of(key);
        self.keys.iter().position(|k| *k == key)
    }

    /// Whether a child was given under `key`: a layout whose children come and go asks
    /// before it measures.
    pub fn has_child(&self, key: impl Hash) -> bool {
        self.index(key).is_some()
    }

    /// Lays the child named `key` out under `constraints` and says how big it came out. A
    /// child never laid out is not shown; one laid out twice keeps the second answer.
    ///
    /// # Panics
    ///
    /// If no child was given under `key`, as the reference's does: the layout is asking
    /// about something that is not there, and a size made up for it would be placed.
    pub fn layout_child(&mut self, key: impl Hash, constraints: ChildConstraints) -> Size {
        let i = self
            .index(key)
            .expect("layout_child: no child was given under this key");
        let rects = (self.measure)(i, constraints);
        let size = rects
            .first()
            .map(|r| Size::new(r.width, r.height))
            .unwrap_or(Size::new(0.0, 0.0));
        self.laid[i] = Some(rects);
        size
    }

    /// Puts the child named `key` with its top left corner at `at`, from the layout's own
    /// top left. A child laid out and never placed is at the corner.
    ///
    /// # Panics
    ///
    /// If no child was given under `key`.
    pub fn position_child(&mut self, key: impl Hash, at: Point) {
        let i = self
            .index(key)
            .expect("position_child: no child was given under this key");
        self.at[i] = at;
    }
}

/// The function that places a [`CustomMultiChildLayout`]'s children: its own size, and the
/// children to measure and place.
pub type LayoutFn = dyn Fn(Size, &mut ChildLayout<'_>);

/// A layout a hook hands the walk: the function and the keys, one per child.
pub struct CustomLayout<'a> {
    /// The function.
    pub delegate: &'a LayoutFn,
    /// One key per child, in order.
    pub keys: &'a [u64],
}

/// **Places its children with a function the application writes**: a circle of buttons, a
/// fan of cards, a bubble whose tail points at its neighbour, nodes placed by a solver.
///
/// Each child is named by a key. The function is given the layout's size and, for each
/// child, asks how big it is under constraints it chooses ([`ChildLayout::layout_child`]),
/// then says where it goes ([`ChildLayout::position_child`]). A child can be measured before
/// another is placed, so one can be put against another's size.
///
/// ```
/// use frus_core::{Point, Size};
/// use frus_widgets::{ChildConstraints, CustomMultiChildLayout, Text};
///
/// // A caption under a title, indented by the title's height.
/// let _card = CustomMultiChildLayout::<()>::new(|size, layout| {
///     let title = layout.layout_child("title", ChildConstraints::loose(size));
///     layout.position_child("title", Point::new(0.0, 0.0));
///     let room = Size::new(size.width - title.height, size.height - title.height);
///     layout.layout_child("caption", ChildConstraints::loose(room));
///     layout.position_child("caption", Point::new(title.height, title.height));
/// })
/// .child("title", Text::new("Seat map"))
/// .child("caption", Text::new("Row 12"));
/// ```
///
/// **Its own size does not depend on its children**, as the reference's does not: it takes
/// the room it is given, or the size set with [`width`](Self::width) and
/// [`height`](Self::height). Children are painted in the order they were given, and a child
/// the function never lays out is not shown. Nothing is cut at the layout's edge.
pub struct CustomMultiChildLayout<Msg = crate::callback::Callback> {
    delegate: Box<LayoutFn>,
    keys: Vec<u64>,
    children: Vec<Box<dyn Widget<Msg>>>,
    width: Option<f32>,
    height: Option<f32>,
}

impl<Msg> CustomMultiChildLayout<Msg> {
    /// A layout placed by `delegate`, with no children yet.
    pub fn new(delegate: impl Fn(Size, &mut ChildLayout<'_>) + 'static) -> Self {
        Self {
            delegate: Box::new(delegate),
            keys: Vec::new(),
            children: Vec::new(),
            width: None,
            height: None,
        }
    }

    /// Adds `child` under `key`, which the function names it by. A key already given is
    /// given again: the first child under it is the one the function reaches.
    pub fn child(mut self, key: impl Hash, child: impl Widget<Msg> + 'static) -> Self {
        self.keys.push(key_of(key));
        self.children.push(Box::new(child));
        self
    }

    /// A width of its own, instead of the room.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(0.0));
        self
    }

    /// A height of its own, instead of the room.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height.max(0.0));
        self
    }
}

impl<Msg: Clone> Widget<Msg> for CustomMultiChildLayout<Msg> {
    fn style(&self) -> Style {
        let dimension = |extent: Option<f32>| extent.map_or(Dimension::Auto, Dimension::Length);
        Style {
            width: dimension(self.width),
            height: dimension(self.height),
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn custom_layout(&self) -> Option<CustomLayout<'_>> {
        Some(CustomLayout {
            delegate: self.delegate.as_ref(),
            keys: &self.keys,
        })
    }

    /// The room, on the axes it has no size of its own on: the reference's layout is as big
    /// as it is allowed.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes {
            horizontal: self.width.is_none(),
            vertical: self.height.is_none(),
        }
    }

    fn debug_name(&self) -> &'static str {
        "CustomMultiChildLayout"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Flex, Runtime, Text};
    use frus_core::{Color, Primitive};

    const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

    fn block(width: f32, height: f32, color: Color) -> Container<()> {
        Container::new().width(width).height(height).color(color)
    }

    /// Every rect of `color` painted.
    fn painted(root: &dyn Widget<()>, size: Size, color: Color) -> Vec<Rect> {
        build_ui(root, size, &Runtime::default(), &Theme::dark())
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color: c, .. } if *c == color => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// **Measured, then placed against another's size**: a blue block goes under the red
    /// one and against the right edge, from sizes the function asked for.
    #[test]
    fn a_child_is_placed_against_another_childs_size() {
        let root = CustomMultiChildLayout::<()>::new(|size, layout| {
            let red = layout.layout_child("red", ChildConstraints::free());
            layout.position_child("red", Point::new(10.0, 10.0));
            let blue = layout.layout_child("blue", ChildConstraints::free());
            layout.position_child(
                "blue",
                Point::new(size.width - blue.width, 10.0 + red.height),
            );
        })
        .child("red", block(40.0, 30.0, RED))
        .child("blue", block(20.0, 20.0, BLUE));
        let size = Size::new(200.0, 100.0);
        let red = painted(&root, size, RED)[0];
        let blue = painted(&root, size, BLUE)[0];
        assert_eq!(
            (red.x, red.y, red.width, red.height),
            (10.0, 10.0, 40.0, 30.0)
        );
        assert_eq!((blue.x, blue.y), (180.0, 40.0));
    }

    /// **A child the function never lays out is not shown**; one laid out and never placed
    /// is at the corner.
    #[test]
    fn a_child_left_out_is_not_shown() {
        let root = CustomMultiChildLayout::<()>::new(|_, layout| {
            layout.layout_child("red", ChildConstraints::free());
        })
        .child("red", block(40.0, 30.0, RED))
        .child("blue", block(20.0, 20.0, BLUE));
        let size = Size::new(200.0, 100.0);
        assert_eq!(painted(&root, size, RED)[0].x, 0.0);
        assert!(painted(&root, size, BLUE).is_empty());
    }

    /// **Its own size is the room, or its own**, never its children's: in a column, what
    /// follows a 50-px-tall layout starts at 50 whatever is inside, and the function is
    /// handed the column's width.
    #[test]
    fn its_size_is_the_room_or_its_own() {
        let seen = std::rc::Rc::new(std::cell::Cell::new(Size::new(0.0, 0.0)));
        let told = seen.clone();
        let root = Flex::<()>::column()
            .width(160.0)
            .child(
                CustomMultiChildLayout::new(move |size, layout| {
                    told.set(size);
                    layout.layout_child("red", ChildConstraints::free());
                })
                .height(50.0)
                .child("red", block(40.0, 300.0, RED)),
            )
            .child(block(10.0, 10.0, BLUE));
        let blue = painted(&root, Size::new(300.0, 400.0), BLUE)[0];
        assert_eq!(blue.y, 50.0);
        assert_eq!(seen.get(), Size::new(160.0, 50.0));
    }

    /// **Exactly** gives a child with no size of its own that size; **at most** lets a
    /// paragraph wrap there.
    #[test]
    fn exact_and_at_most_constraints() {
        let words = "one two three four five six seven eight nine ten eleven twelve";
        let root = CustomMultiChildLayout::<()>::new(|_, layout| {
            layout.layout_child("red", ChildConstraints::tight(Size::new(70.0, 30.0)));
            let text =
                layout.layout_child("text", ChildConstraints::loose(Size::new(120.0, 400.0)));
            assert!(text.width <= 120.0 && text.height > 50.0, "{text:?}");
            layout.position_child("text", Point::new(0.0, 40.0));
        })
        .child("red", Container::new().color(RED))
        .child("text", Text::new(words));
        let red = painted(&root, Size::new(300.0, 300.0), RED)[0];
        assert_eq!((red.width, red.height), (70.0, 30.0));
    }

    /// **A placed child takes taps where it was put**, and not where it would have been.
    #[test]
    fn a_placed_child_takes_taps_where_it_is() {
        let root = CustomMultiChildLayout::<()>::new(|_, layout| {
            layout.layout_child("button", ChildConstraints::free());
            layout.position_child("button", Point::new(100.0, 50.0));
        })
        .child("button", block(40.0, 20.0, RED).on_click(()));
        let ui = build_ui(
            &root,
            Size::new(200.0, 100.0),
            &Runtime::default(),
            &Theme::dark(),
        );
        assert!(ui.hit(Point::new(110.0, 60.0)).is_some());
        assert!(ui.hit(Point::new(10.0, 10.0)).is_none());
    }

    /// **Asking about a key that was never given** is a mistake in the function, and says so.
    #[test]
    #[should_panic(expected = "no child was given under this key")]
    fn an_unknown_key_panics() {
        let root = CustomMultiChildLayout::<()>::new(|_, layout| {
            assert!(!layout.has_child("missing"));
            layout.layout_child("missing", ChildConstraints::free());
        });
        painted(&root, Size::new(100.0, 100.0), RED);
    }

    /// **Through a key**, the layout is still one: a keyed wrapper passes the question on.
    #[test]
    fn a_keyed_layout_still_places_its_children() {
        let root = crate::keyed(
            7,
            CustomMultiChildLayout::<()>::new(|_, layout| {
                layout.layout_child("red", ChildConstraints::free());
                layout.position_child("red", Point::new(30.0, 20.0));
            })
            .child("red", block(40.0, 30.0, RED)),
        );
        let red = painted(&root, Size::new(200.0, 100.0), RED)[0];
        assert_eq!((red.x, red.y), (30.0, 20.0));
    }
}
