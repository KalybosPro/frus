//! [`Timeline`]: a vertical chronology — events joined by a line, each marked with a
//! dot.

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The floor of a row. A row grows past it when its two lines ask for more.
const ROW_H: f32 = 56.0;
const LINE_X: f32 = 8.0;
const DOT: f32 = 10.0;
const TEXT_X: f32 = 28.0;
/// The room above the title and below the detail.
const TEXT_PAD_Y: f32 = 8.0;
/// The room between the two lines.
const TEXT_GAP: f32 = 3.0;

/// The title's style: what the caller said, else what the theme says, else the
/// reference's — an entry reads as a list tile, whose title is `bodyLarge`.
///
/// **Resolved once**, so that the number the row is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
fn title_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.timeline.title_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_large)
        .resolved()
}

/// The detail line's style — a list tile's subtitle is `bodyMedium`. See [`title_style`].
fn detail_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.timeline.detail_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_medium)
        .resolved()
}

/// One event on the timeline: a dot, a line and the texts.
struct Event {
    title: String,
    detail: String,
    title_text_style: Option<TextStyle>,
    detail_text_style: Option<TextStyle>,
}

impl Event {
    /// The two styles this row is measured and drawn with.
    fn styles(&self, theme: Option<&Theme>) -> (ResolvedTextStyle, ResolvedTextStyle) {
        (
            title_style(self.title_text_style, theme),
            detail_style(self.detail_text_style, theme),
        )
    }

    fn sizing(&self, theme: Option<&Theme>) -> Style {
        // The floor, or what the two lines actually need. A row fixed at `ROW_H` puts the
        // detail of the next entry over the title of this one the moment a reader turns
        // the type up, which is the whole reason a height is asked for rather than stated.
        let (title, detail) = self.styles(theme);
        let needed = TEXT_PAD_Y * 2.0 + title.line_height() + TEXT_GAP + detail.line_height();
        Style {
            height: Dimension::Length(ROW_H.max(needed.ceil())),
            ..Default::default()
        }
    }
}

impl<Msg> Widget<Msg> for Event {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
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
        let cy = bounds.y + bounds.height * 0.5;
        scene.draw_rect(
            Rect::new(cx - DOT * 0.5, cy - DOT * 0.5, DOT, DOT),
            theme.primary.fade(o),
            DOT * 0.5,
            0.0,
            Color::TRANSPARENT,
        );
        // Title and detail. The second follows the first's **own** line box: a fixed
        // offset would be right for one type size and wrong for every other.
        let (title, detail) = self.styles(Some(theme));
        let ty = bounds.y + TEXT_PAD_Y;
        scene.text(
            Point::new(bounds.x + TEXT_X, ty),
            self.title.clone(),
            &title,
            theme.on_surface.fade(o),
        );
        scene.text(
            Point::new(bounds.x + TEXT_X, ty + title.line_height() + TEXT_GAP),
            self.detail.clone(),
            &detail,
            theme.muted.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A vertical chronology.
pub struct Timeline<Msg> {
    events: Vec<(String, String)>,
    title_text_style: Option<TextStyle>,
    detail_text_style: Option<TextStyle>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Timeline<Msg> {
    /// Creates an empty timeline.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            title_text_style: None,
            detail_text_style: None,
            children: Vec::new(),
        }
    }

    /// Adds an event, title and detail, oldest to most recent.
    pub fn event(mut self, title: impl Into<String>, detail: impl Into<String>) -> Self {
        self.events.push((title.into(), detail.into()));
        self.rebuild();
        self
    }

    /// The entries' headings, over the theme's and the reference's.
    #[must_use]
    pub fn title_text_style(mut self, style: TextStyle) -> Self {
        self.title_text_style = Some(style);
        self.rebuild();
        self
    }

    /// The line under each heading, over the theme's and the reference's.
    #[must_use]
    pub fn detail_text_style(mut self, style: TextStyle) -> Self {
        self.detail_text_style = Some(style);
        self.rebuild();
        self
    }

    /// Carries the current styles into every row, so that the builders are
    /// order-independent.
    fn rebuild(&mut self) {
        self.children = self
            .events
            .iter()
            .map(|(title, detail)| {
                Box::new(Event {
                    title: title.clone(),
                    detail: detail.clone(),
                    title_text_style: self.title_text_style,
                    detail_text_style: self.detail_text_style,
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
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
