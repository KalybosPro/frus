//! [`Tooltip`]: a short label that appears while a pointer rests on something.
//!
//! The framework has had the *place a tooltip goes* since the overlay system was built
//! — [`Placement::Tooltip`] positions a bubble above its anchor, flips it below when
//! there is no room above, nudges it back inside a window edge, and shows it only while
//! the anchor is hovered — and **nothing used it**. This is the widget that does.

use frus_core::{
    BorderRadius, Color, Insets, Point, Rect, ResolvedTextStyle, Scene, ShapeBorder, TextAlign,
    TextStyle,
};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// The room kept round the label. The reference gives a tooltip sixteen pixels either
/// side on a touch platform and eight on a pointer one (`tooltip.dart`); this takes the
/// pointer pair, since a bubble that appears on hover is a pointer's idiom, and leaves
/// both to [`Tooltip::padding`].
const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 4.0;

/// How wide a bubble is allowed to get before its text wraps. Not a number the reference
/// has — it constrains a tooltip by the window — but a bubble as wide as a desktop
/// window is not a tooltip, it is a paragraph that appeared under the mouse.
const MAX_WIDTH: f32 = 320.0;

/// The bubble's corner (`tooltip.dart:496`). Small, and deliberately not
/// `Theme::radius`: a tooltip is not a surface of the interface, it is a label on top of
/// one, and it reads as attached to what it describes rather than as another panel.
const RADIUS: f32 = 4.0;

/// **A short label that appears while a pointer rests on something.**
///
/// ```ignore
/// Tooltip::new("Delete this row").child(IconButton::new(Icons::DELETE).on_press(Msg::Delete))
/// ```
///
/// It wraps a child, shows nothing until the child is hovered, and then floats a bubble
/// above it — below, if there is no room above. An empty message shows nothing at all
/// and costs nothing, which is what lets a caller write
/// `Tooltip::new(maybe_hint).child(..)` without branching.
///
/// # What this cannot do yet
///
/// The reference's `waitDuration`, `showDuration`, `exitDuration` and `triggerMode` are
/// **not** here. All four are about *time*: how long a pointer must rest before the
/// bubble appears, how long it stays, and whether a long press brings it up on a touch
/// screen. The framework shows the bubble while the anchor is hovered and hides it when
/// the pointer leaves, which is the whole of what the overlay system can express today.
/// See the roadmap.
pub struct Tooltip<Msg> {
    /// `[child]`, or `[child, bubble]` when there is something to show.
    children: Vec<Box<dyn Widget<Msg>>>,
    message: String,
    enabled: bool,
    background: Option<Color>,
    text_style: Option<TextStyle>,
    text_color: Option<Color>,
    text_align: Option<TextAlign>,
    padding: Option<Insets>,
    shape: Option<ShapeBorder>,
    max_width: Option<f32>,
}

impl<Msg: Clone + 'static> Tooltip<Msg> {
    /// Creates a tooltip carrying `message`. Add the thing it describes with
    /// [`child`](Self::child).
    ///
    /// An **empty** message is not an error: the tooltip becomes the child and nothing
    /// else, so a caller with an optional hint does not have to branch.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            children: Vec::new(),
            message: message.into(),
            enabled: true,
            background: None,
            text_style: None,
            text_color: None,
            text_align: None,
            padding: None,
            shape: None,
            max_width: None,
        }
    }

    /// The widget the tooltip describes.
    #[must_use]
    pub fn child(mut self, child: impl Widget<Msg> + 'static) -> Self {
        if self.children.is_empty() {
            self.children.push(Box::new(child));
        } else {
            self.children[0] = Box::new(child);
        }
        self.rebuild();
        self
    }

    /// Whether the tooltip shows at all. A disabled one is its child and nothing more.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// **The bubble's surface**, over the theme's and `inverse_surface`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.rebuild();
        self
    }

    /// The label's type, over the theme's and `body_small`.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// The label's colour, over the theme's and `on_inverse_surface`.
    #[must_use]
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self.rebuild();
        self
    }

    /// How the label sits in the bubble when it has wrapped onto more than one line.
    #[must_use]
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.text_align = Some(align);
        self.rebuild();
        self
    }

    /// The room kept round the label.
    #[must_use]
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self.rebuild();
        self
    }

    /// **What shape the bubble is**, over the theme's and the framework's four.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self.rebuild();
        self
    }

    /// The shorthand for a rounded rectangle.
    #[must_use]
    pub fn radius(self, radius: impl Into<BorderRadius>) -> Self {
        self.shape(ShapeBorder::rounded(radius.into()))
    }

    /// **How wide the bubble may get** before the label wraps.
    #[must_use]
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self.rebuild();
        self
    }

    /// (Re)builds the bubble (child 1).
    ///
    /// Every builder calls this, so **the order they are written in does not matter** —
    /// the trap milestone 458 found in [`BottomSheet`](crate::BottomSheet), where the
    /// panel is built by one particular method and anything said after it is dropped.
    fn rebuild(&mut self) {
        self.children.truncate(1);
        if self.children.is_empty() || self.message.is_empty() || !self.enabled {
            return;
        }
        self.children.push(Box::new(Bubble {
            message: self.message.clone(),
            background: self.background,
            text_style: self.text_style,
            text_color: self.text_color,
            text_align: self.text_align,
            padding: self.padding,
            shape: self.shape,
            max_width: self.max_width,
        }));
    }
}

impl<Msg: Clone> Widget<Msg> for Tooltip<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    /// A tooltip is not a box: it is whatever its child is, with something floating over
    /// it. It forwards the child's structure the way every transparent wrapper in this
    /// crate has to (milestone 425) — a tooltip round a row must not turn the row into a
    /// column.
    fn style_themed(&self, theme: &Theme) -> Style {
        match self.children.first() {
            Some(child) => child.style_themed(theme),
            None => Style::default(),
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|bubble| (bubble.as_ref(), Placement::Tooltip))
    }

    /// A tooltip **says** what it says. Without this the bubble is a picture: the label
    /// is drawn, a sighted reader gets the hint, and a reader who cannot see it gets
    /// nothing — on a control whose whole reason for having a tooltip is usually that
    /// its icon does not say what it does.
    ///
    /// It goes in as a **label**, which
    /// [`SemanticsProperties::over`](frus_core::SemanticsProperties::over) joins to the
    /// child's own on its own line, so a reader hears the hint and the control. The
    /// reference has a separate `tooltip` property and both platforms have a field for
    /// it (`AccessibilityNodeInfo.setTooltipText`, `AXHelp`); this framework's
    /// `SemanticsProperties` does not, and adding it is a change to the platform bridge
    /// rather than to a widget. See the roadmap.
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        if self.message.is_empty() || !self.enabled {
            return None;
        }
        Some(frus_core::SemanticsProperties::new(frus_core::Role::None).label(self.message.clone()))
    }
}

/// The floating label itself.
///
/// A widget rather than a `Container` holding a `Text`, for the reason every themed
/// panel in this crate is one: the rungs resolve against a `&Theme`, and a tree built by
/// a builder has none in hand.
struct Bubble {
    message: String,
    background: Option<Color>,
    text_style: Option<TextStyle>,
    text_color: Option<Color>,
    text_align: Option<TextAlign>,
    padding: Option<Insets>,
    shape: Option<ShapeBorder>,
    max_width: Option<f32>,
}

impl Bubble {
    /// The label's type, **resolved once** so that the number the box is measured with is
    /// the number the glyphs are drawn at. Resolving is the single place the reader's
    /// font setting is applied (milestone 403).
    fn label_style(&self, theme: Option<&Theme>) -> ResolvedTextStyle {
        self.text_style
            .or(theme.and_then(|t| t.widgets.tooltip.text_style))
            .unwrap_or_else(|| crate::theme::type_scale(theme).body_small)
            .resolved()
    }

    /// Where the lines sit in the bubble once the label has wrapped. It travels with the
    /// width rather than on the style, because an alignment with nothing to align
    /// against is not a setting — see [`TextBlock`](frus_core::TextBlock).
    fn align(&self, theme: Option<&Theme>) -> TextAlign {
        self.text_align
            .or(theme.and_then(|t| t.widgets.tooltip.text_align))
            .unwrap_or(TextAlign::Start)
    }

    fn padding(&self, theme: Option<&Theme>) -> Insets {
        self.padding
            .or(theme.and_then(|t| t.widgets.tooltip.padding))
            .unwrap_or(Insets::new(PAD_Y, PAD_X, PAD_Y, PAD_X))
    }

    fn max_width(&self, theme: Option<&Theme>) -> f32 {
        self.max_width
            .or(theme.and_then(|t| t.widgets.tooltip.max_width))
            .unwrap_or(MAX_WIDTH)
    }

    fn shape_of(&self, theme: &Theme) -> ShapeBorder {
        crate::resolve_shape(
            self.shape,
            theme.widgets.tooltip.shape,
            theme.widgets.tooltip.radius.map(BorderRadius::uniform),
            ShapeBorder::rounded(RADIUS),
        )
    }

    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let padding = self.padding(theme);
        let style = self.label_style(theme);
        let room = (self.max_width(theme) - padding.left - padding.right).max(1.0);
        let measured = frus_text::measure_wrapped_resolved(&self.message, &style, Some(room));
        Style {
            width: Dimension::Length((measured.width + padding.left + padding.right).ceil()),
            height: Dimension::Length((measured.height + padding.top + padding.bottom).ceil()),
            ..Default::default()
        }
    }
}

impl<Msg> Widget<Msg> for Bubble {
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
        let t = &theme.widgets.tooltip;
        // The **inverted** surface, which is what a tooltip is everywhere: a label that
        // does not read as another panel of the interface. The reference builds it by
        // hand from white and grey with the theme's brightness in a `switch`
        // (`tooltip.dart:481`); the scheme has had the role for it since it was written.
        let fill = self
            .background
            .or(t.background)
            .unwrap_or(theme.scheme.inverse_surface);
        scene.draw_shape(bounds, self.shape_of(theme), fill.fade(o));
        let padding = self.padding(Some(theme));
        let style = self.label_style(Some(theme));
        let ink = self
            .text_color
            .or(t.text_color)
            .unwrap_or(theme.scheme.on_inverse_surface);
        scene.text_block(
            Point::new(bounds.x + padding.left, bounds.y + padding.top),
            self.message.clone(),
            &style,
            ink.fade(o),
            frus_core::TextBlock {
                width: Some((bounds.width - padding.left - padding.right).max(0.0)),
                soft_wrap: true,
                align: self.align(Some(theme)),
            },
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size, Text};
    use frus_core::Primitive;

    fn child() -> Container<()> {
        Container::<()>::new().width(40.0).height(20.0)
    }

    /// **The framework had the place a tooltip goes and no tooltip.**
    /// `Placement::Tooltip` positions a bubble above its anchor, flips it below when
    /// there is no room, nudges it inside a window edge and shows it only while the
    /// anchor is hovered — and nothing in the crate used it.
    #[test]
    fn a_tooltip_floats_a_bubble_over_its_child() {
        let tip = Tooltip::<()>::new("Delete this row").child(child());
        assert!(Widget::<()>::overlay(&tip).is_some());
        assert_eq!(
            Widget::<()>::overlay(&tip).map(|(_, placement)| placement),
            Some(Placement::Tooltip)
        );
        assert_eq!(Widget::<()>::children(&tip).len(), 2);
    }

    /// **Nothing to say costs nothing.** An empty message, or a disabled tooltip, is the
    /// child and no overlay at all — which is what lets a caller write
    /// `Tooltip::new(hint)` where `hint` may be empty without branching.
    #[test]
    fn an_empty_message_is_no_tooltip() {
        let empty = Tooltip::<()>::new("").child(child());
        assert!(Widget::<()>::overlay(&empty).is_none());
        assert_eq!(Widget::<()>::children(&empty).len(), 1);

        let off = Tooltip::<()>::new("Something")
            .child(child())
            .enabled(false);
        assert!(Widget::<()>::overlay(&off).is_none());
    }

    /// **A tooltip is not a box.** It is whatever its child is, with something floating
    /// over it — so it forwards the child's own layout rather than replacing it with a
    /// default. Milestone 425's rule, which every transparent wrapper here has to keep.
    #[test]
    fn a_tooltip_forwards_its_child_s_layout() {
        let theme = Theme::default();
        let bare = child().width(123.0);
        let wrapped = Tooltip::<()>::new("hint").child(child().width(123.0));
        assert_eq!(
            Widget::<()>::style_themed(&wrapped, &theme).width,
            Widget::<()>::style_themed(&bare, &theme).width
        );
    }

    /// **A tooltip says what it says.** Without semantics the bubble is a picture: a
    /// sighted reader gets the hint and a reader who cannot see it gets nothing, on a
    /// control that usually has a tooltip *because* its icon does not say what it does.
    #[test]
    fn a_tooltip_is_spoken() {
        let tip = Tooltip::<()>::new("Delete this row").child(child());
        let semantics = Widget::<()>::semantics(&tip).expect("a tooltip speaks");
        assert_eq!(semantics.label.as_deref(), Some("Delete this row"));
        assert!(semantics.is_meaningful(), "so it reaches the tree at all");
        assert!(Widget::<()>::semantics(&Tooltip::<()>::new("").child(child())).is_none());
    }

    fn bubble_of(tip: &Tooltip<()>, theme: &Theme) -> (frus_core::Color, BorderRadius) {
        let (bubble, _) = Widget::<()>::overlay(tip).expect("a bubble");
        let mut scene = Scene::new();
        bubble.paint(
            Rect::new(0.0, 0.0, 100.0, 24.0),
            Status::default(),
            theme,
            &mut scene,
        );
        scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { color, radius, .. } => Some((*color, *radius)),
                _ => None,
            })
            .expect("a surface")
    }

    /// The bubble is the **inverted** surface — the role the scheme has had all along,
    /// where the reference builds the same thing out of white and grey by hand
    /// (`tooltip.dart:481`) — and every part of it answers to a theme and to a caller
    /// over the theme.
    #[test]
    fn a_bubble_is_the_inverted_surface_and_answers_to_a_theme() {
        let theme = Theme::default();
        let plain = Tooltip::<()>::new("hint").child(child());
        assert_eq!(
            bubble_of(&plain, &theme),
            (theme.scheme.inverse_surface, BorderRadius::uniform(RADIUS))
        );

        let mut themed = Theme::default();
        themed.widgets.tooltip.background = Some(Color::rgb(0.2, 0.4, 0.6));
        themed.widgets.tooltip.radius = Some(11.0);
        assert_eq!(
            bubble_of(&plain, &themed),
            (Color::rgb(0.2, 0.4, 0.6), BorderRadius::uniform(11.0))
        );

        let told = Tooltip::<()>::new("hint")
            .child(child())
            .background(Color::rgb(0.9, 0.1, 0.1))
            .radius(3.0);
        assert_eq!(
            bubble_of(&told, &themed),
            (Color::rgb(0.9, 0.1, 0.1), BorderRadius::uniform(3.0))
        );
    }

    /// **A bubble stops growing.** A hint long enough to fill a desktop window is not a
    /// tooltip any more, so the label wraps at `max_width` — the one number here the
    /// reference does not have, since it constrains a tooltip by the window instead.
    #[test]
    fn a_long_hint_wraps_rather_than_crossing_the_window() {
        let theme = Theme::default();
        let long = "This row cannot be deleted while it is the last one left in the table, \
                    because a table with no rows has nothing to describe its columns.";
        let box_of = |width: Option<f32>| {
            let mut tip = Tooltip::<()>::new(long).child(child());
            if let Some(width) = width {
                tip = tip.max_width(width);
            }
            let (bubble, _) = Widget::<()>::overlay(&tip).expect("a bubble");
            let style = bubble.style_themed(&theme);
            match (style.width, style.height) {
                (Dimension::Length(w), Dimension::Length(h)) => (w, h),
                other => panic!("a measured box, not {other:?}"),
            }
        };

        // The box is the widest line plus its padding, so it lands at or just under the
        // allowance rather than exactly on it — the text breaks at a word.
        let (wide_w, wide_h) = box_of(None);
        assert!(
            wide_w <= MAX_WIDTH && wide_w > MAX_WIDTH - 20.0,
            "it filled the allowance and did not cross it: {wide_w}"
        );
        assert!(wide_h > 30.0, "it wrapped onto several lines: {wide_h}");

        // And a caller can say how wide is wide enough. Narrower means taller: the same
        // words in a narrower box.
        let (narrow_w, narrow_h) = box_of(Some(120.0));
        assert!(narrow_w <= 120.0, "{narrow_w}");
        assert!(narrow_h > wide_h, "{narrow_h} vs {wide_h}");
    }

    /// The bubble shows **only while the child is hovered**, which is the overlay
    /// system's own rule for this placement and the reason a tooltip needs no state of
    /// its own. With nothing hovered, the frame has no bubble in it.
    #[test]
    fn a_bubble_waits_for_the_pointer() {
        let tip = Tooltip::<()>::new("hint").child(child());
        let ui = build_ui(
            &tip,
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let painted = ui.scene().primitives().len();
        assert_eq!(painted, 0, "nothing is hovered, so nothing floats");

        // The same tree with something to draw, to prove the assertion above is not
        // passing because the tooltip draws nothing at all.
        let with_text = Tooltip::<()>::new("hint").child(Text::new("x"));
        let ui = build_ui(
            &with_text,
            Size::new(300.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(!ui.scene().primitives().is_empty(), "the child is drawn");
    }
}
