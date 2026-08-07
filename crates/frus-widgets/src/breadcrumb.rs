//! [`Breadcrumb`]: a trail of clickable segments, the last of which is the current
//! page.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const SIZE: f32 = 15.0;

/// One segment: a clickable link, the current page, or a separator.
struct Crumb<Msg> {
    label: String,
    /// `Some` is a clickable link; `None` is the current page or a separator.
    message: Option<Msg>,
    separator: bool,
}

impl<Msg: Clone> Widget<Msg> for Crumb<Msg> {
    fn style(&self) -> Style {
        let measured = frus_text::measure(&self.label, SIZE);
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(measured.height.ceil()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let color = if self.separator {
            theme.muted
        } else if self.message.is_some() {
            // A link: discreet, lightening on hover.
            theme.muted.lerp(theme.on_surface, status.hover_progress)
        } else {
            // Page courante.
            theme.on_surface
        };
        scene.text(
            Point::new(bounds.x, bounds.y),
            self.label.clone(),
            SIZE,
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }
}

/// A controlled breadcrumb trail.
pub struct Breadcrumb<Msg> {
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Vec<String>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Breadcrumb<Msg> {
    /// Creates a breadcrumb; `on_select(i)` is emitted when the i-th segment is clicked.
    pub fn new(on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            on_select: Box::new(on_select),
            labels: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Adds a segment (at the end of the trail).
    pub fn crumb(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        let last = self.labels.len().saturating_sub(1);
        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        for (i, label) in self.labels.iter().enumerate() {
            if i > 0 {
                children.push(Box::new(Crumb {
                    label: "›".to_string(),
                    message: None,
                    separator: true,
                }));
            }
            children.push(Box::new(Crumb {
                label: label.clone(),
                message: if i == last {
                    None
                } else {
                    Some((self.on_select)(i))
                },
                separator: false,
            }));
        }
        self.children = children;
    }
}

impl<Msg: Clone> Widget<Msg> for Breadcrumb<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            gap: 6.0,
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

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    #[test]
    fn links_click_but_current_does_not() {
        let bc = Breadcrumb::new(Msg::Go)
            .crumb("Home")
            .crumb("Settings")
            .crumb("Advanced");
        let children = Widget::<Msg>::children(&bc);
        // 3 segments plus 2 separators makes 5 children.
        assert_eq!(children.len(), 5);
        // Enfant 0 = "Accueil" (lien) → Go(0).
        assert_eq!(children[0].on_click(), Some(Msg::Go(0)));
        // The last one, "Advanced", is current and so not clickable.
        assert_eq!(children[4].on_click(), None);
    }
}
