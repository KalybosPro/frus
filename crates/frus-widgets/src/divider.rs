//! [`Divider`]: a thin horizontal separator, in the theme's colours.

use frus_core::{Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The extent a divider takes **across** its axis when the caller has not said
/// otherwise — the line plus the air around it, which is what keeps a separator from
/// touching the rows it separates. The reference's figure.
pub const DIVIDER_SPACE: f32 = 16.0;
/// The line's own thickness, in logical pixels, by default.
pub const DIVIDER_THICKNESS: f32 = 1.0;

/// A separator: a thin line, centred in a taller box that gives it room to breathe.
///
/// The box and the line are **two different measurements**, and conflating them is the
/// usual mistake: [`Divider::height`] is how much room the separator takes in the
/// layout, [`Divider::thickness`] is how thick the line drawn inside it is. A divider
/// with no space around it reads as a border on the row above, not as a separator.
///
/// ```ignore
/// Divider::new()                                   // 16 px of room, a 1 px line
/// Divider::new().height(1.0)                       // no air: a hairline, flush
/// Divider::new().indent(16.0)                      // inset, the way a list insets one
/// Divider::new().thickness(2.0).color(theme.primary)
/// ```
pub struct Divider {
    /// `None` = [`DIVIDER_SPACE`].
    height: Option<f32>,
    /// `None` = [`DIVIDER_THICKNESS`].
    thickness: Option<f32>,
    indent: f32,
    end_indent: f32,
    /// `None` = the theme's discreet outline.
    color: Option<Color>,
}

impl Divider {
    /// Creates a separator with the theme's colour and the default spacing.
    pub fn new() -> Self {
        Self {
            height: None,
            thickness: None,
            indent: 0.0,
            end_indent: 0.0,
            color: None,
        }
    }

    /// The room the separator takes in the layout, line and air together. Defaults to
    /// [`DIVIDER_SPACE`].
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The thickness of the line itself. Defaults to [`DIVIDER_THICKNESS`].
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Insets the line from the **leading** edge, leaving the box where it is — how a
    /// list separates rows without cutting through their leading icons.
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = indent;
        self
    }

    /// Insets the line from the **trailing** edge.
    pub fn end_indent(mut self, end_indent: f32) -> Self {
        self.end_indent = end_indent;
        self
    }

    /// Overrides the line's colour. Defaults to the theme's discreet outline.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for Divider {
    fn style(&self) -> Style {
        Style {
            // Automatic width: the parent stretches it (align: stretch).
            width: Dimension::Auto,
            height: Dimension::Length(self.height.unwrap_or(DIVIDER_SPACE)),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self.color.unwrap_or(theme.scheme.outline_variant);
        // The line is centred in the box: the space above and below it is the point of
        // the box being taller than the line. It never draws thicker than its box.
        let thickness = self
            .thickness
            .unwrap_or(DIVIDER_THICKNESS)
            .min(bounds.height);
        let width = (bounds.width - self.indent - self.end_indent).max(0.0);
        if width <= 0.0 || thickness <= 0.0 {
            return;
        }
        let line = Rect::new(
            bounds.x + self.indent,
            bounds.y + (bounds.height - thickness) / 2.0,
            width,
            thickness,
        );
        scene.fill_rect(line, color.fade(status.opacity));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn painted(divider: &Divider, bounds: Rect) -> Option<Rect> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            divider,
            bounds,
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
    }

    #[test]
    fn paints_a_border_line() {
        let divider = Divider::new();
        let line = painted(&divider, Rect::new(0.0, 0.0, 120.0, 1.0)).expect("a line");
        assert_eq!(line.width, 120.0);
    }

    #[test]
    fn the_box_and_the_line_are_two_different_measurements() {
        // The reference's separator takes 16 px of the layout and draws 1 px of line in
        // the middle of it. Drawing the whole box, which is what this used to do, gives
        // a 16 px bar.
        let divider = Divider::new();
        let style = Widget::<()>::style(&divider);
        assert_eq!(style.height, Dimension::Length(DIVIDER_SPACE));

        let line = painted(&divider, Rect::new(0.0, 100.0, 200.0, DIVIDER_SPACE)).expect("a line");
        assert_eq!(line.height, DIVIDER_THICKNESS, "one pixel of line");
        assert_eq!(
            line.y,
            100.0 + (DIVIDER_SPACE - DIVIDER_THICKNESS) / 2.0,
            "centred in its box, with air on both sides"
        );
    }

    #[test]
    fn a_flush_hairline_is_one_call_away() {
        let divider = Divider::new().height(1.0);
        assert_eq!(Widget::<()>::style(&divider).height, Dimension::Length(1.0));
        let line = painted(&divider, Rect::new(0.0, 0.0, 200.0, 1.0)).expect("a line");
        assert_eq!((line.y, line.height), (0.0, 1.0), "it fills its own box");
    }

    #[test]
    fn the_indents_inset_the_line_and_not_the_box() {
        let divider = Divider::new().indent(16.0).end_indent(8.0);
        let line = painted(&divider, Rect::new(10.0, 0.0, 200.0, DIVIDER_SPACE)).expect("a line");
        assert_eq!(line.x, 26.0, "inset from the leading edge");
        assert_eq!(line.width, 200.0 - 16.0 - 8.0);
    }

    #[test]
    fn a_line_thicker_than_its_box_is_clamped_rather_than_overflowing() {
        let divider = Divider::new().height(2.0).thickness(10.0);
        let line = painted(&divider, Rect::new(0.0, 0.0, 100.0, 2.0)).expect("a line");
        assert_eq!(line.height, 2.0);
    }

    #[test]
    fn indents_wider_than_the_box_draw_nothing() {
        let divider = Divider::new().indent(120.0).end_indent(120.0);
        assert!(painted(&divider, Rect::new(0.0, 0.0, 100.0, DIVIDER_SPACE)).is_none());
    }

    #[test]
    fn the_colour_is_the_callers_when_they_give_one() {
        let mine = Color::rgb8(255, 0, 0);
        let divider = Divider::new().color(mine);
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &divider,
            Rect::new(0.0, 0.0, 100.0, DIVIDER_SPACE),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let color = scene.primitives().iter().find_map(|p| match p {
            Primitive::Rect { color, .. } => Some(*color),
            _ => None,
        });
        assert_eq!(color, Some(mine));
    }
}
