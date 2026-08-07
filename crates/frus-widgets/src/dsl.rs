//! Syntactic sugar for describing an interface **faster**: the
//! [`row!`](crate::row) and [`column!`](crate::column) macros, plus a few shorthand
//! functions. Purely additive — the constructors remain available.

use std::hash::Hash;

use crate::button::Button;
use crate::flex::Flex;
use crate::keyed::Keyed;
use crate::text::Text;
use crate::widget::Widget;

/// Raccourci : un widget de texte. `text("Bonjour")` = `Text::new("Bonjour")`.
pub fn text(content: impl Into<String>) -> Text {
    Text::new(content)
}

/// Raccourci : un **espaceur** flexible qui pousse ses voisins (`flex: 1`).
pub fn spacer<Msg>() -> Flex<Msg> {
    Flex::row().flex(1.0)
}

/// Raccourci : un bouton avec son message au clic.
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
