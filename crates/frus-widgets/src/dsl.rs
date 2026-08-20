//! Syntactic sugar for describing an interface **faster**: the
//! [`row!`](crate::row) and [`column!`](crate::column) macros, plus a few shorthand
//! functions. Purely additive — the constructors remain available.

use std::hash::Hash;

use crate::button::Button;
use crate::expanded::{Expanded, Flexible};
use crate::flex::Flex;
use crate::keyed::Keyed;
use crate::text::Text;
use crate::widget::Widget;

/// Shorthand: a text widget. `text("Hello")` = `Text::new("Hello")`.
pub fn text(content: impl Into<String>) -> Text {
    Text::new(content)
}

/// Shorthand: a flexible **spacer** that pushes its neighbours apart (`flex: 1`).
pub fn spacer<Msg>() -> Flex<Msg> {
    Flex::row().flex(1.0)
}

/// Shorthand: the child that takes the room its siblings left.
/// `expanded(text(&title).ellipsis())` = `Expanded::new(...)`.
pub fn expanded<Msg>(child: impl Widget<Msg> + 'static) -> Expanded<Msg> {
    Expanded::new(child)
}

/// Shorthand: a child that takes **at most** the room its siblings left, and less if it
/// wants less. `flexible(child)` = `Flexible::new(child)`.
pub fn flexible<Msg>(child: impl Widget<Msg> + 'static) -> Flexible<Msg> {
    Flexible::new(child)
}

/// Shorthand: a button with its click message.
/// `button("Ajouter", Msg::Add)` = `Button::new("Ajouter").on_press(Msg::Add)`.
pub fn button<Msg>(label: impl Into<String>, on_press: Msg) -> Button<Msg> {
    Button::new(label).on_press(on_press)
}

/// Shorthand: gives a widget — a list item, say — a **stable identity**.
/// `keyed(todo.id, todo_row(todo))` is `Keyed::new(todo.id, todo_row(todo))`.
pub fn keyed<Msg>(key: impl Hash, widget: impl Widget<Msg> + 'static) -> Keyed<Msg> {
    Keyed::new(key, widget)
}

/// A flex **row**. `row![a, b, c]` is `Flex::row().child(a).child(b).child(c)`.
/// The result stays chainable: `row![a, b].gap(8.0).align(...)`.
#[macro_export]
macro_rules! row {
    () => { $crate::Flex::row() };
    ($($child:expr),+ $(,)?) => {
        $crate::Flex::row()$(.child($child))+
    };
}

/// A flex **column**. `column![a, b, c]` is `Flex::column().child(a)…`.
/// The result stays chainable: `column![a, b].gap(16.0).padding(20.0)`.
#[macro_export]
macro_rules! column {
    () => { $crate::Flex::column() };
    ($($child:expr),+ $(,)?) => {
        $crate::Flex::column()$(.child($child))+
    };
}

/// An image **embedded from a file**, by path. `asset!("../assets/logo.png")` is
/// `Image::memory(include_bytes!("../assets/logo.png"))`, and like every `include_*!`
/// the path is relative to the **file that writes it**.
///
/// ```ignore
/// asset!("../assets/logo.png").width(96.0).semantic_label("frus")
/// ```
///
/// The bytes go into the binary at compile time, so there is no file to find at run
/// time, no path to get wrong on another machine, and no asset manifest to keep in step
/// with the source. That is what Rust gives here that a language without
/// `include_bytes!` has to build an asset bundle to get.
///
/// The result is a plain [`Image`](crate::Image) and stays chainable. It is decoded once
/// per process; see [`Image::memory`](crate::Image::memory) for the store behind it and
/// for what a file that will not decode does.
#[macro_export]
macro_rules! asset {
    ($path:literal) => {
        $crate::Image::memory(include_bytes!($path))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::Widget;

    #[test]
    fn macros_collect_children() {
        let col: Flex<()> = column![text("a"), text("b"), text("c")];
        assert_eq!(Widget::<()>::children(&col).len(), 3);

        let empty: Flex<()> = row![];
        assert_eq!(Widget::<()>::children(&empty).len(), 0);

        // A chainable result, plus nested rows.
        let nested: Flex<()> =
            column![text("title"), row![text("a"), spacer(), text("b")]].gap(8.0);
        assert_eq!(Widget::<()>::children(&nested).len(), 2);
    }

    #[test]
    fn button_helper_sets_message() {
        #[derive(Clone, PartialEq, Debug)]
        enum Msg {
            Go,
        }
        let b = button("Ok", Msg::Go);
        assert_eq!(Widget::on_click(&b), Some(Msg::Go));
    }
}
