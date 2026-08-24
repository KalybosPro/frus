//! [`Semantics`]: states what a widget **is**, from outside the widget.
//!
//! Every other widget in this crate answers [`Widget::semantics`] for *itself*. That
//! covers the ordinary case and misses the one this exists for: a caller is handed a
//! built widget and knows something about it that the widget does not know about itself.
//!
//! Milestone 397 walked into it. An [`AppBar`](crate::AppBar) marks its **text** title as
//! a heading — a landmark a screen reader's user jumps between — and could do nothing of
//! the sort for a *widget* title, because by then the title is a `Box<dyn Widget>` and the
//! bar has no way in. The accessibility of a bar therefore depended on whether the caller
//! had passed a string or a widget, which is not a distinction anybody using assistive
//! technology should be able to feel.

use frus_core::{Rect, Role, Scene, SemanticsProperties};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// What a [`Semantics`] wrapper asks the walk to do with its subtree's annotations.
///
/// Returned by [`Widget::describes`] and read in one place, the way a
/// [`ModalBarrier`](crate::barrier::ModalBarrier) is: the subtree is walked exactly as
/// usual and what it produced is reconciled afterwards. Doing it after rather than instead
/// is what makes it exact — a widget deep inside annotates itself without knowing anything
/// is speaking for it.
#[derive(Clone, Debug, PartialEq)]
pub struct Description {
    /// What the wrapper says about its child.
    pub props: SemanticsProperties,
    /// Replace the subtree's own annotations with this one instead of adding to them.
    pub merging: bool,
}

/// States a **role, a name or a state** for a child that cannot state it for itself — the
/// reference's `Semantics` widget.
///
/// ```ignore
/// // A widget title announced as a landmark rather than as one more run of prose.
/// Semantics::heading(my_title_widget)
///
/// // A row of two controls that reads as one thing.
/// Semantics::merge(Flex::row().child(Checkbox::new(on)).child(Text::new("Notifications")))
/// ```
///
/// # It adds a node; merging replaces them
///
/// By default this **adds** an annotation and leaves the child's alone: safe, and what a
/// caller naming an otherwise anonymous box wants. [`Semantics::merging`] is the other
/// answer — the subtree's annotations are dropped and this one speaks for all of them,
/// carrying over what they said through [`SemanticsProperties::over`].
///
/// The default is the additive one **because the destructive one cannot be undone by the
/// caller**. Wrapping a whole screen to name it, under a merging default, would collapse
/// every control on it into a single node; the reverse mistake leaves one node too many,
/// which is noise rather than loss.
///
/// The reference splits the same two behaviours across `Semantics(excludeSemantics:)` and
/// a separate `MergeSemantics`; [`Semantics::merge`] is that second widget, spelled as the
/// constructor it is.
///
/// # What the flat tree costs
///
/// This framework's accessibility tree is **flat**: every annotated widget is a child of
/// the window. So merging really does discard, where the reference merges a subtree into
/// its container node and keeps the shape. Two labels are joined into one string, one line
/// each, which is what the reference does with a merged subtree too — but a nested
/// structure a reader could descend into is not something ours can express yet.
pub struct Semantics<Msg> {
    props: SemanticsProperties,
    merging: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Semantics<Msg> {
    /// Annotates `child` with `props`, adding a node and leaving the child's own alone.
    pub fn new(props: SemanticsProperties, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            props,
            merging: false,
            children: vec![Box::new(child)],
        }
    }

    /// Announces `child` as a **heading**: a landmark assistive technology can jump
    /// between, rather than one more run of prose.
    ///
    /// It merges, and that is the point of the shorthand. A heading that also announced
    /// the text inside it as a separate label would be read twice — once as a landmark and
    /// once as prose — so the wrapper takes the child's words and speaks for it.
    pub fn heading(child: impl Widget<Msg> + 'static) -> Self {
        Self::new(SemanticsProperties::new(Role::Heading), child).merging(true)
    }

    /// Makes `child`'s whole subtree read as **one thing** — the reference's
    /// `MergeSemantics`, for a row of controls that is really a single control.
    ///
    /// It states nothing of its own: everything announced comes from the subtree, joined.
    pub fn merge(child: impl Widget<Msg> + 'static) -> Self {
        Self::new(SemanticsProperties::default(), child).merging(true)
    }

    /// Whether this speaks **for** the subtree (dropping its annotations and carrying over
    /// what they said) or merely **beside** it. Off by default; see the type's docs for
    /// why that is the safe default rather than the faithful one.
    pub fn merging(mut self, merging: bool) -> Self {
        self.merging = merging;
        self
    }

    /// Sets the accessible name — what a screen reader reads out.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self
    }

    /// Sets the textual value: a field's contents, a slider's position.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = Some(value.into());
        self
    }

    /// Sets the announced role.
    pub fn role(mut self, role: Role) -> Self {
        self.props.role = role;
        self
    }

    /// Marks it checked or unchecked.
    pub fn toggled(mut self, on: bool) -> Self {
        self.props = self.props.toggled(on);
        self
    }

    /// Announces it as actionable — the reader offers "activate".
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.props.clickable = clickable;
        self
    }

    /// Announces it as disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Semantics<Msg> {
    /// A **transparent** box: the child's own style decides everything, and this one is
    /// the pass-through a single-child wrapper has to be to change nothing that is drawn.
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    /// It states what its child **is**, never what it does: an annotation that swallowed a
    /// click would put a hit target over the widget it was describing.
    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn describes(&self) -> Option<Description> {
        Some(Description {
            props: self.props.clone(),
            merging: self.merging,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Flex, Runtime, Text};
    use frus_core::Size as CoreSize;

    /// Every annotation the built tree carries, in walk order.
    fn described(root: &dyn Widget<()>) -> Vec<(Role, Option<String>)> {
        let ui = build_ui(
            root,
            CoreSize::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        ui.semantics()
            .iter()
            .map(|(_, _, s)| (s.role, s.label.clone()))
            .collect()
    }

    /// **A caller can mark an assembled widget as a heading**, which is the gap milestone
    /// 397 left: an app bar could do it for a text title and not for a widget one.
    ///
    /// And it keeps the words. A heading with no label is a landmark that announces
    /// nothing on arrival — worse than the unmarked text it replaced, not better — so the
    /// wrapper states the role and takes the label from what it wrapped.
    #[test]
    fn a_heading_takes_the_role_it_states_and_the_words_it_wrapped() {
        let root: Box<dyn Widget<()>> = Box::new(Semantics::heading(Text::new("Settings")));
        assert_eq!(
            described(root.as_ref()),
            vec![(Role::Heading, Some("Settings".to_string()))],
            "one node: a heading that says the words"
        );
    }

    /// Left alone it **adds**, and the child keeps its own voice.
    ///
    /// This is the half that makes the default safe. A caller naming a box gets a name and
    /// still has everything inside it; a merging default would have silently collapsed a
    /// whole screen into one node the first time somebody named one.
    #[test]
    fn without_merging_the_child_still_speaks() {
        let root: Box<dyn Widget<()>> = Box::new(
            Semantics::new(SemanticsProperties::default(), Text::new("Settings"))
                .label("A section"),
        );
        let found = described(root.as_ref());
        assert_eq!(found.len(), 2, "two nodes, not one: {found:?}");
        assert!(found.contains(&(Role::Label, Some("Settings".to_string()))));
        assert!(found.iter().any(|(_, l)| l.as_deref() == Some("A section")));
    }

    /// **Two labels are joined, not chosen between.** A row of a control and its caption
    /// reads as one thing with both; picking one would drop the other with nothing to say
    /// which was lost.
    #[test]
    fn merging_joins_the_labels_it_swallows() {
        let root: Box<dyn Widget<()>> = Box::new(Semantics::merge(
            Flex::row()
                .child(Text::new("Notifications"))
                .child(Text::new("On")),
        ));
        let found = described(root.as_ref());
        assert_eq!(found.len(), 1, "one node: {found:?}");
        assert_eq!(found[0].1.as_deref(), Some("Notifications\nOn"));
    }

    /// An annotation that says nothing and swallows nothing puts **no node** in the tree.
    ///
    /// The same rule every other widget is held to — `is_meaningful` — applied to the one
    /// widget whose whole purpose is to add a node. A wrapper that announced an empty
    /// container would be one more thing for a reader to step through on the way to the
    /// content.
    #[test]
    fn an_empty_annotation_adds_nothing() {
        let root: Box<dyn Widget<()>> = Box::new(Semantics::merge(crate::Container::new()));
        assert!(described(root.as_ref()).is_empty());
    }
}
