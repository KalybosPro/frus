//! [`Keyed`]: a **transparent** wrapper that gives a widget a **stable identity**,
//! through a key, independent of its position among its siblings.
//!
//! Without a key, identity is positional: removing an item from the middle of a list
//! shifts the identity of everything after it, and their retained state — hover, focus,
//! caret, animations, the leaving fade — jumps. Wrapping each item in a `Keyed`, keyed
//! on its stable domain id, fixes that.

use std::hash::{Hash, Hasher};

use crate::widget::Widget;

/// Wraps a widget in a stable identity key, delegating everything else.
pub struct Keyed<Msg> {
    key: u64,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> Keyed<Msg> {
    /// Wraps `inner` with the key `key`, which may be any hashable type.
    pub fn new(key: impl Hash, inner: impl Widget<Msg> + 'static) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        Self {
            key: hasher.finish(),
            inner: Box::new(inner),
        }
    }

    /// A key is an identity, not a box: the child's own, unchanged.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(Keyed {
    /// The one thing a `Keyed` does not delegate: the identity it exists to give.
    fn key(&self) -> Option<u64> {
        Some(self.key)
    }

    /// Forwarded: a key is not a place — `Keyed(Positioned(…))` must keep its pins.
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }

    /// And the one it does — a key says nothing about a theme.
    fn theme_override(
        &self,
        inherited: &crate::theme::Theme,
    ) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::ReorderAxis;
    use crate::{Container, Text};

    /// A reorderable that is **not** the default in either respect: dragged vertically,
    /// and a drop target only.
    struct Card;

    impl Widget<()> for Card {
        fn style(&self) -> frus_layout::Style {
            frus_layout::Style::default()
        }
        fn children(&self) -> &[Box<dyn Widget<()>>] {
            &[]
        }
        fn paint(
            &self,
            _bounds: frus_core::Rect,
            _status: crate::interaction::Status,
            _theme: &crate::theme::Theme,
            _scene: &mut frus_core::Scene,
        ) {
        }
        fn on_click(&self) -> Option<()> {
            None
        }
        fn reorder_axis(&self) -> ReorderAxis {
            ReorderAxis::Vertical
        }
        fn reorder_draggable(&self) -> bool {
            false
        }
    }

    /// Found by the forwarding test in [`crate::transparent`], not by a device: the
    /// hand-written version of this wrapper forwarded `reorder_index` and `on_reorder`
    /// and stopped there, so a keyed card in a board dragged along the wrong axis and a
    /// drop-only slot could be lifted.
    #[test]
    fn a_keyed_reorderable_keeps_its_axis_and_its_refusal_to_be_lifted() {
        let keyed = Keyed::new(1u64, Card);
        assert_eq!(Widget::<()>::reorder_axis(&keyed), ReorderAxis::Vertical);
        assert!(!Widget::<()>::reorder_draggable(&keyed));
    }

    /// The structural questions decide **how the content is laid out**, so a
    /// transparent wrapper must pass them through. Answering them for itself made a
    /// keyed stack lay its layers out in flow instead of on top of one another — found
    /// on a device, wrapping a swipeable row in `keyed(...)`.
    #[test]
    fn structural_questions_pass_through() {
        let stack = crate::Stack::<()>::new()
            .width(100.0)
            .height(50.0)
            .layer(Container::new())
            .layer(Container::new());
        assert!(Widget::<()>::stack(&stack));
        assert!(
            Widget::<()>::stack(&Keyed::new(1u64, stack)),
            "a keyed stack is still a stack"
        );

        let spinner = crate::Spinner::new();
        assert!(Widget::<()>::continuous(&spinner));
        assert!(Widget::<()>::continuous(&Keyed::new(2u64, spinner)));
    }

    #[test]
    fn reports_key_and_delegates() {
        let inner = Container::<()>::new().width(40.0).child(Text::new("x"));
        let keyed = Keyed::new(99u64, inner);
        // It returns a key.
        assert!(Widget::<()>::key(&keyed).is_some());
        // It delegates the children; the Container has one.
        assert_eq!(Widget::<()>::children(&keyed).len(), 1);
    }

    #[test]
    fn same_key_same_hash() {
        let a: Keyed<()> = Keyed::new("todo-3", Container::new());
        let b: Keyed<()> = Keyed::new("todo-3", Container::new());
        assert_eq!(Widget::<()>::key(&a), Widget::<()>::key(&b));
    }
}
