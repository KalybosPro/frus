//! [`DropdownMenu`]: a dropdown that **looks like a text field**, filters as you type, and
//! drops its choices under itself.
//!
//! It is not [`crate::Autocomplete`], which is the nearest-looking thing here and a
//! different control: an autocomplete *suggests* over free text, where this has a **closed
//! set** and a selected member of it. Nor is it [`crate::DropdownButton`], which is the
//! same closed set behind a button rather than a field — no query, no filtering, no label
//! floating over the border.
//!
//! # The rule that shapes it
//!
//! **A closed field shows the choice that is selected, and nothing else can appear there.**
//! The query the caller is holding is displayed only while the menu is open. So a reader
//! who types three letters, changes their mind and clicks away is looking at their actual
//! choice again the moment the menu shuts — with no message sent, no state restored, and
//! nothing for an application to remember to do.
//!
//! That is the whole reason the display is derived rather than passed in. Every other
//! arrangement — the field showing the query at all times, with the application putting it
//! back on dismissal — leaves the rule as something a caller can forget, and a dropdown
//! showing `"gre"` where `"Green"` is selected is a dropdown that has lied about its own
//! value.

use std::rc::Rc;

use frus_core::{BorderRadius, Color, Insets, Rect, Scene, ShapeBorder, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::dropdown::{label_style, option_row, DropdownOption};
use crate::flex::Flex;
use crate::icons::Icons;
use crate::interaction::Status;
use crate::menu::{Panel, PanelKind, PanelStyle};
use crate::portal::Placement;
use crate::scroll::SingleChildScrollView;
use crate::textinput::TextField;
use crate::theme::Theme;
use crate::widget::Widget;

/// The field's width when the caller has not said otherwise.
const DEFAULT_WIDTH: f32 = 240.0;
/// One choice's height, before the reader's type is applied to it.
const ROW_H: f32 = 40.0;

/// Whether `label` matches `query` — a case-insensitive **substring**, which is what
/// filtering a short list of names wants.
///
/// Not a prefix: a set of `"Dark green"`, `"Light green"` and `"Sea green"` filtered by
/// `"green"` on a prefix rule answers nothing at all, which reads as a broken control
/// rather than as a rule.
///
/// An empty or blank query matches everything: it is not a filter yet.
fn matches(label: &str, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    label.to_lowercase().contains(&query.trim().to_lowercase())
}

/// A dropdown that looks like a text field, filters as it is typed into, and drops its
/// choices below itself.
///
/// Controlled, like everything else here: the application holds the query, whether the
/// menu is open, and which choice is selected.
///
/// ```
/// use frus_widgets::{DropdownMenu, DropdownOption};
///
/// # #[derive(Clone)] enum Msg { Query(String), Toggle, Pick(usize) }
/// # let (query, open, chosen) = (String::new(), false, Some(1));
/// let field = DropdownMenu::new(&query, open, Msg::Query, Msg::Toggle)
///     .label("Colour")
///     .selected(chosen)
///     .options(&["Red", "Green", "Blue"], Msg::Pick);
/// ```
pub struct DropdownMenu<Msg> {
    query: String,
    open: bool,
    enabled: bool,
    filter: bool,
    width: f32,
    selected: Option<usize>,
    label: Option<String>,
    placeholder: Option<String>,
    max_visible: Option<usize>,
    text_style: Option<TextStyle>,
    /// The panel the choices float on — see [`menu_background`](Self::menu_background).
    look: PanelStyle,
    on_input: Rc<dyn Fn(String) -> Msg>,
    on_toggle: Msg,
    options: Vec<DropdownOption<Msg>>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// `[field]`, or `[field, list]` when the menu is showing something.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> DropdownMenu<Msg> {
    /// Creates the control: the query the application is holding, whether the menu is
    /// open, what to send as the query changes, and what to send when the chevron is
    /// pressed.
    ///
    /// `on_input` fires only while the menu is open, because that is the only time the
    /// field shows the query — see the module documentation. An application will usually
    /// open the menu on `on_toggle` and keep it open through `on_input`.
    pub fn new(
        query: impl Into<String>,
        open: bool,
        on_input: impl Fn(String) -> Msg + 'static,
        on_toggle: Msg,
    ) -> Self {
        let mut menu = Self {
            query: query.into(),
            open,
            enabled: true,
            filter: true,
            width: DEFAULT_WIDTH,
            selected: None,
            label: None,
            placeholder: None,
            max_visible: None,
            text_style: None,
            look: PanelStyle::default(),
            on_input: Rc::new(on_input),
            on_toggle,
            options: Vec::new(),
            on_select: None,
            children: Vec::new(),
        };
        menu.rebuild();
        menu
    }

    /// The name of the field, floating over its border the way a
    /// [`TextField`](crate::TextField)'s does.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.rebuild();
        self
    }

    /// What the field shows while **nothing is selected**.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self.rebuild();
        self
    }

    /// The field's width, and the list's, in logical pixels (240 by default).
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Which choice is selected — the one ticked in the list, and the one the closed field
    /// shows.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self.rebuild();
        self
    }

    /// Whether the control can be used at all. Disabled it is **inert** and, like a
    /// disabled [`DropdownButton`](crate::DropdownButton), **never open**.
    ///
    /// See [`crate::disabled`] for the whole contract.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Whether typing **filters** the list. On by default.
    ///
    /// That is the opposite of the reference's own default, and deliberately: filtering is
    /// the reason to reach for this control rather than
    /// [`DropdownButton`](crate::DropdownButton), which is the same closed set behind a
    /// button. A caller who does not want it has that widget already.
    ///
    /// Off, the field is **read-only**: a field that takes typing which changes nothing is
    /// a field that looks broken.
    #[must_use]
    pub fn filter(mut self, filter: bool) -> Self {
        self.filter = filter;
        self.rebuild();
        self
    }

    /// Limits the number of **visible** choices; past that the list scrolls instead of
    /// stretching down the screen.
    #[must_use]
    pub fn max_visible(mut self, rows: usize) -> Self {
        self.max_visible = Some(rows.max(1));
        self.rebuild();
        self
    }

    /// The type of the field and the choices, over the theme's and the reference's.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// **The surface the choices float on**, over
    /// [`MenuTheme::background`](crate::MenuTheme::background) and `surface_container`.
    ///
    /// The choices sit on **one panel**, the one a menu floats on: a surface, the corner,
    /// eight pixels above and below, and a shadow three high. The reference's dropdown menu
    /// hands its panel to its menu anchor, so it answers to the menu theme, and so does
    /// this. A choice has no box of its own; only the selected one and the one under a
    /// pointer are tinted.
    #[must_use]
    pub fn menu_background(mut self, color: Color) -> Self {
        self.look.background = Some(color);
        self.rebuild();
        self
    }

    /// What shape the panel is, over the theme's and the framework's corner.
    #[must_use]
    pub fn menu_shape(mut self, shape: ShapeBorder) -> Self {
        self.look.shape = Some(shape);
        self.rebuild();
        self
    }

    /// The shorthand for a rounded panel.
    #[must_use]
    pub fn menu_radius(self, radius: impl Into<BorderRadius>) -> Self {
        self.menu_shape(ShapeBorder::rounded(radius.into()))
    }

    /// How far off the page the panel sits, in pixels. Three by default.
    #[must_use]
    pub fn menu_elevation(mut self, elevation: f32) -> Self {
        self.look.elevation = Some(elevation);
        self.rebuild();
        self
    }

    /// The colour of the panel's shadow, over the theme's and the scheme's shadow at 30 %.
    /// [`Color::TRANSPARENT`] casts none.
    #[must_use]
    pub fn menu_shadow_color(mut self, color: Color) -> Self {
        self.look.shadow_color = Some(color);
        self.rebuild();
        self
    }

    /// The room kept above and below the choices, inside the panel. Eight by default.
    #[must_use]
    pub fn menu_padding(mut self, padding: Insets) -> Self {
        self.look.padding = Some(padding);
        self.rebuild();
        self
    }

    /// The choices, as words. `on_select` maps the chosen index to a message.
    #[must_use]
    pub fn options(self, labels: &[&str], on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        let options = labels.iter().map(|s| DropdownOption::new(*s)).collect();
        self.options_widgets(options, on_select)
    }

    /// The same, for choices that are not words — see [`DropdownOption`].
    ///
    /// **`on_select` is given the index into this list**, not into whatever the filter left
    /// showing. Handing back a filtered index is the classic bug in this control: it works
    /// perfectly until somebody types, and then picks the wrong thing silently.
    #[must_use]
    pub fn options_widgets(
        mut self,
        options: Vec<DropdownOption<Msg>>,
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        self.options = options;
        self.on_select = Some(Rc::new(on_select));
        self.rebuild();
        self
    }

    /// What the field displays: the query while the menu is open, the selected choice's
    /// words while it is shut.
    ///
    /// A choice the caller drew has no words, so a closed field showing one falls back to
    /// nothing rather than to a placeholder — something *is* selected, and saying
    /// "choose one" would be false. The words are what the field can show; a caller whose
    /// choices are pictures should give the field a
    /// [`label`](Self::label) that says what has been chosen.
    fn display(&self) -> String {
        if self.open && self.enabled && self.filter {
            return self.query.clone();
        }
        self.selected
            .and_then(|i| self.options.get(i))
            .and_then(DropdownOption::label)
            .unwrap_or_default()
            .to_string()
    }

    /// The choices the filter leaves, each with the index it has in the caller's own list.
    fn showing(&self) -> Vec<usize> {
        (0..self.options.len())
            .filter(|&i| {
                if !(self.filter && self.open) {
                    return true;
                }
                match self.options[i].label() {
                    Some(label) => matches(label, &self.query),
                    // A choice the caller drew has no words to match against. It stays:
                    // hiding it would make the swatches disappear as soon as anybody
                    // typed, and a filter that removes what it cannot read is worse than
                    // one that keeps it.
                    None => true,
                }
            })
            .collect()
    }

    fn rebuild(&mut self) {
        let on_input = self.on_input.clone();
        let mut field = TextField::new(self.display())
            .width(self.width)
            .enabled(self.enabled)
            // The chevron turns over with the menu, which is the only thing on a closed
            // field saying that there is more of it.
            .suffix_icon(if self.open && self.enabled {
                Icons::ARROW_DROP_UP
            } else {
                Icons::ARROW_DROP_DOWN
            })
            .on_suffix(self.on_toggle.clone())
            .on_input(move |text| on_input(text));
        if let Some(label) = &self.label {
            field = field.label(label.clone());
        }
        if let Some(placeholder) = &self.placeholder {
            field = field.placeholder(placeholder.clone());
        }
        if let Some(size) = self.text_style.and_then(|style| style.size) {
            field = field.size(size);
        }
        if !self.filter {
            // Typing that changes nothing looks like a broken field, so it does not take
            // any.
            field = field.read_only();
        }
        self.children = vec![Box::new(field)];

        if !self.open || !self.enabled {
            return;
        }
        let showing = self.showing();
        if showing.is_empty() {
            // **No empty panel.** A floating surface with nothing in it is a control that
            // looks broken; a query matching nothing should leave the field alone.
            return;
        }
        // Contiguous, on one panel: the four-pixel gutter the choices used to leave would
        // show the page through the middle of the list.
        let mut list = Flex::column();
        for &index in &showing {
            let option = &self.options[index];
            let on_click = self
                .on_select
                .as_ref()
                .filter(|_| option.is_enabled())
                .map(|f| f(index));
            list = list.child(option_row(
                option,
                self.width,
                self.selected == Some(index),
                self.enabled,
                self.text_style,
                (PanelKind::Menu, self.look.background),
                on_click,
            ));
        }
        let rows = showing.len();
        // **A long list scrolls inside the panel**, not the panel inside a viewport: the
        // surface, its room above and below and its shadow stay whole, and the rows are
        // clipped in the viewport between the two rooms — where the reference's menu puts
        // its scroll view, inside the panel's padding.
        let content: Box<dyn Widget<Msg>> = match self.max_visible {
            Some(n) if rows > n => {
                // The rows' own height, not the floor: a viewport counted at `ROW_H`
                // while the rows are taller shows `n` rows minus a sliver of each. `None`
                // for the theme is the one thing a builder cannot have — the list is built
                // before any theme exists — so an application that retypesets the choices
                // through the theme *and* caps them should say the size on the widget.
                let row = frus_text::line_box(ROW_H, &label_style(self.text_style, None), 0.0);
                Box::new(
                    SingleChildScrollView::new()
                        .width(self.width)
                        .height(n as f32 * row)
                        .child(list),
                )
            }
            _ => Box::new(list),
        };
        self.children.push(Box::new(Panel::new(
            PanelKind::Menu,
            self.look,
            vec![content],
        )));
    }
}

impl<Msg: Clone> Widget<Msg> for DropdownMenu<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            width: Dimension::Length(self.width),
            padding: Insets::ZERO,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.children
            .get(1)
            .map(|list| (list.as_ref(), Placement::Below))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        // A press outside shuts it, and shutting it puts the selected choice back in the
        // field on its own — that is what deriving the display buys.
        (self.open && self.enabled).then(|| self.on_toggle.clone())
    }

    fn overlay_traps_focus(&self) -> bool {
        self.open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size};
    use frus_core::{Color, Point, Primitive};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Query(String),
        Toggle,
        Pick(usize),
    }

    /// Every run of text in the frame.
    fn texts(scene: &frus_core::Scene) -> Vec<String> {
        fn walk(primitives: &[Primitive], out: &mut Vec<String>) {
            for p in primitives {
                match p {
                    Primitive::Text { text, .. } => out.push(text.clone()),
                    Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(scene.primitives(), &mut out);
        out
    }

    fn frame(menu: &DropdownMenu<Msg>) -> crate::Ui<Msg> {
        build_ui(
            menu,
            Size::new(400.0, 500.0),
            &Runtime::default(),
            &Theme::default(),
        )
    }

    fn colours() -> [&'static str; 4] {
        ["Red", "Dark green", "Light green", "Blue"]
    }

    fn menu(query: &str, open: bool) -> DropdownMenu<Msg> {
        DropdownMenu::new(query, open, Msg::Query, Msg::Toggle)
            .selected(Some(1))
            .options(&colours(), Msg::Pick)
    }

    /// **A closed field shows the choice, never the query.** The whole control is shaped
    /// around this: a reader who types three letters, thinks better of it and clicks away
    /// is looking at their actual choice again, with nothing sent and nothing for an
    /// application to remember to put back.
    ///
    /// The alternative — the field showing the query at all times — leaves the rule as
    /// something a caller can forget, and a field reading `gre` while `Dark green` is
    /// selected has lied about its own value.
    #[test]
    fn a_closed_field_shows_the_choice_and_not_the_query() {
        let shut = frame(&menu("gre", false));
        let drawn = texts(shut.scene());
        assert!(
            drawn.iter().any(|t| t == "Dark green"),
            "the selected choice: {drawn:?}"
        );
        assert!(
            !drawn.iter().any(|t| t == "gre"),
            "and not the half-typed query: {drawn:?}"
        );

        let open = frame(&menu("gre", true));
        assert!(
            texts(open.scene()).iter().any(|t| t == "gre"),
            "which is the only thing an open one shows"
        );
    }

    /// **Nothing selected shows nothing**, and the placeholder is what says so.
    #[test]
    fn nothing_selected_shows_the_placeholder() {
        let none = DropdownMenu::new("", false, Msg::Query, Msg::Toggle)
            .placeholder("Choose a colour")
            .options(&colours(), Msg::Pick);
        assert!(texts(frame(&none).scene())
            .iter()
            .any(|t| t == "Choose a colour"));
    }

    /// **The list filters as it is typed into**, on a case-insensitive **substring**. Two
    /// of the four colours have `green` in them and neither of them starts with it: a
    /// prefix rule would answer nothing here, which reads as a broken control rather than
    /// as a rule.
    #[test]
    fn the_list_filters_on_a_substring_and_not_a_prefix() {
        let shown = |query: &str| {
            let ui = frame(&menu(query, true));
            texts(ui.scene())
                .into_iter()
                .filter(|t| colours().contains(&t.as_str()))
                .count()
        };
        assert_eq!(shown(""), 4, "an empty query is not a filter yet");
        assert_eq!(shown("GREEN"), 2, "case-insensitive, and matching inside");
        assert_eq!(shown("blu"), 1);
    }

    /// **The message carries the index in the caller's own list**, not in whatever the
    /// filter left showing.
    ///
    /// This is the bug this control is famous for. It works perfectly until somebody
    /// types, and then picks the wrong thing — silently, because both indices are valid.
    /// `Light green` is the third colour and the second row of the filtered list.
    #[test]
    fn the_message_carries_the_index_in_the_callers_own_list() {
        let ui = frame(&menu("green", true));
        // The field is 56 tall, and the panel keeps eight above its first row.
        let row = |n: f32| {
            ui.hit(Point::new(120.0, 56.0 + 8.0 + ROW_H * (n + 0.5)))
                .and_then(|id| ui.msg_for(id))
        };
        // Both, so that the coordinates are known to be landing on rows at all: an
        // assertion about the second row alone passes just as well when it is hitting
        // nothing.
        assert_eq!(
            row(0.0),
            Some(Msg::Pick(1)),
            "`Dark green` is the second colour"
        );
        assert_eq!(
            row(1.0),
            Some(Msg::Pick(2)),
            "and the second row of the filtered list is the third"
        );
    }

    /// **A query that matches nothing floats no panel.** An empty surface hanging under a
    /// field is a control that looks broken.
    #[test]
    fn a_query_that_matches_nothing_floats_no_panel() {
        let empty = menu("zzz", true);
        assert!(Widget::<Msg>::overlay(&empty).is_none());
        assert!(Widget::<Msg>::overlay(&menu("green", true)).is_some());
    }

    /// **Filtering off keeps the field showing the choice even while open**, because with
    /// nothing to type there is no query to show — and the field is read-only, since one
    /// that takes typing which changes nothing looks broken.
    #[test]
    fn filtering_off_leaves_the_choice_in_the_field() {
        let unfiltered = menu("gre", true).filter(false);
        let drawn = texts(frame(&unfiltered).scene());
        assert!(drawn.iter().any(|t| t == "Dark green"));
        assert!(!drawn.iter().any(|t| t == "gre"));
        assert_eq!(
            texts(frame(&unfiltered).scene())
                .into_iter()
                .filter(|t| colours().contains(&t.as_str()))
                .count(),
            5,
            "all four choices, plus the one in the field"
        );
    }

    /// **Disabled it is inert and never open**, like every other disabled control here.
    #[test]
    fn disabled_is_never_open() {
        let off = menu("", true).enabled(false);
        assert!(Widget::<Msg>::overlay(&off).is_none());
        assert_eq!(Widget::<Msg>::overlay_dismiss(&off), None);
    }

    /// **A press outside shuts it**, and shutting it puts the choice back in the field on
    /// its own — which is what deriving the display buys.
    #[test]
    fn a_press_outside_shuts_it() {
        assert_eq!(
            Widget::<Msg>::overlay_dismiss(&menu("", true)),
            Some(Msg::Toggle)
        );
    }

    /// **A choice the caller drew is not filtered out.** It has no words to match, and
    /// hiding it would make the swatches vanish the moment anybody typed — a filter that
    /// removes what it cannot read is worse than one that keeps it.
    #[test]
    fn a_choice_with_no_words_survives_the_filter() {
        let swatch = |c: Color| Container::<Msg>::new().width(12.0).height(12.0).color(c);
        let menu = DropdownMenu::new("blu", true, Msg::Query, Msg::Toggle).options_widgets(
            vec![
                DropdownOption::new("Red"),
                DropdownOption::widget(swatch(Color::rgb(0.0, 0.0, 1.0))),
                DropdownOption::new("Blue"),
            ],
            Msg::Pick,
        );
        let ui = frame(&menu);
        let painted = texts(ui.scene());
        assert!(!painted.iter().any(|t| t == "Red"), "filtered out");
        assert!(painted.iter().any(|t| t == "Blue"), "kept");
        let swatches = count_colour(ui.scene(), Color::rgb(0.0, 0.0, 1.0));
        assert_eq!(swatches, 1, "and the one with no words is still there");
    }

    use crate::menu::probe::{blurred, crisp, Painted};

    /// The height of one choice on the default theme.
    fn row_height() -> f32 {
        frus_text::line_box(ROW_H, &label_style(None, Some(&Theme::default())), 0.0)
    }

    /// What is painted **under the field** — the field has an outline of its own, and it
    /// is the list this is about.
    fn below_field(scene: &frus_core::Scene) -> Vec<Painted> {
        crisp(scene)
            .into_iter()
            .filter(|r| r.rect.y >= 56.0)
            .collect()
    }

    fn themed_frame(menu: &DropdownMenu<Msg>, theme: &Theme) -> crate::Ui<Msg> {
        build_ui(menu, Size::new(400.0, 500.0), &Runtime::default(), theme)
    }

    /// **The choices float on one panel and draw no box of their own.** Each was an
    /// outlined, rounded rectangle four pixels from the next, with nothing behind them.
    ///
    /// Under the field: the panel in `surface_container`, rounded, unoutlined, as wide as
    /// the field, four rows and eight above and below — and the selected choice's tint,
    /// which is the one other thing a list at rest draws.
    #[test]
    fn the_choices_float_on_one_panel_and_draw_no_box_of_their_own() {
        let theme = Theme::default();
        let ui = frame(&menu("", true));
        let painted = below_field(ui.scene());
        assert_eq!(
            painted.len(),
            2,
            "the panel and the selection: {painted:#?}"
        );
        let (panel, selection) = (painted[0], painted[1]);
        assert_eq!(panel.color, theme.scheme.surface_container);
        assert!(panel.radius != frus_core::BorderRadius::ZERO);
        assert_eq!(panel.rect.width, DEFAULT_WIDTH);
        assert_eq!(panel.rect.height, 4.0 * row_height() + 16.0);
        assert!(
            painted.iter().all(|r| r.border_width == 0.0),
            "no choice is outlined: {painted:#?}"
        );
        assert_eq!(
            selection.color,
            theme.scheme.surface_container.lerp(theme.primary, 0.14)
        );
        assert_eq!(
            selection.rect.y,
            panel.rect.y + 8.0 + row_height(),
            "the second choice, on the panel"
        );
        assert_eq!(
            selection.radius,
            frus_core::BorderRadius::ZERO,
            "a strip, not a box"
        );
    }

    /// **The panel casts a shadow three high**, the reference's menu height, which a
    /// dropdown menu falls through to: a blur of twenty in the scheme's shadow at 30 %.
    #[test]
    fn the_panel_casts_a_shadow_three_high() {
        let theme = Theme::default();
        let ui = frame(&menu("", true));
        let panel = below_field(ui.scene())[0];
        let shadows = blurred(ui.scene());
        assert_eq!(shadows.len(), 1, "{shadows:#?}");
        assert_eq!(shadows[0].blur, 20.0);
        assert_eq!(shadows[0].rect.y, panel.rect.y + 6.0 - 20.0);
        assert_eq!(shadows[0].color, theme.scheme.shadow.with_alpha(0.30));
        assert!(
            blurred(frame(&menu("", false)).scene()).is_empty(),
            "shut, none"
        );
    }

    /// **The panel answers to the menu theme, and to its caller over the theme**, as the
    /// reference's dropdown menu answers to its anchor's.
    #[test]
    fn the_panel_answers_to_the_menu_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        let (surface, shade) = (Color::rgb(0.2, 0.4, 0.6), Color::rgba(0.0, 0.0, 0.5, 0.5));
        theme.widgets.menu.background = Some(surface);
        theme.widgets.menu.radius = Some(3.0);
        theme.widgets.menu.elevation = Some(1.0);
        theme.widgets.menu.shadow_color = Some(shade);
        theme.widgets.menu.padding = Some(Insets::new(20.0, 0.0, 20.0, 0.0));
        let ui = themed_frame(&menu("", true), &theme);
        let painted = below_field(ui.scene());
        let panel = *painted
            .iter()
            .find(|r| r.color == surface)
            .expect("the theme's");
        assert_eq!(panel.radius, frus_core::BorderRadius::uniform(3.0));
        assert_eq!(panel.rect.height, 4.0 * row_height() + 40.0);
        assert!(painted
            .iter()
            .any(|r| r.color == surface.lerp(theme.primary, 0.14)));
        let shadow = blurred(ui.scene())[0];
        assert_eq!((shadow.blur, shadow.color), (12.0, shade));

        let (own, own_shade) = (Color::rgb(0.9, 0.9, 0.1), Color::rgba(0.5, 0.0, 0.0, 0.4));
        let told = menu("", true)
            .menu_background(own)
            .menu_radius(9.0)
            .menu_elevation(2.0)
            .menu_shadow_color(own_shade)
            .menu_padding(Insets::ZERO);
        let ui = themed_frame(&told, &theme);
        let panel = *below_field(ui.scene())
            .iter()
            .find(|r| r.color == own)
            .expect("the caller's");
        assert_eq!(panel.radius, frus_core::BorderRadius::uniform(9.0));
        assert_eq!(panel.rect.height, 4.0 * row_height());
        let shadow = blurred(ui.scene())[0];
        assert_eq!((shadow.blur, shadow.color), (16.0, own_shade));
    }

    /// **A transparent shadow colour casts nothing** (milestone 529), on the widget or on
    /// the menu theme.
    #[test]
    fn a_transparent_shadow_casts_nothing() {
        let clear = menu("", true).menu_shadow_color(Color::TRANSPARENT);
        assert!(blurred(frame(&clear).scene()).is_empty());
        let mut theme = Theme::default();
        theme.widgets.menu.shadow_color = Some(Color::TRANSPARENT);
        assert!(blurred(themed_frame(&menu("", true), &theme).scene()).is_empty());
    }

    /// **A long list scrolls inside the panel, and is clipped there.** The panel stays
    /// whole — two rows showing and its room above and below, and no clip of its own, since
    /// a clip would take its shadow with it — and the rows are drawn clipped to the
    /// viewport between the two rooms, so the choices past the fold are built, laid out
    /// below it and not shown.
    #[test]
    fn a_long_list_scrolls_and_clips_inside_the_panel() {
        let theme = Theme::default();
        let ui = frame(&menu("", true).max_visible(2));
        let painted = below_field(ui.scene());
        let panel = *painted
            .iter()
            .find(|r| r.color == theme.scheme.surface_container)
            .expect("a panel");
        assert_eq!(panel.rect.height, 2.0 * row_height() + 16.0);
        let window = Rect::new(0.0, 0.0, 400.0, 500.0);
        assert_eq!(
            panel.clip, window,
            "the panel is clipped by nothing but the window"
        );
        assert_eq!(
            blurred(ui.scene())[0].clip,
            window,
            "and neither is its shadow"
        );

        // The selected choice, the second, is inside the viewport: its clip is the viewport.
        let selected = *painted
            .iter()
            .find(|r| r.color == theme.scheme.surface_container.lerp(theme.primary, 0.14))
            .expect("the selected choice is drawn");
        let clip = selected.clip;
        assert_eq!(
            clip.y,
            panel.rect.y + 8.0,
            "the viewport starts inside the room"
        );
        assert_eq!(clip.height, 2.0 * row_height(), "and shows two rows");
        assert!(clip.x >= panel.rect.x && clip.x + clip.width <= panel.rect.x + panel.rect.width);

        fn text_at(primitives: &[Primitive], words: &str) -> Option<Point> {
            primitives.iter().find_map(|p| match p {
                Primitive::Text { position, text, .. } if text == words => Some(*position),
                Primitive::Layer { primitives, .. } => text_at(primitives, words),
                _ => None,
            })
        }
        let blue = text_at(ui.scene().primitives(), "Blue").expect("the last choice is built");
        assert!(
            blue.y >= clip.y + clip.height,
            "the last choice is past the fold, so there is something to scroll to: {blue:?}"
        );
    }

    fn count_colour(scene: &frus_core::Scene, wanted: Color) -> usize {
        fn walk(primitives: &[Primitive], wanted: Color) -> usize {
            primitives
                .iter()
                .map(|p| match p {
                    Primitive::Rect { color, .. } if *color == wanted => 1,
                    Primitive::Layer { primitives, .. } => walk(primitives, wanted),
                    _ => 0,
                })
                .sum()
        }
        walk(scene.primitives(), wanted)
    }
}
