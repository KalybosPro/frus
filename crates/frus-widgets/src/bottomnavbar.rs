// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! [`BottomNavigationBar`]: the bar of three to five destinations along the bottom of a
//! phone screen, in its classic form (milestone 731).
//!
//! frus already had [`BottomBar`](crate::BottomBar), the current design's navigation bar —
//! a pill under the selected icon, every label shown. This is the other one, which many
//! applications are designed around and which is what most people ask for by this name:
//!
//! - **two kinds** ([`BottomNavigationBarType`]): `Fixed`, every item the same width in the
//!   selected colour or not; and `Shifting`, where the selected item widens, only its label
//!   shows, and the bar takes the item's own background colour with a splash that spreads
//!   from it. Three items or fewer are fixed and four or more shifting unless told;
//! - the selected and unselected colours, font sizes, label styles, whether each kind of
//!   label is shown, an icon of its own for the selected item, a tooltip per item;
//! - its **landscape layouts** ([`BottomNavigationBarLandscapeLayout`]): spread across the
//!   width, centred in a portrait-wide strip, or each icon beside its label;
//! - a [`BottomNavigationBarTheme`] under `theme.widgets.bottom_navigation_bar` for every
//!   one of those.
//!
//! It is a widget built from widgets: a row of ink-wells whose sizes, colours and label
//! fades follow one [`AnimationHandle`](crate::AnimationHandle) per item, moved by the
//! frames over 200 ms when the selection changes, and a canvas under them that paints the
//! shifting bar's splash. It sits in [`Scaffold::bottom_navigation_bar`](crate::Scaffold),
//! which gives it the bottom edge and the room under it the system's bars take.

use std::rc::Rc;

use frus_core::{AnimationController, Color, Curve, Path, Point, TextStyle};

use crate::callback::Callback;
use crate::lazy::Lazy;
use crate::media::Brightness;
use crate::navrail::DestinationIcon;
use crate::widget::Widget;
use crate::Orientation;
use crate::{
    component, AnimationHandle, Center, Column, CustomPaint, Expanded, Flexible, Icon, InkWell,
    Insets, Justify, MainAxisSize, MediaQuery, Opacity, Padding, Positioned, Row, SizedBox, Stack,
    Text, Tooltip,
};

/// The bar's height, above the room it keeps for the system's bars.
pub const BOTTOM_NAVIGATION_BAR_HEIGHT: f32 = 56.0;

/// How long a change of selection takes to show.
const DURATION: f32 = 0.2;

/// The curve the selection moves along: fast out, slow in.
fn curve() -> Curve {
    Curve::Cubic {
        x1: 0.4,
        y1: 0.0,
        x2: 0.2,
        y2: 1.0,
    }
}

/// How a [`BottomNavigationBar`]'s items are laid out and coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BottomNavigationBarType {
    /// Every item the same width; the selected one in the selected colour.
    Fixed,
    /// The selected item widens and shows its label; the bar takes its background colour.
    Shifting,
}

/// How a [`BottomNavigationBar`] lays its items out when the screen is in landscape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BottomNavigationBarLandscapeLayout {
    /// Spread across the whole width, each icon above its label.
    #[default]
    Spread,
    /// In a strip as wide as the screen is tall, centred, each icon above its label.
    Centered,
    /// Spread across the width, each icon beside its label.
    Linear,
}

/// One destination of a [`BottomNavigationBar`].
#[derive(Clone, Debug, PartialEq)]
pub struct BottomNavigationBarItem {
    icon: DestinationIcon,
    active_icon: Option<DestinationIcon>,
    label: String,
    background_color: Option<Color>,
    tooltip: Option<String>,
}

impl BottomNavigationBarItem {
    /// An item with `icon` — one of the bundled icons, or a glyph — and `label`.
    pub fn new(icon: impl Into<DestinationIcon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            active_icon: None,
            label: label.into(),
            background_color: None,
            tooltip: None,
        }
    }

    /// The icon the item shows while it is selected. Unset, the same icon.
    #[must_use]
    pub fn active_icon(mut self, icon: impl Into<DestinationIcon>) -> Self {
        self.active_icon = Some(icon.into());
        self
    }

    /// The bar's colour while this item is selected, for a [shifting] bar.
    ///
    /// [shifting]: BottomNavigationBarType::Shifting
    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// What a long press or a hover says about the item. Unset, nothing; an empty one,
    /// nothing either.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Defaults for [`BottomNavigationBar`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BottomNavigationBarTheme {
    /// The bar's colour, for a fixed bar. Unset, the scheme's `surface`.
    pub background_color: Option<Color>,
    /// How far the bar sits off the page. Unset, 8.
    pub elevation: Option<f32>,
    /// The selected item's colour. Unset, `primary` in a light theme and `secondary` in a
    /// dark one for a fixed bar, and `surface` for a shifting one.
    pub selected_item_color: Option<Color>,
    /// The other items' colour. Unset, a 54 % black (70 % white in a dark theme) for a
    /// fixed bar, and `surface` for a shifting one.
    pub unselected_item_color: Option<Color>,
    /// The selected label's type.
    pub selected_label_style: Option<TextStyle>,
    /// The other labels' type.
    pub unselected_label_style: Option<TextStyle>,
    /// Whether the selected item's label shows. Unset, it does.
    pub show_selected_labels: Option<bool>,
    /// Whether the other items' labels show. Unset, on a fixed bar and not a shifting one.
    pub show_unselected_labels: Option<bool>,
    /// Which kind of bar. Unset, fixed for three items or fewer, shifting for more.
    pub kind: Option<BottomNavigationBarType>,
    /// How it lays out in landscape. Unset, spread.
    pub landscape_layout: Option<BottomNavigationBarLandscapeLayout>,
}

/// What a bar is made of, until it is laid out.
struct Parts {
    items: Vec<BottomNavigationBarItem>,
    current_index: usize,
    on_tap: Option<Rc<dyn Fn(usize) -> Callback>>,
    elevation: Option<f32>,
    kind: Option<BottomNavigationBarType>,
    background_color: Option<Color>,
    icon_size: f32,
    selected_item_color: Option<Color>,
    unselected_item_color: Option<Color>,
    selected_font_size: f32,
    unselected_font_size: f32,
    selected_label_style: Option<TextStyle>,
    unselected_label_style: Option<TextStyle>,
    show_selected_labels: Option<bool>,
    show_unselected_labels: Option<bool>,
    landscape_layout: Option<BottomNavigationBarLandscapeLayout>,
    use_legacy_color_scheme: bool,
}

/// **A bar of three to five destinations along the bottom of the screen**, the selected
/// one marked by colour — and, on a [shifting] bar, by its width, its label and the bar's
/// own colour.
///
/// ```
/// use frus_widgets::{BottomNavigationBar, BottomNavigationBarItem, Icons};
///
/// let bar: BottomNavigationBar = BottomNavigationBar::new(0, |index| {
///     let _ = index; // show that destination
/// })
/// .item(BottomNavigationBarItem::new(Icons::HOME, "Home"))
/// .item(BottomNavigationBarItem::new(Icons::SEARCH, "Search"))
/// .item(BottomNavigationBarItem::new(Icons::PERSON, "Profile"));
/// ```
///
/// [shifting]: BottomNavigationBarType::Shifting
pub struct BottomNavigationBar<Msg = Callback> {
    inner: Lazy<Parts, Msg>,
}

impl BottomNavigationBar<Callback> {
    /// A bar whose item `current_index` is selected, calling `on_tap` with the index of an
    /// item tapped. The selection is the caller's: the bar shows what it is told.
    pub fn new<R: crate::callback::IntoMsg<Callback>>(
        current_index: usize,
        on_tap: impl Fn(usize) -> R + 'static,
    ) -> Self {
        let on_tap = crate::callback::handler1(on_tap);
        Self {
            inner: Lazy::new(
                Parts {
                    items: Vec::new(),
                    current_index,
                    on_tap: Some(Rc::new(on_tap)),
                    elevation: None,
                    kind: None,
                    background_color: None,
                    icon_size: 24.0,
                    selected_item_color: None,
                    unselected_item_color: None,
                    selected_font_size: 14.0,
                    unselected_font_size: 12.0,
                    selected_label_style: None,
                    unselected_label_style: None,
                    show_selected_labels: None,
                    show_unselected_labels: None,
                    landscape_layout: None,
                    use_legacy_color_scheme: true,
                },
                build,
            ),
        }
    }

    fn p(&mut self) -> &mut Parts {
        self.inner.parts_mut()
    }

    /// Adds a destination.
    #[must_use]
    pub fn item(mut self, item: BottomNavigationBarItem) -> Self {
        self.p().items.push(item);
        self
    }

    /// Adds several destinations.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = BottomNavigationBarItem>) -> Self {
        self.p().items.extend(items);
        self
    }

    /// Which kind of bar. Unset, fixed for three items or fewer and shifting for more.
    #[must_use]
    pub fn kind(mut self, kind: BottomNavigationBarType) -> Self {
        self.p().kind = Some(kind);
        self
    }

    /// How far the bar sits off the page. 8 unless told.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.p().elevation = Some(elevation.max(0.0));
        self
    }

    /// The bar's colour, for a fixed bar. A shifting bar takes its selected item's.
    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.p().background_color = Some(color);
        self
    }

    /// The icons' size. 24 unless told.
    #[must_use]
    pub fn icon_size(mut self, size: f32) -> Self {
        self.p().icon_size = size.max(0.0);
        self
    }

    /// The selected item's colour, icon and label.
    #[must_use]
    pub fn selected_item_color(mut self, color: Color) -> Self {
        self.p().selected_item_color = Some(color);
        self
    }

    /// The other items' colour.
    #[must_use]
    pub fn unselected_item_color(mut self, color: Color) -> Self {
        self.p().unselected_item_color = Some(color);
        self
    }

    /// The selected label's size. 14 unless told; a selected label style's own size wins.
    #[must_use]
    pub fn selected_font_size(mut self, size: f32) -> Self {
        self.p().selected_font_size = size.max(0.0);
        self
    }

    /// The other labels' size. 12 unless told; an unselected label style's own size wins.
    #[must_use]
    pub fn unselected_font_size(mut self, size: f32) -> Self {
        self.p().unselected_font_size = size.max(0.0);
        self
    }

    /// The selected label's type.
    #[must_use]
    pub fn selected_label_style(mut self, style: TextStyle) -> Self {
        self.p().selected_label_style = Some(style);
        self
    }

    /// The other labels' type.
    #[must_use]
    pub fn unselected_label_style(mut self, style: TextStyle) -> Self {
        self.p().unselected_label_style = Some(style);
        self
    }

    /// Whether the selected item's label shows. It does unless told.
    #[must_use]
    pub fn show_selected_labels(mut self, show: bool) -> Self {
        self.p().show_selected_labels = Some(show);
        self
    }

    /// Whether the other items' labels show. On a fixed bar and not a shifting one, unless
    /// told.
    #[must_use]
    pub fn show_unselected_labels(mut self, show: bool) -> Self {
        self.p().show_unselected_labels = Some(show);
        self
    }

    /// How the bar lays out in landscape. Spread unless told.
    #[must_use]
    pub fn landscape_layout(mut self, layout: BottomNavigationBarLandscapeLayout) -> Self {
        self.p().landscape_layout = Some(layout);
        self
    }

    /// Whether the icons and labels take one colour between them (`true`, the default and
    /// the long-standing look), or the labels' styles and the shifting bar's selected
    /// colour each have their say (`false`).
    #[must_use]
    pub fn use_legacy_color_scheme(mut self, legacy: bool) -> Self {
        self.p().use_legacy_color_scheme = legacy;
        self
    }

    /// The bar, made now.
    pub fn build(self) -> Box<dyn Widget<Callback>> {
        self.inner.into_widget()
    }
}

impl<Msg> BottomNavigationBar<Msg> {
    /// Transparent: it is what it assembles.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(BottomNavigationBar {
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(&self, inherited: &crate::theme::Theme) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    fn scaffold_override(&self) -> Option<crate::ScaffoldInfo> {
        self.inner.scaffold_override()
    }
    fn scroll_behavior_override(&self) -> Option<crate::ScrollBehavior> {
        self.inner.scroll_behavior_override()
    }
    fn autofill_group(&self) -> bool {
        self.inner.autofill_group()
    }
    fn area_toolbar(
        &self,
        context: crate::ToolbarContext,
    ) -> Option<Option<&dyn crate::widget::Widget<Msg>>> {
        self.inner.area_toolbar(context)
    }
});

/// A splash spreading from a shifting bar's newly selected item.
struct Circle {
    index: usize,
    color: Color,
    spread: AnimationHandle,
}

/// What a bar keeps between frames: each item's selection progress, the colour the bar
/// has settled on, and the splashes still spreading.
struct BarState {
    selection: Vec<AnimationHandle>,
    current: usize,
    background: Option<Color>,
    circles: Vec<Circle>,
}

impl BarState {
    fn new(items: &[BottomNavigationBarItem], current: usize) -> Self {
        let selection = (0..items.len())
            .map(|index| {
                let mut controller = AnimationController::unit();
                if index == current {
                    controller.set_value(1.0);
                }
                AnimationHandle::new(controller)
            })
            .collect();
        Self {
            selection,
            current,
            background: items.get(current).and_then(|item| item.background_color),
            circles: Vec::new(),
        }
    }

    /// Follows the selection the bar is told: the old item goes, the new one comes, and a
    /// shifting bar starts a splash of the new item's colour. A splash that has spread
    /// becomes the bar's colour.
    fn follow(
        &mut self,
        items: &[BottomNavigationBarItem],
        current: usize,
        kind: BottomNavigationBarType,
    ) {
        if self.selection.len() != items.len() {
            *self = Self::new(items, current);
            return;
        }
        if current != self.current {
            if kind == BottomNavigationBarType::Shifting {
                if let Some(color) = items[current].background_color {
                    let spread = AnimationHandle::new(AnimationController::unit());
                    spread.forward(DURATION, curve());
                    self.circles.push(Circle {
                        index: current,
                        color,
                        spread,
                    });
                }
            }
            self.selection[self.current].reverse(DURATION, curve());
            self.selection[current].forward(DURATION, curve());
            self.current = current;
        } else if self.circles.is_empty() {
            self.background = items[current].background_color;
        }
        while let Some(first) = self.circles.first() {
            if first.spread.is_animating() || first.spread.value() < 1.0 {
                break;
            }
            self.background = Some(first.color);
            self.circles.remove(0);
        }
    }
}

/// A selected tile's share of a shifting bar's width: 1 at rest, 1.5 when selected.
fn flex_of(progress: f32) -> f32 {
    1.0 + 0.5 * progress
}

/// The top and bottom padding of a tile at `progress`, from which labels show — the room
/// a hidden label leaves goes above the icon, so the icons stay level as labels come and go.
fn tile_padding(
    progress: f32,
    font: f32,
    show_selected: bool,
    show_unselected: bool,
) -> (f32, f32) {
    let lerp = |a: f32, b: f32| a + (b - a) * progress;
    if show_selected && !show_unselected {
        (lerp(font, font / 2.0), lerp(0.0, font / 2.0))
    } else if !show_selected && !show_unselected {
        (font, 0.0)
    } else {
        (font / 2.0, font / 2.0)
    }
}

/// Where a splash starts across the bar, as a fraction of its width: the centre of its
/// item, the items weighed by their current flex.
fn splash_centre(flexes: &[f32], index: usize) -> f32 {
    let total: f32 = flexes.iter().sum();
    if total <= 0.0 {
        return 0.5;
    }
    let before: f32 = flexes[..index].iter().sum();
    (before + flexes[index] / 2.0) / total
}

fn icon_widget(icon: &DestinationIcon, size: f32, color: Color) -> Box<dyn Widget> {
    match icon {
        DestinationIcon::Icon(data) => Box::new(Icon::new(*data).size(size).color(color)),
        DestinationIcon::Glyph(glyph) => Box::new(Text::new(glyph.clone()).size(size).color(color)),
    }
}

/// The bar, from its parts: a component, so that it keeps its animations between frames.
fn build(parts: Parts) -> Box<dyn Widget> {
    let parts = Rc::new(parts);
    Box::new(component(move |cx| {
        let p = parts.clone();
        let theme = cx.theme();
        let t = theme.widgets.bottom_navigation_bar;
        let count = p.items.len();
        if count == 0 {
            return Box::new(SizedBox::empty());
        }
        let current = p.current_index.min(count - 1);
        let kind = p.kind.or(t.kind).unwrap_or(if count <= 3 {
            BottomNavigationBarType::Fixed
        } else {
            BottomNavigationBarType::Shifting
        });
        let state = cx.use_ref(|| BarState::new(&p.items, current));
        state.update(|s| s.follow(&p.items, current, kind));
        let progress: Vec<f32> = state.with(|s| s.selection.iter().map(|a| a.value()).collect());

        // The colours, as the reference resolves them: the caller's, the theme's, then the
        // kind's own.
        let dark = theme.brightness() == Brightness::Dark;
        let theme_color = if dark {
            theme.scheme.secondary
        } else {
            theme.scheme.primary
        };
        let unselected_widget = if dark {
            Color::rgba(1.0, 1.0, 1.0, 0.70)
        } else {
            Color::rgba(0.0, 0.0, 0.0, 0.54)
        };
        let selected = p.selected_item_color.or(t.selected_item_color);
        let unselected = p.unselected_item_color.or(t.unselected_item_color);
        let (from, to) = match kind {
            BottomNavigationBarType::Fixed => (
                unselected.unwrap_or(unselected_widget),
                selected.unwrap_or(theme_color),
            ),
            BottomNavigationBarType::Shifting => (
                unselected.unwrap_or(theme.scheme.surface),
                selected.unwrap_or(theme.scheme.surface),
            ),
        };
        let selected_style = p.selected_label_style.or(t.selected_label_style);
        let unselected_style = p.unselected_label_style.or(t.unselected_label_style);
        // Without the legacy scheme the labels' own colours come first, and a shifting
        // bar's selected label falls back on the theme's colour rather than `surface`.
        let (label_from, label_to) = if p.use_legacy_color_scheme {
            (from, to)
        } else {
            let to_default = match kind {
                BottomNavigationBarType::Fixed => to,
                BottomNavigationBarType::Shifting => selected.unwrap_or(theme_color),
            };
            (
                unselected_style.and_then(|s| s.color).unwrap_or(from),
                selected_style.and_then(|s| s.color).unwrap_or(to_default),
            )
        };
        let (icon_from, icon_to) = if p.use_legacy_color_scheme {
            (from, to)
        } else {
            match kind {
                BottomNavigationBarType::Fixed => (from, to),
                BottomNavigationBarType::Shifting => (from, selected.unwrap_or(theme_color)),
            }
        };
        let selected_font = selected_style
            .and_then(|s| s.size)
            .unwrap_or(p.selected_font_size);
        let unselected_font = unselected_style
            .and_then(|s| s.size)
            .unwrap_or(p.unselected_font_size);
        let show_selected = p
            .show_selected_labels
            .or(t.show_selected_labels)
            .unwrap_or(true);
        let show_unselected = p
            .show_unselected_labels
            .or(t.show_unselected_labels)
            .unwrap_or(kind == BottomNavigationBarType::Fixed);
        let layout = p
            .landscape_layout
            .or(t.landscape_layout)
            .unwrap_or_default();
        let media = MediaQuery::of();
        let landscape = media.orientation() == Orientation::Landscape;
        // What the scaffold told it about the system's bars: the colour runs behind them,
        // the items keep clear of them.
        let safe = media.padding;
        let bottom = safe.bottom;

        let flexes: Vec<f32> = progress
            .iter()
            .map(|&a| match kind {
                BottomNavigationBarType::Fixed => 1.0,
                BottomNavigationBarType::Shifting => flex_of(a),
            })
            .collect();

        let mut row = Row::new().main_axis_alignment(Justify::SpaceBetween);
        for (index, item) in p.items.iter().enumerate() {
            let a = progress[index];
            let is_selected = index == current;
            let (top, below) = tile_padding(a, selected_font, show_selected, show_unselected);
            let icon = if is_selected {
                item.active_icon.as_ref().unwrap_or(&item.icon)
            } else {
                &item.icon
            };
            let icon = icon_widget(icon, p.icon_size, icon_from.lerp(icon_to, a));

            // The label's size moves with the selection, from the unselected size to the
            // selected one — text is laid out at any size, so it grows smoothly and stays
            // sharp — in a line as tall as the selected size needs, so that the icons do
            // not move as it grows.
            let style = if is_selected {
                selected_style
            } else {
                unselected_style
            };
            let mut text = Text::new(item.label.clone())
                .size(unselected_font + (selected_font - unselected_font) * a)
                .color(label_from.lerp(label_to, a))
                .no_wrap()
                .ellipsis();
            if let Some(weight) = style.and_then(|s| s.weight) {
                text = text.weight(weight);
            }
            let label = SizedBox::new(Center::new(text))
                .height(selected_font * frus_core::DEFAULT_LINE_HEIGHT);
            let opacity = if !show_selected && !show_unselected {
                0.0
            } else if !show_unselected {
                a
            } else if !show_selected {
                1.0 - a
            } else {
                1.0
            };
            let label = Opacity::new(opacity, label);

            let content: Box<dyn Widget> =
                if landscape && layout == BottomNavigationBarLandscapeLayout::Linear {
                    Box::new(
                        Row::new()
                            .main_axis_size(MainAxisSize::Min)
                            .spacing(8.0)
                            .child(icon)
                            .child(Flexible::new(label)),
                    )
                } else {
                    Box::new(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .main_axis_alignment(Justify::SpaceBetween)
                            .child(icon)
                            .child(label),
                    )
                };
            let on_tap = p.on_tap.clone();
            // The label is announced whether or not it is drawn, with where the tab is
            // among the others.
            let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Tab)
                .label(format!(
                    "{}, {}",
                    item.label,
                    crate::localizations::of().tab_label(index + 1, count)
                ))
                .toggled(is_selected)
                .clickable();
            let mut tile: Box<dyn Widget> = Box::new(
                InkWell::new()
                    .semantics(semantics)
                    .on_click(Callback::new(move || {
                        if let Some(on_tap) = &on_tap {
                            on_tap(index).call();
                        }
                    }))
                    .child(Center::new(Padding::new(
                        Insets::new(top, 0.0, below, 0.0),
                        content,
                    ))),
            );
            if let Some(tooltip) = item.tooltip.as_ref().filter(|t| !t.is_empty()) {
                tile = Box::new(Tooltip::new(tooltip.clone()).child(tile));
            }
            row = row.child(Expanded::new(tile).flex(flexes[index]));
        }

        let content: Box<dyn Widget> =
            if landscape && layout == BottomNavigationBarLandscapeLayout::Centered {
                Box::new(Center::new(SizedBox::new(row).width(media.size.height)))
            } else {
                Box::new(row)
            };

        // The surface under the row: its shadow, its colour, and the splashes spreading.
        let elevation = p.elevation.or(t.elevation).unwrap_or(8.0);
        let base = match kind {
            BottomNavigationBarType::Fixed => p.background_color.or(t.background_color),
            BottomNavigationBarType::Shifting => state.with(|s| s.background),
        }
        .unwrap_or(theme.scheme.surface);
        let splashes: Vec<(f32, Color, f32)> = state.with(|s| {
            s.circles
                .iter()
                .map(|c| (splash_centre(&flexes, c.index), c.color, c.spread.value()))
                .collect()
        });
        let rtl = theme.direction == frus_core::TextDirection::Rtl;
        let shadow = theme.scheme.shadow;
        let surface = CustomPaint::new(0.0, 0.0, move |scene, bounds, _| {
            frus_core::paint_elevation(
                scene,
                bounds,
                frus_core::BorderRadius::uniform(0.0),
                elevation,
                shadow,
            );
            scene.fill_rect(bounds, base);
            let clip = scene.current_clip();
            scene.set_clip(clip.intersect(bounds));
            for &(fraction, color, spread) in &splashes {
                let fraction = if rtl { 1.0 - fraction } else { fraction };
                let centre = Point::new(
                    bounds.x + fraction * bounds.width,
                    bounds.y + bounds.height / 2.0,
                );
                let reach_x = (centre.x - bounds.x).max(bounds.x + bounds.width - centre.x);
                let reach_y = bounds.height / 2.0;
                let radius = (reach_x * reach_x + reach_y * reach_y).sqrt() * spread;
                scene.fill_path(&Path::circle(centre, radius), color);
            }
            scene.set_clip(clip);
        });

        Box::new(
            // The width it is offered, all of it: the tiles share it out.
            SizedBox::expand(
                Stack::new()
                    .fit(crate::StackFit::Expand)
                    // The shadow falls outside the bar.
                    .clip_behavior(crate::Clip::None)
                    .layer(Positioned::fill(surface))
                    .layer(Padding::new(
                        Insets::new(0.0, safe.right, bottom, safe.left),
                        content,
                    )),
            )
            .height(BOTTOM_NAVIGATION_BAR_HEIGHT + bottom),
        ) as Box<dyn Widget>
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selected_tile_takes_half_again_the_room() {
        assert_eq!(flex_of(0.0), 1.0);
        assert_eq!(flex_of(1.0), 1.5);
    }

    #[test]
    fn a_splash_starts_at_the_centre_of_its_item() {
        // Three items, the middle one selected and widened.
        let flexes = [1.0, 1.5, 1.0];
        assert!((splash_centre(&flexes, 1) - 0.5).abs() < 1e-6);
        assert!((splash_centre(&flexes, 0) - 0.5 / 3.5).abs() < 1e-6);
    }

    #[test]
    fn labels_that_only_show_when_selected_keep_the_icons_level() {
        // Unselected: the room a label would take goes above the icon.
        assert_eq!(tile_padding(0.0, 14.0, true, false), (14.0, 0.0));
        // Selected: shared above and below.
        assert_eq!(tile_padding(1.0, 14.0, true, false), (7.0, 7.0));
        // Every label shown: the same at rest and selected.
        assert_eq!(tile_padding(0.0, 14.0, true, true), (7.0, 7.0));
    }

    #[test]
    fn a_change_of_selection_moves_the_old_item_out_and_the_new_one_in() {
        let items = vec![
            BottomNavigationBarItem::new("A", "A").background_color(Color::rgb8(255, 0, 0)),
            BottomNavigationBarItem::new("B", "B").background_color(Color::rgb8(0, 0, 255)),
        ];
        let mut state = BarState::new(&items, 0);
        assert_eq!(state.selection[0].value(), 1.0);
        assert_eq!(state.background, Some(Color::rgb8(255, 0, 0)));
        state.follow(&items, 1, BottomNavigationBarType::Shifting);
        assert!(state.selection[0].is_animating() && state.selection[1].is_animating());
        assert_eq!(
            state.circles.len(),
            1,
            "a shifting bar splashes the new colour"
        );
        // Once the splash has spread, it is the bar's colour.
        state.circles[0].spread.set_value(1.0);
        state.follow(&items, 1, BottomNavigationBarType::Shifting);
        assert!(state.circles.is_empty());
        assert_eq!(state.background, Some(Color::rgb8(0, 0, 255)));
    }

    #[test]
    fn a_fixed_bar_does_not_splash() {
        let items = vec![
            BottomNavigationBarItem::new("A", "A").background_color(Color::rgb8(255, 0, 0)),
            BottomNavigationBarItem::new("B", "B"),
        ];
        let mut state = BarState::new(&items, 0);
        state.follow(&items, 1, BottomNavigationBarType::Fixed);
        assert!(state.circles.is_empty());
    }
}
