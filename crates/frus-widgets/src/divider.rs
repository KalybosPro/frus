//! [`Divider`] and [`VerticalDivider`]: a thin separator in the theme's colours, across
//! a column or down a row.

use frus_core::{BorderRadius, Color, Rect, Scene};
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
    /// `None` = the theme's, then none.
    indent: Option<f32>,
    /// `None` = the theme's, then none.
    end_indent: Option<f32>,
    /// `None` = the theme's discreet outline.
    color: Option<Color>,
    /// `None` = square ends, as a hairline wants.
    radius: Option<BorderRadius>,
}

impl Divider {
    /// Creates a separator with the theme's colour and the default spacing.
    pub fn new() -> Self {
        Self {
            height: None,
            thickness: None,
            indent: None,
            end_indent: None,
            color: None,
            radius: None,
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
        self.indent = Some(indent);
        self
    }

    /// Insets the line from the **trailing** edge.
    pub fn end_indent(mut self, end_indent: f32) -> Self {
        self.end_indent = Some(end_indent);
        self
    }

    /// Overrides the line's colour. Defaults to the theme's discreet outline.
    /// **The rule's own corner radius** — the reference's `Divider.radius`
    /// (`divider.dart:68`). A thick rule reads as a bar, and a bar with square ends is the
    /// only thing in an interface that still has them.
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

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

impl Divider {
    /// The room it takes, resolved: what the caller said, then the theme, then ours.
    fn space(&self, theme: Option<&Theme>) -> f32 {
        self.height
            .or_else(|| theme.and_then(|t| t.widgets.divider.height))
            .unwrap_or(DIVIDER_SPACE)
    }
}

impl<Msg> Widget<Msg> for Divider {
    fn style(&self) -> Style {
        Style {
            // Automatic width: the parent stretches it (align: stretch).
            width: Dimension::Auto,
            height: Dimension::Length(self.space(None)),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            height: Dimension::Length(self.space(Some(theme))),
            ..Widget::<Msg>::style(self)
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self
            .color
            .or(theme.widgets.divider.color)
            .unwrap_or(theme.scheme.outline_variant);
        // The line is centred in the box: the space above and below it is the point of
        // the box being taller than the line. It never draws thicker than its box.
        let thickness = self
            .thickness
            .or(theme.widgets.divider.thickness)
            .unwrap_or(DIVIDER_THICKNESS)
            .min(bounds.height);
        let indent = self.indent.or(theme.widgets.divider.indent).unwrap_or(0.0);
        let end_indent = self
            .end_indent
            .or(theme.widgets.divider.end_indent)
            .unwrap_or(0.0);
        let width = (bounds.width - indent - end_indent).max(0.0);
        if width <= 0.0 || thickness <= 0.0 {
            return;
        }
        let line = Rect::new(
            bounds.x + indent,
            bounds.y + (bounds.height - thickness) / 2.0,
            width,
            thickness,
        );
        match self.radius {
            Some(radius) => scene.draw_rect(
                line,
                color.fade(status.opacity),
                radius,
                0.0,
                Color::TRANSPARENT,
            ),
            None => scene.fill_rect(line, color.fade(status.opacity)),
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A **vertical** separator: a thin line, centred in a wider box that gives it room.
///
/// The horizontal one above and this one are two widgets in the reference too
/// (`divider.dart:244`), reading the same theme, because the field that means *the room
/// the separator takes* is called `height` on one and `width` on the other and there is no
/// honest way to give a single widget one name for it. Everything else is the same
/// question turned ninety degrees: [`thickness`] is the line, [`indent`] is measured from
/// the **top** here rather than from the leading edge, and [`end_indent`] from the bottom.
///
/// ```ignore
/// Flex::row()
///     .child(left_pane)
///     .child(VerticalDivider::new())     // 16 px of room, a 1 px line
///     .child(right_pane)
/// ```
///
/// **It takes its row's full height, whatever the row said about alignment.** A row that
/// centres its children would otherwise give this one the height of its content, and its
/// content is nothing — so a rule in a centred row would be a rule nobody can see, which
/// is the first thing anybody hits. It asks for the cross axis on its own behalf
/// (`align_self`), the way the reference's `Center` inside a `SizedBox` fills whatever
/// height it is offered.
///
/// It still needs a height to fill: a row is as tall as its tallest child, so a rule
/// between two things that size themselves is as tall as they are, and one standing alone
/// in a row of nothing else is nothing at all.
///
/// [`thickness`]: VerticalDivider::thickness
/// [`indent`]: VerticalDivider::indent
/// [`end_indent`]: VerticalDivider::end_indent
pub struct VerticalDivider {
    /// `None` = [`DIVIDER_SPACE`].
    width: Option<f32>,
    /// `None` = [`DIVIDER_THICKNESS`].
    thickness: Option<f32>,
    /// From the **top**. `None` = the theme's, then none.
    indent: Option<f32>,
    /// From the **bottom**. `None` = the theme's, then none.
    end_indent: Option<f32>,
    /// `None` = the theme's discreet outline.
    color: Option<Color>,
    /// `None` = square ends.
    radius: Option<BorderRadius>,
}

impl VerticalDivider {
    /// Creates a vertical separator with the theme's colour and the default spacing.
    pub fn new() -> Self {
        Self {
            width: None,
            thickness: None,
            indent: None,
            end_indent: None,
            color: None,
            radius: None,
        }
    }

    /// The room the separator takes **across** the row, line and air together. Defaults to
    /// [`DIVIDER_SPACE`] — the same figure a horizontal one takes, and the same theme
    /// field, which is why one number covers both.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The thickness of the line itself. Defaults to [`DIVIDER_THICKNESS`].
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Insets the line from the **top**, leaving the box where it is.
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = Some(indent);
        self
    }

    /// Insets the line from the **bottom**.
    pub fn end_indent(mut self, end_indent: f32) -> Self {
        self.end_indent = Some(end_indent);
        self
    }

    /// Overrides the line's colour. Defaults to the theme's discreet outline.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The rule's own corner radius (`divider.dart:68`).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// The room it takes, resolved: what the caller said, then the theme, then ours.
    ///
    /// **The theme field is the horizontal one's**, deliberately. The reference keeps a
    /// single `DividerThemeData.space` for both orientations, so an application that wants
    /// its rules tighter says it once.
    fn space(&self, theme: Option<&Theme>) -> f32 {
        self.width
            .or_else(|| theme.and_then(|t| t.widgets.divider.height))
            .unwrap_or(DIVIDER_SPACE)
    }
}

impl Default for VerticalDivider {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> Widget<Msg> for VerticalDivider {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.space(None)),
            // Automatic height, **stretched on its own say-so**: see the type's note.
            height: Dimension::Auto,
            align_self: Some(frus_layout::Align::Stretch),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: Dimension::Length(self.space(Some(theme))),
            ..Widget::<Msg>::style(self)
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let color = self
            .color
            .or(theme.widgets.divider.color)
            .unwrap_or(theme.scheme.outline_variant);
        let thickness = self
            .thickness
            .or(theme.widgets.divider.thickness)
            .unwrap_or(DIVIDER_THICKNESS)
            .min(bounds.width);
        let indent = self.indent.or(theme.widgets.divider.indent).unwrap_or(0.0);
        let end_indent = self
            .end_indent
            .or(theme.widgets.divider.end_indent)
            .unwrap_or(0.0);
        let height = (bounds.height - indent - end_indent).max(0.0);
        if height <= 0.0 || thickness <= 0.0 {
            return;
        }
        let line = Rect::new(
            bounds.x + (bounds.width - thickness) / 2.0,
            bounds.y + indent,
            thickness,
            height,
        );
        match self.radius {
            Some(radius) => scene.draw_rect(
                line,
                color.fade(status.opacity),
                radius,
                0.0,
                Color::TRANSPARENT,
            ),
            None => scene.fill_rect(line, color.fade(status.opacity)),
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    /// **The two rules are the same widget turned ninety degrees**, and the box is where
    /// that shows: one declares a height and lets the column stretch its width, the other
    /// declares a width and lets the row stretch its height. Get it backwards and a
    /// vertical rule is an invisible 16-pixel-tall band across the row.
    #[test]
    fn a_vertical_rule_declares_its_width_and_takes_the_rows_height() {
        let across = Widget::<()>::style(&Divider::new());
        assert_eq!(across.height, Dimension::Length(DIVIDER_SPACE));
        assert_eq!(across.width, Dimension::Auto);

        let down = Widget::<()>::style(&VerticalDivider::new());
        assert_eq!(down.width, Dimension::Length(DIVIDER_SPACE));
        assert_eq!(down.height, Dimension::Auto);
    }

    /// **A rule in a centred row is still a full-height rule.** It asks for the cross axis
    /// itself, because the alternative is a widget whose first use is invisible: a row that
    /// centres its children hands this one the height of its content, and its content is
    /// nothing.
    #[test]
    fn a_vertical_rule_fills_its_row_whatever_the_row_said() {
        assert_eq!(
            Widget::<()>::style(&VerticalDivider::new()).align_self,
            Some(frus_layout::Align::Stretch)
        );
        assert_eq!(
            Widget::<()>::style(&Divider::new()).align_self,
            None,
            "the horizontal one has no such problem: a column stretches it by default and              a column that does not is a column with a reason"
        );
    }

    /// The line is centred **across** its box, and its two indents are measured from the
    /// top and the bottom rather than from the two ends of a row.
    #[test]
    fn a_vertical_rule_is_centred_across_its_box_and_indented_down_it() {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &VerticalDivider::new()
                .thickness(2.0)
                .indent(10.0)
                .end_indent(6.0),
            Rect::new(0.0, 0.0, DIVIDER_SPACE, 100.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let line = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("it draws one");
        assert_eq!(line.width, 2.0, "the thickness is across the row");
        assert_eq!(line.x, (DIVIDER_SPACE - 2.0) / 2.0, "centred in its box");
        assert_eq!(line.y, 10.0, "the indent is from the top");
        assert_eq!(
            line.height,
            100.0 - 10.0 - 6.0,
            "and the end from the bottom"
        );
    }

    /// **One theme field for both orientations**, as the reference has it: `space` there,
    /// [`DividerTheme::height`](crate::widgettheme::DividerTheme::height) here. An
    /// application that wants its rules tighter says it once and both obey — which is the
    /// only reason the field is not called `width` on one of them.
    #[test]
    fn both_rules_read_the_same_theme_field() {
        let mut theme = Theme::default();
        theme.widgets.divider.height = Some(4.0);
        assert_eq!(
            Widget::<()>::style_themed(&Divider::new(), &theme).height,
            Dimension::Length(4.0)
        );
        assert_eq!(
            Widget::<()>::style_themed(&VerticalDivider::new(), &theme).width,
            Dimension::Length(4.0)
        );
    }

    /// **A rule can round its ends** — the reference's `Divider.radius`
    /// (`divider.dart:68`). A thick rule reads as a bar, and a bar with square ends is the
    /// only thing left in an interface that still has them.
    #[test]
    fn a_rule_can_round_its_ends() {
        let corners = |divider: &Divider| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                divider,
                Rect::new(0.0, 0.0, 200.0, 16.0),
                Status::default(),
                &Theme::default(),
                &mut scene,
            );
            scene.primitives().iter().find_map(|p| match p {
                frus_core::Primitive::Rect { radius, .. } => Some(*radius),
                _ => None,
            })
        };

        assert_eq!(
            corners(&Divider::new().thickness(6.0).radius(3.0)),
            Some(frus_core::BorderRadius::uniform(3.0))
        );
        assert_eq!(
            corners(&Divider::new().thickness(6.0)),
            Some(frus_core::BorderRadius::ZERO),
            "a hairline is still square, which is what a hairline wants"
        );
    }

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
