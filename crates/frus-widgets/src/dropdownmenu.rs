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

use frus_core::{Insets, Rect, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::dropdown::{label_style, option_row, DropdownOption};
use crate::flex::Flex;
use crate::icons::Icons;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::scroll::SingleChildScrollView;
use crate::textinput::TextField;
use crate::theme::Theme;
use crate::widget::Widget;

/// The field's width when the caller has not said otherwise.
const DEFAULT_WIDTH: f32 = 240.0;
/// The gap between the choices, matching [`crate::DropdownButton`]'s list.
const ROW_GAP: f32 = 4.0;
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
        let mut list = Flex::column().gap(ROW_GAP);
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
                on_click,
            ));
        }
        let rows = showing.len();
        match self.max_visible {
            Some(n) if rows > n => {
                // The rows' own height, not the floor: a viewport counted at `ROW_H`
                // while the rows are taller shows `n` rows minus a sliver of each. `None`
                // for the theme is the one thing a builder cannot have — the list is built
                // before any theme exists — so an application that retypesets the choices
                // through the theme *and* caps them should say the size on the widget.
                let row = frus_text::line_box(ROW_H, &label_style(self.text_style, None), 0.0);
                let viewport = n as f32 * row + (n as f32 - 1.0) * ROW_GAP;
                self.children.push(Box::new(
                    SingleChildScrollView::new()
                        .width(self.width)
                        .height(viewport)
                        .child(list),
                ));
            }
            _ => self.children.push(Box::new(list)),
        }
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
        let row = |n: f32| {
            ui.hit(Point::new(120.0, 56.0 + ROW_H * (n + 0.5) + ROW_GAP * n))
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
