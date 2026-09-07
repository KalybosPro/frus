//! [`NavigationDrawer`]: the **third** presentation of the primary navigation, beside
//! [`NavigationRail`](crate::NavigationRail) and [`BottomBar`](crate::BottomBar).
//!
//! A rail trades words for width and a bar trades height for reach. A drawer trades
//! nothing: it is the one form with room for a full label on every destination, a heading
//! above them and a footer below, which is why an application with more than five
//! destinations has nowhere else to put them.
//!
//! It is the **content** of a side panel, not the panel: [`Drawer`](crate::Drawer) is the
//! shell that slides, scrims and docks, and this is what goes inside it.
//!
//! ```ignore
//! Drawer::new(app.menu_open)
//!     .on_dismiss(Msg::CloseMenu)
//!     .panel(
//!         NavigationDrawer::new(app.tab, Msg::Go)
//!             .header(Text::new("Mailbox"))
//!             .item("in", "Inbox").badge(12)
//!             .item("st", "Starred")
//!             .child(Divider::new())
//!             .item("tr", "Trash"),
//!     )
//!     .body(page)
//! ```
//!
//! **A destination's index counts destinations**, not children: the divider above is one
//! of the drawer's children and none of its destinations, so `Trash` is destination 2 and
//! the message it emits says 2 (`navigation_drawer.dart:180`). That is why the reference
//! walks its child list twice, and why [`child`] exists here rather than a caller building
//! the column itself.
//!
//! Everything it paints is a default and nothing is a rule: the surface, how far off the
//! page it sits, the indicator's colour, shape and size, the height of a tile and the room
//! either side of it — each resolved from the instance, then
//! [`NavDrawerTheme`](crate::widgettheme::NavDrawerTheme), then the scheme's role.
//!
//! [`child`]: NavigationDrawer::child

use std::cell::{OnceCell, RefCell};

use frus_core::{Color, Insets, Point, Rect, ResolvedTextStyle, Scene, ShapeBorder, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::flex::Flex;
use crate::interaction::Status;
use crate::navrail::{DestinationIcon, NavigationDestination};
use crate::scroll::SingleChildScrollView;
use crate::theme::Theme;
use crate::widget::{FillAxes, Widget};
use crate::widgetstate::WidgetStateProperty;

/// A tile's height (`navigation_drawer.dart:730`).
const TILE_HEIGHT: f32 = 56.0;
/// The room either side of a tile (`navigation_drawer.dart:69`).
const TILE_PADDING: Insets = Insets::new(0.0, 12.0, 0.0, 12.0);
/// The selected indicator's box (`navigation_drawer.dart:732`). The width is a **ceiling**
/// — see the paint.
const INDICATOR_WIDTH: f32 = 336.0;
const INDICATOR_HEIGHT: f32 = 56.0;
/// From the tile's leading edge to the glyph (`navigation_drawer.dart:398`).
const ICON_LEAD: f32 = 16.0;
/// And from the glyph to the label (`:400`).
const ICON_GAP: f32 = 12.0;
/// The glyph's own box (`navigation_drawer.dart:755`).
const ICON_SIZE: f32 = 24.0;
/// How far the notification count sits from the trailing edge.
const BADGE_TRAIL: f32 = 24.0;
/// How far off the page the panel sits (`navigation_drawer.dart:729`).
const ELEVATION: f32 = 1.0;

/// The destinations' type — `labelLarge` (`navigation_drawer.dart:768`), where a rail's is
/// `labelMedium`. A drawer's label has a line to itself and can afford the larger step.
fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.nav_drawer.label_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_large)
        .resolved()
}

/// The notification count's type — `labelSmall`, the step [`crate::Badge`] reads.
fn badge_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.nav_drawer.badge_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_small)
        .resolved()
}

/// **What the caller put in the drawer**, in the order they put it there.
///
/// A drawer's children are a *mixed* list in the reference (`navigation_drawer.dart:122`):
/// destinations, and whatever else an application wants between them. Keeping the two in
/// one list rather than two is what makes a divider land where it was written instead of
/// after the last destination.
enum Entry<Msg> {
    /// A destination, which takes the next index. Boxed: a declared destination is a
    /// wide value — two marks, three optional colours, a shape and two strings — and the
    /// other arm is one pointer, so the list would otherwise pay a destination's width
    /// for every divider in it.
    Stop(Box<NavigationDestination>),
    /// Anything else — a heading, a rule, a spacer. It takes no index.
    Other(Box<dyn Widget<Msg>>),
}

/// **One destination**, as a full-width row: the indicator behind it, the glyph, the
/// label, and the count if it has one.
///
/// This is not a rail's row with different numbers. There the indicator is a pill **around
/// the glyph** and the label sits below it on the rail's own surface, which is why the two
/// take different colours when selected. Here the indicator is the row, so the label
/// stands on it and takes the indicator's content colour like the glyph does
/// (`navigation_drawer.dart:773` against `navigation_rail.dart:1251`).
struct DrawerTile<Msg> {
    icon: DestinationIcon,
    label: String,
    badge: Option<u32>,
    selected: bool,
    disabled: bool,
    /// Which destination this is, and how many there are — for what a reader hears.
    index: usize,
    count: usize,
    /// This destination's own indicator colour, over the drawer's, over the theme's.
    indicator_color: Option<Color>,
    indicator_shape: Option<ShapeBorder>,
    overlay_color: Option<WidgetStateProperty<Color>>,
    /// The surface the row stands on, so a state layer is measured from the colour that
    /// was actually painted under it.
    ground: Option<Color>,
    /// The whole rectangular area behind this one destination, when it was given one
    /// (`navigation_drawer.dart:228`). Not the indicator — that is the field above.
    background: Option<Color>,
    tile_height: Option<f32>,
    tile_padding: Option<Insets>,
    indicator_size: Option<(f32, f32)>,
    icon_size: Option<f32>,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
    message: Msg,
}

impl<Msg> DrawerTile<Msg> {
    /// The room either side, resolved.
    fn padding(&self, theme: Option<&Theme>) -> Insets {
        self.tile_padding
            .or(theme.and_then(|t| t.widgets.nav_drawer.tile_padding))
            .unwrap_or(TILE_PADDING)
    }

    /// The glyph's box, resolved.
    fn glyph_size(&self, theme: Option<&Theme>) -> f32 {
        self.icon_size
            .or(theme.and_then(|t| t.widgets.nav_drawer.icon_size))
            .unwrap_or(ICON_SIZE)
    }

    /// A tile keeps its declared height unless the reader enlarged the type past it: a row
    /// that clipped its own label would be a constant winning an argument with the
    /// system's font size. The same floor a rail's rows keep.
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let pad = self.padding(theme);
        let floor = self
            .tile_height
            .or(theme.and_then(|t| t.widgets.nav_drawer.tile_height))
            .unwrap_or(TILE_HEIGHT);
        let icon = frus_text::line_height(self.glyph_size(theme));
        let label = label_style(self.label_text_style, theme).line_height();
        Style {
            height: Dimension::Length(floor.max(icon.max(label) + 8.0) + pad.top + pad.bottom),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for DrawerTile<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    /// A row takes the width the drawer offers, and the **whole** width answers a tap even
    /// where the indicator is narrower than it — the reference's ink well wraps the stack,
    /// not the indicator (`navigation_drawer.dart:375`).
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let t = &theme.widgets.nav_drawer;
        let pad = self.padding(Some(theme));

        // **The whole rectangular area behind the destination** — outside the padding, so
        // it runs edge to edge (`navigation_drawer.dart:226`). Painted first: everything
        // else stands on it.
        if let Some(background) = self.background {
            scene.fill_rect(bounds, background.fade(o));
        }

        let inner = Rect::new(
            bounds.x + pad.left,
            bounds.y + pad.top,
            (bounds.width - pad.left - pad.right).max(0.0),
            (bounds.height - pad.top - pad.bottom).max(0.0),
        );

        // **The indicator's declared width is a ceiling, not a promise.** The reference
        // asks for 336 inside a panel that is 304 wide by default, and the stack's
        // constraints settle it: what is drawn is the narrower of the two. Reading the
        // constant as a width would push the pill past the panel's edge in every
        // application that never changed the drawer's width — which is all of them.
        let (want_w, want_h) = self
            .indicator_size
            .or(t.indicator_size)
            .unwrap_or((INDICATOR_WIDTH, INDICATOR_HEIGHT));
        let ind_w = want_w.min(inner.width);
        let ind_h = want_h.min(inner.height);
        let pill = Rect::new(
            inner.x + (inner.width - ind_w) * 0.5,
            inner.y + (inner.height - ind_h) * 0.5,
            ind_w,
            ind_h,
        );

        // The ground a state layer is measured from: what was actually painted under this
        // row — this destination's own colour if it has one, else the drawer's.
        let ground = self
            .background
            .or(self.ground)
            .or(t.background_color)
            .unwrap_or(theme.scheme.surface_container_low);
        let indicator = self
            .indicator_color
            .or(t.indicator_color)
            .unwrap_or(theme.scheme.secondary_container);
        let base = if self.selected { indicator } else { ground };

        let states = status
            .states()
            .set(crate::WidgetState::Selected, self.selected)
            .set(crate::WidgetState::Disabled, self.disabled);
        // The destination's word, then the theme's, then the framework's state layer.
        // Nothing lights on a destination that cannot be reached: a state layer is the
        // promise of an interaction, and there is none here.
        let told = self
            .overlay_color
            .as_ref()
            .and_then(|own| own.resolve(states))
            .or_else(|| t.overlay_color.as_ref().and_then(|w| w.resolve(states)));
        let fill = if self.disabled {
            base
        } else if let Some(overlay) = told {
            base.lerp(overlay.fade(1.0), overlay.a)
        } else {
            theme.state_layer(base, theme.scheme.primary, &status)
        };
        if self.selected || fill != base {
            let shape = self
                .indicator_shape
                .or(t.indicator_shape)
                .unwrap_or_else(ShapeBorder::stadium);
            scene.draw_shape(pill, shape, fill.fade(o));
        }

        // **One colour for both halves.** The glyph and the label are both on the
        // indicator here, so both take its content colour when selected
        // (`navigation_drawer.dart:759` and `:773`) — the one thing a drawer's row does
        // differently from a rail's, and it follows from the indicator's size rather than
        // from a separate decision.
        let ink = if self.disabled {
            disabled_content(theme)
        } else if self.selected {
            t.selected_color
                .unwrap_or(theme.scheme.on_secondary_container)
        } else {
            t.unselected_color
                .unwrap_or(theme.scheme.on_surface_variant)
        };

        let size = self.glyph_size(Some(theme));
        let label_s = label_style(self.label_text_style, Some(theme));
        let icon_m = self.icon.measure(size);
        let label_m = frus_text::measure_resolved(&self.label, &label_s);

        // The row starts at the tile's leading edge, not the indicator's: in a drawer wide
        // enough for the full 336 the pill is centred and the glyph is not. And the label
        // starts a fixed distance in, from the glyph's **box** rather than from the ink it
        // happened to put in it — otherwise a column of destinations has its labels at as
        // many different offsets as it has glyph widths.
        let x = inner.x + ICON_LEAD;
        self.icon.paint(
            size,
            x,
            inner.y + (inner.height - icon_m.height) * 0.5,
            ink.fade(o),
            theme,
            scene,
        );
        scene.text(
            Point::new(
                x + size + ICON_GAP,
                inner.y + (inner.height - label_m.height) * 0.5,
            ),
            self.label.clone(),
            &label_s,
            ink.fade(o),
        );

        // The count, at the trailing end of the row. A rail hangs it off the glyph because
        // an 80-pixel column has nowhere else; here the row is wide and it goes where the
        // eye finishes reading.
        if let Some(count) = self.badge.filter(|&n| n > 0) {
            let text = if count > 99 {
                "99+".to_string()
            } else {
                count.to_string()
            };
            let badge_s = badge_style(self.badge_text_style, Some(theme));
            let m = frus_text::measure_resolved(&text, &badge_s);
            let right = (pill.x + pill.width) - BADGE_TRAIL;
            scene.text(
                Point::new(right - m.width, inner.y + (inner.height - m.height) * 0.5),
                text,
                &badge_s,
                ink.fade(o),
            );
        }
    }

    /// A destination that cannot be reached emits nothing.
    fn on_click(&self) -> Option<Msg> {
        (!self.disabled).then(|| self.message.clone())
    }

    /// And the keyboard skips it.
    fn focusable(&self) -> bool {
        !self.disabled
    }

    /// **What a reader hears**: the destination's name, then where it sits in the list.
    ///
    /// The reference reads out "Home, Tab 1 of 3" (`navigation_drawer.dart:464`), and the
    /// second half is the part a list cannot supply on its own — someone hearing five
    /// names in a row has no way to know how many are left. The role is the tab's, as
    /// [`crate::Tabs`] already uses, and *which one is live* survives being disabled: a
    /// reader who cannot switch is still owed where they are.
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let words = crate::localizations::of();
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Tab)
            .label(format!(
                "{}, {}",
                self.label,
                words.tab_label(self.index + 1, self.count)
            ))
            .toggled(self.selected);
        Some(if self.disabled {
            semantics.disabled(true)
        } else {
            semantics.clickable()
        })
    }
}

/// A **panel of destinations**: the navigation's widest form.
pub struct NavigationDrawer<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    /// Destinations and whatever the caller put between them, in order.
    entries: RefCell<Vec<Entry<Msg>>>,
    header: RefCell<Option<Box<dyn Widget<Msg>>>>,
    footer: RefCell<Option<Box<dyn Widget<Msg>>>>,
    width: Option<f32>,
    background: Option<Color>,
    elevation: Option<f32>,
    indicator_color: Option<Color>,
    indicator_shape: Option<ShapeBorder>,
    indicator_size: Option<(f32, f32)>,
    tile_height: Option<f32>,
    tile_padding: Option<Insets>,
    icon_size: Option<f32>,
    label_text_style: Option<TextStyle>,
    badge_text_style: Option<TextStyle>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> NavigationDrawer<Msg> {
    /// Creates a drawer: `selected` is the live destination and `on_select(i)` is emitted
    /// when one is chosen.
    ///
    /// An index outside the list leaves every destination unselected, which is what the
    /// reference's `-1` is for (`navigation_drawer.dart:138`) and what a screen showing
    /// none of them wants.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_select: Box::new(on_select),
            entries: RefCell::new(Vec::new()),
            header: RefCell::new(None),
            footer: RefCell::new(None),
            width: None,
            background: None,
            elevation: None,
            indicator_color: None,
            indicator_shape: None,
            indicator_size: None,
            tile_height: None,
            tile_padding: None,
            icon_size: None,
            label_text_style: None,
            badge_text_style: None,
            built: OnceCell::new(),
        }
    }

    /// Adds a destination: a glyph and a name.
    pub fn item(self, icon: impl Into<DestinationIcon>, label: impl Into<String>) -> Self {
        self.destination(NavigationDestination::new(icon, label))
    }

    /// Adds a destination declared elsewhere — what [`crate::NavScaffold`] forwards, so
    /// that one list feeds all three forms.
    pub(crate) fn destination(mut self, destination: NavigationDestination) -> Self {
        self.entries
            .borrow_mut()
            .push(Entry::Stop(Box::new(destination)));
        self.rebuild();
        self
    }

    /// Puts **anything else** in the list at this point — a heading, a
    /// [`Divider`](crate::Divider), a spacer (`navigation_drawer.dart:120`).
    ///
    /// It takes no destination index, so the destinations either side of it keep the
    /// numbers they would have had.
    pub fn child(self, child: impl Widget<Msg> + 'static) -> Self {
        self.child_boxed(Box::new(child))
    }

    /// [`Self::child`], for a widget already boxed.
    pub fn child_boxed(mut self, child: Box<dyn Widget<Msg>>) -> Self {
        self.entries.borrow_mut().push(Entry::Other(child));
        self.rebuild();
        self
    }

    /// Adds a whole list of destinations **declared elsewhere** — one navigation, handed
    /// to whichever chrome the width called for.
    ///
    /// This is the reason [`NavigationDestination`] is a value. An application that shows
    /// a bar when narrow and a rail when wide used to declare its destinations twice, and
    /// the two drifted; now it declares them once.
    ///
    /// ```
    /// use frus_widgets::{Icons, NavigationDestination, NavigationDrawer};
    ///
    /// let places = vec![
    ///     NavigationDestination::new(Icons::FAVORITE_BORDER, "Saved")
    ///         .selected_icon(Icons::FAVORITE),
    ///     NavigationDestination::new(Icons::MAIL_OUTLINE, "Inbox").badge(3),
    /// ];
    /// let _drawer = NavigationDrawer::new(0, |i| i).destinations(places);
    /// ```
    #[must_use]
    pub fn destinations(
        mut self,
        destinations: impl IntoIterator<Item = NavigationDestination>,
    ) -> Self {
        self.entries
            .borrow_mut()
            .extend(destinations.into_iter().map(|d| Entry::Stop(Box::new(d))));
        self.rebuild();
        self
    }

    /// A notification count on the **last** destination.
    pub fn badge(self, count: u32) -> Self {
        self.decorate(|last| last.badge = Some(count))
    }
    /// What a pointer resting on the **last** destination is told
    /// (`navigation_rail.dart:1155`).
    ///
    /// The case this exists for is a rail: it shows glyphs without labels by default, so
    /// the mark is all a destination says about itself, and a mark is not always enough.
    #[must_use]
    pub fn tooltip(self, message: impl Into<String>) -> Self {
        let message = message.into();
        self.decorate(move |last| last.tooltip = Some(message))
    }

    /// The glyph the **last** destination shows while it is selected
    /// (`navigation_drawer.dart:248`).
    pub fn selected_icon(self, icon: impl Into<DestinationIcon>) -> Self {
        let icon = icon.into();
        self.decorate(move |last| last.selected_icon = Some(icon))
    }

    /// Marks the **last** destination unreachable (`navigation_drawer.dart:260`).
    pub fn disabled(self) -> Self {
        self.decorate(|last| last.disabled = true)
    }

    /// The area behind the **last** destination, edge to edge — not its indicator
    /// (`navigation_drawer.dart:228`).
    pub fn tile_color(self, color: Color) -> Self {
        self.decorate(move |last| last.background = Some(color))
    }

    /// The **last** destination's own indicator colour, over the drawer's.
    pub fn destination_indicator_color(self, color: Color) -> Self {
        self.decorate(move |last| last.indicator_color = Some(color))
    }

    /// The **last** destination's own indicator shape, over the drawer's.
    pub fn destination_indicator_shape(self, shape: ShapeBorder) -> Self {
        self.decorate(move |last| last.indicator_shape = Some(shape))
    }

    /// The **last** destination's own highlight per state, over the framework's state
    /// layer.
    pub fn destination_overlay_color(self, overlay: WidgetStateProperty<Color>) -> Self {
        self.decorate(move |last| last.overlay_color = Some(overlay))
    }

    /// Applies `f` to the destination most recently added; silent when there is none.
    fn decorate(mut self, f: impl FnOnce(&mut NavigationDestination)) -> Self {
        {
            let mut entries = self.entries.borrow_mut();
            if let Some(Entry::Stop(last)) = entries
                .iter_mut()
                .rev()
                .find(|e| matches!(e, Entry::Stop(_)))
            {
                f(last);
            }
        }
        self.rebuild();
        self
    }

    /// A widget above the destinations, outside the part that scrolls
    /// (`navigation_drawer.dart:127`) — a title, an account row, a logo.
    pub fn header(self, header: impl Widget<Msg> + 'static) -> Self {
        self.header_boxed(Box::new(header))
    }

    /// [`Self::header`], for a widget already boxed.
    pub fn header_boxed(mut self, header: Box<dyn Widget<Msg>>) -> Self {
        *self.header.borrow_mut() = Some(header);
        self.rebuild();
        self
    }

    /// A widget below the destinations, outside the part that scrolls
    /// (`navigation_drawer.dart:132`) — settings, a sign-out, a version string.
    pub fn footer(self, footer: impl Widget<Msg> + 'static) -> Self {
        self.footer_boxed(Box::new(footer))
    }

    /// [`Self::footer`], for a widget already boxed.
    pub fn footer_boxed(mut self, footer: Box<dyn Widget<Msg>>) -> Self {
        *self.footer.borrow_mut() = Some(footer);
        self.rebuild();
        self
    }

    /// How wide the panel asks to be. Unset, it takes the width it is offered — which is
    /// what a [`Drawer`](crate::Drawer) gives it, so a panel's width stays the panel's
    /// business and this is for one standing on its own.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The panel's surface. Unset, the theme's, then the scheme's `surface_container_low`
    /// (`navigation_drawer.dart:740`).
    pub fn background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.rebuild();
        self
    }

    /// How far off the page the panel sits. Unset, the reference's `1`.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// The pill behind the selected destination. Unset, the scheme's
    /// `secondary_container`.
    pub fn indicator_color(mut self, color: Color) -> Self {
        self.indicator_color = Some(color);
        self.rebuild();
        self
    }

    /// That pill's shape. Unset, a stadium (`navigation_drawer.dart:731`).
    pub fn indicator_shape(mut self, shape: ShapeBorder) -> Self {
        self.indicator_shape = Some(shape);
        self.rebuild();
        self
    }

    /// That pill's box, `(width, height)`. The width is a **ceiling**: a pill never grows
    /// past the room its tile has. Unset, the reference's `336 x 56`.
    pub fn indicator_size(mut self, width: f32, height: f32) -> Self {
        self.indicator_size = Some((width, height));
        self.rebuild();
        self
    }

    /// A tile's height. Unset, `56`.
    pub fn tile_height(mut self, height: f32) -> Self {
        self.tile_height = Some(height);
        self.rebuild();
        self
    }

    /// The room either side of a tile. Unset, `12` left and right
    /// (`navigation_drawer.dart:69`).
    pub fn tile_padding(mut self, padding: Insets) -> Self {
        self.tile_padding = Some(padding);
        self.rebuild();
        self
    }

    /// The destinations' glyphs. Unset, `24`.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(size);
        self.rebuild();
        self
    }

    /// The destinations' labels. Unset, `labelLarge`.
    pub fn label_text_style(mut self, style: TextStyle) -> Self {
        self.label_text_style = Some(style);
        self.rebuild();
        self
    }

    /// The count a destination carries. Unset, `labelSmall`.
    pub fn badge_text_style(mut self, style: TextStyle) -> Self {
        self.badge_text_style = Some(style);
        self.rebuild();
        self
    }

    /// Throws the assembled subtree away, so that the next read of it sees everything the
    /// builders have said and their order cannot change the answer.
    fn rebuild(&mut self) {
        self.built.take();
    }

    /// The subtree: the header, the destinations in a list that scrolls, the footer.
    ///
    /// The reference's `ListView` is why the middle scrolls and the two ends do not
    /// (`navigation_drawer.dart:194`): a drawer with twelve destinations on a short window
    /// must not push its footer off the bottom, and a header that scrolled away would take
    /// the account it names with it.
    fn assemble(&self) -> Vec<Box<dyn Widget<Msg>>> {
        let entries = std::mem::take(&mut *self.entries.borrow_mut());
        let count = entries
            .iter()
            .filter(|e| matches!(e, Entry::Stop(_)))
            .count();

        let mut list = Flex::<Msg>::column();
        let mut index = 0;
        for entry in entries {
            match entry {
                Entry::Other(child) => list = list.child_boxed(child),
                Entry::Stop(stop) => {
                    let selected = index == self.selected;
                    let tooltip = stop.tooltip.clone();
                    let tile = DrawerTile {
                        icon: stop.glyph(selected).clone(),
                        label: stop.label.clone(),
                        badge: stop.badge,
                        selected,
                        disabled: stop.disabled,
                        index,
                        count,
                        indicator_color: stop.indicator_color.or(self.indicator_color),
                        indicator_shape: stop.indicator_shape.or(self.indicator_shape),
                        overlay_color: stop.overlay_color.clone(),
                        ground: self.background,
                        background: stop.background,
                        tile_height: self.tile_height,
                        tile_padding: self.tile_padding,
                        indicator_size: self.indicator_size,
                        icon_size: self.icon_size,
                        label_text_style: self.label_text_style,
                        badge_text_style: self.badge_text_style,
                        message: (self.on_select)(index),
                    };
                    // A row that was given a hint is wrapped in the widget that shows
                    // hints, rather than growing a second one inside the tile.
                    list = match tooltip {
                        Some(message) => list.child(crate::Tooltip::new(message).child(tile)),
                        None => list.child(tile),
                    };
                    index += 1;
                }
            }
        }

        let mut out: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        if let Some(header) = self.header.borrow_mut().take() {
            out.push(header);
        }
        out.push(Box::new(
            SingleChildScrollView::<Msg>::new().flex(1.0).child(list),
        ));
        if let Some(footer) = self.footer.borrow_mut().take() {
            out.push(footer);
        }
        out
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for NavigationDrawer<Msg> {
    /// A column, clear of the intrusion at the top.
    ///
    /// The reference wraps its content in a `SafeArea` with `bottom: false`
    /// (`navigation_drawer.dart:189`): the notch is kept off the header, and the gesture
    /// bar is left to whatever the panel stands in — the shell that already consumed it.
    fn style(&self) -> Style {
        let safe = crate::MediaQuery::of().padding;
        Style {
            width: self.width.map_or(Dimension::Auto, Dimension::Length),
            flex_direction: FlexDirection::Column,
            padding: Insets::new(safe.top, 0.0, 0.0, 0.0),
            ..Default::default()
        }
    }

    /// It fills the panel it was put in, on both axes unless it was given a width.
    fn fill_axes(&self, _theme: &Theme) -> FillAxes {
        if self.width.is_some() {
            FillAxes::HEIGHT
        } else {
            FillAxes::BOTH
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble())
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let t = &theme.widgets.nav_drawer;
        let depth = self.elevation.or(t.elevation).unwrap_or(ELEVATION);
        if depth > 0.0 {
            let blur = depth * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                frus_core::BorderRadius::uniform(blur),
                blur,
            );
        }
        let fill = self
            .background
            .or(t.background_color)
            .unwrap_or(theme.scheme.surface_container_low);
        scene.fill_rect(bounds, fill.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::divider::Divider;
    use crate::text::Text;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Go(usize),
    }

    /// Three destinations, the middle one selected, with a rule between the second and
    /// the third.
    fn drawer() -> NavigationDrawer<Msg> {
        NavigationDrawer::new(1, Msg::Go)
            .item("I", "Inbox")
            .item("S", "Starred")
            .child(Divider::new())
            .item("T", "Trash")
    }

    /// Every tile in a built drawer, in order — found the way the walk finds anything,
    /// by descending. There is no downcast on the trait, and a test that reached for one
    /// would be checking a shape the framework does not have.
    fn tiles(drawer: &NavigationDrawer<Msg>) -> Vec<&dyn Widget<Msg>> {
        fn walk<'a>(node: &'a dyn Widget<Msg>, out: &mut Vec<&'a dyn Widget<Msg>>) {
            if node.debug_name() == "DrawerTile" {
                out.push(node);
            }
            for child in node.children() {
                walk(&**child, out);
            }
        }
        let mut out = Vec::new();
        for child in Widget::<Msg>::children(drawer) {
            walk(&**child, &mut out);
        }
        out
    }

    fn scene_of(widget: &dyn Widget<Msg>, bounds: Rect, theme: &Theme) -> Scene {
        let mut scene = Scene::new();
        widget.paint(
            bounds,
            Status {
                opacity: 1.0,
                ..Default::default()
            },
            theme,
            &mut scene,
        );
        scene
    }

    fn texts(scene: &Scene) -> Vec<(String, Color)> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, color, .. } => Some((text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    /// **A destination's index counts destinations.**
    ///
    /// The rule the reference spends a second walk of its child list on
    /// (`navigation_drawer.dart:180`), and the one that breaks silently: put a rule
    /// between two groups of destinations and every destination below it answers with the
    /// wrong screen. Nothing about the picture would say so.
    #[test]
    fn a_rule_between_destinations_takes_no_number() {
        let drawer = drawer();
        let tiles = tiles(&drawer);
        assert_eq!(
            tiles.len(),
            3,
            "three destinations, and the rule is not one"
        );
        assert_eq!(
            tiles
                .iter()
                .filter_map(|t| t.on_click())
                .collect::<Vec<_>>(),
            vec![Msg::Go(0), Msg::Go(1), Msg::Go(2)],
            "the third destination is 2, not 3"
        );
        assert_eq!(
            tiles
                .iter()
                .map(|t| t.semantics().expect("announced").toggled)
                .collect::<Vec<_>>(),
            vec![
                frus_core::Toggled::False,
                frus_core::Toggled::True,
                frus_core::Toggled::False
            ],
            "and the middle one is the live one"
        );
    }

    /// And what a reader hears counts them the same way.
    #[test]
    fn a_reader_is_told_which_of_how_many() {
        let drawer = drawer();
        let heard: Vec<String> = tiles(&drawer)
            .iter()
            .filter_map(|t| t.semantics()?.label)
            .collect();
        assert_eq!(
            heard,
            vec![
                "Inbox, Tab 1 of 3".to_string(),
                "Starred, Tab 2 of 3".to_string(),
                "Trash, Tab 3 of 3".to_string(),
            ],
            "the rule between them is not a destination, so it is not counted either"
        );
    }

    /// One tile on its own, to be painted at a chosen width.
    fn lone_tile(selected: bool) -> DrawerTile<Msg> {
        DrawerTile {
            icon: "I".into(),
            label: "Inbox".into(),
            badge: None,
            selected,
            disabled: false,
            index: 0,
            count: 1,
            indicator_color: None,
            indicator_shape: None,
            overlay_color: None,
            ground: None,
            background: None,
            tile_height: None,
            tile_padding: None,
            indicator_size: None,
            icon_size: None,
            label_text_style: None,
            badge_text_style: None,
            message: Msg::Go(0),
        }
    }

    /// **The indicator's width is a ceiling.** The reference asks for 336 inside a panel
    /// 304 wide, and what is drawn is the narrower of the two — otherwise the pill hangs
    /// out past the panel's edge in every application that never set a width, which is
    /// all of them.
    #[test]
    fn the_pill_never_grows_past_the_room_the_tile_has() {
        let theme = Theme::default();
        let tile = lone_tile(true);
        let pill_width = |width: f32| {
            let scene = scene_of(&tile, Rect::new(0.0, 0.0, width, TILE_HEIGHT), &theme);
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { rect, .. } => Some(rect.width),
                    _ => None,
                })
                .expect("a selected destination has an indicator")
        };

        assert_eq!(
            pill_width(304.0),
            304.0 - 24.0,
            "in a 304-wide panel the pill takes what is left after the padding"
        );
        assert_eq!(
            pill_width(600.0),
            INDICATOR_WIDTH,
            "and stops at its declared width once there is room for it"
        );
    }

    /// **Both halves take the indicator's colour**, which is the one thing a drawer's row
    /// does differently from a rail's: there the label sits below the pill on the rail's
    /// own surface, here the pill is the row.
    #[test]
    fn a_selected_row_paints_its_glyph_and_its_label_the_same() {
        let theme = Theme::default();
        let scene = scene_of(
            &lone_tile(true),
            Rect::new(0.0, 0.0, 360.0, TILE_HEIGHT),
            &theme,
        );
        let painted = texts(&scene);
        assert_eq!(painted.len(), 2, "a glyph and a label");
        assert_eq!(
            painted[0].1, painted[1].1,
            "the glyph and the label stand on the same indicator"
        );
        assert_eq!(painted[0].1, theme.scheme.on_secondary_container);

        let scene = scene_of(
            &lone_tile(false),
            Rect::new(0.0, 0.0, 360.0, TILE_HEIGHT),
            &theme,
        );
        assert_eq!(texts(&scene)[0].1, theme.scheme.on_surface_variant);
    }

    /// A destination that cannot be reached is inert to the tap and to the tab, and says
    /// so rather than falling silent.
    #[test]
    fn a_destination_that_cannot_be_reached_answers_nowhere() {
        let drawer = NavigationDrawer::new(0, Msg::Go)
            .item("I", "Inbox")
            .item("A", "Archive")
            .disabled();
        let tiles = tiles(&drawer);
        let off = *tiles.last().expect("two of them");
        assert_eq!(off.on_click(), None, "the tap goes nowhere");
        assert!(!off.focusable(), "and the tab skips it");
        let semantics = off.semantics().expect("it is still announced");
        assert!(semantics.disabled, "and it is announced as unavailable");
        assert!(!semantics.clickable);
        assert_eq!(
            tiles[0].on_click(),
            Some(Msg::Go(0)),
            "its neighbour is untouched"
        );
    }

    /// Nothing lights on a destination that cannot be reached: a state layer is the
    /// promise of an interaction and there is none here.
    #[test]
    fn a_destination_that_cannot_be_reached_does_not_light_up() {
        let theme = Theme::default();
        let mut tile = lone_tile(false);
        tile.disabled = true;
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &tile,
            Rect::new(0.0, 0.0, 360.0, TILE_HEIGHT),
            Status {
                opacity: 1.0,
                hover_progress: 1.0,
                ..Default::default()
            },
            &theme,
            &mut scene,
        );
        assert!(
            !scene
                .primitives()
                .iter()
                .any(|p| matches!(p, frus_core::Primitive::Rect { .. })),
            "an unreachable destination has nothing to say to a pointer resting on it"
        );
    }

    /// The header and the footer are outside the part that scrolls, so a long list cannot
    /// push either of them off the panel.
    #[test]
    fn the_two_ends_do_not_scroll() {
        let drawer = drawer()
            .header(Text::new("Mailbox"))
            .footer(Text::new("v1"));
        let children = Widget::<Msg>::children(&drawer);
        assert_eq!(children.len(), 3, "header, list, footer");
        assert_eq!(
            children[1].debug_name(),
            "SingleChildScrollView",
            "and only the middle one scrolls"
        );
    }

    /// **The builders are order-independent**, which is what throwing the assembled
    /// subtree away on every builder buys: a colour named after the destinations still
    /// reaches them.
    #[test]
    fn saying_it_afterwards_says_it() {
        let theme = Theme::default();
        let ink = Color::rgb8(200, 30, 30);
        let before = NavigationDrawer::new(0, Msg::Go)
            .indicator_color(ink)
            .item("I", "Inbox");
        let after = NavigationDrawer::new(0, Msg::Go)
            .item("I", "Inbox")
            .indicator_color(ink);
        let fill = |drawer: &NavigationDrawer<Msg>| {
            let found = tiles(drawer);
            let scene = scene_of(found[0], Rect::new(0.0, 0.0, 360.0, TILE_HEIGHT), &theme);
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { color, .. } => Some(*color),
                    _ => None,
                })
                .expect("the selected destination has an indicator")
        };
        assert_eq!(fill(&before), ink);
        assert_eq!(fill(&after), ink, "the order the builders were called in");
    }

    /// A tile keeps its declared height until the reader's own type outgrows it, at which
    /// point the row grows rather than clipping the label.
    #[test]
    fn a_row_grows_before_it_clips() {
        let theme = Theme::default();
        let plain_drawer = drawer();
        let plain = tiles(&plain_drawer)[0].style_themed(&theme).height;
        assert_eq!(plain, Dimension::Length(TILE_HEIGHT));

        let big = drawer().label_text_style(TextStyle::new(48.0));
        let grown = tiles(&big)[0].style_themed(&theme).height;
        assert!(
            matches!(grown, Dimension::Length(h) if h > TILE_HEIGHT),
            "the row makes room for the type rather than cutting it: {grown:?}"
        );
    }

    /// The panel's surface is a rung above a rail's: a drawer is a sheet over the page,
    /// not a strip beside it.
    #[test]
    fn the_panel_stands_a_rung_above_a_rail() {
        let theme = Theme::default();
        let scene = scene_of(&drawer(), Rect::new(0.0, 0.0, 304.0, 600.0), &theme);
        let surface = scene
            .primitives()
            .iter()
            .rev()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(*color),
                _ => None,
            })
            .expect("the panel paints one");
        assert_eq!(surface, theme.scheme.surface_container_low);
        assert_ne!(
            theme.scheme.surface_container_low, theme.scheme.surface,
            "the two rungs have to be tellable apart for that to mean anything"
        );
    }
}
