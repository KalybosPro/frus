//! [`SegmentedControl`]: a **controlled** segmented picker — one outline around several
//! segments, of which one is chosen.
//!
//! It is one control rather than a row of buttons: a single outline runs around the group,
//! hairlines divide the segments, and the chosen one fills with the tonal container and
//! takes a checkmark. That is the shape of the reference's segmented button, and it is what
//! distinguishes the control from three buttons that happen to be touching — which is what
//! this was until milestone 314.
//!
//! ```ignore
//! SegmentedControl::new(selected, Msg::Pick)
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

use crate::icons::IconName;
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
    /// The reference gives every segment the width of the widest, which is why each one
    /// carries the whole list of labels: a segment that sized itself would make a control
    /// whose divisions move when a label is renamed.
    fn segment_width(&self, theme: &Theme, labels: &[String]) -> f32 {
        let style = self.label_style(theme);
        let widest = labels
            .iter()
            .map(|label| {
                frus_text::measure_styled(label, style.size, style.weight, style.italic).width
            })
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
    labels: Rc<Vec<String>>,
    index: usize,
    count: usize,
    selected: bool,
    style: SegmentedStyle,
    message: Msg,
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

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: Dimension::Length(self.style.segment_width(theme, &self.labels)),
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
        let color = self.style.label_color(theme, self.selected);

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
                self.style.selected_color(theme).fade(o),
                self.radius(theme),
                0.0,
                Color::TRANSPARENT,
            );
        }

        // The checkmark and the label are centred **together**, so that the pair reads as
        // one thing rather than a label pushed aside by a tick.
        let measured = frus_text::measure_styled(
            &self.label,
            label_style.size,
            label_style.weight,
            label_style.italic,
        );
        let icon = self.style.icon_size(theme);
        let shows_icon = self.selected && self.style.show_selected_icon(theme);
        let group = measured.width
            + if shows_icon {
                icon + SEGMENTED_ICON_GAP
            } else {
                0.0
            };
        let mut x = bounds.x + (bounds.width - group) / 2.0;
        if shows_icon {
            let path = IconName::Check
                .path()
                .scaled(icon / ICON_GRID)
                .translated(x, bounds.y + (bounds.height - icon) / 2.0);
            scene.fill_path(&path, color.fade(o));
            x += icon + SEGMENTED_ICON_GAP;
        }
        scene.text_styled(
            Point::new(x, bounds.y + (bounds.height - measured.height) / 2.0),
            self.label.clone(),
            &label_style,
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        let splash = self.style.label_color(theme, self.selected).fade(0.10);
        Some(
            crate::InkStyle::of(theme)
                .color(splash)
                .radius(self.radius(theme)),
        )
    }

    fn focusable(&self) -> bool {
        true
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        Some(
            frus_core::Semantics::new(frus_core::Role::Button)
                .label(self.label.clone())
                .toggled(self.selected)
                .clickable(),
        )
    }
}

/// A single-selection segmented control.
pub struct SegmentedControl<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    labels: Rc<Vec<String>>,
    style: SegmentedStyle,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SegmentedControl<Msg> {
    /// Creates a control: `selected` is the active index, `on_select(i)` fires on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            labels: Rc::new(Vec::new()),
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
                    labels: Rc::clone(&self.labels),
                    index: i,
                    count,
                    selected: i == self.selected,
                    style: self.style,
                    message: (self.on_select)(i),
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for SegmentedControl<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            width: Dimension::Length(
                self.style.segment_width(theme, &self.labels) * self.labels.len() as f32,
            ),
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
        let color = self.style.border_color(theme).fade(o);
        scene.draw_rect(
            bounds,
            Color::TRANSPARENT,
            BorderRadius::uniform(self.style.radius(theme)),
            width,
            color,
        );
        // The divisions are taken from the width the segments were **given**, not from the
        // control's box divided by their number: the two agree today, and would stop
        // agreeing the moment anything stretched the control — leaving hairlines that no
        // longer fall where the segments meet.
        let segment = self.style.segment_width(theme, &self.labels);
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

    fn three(selected: usize) -> SegmentedControl<Msg> {
        SegmentedControl::new(selected, Msg::Select)
            .segment("One")
            .segment("Two")
            .segment("Three")
    }

    fn primitives(control: SegmentedControl<Msg>) -> Vec<Primitive> {
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
        let control = three(0);
        let seg_width = match Widget::<Msg>::children(&control)[0]
            .style_themed(&theme)
            .width
        {
            Dimension::Length(w) => w,
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
            |control: SegmentedControl<Msg>, theme: &Theme| match Widget::<Msg>::style_themed(
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
