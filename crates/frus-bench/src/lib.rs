//! The screens the benchmarks measure, in one place so that every bench measures the
//! *same* trees and a number from one run means the same as a number from the next.
//!
//! Nothing here is part of frus. It exists so that `benches/*.rs` do not each invent
//! their own idea of "a realistic screen", which is how benchmark suites end up
//! measuring their own fixtures.

use frus_core::Size;
use frus_widgets::{
    build_ui, button, row, text, Align, Card, Checkbox, Container, Flex, Icon, Icons, Runtime,
    Theme, Ui, Widget,
};

/// The window every screen here is built for.
pub const VIEWPORT: Size = Size {
    width: 400.0,
    height: 800.0,
};

/// A list of `rows` task rows inside a card — a checkbox, a label, a spacer, an icon
/// and a button apiece. This is the shape frus actually draws, and the same tree the
/// batching guard in `frus-test` uses, so the two numbers can be read together.
pub fn task_list(rows: usize) -> Container<()> {
    let mut list = Flex::column().gap(8.0);
    for i in 0..rows {
        list = list.child_boxed(Box::new(
            row![
                Checkbox::new(i % 2 == 0),
                text(format!("Task number {i}")).size(15.0),
                Container::new().flex(1.0),
                Icon::new(Icons::CLOSE).size(16.0),
                button("Open", ()),
            ]
            .gap(12.0)
            .align(Align::Center)
            .height(48.0),
        ));
    }
    Container::new()
        .width(VIEWPORT.width)
        .height(VIEWPORT.height)
        .padding(16.0)
        .child(Card::new().padding(12.0).child(list))
}

/// The same list with every string replaced by a box of the size the string would
/// have taken. Identical widget count, identical layout work — no shaping. The gap
/// between this and [`task_list`] is what measuring text costs per frame.
pub fn task_list_wordless(rows: usize) -> Container<()> {
    let mut list = Flex::column().gap(8.0);
    for i in 0..rows {
        list = list.child_boxed(Box::new(
            Flex::row()
                .child(Checkbox::new(i % 2 == 0))
                .child(Container::new().width(120.0).height(18.0))
                .child(Container::new().flex(1.0))
                .child(Icon::new(Icons::CLOSE).size(16.0))
                .child(Container::new().width(56.0).height(32.0))
                .gap(12.0)
                .align(Align::Center)
                .height(48.0),
        ));
    }
    Container::new()
        .width(VIEWPORT.width)
        .height(VIEWPORT.height)
        .padding(16.0)
        .child(Card::new().padding(12.0).child(list))
}

/// A tree `depth` containers deep with a line of text at the bottom. Layout cost grows
/// with nesting even when the drawn output does not, and this is what separates the
/// two.
pub fn nested(depth: usize) -> Container<()> {
    let mut node: Box<dyn Widget<()>> = Box::new(text("the bottom of the tree").size(14.0));
    for _ in 0..depth {
        node = Box::new(Container::new().padding(1.0).child(node));
    }
    Container::new()
        .width(VIEWPORT.width)
        .height(VIEWPORT.height)
        .child(node)
}

/// Builds a tree the way the shell would, on a neutral runtime.
pub fn build(root: &dyn Widget<()>) -> Ui<()> {
    build_ui(root, VIEWPORT, &Runtime::default(), &Theme::dark())
}
