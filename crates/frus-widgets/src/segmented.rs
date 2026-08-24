//! [`SegmentedButton`]: a **controlled** segmented picker — one outline around several
//! segments, of which one is chosen.
//!
//! It is one control rather than a row of buttons: a single outline runs around the group,
//! hairlines divide the segments, and the chosen one fills with the tonal container and
//! takes a checkmark. That is the shape of the reference's segmented button, and it is what
//! distinguishes the control from three buttons that happen to be touching — which is what
//! this was until milestone 314.
//!
//! ```ignore
//! SegmentedButton::new(selected, Msg::Pick)
//!     .segment("Day")
//!     .segment("Week")
//!     .segment("Month")
//! ```
//!
//! Every measurement and colour is overridable, per call or through
//! [`SegmentedTheme`](crate::SegmentedTheme).

use std::rc::Rc;

use frus_core::{BorderRadius, Color, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The control's height.
pub const SEGMENTED_HEIGHT: f32 = 40.0;
/// The room either side of a segment's label.
pub const SEGMENTED_PADDING: f32 = 12.0;
/// The outline around the group, and the hairlines between its segments.
pub const SEGMENTED_BORDER_WIDTH: f32 = 1.0;
/// The checkmark on the chosen segment.
pub const SEGMENTED_ICON_SIZE: f32 = 18.0;
/// Between that checkmark and the label.
pub const SEGMENTED_ICON_GAP: f32 = 8.0;

/// The icon grid the vector icons are drawn on; see [`crate::icons`].
const ICON_GRID: f32 = 24.0;

/// Everything about the control's appearance, each part `None` until someone says
/// otherwise. Shared by the control and its segments, so that the outline the one draws and
/// the fills the others draw agree about where the edges are.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SegmentedStyle {
    pub selected_color: Option<Color>,
    pub label_color: Option<Color>,
    pub selected_label_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
    pub height: Option<f32>,
    pub padding: Option<f32>,
    pub label_style: Option<TextStyle>,
    pub icon_size: Option<f32>,
    pub show_selected_icon: Option<bool>,
}

impl SegmentedStyle {
    fn selected_color(&self, theme: &Theme) -> Color {
        self.selected_color
            .or(theme.widgets.segmented.selected_color)
            .unwrap_or(theme.scheme.secondary_container)
    }

    fn label_color(&self, theme: &Theme, selected: bool) -> Color {
        if selected {
            self.selected_label_color
                .or(theme.widgets.segmented.selected_label_color)
                .unwrap_or(theme.scheme.on_secondary_container)
        } else {
            self.label_color
                .or(theme.widgets.segmented.label_color)
                .unwrap_or(theme.scheme.on_surface)
        }
    }

    fn border_color(&self, theme: &Theme) -> Color {
        self.border_color
            .or(theme.widgets.segmented.border_color)
            .unwrap_or(theme.scheme.outline)
    }

    fn border_width(&self, theme: &Theme) -> f32 {
        self.border_width
            .or(theme.widgets.segmented.border_width)
            .unwrap_or(SEGMENTED_BORDER_WIDTH)
    }

    fn height(&self, theme: &Theme) -> f32 {
        self.height
            .or(theme.widgets.segmented.height)
            .unwrap_or(SEGMENTED_HEIGHT)
    }

    /// The group's outer corners. Unset, the ends are **stadium** — half the height — which
    /// is what the reference's shape means when the height is free to change.
    fn radius(&self, theme: &Theme) -> f32 {
        self.radius
            .or(theme.widgets.segmented.radius)
            .unwrap_or(self.height(theme) / 2.0)
    }

    fn padding(&self, theme: &Theme) -> f32 {
        self.padding
            .or(theme.widgets.segmented.padding)
            .unwrap_or(SEGMENTED_PADDING)
    }

    fn label_style(&self, theme: &Theme) -> TextStyle {
        self.label_style
            .or(theme.widgets.segmented.label_style)
            .unwrap_or(theme.text.label_large)
    }

    fn icon_size(&self, theme: &Theme) -> f32 {
        self.icon_size
            .or(theme.widgets.segmented.icon_size)
            .unwrap_or(SEGMENTED_ICON_SIZE)
    }

    fn show_selected_icon(&self, theme: &Theme) -> bool {
        self.show_selected_icon
            .or(theme.widgets.segmented.show_selected_icon)
            .unwrap_or(true)
    }

    /// The width **every** segment takes: the widest label, plus the checkmark's room if
    /// there is one, plus the padding.
    ///
    /// The reference gives every segment the width of the widest, which is why this is
    /// asked of the **control** rather than of a segment: a segment that sized itself
    /// would make a control whose divisions move when a label is renamed. It is the
    /// control's natural width; the segments then divide whatever width it is granted,
    /// equally, which comes to the same thing whenever there is room for it.
    fn segment_width(&self, theme: &Theme, labels: &[String]) -> f32 {
        let style = self.label_style(theme);
        let widest = labels
            .iter()
            .map(|label| frus_text::measure_style(label, style).width)
            .fold(0.0_f32, f32::max);
        let icon = if self.show_selected_icon(theme) {
            self.icon_size(theme) + SEGMENTED_ICON_GAP
        } else {
            0.0
        };
        (widest + icon + self.padding(theme) * 2.0).ceil()
    }
}

/// One segment: a fill when it is the chosen one, a label, and a tap.
struct Segment<Msg> {
    label: String,
    index: usize,
    count: usize,
    selected: bool,
    style: SegmentedStyle,
    message: Msg,
    /// The control's availability. A segment is never disabled on its own here — the
    /// reference can disable one of several, which is noted as missing.
    enabled: bool,
}

impl<Msg> Segment<Msg> {
    /// The segment's own corners: the group's radius on the outside, square at the joints.
    fn radius(&self, theme: &Theme) -> BorderRadius {
        let r = self.style.radius(theme);
        match (self.index == 0, self.index + 1 == self.count) {
            (true, true) => BorderRadius::uniform(r),
            (true, false) => BorderRadius {
                top_left: r,
                bottom_left: r,
                top_right: 0.0,
                bottom_right: 0.0,
            },
            (false, true) => BorderRadius {
                top_right: r,
                bottom_right: r,
                top_left: 0.0,
                bottom_left: 0.0,
            },
            (false, false) => BorderRadius::ZERO,
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Segment<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, _theme: &Theme) -> Style {
        Style {
            // **An equal share**, not a fixed width. The reference sizes every segment
            // alike and caps that size at `maxWidth / count`, so the control never runs
            // past its parent; a fixed width sized from the widest label cannot do the
            // capping half, and on a phone the last segment was simply drawn outside the
            // card (found by the overflow survey, milestone 335).
            flex_basis: Dimension::Length(0.0),
            flex_grow: 1.0,
            min_width: Dimension::Length(0.0),
            height: Dimension::Percent(1.0),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let border = self.style.border_width(theme);
        let label_style = self.style.label_style(theme);
        let color = if self.enabled {
            self.style.label_color(theme, self.selected)
        } else {
            disabled_content(theme)
        };

        // The fill sits **inside** the group's outline rather than over it: the control
        // draws one outline around the lot, and a fill painted edge to edge would rub out
        // the part of it that runs along the chosen segment.
        if self.selected {
            let inset = Rect::new(
                bounds.x + border,
                bounds.y + border,
                (bounds.width - border * 2.0).max(0.0),
                (bounds.height - border * 2.0).max(0.0),
            );
            scene.draw_rect(
                inset,
                if self.enabled {
                    self.style.selected_color(theme)
                } else {
                    disabled_container(theme)
                }
                .fade(o),
                self.radius(theme),
                0.0,
                Color::TRANSPARENT,
            );
        }

        // The checkmark and the label are centred **together**, so that the pair reads as
        // one thing rather than a label pushed aside by a tick.
        let icon = self.style.icon_size(theme);
        let shows_icon = self.selected && self.style.show_selected_icon(theme);
        // Cut to the room left once the padding and any checkmark are taken out. A
        // segment that has had to share a narrow control shows an ellipsis rather than a
        // label running into its neighbour.
        let room = bounds.width
            - self.style.padding(theme) * 2.0
            - if shows_icon {
                icon + SEGMENTED_ICON_GAP
            } else {
                0.0
            };
        let label = crate::text::truncate(&self.label, &label_style.resolved(), room);
        let measured = frus_text::measure_style(&label, label_style);
        let group = measured.width
            + if shows_icon {
                icon + SEGMENTED_ICON_GAP
            } else {
                0.0
            };
        let mut x = bounds.x + (bounds.width - group) / 2.0;
        if shows_icon {
            let path = Icons::Check
                .path()
                .scaled(icon / ICON_GRID)
                .translated(x, bounds.y + (bounds.height - icon) / 2.0);
            scene.fill_path(&path, color.fade(o));
            x += icon + SEGMENTED_ICON_GAP;
        }
        scene.text_styled(
            Point::new(x, bounds.y + (bounds.height - measured.height) / 2.0),
            label,
            &label_style.resolved(),
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.message.clone())
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        // No splash where there is nothing to answer it.
        if !self.enabled {
            return None;
        }
        let splash = self.style.label_color(theme, self.selected).fade(0.10);
        Some(
            crate::InkStyle::of(theme)
                .color(splash)
                .radius(self.radius(theme)),
        )
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Button)
            .label(self.label.clone())
            .toggled(self.selected);
        // Still announced, and still saying which one is chosen: a disabled control is
        // read-only, not invisible.
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// A single-selection segmented control.
pub struct SegmentedButton<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Rc<Vec<String>>,
    /// A control that is shown but cannot be used: greyed out and inert, its labels and
    /// its current choice still legible. The reference dims rather than hides.
    enabled: bool,
    style: SegmentedStyle,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SegmentedButton<Msg> {
    /// Creates a control: `selected` is the active index, `on_select(i)` fires on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            labels: Rc::new(Vec::new()),
            enabled: true,
            style: SegmentedStyle::default(),
            children: Vec::new(),
        }
    }

    /// Adds a segment.
    pub fn segment(mut self, label: impl Into<String>) -> Self {
        Rc::make_mut(&mut self.labels).push(label.into());
        self.rebuild();
        self
    }

    /// The radius of the group's outer corners. Unset, the ends are stadium-rounded.
    pub fn radius(mut self, radius: f32) -> Self {
        self.style.radius = Some(radius);
        self.rebuild();
        self
    }

    /// The fill under the chosen segment.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.style.selected_color = Some(color);
        self.rebuild();
        self
    }

    /// The labels' colour.
    pub fn label_color(mut self, color: Color) -> Self {
        self.style.label_color = Some(color);
        self.rebuild();
        self
    }

    /// The chosen segment's label colour, and its checkmark's.
    pub fn selected_label_color(mut self, color: Color) -> Self {
        self.style.selected_label_color = Some(color);
        self.rebuild();
        self
    }

    /// The outline's colour, which is also the hairlines' between segments.
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self.rebuild();
        self
    }

    /// The outline's thickness; `0.0` removes it, hairlines included.
    pub fn border_width(mut self, width: f32) -> Self {
        self.style.border_width = Some(width);
        self.rebuild();
        self
    }

    /// The control's height.
    pub fn height(mut self, height: f32) -> Self {
        self.style.height = Some(height);
        self.rebuild();
        self
    }

    /// The room either side of a label.
    pub fn padding(mut self, padding: f32) -> Self {
        self.style.padding = Some(padding);
        self.rebuild();
        self
    }

    /// The labels' type. Defaults to the theme's `label_large` step.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.style.label_style = Some(style);
        self.rebuild();
        self
    }

    /// Greys the whole control out and makes it inert: no press, no ink, out of the tab
    /// order, and announced as disabled. The labels and the current choice stay legible —
    /// a disabled control is read-only, not invisible.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Whether the chosen segment carries a checkmark. On by default; turning it off gives
    /// every segment back the room it reserved.
    pub fn show_selected_icon(mut self, show: bool) -> Self {
        self.style.show_selected_icon = Some(show);
        self.rebuild();
        self
    }

    /// (Re)builds the segments. Every builder calls it, so that the order of the chain is
    /// not part of the API.
    fn rebuild(&mut self) {
        let count = self.labels.len();
        self.children = (0..count)
            .map(|i| {
                Box::new(Segment {
                    label: self.labels[i].clone(),
                    index: i,
                    count,
                    selected: i == self.selected,
                    style: self.style,
                    message: (self.on_select)(i),
                    enabled: self.enabled,
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for SegmentedButton<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: Dimension::Length(
                self.style.segment_width(theme, &self.labels) * self.labels.len() as f32,
            ),
            // The cap. With room, the control is its natural width and the segments each
            // get exactly `segment_width` back — the same picture as before. Without, it
            // stops at the parent's edge and the segments divide what there is.
            max_width: Dimension::Percent(1.0),
            height: Dimension::Length(self.style.height(theme)),
            flex_direction: FlexDirection::Row,
            // No gap: the segments touch, and the hairline between them is drawn here.
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    /// The **group's** outline and the hairlines between its segments — drawn by the
    /// control, under the segments, because they belong to the control rather than to any
    /// one of them. Segments that drew their own would double every join.
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let count = self.labels.len();
        let width = self.style.border_width(theme);
        if count == 0 || width <= 0.0 || bounds.width <= 0.0 {
            return;
        }
        let o = status.opacity;
        // Disabled flattens to `on_surface` at 12 %, as `Button` and `Chip` do: the same
        // grey everywhere says unavailable, where a faded accent only says quieter.
        let color = if self.enabled {
            self.style.border_color(theme)
        } else {
            disabled_container(theme)
        }
        .fade(o);
        scene.draw_rect(
            bounds,
            Color::TRANSPARENT,
            BorderRadius::uniform(self.style.radius(theme)),
            width,
            color,
        );
        // The divisions are the control's **own box** divided by their number, which is
        // exactly how the segments divide it. Taken from the natural segment width
        // instead, the two agreed only while the control had all the room it wanted: on a
        // phone, where milestone 335 caps it, the hairlines stayed at their roomy spacing
        // and stopped falling where the segments meet — visible on a device before this
        // line was changed, and the reason a fix should be looked at as well as measured.
        let segment = bounds.width / count as f32;
        for i in 1..count {
            scene.fill_rect(
                Rect::new(
                    bounds.x + segment * i as f32,
                    bounds.y,
                    width,
                    bounds.height,
                ),
                color,
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
    }

    fn three(selected: usize) -> SegmentedButton<Msg> {
        SegmentedButton::new(selected, Msg::Select)
            .segment("One")
            .segment("Two")
            .segment("Three")
    }

    fn primitives(control: SegmentedButton<Msg>) -> Vec<Primitive> {
        let root = crate::flex::Flex::column()
            .width(400.0)
            .height(60.0)
            .child(control);
        build_ui(
            &root,
            Size::new(400.0, 60.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .to_vec()
    }

    /// The gap milestones 312, 313 and 314 each recorded as missing. Greying out is the
    /// easy half: a disabled control must also be out of the tab order, refuse the press,
    /// splash at nothing, and still tell a reader which segment is chosen — read-only, not
    /// invisible.
    #[test]
    fn a_disabled_control_is_inert_but_still_readable() {
        let live = three(1);
        let dead = three(1).enabled(false);
        fn segments(c: &SegmentedButton<Msg>) -> &[Box<dyn Widget<Msg>>] {
            Widget::<Msg>::children(c)
        }
        for seg in segments(&live) {
            assert!(seg.on_click().is_some());
            assert!(seg.focusable());
        }
        for seg in segments(&dead) {
            assert_eq!(seg.on_click(), None, "disabled: the press goes nowhere");
            assert!(!seg.focusable(), "disabled: out of the tab order");
            assert!(
                seg.ink(&Theme::default()).is_none(),
                "disabled: nothing to splash for"
            );
            let semantics = seg.semantics().expect("still announced");
            assert!(semantics.disabled, "and announced as disabled");
        }
        // The chosen segment is still identifiable to a reader: a control that stops
        // saying which one is on has become a row of words.
        let chosen = segments(&dead)[1].semantics().unwrap();
        assert_eq!(chosen.toggled, frus_core::Toggled::True);
    }

    /// Disabled flattens to one grey — the reference collapses every state to
    /// `on_surface` at 12 % under a label at 38 %, so unavailable reads as unavailable
    /// rather than as a quieter version of the accent.
    #[test]
    fn a_disabled_control_flattens_rather_than_fades() {
        let theme = Theme::default();
        let accent = SegmentedStyle::default().selected_color(&theme);
        let fills = |c: SegmentedButton<Msg>| -> Vec<frus_core::Color> {
            primitives(c)
                .into_iter()
                .filter_map(|p| match p {
                    Primitive::Rect { color, .. } if color.a > 0.0 => Some(color),
                    _ => None,
                })
                .collect()
        };
        assert!(
            fills(three(1)).contains(&accent),
            "a live control fills the chosen segment with the accent"
        );
        assert!(
            !fills(three(1).enabled(false)).contains(&accent),
            "a disabled one never uses it"
        );
    }

    #[test]
    fn click_emits_the_index() {
        let seg = three(0);
        let children = Widget::<Msg>::children(&seg);
        assert_eq!(children.len(), 3);
        assert_eq!(children[2].on_click(), Some(Msg::Select(2)));
    }

    #[test]
    fn one_outline_runs_around_the_group() {
        // Not three outlines touching: a single stroked box with the group's radius, and
        // hairlines where the segments meet.
        let theme = Theme::default();
        let painted = primitives(three(0));
        let outlines: Vec<(Rect, BorderRadius)> = painted
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    radius,
                    border_width,
                    ..
                } if *border_width > 0.0 => Some((*rect, *radius)),
                _ => None,
            })
            .collect();
        assert_eq!(outlines.len(), 1, "one outline, not one per segment");
        assert_eq!(
            outlines[0].1,
            BorderRadius::uniform(SEGMENTED_HEIGHT / 2.0),
            "stadium ends"
        );
        // Two hairlines for three segments, in the outline's colour.
        let hairlines = painted
            .iter()
            .filter(|p| {
                matches!(p, Primitive::Rect { rect, color, border_width, .. }
                    if *border_width == 0.0
                        && *color == theme.scheme.outline
                        && rect.width == SEGMENTED_BORDER_WIDTH)
            })
            .count();
        assert_eq!(hairlines, 2);
        // And they fall where the segments actually meet.
        // Asked of the **control**: a segment no longer carries a width of its own, it
        // takes an equal share of whatever the control is granted (milestone 335). With
        // room, that share is the natural width, which is what this reads.
        let control = three(0);
        let seg_width = match Widget::<Msg>::style_themed(&control, &theme).width {
            Dimension::Length(w) => w / 3.0,
            other => panic!("{other:?}"),
        };
        let xs: Vec<f32> = painted
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    color,
                    border_width,
                    ..
                } if *border_width == 0.0
                    && *color == theme.scheme.outline
                    && rect.width == SEGMENTED_BORDER_WIDTH =>
                {
                    Some(rect.x)
                }
                _ => None,
            })
            .collect();
        assert_eq!(xs, vec![seg_width, seg_width * 2.0]);
    }

    /// The reference caps a segment at `maxWidth / count`, so the control never runs past
    /// its parent. Ours sized every segment from the widest label and let the sum go where
    /// it liked: on a phone the chart dashboard's four segments came to 584 px in a 363 px
    /// row and the last one was drawn 221 px outside the card, which the overflow survey
    /// of milestone 335 found on its first run.
    #[test]
    fn a_control_too_wide_for_its_room_divides_it_instead_of_leaving_it() {
        use crate::interaction::WidgetId;
        use crate::runtime::Runtime;

        let theme = Theme::default();
        let natural = match Widget::<Msg>::style_themed(&three(0), &theme).width {
            Dimension::Length(w) => w,
            other => panic!("{other:?}"),
        };
        let room = natural / 2.0;

        let row = crate::Flex::row().child(three(0)).width(room);
        let runtime = Runtime::default();
        let mut layout = frus_layout::Layout::new();
        let node = crate::ui::build_layout(&row, WidgetId::ROOT, &runtime, &theme, &mut layout);
        layout.compute_filled(node, room, 60.0);

        assert!(
            layout.overflows(node).is_empty(),
            "it fits: {:?}",
            layout.overflows(node)
        );
        let rects: Vec<f32> = layout
            .absolute_rects(node)
            .iter()
            .map(|(r, _)| r.width)
            .collect();
        // Taffy rounds a box to whole pixels, so "exactly the room" is to within one.
        assert!(
            (rects[1] - room).abs() <= 1.0,
            "the control is {}, not the {room} on offer",
            rects[1]
        );
        // Three segments, an equal share each.
        for (i, w) in rects[2..5].iter().enumerate() {
            assert!(
                (w - room / 3.0).abs() <= 1.0,
                "segment {i} is {w}, not a third of {room}"
            );
        }
    }

    /// And the hairlines follow. They used to be spaced by the natural segment width,
    /// which is the same number only while the control has all the room it wants.
    #[test]
    fn the_hairlines_fall_where_the_segments_meet_however_narrow_it_is() {
        let theme = Theme::default();
        let control = three(0);
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut scene = Scene::new();
        Widget::<Msg>::paint(&control, bounds, Status::default(), &theme, &mut scene);
        let xs: Vec<f32> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect, border_width, ..
                } if *border_width == 0.0 && rect.width == SEGMENTED_BORDER_WIDTH => Some(rect.x),
                _ => None,
            })
            .collect();
        assert_eq!(xs, vec![40.0, 80.0], "thirds of the box it was given");
    }

    #[test]
    fn the_chosen_segment_fills_and_takes_a_checkmark() {
        let theme = Theme::default();
        let painted = primitives(three(1));
        let fills = painted
            .iter()
            .filter(|p| {
                matches!(p, Primitive::Rect { color, .. } if *color == theme.scheme.secondary_container)
            })
            .count();
        assert_eq!(fills, 1, "one segment fills, and it is the tonal container");
        let checks = painted
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(checks, 1, "and it carries a checkmark");
        assert_eq!(
            primitives(three(1).show_selected_icon(false))
                .iter()
                .filter(|p| matches!(p, Primitive::Path { .. }))
                .count(),
            0,
            "which can be refused"
        );
    }

    #[test]
    fn the_fill_sits_inside_the_outline() {
        // A fill painted edge to edge would rub out the part of the group's outline that
        // runs along the chosen segment.
        let theme = Theme::default();
        let painted = primitives(three(0));
        let outline = painted
            .iter()
            .find_map(|p| match p {
                Primitive::Rect {
                    rect, border_width, ..
                } if *border_width > 0.0 => Some(*rect),
                _ => None,
            })
            .expect("an outline");
        let fill = painted
            .iter()
            .find_map(|p| match p {
                Primitive::Rect { rect, color, .. }
                    if *color == theme.scheme.secondary_container =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("a fill");
        assert!(
            fill.y > outline.y,
            "the fill starts below the outline's edge"
        );
        assert!(fill.y + fill.height < outline.y + outline.height);
    }

    #[test]
    fn every_segment_is_as_wide_as_the_widest() {
        // The reference gives every segment the width of the widest, so that renaming one
        // does not move the divisions between the others.
        let theme = Theme::default();
        let seg = three(0);
        let widths: Vec<Dimension> = Widget::<Msg>::children(&seg)
            .iter()
            .map(|child| child.style_themed(&theme).width)
            .collect();
        assert_eq!(widths[0], widths[1]);
        assert_eq!(widths[1], widths[2]);
    }

    #[test]
    fn a_segment_says_whether_it_is_the_chosen_one() {
        let seg = three(1);
        let children = Widget::<Msg>::children(&seg);
        assert_eq!(
            children[1].semantics().unwrap().toggled,
            frus_core::Toggled::True
        );
        assert_eq!(
            children[0].semantics().unwrap().toggled,
            frus_core::Toggled::False
        );
    }

    #[test]
    fn every_measurement_is_the_callers_and_then_the_themes() {
        let mut theme = Theme::default();
        theme.widgets.segmented.height = Some(48.0);
        let height =
            |control: SegmentedButton<Msg>, theme: &Theme| match Widget::<Msg>::style_themed(
                &control, theme,
            )
            .height
            {
                Dimension::Length(h) => h,
                other => panic!("{other:?}"),
            };
        assert_eq!(
            height(three(0), &Theme::default()),
            SEGMENTED_HEIGHT,
            "the framework's"
        );
        assert_eq!(height(three(0), &theme), 48.0, "the theme's");
        assert_eq!(
            height(three(0).height(32.0), &theme),
            32.0,
            "the caller's, over the theme's"
        );
    }
}
