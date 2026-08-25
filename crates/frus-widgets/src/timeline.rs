//! [`Timeline`]: a vertical chronology — events joined by a line, each marked with a
//! dot.

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 56.0;
/// The entry's title, and the line under it.
const TITLE_SIZE: f32 = 16.0;
const DETAIL_SIZE: f32 = 13.0;

/// The title's style, **resolved once** so that the number the row is measured with is the
/// number the glyphs are drawn at.
fn title_style() -> ResolvedTextStyle {
    TextStyle::new(TITLE_SIZE).resolved()
}

/// The detail line's style. See [`title_style`].
fn detail_style() -> ResolvedTextStyle {
    TextStyle::new(DETAIL_SIZE).resolved()
}
const LINE_X: f32 = 8.0;
const DOT: f32 = 10.0;
const TEXT_X: f32 = 28.0;

/// One event on the timeline: a dot, a line and the texts.
struct Event {
    title: String,
    detail: String,
}

impl<Msg> Widget<Msg> for Event {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let cx = bounds.x + LINE_X;
        // A continuous line, crossing the whole row so that the rows join up.
        scene.fill_rect(
            Rect::new(cx - 1.0, bounds.y, 2.0, bounds.height),
            theme.border.fade(o),
        );
        // Point.
        let cy = bounds.y + ROW_H * 0.5;
        scene.draw_rect(
            Rect::new(cx - DOT * 0.5, cy - DOT * 0.5, DOT, DOT),
            theme.primary.fade(o),
            DOT * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // Title and detail.
        scene.text(
            Point::new(bounds.x + TEXT_X, bounds.y + 8.0),
            self.title.clone(),
            &title_style(),
            theme.on_surface.fade(o),
        );
        scene.text(
            Point::new(bounds.x + TEXT_X, bounds.y + 30.0),
            self.detail.clone(),
            &detail_style(),
            theme.muted.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A vertical chronology.
pub struct Timeline<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Timeline<Msg> {
    /// Creates an empty timeline.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Adds an event, title and detail, oldest to most recent.
    pub fn event(mut self, title: impl Into<String>, detail: impl Into<String>) -> Self {
        self.children.push(Box::new(Event {
            title: title.into(),
            detail: detail.into(),
        }));
        self
    }
}

impl<Msg> Default for Timeline<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Timeline<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
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
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[test]
    fn events_paint_dots_and_text() {
        let timeline = Timeline::<()>::new()
            .event("Milestone 1", "window")
            .event("Milestone 2", "layout");
        assert_eq!(Widget::<()>::children(&timeline).len(), 2);

        let ui = build_ui(
            &timeline,
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Milestone 1") && has("Milestone 2"));
    }
}
