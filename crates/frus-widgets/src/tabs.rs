//! [`TabBar`]: a **controlled** tab bar plus the selected panel.
//!
//! It is composite: its children are `[bar, panel]`, in a column. Only the selected tab's
//! content is realised, the application rebuilding the view every frame.
//!
//! The bar is a **tab bar**, not a row of buttons: labels on the surface, a sliding
//! indicator under the selected one, and a hairline across the whole bar separating it from
//! the panel. The two variants are the reference's — a *primary* bar's indicator is as wide
//! as its **label** and rounded at the top, a *secondary* bar's spans the whole **tab** and
//! is square — and every measurement and colour in either is overridable, by the caller or
//! by [`crate::TabBarTheme`].

use frus_core::{BorderRadius, Color, Point, Rect, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::icons::Icons;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The height of the tabs themselves, indicator excluded.
pub const TAB_HEIGHT: f32 = 46.0;
/// The height of the tabs when **any** of them stacks an icon over a label. The
/// reference's second figure, and it applies to the whole row: tabs of two heights in
/// one bar would put their labels on two different lines.
pub const TAB_ICON_HEIGHT: f32 = 72.0;
/// The side of a tab's icon.
pub const TAB_ICON_SIZE: f32 = 24.0;
/// The room between a tab's icon and the label under it.
pub const TAB_ICON_GAP: f32 = 2.0;
/// The room on either side of a label.
pub const TAB_LABEL_PADDING: f32 = 16.0;
/// The inset [`TabAlignment::StartOffset`] leaves before the first tab.
pub const TAB_START_OFFSET: f32 = 52.0;
/// The thickness of a **primary** bar's indicator, which is also its corner radius.
pub const TAB_INDICATOR_PRIMARY: f32 = 3.0;
/// The thickness of a **secondary** bar's indicator.
pub const TAB_INDICATOR_SECONDARY: f32 = 2.0;
/// The hairline between the bar and the panel.
pub const TAB_DIVIDER_HEIGHT: f32 = 1.0;

/// Which of the two tab bars this is.
///
/// They differ in more than looks: a *primary* bar marks the selected tab with an indicator
/// as wide as its label, and a *secondary* bar with one as wide as the whole tab. A primary
/// bar is for the top level of a screen, a secondary one for a division inside it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TabBarVariant {
    /// Label-wide indicator, rounded at the top; the selected label takes the accent.
    #[default]
    Primary,
    /// Tab-wide indicator, square.
    Secondary,
}

/// Where the tabs sit in a bar with room to spare.
///
/// The question only arises when the tabs do not already fill the bar — which is either
/// because they were told not to share it, or because there are two of them in a window
/// meant for six.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TabAlignment {
    /// Whatever the bar's mode implies: [`Fill`](Self::Fill) when it shares its width,
    /// [`Start`](Self::Start) when it scrolls. The default, and the reference's.
    #[default]
    Auto,
    /// Each tab as wide as it needs, packed against the leading edge.
    Start,
    /// The same, after an inset of [`TAB_START_OFFSET`].
    StartOffset,
    /// The tabs share the bar's width in equal parts.
    ///
    /// Meaningless on a bar that **scrolls**, since sharing a width is the opposite of
    /// exceeding it, and it reads as [`Start`](Self::Start) there. The reference throws
    /// instead; a layout that throws is a crash on somebody's phone for a combination
    /// that has an obvious reading.
    Fill,
    /// Each tab as wide as it needs, the row centred in the bar.
    ///
    /// For a bar that does **not** scroll. Centring needs something definite to centre
    /// in, and the content of a scroll region has no definite width across the axis it
    /// scrolls along — the strip is measured by its own tabs, so a percentage of its
    /// parent resolves against nothing and it comes out exactly as wide as the tabs it
    /// holds, with no room left over to share. So it reads as [`Start`](Self::Start)
    /// there.
    ///
    /// The two are not really alternatives anyway: a bar whose tabs might overflow wants
    /// `Start` and a scroll, and a bar whose tabs certainly fit wants `Center` and no
    /// scroll at all.
    Center,
}

impl TabAlignment {
    /// What this means for a bar that does or does not scroll.
    ///
    /// The reference forbids [`Start`](Self::Start) and [`StartOffset`](Self::StartOffset)
    /// on a bar that shares its width, for reasons that are its own history rather than
    /// anything about the layout — packing natural-width tabs against the leading edge of
    /// a bar that does not scroll is a perfectly ordinary thing to want, and it is what
    /// they do here.
    fn resolved(self, scrolls: bool) -> Self {
        match (self, scrolls) {
            (Self::Auto, true) | (Self::Fill, true) | (Self::Center, true) => Self::Start,
            (Self::Auto, false) => Self::Fill,
            (other, _) => other,
        }
    }

    /// The inset before the first tab.
    fn lead(self) -> f32 {
        match self {
            Self::StartOffset => TAB_START_OFFSET,
            _ => 0.0,
        }
    }
}

/// One tab's content: what it says, what it shows, and which of the two is drawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TabSpec {
    /// The tab's text — drawn when `show_label`, and what a screen reader is told
    /// either way.
    pub label: String,
    /// The icon above the label, if any.
    pub icon: Option<Icons>,
    /// Whether the label is **drawn**. False for an icon-only tab, whose label is still
    /// its name as far as anything that cannot see is concerned. The reference leaves an
    /// icon-only tab nameless and expects the caller to wrap it; asking for the word up
    /// front costs nothing and cannot be forgotten.
    pub show_label: bool,
}

impl TabSpec {
    /// A tab that shows its text.
    fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            show_label: true,
        }
    }

    /// Does this tab stack an icon **over** a label? The question the row's height turns
    /// on: an icon on its own sits in an ordinary tab.
    fn stacked(&self) -> bool {
        self.icon.is_some() && self.show_label && !self.label.is_empty()
    }
}

/// Everything about a tab bar's appearance, each part `None` until someone says otherwise.
///
/// Held by the bar and by each tab, so that both measure the indicator the same way — the
/// bar draws it, the tabs are what it is measured against, and the two disagreeing would be
/// an indicator that does not line up with the label it points at.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TabStyle {
    pub variant: Option<TabBarVariant>,
    pub indicator_color: Option<Color>,
    pub indicator_weight: Option<f32>,
    pub label_color: Option<Color>,
    pub unselected_label_color: Option<Color>,
    pub label_style: Option<TextStyle>,
    pub divider_color: Option<Color>,
    pub divider_height: Option<f32>,
    pub label_padding: Option<f32>,
    pub tab_height: Option<f32>,
    /// Where the tabs sit when they do not fill the bar; see [`TabBar::alignment`].
    pub alignment: Option<TabAlignment>,
    /// Does any tab in this bar stack an icon over a label? Not an override but a fact
    /// about the row, recorded here because the bar and every tab must measure the same
    /// height and this is the one thing both already hold.
    pub tall: bool,
}

impl TabStyle {
    fn variant(&self, theme: &Theme) -> TabBarVariant {
        self.variant
            .or(theme.widgets.tab_bar.variant)
            .unwrap_or_default()
    }

    fn indicator_weight(&self, theme: &Theme) -> f32 {
        self.indicator_weight
            .or(theme.widgets.tab_bar.indicator_weight)
            .unwrap_or(match self.variant(theme) {
                TabBarVariant::Primary => TAB_INDICATOR_PRIMARY,
                TabBarVariant::Secondary => TAB_INDICATOR_SECONDARY,
            })
    }

    fn indicator_color(&self, theme: &Theme) -> Color {
        self.indicator_color
            .or(theme.widgets.tab_bar.indicator_color)
            .unwrap_or(theme.scheme.primary)
    }

    fn label_color(&self, theme: &Theme) -> Color {
        self.label_color
            .or(theme.widgets.tab_bar.label_color)
            .unwrap_or(match self.variant(theme) {
                // A primary bar's selected label takes the accent; a secondary bar's stays
                // on the surface, the indicator alone marking it.
                TabBarVariant::Primary => theme.scheme.primary,
                TabBarVariant::Secondary => theme.scheme.on_surface,
            })
    }

    fn unselected_label_color(&self, theme: &Theme) -> Color {
        self.unselected_label_color
            .or(theme.widgets.tab_bar.unselected_label_color)
            .unwrap_or(theme.scheme.on_surface_variant)
    }

    fn label_style(&self, theme: &Theme) -> TextStyle {
        self.label_style
            .or(theme.widgets.tab_bar.label_style)
            .unwrap_or(theme.text.title_small)
    }

    fn divider_color(&self, theme: &Theme) -> Color {
        self.divider_color
            .or(theme.widgets.tab_bar.divider_color)
            .unwrap_or(theme.scheme.outline_variant)
    }

    fn divider_height(&self, theme: &Theme) -> f32 {
        self.divider_height
            .or(theme.widgets.tab_bar.divider_height)
            .unwrap_or(TAB_DIVIDER_HEIGHT)
    }

    fn label_padding(&self, theme: &Theme) -> f32 {
        self.label_padding
            .or(theme.widgets.tab_bar.label_padding)
            .unwrap_or(TAB_LABEL_PADDING)
    }

    /// Where the tabs sit, resolved against what the bar does.
    fn alignment(&self, theme: &Theme, scrolls: bool) -> TabAlignment {
        self.alignment
            .or(theme.widgets.tab_bar.alignment)
            .unwrap_or_default()
            .resolved(scrolls)
    }

    fn tab_height(&self, theme: &Theme) -> f32 {
        self.tab_height
            .or(theme.widgets.tab_bar.tab_height)
            .unwrap_or(match self.tall {
                true => TAB_ICON_HEIGHT,
                false => TAB_HEIGHT,
            })
    }

    /// The **whole bar**'s height: the tabs, plus the indicator sitting under them.
    fn bar_height(&self, theme: &Theme) -> f32 {
        self.tab_height(theme) + self.indicator_weight(theme)
    }

    /// The width the indicator takes under the tab holding `label`, within a tab `tab_width`
    /// wide. A primary bar measures the label; a secondary one takes the tab.
    /// How wide one tab is when the bar **scrolls**: its label, plus the label's own
    /// room either side.
    ///
    /// The tab lays itself out to this number and the indicator is placed by it, which
    /// is the point of there being one function. A tab that measured itself one way and
    /// an indicator that measured it another would agree on every label until they did
    /// not, and the failure would be an underline creeping away from its tab as the bar
    /// filled up.
    fn scrolled_tab_width(&self, theme: &Theme, spec: &TabSpec) -> f32 {
        self.content_width(theme, spec) + 2.0 * self.label_padding(theme)
    }

    /// How wide a tab's **content** is: its text, its icon, or whichever of the two is
    /// wider when it has both, since they are stacked rather than side by side.
    ///
    /// One function, and both the tab's width and the indicator's come from it. A tab
    /// measured one way and an indicator measured another would agree on every label
    /// until they did not.
    fn content_width(&self, theme: &Theme, spec: &TabSpec) -> f32 {
        let text = match spec.show_label && !spec.label.is_empty() {
            true => {
                let style = self.label_style(theme);
                frus_text::measure_style(&spec.label, style).width
            }
            false => 0.0,
        };
        let icon = match spec.icon {
            Some(_) => TAB_ICON_SIZE,
            None => 0.0,
        };
        text.max(icon)
    }

    /// Where each tab starts and how wide it is, in the bar's own coordinates.
    ///
    /// Not scrollable, the tabs share the width in equal parts — the reference's rule,
    /// and the one that keeps a tab still when another is renamed. Scrollable, each takes
    /// the room its label needs.
    fn tab_spans(
        &self,
        theme: &Theme,
        tabs: &[TabSpec],
        width: f32,
        alignment: TabAlignment,
    ) -> Vec<(f32, f32)> {
        if alignment == TabAlignment::Fill {
            let each = width / tabs.len().max(1) as f32;
            return (0..tabs.len()).map(|i| (i as f32 * each, each)).collect();
        }
        let widths: Vec<f32> = tabs
            .iter()
            .map(|spec| self.scrolled_tab_width(theme, spec))
            .collect();
        // Centred, the row's own leading inset is half of whatever is left over — and
        // nothing when there is nothing left over, which is the case that matters: a
        // scrolling bar's strip is as wide as its tabs, so the two agree at zero rather
        // than the indicator drifting off by half an overflow.
        let lead = match alignment {
            TabAlignment::Center => (width - widths.iter().sum::<f32>()) / 2.0,
            other => other.lead(),
        }
        .max(0.0);
        let mut spans = Vec::with_capacity(tabs.len());
        let mut x = lead;
        for w in widths {
            spans.push((x, w));
            x += w;
        }
        spans
    }

    fn indicator_width(&self, theme: &Theme, spec: &TabSpec, tab_width: f32) -> f32 {
        match self.variant(theme) {
            TabBarVariant::Secondary => tab_width,
            TabBarVariant::Primary => {
                // Never past the content's own room: an indicator running into the next
                // tab's would point at two labels at once.
                let room = (tab_width - 2.0 * self.label_padding(theme)).max(0.0);
                self.content_width(theme, spec).min(room)
            }
        }
    }
}

/// One tab: a label, a tap, and the ink a tap leaves.
struct Tab<Msg> {
    spec: TabSpec,
    selected: bool,
    style: TabStyle,
    /// The bar's availability, handed down to every tab.
    enabled: bool,
    /// Whether the bar scrolls. Half of the question "does this tab share the bar's
    /// width"; the other half is the [`TabAlignment`], which only the theme can settle.
    scrolls: bool,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Tab<Msg> {
    fn style(&self) -> Style {
        self.style_themed(&Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        // A zero basis with equal growth: the tabs share the bar's width in equal parts,
        // which is what the reference does with a bar that is not scrollable. Sharing it in
        // proportion to the labels instead would move every tab whenever one was renamed.
        // The padding is the label's own room, and the smallest a tab will agree to be.
        let pad = self.style.label_padding(theme);
        // Asked of the **theme**, here rather than once at build time: an application
        // that sets the alignment on its theme sets it for tabs that were built before
        // the theme was ever consulted.
        if self.style.alignment(theme, self.scrolls) == TabAlignment::Fill {
            return Style {
                width: Dimension::Length(0.0),
                flex_grow: 1.0,
                padding: frus_core::Insets::new(0.0, pad, 0.0, pad),
                ..Default::default()
            };
        }
        // Otherwise the tab takes the room its content needs and no more, which is the
        // whole difference — eight tabs on a phone stop being eight crushed columns and
        // become eight readable ones the bar scrolls through.
        //
        // The width is **stated**, not left to the content, because a tab paints its own
        // label rather than holding a `Text` child: there is nothing here for the layout
        // engine to measure. Stating it also makes the tab and the indicator agree by
        // construction, since both ask the same function.
        Style {
            width: Dimension::Length(self.style.scrolled_tab_width(theme, &self.spec)),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            padding: frus_core::Insets::new(0.0, pad, 0.0, pad),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let label_style = self.style.label_style(theme);
        // A label is content whichever tab it is on, so both the selected and the
        // unselected one flatten to the same grey. Fading the accent instead would read as
        // *quietly selected* rather than as unavailable.
        let color = if !self.enabled {
            disabled_content(theme)
        } else if self.selected {
            self.style.label_color(theme)
        } else {
            self.style.unselected_label_color(theme)
        };
        let shows_label = self.spec.show_label && !self.spec.label.is_empty();
        let measured = match shows_label {
            true => frus_text::measure_style(&self.spec.label, label_style),
            false => frus_core::Size::new(0.0, 0.0),
        };
        // Centred across the tab, and centred in the **tabs' own** height rather than the
        // bar's: the indicator's weight belongs to the bar, not to the row of labels, so
        // counting it here would push every label a pixel and a half down.
        let height = self.style.tab_height(theme);
        // Icon over label, as one block, centred as one block. Measuring the two
        // separately and centring each would leave the pair looking untidy in a row
        // where one tab has an icon and its neighbour does not.
        let icon_block = match self.spec.icon {
            Some(_) if shows_label => TAB_ICON_SIZE + TAB_ICON_GAP,
            Some(_) => TAB_ICON_SIZE,
            None => 0.0,
        };
        let block = icon_block + measured.height * f32::from(u8::from(shows_label));
        let top = bounds.y + (height - block) / 2.0;
        if let Some(icon) = self.spec.icon {
            // The 24x24 grid, unscaled, centred across the tab.
            let path = icon
                .path()
                .translated(bounds.x + (bounds.width - TAB_ICON_SIZE) / 2.0, top);
            scene.fill_path(&path, color.fade(o));
        }
        if shows_label {
            scene.text(
                Point::new(
                    bounds.x + (bounds.width - measured.width) / 2.0,
                    top + icon_block,
                ),
                self.spec.label.clone(),
                &label_style.resolved(),
                color.fade(o),
            );
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        Some(self.message.clone())
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        // A disabled tab does not answer a tap, so it does not splash either.
        if !self.enabled {
            return None;
        }
        // The splash follows the label: the accent under a selected tab, the surface's own
        // ink under the others — the reference's overlay, which changes with the state
        // rather than being one colour for the bar.
        let splash = if self.selected {
            self.style.indicator_color(theme).fade(0.10)
        } else {
            theme.scheme.on_surface.fade(0.08)
        };
        Some(crate::InkStyle::of(theme).color(splash).radius(0.0))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // Which tab is showing survives: a reader who cannot switch is still owed where
        // they are.
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Tab)
            .label(self.spec.label.clone())
            .toggled(self.selected);
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// The bar itself: the tabs in a row, the hairline under them, and the indicator that
/// slides from one to the next.
struct TabStrip<Msg> {
    selected: usize,
    tabs: Vec<TabSpec>,
    style: TabStyle,
    /// Whether the bar scrolls: it decides both the strip's width and where the
    /// indicator goes.
    scrolls: bool,
    /// The bar's availability. The **indicator** is the part that would otherwise keep the
    /// accent on a disabled bar: milestone 324's golden showed exactly that, flattened
    /// labels above a bar still painted in the accent colour.
    enabled: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for TabStrip<Msg> {
    fn style(&self) -> Style {
        self.style_themed(&Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        // Centring needs something to centre **in**: the strip takes the bar's whole
        // width, and the row is placed inside it. Only reachable on a bar that does not
        // scroll, for exactly that reason — see [`TabAlignment::Center`].
        let alignment = self.style.alignment(theme, self.scrolls);
        let centred = alignment == TabAlignment::Center;
        Style {
            // Scrollable, the strip is as wide as its tabs, which is what gives the
            // scroll something to scroll through.
            width: match self.scrolls && !centred {
                true => Dimension::Auto,
                false => Dimension::Percent(1.0),
            },
            height: Dimension::Length(self.style.bar_height(theme)),
            flex_direction: FlexDirection::Row,
            justify: match centred {
                true => frus_layout::Justify::Center,
                false => frus_layout::Justify::Start,
            },
            padding: frus_core::Insets::new(0.0, 0.0, 0.0, alignment.lead()),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    /// The selected index, handed to the runtime so that it arrives here as a **fractional**
    /// index while the selection moves: that is what the indicator slides along.
    fn anim_target(&self) -> Option<f32> {
        Some(self.selected as f32)
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let count = self.tabs.len();
        if count == 0 || bounds.width <= 0.0 {
            return;
        }
        let o = status.opacity;
        let weight = self.style.indicator_weight(theme);

        // The hairline is **not** here. It divides the bar from the panel and belongs to
        // neither tab -- and once the strip can scroll, drawing it here would send it
        // sliding away with the labels and leave a scrollable bar with two tabs ruling a
        // stub instead of a line. `TabBar` draws it, across its own width, which is the
        // width it always meant.
        if weight <= 0.0 {
            return;
        }

        // Where the indicator is, which is generally **between** two tabs: the runtime
        // tweens the selected index, so this is a fractional position and the indicator
        // slides rather than jumps. Its width travels with it, so an indicator moving
        // between a short label and a long one grows on the way.
        let spans = self.style.tab_spans(
            theme,
            &self.tabs,
            bounds.width,
            self.style.alignment(theme, self.scrolls),
        );
        let last = (count - 1) as f32;
        let value = status.value.clamp(0.0, last);
        let (low, high) = (value.floor(), value.ceil());
        let t = value - low;
        let span = |index: f32| {
            let i = index as usize;
            let (start, tab_width) = spans[i];
            let centre = bounds.x + start + tab_width / 2.0;
            let width = self.style.indicator_width(theme, &self.tabs[i], tab_width);
            (centre, width)
        };
        let (centre_low, width_low) = span(low);
        let (centre_high, width_high) = span(high);
        let centre = centre_low + (centre_high - centre_low) * t;
        let width = width_low + (width_high - width_low) * t;

        // It sits **on** the hairline, at the bar's bottom edge, rounded at the top for a
        // primary bar only — square where it spans the whole tab, since a rounded end
        // touching the next tab's would read as a gap that is not there.
        let radius = match self.style.variant(theme) {
            TabBarVariant::Primary => BorderRadius {
                top_left: weight,
                top_right: weight,
                ..BorderRadius::ZERO
            },
            TabBarVariant::Secondary => BorderRadius::ZERO,
        };
        scene.draw_rect(
            Rect::new(
                centre - width / 2.0,
                bounds.y + bounds.height - weight,
                width,
                weight,
            ),
            if self.enabled {
                self.style.indicator_color(theme)
            } else {
                disabled_content(theme)
            }
            .fade(o),
            radius,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// The selected tab, so a bar wider than its window slides to it.
    ///
    /// Only when the bar **scrolls**. A bar that shares its width has every tab on
    /// screen already, and the region it would ask does not exist.
    ///
    /// Centred rather than merely brought in, which is the reference's behaviour and the
    /// readable one: a selected tab flush against the edge reads as the end of the row
    /// when it is only the edge of the window. The clamp keeps the first and last tabs
    /// where they belong.
    fn keep_visible(&self, size: frus_core::Size, theme: &Theme) -> Option<crate::ui::KeepVisible> {
        if !self.scrolls {
            return None;
        }
        let spans = self.style.tab_spans(
            theme,
            &self.tabs,
            size.width,
            self.style.alignment(theme, self.scrolls),
        );
        let &(x, width) = spans.get(self.selected)?;
        Some(crate::ui::KeepVisible {
            key: self.selected as u64,
            rect: Rect::new(x, 0.0, width, size.height),
            centre: true,
        })
    }
}

/// A tabbed view: the bar, and the selected tab's panel under it.
pub struct TabBar<Msg> {
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Msg>,
    tabs: Vec<TabSpec>,
    enabled: bool,
    style: TabStyle,
    /// Whether the bar scrolls rather than sharing its width; see [`TabBar::scrollable`].
    scrollable: bool,
    /// Either `[bar]` or `[bar, panel]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> TabBar<Msg> {
    /// Creates tabs: `selected` is the active index, `on_select(i)` the message on click.
    pub fn new(selected: usize, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let mut tabs = Self {
            selected,
            on_select: Box::new(on_select),
            tabs: Vec::new(),
            enabled: true,
            style: TabStyle::default(),
            scrollable: false,
            children: Vec::new(),
        };
        tabs.rebuild_bar();
        tabs
    }

    /// Adds a tab, a label plus content. The content is realised only when it belongs
    /// to the selected tab.
    pub fn tab(mut self, label: impl Into<String>, content: impl Widget<Msg> + 'static) -> Self {
        self.push(TabSpec::text(label), content);
        self
    }

    /// Adds a tab with an **icon over its label**.
    ///
    /// The whole row grows to [`TAB_ICON_HEIGHT`] as soon as one tab does this, because
    /// tabs of two heights in one bar would put their labels on two different lines.
    pub fn icon_tab(
        mut self,
        icon: Icons,
        label: impl Into<String>,
        content: impl Widget<Msg> + 'static,
    ) -> Self {
        self.push(
            TabSpec {
                label: label.into(),
                icon: Some(icon),
                show_label: true,
            },
            content,
        );
        self
    }

    /// Adds a tab showing **only** an icon, in a row of the ordinary height.
    ///
    /// The label is not drawn, and it is not optional: it is what a screen reader says,
    /// and a tab nobody can name is a tab nobody can use. The reference leaves an
    /// icon-only tab nameless and expects the caller to wrap it in something that names
    /// it; asking for the word here costs nothing and cannot be forgotten.
    pub fn icon_only_tab(
        mut self,
        icon: Icons,
        label: impl Into<String>,
        content: impl Widget<Msg> + 'static,
    ) -> Self {
        self.push(
            TabSpec {
                label: label.into(),
                icon: Some(icon),
                show_label: false,
            },
            content,
        );
        self
    }

    /// Adds `spec` to the row, keeping `content` only when it is the selected tab's.
    fn push(&mut self, spec: TabSpec, content: impl Widget<Msg> + 'static) {
        let index = self.tabs.len();
        self.tabs.push(spec);
        self.rebuild_bar();
        if index == self.selected {
            if self.children.len() > 1 {
                self.children[1] = Box::new(content);
            } else {
                self.children.push(Box::new(content));
            }
        }
    }

    /// Whether the bar can be switched. Disabled every tab is **inert** - no press, no
    /// ink, out of the tab order - and the **panel stays**, because which tab is showing
    /// is still the answer even when it cannot be changed.
    ///
    /// See [`crate::disabled`] for the whole contract.
    /// Lets the bar **scroll** rather than sharing its width between the tabs.
    ///
    /// Off by default, which is the reference's default and the right one for the two or
    /// three tabs most bars have: equal columns, and a tab that stays where it is when
    /// another is renamed.
    ///
    /// On, each tab takes the room its label needs and the bar scrolls through them. That
    /// is what eight tabs on a phone want — the alternative is eight columns of forty-odd
    /// pixels, which is not a tab bar with more tabs, it is a tab bar with none you can
    /// read.
    ///
    /// The indicator follows the tabs at their real widths, and the hairline still runs
    /// the whole foot of the bar even when the tabs do not fill it.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self.rebuild_bar();
        self
    }

    /// Where the tabs sit when they do not fill the bar.
    ///
    /// [`Auto`](TabAlignment::Auto) — the default — is what the bar's mode implies:
    /// shared width when it does not scroll, packed at the leading edge when it does.
    pub fn alignment(mut self, alignment: TabAlignment) -> Self {
        self.style.alignment = Some(alignment);
        self.rebuild_bar();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild_bar();
        self
    }

    /// Chooses between the two bars. See [`TabBarVariant`].
    pub fn variant(mut self, variant: TabBarVariant) -> Self {
        self.style.variant = Some(variant);
        self.rebuild_bar();
        self
    }

    /// The indicator's colour. Defaults to the theme's accent.
    pub fn indicator_color(mut self, color: Color) -> Self {
        self.style.indicator_color = Some(color);
        self.rebuild_bar();
        self
    }

    /// The indicator's thickness — and, on a primary bar, the radius of its top corners.
    /// `0.0` removes it.
    pub fn indicator_weight(mut self, weight: f32) -> Self {
        self.style.indicator_weight = Some(weight);
        self.rebuild_bar();
        self
    }

    /// The **selected** label's colour.
    pub fn label_color(mut self, color: Color) -> Self {
        self.style.label_color = Some(color);
        self.rebuild_bar();
        self
    }

    /// The colour of every label but the selected one.
    pub fn unselected_label_color(mut self, color: Color) -> Self {
        self.style.unselected_label_color = Some(color);
        self.rebuild_bar();
        self
    }

    /// The labels' type. Defaults to the theme's `title_small` step.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.style.label_style = Some(style);
        self.rebuild_bar();
        self
    }

    /// The hairline's colour.
    pub fn divider_color(mut self, color: Color) -> Self {
        self.style.divider_color = Some(color);
        self.rebuild_bar();
        self
    }

    /// The hairline's thickness. `0.0` removes it — a bar sitting directly on a coloured
    /// surface usually wants that.
    pub fn divider_height(mut self, height: f32) -> Self {
        self.style.divider_height = Some(height);
        self.rebuild_bar();
        self
    }

    /// The room on either side of a label.
    pub fn label_padding(mut self, padding: f32) -> Self {
        self.style.label_padding = Some(padding);
        self.rebuild_bar();
        self
    }

    /// The tabs' height, the indicator excluded.
    pub fn tab_height(mut self, height: f32) -> Self {
        self.style.tab_height = Some(height);
        self.rebuild_bar();
        self
    }

    /// (Re)builds the bar at index 0.
    ///
    /// Every builder calls this, because the bar carries the style and a builder run after
    /// `tab(…)` would otherwise be written into a bar that had already been built — the
    /// order of a builder chain is not something a caller should have to think about.
    fn rebuild_bar(&mut self) {
        // One tall tab makes the whole row tall. Recorded on the style, which the bar and
        // every tab already share, so the height cannot be answered two ways.
        self.style.tall = self.tabs.iter().any(TabSpec::stacked);
        let tabs: Vec<Box<dyn Widget<Msg>>> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, spec)| {
                Box::new(Tab {
                    spec: spec.clone(),
                    selected: i == self.selected,
                    style: self.style,
                    enabled: self.enabled,
                    scrolls: self.scrollable,
                    message: (self.on_select)(i),
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
        let strip = TabStrip {
            selected: self.selected,
            tabs: self.tabs.clone(),
            style: self.style,
            enabled: self.enabled,
            scrolls: self.scrollable,
            children: tabs,
        };
        // Scrollable, the strip goes inside a horizontal scroll -- and inside nothing
        // else, because the scroll is the only difference. Everything that makes the bar
        // a bar (the indicator, the hairline, the ink, the tap) stays in the strip and
        // does not know it is being scrolled.
        let bar: Box<dyn Widget<Msg>> = match self.scrollable {
            true => Box::new(
                crate::SingleChildScrollView::new()
                    .axis(crate::scroll::Axis::Horizontal)
                    .child(strip),
            ),
            false => Box::new(strip),
        };
        if self.children.is_empty() {
            self.children.push(bar);
        } else {
            self.children[0] = bar;
        }
    }
}

impl<Msg: Clone> Widget<Msg> for TabBar<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            // No gap: the hairline at the bar's foot is what separates it from the panel,
            // and a gap on top of it would leave the line floating.
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    /// A tab set takes the **width it is offered**, as the reference's does. Sized by its
    /// content instead, the widest thing on the busiest tab decides how wide the whole
    /// control is — so the bar jumps from tab to tab, and a panel that does not fit hangs
    /// out of whatever is centring it rather than being told to fit.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    /// The hairline at the foot of the bar.
    ///
    /// Drawn here rather than by the strip because it belongs to the bar: it runs the
    /// bar's whole width whatever the tabs do, and a scrollable strip would carry it off
    /// the side. The measurement is the same one the strip uses for its own height, so
    /// the line lands exactly where it always did.
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let divider = self.style.divider_height(theme);
        if divider <= 0.0 || bounds.width <= 0.0 {
            return;
        }
        scene.fill_rect(
            Rect::new(
                bounds.x,
                bounds.y + self.style.bar_height(theme) - divider,
                bounds.width,
                divider,
            ),
            self.style.divider_color(theme).fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::Icons;
    use crate::runtime::Runtime;
    use crate::ui::build_ui;
    use crate::Text;
    use frus_core::{Primitive, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Select(usize),
    }

    /// Ten tabs in a 300 px window: far more than fits, which is what `scrollable` is
    /// for and where the selected tab can go missing.
    fn wide(selected: usize) -> TabBar<Msg> {
        let mut tabs = TabBar::new(selected, Msg::Select).scrollable(true);
        for i in 0..10 {
            tabs = tabs.tab(format!("Section {i}"), Text::new("panel"));
        }
        tabs
    }

    fn frame(tabs: TabBar<Msg>, runtime: &Runtime) -> crate::Ui<Msg> {
        build_ui(&tabs, Size::new(300.0, 400.0), runtime, &Theme::default())
    }

    /// Where a label was drawn, or `None` if it was not.
    fn label_x(ui: &crate::Ui<Msg>, label: &str) -> Option<f32> {
        ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Text { position, text, .. } if text == label => Some(position.x),
            _ => None,
        })
    }

    /// A bar wider than its window **opens** on the selected tab. Before this, an
    /// application restored on its ninth tab showed the first three and nothing said
    /// where the selection had gone.
    #[test]
    fn a_scrolling_bar_opens_on_its_selected_tab() {
        let runtime = Runtime::default();
        let ui = frame(wide(8), &runtime);
        let area = ui.scroll_regions()[0];
        let keep = area
            .keep_visible
            .expect("the bar keeps its selected tab in view");
        assert_eq!(keep.key, 8);
        assert!(keep.centre, "centred, as the reference's bar is");

        // And it is really on screen: the ninth label is drawn inside the window.
        let x = label_x(&ui, "Section 8").expect("the selected tab is drawn");
        assert!(x > 0.0 && x < 300.0, "Section 8 drawn at {x}");
        // The first one is off the leading edge, which is the point.
        assert!(label_x(&ui, "Section 0").is_none_or(|x| x < 0.0));
    }

    /// A bar that shares its width has every tab on screen already, and asks for
    /// nothing: there is no region to ask.
    #[test]
    fn a_bar_that_shares_its_width_asks_for_nothing() {
        let runtime = Runtime::default();
        let mut tabs = TabBar::new(8, Msg::Select);
        for i in 0..10 {
            tabs = tabs.tab(format!("Section {i}"), Text::new("panel"));
        }
        assert!(frame(tabs, &runtime).scroll_regions().is_empty());
    }

    /// Changing the selection **glides**: a target, not a jump, so the bar reads as one
    /// movement rather than a cut.
    #[test]
    fn changing_the_tab_slides_the_bar() {
        let mut runtime = Runtime::default();
        let regions = frame(wide(0), &runtime).scroll_regions().to_vec();
        let id = regions[0].id;
        // The first sighting is where the bar opens, and it opens on tab 0, at the start.
        runtime.sync_visible(&regions);
        assert_eq!(runtime.scroll[&id], (0.0, 0.0));
        assert!(!runtime.scroll_target.contains_key(&id));

        let regions = frame(wide(8), &runtime).scroll_regions().to_vec();
        runtime.sync_visible(&regions);
        let target = runtime.scroll_target[&id];
        assert!(
            target.0 > 100.0,
            "glides across to the ninth tab: {target:?}"
        );
        assert_eq!(runtime.scroll[&id], (0.0, 0.0), "a target, not a jump");
    }

    /// The same selection twice does not pull the bar back: the reader is allowed to
    /// look at the other tabs. A region chasing the box every frame would pin the strip
    /// in place and no finger could move it.
    #[test]
    fn the_same_tab_twice_does_not_fight_the_finger() {
        let mut runtime = Runtime::default();
        let regions = frame(wide(3), &runtime).scroll_regions().to_vec();
        let id = regions[0].id;
        runtime.sync_visible(&regions);
        runtime.scroll_target.remove(&id);

        // The reader drags the bar somewhere else.
        runtime.scroll.insert(id, (500.0, 0.0));
        let regions = frame(wide(3), &runtime).scroll_regions().to_vec();
        runtime.sync_visible(&regions);
        assert!(
            !runtime.scroll_target.contains_key(&id),
            "nothing pulls it back"
        );
        assert_eq!(runtime.scroll[&id], (500.0, 0.0));
    }

    /// The bar's own height, as the hairline it draws across its foot reveals it.
    fn drawn_bar_height(tabs: TabBar<Msg>) -> f32 {
        let root = crate::flex::Flex::column()
            .width(400.0)
            .height(300.0)
            .child(tabs);
        let mut runtime = Runtime::default();
        runtime.advance_values::<Msg>(&root, 0.0);
        build_ui(&root, Size::new(400.0, 300.0), &runtime, &Theme::default())
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                // The hairline: the bar's full width, one pixel tall.
                Primitive::Rect { rect, .. } if rect.width > 399.0 && rect.height < 2.0 => {
                    Some(rect.y + rect.height)
                }
                _ => None,
            })
            .next()
            .expect("the hairline is drawn")
    }

    /// What the frame drew for a bar: the paths (icons) and the texts (labels).
    fn drawn(tabs: TabBar<Msg>) -> (usize, Vec<String>) {
        let root = crate::flex::Flex::column()
            .width(400.0)
            .height(300.0)
            .child(tabs);
        let mut runtime = Runtime::default();
        runtime.advance_values::<Msg>(&root, 0.0);
        let ui = build_ui(&root, Size::new(400.0, 300.0), &runtime, &Theme::default());
        let mut paths = 0;
        let mut texts = Vec::new();
        for p in ui.scene().primitives() {
            match p {
                Primitive::Path { .. } => paths += 1,
                Primitive::Text { text, .. } => texts.push(text.clone()),
                _ => {}
            }
        }
        (paths, texts)
    }

    /// An icon over a label draws both, and the row grows to the taller measurement.
    #[test]
    fn an_icon_tab_draws_its_icon_and_its_label() {
        let tabs = TabBar::new(0, Msg::Select)
            .icon_tab(Icons::Star, "Home", Text::new("panel"))
            .icon_tab(Icons::Heart, "Search", Text::new("panel"));
        let (paths, texts) = drawn(tabs);
        assert_eq!(paths, 2, "one icon per tab");
        assert!(texts.contains(&"Home".to_string()));
        assert!(texts.contains(&"Search".to_string()));
    }

    /// One tall tab makes the **whole row** tall. Two heights in one bar would put the
    /// labels on two different lines.
    #[test]
    fn one_icon_tab_raises_the_whole_row() {
        let plain = TabBar::new(0, Msg::Select)
            .tab("One", Text::new("panel"))
            .tab("Two", Text::new("panel"));
        let mixed = TabBar::new(0, Msg::Select)
            .tab("One", Text::new("panel"))
            .icon_tab(Icons::Heart, "Two", Text::new("panel"));
        let short = drawn_bar_height(plain);
        let tall = drawn_bar_height(mixed);
        assert!(
            (short - (TAB_HEIGHT + TAB_INDICATOR_PRIMARY)).abs() < 0.5,
            "{short}"
        );
        assert!(
            (tall - (TAB_ICON_HEIGHT + TAB_INDICATOR_PRIMARY)).abs() < 0.5,
            "{tall}"
        );
    }

    /// An icon **on its own** stays in an ordinary row, and its label is not drawn —
    /// but it is still what a reader is told.
    #[test]
    fn an_icon_only_tab_is_short_and_still_named() {
        let tabs = TabBar::new(0, Msg::Select)
            .icon_only_tab(Icons::Star, "Home", Text::new("panel"))
            .icon_only_tab(Icons::Heart, "Search", Text::new("panel"));
        let (paths, texts) = drawn(tabs);
        assert_eq!(paths, 2, "the icons");
        assert!(!texts.contains(&"Home".to_string()), "no label is drawn");

        let short = drawn_bar_height(TabBar::new(0, Msg::Select).icon_only_tab(
            Icons::Star,
            "Home",
            Text::new("panel"),
        ));
        assert!(
            (short - (TAB_HEIGHT + TAB_INDICATOR_PRIMARY)).abs() < 0.5,
            "{short}"
        );

        // The word survives where it matters.
        let root = crate::flex::Flex::column()
            .width(400.0)
            .height(300.0)
            .child(TabBar::new(0, Msg::Select).icon_only_tab(
                Icons::Star,
                "Home",
                Text::new("panel"),
            ));
        let ui = build_ui(
            &root,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(
            ui.semantics()
                .iter()
                .any(|(_, _, s)| s.label.as_deref() == Some("Home")),
            "the icon-only tab still has a name"
        );
    }

    /// A scrolling bar measures an icon tab by whichever of its two parts is wider,
    /// since they are stacked rather than side by side.
    #[test]
    fn a_stacked_tab_is_as_wide_as_its_widest_part() {
        let style = TabStyle::default();
        let theme = Theme::default();
        let narrow = TabSpec {
            label: "Hi".into(),
            icon: Some(Icons::Star),
            show_label: true,
        };
        // "Hi" is narrower than a 24 px icon, so the icon decides.
        assert_eq!(
            style.scrolled_tab_width(&theme, &narrow),
            TAB_ICON_SIZE + 2.0 * TAB_LABEL_PADDING
        );
        // A long label decides instead.
        let wide = TabSpec {
            label: "Notifications".into(),
            icon: Some(Icons::Star),
            show_label: true,
        };
        assert!(
            style.scrolled_tab_width(&theme, &wide) > style.scrolled_tab_width(&theme, &narrow)
        );
    }

    /// Where each tab's label was actually drawn, left to right.
    fn label_xs(tabs: TabBar<Msg>, width: f32) -> Vec<f32> {
        let root = crate::flex::Flex::column()
            .width(width)
            .height(300.0)
            .child(tabs);
        let mut runtime = Runtime::default();
        runtime.advance_values::<Msg>(&root, 0.0);
        let ui = build_ui(&root, Size::new(width, 300.0), &runtime, &Theme::default());
        // The tabs' own labels, and not the panel's text, which is empty and sits at
        // the origin: a stray zero at the head of the list would make every comparison
        // below quietly about the wrong thing.
        let mut xs: Vec<f32> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { position, text, .. } if text == "One" || text == "Two" => {
                    Some(position.x)
                }
                _ => None,
            })
            .collect();
        xs.sort_by(f32::total_cmp);
        xs
    }

    fn two(alignment: TabAlignment, scrolls: bool) -> TabBar<Msg> {
        TabBar::new(0, Msg::Select)
            .scrollable(scrolls)
            .alignment(alignment)
            .tab("One", crate::text(""))
            .tab("Two", crate::text(""))
    }

    /// The default is the bar's mode, said out loud: shared width when it does not
    /// scroll, packed at the leading edge when it does.
    #[test]
    fn auto_is_whatever_the_bar_already_did() {
        assert_eq!(TabAlignment::Auto.resolved(false), TabAlignment::Fill);
        assert_eq!(TabAlignment::Auto.resolved(true), TabAlignment::Start);
        // And filling a bar that scrolls is a contradiction, so it reads as `Start`
        // rather than throwing the way the reference does.
        assert_eq!(TabAlignment::Fill.resolved(true), TabAlignment::Start);
        // The rest mean what they say either way.
        assert_eq!(TabAlignment::Center.resolved(false), TabAlignment::Center);
        assert_eq!(TabAlignment::Center.resolved(true), TabAlignment::Start);
        assert_eq!(TabAlignment::Start.resolved(false), TabAlignment::Start);
    }

    /// Two tabs in a wide bar: filled they sit in the middle of their halves, started
    /// they hug the leading edge, centred they sit together in the middle.
    #[test]
    fn the_tabs_go_where_they_were_told() {
        let filled = label_xs(two(TabAlignment::Fill, false), 600.0);
        let started = label_xs(two(TabAlignment::Start, false), 600.0);
        let centred = label_xs(two(TabAlignment::Center, false), 600.0);

        // Filled, each tab is 300 wide, so the labels are nowhere near each other.
        assert!(filled[1] - filled[0] > 250.0, "{filled:?}");
        // Started, they are as wide as their words and sit at the leading edge.
        assert!((started[0] - TAB_LABEL_PADDING).abs() < 1.0, "{started:?}");
        assert!(started[1] - started[0] < 120.0, "{started:?}");
        // Centred, the pair keeps its spacing and moves right as a block.
        let gap = |xs: &[f32]| xs[1] - xs[0];
        assert!((gap(&centred) - gap(&started)).abs() < 1.0, "{centred:?}");
        assert!(centred[0] > started[0] + 100.0, "{centred:?}");
        // And the block really is centred: as much room after it as before it.
        let width: f32 = (0..2)
            .map(|i| {
                TabStyle::default().scrolled_tab_width(
                    &Theme::default(),
                    &TabSpec::text(if i == 0 { "One" } else { "Two" }),
                )
            })
            .sum();
        assert!(
            (centred[0] - TAB_LABEL_PADDING - (600.0 - width) / 2.0).abs() < 1.0,
            "{centred:?}"
        );
    }

    /// The offset is an inset before the first tab and nothing else: the second tab
    /// moves by exactly the same amount.
    #[test]
    fn the_start_offset_moves_the_whole_row() {
        let started = label_xs(two(TabAlignment::Start, false), 600.0);
        let offset = label_xs(two(TabAlignment::StartOffset, false), 600.0);
        for (a, b) in started.iter().zip(&offset) {
            assert!(
                (b - a - TAB_START_OFFSET).abs() < 1.0,
                "{started:?} {offset:?}"
            );
        }
    }

    /// A **scrolling** bar cannot centre, and says so by reading as `Start` rather than
    /// by pretending.
    ///
    /// Centring wants something definite to centre in, and the content of a scroll has
    /// no definite width across the axis it scrolls along: the strip is measured by its
    /// own tabs, so a percentage of its parent resolves against nothing and it comes out
    /// exactly as wide as the tabs, with nothing left over to share. Measured here
    /// rather than assumed — this is the trap of milestones 368 and 377, and the third
    /// time it has been paid for.
    #[test]
    fn a_scrolling_bar_cannot_centre_and_does_not_pretend() {
        let started = label_xs(two(TabAlignment::Start, true), 600.0);
        let centred = label_xs(two(TabAlignment::Center, true), 600.0);
        assert_eq!(started, centred);
        assert!((started[0] - TAB_LABEL_PADDING).abs() < 1.0, "{started:?}");
        // Without the scroll, the same bar centres.
        let still = label_xs(two(TabAlignment::Center, false), 600.0);
        assert!(still[0] > started[0] + 100.0, "{still:?}");
    }

    /// The indicator follows, because it reads the same spans the tabs are laid out to.
    /// This is the part that would drift.
    #[test]
    fn the_indicator_follows_the_alignment() {
        let indicator = |alignment: TabAlignment| {
            let root = crate::flex::Flex::column()
                .width(600.0)
                .height(300.0)
                .child(two(alignment, false));
            let mut runtime = Runtime::default();
            runtime.advance_values::<Msg>(&root, 0.0);
            build_ui(&root, Size::new(600.0, 300.0), &runtime, &Theme::default())
                .scene()
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    // The indicator: as thick as a primary bar's, not the hairline.
                    Primitive::Rect { rect, .. }
                        if (rect.height - TAB_INDICATOR_PRIMARY).abs() < 0.5 =>
                    {
                        Some(rect.x + rect.width / 2.0)
                    }
                    _ => None,
                })
                .next()
                .expect("an indicator")
        };
        let started = indicator(TabAlignment::Start);
        let centred = indicator(TabAlignment::Center);
        let labels_started = label_xs(two(TabAlignment::Start, false), 600.0);
        let labels_centred = label_xs(two(TabAlignment::Center, false), 600.0);
        // The indicator moved by exactly as much as the label it marks.
        assert!(
            ((centred - started) - (labels_centred[0] - labels_started[0])).abs() < 1.0,
            "{started} {centred}"
        );
    }

    fn three(selected: usize) -> TabBar<Msg> {
        TabBar::new(selected, Msg::Select)
            .tab("One", Text::new("panel one"))
            .tab("Two", Text::new("panel two"))
            .tab("Three", Text::new("panel three"))
    }

    /// A tab set takes the width it is offered rather than the width of whatever is on
    /// the busiest tab. Sized by its content, a centring row lets it hang out either side
    /// instead of telling it to fit — which is how a settings screen came to draw 5 px
    /// off a phone (milestone 335).
    #[test]
    fn a_tab_set_takes_the_width_it_is_offered() {
        let root = crate::flex::Flex::row()
            .width(300.0)
            .height(200.0)
            .justify(frus_layout::Justify::Center)
            .child(three(0));
        let mut runtime = Runtime::default();
        runtime.advance_values::<Msg>(&root, 0.0);
        let ui = build_ui(&root, Size::new(300.0, 200.0), &runtime, &Theme::default());
        // The bar is `width: 100%` of the tab set, so the hairline under it is the tab
        // set's width made visible.
        let widest = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } => Some(rect.width),
                _ => None,
            })
            .fold(0.0_f32, f32::max);
        assert!(
            (widest - 300.0).abs() < 0.5,
            "the whole row, not the widest label: {widest}"
        );
    }

    /// The frame's crisp boxes: the hairline, then the indicator.
    fn boxes(tabs: TabBar<Msg>) -> Vec<(Rect, Color)> {
        let root = crate::flex::Flex::column()
            .width(300.0)
            .height(200.0)
            .child(tabs);
        // The shell settles the animated values before it builds; a bare runtime would
        // catch the indicator on its way to the selected tab rather than under it.
        let mut runtime = Runtime::default();
        runtime.advance_values::<Msg>(&root, 0.0);
        build_ui(&root, Size::new(300.0, 200.0), &runtime, &Theme::default())
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect, color, blur, ..
                } if *blur == 0.0 => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn shows_bar_and_selected_panel() {
        let tabs = three(1);
        // [bar, panel] — the panel is the selected tab's content, tab 1.
        let children = Widget::<Msg>::children(&tabs);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].children().len(), 3, "three tabs in the bar");
    }

    #[test]
    fn no_panel_when_selection_out_of_range() {
        let tabs = TabBar::new(9, Msg::Select).tab("One", Text::new("x"));
        assert_eq!(Widget::<Msg>::children(&tabs).len(), 1); // the bar alone
    }

    #[test]
    fn the_indicator_sits_under_the_selected_tab() {
        // The point of the whole widget: which tab is selected has to be visible without
        // reading the labels' colours.
        let theme = Theme::default();
        let painted = boxes(three(1));
        let indicator = painted
            .iter()
            .find(|(_, color)| *color == theme.scheme.primary)
            .expect("an indicator in the accent colour");
        // Three tabs across 300: the middle one runs from 100 to 200.
        let centre = indicator.0.x + indicator.0.width / 2.0;
        assert!(
            (centre - 150.0).abs() < 1.0,
            "centred on the second tab, not at {centre}"
        );
        assert_eq!(indicator.0.height, TAB_INDICATOR_PRIMARY);
    }

    #[test]
    fn a_primary_indicator_is_as_wide_as_its_label_and_a_secondary_one_as_wide_as_its_tab() {
        // The difference between the two bars is not decoration: it is what the indicator
        // measures itself against.
        let theme = Theme::default();
        let width = |tabs: TabBar<Msg>| {
            boxes(tabs)
                .iter()
                .find(|(_, color)| *color == theme.scheme.primary)
                .expect("an indicator")
                .0
                .width
        };
        let primary = width(three(1));
        let secondary = width(three(1).variant(TabBarVariant::Secondary));
        assert_eq!(secondary, 100.0, "a third of 300 — the whole tab");
        assert!(
            primary < secondary,
            "a label is narrower than its tab: {primary} against {secondary}"
        );
        assert!(primary > 0.0, "and it is not nothing");
    }

    #[test]
    fn the_hairline_runs_the_whole_bar() {
        let theme = Theme::default();
        let painted = boxes(three(0));
        let line = painted
            .iter()
            .find(|(_, color)| *color == theme.scheme.outline_variant)
            .expect("a divider");
        assert_eq!(line.0.width, 300.0, "across the bar, not across a tab");
        assert_eq!(line.0.height, TAB_DIVIDER_HEIGHT);
        // The indicator sits on it: both end at the bar's foot.
        let indicator = painted
            .iter()
            .find(|(_, color)| *color == theme.scheme.primary)
            .expect("an indicator");
        assert_eq!(
            line.0.y + line.0.height,
            indicator.0.y + indicator.0.height,
            "the indicator sits on the hairline"
        );
    }

    #[test]
    fn the_bar_is_the_tabs_plus_the_indicator() {
        let theme = Theme::default();
        let tabs = three(0);
        let bar = &Widget::<Msg>::children(&tabs)[0];
        assert_eq!(
            bar.style_themed(&theme).height,
            Dimension::Length(TAB_HEIGHT + TAB_INDICATOR_PRIMARY)
        );
    }

    #[test]
    fn every_measurement_is_the_callers_and_then_the_themes() {
        // `caller ?? theme ?? framework`, on the one measurement that reaches layout.
        let mut theme = Theme::default();
        theme.widgets.tab_bar.tab_height = Some(60.0);
        let height = |tabs: TabBar<Msg>, theme: &Theme| {
            Widget::<Msg>::style_themed(&Widget::<Msg>::children(&tabs)[0], theme).height
        };
        assert_eq!(
            height(three(0), &Theme::default()),
            Dimension::Length(TAB_HEIGHT + TAB_INDICATOR_PRIMARY),
            "the framework's"
        );
        assert_eq!(
            height(three(0), &theme),
            Dimension::Length(60.0 + TAB_INDICATOR_PRIMARY),
            "the theme's"
        );
        assert_eq!(
            height(three(0).tab_height(20.0), &theme),
            Dimension::Length(20.0 + TAB_INDICATOR_PRIMARY),
            "the caller's, over the theme's"
        );
    }

    #[test]
    fn a_builder_after_the_tabs_still_reaches_them() {
        // The trap in a builder that assembles its children as it goes: `.tab(…)` builds
        // the bar, so anything said after it has to rebuild the bar or be dropped.
        let painted = boxes(three(1).indicator_weight(9.0));
        assert!(
            painted.iter().any(|(rect, _)| rect.height == 9.0),
            "the indicator took the weight set after the tabs"
        );
    }

    #[test]
    fn tabs_are_tabs_to_a_screen_reader() {
        let tabs = three(0);
        let bar = &Widget::<Msg>::children(&tabs)[0];
        let first = &bar.children()[0];
        let semantics = first.semantics().expect("a tab announces itself");
        assert_eq!(semantics.role, frus_core::Role::Tab);
        assert_eq!(semantics.label.as_deref(), Some("One"));
        assert!(semantics.clickable);
    }

    #[test]
    fn a_tab_emits_its_index() {
        let tabs = three(0);
        let bar = &Widget::<Msg>::children(&tabs)[0];
        assert_eq!(bar.children()[2].on_click(), Some(Msg::Select(2)));
    }

    #[test]
    fn a_disabled_bar_is_inert_but_keeps_its_panel() {
        let dead = TabBar::new(1, Msg::Select)
            .tab("One", Text::new("first"))
            .tab("Two", Text::new("second"))
            .enabled(false);
        // The panel survives: which tab is showing is still the answer.
        assert_eq!(
            Widget::<Msg>::children(&dead).len(),
            2,
            "the bar and its panel"
        );
        let bar = &Widget::<Msg>::children(&dead)[0];
        for (i, tab) in bar.children().iter().enumerate() {
            assert_eq!(tab.on_click(), None, "tab {i} still answers");
            assert!(!tab.focusable(), "tab {i} is still in the tab order");
            assert!(
                tab.ink(&Theme::default()).is_none(),
                "tab {i} still splashes"
            );
            let semantics = tab.semantics().expect("still announced");
            assert!(semantics.disabled, "tab {i} does not say it is disabled");
        }
        // And the selected one still says so.
        assert_eq!(
            bar.children()[1].semantics().unwrap().toggled,
            frus_core::Toggled::True
        );
    }

    #[test]
    fn a_disabled_bar_flattens_rather_than_fading_the_accent() {
        // Selected and unselected labels go to the same grey; the accent never appears.
        let theme = Theme::default();
        let dead = TabBar::new(0, Msg::Select)
            .tab("One", Text::new("first"))
            .tab("Two", Text::new("second"))
            .enabled(false);
        let bar = &Widget::<Msg>::children(&dead)[0];
        for tab in bar.children() {
            let mut scene = frus_core::Scene::new();
            tab.paint(
                Rect::new(0.0, 0.0, 80.0, 48.0),
                Status {
                    opacity: 1.0,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            let color = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Text { color, .. } => Some(*color),
                    _ => None,
                })
                .expect("a label");
            assert_eq!(color, disabled_content(&theme));
        }
    }

    /// The indicator is the part a flattened bar would otherwise keep the accent on -
    /// found by reading milestone 324's golden, not by a test.
    #[test]
    fn a_disabled_bars_indicator_loses_the_accent_too() {
        let theme = Theme::default();
        let indicator = |enabled: bool| {
            let tabs = TabBar::new(0, Msg::Select)
                .tab("One", Text::new("first"))
                .tab("Two", Text::new("second"))
                .enabled(enabled);
            let bar = &Widget::<Msg>::children(&tabs)[0];
            let mut scene = frus_core::Scene::new();
            bar.paint(
                Rect::new(0.0, 0.0, 200.0, 48.0),
                Status {
                    opacity: 1.0,
                    ..Default::default()
                },
                &theme,
                &mut scene,
            );
            // The indicator is the last box the bar paints, over the hairline.
            scene
                .primitives()
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Rect { color, .. } => Some(*color),
                    _ => None,
                })
                .expect("an indicator")
        };
        assert_eq!(indicator(false), disabled_content(&theme));
        assert_ne!(indicator(true), disabled_content(&theme));
        assert_ne!(
            indicator(false),
            indicator(true),
            "the instrument must be able to tell the two apart"
        );
    }
}

/// A bar with more tabs than fit.
#[cfg(test)]
mod scrollable_tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pick(usize),
    }

    /// Eight tabs of deliberately unequal label lengths.
    const LABELS: [&str; 8] = [
        "All", "Unread", "Starred", "Drafts", "Sent", "Archived", "Spam", "Bin",
    ];

    fn bar(scrollable: bool) -> TabBar<Msg> {
        let mut tabs = TabBar::new(0, Msg::Pick);
        for label in LABELS {
            tabs = tabs.tab(label, crate::text(""));
        }
        tabs.scrollable(scrollable)
    }

    /// The rects the bar painted, largest first -- the indicator is the small one.
    fn painted(tabs: TabBar<Msg>, width: f32) -> Vec<frus_core::Rect> {
        build_ui(
            &tabs,
            Size::new(width, 200.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
    }

    /// Off by default: nothing about an ordinary bar changes.
    #[test]
    fn a_bar_says_nothing_and_shares_its_width() {
        let tabs = bar(false);
        let ui = build_ui(
            &tabs,
            Size::new(360.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(
            ui.scrollable_maxes().is_empty(),
            "an ordinary bar registers no scrollable"
        );
    }

    /// Scrollable, the strip is wider than the bar and there is somewhere to scroll to.
    ///
    /// Eight labels at their own widths do not fit in 360 px -- which is the whole reason
    /// this exists, and the assertion says so rather than assuming it.
    #[test]
    fn eight_tabs_on_a_phone_scroll() {
        let ui = build_ui(
            &bar(true),
            Size::new(360.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1, "the strip is inside a scroll");
        assert!(maxes[0].1 > 0.0, "and it has room to move: {:?}", maxes[0]);
        assert_eq!(maxes[0].2, 0.0, "across, not down");
    }

    /// Each tab takes its label's room, so a long label gets a wide tab and a short one
    /// does not. Shared equally, every tab is the same width and this cannot be true.
    #[test]
    fn a_long_label_gets_a_wide_tab() {
        let theme = Theme::default();
        let style = TabStyle::default();
        let narrow = style.scrolled_tab_width(&theme, &TabSpec::text("Bin"));
        let wide = style.scrolled_tab_width(&theme, &TabSpec::text("Archived"));
        assert!(wide > narrow, "{wide} should exceed {narrow}");
    }

    /// The indicator follows the **real** tab widths.
    ///
    /// This is the part that would drift: the tab lays itself out to one number and the
    /// indicator is placed by another. They ask the same function, and this checks the
    /// answer against a hand-computed prefix sum rather than against that function --
    /// otherwise the test would agree with a mistake.
    #[test]
    fn the_indicator_sits_over_the_tab_it_marks() {
        let theme = Theme::default();
        let style = TabStyle::default();
        let labels: Vec<TabSpec> = LABELS.iter().map(|s| TabSpec::text(*s)).collect();
        let spans = style.tab_spans(&theme, &labels, 360.0, TabAlignment::Start);

        let mut expected = 0.0;
        for (i, label) in labels.iter().enumerate() {
            assert!(
                (spans[i].0 - expected).abs() < 0.01,
                "tab {i} starts at {} rather than {expected}",
                spans[i].0
            );
            expected += style.scrolled_tab_width(&theme, label);
        }
        // And the last tab ends where the strip does.
        let (start, width) = spans[labels.len() - 1];
        assert!((start + width - expected).abs() < 0.01);
    }

    /// Not scrollable, the spans are still equal parts -- the old rule, untouched.
    #[test]
    fn an_ordinary_bar_still_divides_its_width_evenly() {
        let labels: Vec<TabSpec> = LABELS.iter().map(|s| TabSpec::text(*s)).collect();
        let spans =
            TabStyle::default().tab_spans(&Theme::default(), &labels, 800.0, TabAlignment::Fill);
        assert_eq!(spans[0], (0.0, 100.0));
        assert_eq!(spans[7], (700.0, 100.0));
    }

    /// Two tabs in a scrollable bar still get a hairline across the whole foot, rather
    /// than a stub under the labels. The strip is never narrower than the bar.
    #[test]
    fn a_short_scrollable_bar_still_rules_a_full_line() {
        let mut tabs = TabBar::new(0, Msg::Pick);
        tabs = tabs.tab("A", crate::text("")).tab("B", crate::text(""));
        let rects = painted(tabs.scrollable(true), 400.0);
        assert!(
            rects.iter().any(|r| r.width >= 399.0),
            "something runs the full width: {rects:?}"
        );
    }
}
