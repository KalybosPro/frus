//! [`AppBar`]: an **adaptive application bar**, Material style.
//!
//! The developer declares **one** title, an optional `leading` and a list of
//! **actions** — never saying "this is for mobile / desktop". The AppBar decides on
//! its own, from the **available width**, how many actions fit inline and **folds
//! the rest into a `⋯` overflow menu**. A wide screen → everything inline; a narrow
//! phone → overflow. One piece of code, adapting automatically.
//!
//! **Everything is customisable** (themed defaults, never imposed): the title can
//! be an arbitrary widget (`title_widget`) or styled text (`title_style`), an
//! action can be an arbitrary widget (`action_widget`, always inline), and the
//! spacing, the action size, the background and the height can all be overridden.
//!
//! ```ignore
//! AppBar::new("My Tasks")
//!     .width(available_width)                 // a size, not a platform
//!     .title_style(TextStyle::new(22.0))      // ou .title_widget(logo_row)
//!     .leading(button("☰", Msg::ToggleMenu))
//!     .overflow(app.menu_open, Msg::ToggleMenu)
//!     .action("Pause", Msg::ToggleTimer)
//!     .action_widget(Badge::new("3"))         // a free widget, never folded
//!     .action("Settings →", Msg::OpenSettings)
//!     .build()
//! ```

use frus_core::{Color, FontWeight, TextStyle};
use frus_layout::{Align, Dimension};

use crate::button::Variant;
use crate::container::Container;
use crate::dsl::button;
use crate::flex::Flex;
use crate::menu::Menu;
use crate::text::Text;
use crate::widget::Widget;

/// The title's font size (the default, overridden by [`AppBar::title_style`]).
const TITLE_SIZE: f32 = 20.0;
/// The actions' font size (the default, overridden by [`AppBar::action_size`]).
const ACTION_SIZE: f32 = 16.0;
/// A button's inner horizontal padding (must follow `button::PAD_X`).
const BTN_PAD_X: f32 = 20.0;
/// The space between the bar's elements (the default, overridden by [`AppBar::gap`]).
const GAP: f32 = 8.0;
/// The width reserved for the `leading` slot (a leading icon, Material style).
const LEADING_SLOT: f32 = 56.0;
/// The bar's horizontal margin: the content does not touch the edges (Material
/// style). Counted in the folding budget.
const H_PAD: f32 = 8.0;

/// The title: styled text, or any widget at all.
enum Title<Msg> {
    Text(String),
    Widget(Box<dyn Widget<Msg>>),
}

/// An action: labelled (foldable into the overflow) or a free widget (always
/// inline — an arbitrary widget cannot become a text menu row).
enum Action<Msg> {
    Labeled { label: String, message: Msg },
    Custom(Box<dyn Widget<Msg>>),
}

/// An adaptive application bar. A fluent builder finished by [`AppBar::build`].
pub struct AppBar<Msg> {
    title: Title<Msg>,
    title_style: TextStyle,
    width: f32,
    leading: Option<Box<dyn Widget<Msg>>>,
    overflow: Option<(bool, Msg)>,
    actions: Vec<Action<Msg>>,
    action_size: f32,
    gap: f32,
    background: Option<Color>,
    height: Option<f32>,
}

impl<Msg: Clone + 'static> AppBar<Msg> {
    /// Creates a bar with a text title. Without [`AppBar::width`], nothing folds.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Title::Text(title.into()),
            title_style: TextStyle::new(TITLE_SIZE).weight(FontWeight::Medium),
            width: f32::MAX,
            leading: None,
            overflow: None,
            actions: Vec::new(),
            action_size: ACTION_SIZE,
            gap: GAP,
            background: None,
            height: None,
        }
    }

    /// The **available width** for the bar, in logical pixels: what drives the
    /// folding. It is a *size*, not a platform indicator.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The text title's style (size/weight/italic/color). Default: 20 px, medium
    /// weight, the theme's color.
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = style;
        self
    }

    /// Replaces the title with an **arbitrary widget** — a logo, a composed row, and
    /// so on.
    pub fn title_widget(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.title = Title::Widget(Box::new(widget));
        self
    }

    /// The leading element (a menu or back button…), optional.
    pub fn leading(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.leading = Some(Box::new(widget));
        self
    }

    /// Enables the overflow menu: its open state (controlled by the app) and the
    /// toggle message (emitted by the `⋯` button and on an outside click).
    pub fn overflow(mut self, open: bool, toggle: Msg) -> Self {
        self.overflow = Some((open, toggle));
        self
    }

    /// Adds a labelled action (a button). Shown inline if it fits, otherwise folded
    /// into the overflow menu.
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.actions.push(Action::Labeled {
            label: label.into(),
            message,
        });
        self
    }

    /// Adds a **free widget** action (a badge, an avatar, a field…). Always inline —
    /// an arbitrary widget cannot fold into a menu row.
    pub fn action_widget(mut self, widget: impl Widget<Msg> + 'static) -> Self {
        self.actions.push(Action::Custom(Box::new(widget)));
        self
    }

    /// The labelled actions' font size (16 px by default).
    pub fn action_size(mut self, size: f32) -> Self {
        self.action_size = size;
        self
    }

    /// The space between the bar's elements (8 px by default).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// The bar's background color (transparent by default: the parent decides).
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// An imposed bar height (by default, the content's natural height).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The width an action button would take for this label.
    fn action_width(label: &str, size: f32) -> f32 {
        frus_text::measure(label, size).width + BTN_PAD_X * 2.0
    }

    /// A widget's declared width (0 if it depends on layout).
    fn widget_width(widget: &dyn Widget<Msg>) -> f32 {
        match widget.style().width {
            Dimension::Length(v) => v,
            _ => 0.0,
        }
    }

    /// Assembles the bar into a widget ready to display (the row `leading · title ·
    /// spring · inline actions · overflow`).
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let AppBar {
            title,
            title_style,
            width,
            leading,
            overflow,
            actions,
            action_size,
            gap,
            background,
            height,
        } = self;

        // The horizontal budget left for the inline actions, after the leading, the
        // title and the margins. Conservative: when in doubt, fold.
        let leading_w = if leading.is_some() { LEADING_SLOT } else { 0.0 };
        let title_w = match &title {
            Title::Text(content) => {
                frus_text::measure_styled(
                    content,
                    title_style.size,
                    title_style.weight,
                    title_style.italic,
                )
                .width
            }
            Title::Widget(widget) => Self::widget_width(widget.as_ref()),
        };
        // The horizontal margin eats into the budget on both sides (the content does
        // not touch the edges). An unset `width` (f32::MAX) stays roughly infinite.
        let budget = width - H_PAD * 2.0 - leading_w - title_w - gap * 3.0;
        let overflow_btn_w = Self::action_width("⋯", action_size) + gap;

        // Each action's width; free widgets are **always** inline.
        let widths: Vec<f32> = actions
            .iter()
            .map(|action| match action {
                Action::Labeled { label, .. } => Self::action_width(label, action_size) + gap,
                Action::Custom(widget) => Self::widget_width(widget.as_ref()) + gap,
            })
            .collect();
        let total: f32 = widths.iter().sum();
        let custom_total: f32 = actions
            .iter()
            .zip(&widths)
            .filter(|(action, _)| matches!(action, Action::Custom(_)))
            .map(|(_, w)| *w)
            .sum();

        // How many **labelled** actions fit inline? If everything fits, no overflow;
        // otherwise reserve the `⋯` button and the free widgets, and keep as many
        // labelled ones as possible (a prefix, in order).
        let kept_labeled = if total <= budget {
            usize::MAX
        } else {
            let mut used = overflow_btn_w + custom_total;
            let mut kept = 0;
            for (action, w) in actions.iter().zip(&widths) {
                if matches!(action, Action::Custom(_)) {
                    continue;
                }
                if used + w <= budget {
                    used += w;
                    kept += 1;
                } else {
                    break;
                }
            }
            kept
        };

        let mut row = Flex::row().align(Align::Center).gap(gap);
        if let Some(leading) = leading {
            row = row.child(leading);
        }
        match title {
            Title::Text(content) => row = row.child(Text::styled(content, title_style)),
            Title::Widget(widget) => row = row.child(widget),
        }
        // The spring: it pushes the actions to the right.
        row = row.child(Container::new().flex(1.0));

        let mut labeled_seen = 0;
        let mut folded: Vec<(String, Msg)> = Vec::new();
        for action in actions {
            match action {
                Action::Custom(widget) => row = row.child(widget),
                Action::Labeled { label, message } => {
                    if labeled_seen < kept_labeled {
                        row = row.child(
                            button(label, message)
                                .variant(Variant::Secondary)
                                .size(action_size),
                        );
                    } else {
                        folded.push((label, message));
                    }
                    labeled_seen += 1;
                }
            }
        }

        if !folded.is_empty() {
            match overflow {
                // A controlled overflow menu: `⋯` opens it, the items emit the actions.
                Some((open, toggle)) => {
                    let mut menu = Menu::new(
                        button("⋯", toggle.clone())
                            .variant(Variant::Secondary)
                            .size(action_size),
                        open,
                        toggle,
                    );
                    for (label, message) in folded {
                        menu = menu.item(label, message);
                    }
                    row = row.child(menu);
                }
                // No overflow configured: show everything inline (it may overflow).
                None => {
                    for (label, message) in folded {
                        row = row.child(
                            button(label, message)
                                .variant(Variant::Secondary)
                                .size(action_size),
                        );
                    }
                }
            }
        }

        // The horizontal margin (+ optional chrome: background, height). The row is
        // always framed so that the content does not touch the edges.
        let mut chrome = Container::new().padding_each(0.0, H_PAD, 0.0, H_PAD);
        if let Some(color) = background {
            chrome = chrome.color(color);
        }
        if let Some(h) = height {
            chrome = chrome.height(h);
        }
        Box::new(chrome.child(row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Menu,
        A,
        B,
        C,
    }

    /// Counts the buttons (rectangles with a shadow), excluding floating menu items.
    fn inline_buttons(width: f32, open: bool) -> usize {
        let bar = AppBar::new("Title")
            .width(width)
            .overflow(open, Msg::Menu)
            .action("Action One", Msg::A)
            .action("Action Two", Msg::B)
            .action("Action Three", Msg::C)
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(width, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // Every button draws a shadow (a blurred `Rect`, `blur > 0`), so count those.
        ui.scene()
            .primitives()
            .iter()
            .filter(|p| matches!(p, frus_core::Primitive::Rect { blur, .. } if *blur > 0.0))
            .count()
    }

    #[test]
    fn wide_bar_shows_all_actions_inline() {
        // Wide enough: the 3 actions fit, and there is no overflow button.
        assert_eq!(inline_buttons(1200.0, false), 3);
    }

    #[test]
    fn narrow_bar_collapses_into_overflow() {
        // Narrow: at most one or two inline actions + the `⋯` button.
        let n = inline_buttons(300.0, false);
        assert!(
            n < 3,
            "expected a fold into overflow, got {n} inline buttons"
        );
        assert!(n >= 1, "the overflow button must be present");
    }

    /// The bar keeps a horizontal margin: the content (the leading on the left, the
    /// last action on the right) does not touch the viewport's edges.
    #[test]
    fn content_keeps_a_horizontal_margin() {
        const W: f32 = 400.0;
        let bar = AppBar::new("Title")
            .width(W)
            .leading(button("M", Msg::Menu).size(16.0))
            .overflow(false, Msg::Menu)
            .action("One", Msg::A)
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(W, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The horizontal bounds of the **texts** (title + labels): having no shadow,
        // they reflect the content's real position (a shadow's blur, by contrast,
        // legitimately overflows).
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        for p in ui.scene().primitives() {
            if let frus_core::Primitive::Text {
                position,
                size,
                text,
                ..
            } = p
            {
                min_x = min_x.min(position.x);
                // An approximate text width (an upper bound is enough here).
                max_x = max_x.max(position.x + text.chars().count() as f32 * size * 0.7);
            }
        }
        assert!(
            min_x >= H_PAD - 0.5,
            "content flush against the left edge ({min_x})"
        );
        assert!(
            max_x <= W - H_PAD + 0.5,
            "content overflowing on the right ({max_x} > {})",
            W - H_PAD
        );
    }

    #[test]
    fn title_style_is_customizable() {
        // An overridden title style: bold 24, instead of the default medium 20.
        let bar = AppBar::<Msg>::new("Title")
            .title_style(TextStyle::new(24.0).weight(FontWeight::Bold))
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(800.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let styled = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                frus_core::Primitive::Text { text, size, weight, .. }
                    if text == "Title" && *size == 24.0 && *weight == FontWeight::Bold
            )
        });
        assert!(styled, "the title must carry the overridden style");
    }

    #[test]
    fn title_can_be_an_arbitrary_widget() {
        let bar = AppBar::<Msg>::new("ignored")
            .title_widget(Text::new("Logo").size(18.0))
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(800.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let texts: Vec<_> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.contains(&"Logo".to_string()),
            "the title widget is rendered"
        );
        assert!(
            !texts.contains(&"ignored".to_string()),
            "the text title is replaced"
        );
    }

    #[test]
    fn custom_widget_action_never_folds() {
        // A very narrow bar: the labelled actions fold, but the free widget (which
        // cannot be represented as a menu row) stays inline.
        let bar = AppBar::new("Title")
            .width(260.0)
            .overflow(false, Msg::Menu)
            .action("A long labelled action", Msg::A)
            .action_widget(Text::new("★badge★").size(14.0))
            .action("Another long action", Msg::B)
            .build();
        let ui = build_ui(
            bar.as_ref(),
            Size::new(260.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has_badge = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, frus_core::Primitive::Text { text, .. } if text == "★badge★"));
        assert!(
            has_badge,
            "the widget action stays inline even when cramped"
        );
    }
}
