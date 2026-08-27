//! [`NavigationRail`] and [`BottomBar`]: the two presentations of a single-selection
//! **main navigation**. Same API (`new(selected, on_select).item(icon, label)`);
//! [`crate::NavScaffold`] picks one or the other by size. The "icon" is a text
//! glyph (the framework has no icon font): an emoji, or a Unicode character.

use frus_core::{Color, Insets, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// **When a navigation widget shows its destinations' labels.**
///
/// The reference keeps two names for one idea — `NavigationRailLabelType`
/// (`navigation_rail.dart:1238`) and `NavigationDestinationLabelBehavior`
/// (`navigation_bar.dart:342`) — and gives them **different defaults**, which is the part
/// worth knowing: a rail shows no labels until asked, a bar shows all of them.
///
/// The reason is what each is for. A rail stands beside a page it does not own, and glyphs
/// alone keep it narrow; a bar owns the bottom of the screen and has the room to say what
/// its destinations are.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RailLabels {
    /// Never — glyphs alone. A rail's default.
    None,
    /// On the selected destination only, so the one that matters names itself.
    Selected,
    /// On every destination. A bar's default.
    All,
}

impl RailLabels {
    /// Whether destination `index` shows its label, `selected` being the live one.
    fn shows(self, index: usize, selected: usize) -> bool {
        match self {
            RailLabels::None => false,
            RailLabels::Selected => index == selected,
            RailLabels::All => true,
        }
    }
}

/// Width of a vertical rail, in logical pixels.
pub(crate) const RAIL_WIDTH: f32 = 80.0;
/// Height of a bottom navigation bar, in logical pixels.
pub(crate) const BAR_HEIGHT: f32 = 60.0;
const ITEM_HEIGHT: f32 = 58.0;
/// The destinations' glyphs, at the reference's size (`navigation_bar.dart:1452`).
const ICON_SIZE: f32 = 24.0;
/// The air between a glyph and its label (`navigation_bar.dart:1483`).
const LABEL_GAP: f32 = 4.0;

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
///
/// A destination with **no** label still keeps the floor: a rail whose rows shrank when the
/// labels went away would move every destination the first time one was selected under
/// [`RailLabels::Selected`].
fn item_height(floor: f32, label: Option<&ResolvedTextStyle>) -> f32 {
    let content = frus_text::line_height(ICON_SIZE)
        + label.map_or(0.0, |l| LABEL_GAP + l.line_height())
        + 8.0;
    floor.max(content)
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
    /// Whether this destination says what it is. See [`RailLabels`].
    show_label: bool,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
    message: Msg,
}

impl<Msg> NavItem<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let label = self
            .show_label
            .then(|| label_style(self.label_text_style, theme));
        if self.rail {
            Style {
                width: Dimension::Length(RAIL_WIDTH),
                height: Dimension::Length(item_height(ITEM_HEIGHT, label.as_ref())),
                ..Default::default()
            }
        } else {
            // In a bar, the items share the width equally.
            Style {
                flex_grow: 1.0,
                height: Dimension::Length(item_height(BAR_HEIGHT, label.as_ref())),
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
        let t = &theme.widgets.nav_rail;
        let icon_style = ResolvedTextStyle::exact(t.icon_size.unwrap_or(ICON_SIZE));
        let label_s = label_style(self.label_text_style, Some(theme));
        let icon_m = frus_text::measure_resolved(&self.icon, &icon_style);
        let label_m = self
            .show_label
            .then(|| frus_text::measure_resolved(&self.label, &label_s));
        let gap = LABEL_GAP;
        // With no label the glyph centres on its own, rather than staying where it sat
        // when there was one below it.
        let total_h = icon_m.height + label_m.as_ref().map_or(0.0, |m| gap + m.height);
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
            // The indicator is a **container**, not a wash: the reference fills it with
            // an opaque `secondaryContainer` (`navigation_bar.dart:1463`,
            // `navigation_rail.dart:1272`) where this painted `primary` at 16 %. A tint
            // was the wrong role and the wrong kind of colour at once — a translucent
            // fill blends in linear light here, so 16 % does not paint at 16 %, which is
            // the trap milestone 329 resolved for the disabled tokens.
            scene.draw_rect(
                pill,
                t.indicator_color
                    .unwrap_or(theme.scheme.secondary_container)
                    .fade(o),
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

        // The glyph is drawn **on** the indicator and the label below it, so the two do
        // not take the same colour when selected: the glyph is the indicator's content
        // (`navigation_bar.dart:1456`) and the label is the surface's (`:1476`).
        let icon_color = if self.selected {
            t.selected_icon_color
                .unwrap_or(theme.scheme.on_secondary_container)
        } else {
            t.unselected_icon_color
                .unwrap_or(theme.scheme.on_surface_variant)
        };
        let label_color = if self.selected {
            t.selected_label_color.unwrap_or(theme.scheme.on_surface)
        } else {
            t.unselected_label_color.unwrap_or(if self.rail {
                // The one place the reference answers differently for the two:
                // `navigation_rail.dart:1251` against `navigation_bar.dart:1477`.
                theme.scheme.on_surface
            } else {
                theme.scheme.on_surface_variant
            })
        };
        scene.text(
            Point::new(bounds.x + (bounds.width - icon_m.width) * 0.5, top),
            self.icon.clone(),
            &icon_style,
            icon_color.fade(o),
        );
        if let Some(label_m) = &label_m {
            scene.text(
                Point::new(
                    bounds.x + (bounds.width - label_m.width) * 0.5,
                    top + icon_m.height + gap,
                ),
                self.label.clone(),
                &label_s,
                label_color.fade(o),
            );
        }

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
            // A badge is a badge. This one used to carry a red of its own, on the
            // reasoning that an alert dot reads as red whatever the theme — but the
            // [`Badge`](crate::Badge) widget beside it already answers the same question
            // from the scheme's `error`, and two badges in one framework painting
            // different reds is the part that is actually wrong. Same roles, same theme,
            // so recolouring one recolours both.
            let fill = theme
                .widgets
                .badge
                .background_color
                .unwrap_or(theme.scheme.error);
            let ink = theme
                .widgets
                .badge
                .text_color
                .unwrap_or(theme.scheme.on_error);
            scene.draw_rect(rect, fill.fade(o), bh * 0.5, 0.0, Color::TRANSPARENT);
            scene.text(
                Point::new(bx + (bw - m.width) * 0.5, by + 2.0),
                text,
                &badge_s,
                ink.fade(o),
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
    labels: RailLabels,
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
                show_label: labels.shows(i, selected),
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
    background: Option<Color>,
    labels: RailLabels,
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
            background: None,
            // The reference's default for a **rail** (`navigation_rail.dart:1238`), which
            // is not the one it gives a bar. A rail stands beside a page it does not own
            // and glyphs alone keep it narrow.
            labels: RailLabels::None,
            children: Vec::new(),
        }
    }

    /// When the destinations say what they are. [`RailLabels::None`] by default, as the
    /// reference's rail does.
    #[must_use]
    pub fn labels(mut self, labels: RailLabels) -> Self {
        self.labels = labels;
        self.rebuild();
        self
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

    /// The rail's surface. Unset, the theme's, then the scheme's `surface` — where the
    /// reference puts a rail (`navigation_rail.dart:1202`), a rung below the bottom bar.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
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
            self.labels,
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
        // The rail's own surface. It had none until milestone 427 and showed whatever was
        // behind it; the reference gives it `surface` (`navigation_rail.dart:1202`).
        let fill = self
            .background
            .or(theme.widgets.nav_rail.background_color)
            .unwrap_or(theme.scheme.surface);
        scene.fill_rect(bounds, fill.fade(status.opacity));

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
    background: Option<Color>,
    labels: RailLabels,
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
            background: None,
            // A **bar** shows them all (`navigation_bar.dart:1388`), which is the other
            // half of the reference's answer: a bar owns the bottom of the screen and has
            // the room to say what its destinations are.
            labels: RailLabels::All,
            children: Vec::new(),
        }
    }

    /// When the destinations say what they are. [`RailLabels::All`] by default, as the
    /// reference's bar does.
    #[must_use]
    pub fn labels(mut self, labels: RailLabels) -> Self {
        self.labels = labels;
        self.rebuild();
        self
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

    /// The bar's surface. Unset, the theme's, then the scheme's `surface_container` —
    /// where the reference puts a navigation bar (`navigation_bar.dart:1440`), a rung
    /// above the rail.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
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
            self.labels,
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
        // The bar keeps the height a labelled destination needs as soon as **any** of
        // them is labelled: under [`RailLabels::Selected`] only one is at a time, and a
        // bar that resized as the selection moved would shift the page under it.
        let label =
            (self.labels != RailLabels::None).then(|| label_style(self.label_text_style, theme));
        let safe = crate::MediaQuery::of().padding;
        Style {
            height: Dimension::Length(item_height(BAR_HEIGHT, label.as_ref()) + safe.bottom),
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
        // The bar's own surface, a rung above the rail's: it stands on the page rather
        // than beside it (`navigation_bar.dart:1440`).
        let fill = self
            .background
            .or(theme.widgets.nav_rail.bar_background_color)
            .unwrap_or(theme.scheme.surface_container);
        scene.fill_rect(bounds, fill.fade(status.opacity));

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

    /// The **surface each of the two navigation widgets stands on**.
    ///
    /// Neither painted one until milestone 427: they drew a hairline and let whatever was
    /// behind them show through, so a bar sitting on a page was the page with a line above
    /// it. The reference gives the rail `surface` (`navigation_rail.dart:1202`) and the bar
    /// `surface_container` (`navigation_bar.dart:1440`) — a rung apart, because a bar
    /// stands *on* the page and a rail stands beside it.
    #[test]
    fn a_bar_and_a_rail_each_paint_the_rung_they_stand_on() {
        let theme = Theme::default();
        let bar = BottomBar::new(0, Msg::Go).item("H", "Home");
        let rail = NavigationRail::new(0, Msg::Go).item("H", "Home");
        let bar_box = Rect::new(0.0, 0.0, 320.0, BAR_HEIGHT);
        let rail_box = Rect::new(0.0, 0.0, RAIL_WIDTH, 600.0);

        assert_eq!(
            surface_of(&bar, bar_box, &theme),
            Some(theme.scheme.surface_container),
            "a bar stands on the page"
        );
        assert_eq!(
            surface_of(&rail, rail_box, &theme),
            Some(theme.scheme.surface),
            "a rail stands beside it"
        );
        assert_ne!(
            theme.scheme.surface_container, theme.scheme.surface,
            "the two rungs have to be tellable apart for the assertions above to mean \
             anything"
        );
    }

    /// The caller outranks the theme and the theme outranks the rung — both surfaces.
    #[test]
    fn the_caller_and_the_theme_outrank_the_rung() {
        let mut theme = Theme::default();
        theme.widgets.nav_rail.background_color = Some(Color::rgb8(1, 2, 3));
        theme.widgets.nav_rail.bar_background_color = Some(Color::rgb8(4, 5, 6));
        let bar_box = Rect::new(0.0, 0.0, 320.0, BAR_HEIGHT);
        let rail_box = Rect::new(0.0, 0.0, RAIL_WIDTH, 600.0);

        let bar = BottomBar::new(0, Msg::Go).item("H", "Home");
        let rail = NavigationRail::new(0, Msg::Go).item("H", "Home");
        assert_eq!(
            surface_of(&bar, bar_box, &theme),
            Some(Color::rgb8(4, 5, 6))
        );
        assert_eq!(
            surface_of(&rail, rail_box, &theme),
            Some(Color::rgb8(1, 2, 3))
        );

        let told = Color::rgb8(7, 8, 9);
        let bar = BottomBar::new(0, Msg::Go)
            .item("H", "Home")
            .background(told);
        let rail = NavigationRail::new(0, Msg::Go)
            .item("H", "Home")
            .background(told);
        assert_eq!(surface_of(&bar, bar_box, &theme), Some(told));
        assert_eq!(surface_of(&rail, rail_box, &theme), Some(told));
    }

    /// The colour of the first rect covering the whole box — the widget's own surface,
    /// drawn before the hairline that sits on one edge of it.
    fn surface_of(widget: &dyn Widget<Msg>, bounds: Rect, theme: &Theme) -> Option<Color> {
        let mut scene = Scene::new();
        widget.paint(bounds, Status::default(), theme, &mut scene);
        scene.primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Rect { rect, color, .. }
                if rect.width == bounds.width && rect.height == bounds.height =>
            {
                Some(*color)
            }
            _ => None,
        })
    }

    /// One destination, painted.
    fn destination(rail: bool, selected: bool, badge: Option<u32>, theme: &Theme) -> Scene {
        labelled(rail, selected, badge, true, theme)
    }

    /// The same, saying whether the destination shows its label.
    fn labelled(
        rail: bool,
        selected: bool,
        badge: Option<u32>,
        show_label: bool,
        theme: &Theme,
    ) -> Scene {
        let item = NavItem::<Msg> {
            icon: "H".into(),
            label: "Home".into(),
            selected,
            badge,
            rail,
            show_label,
            label_text_style: None,
            badge_text_style: None,
            message: Msg::Go(0),
        };
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &item,
            Rect::new(0.0, 0.0, RAIL_WIDTH, ITEM_HEIGHT),
            Status {
                opacity: 1.0,
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        scene
    }

    fn rects(scene: &Scene) -> Vec<Color> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    fn texts(scene: &Scene) -> Vec<Color> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    /// A destination's colours, each against the role the reference names.
    ///
    /// The indicator is the one that mattered most: it was `primary` at 16 %, which was
    /// the wrong **role** and the wrong **kind** of colour at once — a translucent fill
    /// blends in linear light here, so 16 % never painted at 16 %.
    #[test]
    fn a_destination_takes_the_roles_the_reference_names() {
        let theme = Theme::default();

        let on = destination(false, true, None, &theme);
        assert_eq!(
            rects(&on).first().copied(),
            Some(theme.scheme.secondary_container),
            "the indicator is an opaque container (`navigation_bar.dart:1463`)"
        );
        assert_eq!(
            texts(&on),
            vec![theme.scheme.on_secondary_container, theme.scheme.on_surface],
            "the glyph is the indicator's content, the label is the surface's"
        );
        assert_ne!(
            theme.scheme.on_secondary_container, theme.scheme.on_surface,
            "the two have to differ for that split to be worth making"
        );

        let off = destination(false, false, None, &theme);
        assert!(rects(&off).is_empty(), "nothing behind an unselected one");
        assert_eq!(
            texts(&off),
            vec![
                theme.scheme.on_surface_variant,
                theme.scheme.on_surface_variant
            ]
        );
    }

    /// The one question the reference answers differently for the two widgets: an
    /// unselected label is `on_surface` on a rail (`navigation_rail.dart:1251`) and
    /// `on_surface_variant` on a bar (`navigation_bar.dart:1477`).
    #[test]
    fn a_rail_and_a_bar_part_company_on_one_colour() {
        let theme = Theme::default();
        let rail = texts(&destination(true, false, None, &theme));
        let bar = texts(&destination(false, false, None, &theme));
        assert_eq!(rail[1], theme.scheme.on_surface);
        assert_eq!(bar[1], theme.scheme.on_surface_variant);
        assert_eq!(rail[0], bar[0], "and agree on the glyph");
    }

    /// **One badge, one theme.** The rail drew its own red on the reasoning that an alert
    /// dot reads as red whatever the theme; the [`Badge`](crate::Badge) widget beside it
    /// already answered the same question from the scheme. Two badges in one framework
    /// painting different reds was the part that was actually wrong.
    #[test]
    fn the_rail_s_badge_is_the_badge_widget_s_badge() {
        let mut theme = Theme::default();
        let scene = destination(false, false, Some(3), &theme);
        assert_eq!(rects(&scene).first().copied(), Some(theme.scheme.error));
        assert_eq!(
            texts(&scene).last().copied(),
            Some(theme.scheme.on_error),
            "the count is what is legible on it"
        );

        let told = Color::rgb8(1, 2, 3);
        theme.widgets.badge.background_color = Some(told);
        assert_eq!(
            rects(&destination(false, false, Some(3), &theme))
                .first()
                .copied(),
            Some(told),
            "and recolouring badges recolours this one too"
        );
    }

    /// Which destinations name themselves, under each of the three modes.
    fn labelled_indices(labels: RailLabels, count: usize, selected: usize) -> Vec<usize> {
        (0..count).filter(|&i| labels.shows(i, selected)).collect()
    }

    /// The three modes, and the two **different defaults** the reference gives the two
    /// widgets (`navigation_rail.dart:1238` against `navigation_bar.dart:1388`).
    ///
    /// The asymmetry is the part worth holding onto: a rail stands beside a page it does
    /// not own and glyphs alone keep it narrow, a bar owns the bottom of the screen and
    /// has the room to say what its destinations are.
    #[test]
    fn a_rail_and_a_bar_start_from_opposite_defaults() {
        assert_eq!(
            labelled_indices(RailLabels::None, 3, 1),
            Vec::<usize>::new()
        );
        assert_eq!(labelled_indices(RailLabels::Selected, 3, 1), vec![1]);
        assert_eq!(labelled_indices(RailLabels::All, 3, 1), vec![0, 1, 2]);

        let rail = NavigationRail::new(0, Msg::Go).item("H", "Home");
        let bar = BottomBar::new(0, Msg::Go).item("H", "Home");
        assert_eq!(
            rail.labels,
            RailLabels::None,
            "a rail says nothing until asked"
        );
        assert_eq!(bar.labels, RailLabels::All, "a bar says everything");
    }

    /// A destination with no label paints no label — and centres the glyph on its own
    /// rather than leaving it where it sat when there was something below it.
    #[test]
    fn a_silent_destination_centres_its_glyph() {
        let theme = Theme::default();
        let with = labelled(true, false, None, true, &theme);
        let without = labelled(true, false, None, false, &theme);

        let glyph_y = |scene: &Scene| {
            scene.primitives().iter().find_map(|p| match p {
                frus_core::Primitive::Text { position, .. } => Some(position.y),
                _ => None,
            })
        };
        assert_eq!(
            without
                .primitives()
                .iter()
                .filter(|p| matches!(p, frus_core::Primitive::Text { .. }))
                .count(),
            1,
            "the glyph and nothing else"
        );
        assert_eq!(texts(&with).len(), 2, "the glyph and its label");
        assert!(
            glyph_y(&without) > glyph_y(&with),
            "and the glyph moved down into the room the label was using"
        );
    }

    /// The row keeps its height when the label goes.
    ///
    /// Under [`RailLabels::Selected`] exactly one destination is labelled at a time, so a
    /// row that shrank without one would move every destination in the rail the first
    /// time the selection changed.
    #[test]
    fn a_row_does_not_shrink_when_its_label_goes() {
        let theme = Theme::default();
        let height = |show_label: bool| {
            let item = NavItem::<Msg> {
                icon: "H".into(),
                label: "Home".into(),
                selected: false,
                badge: None,
                rail: true,
                show_label,
                label_text_style: None,
                badge_text_style: None,
                message: Msg::Go(0),
            };
            match Widget::<Msg>::style_themed(&item, &theme).height {
                Dimension::Length(h) => h,
                other => panic!("a rail row names its height, got {other:?}"),
            }
        };
        assert_eq!(height(true), height(false));
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
