//! [`Chip`]: a compact element for an attribute, a filter or an entry — pressable,
//! selectable, and removable.
//!
//! The reference has four of these (assist, filter, input, suggestion), and what separates
//! them is not shape but **affordance**: whether the chip can be selected, whether it
//! carries a leading icon, whether it can be deleted. They share one set of measurements —
//! 32 px high, an 8 px radius, a `label_large` label, an outline that gives way to a filled
//! surface once selected — so there is one `Chip` here with those affordances as builders:
//!
//! ```ignore
//! Chip::new("Draft")                                   // an attribute
//! Chip::new("Unread").selected(on).on_press(Msg::Toggle)   // a filter
//! Chip::new(name).leading(Icons::Star).on_remove(Msg::Drop(id))  // an entry
//! ```
//!
//! Every measurement and colour is overridable, per call or through
//! [`ChipTheme`](crate::ChipTheme).

use frus_core::{BorderRadius, Color, Insets, Point, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A chip's height.
pub const CHIP_HEIGHT: f32 = 32.0;
/// Its corner radius — a rounded rectangle, not a stadium.
pub const CHIP_RADIUS: f32 = 8.0;
/// The room inside the outline, on every side.
pub const CHIP_PADDING: f32 = 8.0;
/// The room on either side of the label, inside that.
pub const CHIP_LABEL_PADDING: f32 = 8.0;
/// A leading icon, a checkmark or a delete cross.
pub const CHIP_ICON_SIZE: f32 = 18.0;
/// The outline's thickness.
pub const CHIP_BORDER_WIDTH: f32 = 1.0;

/// The icon grid the vector icons are drawn on; see [`crate::icons`].
const ICON_GRID: f32 = 24.0;

/// Everything about a chip's appearance, each part `None` until someone says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ChipStyle {
    pub color: Option<Color>,
    pub selected_color: Option<Color>,
    pub label_color: Option<Color>,
    pub selected_label_color: Option<Color>,
    pub label_style: Option<TextStyle>,
    pub border_color: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<BorderRadius>,
    pub padding: Option<f32>,
    pub label_padding: Option<f32>,
    pub height: Option<f32>,
    pub icon_size: Option<f32>,
    pub show_checkmark: Option<bool>,
}

impl ChipStyle {
    /// The surface under the label. A chip at rest has **none**: the outline is what marks
    /// it out, and a fill would make an attribute look like a button.
    fn background(&self, theme: &Theme, selected: bool) -> Color {
        if selected {
            self.selected_color
                .or(theme.widgets.chip.selected_color)
                .unwrap_or(theme.scheme.secondary_container)
        } else {
            self.color
                .or(theme.widgets.chip.color)
                .unwrap_or(Color::TRANSPARENT)
        }
    }

    fn label_color(&self, theme: &Theme, selected: bool) -> Color {
        if selected {
            self.selected_label_color
                .or(theme.widgets.chip.selected_label_color)
                .unwrap_or(theme.scheme.on_secondary_container)
        } else {
            self.label_color
                .or(theme.widgets.chip.label_color)
                .unwrap_or(theme.scheme.on_surface_variant)
        }
    }

    fn label_style(&self, theme: &Theme) -> TextStyle {
        self.label_style
            .or(theme.widgets.chip.label_style)
            .unwrap_or(theme.text.label_large)
    }

    /// The outline. A **selected** chip has none by default — it is filled, and an outline
    /// around a filled surface reads as a second border.
    fn border(&self, theme: &Theme, selected: bool) -> (f32, Color) {
        let width = self
            .border_width
            .or(theme.widgets.chip.border_width)
            .unwrap_or(if selected { 0.0 } else { CHIP_BORDER_WIDTH });
        let color = self
            .border_color
            .or(theme.widgets.chip.border_color)
            .unwrap_or(theme.scheme.outline_variant);
        (width, color)
    }

    fn radius(&self, theme: &Theme) -> BorderRadius {
        self.radius
            .or(theme.widgets.chip.radius)
            .unwrap_or(BorderRadius::uniform(CHIP_RADIUS))
    }

    fn padding(&self, theme: &Theme) -> f32 {
        self.padding
            .or(theme.widgets.chip.padding)
            .unwrap_or(CHIP_PADDING)
    }

    fn label_padding(&self, theme: &Theme) -> f32 {
        self.label_padding
            .or(theme.widgets.chip.label_padding)
            .unwrap_or(CHIP_LABEL_PADDING)
    }

    fn height(&self, theme: &Theme) -> f32 {
        self.height
            .or(theme.widgets.chip.height)
            .unwrap_or(CHIP_HEIGHT)
    }

    fn icon_size(&self, theme: &Theme) -> f32 {
        self.icon_size
            .or(theme.widgets.chip.icon_size)
            .unwrap_or(CHIP_ICON_SIZE)
    }

    fn show_checkmark(&self, theme: &Theme) -> bool {
        self.show_checkmark
            .or(theme.widgets.chip.show_checkmark)
            .unwrap_or(true)
    }
}

/// A chip's delete cross: its own hit target, so that removing a chip and pressing it are
/// two different gestures.
struct Remove<Msg> {
    message: Msg,
    /// The chip's style and state, so that the cross resolves its size and colour from the
    /// same theme the chip does — a cross on a selected chip has to read against the fill.
    style: ChipStyle,
    selected: bool,
    /// The chip's availability. A cross that still answered on a disabled chip would be
    /// the one live control on an inert thing.
    enabled: bool,
}

impl<Msg: Clone> Widget<Msg> for Remove<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let size = self.style.icon_size(theme);
        Style {
            width: Dimension::Length(size),
            height: Dimension::Length(size),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Brighter under the pointer: the cross is the one part of a chip that does
        // something different from the chip, and hovering has to say so.
        let size = self.style.icon_size(theme);
        let color = if self.enabled {
            self.style
                .label_color(theme, self.selected)
                .lerp(theme.scheme.on_surface, status.hover_progress)
        } else {
            disabled_content(theme)
        }
        .fade(status.opacity);
        let path = Icons::Close
            .path()
            .scaled(size / ICON_GRID)
            .translated(bounds.x, bounds.y + (bounds.height - size) * 0.5);
        scene.fill_path(&path, color);
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Button).label("Remove");
        // Milestone 320 stopped this cross answering a tap and stopped there. Tab still
        // landed on it and a reader was still told it could be pressed — the same control
        // reported three different ways, and the two that were wrong are the two nobody
        // looks at in a screenshot. Milestone 322's guard is what found it.
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// A compact label — an attribute, a filter or an entry.
pub struct Chip<Msg> {
    label: String,
    selected: bool,
    leading: Option<Icons>,
    on_press: Option<Msg>,
    on_remove: Option<Msg>,
    /// A chip that is shown but cannot be acted on: greyed out and inert, its label still
    /// legible. The reference dims a disabled control rather than hiding it — it is often
    /// the answer to why the rest of a row looks the way it does.
    enabled: bool,
    style: ChipStyle,
    /// The delete cross, if there is one — the chip's only child. The label and the
    /// leading icon are painted by the chip, so that their colour can follow its state.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Chip<Msg> {
    /// Creates a chip with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            leading: None,
            on_press: None,
            on_remove: None,
            enabled: true,
            style: ChipStyle::default(),
            children: Vec::new(),
        }
    }

    /// (Re)builds the delete cross, which carries a copy of the chip's style and state.
    ///
    /// Every builder calls this, because a builder run after `on_remove(…)` would
    /// otherwise be written into a cross that had already been built — the order of a
    /// builder chain is not something a caller should have to think about.
    fn rebuild(mut self) -> Self {
        self.children.clear();
        if let Some(message) = self.on_remove.clone() {
            self.children.push(Box::new(Remove {
                message,
                style: self.style,
                selected: self.selected,
                enabled: self.enabled,
            }));
        }
        self
    }

    /// Marks the chip **selected**: it fills, its label takes the colour that goes with
    /// the fill, and a checkmark appears in the leading slot unless there is an icon there
    /// already (or [`Chip::show_checkmark`] says otherwise).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.rebuild()
    }

    /// A leading icon — the avatar or glyph an entry chip carries.
    pub fn leading(mut self, icon: Icons) -> Self {
        self.leading = Some(icon);
        self.rebuild()
    }

    /// Greys the chip out and makes it inert: no press, no ink, out of the tab order,
    /// and announced to a reader as disabled rather than simply going quiet. Its label
    /// stays legible — the reference dims a disabled control rather than hiding it.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild()
    }

    /// The message the chip emits when pressed. A chip with none is inert, which is what
    /// an attribute chip is.
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Adds a delete cross that emits `message`.
    pub fn on_remove(mut self, message: Msg) -> Self {
        self.on_remove = Some(message);
        self.rebuild()
    }

    /// Whether a selected chip shows a checkmark. On by default.
    pub fn show_checkmark(mut self, show: bool) -> Self {
        self.style.show_checkmark = Some(show);
        self.rebuild()
    }

    /// The surface under an **unselected** chip. Transparent by default.
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self.rebuild()
    }

    /// The surface under a **selected** one.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.style.selected_color = Some(color);
        self.rebuild()
    }

    /// The label's colour when unselected.
    pub fn label_color(mut self, color: Color) -> Self {
        self.style.label_color = Some(color);
        self.rebuild()
    }

    /// The label's colour when selected.
    pub fn selected_label_color(mut self, color: Color) -> Self {
        self.style.selected_label_color = Some(color);
        self.rebuild()
    }

    /// The label's type. Defaults to the theme's `label_large` step.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.style.label_style = Some(style);
        self.rebuild()
    }

    /// The outline's colour.
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self.rebuild()
    }

    /// The outline's thickness; `0.0` removes it.
    pub fn border_width(mut self, width: f32) -> Self {
        self.style.border_width = Some(width);
        self.rebuild()
    }

    /// The corner radii (uniform via `f32`, per corner via [`BorderRadius`]).
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.style.radius = Some(radius.into());
        self.rebuild()
    }

    /// The room inside the outline.
    pub fn padding(mut self, padding: f32) -> Self {
        self.style.padding = Some(padding);
        self.rebuild()
    }

    /// The room on either side of the label.
    pub fn label_padding(mut self, padding: f32) -> Self {
        self.style.label_padding = Some(padding);
        self.rebuild()
    }

    /// The chip's height.
    pub fn height(mut self, height: f32) -> Self {
        self.style.height = Some(height);
        self.rebuild()
    }

    /// The size of the leading icon, the checkmark and the delete cross.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.style.icon_size = Some(size);
        self.rebuild()
    }

    /// Whether anything is drawn in the leading slot, and what: an icon if there is one,
    /// otherwise the checkmark of a selected chip.
    fn leading_glyph(&self, theme: &Theme) -> Option<Icons> {
        self.leading
            .or_else(|| (self.selected && self.style.show_checkmark(theme)).then_some(Icons::Check))
    }

    /// Where the label starts, measured from the chip's left edge.
    fn label_offset(&self, theme: &Theme) -> f32 {
        let leading = self
            .leading_glyph(theme)
            .map(|_| self.style.icon_size(theme))
            .unwrap_or(0.0);
        self.style.padding(theme) + leading + self.style.label_padding(theme)
    }

    fn label_size(&self, theme: &Theme) -> frus_core::Size {
        let style = self.style.label_style(theme);
        frus_text::measure_styled(&self.label, style.size, style.weight, style.italic)
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Chip<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        // The chip is sized here rather than by its content, because its content is
        // mostly painted rather than laid out: the label and the leading glyph belong to
        // the chip, so that their colour can follow its state. What is left is the delete
        // cross, and the padding is what puts it after the label.
        let pad = self.style.padding(theme);
        let label_pad = self.style.label_padding(theme);
        let icon = self.style.icon_size(theme);
        let after_label = self.label_offset(theme) + self.label_size(theme).width + label_pad;
        let removable = !self.children.is_empty();
        Style {
            width: Dimension::Length(after_label + if removable { icon } else { 0.0 } + pad),
            height: Dimension::Length(self.style.height(theme)),
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            padding: Insets::new(0.0, pad, 0.0, after_label),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let selected = self.selected;
        let (border_width, border_color) = self.style.border(theme, selected);
        // Disabled flattens the same way `Button` does, and for the same reason: the
        // reference collapses every variant to `on_surface` at 12 % under a label at 38 %,
        // so unavailable reads as unavailable rather than as a quieter kind of chip.
        // Selected or not stops mattering, which is the point — a disabled filter is not
        // offering to tell you whether it is on.
        let (background, border_color, label_color) = if self.enabled {
            (
                self.style.background(theme, selected),
                border_color,
                self.style.label_color(theme, selected),
            )
        } else {
            (
                if selected {
                    disabled_container(theme)
                } else {
                    Color::TRANSPARENT
                },
                disabled_container(theme),
                disabled_content(theme),
            )
        };

        scene.draw_rect(
            bounds,
            background.fade(o),
            self.style.radius(theme),
            border_width,
            if border_width > 0.0 {
                border_color.fade(o)
            } else {
                Color::TRANSPARENT
            },
        );

        // The leading slot: an icon if the chip has one, otherwise a selected chip's
        // checkmark. An icon takes the accent when the chip is at rest, and the colour of
        // the fill once it is selected — the reference's icon theme, which is a different
        // colour from the label's only in the first case.
        let icon_size = self.style.icon_size(theme);
        if let Some(name) = self.leading_glyph(theme) {
            let color = if selected {
                label_color
            } else {
                theme.scheme.primary
            };
            let path = name.path().scaled(icon_size / ICON_GRID).translated(
                bounds.x + self.style.padding(theme),
                bounds.y + (bounds.height - icon_size) * 0.5,
            );
            scene.fill_path(&path, color.fade(o));
        }

        let label_style = self.style.label_style(theme);
        let measured = self.label_size(theme);
        scene.text_styled(
            Point::new(
                bounds.x + self.label_offset(theme),
                bounds.y + (bounds.height - measured.height) * 0.5,
            ),
            self.label.clone(),
            &label_style,
            label_color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.on_press.clone()).flatten()
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        // Only where there is something to press: ink on an inert attribute chip — or on
        // a disabled one — would promise an action it does not have.
        if !self.enabled {
            return None;
        }
        self.on_press.as_ref()?;
        Some(crate::InkStyle::of(theme).radius(self.style.radius(theme)))
    }

    fn focusable(&self) -> bool {
        self.enabled && self.on_press.is_some()
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Button).label(self.label.clone());
        // A chip that can be selected says whether it is: a filter that is on and one that
        // is off are the same words otherwise.
        let semantics = if self.on_press.is_some() && self.enabled {
            semantics.toggled(self.selected).clickable()
        } else {
            semantics
        };
        // Still announced, and announced as unavailable: a reader that simply stopped
        // hearing about a chip would be told the filter had gone away.
        let semantics = if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        };
        Some(semantics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Point as P, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Press,
        Remove,
    }

    fn frame(chip: Chip<Msg>) -> crate::Ui<Msg> {
        let root = crate::flex::Flex::column()
            .width(200.0)
            .height(60.0)
            .child(chip);
        build_ui(
            &root,
            Size::new(200.0, 60.0),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    fn primitives(chip: Chip<Msg>) -> Vec<Primitive> {
        frame(chip).scene().primitives().to_vec()
    }

    /// The chip's own box: the first rectangle painted.
    fn surface(chip: Chip<Msg>) -> (Rect, Color, BorderRadius, f32) {
        primitives(chip)
            .iter()
            .find_map(|p| match p {
                Primitive::Rect {
                    rect,
                    color,
                    radius,
                    border_width,
                    ..
                } => Some((*rect, *color, *radius, *border_width)),
                _ => None,
            })
            .expect("a chip paints its box")
    }

    /// The same gap, in the widget that recorded it first (milestone 312). A disabled
    /// chip is greyed out **and** inert: no press, no ink, out of the tab order, its
    /// delete cross dead too — a live cross on an inert chip would be the one thing on it
    /// that still answered.
    #[test]
    fn a_disabled_chip_is_inert_including_its_cross() {
        let live = Chip::new("Filter")
            .on_press(Msg::Press)
            .on_remove(Msg::Remove);
        let dead = Chip::new("Filter")
            .on_press(Msg::Press)
            .on_remove(Msg::Remove)
            .enabled(false);
        assert_eq!(Widget::on_click(&live), Some(Msg::Press));
        assert_eq!(Widget::on_click(&dead), None, "the press goes nowhere");
        assert!(Widget::<Msg>::focusable(&live));
        assert!(!Widget::<Msg>::focusable(&dead), "out of the tab order");
        assert!(Widget::<Msg>::ink(&live, &Theme::default()).is_some());
        assert!(
            Widget::<Msg>::ink(&dead, &Theme::default()).is_none(),
            "nothing to splash for"
        );
        // The cross is a child, and it has to follow the chip it sits on.
        let cross = |c: &Chip<Msg>| Widget::<Msg>::children(c)[0].on_click();
        assert_eq!(cross(&live), Some(Msg::Remove));
        assert_eq!(cross(&dead), None, "the cross follows the chip");
        // Announced, and announced as unavailable: a reader that stopped hearing about a
        // chip would be told the filter had gone away.
        let semantics = Widget::<Msg>::semantics(&dead).unwrap();
        assert!(semantics.disabled);
        assert_eq!(semantics.label.as_deref(), Some("Filter"));
    }

    /// Disabled must never draw the eye more than live. The measure is **contrast
    /// against the surface**, not brightness: on a light theme a quieter colour is
    /// closer to white, on a dark one closer to black, and a test that compared raw
    /// luminance would pass on one and fail on the other for no real reason.
    ///
    /// It is not obvious from the rule that this holds: a live unselected chip's label
    /// is `on_surface_variant` and a disabled one's is `on_surface` at 38 % — two
    /// different tokens, and which reads louder depends on the palette.
    #[test]
    fn disabled_is_never_louder_than_live() {
        for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
            let bg = theme.scheme.surface;
            let lum = |r: f32, g: f32, b: f32| 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let ground = lum(bg.r, bg.g, bg.b);
            // How far the colour ends up from the surface behind it, alpha included.
            let contrast = |c: frus_core::Color| {
                let over = |f: f32, b: f32| f * c.a + b * (1.0 - c.a);
                (lum(over(c.r, bg.r), over(c.g, bg.g), over(c.b, bg.b)) - ground).abs()
            };
            let live = ChipStyle::default().label_color(&theme, false);
            let dead = theme.scheme.on_surface.fade(0.38);
            assert!(
                contrast(dead) < contrast(live),
                "{name}: the disabled label must be quieter, got {} against {}",
                contrast(dead),
                contrast(live)
            );
            // The **outline** now clears the same bar. Milestone 320 left this assertion
            // out rather than weaken it: the rule was the reference's exactly, but both
            // shipped palettes put `outline_variant` on top of `on_surface` at 12 %, so a
            // disabled hairline carried marginally *more* contrast than a live one. J325
            // moved the two outline roles to their reference tones, which is what lets the
            // claim be asserted instead of written up.
            let live_edge = ChipStyle::default().border(&theme, false).1;
            let dead_edge = theme.scheme.on_surface.fade(0.12);
            assert!(
                contrast(dead_edge) < contrast(live_edge),
                "{name}: the disabled outline must be quieter, got {} against {}",
                contrast(dead_edge),
                contrast(live_edge)
            );
        }
    }

    /// Disabled **flattens** rather than fades, following `Button`: the reference
    /// collapses every state to one grey, so unavailable does not read as a quieter kind
    /// of selected.
    #[test]
    fn a_disabled_chip_flattens_rather_than_fades() {
        let theme = Theme::default();
        let accent = ChipStyle::default().background(&theme, true);
        let fills = |chip: Chip<Msg>| -> Vec<frus_core::Color> {
            frame(chip)
                .scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { color, .. } if color.a > 0.0 => Some(*color),
                    _ => None,
                })
                .collect()
        };
        let selected = || Chip::new("On").on_press(Msg::Press).selected(true);
        assert!(
            fills(selected()).contains(&accent),
            "a live selected chip fills with the accent"
        );
        assert!(
            !fills(selected().enabled(false)).contains(&accent),
            "a disabled one never does"
        );
    }

    #[test]
    fn a_chip_is_the_references_size_and_shape() {
        // Not a stadium: the reference's chip is a 32 px rounded rectangle with an 8 px
        // radius, and a pill is a different component.
        let (rect, _, radius, _) = surface(Chip::new("Draft"));
        assert_eq!(rect.height, CHIP_HEIGHT);
        assert_eq!(radius, BorderRadius::uniform(CHIP_RADIUS));
        assert!(
            radius.top_left < rect.height / 2.0,
            "a rounded rectangle, not a pill"
        );
    }

    #[test]
    fn at_rest_it_is_an_outline_and_nothing_else() {
        let theme = Theme::default();
        let (_, color, _, border) = surface(Chip::new("Draft"));
        assert_eq!(
            color,
            Color::TRANSPARENT,
            "no fill under an unselected chip"
        );
        assert_eq!(border, CHIP_BORDER_WIDTH);
        // Selected: it fills, and the outline goes.
        let (_, color, _, border) = surface(Chip::new("Draft").selected(true));
        assert_eq!(color, theme.scheme.secondary_container);
        assert_eq!(border, 0.0, "an outline around a fill reads as two borders");
    }

    #[test]
    fn a_selected_chip_shows_a_checkmark_and_the_label_makes_room_for_it() {
        let count_paths = |chip: Chip<Msg>| {
            primitives(chip)
                .iter()
                .filter(|p| matches!(p, Primitive::Path { .. }))
                .count()
        };
        assert_eq!(count_paths(Chip::new("Unread")), 0);
        assert_eq!(count_paths(Chip::new("Unread").selected(true)), 1);
        assert_eq!(
            count_paths(Chip::new("Unread").selected(true).show_checkmark(false)),
            0,
            "and it can be refused"
        );
        // The label moves aside for it rather than sitting under it.
        let text_x = |chip: Chip<Msg>| {
            primitives(chip)
                .iter()
                .find_map(|p| match p {
                    Primitive::Text { position, .. } => Some(position.x),
                    _ => None,
                })
                .expect("a label")
        };
        assert_eq!(
            text_x(Chip::new("Unread").selected(true)) - text_x(Chip::new("Unread")),
            CHIP_ICON_SIZE
        );
    }

    #[test]
    fn a_leading_icon_takes_the_slot_the_checkmark_would_have() {
        // Both at once would be two glyphs in a 32 px chip; the icon wins, as it does in
        // the reference for an input chip with an avatar.
        let paths = |chip: Chip<Msg>| {
            primitives(chip)
                .iter()
                .filter(|p| matches!(p, Primitive::Path { .. }))
                .count()
        };
        assert_eq!(paths(Chip::new("Ada").leading(Icons::Star)), 1);
        assert_eq!(
            paths(Chip::new("Ada").leading(Icons::Star).selected(true)),
            1,
            "the icon, not the icon and a checkmark"
        );
    }

    #[test]
    fn the_cross_is_a_target_of_its_own() {
        // Pressing a chip and removing it are two gestures, so the cross has to be its own
        // hit target rather than a region of the chip's.
        let chip = || Chip::new("Ada").on_press(Msg::Press).on_remove(Msg::Remove);
        let ui = frame(chip());
        let at = |x: f32| ui.hit(P::new(x, 16.0)).and_then(|id| ui.msg_for(id));
        let width = surface(chip()).0.width;
        assert_eq!(at(20.0), Some(Msg::Press), "the chip presses");
        assert_eq!(at(width - 14.0), Some(Msg::Remove), "the cross removes");
    }

    #[test]
    fn the_cross_is_visible_before_anyone_hovers_it() {
        // It was built with a transparent colour and only appeared once the pointer
        // reached it, which is a delete affordance nobody can find. It resolves its colour
        // against the theme now — and against the chip's **state**, set after it or not.
        let theme = Theme::default();
        let cross = |chip: Chip<Msg>| {
            primitives(chip)
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Path { fill, .. } => *fill,
                    _ => None,
                })
                .expect("the cross is painted")
        };
        assert_eq!(
            cross(Chip::new("Ada").on_remove(Msg::Remove)),
            theme.scheme.on_surface_variant
        );
        assert_eq!(
            cross(Chip::new("Ada").on_remove(Msg::Remove).selected(true)),
            theme.scheme.on_secondary_container,
            "a builder after `on_remove` still reaches the cross"
        );
    }

    #[test]
    fn an_inert_chip_neither_splashes_nor_takes_focus() {
        // An attribute chip is a label. Ink on it would promise an action it has not got.
        let plain = Chip::<Msg>::new("Draft");
        assert!(Widget::<Msg>::ink(&plain, &Theme::default()).is_none());
        assert!(!Widget::<Msg>::focusable(&plain));
        let pressable = Chip::new("Draft").on_press(Msg::Press);
        assert!(Widget::<Msg>::ink(&pressable, &Theme::default()).is_some());
        assert!(Widget::<Msg>::focusable(&pressable));
    }

    #[test]
    fn a_filter_chip_announces_whether_it_is_on() {
        let on = Chip::new("Unread").selected(true).on_press(Msg::Press);
        let semantics = Widget::<Msg>::semantics(&on).expect("semantics");
        assert_eq!(semantics.label.as_deref(), Some("Unread"));
        assert_eq!(semantics.toggled, frus_core::Toggled::True);
        let off = Chip::new("Unread").on_press(Msg::Press);
        assert_eq!(
            Widget::<Msg>::semantics(&off).unwrap().toggled,
            frus_core::Toggled::False
        );
    }

    #[test]
    fn every_measurement_is_the_callers_and_then_the_themes() {
        let mut theme = Theme::default();
        theme.widgets.chip.height = Some(40.0);
        let height =
            |chip: Chip<Msg>, theme: &Theme| match Widget::<Msg>::style_themed(&chip, theme).height
            {
                Dimension::Length(h) => h,
                other => panic!("{other:?}"),
            };
        assert_eq!(
            height(Chip::new("x"), &Theme::default()),
            CHIP_HEIGHT,
            "the framework's"
        );
        assert_eq!(height(Chip::new("x"), &theme), 40.0, "the theme's");
        assert_eq!(
            height(Chip::new("x").height(24.0), &theme),
            24.0,
            "the caller's, over the theme's"
        );
    }
}
