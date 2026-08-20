//! The **focus** wrappers: which parts of a tree the keyboard can reach, and in what
//! order.
//!
//! Focus already existed here as a property of a widget — a text field is focusable, a
//! label is not — and Tab already walked the tree. What was missing is the declarative
//! surface the reference puts over it: the five wrappers that let a caller say *this
//! subtree is out of reach*, *skip this one with Tab but let a click land on it*, *these
//! come in this order*, and *resolve that order among these and nowhere else*.
//!
//! ```ignore
//! FocusTraversalGroup::new(
//!     column![
//!         FocusTraversalOrder::new(2.0, name_field),   // second, whatever the tree says
//!         FocusTraversalOrder::new(1.0, email_field),  // first
//!         ExcludeFocusTraversal::new(help_button),     // clickable, not in the order
//!     ],
//! )
//! ```
//!
//! All five are **single-child boxes** that take their child's sizing and add nothing to
//! the picture. The flags they set are *subtree-scoped*: the walk pushes them on the way
//! in and pops them on the way out, so a sibling never inherits one.
//!
//! ## The two questions Tab and a click ask separately
//!
//! [`ExcludeFocus`] removes a subtree from focus entirely — nothing in it registers a stop
//! at all. [`ExcludeFocusTraversal`] keeps the stops and takes them out of *Tab's order*:
//! a click still lands, the keyboard passes by. They are separate because the reasons are
//! separate — a panel behind a sheet is unreachable, a toolbar button is reachable but
//! does not belong in a form's keyboard order.

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::{sizing_of, Widget};

/// The shared body of all five: one child, its box, and a set of focus flags.
struct Wrapper<Msg> {
    child: Vec<Box<dyn Widget<Msg>>>,
    focusable: bool,
    descendants_focusable: bool,
    skip_traversal: bool,
    order: Option<f32>,
    group: bool,
}

impl<Msg> Wrapper<Msg> {
    fn new(child: Box<dyn Widget<Msg>>) -> Self {
        Self {
            child: vec![child],
            focusable: false,
            descendants_focusable: true,
            skip_traversal: false,
            order: None,
            group: false,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Wrapper<Msg> {
    fn style(&self) -> Style {
        sizing_of(self.child[0].style())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        sizing_of(self.child[0].style_themed(theme))
    }

    fn build_themed(&self, theme: &Theme) {
        self.child[0].build_themed(theme);
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn focusable(&self) -> bool {
        self.focusable
    }

    fn descendants_focusable(&self) -> bool {
        self.descendants_focusable
    }

    fn focus_skip_traversal(&self) -> bool {
        self.skip_traversal
    }

    fn focus_order(&self) -> Option<f32> {
        self.order
    }

    fn focus_group(&self) -> bool {
        self.group
    }
}

/// Makes its child a **focus stop**: somewhere Tab can land and a click can focus, even
/// when the child itself has no opinion about focus.
pub struct Focus<Msg>(Wrapper<Msg>);

impl<Msg> Focus<Msg> {
    /// A focus stop around `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut w = Wrapper::new(Box::new(child));
        w.focusable = true;
        Self(w)
    }

    /// Whether it can take focus at all — the reference's `canRequestFocus`. `false`
    /// leaves the subtree in place and simply stops being a stop.
    pub fn can_request_focus(mut self, can: bool) -> Self {
        self.0.focusable = can;
        self
    }

    /// Focusable by a click, passed over by Tab.
    pub fn skip_traversal(mut self) -> Self {
        self.0.skip_traversal = true;
        self
    }
}

/// **Nothing inside can take focus.** The subtree is still drawn and still measured; it
/// is simply not somewhere the keyboard can go.
pub struct ExcludeFocus<Msg>(Wrapper<Msg>);

impl<Msg> ExcludeFocus<Msg> {
    /// Puts `child` out of the keyboard's reach.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut w = Wrapper::new(Box::new(child));
        w.descendants_focusable = false;
        Self(w)
    }
}

/// **Skipped by Tab, reachable by a click** — a different question from
/// [`ExcludeFocus`], which takes the subtree out of the keyboard's reach altogether. A
/// panel behind a sheet is unreachable; a toolbar button is reachable and simply does not
/// belong in a form's keyboard order.
pub struct ExcludeFocusTraversal<Msg>(Wrapper<Msg>);

impl<Msg> ExcludeFocusTraversal<Msg> {
    /// Takes `child`'s focus stops out of Tab's order.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut w = Wrapper::new(Box::new(child));
        w.skip_traversal = true;
        Self(w)
    }
}

/// An explicit **traversal position** for a subtree, smallest first.
///
/// Ordered stops come before unordered ones, and everything without an order keeps tree
/// order — so this is a local statement rather than a rearrangement of the frame.
pub struct FocusTraversalOrder<Msg>(Wrapper<Msg>);

impl<Msg> FocusTraversalOrder<Msg> {
    /// Gives `child`'s focus stops the position `order`.
    pub fn new(order: f32, child: impl Widget<Msg> + 'static) -> Self {
        let mut w = Wrapper::new(Box::new(child));
        w.order = Some(order);
        Self(w)
    }
}

/// A **traversal group**: an order set inside it is resolved among its own members and
/// nowhere else, so a reordered dialog does not reshuffle the page behind it.
pub struct FocusTraversalGroup<Msg>(Wrapper<Msg>);

impl<Msg> FocusTraversalGroup<Msg> {
    /// Scopes `child`'s traversal order to `child`.
    pub fn new(child: impl Widget<Msg> + 'static) -> Self {
        let mut w = Wrapper::new(Box::new(child));
        w.group = true;
        Self(w)
    }
}

/// Each of the five is the same box with different flags; the delegation is mechanical
/// and identical, so it is written once.
macro_rules! delegate {
    ($($ty:ident),* $(,)?) => {$(
        impl<Msg: Clone> Widget<Msg> for $ty<Msg> {
            fn style(&self) -> Style {
                Widget::<Msg>::style(&self.0)
            }
            fn style_themed(&self, theme: &Theme) -> Style {
                Widget::<Msg>::style_themed(&self.0, theme)
            }
            fn build_themed(&self, theme: &Theme) {
                Widget::<Msg>::build_themed(&self.0, theme)
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                Widget::<Msg>::children(&self.0)
            }
            fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
                Widget::<Msg>::paint(&self.0, bounds, status, theme, scene)
            }
            fn on_click(&self) -> Option<Msg> {
                Widget::<Msg>::on_click(&self.0)
            }
            fn focusable(&self) -> bool {
                Widget::<Msg>::focusable(&self.0)
            }
            fn descendants_focusable(&self) -> bool {
                Widget::<Msg>::descendants_focusable(&self.0)
            }
            fn focus_skip_traversal(&self) -> bool {
                Widget::<Msg>::focus_skip_traversal(&self.0)
            }
            fn focus_order(&self) -> Option<f32> {
                Widget::<Msg>::focus_order(&self.0)
            }
            fn focus_group(&self) -> bool {
                Widget::<Msg>::focus_group(&self.0)
            }
        }
    )*};
}

delegate!(
    Focus,
    ExcludeFocus,
    ExcludeFocusTraversal,
    FocusTraversalOrder,
    FocusTraversalGroup,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;
    use crate::ui::build_ui;
    use crate::{Container, TextField};
    use frus_core::Size;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Edited(String),
    }

    /// Three fields in a column, so there is a tree order to disturb.
    fn field(label: &str) -> TextField<Msg> {
        TextField::new(label.to_string())
            .label(label.to_string())
            .on_input(Msg::Edited)
    }

    fn order_of(root: &dyn Widget<Msg>) -> Vec<usize> {
        let ui = build_ui(
            root,
            Size::new(400.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let ids: Vec<_> = ui.focusable_ids().collect();
        ui.traversal_order()
            .iter()
            .map(|id| ids.iter().position(|x| x == id).expect("a known stop"))
            .collect()
    }

    /// Untouched, Tab follows the tree.
    #[test]
    fn tab_follows_the_tree_by_default() {
        let root = crate::column![field("a"), field("b"), field("c")];
        assert_eq!(order_of(&root), vec![0, 1, 2]);
    }

    /// An order moves a subtree without moving anything else.
    #[test]
    fn an_order_moves_only_what_it_names() {
        let root = crate::column![
            FocusTraversalOrder::new(2.0, field("a")),
            FocusTraversalOrder::new(1.0, field("b")),
            field("c"),
        ];
        // b then a, both ordered; c has no order, so it comes after them.
        assert_eq!(order_of(&root), vec![1, 0, 2]);
    }

    /// Skipped by Tab, and still a focus stop for a click.
    #[test]
    fn skipping_traversal_leaves_the_stop_where_it_was() {
        let root = crate::column![
            field("a"),
            ExcludeFocusTraversal::new(field("b")),
            field("c"),
        ];
        assert_eq!(order_of(&root), vec![0, 2], "Tab passes b by");

        let ui = build_ui(
            &root,
            Size::new(400.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(
            ui.focusable_ids().count(),
            3,
            "but b is still somewhere a click can land"
        );
    }

    /// Excluded outright: not a stop at all.
    #[test]
    fn an_excluded_subtree_registers_nothing() {
        let root = crate::column![field("a"), ExcludeFocus::new(field("b")), field("c")];
        let ui = build_ui(
            &root,
            Size::new(400.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(ui.focusable_ids().count(), 2);
        assert_eq!(order_of(&root), vec![0, 1]);
    }

    /// The flags are **subtree-scoped**: what follows an excluded subtree is not itself
    /// excluded. This is the bug the scope exists to prevent, and the one a flag set on
    /// the way in and never cleared would produce.
    #[test]
    fn the_flags_do_not_leak_to_what_comes_after() {
        let root = crate::column![
            ExcludeFocus::new(crate::column![field("a"), field("b")]),
            field("c"),
            field("d"),
        ];
        let ui = build_ui(
            &root,
            Size::new(400.0, 400.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert_eq!(ui.focusable_ids().count(), 2, "only c and d");
    }

    /// A group keeps an order to itself: the dialog reorders, the page behind it does not.
    #[test]
    fn a_group_resolves_its_order_among_its_own_members() {
        let root = crate::column![
            field("page-1"),
            FocusTraversalGroup::new(crate::column![
                FocusTraversalOrder::new(2.0, field("dialog-1")),
                FocusTraversalOrder::new(1.0, field("dialog-2")),
            ]),
            field("page-2"),
        ];
        // The page keeps tree order around a dialog that swapped its two fields.
        assert_eq!(order_of(&root), vec![0, 2, 1, 3]);
    }

    /// A `Focus` makes something focusable that had no opinion about it.
    #[test]
    fn it_makes_an_ordinary_box_a_focus_stop() {
        let plain = Container::<Msg>::new().width(40.0).height(40.0);
        assert!(!Widget::<Msg>::focusable(&plain));
        let wrapped = Focus::new(Container::<Msg>::new().width(40.0).height(40.0));
        assert!(Widget::<Msg>::focusable(&wrapped));
        assert!(!Widget::<Msg>::focusable(
            &Focus::new(Container::<Msg>::new()).can_request_focus(false)
        ));
    }
}
