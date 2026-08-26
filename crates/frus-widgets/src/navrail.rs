//! [`NavigationRail`] and [`BottomBar`]: the two presentations of a single-selection
//! **main navigation**. Same API (`new(selected, on_select).item(icon, label)`);
//! [`crate::NavScaffold`] picks one or the other by size. The "icon" is a text
//! glyph (the framework has no icon font): an emoji, or a Unicode character.

use frus_core::{Color, Insets, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Width of a vertical rail, in logical pixels.
pub(crate) const RAIL_WIDTH: f32 = 76.0;
/// Height of a bottom navigation bar, in logical pixels.
pub(crate) const BAR_HEIGHT: f32 = 60.0;
const ITEM_HEIGHT: f32 = 58.0;
const ICON_SIZE: f32 = 22.0;
/// Notification red (a constant: an alert dot reads as red whatever the
/// theme).
const BADGE_COLOR: Color = Color::rgb(0.90, 0.24, 0.24);

/// The item's label: what the caller said, else what the theme says, else the reference's
/// — a Material 3 rail labels its destinations in `labelMedium`.
///
/// **Resolved once** so that the number the bar is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403).
fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.nav_rail.label_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_medium)
        .resolved()
}

/// The notification count — `labelSmall`, the step [`crate::Badge`] already reads. See
/// [`label_style`].
fn badge_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.nav_rail.badge_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_small)
        .resolved()
}

/// The height an item needs: the constant, unless the icon and a label the reader asked to
/// enlarge no longer fit inside it.
fn item_height(floor: f32, label: &ResolvedTextStyle) -> f32 {
    floor.max(frus_text::line_height(ICON_SIZE) + 2.0 + label.line_height() + 8.0)
}

/// One navigation destination (glyph + label), painted according to its state.
struct NavItem<Msg> {
    icon: String,
    label: String,
    selected: bool,
    /// Notification count (a dot on the icon). `0`/`None` = nothing.
    badge: Option<u32>,
    /// `true` = a rail item (fixed width); `false` = a bar item (flex).
    rail: bool,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
    message: Msg,
}

impl<Msg> NavItem<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let label = label_style(self.label_text_style, theme);
        if self.rail {
            Style {
                width: Dimension::Length(RAIL_WIDTH),
                height: Dimension::Length(item_height(ITEM_HEIGHT, &label)),
                ..Default::default()
            }
        } else {
            // In a bar, the items share the width equally.
            Style {
                flex_grow: 1.0,
                height: Dimension::Length(item_height(BAR_HEIGHT, &label)),
                ..Default::default()
            }
        }
    }
}

impl<Msg: Clone> Widget<Msg> for NavItem<Msg> {
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
        // The icon is a glyph standing in for an icon: `exact`, so that it stays on its
        // own grid while the label beside it follows the reader.
        let icon_style = ResolvedTextStyle::exact(ICON_SIZE);
        let label_s = label_style(self.label_text_style, Some(theme));
        let icon_m = frus_text::measure_resolved(&self.icon, &icon_style);
        let label_m = frus_text::measure_resolved(&self.label, &label_s);
        let gap = 2.0;
        let total_h = icon_m.height + gap + label_m.height;
        let top = bounds.y + ((bounds.height - total_h) * 0.5).max(0.0);

        // Background pill: solid when selected, discreet on hover.
        let pill_w = icon_m.width + 28.0;
        let pill_h = icon_m.height + 8.0;
        let pill = Rect::new(
            bounds.x + (bounds.width - pill_w) * 0.5,
            top - 4.0,
            pill_w,
            pill_h,
        );
        if self.selected {
            scene.draw_rect(
                pill,
                theme.primary.fade(0.16 * o),
                pill_h * 0.5,
                0.0,
                Color::TRANSPARENT,
            );
        } else if status.hover_progress > 0.0 {
            let a = 0.12 * status.hover_progress * o;
            scene.draw_rect(
                pill,
                theme.muted.fade(a),
                pill_h * 0.5,
                0.0,
                Color::TRANSPARENT,
            );
        }

        let color = if self.selected {
            theme.primary
        } else {
            theme.muted
        };
        scene.text(
            Point::new(bounds.x + (bounds.width - icon_m.width) * 0.5, top),
            self.icon.clone(),
            &icon_style,
            color.fade(o),
        );
        scene.text(
            Point::new(
                bounds.x + (bounds.width - label_m.width) * 0.5,
                top + icon_m.height + gap,
            ),
            self.label.clone(),
            &label_s,
            color.fade(o),
        );

        // Notification dot, anchored to the top-right corner of the icon glyph.
        if let Some(count) = self.badge.filter(|&n| n > 0) {
            let text = if count > 99 {
                "99+".to_string()
            } else {
                count.to_string()
            };
            let badge_s = badge_style(self.badge_text_style, Some(theme));
            let m = frus_text::measure_resolved(&text, &badge_s);
            let bw = (m.width + 8.0).max(m.height + 4.0);
            let bh = m.height + 4.0;
            let icon_right = bounds.x + (bounds.width + icon_m.width) * 0.5;
            let bx = (icon_right - bw * 0.4).min(bounds.x + bounds.width - bw);
            let by = top - bh * 0.35;
            let rect = Rect::new(bx, by, bw, bh);
            scene.draw_rect(rect, BADGE_COLOR.fade(o), bh * 0.5, 0.0, Color::TRANSPARENT);
            scene.text(
                Point::new(bx + (bw - m.width) * 0.5, by + 2.0),
                text,
                &badge_s,
                Color::WHITE.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// A declared destination: glyph, label, and an optional badge count.
type Destination = (String, String, Option<u32>);

/// Builds the navigation items from the declared destinations.
fn build_items<Msg: Clone + 'static>(
    items: &[Destination],
    selected: usize,
    on_select: &dyn Fn(usize) -> Msg,
    rail: bool,
    styles: (Option<TextStyle>, Option<TextStyle>),
) -> Vec<Box<dyn Widget<Msg>>> {
    items
        .iter()
        .enumerate()
        .map(|(i, (icon, label, badge))| {
            Box::new(NavItem {
                icon: icon.clone(),
                label: label.clone(),
                selected: i == selected,
                badge: *badge,
                rail,
                label_text_style: styles.0,
                badge_text_style: styles.1,
                message: on_select(i),
            }) as Box<dyn Widget<Msg>>
        })
        .collect()
}

/// A **vertical** navigation rail (tablet / desktop).
pub struct NavigationRail<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    items: Vec<Destination>,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> NavigationRail<Msg> {
    /// Creates a rail: `selected` = the active index, `on_select(i)` on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            items: Vec::new(),
            label_text_style: None,
            badge_text_style: None,
            children: Vec::new(),
        }
    }

    /// Adds a destination (glyph + label).
    pub fn item(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.items.push((icon.into(), label.into(), None));
        self.rebuild();
        self
    }

    /// Adds a notification count to the **last** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.items.last_mut() {
            last.2 = Some(count);
            self.rebuild();
        }
        self
    }

    /// The destinations' labels, over the theme's and the reference's.
    #[must_use]
    pub fn label_text_style(mut self, style: TextStyle) -> Self {
        self.label_text_style = Some(style);
        self.rebuild();
        self
    }

    /// The notification counts, over the theme's and the reference's.
    #[must_use]
    pub fn badge_text_style(mut self, style: TextStyle) -> Self {
        self.badge_text_style = Some(style);
        self.rebuild();
        self
    }

    /// Carries the current destinations *and* styles into the items, so that the builders
    /// are order-independent.
    fn rebuild(&mut self) {
        self.children = build_items(
            &self.items,
            self.selected,
            &*self.on_select,
            true,
            (self.label_text_style, self.badge_text_style),
        );
    }
}

impl<Msg: Clone> Widget<Msg> for NavigationRail<Msg> {
    /// The rail's box: its column of destinations, plus the intrusions it was **told**
    /// about.
    ///
    /// The rail consumes them; its parent does not (milestone 420). The reference keeps
    /// its safe area inside the `Material` (`navigation_rail.dart:553`) and takes the
    /// **leading** side, the top and the bottom — never the trailing one, which is where
    /// the body is. So the rail's surface, and the rule down its edge, run the full
    /// height of the screen while the destinations stay clear of the notch and the
    /// gesture bar.
    fn style(&self) -> Style {
        let safe = crate::MediaQuery::of().padding;
        Style {
            width: Dimension::Length(RAIL_WIDTH + safe.left),
            flex_direction: FlexDirection::Column,
            align: Align::Center,
            padding: Insets::new(8.0 + safe.top, 0.0, 8.0 + safe.bottom, safe.left),
            gap: 4.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Vertical separator on the right edge.
        let x = bounds.x + bounds.width - 1.0;
        scene.fill_rect(
            Rect::new(x, bounds.y, 1.0, bounds.height),
            theme.scheme.outline_variant.fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A **horizontal** navigation bar at the bottom (phone).
pub struct BottomBar<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    items: Vec<Destination>,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> BottomBar<Msg> {
    /// Creates a bar: `selected` = the active index, `on_select(i)` on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            items: Vec::new(),
            label_text_style: None,
            badge_text_style: None,
            children: Vec::new(),
        }
    }

    /// Adds a destination (glyph + label).
    pub fn item(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.items.push((icon.into(), label.into(), None));
        self.rebuild();
        self
    }

    /// Adds a notification count to the **last** destination.
    pub fn badge(mut self, count: u32) -> Self {
        if let Some(last) = self.items.last_mut() {
            last.2 = Some(count);
            self.rebuild();
        }
        self
    }

    /// The destinations' labels, over the theme's and the reference's.
    #[must_use]
    pub fn label_text_style(mut self, style: TextStyle) -> Self {
        self.label_text_style = Some(style);
        self.rebuild();
        self
    }

    /// The notification counts, over the theme's and the reference's.
    #[must_use]
    pub fn badge_text_style(mut self, style: TextStyle) -> Self {
        self.badge_text_style = Some(style);
        self.rebuild();
        self
    }

    /// Carries the current destinations *and* styles into the items, so that the builders
    /// are order-independent.
    fn rebuild(&mut self) {
        self.children = build_items(
            &self.items,
            self.selected,
            &*self.on_select,
            false,
            (self.label_text_style, self.badge_text_style),
        );
    }
}

impl<Msg> BottomBar<Msg> {
    /// The bar's box: a row of destinations, plus the intrusions it was **told** about.
    ///
    /// The bar consumes them; its parent does not (milestone 418). The reference wraps
    /// the row in a safe area and leaves the `Material` **outside** it
    /// (`navigation_bar.dart:285`), so the background runs behind the gesture bar and
    /// only the destinations are held clear of it. Padding the whole bar from outside
    /// gives the opposite picture: a bar that stops short of the edge with a strip of
    /// the screen behind it showing through.
    ///
    /// The top intrusion is never consumed here. A shell removes it before handing the
    /// slot over (`scaffold.dart:3169`), and a bar along the bottom of a screen has
    /// nothing above it to keep clear of anyway.
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let label = label_style(self.label_text_style, theme);
        let safe = crate::MediaQuery::of().padding;
        Style {
            height: Dimension::Length(item_height(BAR_HEIGHT, &label) + safe.bottom),
            padding: Insets::new(0.0, safe.right, safe.bottom, safe.left),
            flex_direction: FlexDirection::Row,
            justify: Justify::SpaceAround,
            align: Align::Stretch,
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for BottomBar<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // Horizontal separator on the top edge.
        scene.fill_rect(
            Rect::new(bounds.x, bounds.y, bounds.width, 1.0),
            theme.scheme.outline_variant.fade(status.opacity),
        );
    }

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
    fn rail_items_emit_index_and_track_selection() {
        let rail = NavigationRail::new(1, Msg::Go)
            .item("H", "Home")
            .item("S", "Search")
            .item("P", "Profile");
        let children = Widget::<Msg>::children(&rail);
        assert_eq!(children.len(), 3);
        assert_eq!(children[2].on_click(), Some(Msg::Go(2)));
    }

    #[test]
    fn badge_decorates_last_item_and_paints_counter() {
        let rail = NavigationRail::new(0, Msg::Go)
            .item("H", "Home")
            .item("M", "Mail")
            .badge(5);
        let children = Widget::<Msg>::children(&rail);
        // The badge paints a dot + the count text on the targeted item.
        let mut scene = Scene::new();
        children[1].paint(
            Rect::new(0.0, 0.0, RAIL_WIDTH, ITEM_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "5")));
        // The item without a badge does not paint that count.
        let mut bare = Scene::new();
        children[0].paint(
            Rect::new(0.0, 0.0, RAIL_WIDTH, ITEM_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut bare,
        );
        assert!(!bare
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "5")));
    }

    #[test]
    fn badge_over_99_is_capped() {
        let bar = BottomBar::new(0, Msg::Go).item("M", "Mail").badge(150);
        let children = Widget::<Msg>::children(&bar);
        let mut scene = Scene::new();
        children[0].paint(
            Rect::new(0.0, 0.0, 80.0, BAR_HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "99+")));
    }

    /// **The bar consumes what it is told about** (milestone 418). Its parent used to
    /// pad it from outside, which put the bar's surface above the gesture bar rather
    /// than behind it; the reference keeps the safe area inside the `Material`
    /// (`navigation_bar.dart:285`) and the bar grows by the intrusion instead.
    #[test]
    fn a_bottom_bar_consumes_the_intrusion_it_was_told_about() {
        use crate::{MediaQuery, Size};
        const GESTURE: f32 = 24.0;
        let bar = BottomBar::new(0, Msg::Go).item("H", "Home");
        let bare = match Widget::<Msg>::style(&bar).height {
            Dimension::Length(h) => h,
            other => panic!("a bar declares a height, not {other:?}"),
        };
        let told = MediaQuery::new(Size::new(400.0, 800.0))
            .with_insets(frus_core::WindowInsets::bars(Insets::new(
                0.0, 0.0, GESTURE, 0.0,
            )))
            .scope(|| Widget::<Msg>::style(&bar));
        match told.height {
            Dimension::Length(h) => assert!(
                (h - (bare + GESTURE)).abs() < 0.01,
                "the bar did not grow by the intrusion: {h} vs {bare}"
            ),
            other => panic!("a bar declares a height, not {other:?}"),
        }
        // And it is the **content** that is held clear, not the box: the padding is what
        // keeps the destinations off the edge while the surface reaches it.
        assert!(
            (told.padding.bottom - GESTURE).abs() < 0.01,
            "the destinations were not held clear: {:?}",
            told.padding
        );
        // Never the top: a shell removes it before handing the slot over, and a bar at
        // the bottom of a screen has nothing above it to avoid.
        assert_eq!(told.padding.top, 0.0);
    }

    #[test]
    fn bottom_bar_items_are_flexible() {
        let bar = BottomBar::new(0, Msg::Go)
            .item("H", "Home")
            .item("S", "Search");
        let children = Widget::<Msg>::children(&bar);
        assert_eq!(children.len(), 2);
        // A bar item: shares the width (flex_grow > 0), no fixed width.
        assert_eq!(Widget::<Msg>::style(&*children[0]).flex_grow, 1.0);
        assert_eq!(children[1].on_click(), Some(Msg::Go(1)));
    }
}
