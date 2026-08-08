//! [`ToastHost`]: the **notification layer** — places and stacks [`crate::Toast`]s
//! in a corner of the screen, with an optional **entry transition**.
//!
//! Put it as the last layer of a [`crate::Stack`], above the interface. The widget fills
//! the available surface and aligns its toasts in the chosen corner ([`ToastPosition`]);
//! several toasts **stack** in a column. `fade_in` wraps each toast in an animated
//! opacity (the existing animation layer, [`crate::AnimatedOpacity`]) for a fade-in.
//!
//! The **content** (which toast(s) to show, their queue and auto-dismiss) stays driven
//! by the application through [`crate::SnackbarQueue`]: `ToastHost` only places them.

use frus_core::{Curve, Insets, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::animated::AnimatedOpacity;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Default margin between the toasts and the edges.
const HOST_PAD: f32 = 16.0;
/// Vertical gap between stacked toasts.
const STACK_GAP: f32 = 8.0;

/// The corner the notifications are anchored to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToastPosition {
    TopStart,
    TopCenter,
    TopEnd,
    BottomStart,
    BottomCenter,
    BottomEnd,
}

impl ToastPosition {
    /// Vertical alignment (the column's main axis): top vs bottom.
    fn justify(self) -> Justify {
        match self {
            ToastPosition::TopStart | ToastPosition::TopCenter | ToastPosition::TopEnd => {
                Justify::Start
            }
            _ => Justify::End,
        }
    }

    /// Horizontal alignment (the cross axis): left / centre / right.
    fn align(self) -> Align {
        match self {
            ToastPosition::TopStart | ToastPosition::BottomStart => Align::Start,
            ToastPosition::TopCenter | ToastPosition::BottomCenter => Align::Center,
            ToastPosition::TopEnd | ToastPosition::BottomEnd => Align::End,
        }
    }
}

/// A notification layer anchored in a corner.
pub struct ToastHost<Msg> {
    position: ToastPosition,
    padding: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> ToastHost<Msg> {
    /// An empty layer anchored at `position`.
    pub fn new(position: ToastPosition) -> Self {
        Self {
            position,
            padding: HOST_PAD,
            children: Vec::new(),
        }
    }

    /// Margin between the toasts and the edges (16 px by default).
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Adds a toast to the stack (call it several times to stack several).
    pub fn toast(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.children.push(Box::new(widget));
        self
    }

    /// Wraps **every** toast in an animated opacity (`duration` seconds): a fade-in
    /// built on the existing animation layer. Call this after the `toast`s.
    pub fn fade_in(self, duration: f32) -> Self {
        self.wrap_opacity(1.0, duration)
    }

    /// The mirror of [`fade_in`](Self::fade_in): animates the opacity toward **0** — the
    /// **exit** transition (the toast fades out before it is removed, Material style).
    /// The application plays it when the notification is leaving (see
    /// [`crate::SnackbarQueue::is_leaving`]).
    pub fn fade_out(self, duration: f32) -> Self {
        self.wrap_opacity(0.0, duration)
    }

    /// Wraps each toast in an opacity animated toward `target`.
    fn wrap_opacity(mut self, target: f32, duration: f32) -> Self {
        self.children = self
            .children
            .into_iter()
            .map(|child| {
                Box::new(AnimatedOpacity::new(
                    target,
                    duration,
                    Curve::ease_in_out(),
                    child,
                )) as Box<dyn Widget<Msg>>
            })
            .collect();
        self
    }
}

impl<Msg: Clone> Widget<Msg> for ToastHost<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            justify: self.position.justify(),
            align: self.position.align(),
            gap: STACK_GAP,
            padding: Insets::new(self.padding, self.padding, self.padding, self.padding),
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
    use crate::text::Text;

    #[test]
    fn empty_host_has_no_children() {
        let host = ToastHost::<()>::new(ToastPosition::TopEnd);
        assert!(Widget::<()>::children(&host).is_empty());
    }

    #[test]
    fn position_maps_to_justify_and_align() {
        let host = ToastHost::<()>::new(ToastPosition::BottomEnd).toast(Text::new("x"));
        let style = Widget::<()>::style(&host);
        assert!(
            matches!(style.justify, Justify::End),
            "bottom → justify End"
        );
        assert!(matches!(style.align, Align::End), "right → align End");
        assert_eq!(Widget::<()>::children(&host).len(), 1);

        let top_center = Widget::<()>::style(&ToastHost::<()>::new(ToastPosition::TopCenter));
        assert!(matches!(top_center.justify, Justify::Start));
        assert!(matches!(top_center.align, Align::Center));
    }

    #[test]
    fn stacks_multiple_and_fade_in_preserves_count() {
        let host = ToastHost::<()>::new(ToastPosition::BottomCenter)
            .toast(Text::new("a"))
            .toast(Text::new("b"))
            .fade_in(0.2);
        assert_eq!(
            Widget::<()>::children(&host).len(),
            2,
            "two toasts, wrapped in a fade"
        );
    }

    #[test]
    fn fade_out_wraps_children() {
        let host = ToastHost::<()>::new(ToastPosition::BottomCenter)
            .toast(Text::new("bye"))
            .fade_out(0.3);
        assert_eq!(
            Widget::<()>::children(&host).len(),
            1,
            "toast wrapped in a fade-out"
        );
    }
}
