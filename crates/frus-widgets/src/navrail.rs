//! [`NavigationRail`] and [`BottomBar`]: the two presentations of a single-selection
//! **main navigation**. Same API (`new(selected, on_select).item(icon, label)`);
//! [`crate::NavScaffold`] picks one or the other by size. The "icon" is a text
//! glyph (the framework has no icon font): an emoji, or a Unicode character.
//!
//! The rail has three things a column of its own height can have and a single row
//! cannot: an [extended](NavigationRail::extended) form, a slot
//! [above](NavigationRail::leading) and [below](NavigationRail::trailing) the
//! destinations, and a say in [where those destinations
//! sit](NavigationRail::group_alignment) between its two ends.

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Insets, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::disabled::disabled_content;
use crate::flex::Flex;
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
/// Width of an **extended** rail (`navigation_rail.dart:1241`), where the labels stand
/// beside the glyphs instead of under them.
pub(crate) const EXTENDED_RAIL_WIDTH: f32 = 256.0;
/// Height of a bottom navigation bar, in logical pixels.
pub(crate) const BAR_HEIGHT: f32 = 60.0;
const ITEM_HEIGHT: f32 = 58.0;
/// The destinations' glyphs, at the reference's size (`navigation_bar.dart:1452`).
const ICON_SIZE: f32 = 24.0;
/// The air between a glyph and its label (`navigation_bar.dart:1483`).
const LABEL_GAP: f32 = 4.0;
/// The air between two destinations.
const DESTINATION_GAP: f32 = 4.0;
/// The air a slot keeps from the destinations (`navigation_rail.dart:1179`).
const SLOT_GAP: f32 = 8.0;

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
///
/// `beside` is an [extended](NavigationRail::extended) rail's row, where the label stands
/// next to the glyph rather than under it: the row is then as tall as the taller of the
/// two, not as tall as both.
fn item_height(floor: f32, label: Option<&ResolvedTextStyle>, beside: bool) -> f32 {
    let icon = frus_text::line_height(ICON_SIZE);
    let content = match label {
        Some(label) if beside => icon.max(label.line_height()),
        Some(label) => icon + LABEL_GAP + label.line_height(),
        None => icon,
    } + 8.0;
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
    /// Whether the label stands **beside** the glyph rather than under it. See
    /// [`NavigationRail::extended`].
    extended: bool,
    /// This destination cannot be reached. See [`NavigationRail::disabled`].
    disabled: bool,
    /// This destination's own indicator colour, over the theme's.
    indicator_color: Option<Color>,
    /// The surface the destination **stands on**, when the widget carrying it was told
    /// one. A state layer is a lerp from the ground toward the ink, so it has to be the
    /// ground the caller actually painted.
    ground: Option<Color>,
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
                width: Dimension::Length(if self.extended {
                    EXTENDED_RAIL_WIDTH
                } else {
                    RAIL_WIDTH
                }),
                height: Dimension::Length(item_height(ITEM_HEIGHT, label.as_ref(), self.extended)),
                ..Default::default()
            }
        } else {
            // In a bar, the items share the width equally.
            Style {
                flex_grow: 1.0,
                height: Dimension::Length(item_height(BAR_HEIGHT, label.as_ref(), false)),
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
        // **The column the glyph lives in**, which is not the row on an extended rail.
        // There the label stands beside the glyph (`navigation_rail.dart:796`) and the
        // glyph keeps the column it would have had unextended (`:753`) — so extending a
        // rail does not move its destinations sideways, and the indicator stays around
        // the glyph alone (`:756`) rather than growing to swallow the label.
        let col = if self.extended {
            RAIL_WIDTH.min(bounds.width)
        } else {
            bounds.width
        };
        // With no label the glyph centres on its own, rather than staying where it sat
        // when there was one below it.
        let stacked_h = icon_m.height + label_m.as_ref().map_or(0.0, |m| gap + m.height);
        let top = bounds.y
            + ((bounds.height
                - if self.extended {
                    icon_m.height
                } else {
                    stacked_h
                })
                * 0.5)
                .max(0.0);

        // Background pill: solid when selected, discreet on hover.
        let pill_w = icon_m.width + 28.0;
        let pill_h = icon_m.height + 8.0;
        let pill = Rect::new(bounds.x + (col - pill_w) * 0.5, top - 4.0, pill_w, pill_h);
        // **The ground this destination stands on**, which is what a state layer is
        // measured from: the indicator when it has one, else the surface the rail or the
        // bar painted under it — the caller's, the theme's, or the rung each of the two
        // takes by default (milestone 427).
        let ground = self.ground.or(if self.rail {
            theme.widgets.nav_rail.background_color
        } else {
            theme.widgets.nav_rail.bar_background_color
        });
        let ground = ground.unwrap_or(if self.rail {
            theme.scheme.surface
        } else {
            theme.scheme.surface_container
        });
        // The indicator is a **container**, not a wash: the reference fills it with an
        // opaque `secondaryContainer` (`navigation_bar.dart:1463`,
        // `navigation_rail.dart:1272`) where this painted `primary` at 16 %. A tint was
        // the wrong role and the wrong kind of colour at once — a translucent fill blends
        // in linear light here, so 16 % does not paint at 16 %, which is the trap
        // milestone 329 resolved for the disabled tokens.
        //
        // The destination's own colour outranks the theme's (`navigation_rail.dart:1144`),
        // which is how one entry in a list marks itself out from the rest.
        let indicator = self
            .indicator_color
            .or(t.indicator_color)
            .unwrap_or(theme.scheme.secondary_container);
        let base = if self.selected { indicator } else { ground };
        // **And the state layer over it, resolved opaquely.** This used to be `muted` at
        // 12 % handed to the GPU as an alpha, which is three mistakes in one line: the
        // wrong kind of colour (a translucent overlay blends in linear light here, so
        // 12 % paints like a third), the wrong role (the reference's ink for this widget
        // is `primary`, `navigation_rail.dart:946`), and the wrong number (12 % is the
        // *splash*'s, `:943`; the hover's is smaller). [`Theme::state_layer`] is the one
        // rule the rest of the framework already asks — a lerp from the ground toward the
        // ink, in the space the tokens were written in — and it answers hover, focus and
        // press at once, where this answered only hover.
        //
        // Nothing lights on a destination that cannot be reached: a state layer is the
        // promise of an interaction, and there is none here (milestone 436).
        let fill = if self.disabled {
            base
        } else {
            theme.state_layer(base, theme.scheme.primary, &status)
        };
        if self.selected || fill != base {
            scene.draw_rect(pill, fill.fade(o), pill_h * 0.5, 0.0, Color::TRANSPARENT);
        }

        // The glyph is drawn **on** the indicator and the label below it, so the two do
        // not take the same colour when selected: the glyph is the indicator's content
        // (`navigation_bar.dart:1456`) and the label is the surface's (`:1476`).
        let icon_color = if self.disabled {
            // **One rule for both halves** (`navigation_rail.dart:717`, `:723`):
            // `on_surface` at 38 %, which this framework resolves opaque rather than
            // handing the GPU an alpha — see [`crate::disabled_content`].
            disabled_content(theme)
        } else if self.selected {
            t.selected_icon_color
                .unwrap_or(theme.scheme.on_secondary_container)
        } else {
            t.unselected_icon_color
                .unwrap_or(theme.scheme.on_surface_variant)
        };
        let label_color = if self.disabled {
            disabled_content(theme)
        } else if self.selected {
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
            Point::new(bounds.x + (col - icon_m.width) * 0.5, top),
            self.icon.clone(),
            &icon_style,
            icon_color.fade(o),
        );
        if let Some(label_m) = &label_m {
            // Beside the glyph's column on an extended rail, centred under it otherwise.
            let position = if self.extended {
                Point::new(
                    bounds.x + col,
                    bounds.y + ((bounds.height - label_m.height) * 0.5).max(0.0),
                )
            } else {
                Point::new(
                    bounds.x + (col - label_m.width) * 0.5,
                    top + icon_m.height + gap,
                )
            };
            scene.text(position, self.label.clone(), &label_s, label_color.fade(o));
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
            let icon_right = bounds.x + (col + icon_m.width) * 0.5;
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

    /// A destination that cannot be reached emits nothing (`navigation_rail.dart:957`,
    /// where the reference passes the ink well a null `onTap`).
    fn on_click(&self) -> Option<Msg> {
        (!self.disabled).then(|| self.message.clone())
    }

    /// And the keyboard skips it, as it skips a disabled button.
    fn focusable(&self) -> bool {
        !self.disabled
    }
}

/// **A declared destination**: everything the caller said about one entry in the list.
///
/// It was a `(glyph, label, badge)` tuple until milestone 436, which is exactly as far as a
/// tuple goes. The two shells declare their destinations with this too, so a property added
/// here is a property they can both forward.
#[derive(Clone, Default)]
pub(crate) struct Destination {
    /// The glyph standing in for an icon.
    pub icon: String,
    /// What the destination is called.
    pub label: String,
    /// The glyph shown **while selected**, when it differs from the resting one
    /// (`navigation_rail.dart:1132`).
    pub selected_icon: Option<String>,
    /// Notification count. `None` or `0` = nothing.
    pub badge: Option<u32>,
    /// This destination cannot be reached (`navigation_rail.dart:1161`).
    pub disabled: bool,
    /// This destination's own indicator colour, over the theme's
    /// (`navigation_rail.dart:1144`).
    pub indicator_color: Option<Color>,
}

impl Destination {
    /// A destination with nothing said about it but its glyph and its name.
    pub(crate) fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            ..Default::default()
        }
    }

    /// The glyph to draw in this state: the selected one when there is one and the
    /// destination is selected, the resting one otherwise.
    fn glyph(&self, selected: bool) -> &str {
        match &self.selected_icon {
            Some(icon) if selected => icon,
            _ => &self.icon,
        }
    }
}

/// **How a list of destinations is being presented** — everything an item needs that is
/// the same for every item in the list.
///
/// One argument rather than six: the two widgets pass the same thing, and a function taking
/// four booleans and two colours in a row is a function whose call sites are read by
/// counting commas.
#[derive(Copy, Clone)]
struct Presentation {
    /// `true` = a rail's items (fixed width); `false` = a bar's (they share the row).
    rail: bool,
    labels: RailLabels,
    extended: bool,
    /// The surface the destinations stand on, when the widget carrying them was told one.
    ground: Option<Color>,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
}

/// Builds the navigation items from the declared destinations.
fn build_items<Msg: Clone + 'static>(
    items: &[Destination],
    selected: usize,
    on_select: &dyn Fn(usize) -> Msg,
    how: Presentation,
) -> Vec<Box<dyn Widget<Msg>>> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected;
            Box::new(NavItem {
                icon: item.glyph(is_selected).to_string(),
                label: item.label.clone(),
                selected: is_selected,
                badge: item.badge,
                rail: how.rail,
                // An extended rail labels **every** destination
                // (`navigation_rail.dart:219`): the label has its own room there, so
                // there is nothing for a mode to trade away.
                show_label: how.extended || how.labels.shows(i, selected),
                extended: how.extended,
                disabled: item.disabled,
                indicator_color: item.indicator_color,
                ground: how.ground,
                label_text_style: how.label_text_style,
                badge_text_style: how.badge_text_style,
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
    extended: bool,
    group_alignment: f32,
    leading: RefCell<Option<Box<dyn Widget<Msg>>>>,
    trailing: RefCell<Option<Box<dyn Widget<Msg>>>>,
    leading_at_top: bool,
    trailing_at_bottom: bool,
    /// The assembled subtree — see [`Self::assemble`]. Built on the **first read**
    /// rather than as the builders run, because assembling it consumes the slots and a
    /// builder can still arrive after the one that set them.
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
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
            extended: false,
            // Against the top (`navigation_rail.dart:1237`).
            group_alignment: -1.0,
            leading: RefCell::new(None),
            trailing: RefCell::new(None),
            // **The reference's asymmetry** (`navigation_rail.dart:112`, `:113`): the
            // leading slot is chrome at the top of the rail, the trailing one is the tail
            // of the list of destinations and travels with it.
            leading_at_top: true,
            trailing_at_bottom: false,
            built: OnceCell::new(),
        }
    }

    /// When the destinations say what they are. [`RailLabels::None`] by default, as the
    /// reference's rail does.
    ///
    /// Silent on an [extended](Self::extended) rail, which labels every destination. The
    /// two are different label **layouts** rather than modes of one another, and the
    /// reference forbids the combination outright (`navigation_rail.dart:121`).
    #[must_use]
    pub fn labels(mut self, labels: RailLabels) -> Self {
        self.labels = labels;
        self.rebuild();
        self
    }

    /// **The wide form**: 256 across instead of 80, with every label beside its glyph
    /// instead of under it (`navigation_rail.dart:131`).
    ///
    /// The glyphs keep the 80-pixel column they had, so extending a rail widens it and
    /// moves nothing: the destinations stay on the line they were on and the label opens
    /// out to the side of them. A rail with room for words is what a desktop window has
    /// and a tablet in portrait does not, which is why this is a property and not a size
    /// class — the caller knows which it is building.
    #[must_use]
    pub fn extended(mut self, extended: bool) -> Self {
        self.extended = extended;
        self.rebuild();
        self
    }

    /// **Where the destinations sit** between the rail's top and bottom: `-1.0` against
    /// the top (the default), `0.0` centred, `1.0` against the bottom — and every value
    /// in between (`navigation_rail.dart:205`).
    ///
    /// Continuous rather than three names, as the reference's is: a rail whose
    /// destinations sit a third of the way down is a thing an application asks for, and a
    /// three-valued enum would have to be replaced the first time one did.
    ///
    /// It moves the **group**, which is the destinations plus whichever slot is not
    /// pinned. See [`Self::leading_at_top`].
    #[must_use]
    pub fn group_alignment(mut self, alignment: f32) -> Self {
        self.group_alignment = alignment.clamp(-1.0, 1.0);
        self.rebuild();
        self
    }

    /// The slot **above** the destinations, where an application puts a floating action
    /// button or a menu button (`navigation_rail.dart:145`).
    #[must_use]
    pub fn leading(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.leading_boxed(Box::new(widget))
    }

    /// [`Self::leading`], for a slot that is already boxed.
    #[must_use]
    pub fn leading_boxed(mut self, widget: Box<dyn Widget<Msg>>) -> Self {
        *self.leading.borrow_mut() = Some(widget);
        self.rebuild();
        self
    }

    /// The slot **below** the destinations (`navigation_rail.dart:156`) — an account
    /// switcher, a settings button, the end of the list rather than chrome above it.
    #[must_use]
    pub fn trailing(self, widget: impl Widget<Msg> + 'static) -> Self {
        self.trailing_boxed(Box::new(widget))
    }

    /// [`Self::trailing`], for a slot that is already boxed.
    #[must_use]
    pub fn trailing_boxed(mut self, widget: Box<dyn Widget<Msg>>) -> Self {
        *self.trailing.borrow_mut() = Some(widget);
        self.rebuild();
        self
    }

    /// Whether the leading slot is **pinned to the top** instead of travelling with the
    /// destinations. `true` by default (`navigation_rail.dart:112`), which is why
    /// [`Self::group_alignment`] leaves it where it is.
    #[must_use]
    pub fn leading_at_top(mut self, pinned: bool) -> Self {
        self.leading_at_top = pinned;
        self.rebuild();
        self
    }

    /// Whether the trailing slot is **pinned to the bottom**. `false` by default
    /// (`navigation_rail.dart:113`), the other half of the reference's asymmetry: a
    /// leading slot is chrome at the top of the rail, a trailing one is the tail of the
    /// list of destinations and moves when the list does.
    #[must_use]
    pub fn trailing_at_bottom(mut self, pinned: bool) -> Self {
        self.trailing_at_bottom = pinned;
        self.rebuild();
        self
    }

    /// Adds a destination (glyph + label).
    pub fn item(mut self, icon: impl Into<String>, label: impl Into<String>) -> Self {
        self.items.push(Destination::new(icon, label));
        self.rebuild();
        self
    }

    /// Adds a destination declared **in full** — what a shell hands over, having taken
    /// the caller's decorations itself.
    pub(crate) fn destination(mut self, destination: Destination) -> Self {
        self.items.push(destination);
        self.rebuild();
        self
    }

    /// Adds a notification count to the **last** destination.
    pub fn badge(self, count: u32) -> Self {
        self.decorate(|last| last.badge = Some(count))
    }

    /// The glyph the **last** destination shows while it is selected, where that differs
    /// from its resting one (`navigation_rail.dart:1132`).
    ///
    /// The reference pairs a stroked icon with its filled version, which is how a
    /// selected destination reads as selected without leaning on colour alone.
    #[must_use]
    pub fn selected_icon(self, icon: impl Into<String>) -> Self {
        let icon = icon.into();
        self.decorate(move |last| last.selected_icon = Some(icon))
    }

    /// Marks the **last** destination inaccessible (`navigation_rail.dart:1161`): its
    /// glyph and its label take the disabled ink, nothing lights under the pointer, it
    /// emits no message, and the keyboard steps over it.
    #[must_use]
    pub fn disabled(self) -> Self {
        self.decorate(|last| last.disabled = true)
    }

    /// The **last** destination's own indicator colour, over the theme's
    /// (`navigation_rail.dart:1144`) — how one entry marks itself out from the rest.
    #[must_use]
    pub fn indicator_color(self, color: Color) -> Self {
        self.decorate(move |last| last.indicator_color = Some(color))
    }

    /// Applies `f` to the destination just added. Silent when there is none, which is the
    /// shape `badge` has always had.
    fn decorate(mut self, f: impl FnOnce(&mut Destination)) -> Self {
        if let Some(last) = self.items.last_mut() {
            f(last);
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
        // The destinations read it too, since milestone 437: a state layer is a lerp from
        // the ground toward the ink, and this is the ground.
        self.rebuild();
        self
    }

    /// Throws the assembled subtree away, so that the next read of it is built from
    /// everything the builders have said by then and they stay order-independent.
    fn rebuild(&mut self) {
        self.built.take();
    }

    /// **How wide the rail asks to be**: [`RAIL_WIDTH`], or [`EXTENDED_RAIL_WIDTH`] when
    /// the labels have moved out beside the glyphs. Without the leading intrusion, which
    /// the rail adds on top of this and consumes on its parent's behalf.
    ///
    /// A shell that puts something else beside a rail has to know how much of the window
    /// the rail took. Asking the rail is the only answer that stays right: reading the
    /// constant is right until the caller extends it, and then it is 176 pixels wrong.
    pub(crate) fn declared_width(&self) -> f32 {
        if self.extended {
            EXTENDED_RAIL_WIDTH
        } else {
            RAIL_WIDTH
        }
    }

    /// The rail's subtree, in the reference's shape (`navigation_rail.dart:559`):
    ///
    /// ```text
    /// leading      if it is pinned to the top
    /// spacer       (1 + alignment) / 2
    /// group        an unpinned leading slot, the destinations, an unpinned trailing one
    /// spacer       (1 - alignment) / 2
    /// trailing     if it is pinned to the bottom
    /// ```
    ///
    /// **The two spacers are what makes the alignment continuous.** A `justify` offers
    /// three positions; two flexible boxes whose grow factors add up to one offer every
    /// position between them, and the ends of the range are the same three. The
    /// destinations' own spacing belongs to the group they are in rather than to the
    /// rail, because a gap on the rail would also push the spacers away from the group
    /// and move the top-aligned default four pixels down.
    fn assemble(&self) -> Vec<Box<dyn Widget<Msg>>> {
        // Taking the slots is why this runs once: a `Box<dyn Widget>` cannot be cloned,
        // so assembling **consumes** them. A widget tree is rebuilt from `view` rather
        // than mutated, so once per instance and once per frame are the same thing — the
        // reasoning [`Widget::build_themed`] is written on.
        let leading = self.leading.borrow_mut().take();
        let trailing = self.trailing.borrow_mut().take();

        let mut out: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        let mut group = Flex::<Msg>::column()
            .align(Align::Center)
            .gap(DESTINATION_GAP);

        if let Some(leading) = leading {
            let slot = Flex::<Msg>::column()
                .align(Align::Center)
                .padding_each(0.0, 0.0, SLOT_GAP, 0.0)
                .child_boxed(leading);
            if self.leading_at_top {
                out.push(Box::new(slot));
            } else {
                group = group.child(slot);
            }
        }
        for item in build_items(
            &self.items,
            self.selected,
            &*self.on_select,
            Presentation {
                rail: true,
                labels: self.labels,
                extended: self.extended,
                ground: self.background,
                label_text_style: self.label_text_style,
                badge_text_style: self.badge_text_style,
            },
        ) {
            group = group.child_boxed(item);
        }
        let mut pinned = None;
        match trailing {
            Some(trailing) if self.trailing_at_bottom => pinned = Some(trailing),
            Some(trailing) => group = group.child_boxed(trailing),
            None => {}
        }

        let a = self.group_alignment;
        out.push(Box::new(Flex::<Msg>::column().flex((1.0 + a) * 0.5)));
        out.push(Box::new(group));
        out.push(Box::new(Flex::<Msg>::column().flex((1.0 - a) * 0.5)));
        if let Some(trailing) = pinned {
            out.push(trailing);
        }
        out
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for NavigationRail<Msg> {
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
            width: Dimension::Length(self.declared_width() + safe.left),
            flex_direction: FlexDirection::Column,
            align: Align::Center,
            padding: Insets::new(8.0 + safe.top, 0.0, 8.0 + safe.bottom, safe.left),
            // No gap: see [`Self::assemble`]. The destinations are spaced inside the
            // group, so that the boxes placing that group are not spaced away from it.
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble())
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
        self.items.push(Destination::new(icon, label));
        self.rebuild();
        self
    }

    /// Adds a destination declared **in full** — what a shell hands over, having taken
    /// the caller's decorations itself.
    pub(crate) fn destination(mut self, destination: Destination) -> Self {
        self.items.push(destination);
        self.rebuild();
        self
    }

    /// Adds a notification count to the **last** destination.
    pub fn badge(self, count: u32) -> Self {
        self.decorate(|last| last.badge = Some(count))
    }

    /// The glyph the **last** destination shows while it is selected, where that differs
    /// from its resting one (`navigation_rail.dart:1132`).
    ///
    /// The reference pairs a stroked icon with its filled version, which is how a
    /// selected destination reads as selected without leaning on colour alone.
    #[must_use]
    pub fn selected_icon(self, icon: impl Into<String>) -> Self {
        let icon = icon.into();
        self.decorate(move |last| last.selected_icon = Some(icon))
    }

    /// Marks the **last** destination inaccessible (`navigation_rail.dart:1161`): its
    /// glyph and its label take the disabled ink, nothing lights under the pointer, it
    /// emits no message, and the keyboard steps over it.
    #[must_use]
    pub fn disabled(self) -> Self {
        self.decorate(|last| last.disabled = true)
    }

    /// The **last** destination's own indicator colour, over the theme's
    /// (`navigation_rail.dart:1144`) — how one entry marks itself out from the rest.
    #[must_use]
    pub fn indicator_color(self, color: Color) -> Self {
        self.decorate(move |last| last.indicator_color = Some(color))
    }

    /// Applies `f` to the destination just added. Silent when there is none, which is the
    /// shape `badge` has always had.
    fn decorate(mut self, f: impl FnOnce(&mut Destination)) -> Self {
        if let Some(last) = self.items.last_mut() {
            f(last);
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
        // The destinations read it too, since milestone 437: a state layer is a lerp from
        // the ground toward the ink, and this is the ground.
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
            Presentation {
                rail: false,
                labels: self.labels,
                // A bar is one row: it has no extended form, and no room for one.
                extended: false,
                ground: self.background,
                label_text_style: self.label_text_style,
                badge_text_style: self.badge_text_style,
            },
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
            height: Dimension::Length(item_height(BAR_HEIGHT, label.as_ref(), false) + safe.bottom),
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
        painted(rail, selected, badge, show_label, false, theme)
    }

    /// The same again, saying whether the rail it stands in is **extended** — which
    /// widens its box and moves its label beside the glyph.
    fn painted(
        rail: bool,
        selected: bool,
        badge: Option<u32>,
        show_label: bool,
        extended: bool,
        theme: &Theme,
    ) -> Scene {
        let item = row(rail, show_label, extended, None);
        let width = if extended {
            EXTENDED_RAIL_WIDTH
        } else {
            RAIL_WIDTH
        };
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &NavItem {
                selected,
                badge,
                ..item
            },
            Rect::new(0.0, 0.0, width, ITEM_HEIGHT),
            Status {
                opacity: 1.0,
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        scene
    }

    /// One destination, unpainted — the thing whose style the layout reads.
    fn row(
        rail: bool,
        show_label: bool,
        extended: bool,
        label_style: Option<TextStyle>,
    ) -> NavItem<Msg> {
        NavItem::<Msg> {
            icon: "H".into(),
            label: "Home".into(),
            selected: false,
            badge: None,
            rail,
            show_label,
            extended,
            disabled: false,
            indicator_color: None,
            ground: None,
            label_text_style: label_style,
            badge_text_style: None,
            message: Msg::Go(0),
        }
    }

    /// Every text primitive in a scene, with where it was drawn.
    fn placed(scene: &Scene) -> Vec<(String, Point)> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, position, .. } => {
                    Some((text.clone(), *position))
                }
                _ => None,
            })
            .collect()
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
            let item = row(true, show_label, false, None);
            match Widget::<Msg>::style_themed(&item, &theme).height {
                Dimension::Length(h) => h,
                other => panic!("a rail row names its height, got {other:?}"),
            }
        };
        assert_eq!(height(true), height(false));
    }

    /// The one rectangle a destination paints under itself, if it paints one.
    fn layer(rail: &NavigationRail<Msg>, status: Status, theme: &Theme) -> Option<Color> {
        rects(&paint_destination(rail, 0, status, theme))
            .first()
            .copied()
    }

    /// **A state layer is opaque, and it is a lerp from the ground** (milestone 437).
    ///
    /// This painted `muted` at 12 % handed to the GPU as an alpha, which is three
    /// mistakes in one line: the wrong *kind* of colour, since a translucent overlay
    /// blends in linear light here and 12 % paints like a third (milestone 329); the
    /// wrong *role*, the reference's ink for this widget being `primary`
    /// (`navigation_rail.dart:946`); and the wrong *number*, 12 % being the splash's
    /// (`:943`) rather than the hover's.
    #[test]
    fn a_destination_s_state_layer_is_opaque_and_starts_from_the_ground() {
        let theme = Theme::default();
        let rail = NavigationRail::new(9, Msg::Go).item("H", "Home");
        let painted = layer(&rail, hovered(), &theme).expect("a hovered destination lights");
        assert_eq!(painted.a, 1.0, "resolved here, not handed over as an alpha");
        assert_eq!(
            painted,
            theme.state_layer(theme.scheme.surface, theme.scheme.primary, &hovered()),
            "the framework's one state rule, over the rail's own rung"
        );
        assert_eq!(
            layer(&rail, live(), &theme),
            None,
            "and nothing at rest, which is what it has always drawn"
        );
    }

    /// It starts from the ground the widget **actually painted**: the two widgets stand on
    /// different rungs (milestone 427), and either can be told a third colour.
    #[test]
    fn the_layer_starts_from_the_ground_the_widget_painted() {
        let theme = Theme::default();
        let rail = NavigationRail::new(9, Msg::Go).item("H", "Home");
        let on_rail = layer(&rail, hovered(), &theme).expect("lit");

        let mut bar_scene = Scene::new();
        let bar = BottomBar::new(9, Msg::Go).item("H", "Home");
        Widget::<Msg>::children(&bar)[0].paint(
            Rect::new(0.0, 0.0, RAIL_WIDTH, BAR_HEIGHT),
            hovered(),
            &theme,
            &mut bar_scene,
        );
        let on_bar = rects(&bar_scene).first().copied().expect("lit");
        assert_ne!(
            on_rail, on_bar,
            "a bar stands a rung above a rail, so its layer starts higher"
        );

        let told = Color::rgb8(120, 20, 20);
        let painted = NavigationRail::new(9, Msg::Go)
            .item("H", "Home")
            .background(told);
        assert_eq!(
            layer(&painted, hovered(), &theme),
            Some(theme.state_layer(told, theme.scheme.primary, &hovered())),
            "and a rail told a colour is the colour its destinations stand on"
        );
    }

    /// **A selected destination lights too.** The old branch answered hover only when
    /// there was no indicator, so the one destination a pointer is most likely to be over
    /// was the one that never responded.
    #[test]
    fn a_selected_destination_lights_under_the_pointer_too() {
        let theme = Theme::default();
        let rail = NavigationRail::new(0, Msg::Go).item("H", "Home");
        let resting = layer(&rail, live(), &theme).expect("the indicator");
        let lit = layer(&rail, hovered(), &theme).expect("the indicator, plus the layer");
        assert_eq!(resting, theme.scheme.secondary_container);
        assert_ne!(lit, resting, "the indicator takes the layer over it");
        assert_eq!(
            lit,
            theme.state_layer(
                theme.scheme.secondary_container,
                theme.scheme.primary,
                &hovered()
            )
        );
    }

    /// And **focus and press** light it as well, which nothing did before: the old line
    /// read `hover_progress` alone, where the theme's rule answers all three states.
    #[test]
    fn focus_and_press_light_it_as_well() {
        let theme = Theme::default();
        let rail = NavigationRail::new(9, Msg::Go).item("H", "Home");
        let focused = Status {
            focus_progress: 1.0,
            ..live()
        };
        let pressed = Status {
            interaction: crate::interaction::Interaction::Pressed,
            ..live()
        };
        assert!(layer(&rail, focused, &theme).is_some(), "focus");
        assert!(layer(&rail, pressed, &theme).is_some(), "press");
        assert!(
            layer(&rail, live(), &theme).is_none(),
            "and still nothing at rest"
        );
    }

    /// Nothing lights on a destination that cannot be reached, in any of the three states:
    /// a state layer is the promise of an interaction, and there is none here.
    #[test]
    fn a_disabled_destination_lights_in_no_state_at_all() {
        let theme = Theme::default();
        let rail = NavigationRail::new(9, Msg::Go).item("H", "Home").disabled();
        for status in [
            hovered(),
            Status {
                focus_progress: 1.0,
                ..live()
            },
            Status {
                interaction: crate::interaction::Interaction::Pressed,
                ..live()
            },
        ] {
            assert_eq!(layer(&rail, status, &theme), None);
        }
    }

    /// **The wide form.** An extended rail puts each label beside its glyph rather than
    /// under it (`navigation_rail.dart:796`), and the glyph keeps the 80-pixel column it
    /// had (`:753`) — so extending a rail widens it and moves nothing.
    #[test]
    fn an_extended_rail_puts_the_label_beside_the_glyph() {
        let theme = Theme::default();
        let under = placed(&painted(true, false, None, true, false, &theme));
        let beside = placed(&painted(true, false, None, true, true, &theme));
        assert_eq!(under.len(), 2, "the glyph and its label");
        assert_eq!(beside.len(), 2);

        assert!(
            under[1].1.y > under[0].1.y + frus_text::line_height(ICON_SIZE),
            "unextended, the label is clear of the glyph's line"
        );
        assert!(
            beside[1].1.x >= RAIL_WIDTH,
            "extended, the label starts where the glyph's column ends, not at x = {}",
            beside[1].1.x
        );
        assert!(
            beside[1].1.y > beside[0].1.y
                && beside[1].1.y < beside[0].1.y + frus_text::line_height(ICON_SIZE),
            "extended, the two stand on one line: {} against {}",
            beside[1].1.y,
            beside[0].1.y
        );
        assert!(
            (beside[0].1.x - under[0].1.x).abs() < 0.01,
            "and the glyph kept its column: {} against {}",
            beside[0].1.x,
            under[0].1.x
        );
    }

    /// An extended row is **wider, not taller**: the label left the column, so the row is
    /// as tall as the taller of the two rather than as tall as both.
    ///
    /// Asked with a label big enough to beat the row's constant floor, which is what
    /// makes the two numbers differ at all.
    #[test]
    fn an_extended_row_is_wider_and_not_taller() {
        let theme = Theme::default();
        let big = Some(TextStyle {
            size: Some(40.0),
            ..Default::default()
        });
        let size = |extended: bool| {
            let style = Widget::<Msg>::style_themed(&row(true, true, extended, big), &theme);
            match (style.width, style.height) {
                (Dimension::Length(w), Dimension::Length(h)) => (w, h),
                other => panic!("a rail row names both of its sides, got {other:?}"),
            }
        };
        let (narrow, tall) = size(false);
        let (wide, short) = size(true);
        assert_eq!(narrow, RAIL_WIDTH);
        assert_eq!(wide, EXTENDED_RAIL_WIDTH);
        assert!(
            short < tall,
            "the label moved out of the column and the row did not follow it: {short} \
             against {tall}"
        );
    }

    /// And it labels **every** destination, whatever the rail's label mode says
    /// (`navigation_rail.dart:219`): the label has room of its own there, so there is
    /// nothing for a mode to trade away.
    #[test]
    fn an_extended_rail_labels_every_destination() {
        let theme = Theme::default();
        let rail = NavigationRail::new(0, Msg::Go)
            .item("H", "Home")
            .item("S", "Search")
            .labels(RailLabels::None)
            .extended(true);
        for destination in destinations(&rail) {
            let mut scene = Scene::new();
            destination.paint(
                Rect::new(0.0, 0.0, EXTENDED_RAIL_WIDTH, ITEM_HEIGHT),
                Status::default(),
                &theme,
                &mut scene,
            );
            assert_eq!(
                placed(&scene).len(),
                2,
                "an extended destination says what it is even under RailLabels::None"
            );
        }
    }

    /// The rail, in a window tall enough for its destinations to have somewhere to go.
    fn glyph_ys(rail: NavigationRail<Msg>) -> Vec<f32> {
        marks(rail)
            .into_iter()
            .filter(|(text, _)| text == "H" || text == "S")
            .map(|(_, y)| y)
            .collect()
    }

    /// Every glyph the rail painted, top to bottom, as `(text, y)`.
    fn marks(rail: NavigationRail<Msg>) -> Vec<(String, f32)> {
        const TALL: f32 = 600.0;
        let root = crate::Flex::row().width(200.0).height(TALL).child(rail);
        let ui = crate::build_ui(
            &root as &dyn Widget<Msg>,
            crate::Size::new(200.0, TALL),
            &crate::Runtime::default(),
            &Theme::default(),
        );
        let mut found: Vec<(String, f32)> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, position, .. } => {
                    Some((text.clone(), position.y))
                }
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| a.1.partial_cmp(&b.1).expect("no NaN in a laid-out rail"));
        found
    }

    /// **Where the destinations sit**, anywhere between the rail's two ends
    /// (`navigation_rail.dart:205`).
    ///
    /// Continuous, which is the part a three-valued enum could not have done: `-0.5`
    /// lands between the top and the middle rather than at one of them.
    #[test]
    fn the_group_travels_between_the_rail_s_ends() {
        let at = |alignment: f32| {
            glyph_ys(
                NavigationRail::new(0, Msg::Go)
                    .item("H", "Home")
                    .item("S", "Search")
                    .group_alignment(alignment),
            )
        };
        let top = at(-1.0);
        let quarter = at(-0.5);
        let middle = at(0.0);
        let bottom = at(1.0);
        assert!(
            top[0] < quarter[0] && quarter[0] < middle[0] && middle[0] < bottom[0],
            "the group did not travel: {top:?} {quarter:?} {middle:?} {bottom:?}"
        );
        assert!(
            (bottom[1] - bottom[0] - (top[1] - top[0])).abs() < 0.01,
            "the group changed shape on the way: {top:?} against {bottom:?}"
        );
    }

    /// **A pinned slot does not travel with the group.** The reference pins the leading
    /// slot to the top and lets the trailing one travel (`navigation_rail.dart:148`,
    /// `:158`): a leading slot is chrome at the top of the rail, a trailing one is the
    /// tail of the list of destinations.
    #[test]
    fn the_leading_slot_stays_where_the_trailing_one_travels() {
        let at = |alignment: f32| {
            marks(
                NavigationRail::new(0, Msg::Go)
                    .leading(crate::text("L"))
                    .trailing(crate::text("T"))
                    .item("H", "Home")
                    .group_alignment(alignment),
            )
        };
        let (up, down) = (at(-1.0), at(1.0));
        let (top, bottom) = (y_of(&up), y_of(&down));
        assert!(
            (top("L") - bottom("L")).abs() < 0.01,
            "the leading slot moved with the group: {} against {}",
            top("L"),
            bottom("L")
        );
        assert!(
            bottom("T") > top("T"),
            "the trailing slot did not travel: {} against {}",
            bottom("T"),
            top("T")
        );
        assert!(
            bottom("T") > bottom("H"),
            "and it is still the tail of the list"
        );
        assert!(top("L") < top("H"), "the leading slot is above them");
    }

    /// And **which** of the two is pinned is the caller's to say, both ways round.
    #[test]
    fn and_which_of_them_is_pinned_can_be_swapped() {
        let at = |alignment: f32| {
            marks(
                NavigationRail::new(0, Msg::Go)
                    .leading(crate::text("L"))
                    .trailing(crate::text("T"))
                    .leading_at_top(false)
                    .trailing_at_bottom(true)
                    .item("H", "Home")
                    .group_alignment(alignment),
            )
        };
        let (up, down) = (at(-1.0), at(1.0));
        let (top, bottom) = (y_of(&up), y_of(&down));
        assert!(
            bottom("L") > top("L"),
            "an unpinned leading slot travels with the group"
        );
        assert!(
            (top("T") - bottom("T")).abs() < 0.01,
            "a pinned trailing slot does not: {} against {}",
            top("T"),
            bottom("T")
        );
        assert!(
            top("T") > top("H"),
            "and it stays below the destinations either way"
        );
    }

    /// Reads one glyph's `y` out of what [`marks`] collected.
    fn y_of(found: &[(String, f32)]) -> impl Fn(&str) -> f32 + '_ {
        move |glyph| {
            found
                .iter()
                .find(|(text, _)| text == glyph)
                .unwrap_or_else(|| panic!("{glyph} was never painted, only {found:?}"))
                .1
        }
    }

    /// A [`Status`] a destination is actually being painted under, rather than the
    /// default's zero opacity.
    fn live() -> Status {
        Status {
            opacity: 1.0,
            ..Default::default()
        }
    }

    /// The same, with the pointer over it.
    fn hovered() -> Status {
        Status {
            hover_progress: 1.0,
            ..live()
        }
    }

    /// Paints destination `index` of `rail`, in a rail-sized box.
    fn paint_destination(
        rail: &NavigationRail<Msg>,
        index: usize,
        status: Status,
        theme: &Theme,
    ) -> Scene {
        let mut scene = Scene::new();
        destinations(rail)[index].paint(
            Rect::new(0.0, 0.0, RAIL_WIDTH, ITEM_HEIGHT),
            status,
            theme,
            &mut scene,
        );
        scene
    }

    /// **A destination that cannot be reached says so, four ways** (milestone 436).
    ///
    /// The reference gives the glyph and the label one rule — `on_surface` at 38 %
    /// (`navigation_rail.dart:717`, `:723`) — and hands the ink well a null `onTap`
    /// (`:957`). The hover and the focus follow from there rather than from a fifth
    /// property: a hover is the promise of a click, and there is no click here.
    #[test]
    fn a_destination_that_cannot_be_reached_says_so() {
        let theme = Theme::default();
        // 9 with two destinations: nothing selected, so no indicator confuses the count.
        let rail = NavigationRail::new(9, Msg::Go)
            .labels(RailLabels::All)
            .item("H", "Home")
            .disabled();
        let item = &destinations(&rail)[0];
        assert_eq!(item.on_click(), None, "it emits nothing");
        assert!(!item.focusable(), "and the keyboard steps over it");

        let ink = disabled_content(&theme);
        assert_eq!(
            texts(&paint_destination(&rail, 0, live(), &theme)),
            vec![ink, ink],
            "one rule for the glyph and for the label"
        );
        assert!(
            rects(&paint_destination(&rail, 0, hovered(), &theme)).is_empty(),
            "and nothing lights under the pointer"
        );

        // Which means something only if a live one does all four.
        let live_rail = NavigationRail::new(9, Msg::Go)
            .labels(RailLabels::All)
            .item("H", "Home");
        let live_item = &destinations(&live_rail)[0];
        assert_eq!(live_item.on_click(), Some(Msg::Go(0)));
        assert!(live_item.focusable());
        assert_ne!(
            texts(&paint_destination(&live_rail, 0, live(), &theme))[0],
            ink
        );
        assert!(!rects(&paint_destination(&live_rail, 0, hovered(), &theme)).is_empty());
    }

    /// **A selected destination can show a different glyph** (`navigation_rail.dart:1132`).
    ///
    /// The reference pairs a stroked icon with its filled version, which is how a
    /// selected destination reads as selected without leaning on colour alone.
    #[test]
    fn a_selected_destination_can_show_a_different_glyph() {
        let theme = Theme::default();
        let glyph = |selected: usize| {
            let rail = NavigationRail::new(selected, Msg::Go)
                .item("H", "Home")
                .selected_icon("\u{2605}")
                .item("S", "Search");
            placed(&paint_destination(&rail, 0, live(), &theme))[0]
                .0
                .clone()
        };
        assert_eq!(glyph(1), "H", "at rest, the one it was given");
        assert_eq!(glyph(0), "\u{2605}", "selected, the other one");

        // A destination that names no second glyph keeps its first in both states.
        let plain = |selected: usize| {
            let rail = NavigationRail::new(selected, Msg::Go).item("H", "Home");
            placed(&paint_destination(&rail, 0, live(), &theme))[0]
                .0
                .clone()
        };
        assert_eq!(plain(0), plain(1));
    }

    /// **A destination's own indicator colour outranks the theme's**
    /// (`navigation_rail.dart:1144`) — how one entry marks itself out from the rest.
    #[test]
    fn a_destination_can_carry_its_own_indicator_colour() {
        let mut theme = Theme::default();
        theme.widgets.nav_rail.indicator_color = Some(Color::rgb8(1, 2, 3));
        let told = Color::rgb8(4, 5, 6);

        let mine = NavigationRail::new(0, Msg::Go)
            .item("H", "Home")
            .indicator_color(told);
        assert_eq!(
            rects(&paint_destination(&mine, 0, live(), &theme))
                .first()
                .copied(),
            Some(told)
        );

        let theirs = NavigationRail::new(0, Msg::Go).item("H", "Home");
        assert_eq!(
            rects(&paint_destination(&theirs, 0, live(), &theme))
                .first()
                .copied(),
            Some(Color::rgb8(1, 2, 3)),
            "and the theme's when the destination says nothing"
        );
    }

    /// The rail's destinations, which since milestone 433 sit inside the **group** that
    /// [`NavigationRail::group_alignment`] moves rather than directly under the rail.
    fn destinations(rail: &NavigationRail<Msg>) -> &[Box<dyn Widget<Msg>>] {
        // By what the children **are**, not by whether they are clickable: a rail whose
        // only destination is disabled has nothing clickable in it (milestone 436).
        Widget::<Msg>::children(rail)
            .iter()
            .find(|child| child.children().iter().any(|c| c.debug_name() == "NavItem"))
            .expect("the rail assembles its destinations into one group")
            .children()
    }

    #[test]
    fn rail_items_emit_index_and_track_selection() {
        let rail = NavigationRail::new(1, Msg::Go)
            .item("H", "Home")
            .item("S", "Search")
            .item("P", "Profile");
        let children = destinations(&rail);
        assert_eq!(children.len(), 3);
        assert_eq!(children[2].on_click(), Some(Msg::Go(2)));
    }

    #[test]
    fn badge_decorates_last_item_and_paints_counter() {
        let rail = NavigationRail::new(0, Msg::Go)
            .item("H", "Home")
            .item("M", "Mail")
            .badge(5);
        let children = destinations(&rail);
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
