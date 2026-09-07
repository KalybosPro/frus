//! [`Autocomplete`]: a text field with a floating **suggestion list**. Controlled:
//! the application supplies the value **and** the suggestions (already filtered);
//! the list only floats when it is non-empty. Adjustable width
//! ([`width`](Autocomplete::width)); the suggestions take keyboard focus.
//!
//! Each suggestion **brings out** the portion matching the query (in the `primary`
//! color), and the **active** suggestion ([`active`](Autocomplete::active), stepped
//! through from the keyboard) is highlighted — like a `DropdownButton`'s menu.

use std::rc::Rc;

use frus_core::{Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::scroll::SingleChildScrollView;
use crate::textinput::TextField;
use crate::theme::Theme;
use crate::widget::Widget;

const DEFAULT_WIDTH: f32 = 260.0;
const ROW_H: f32 = 32.0;
const PAD_X: f32 = 10.0;
/// The vertical gap between suggestions.
const ROW_GAP: f32 = 2.0;

/// The style the suggestions are drawn in: what the caller said, else what the theme says,
/// else the reference's — the reference's own suggestion list is built out of list tiles,
/// whose title is `bodyLarge`.
///
/// **Resolved once**, so that the number the box is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.autocomplete.text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_large)
        .resolved()
}

/// The height of one suggestion — the floor, or the line if the type asks for more.
fn row_height(style: &ResolvedTextStyle) -> f32 {
    frus_text::line_box(ROW_H, style, 0.0)
}

/// The portion of the label (in **character** indices) matching the query
/// (a case-insensitive search). `None` if the query is empty or absent.
fn match_range(label: &str, query: &str) -> Option<(usize, usize)> {
    if query.trim().is_empty() {
        return None;
    }
    let ll: Vec<char> = label.to_lowercase().chars().collect();
    let ql: Vec<char> = query.to_lowercase().chars().collect();
    if ql.is_empty() || ql.len() > ll.len() {
        return None;
    }
    (0..=ll.len() - ql.len())
        .find(|&i| ll[i..i + ql.len()] == ql[..])
        .map(|i| (i, i + ql.len()))
}

/// A clickable suggestion. The portion matching `query` is brought out (in the
/// `primary` color); the **active** one (stepped to from the keyboard) is highlighted.
struct Suggestion<Msg> {
    label: String,
    /// The current query, used to highlight the matching portion.
    query: String,
    width: f32,
    /// The **active** suggestion (the one that would be picked): a tinted background.
    active: bool,
    text_style: Option<TextStyle>,
    message: Msg,
}

impl<Msg> Suggestion<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(row_height(&label_style(self.text_style, theme))),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Suggestion<Msg> {
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
        // A menu panel is a distinct area within the surface, the `surface_container`
        // role (`menu_anchor.dart:4240`). The active suggestion: a `primary`-tinted
        // background; hover on top.
        let panel = theme.scheme.surface_container;
        let base = if self.active {
            panel.lerp(theme.primary, 0.14)
        } else {
            panel
        };
        let bg = theme.state_layer(base, theme.on_surface, &status);
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, theme.border.fade(o));

        let style = label_style(self.text_style, Some(theme));
        let ty = bounds.y + (bounds.height - style.line_height()) * 0.5;
        let chars: Vec<char> = self.label.chars().collect();
        let normal = theme.on_surface.fade(o);
        let hilite = theme.primary.fade(o);
        let mut x = bounds.x + PAD_X;
        // Segments [before | match | after]: the match in `primary`.
        let segments: [(std::ops::Range<usize>, frus_core::Color); 3] =
            match match_range(&self.label, &self.query) {
                Some((i, j)) => [(0..i, normal), (i..j, hilite), (j..chars.len(), normal)],
                None => [(0..chars.len(), normal), (0..0, normal), (0..0, normal)],
            };
        for (range, color) in segments {
            if range.is_empty() {
                continue;
            }
            let text: String = chars[range].iter().collect();
            let width = frus_text::measure_resolved(&text, &style).width;
            scene.text(Point::new(x, ty), text, &style, color);
            x += width;
        }
    }

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// A text field with suggestions.
pub struct Autocomplete<Msg> {
    value: String,
    width: f32,
    /// The **active** suggestion (stepped to from the keyboard, highlighted), if any.
    active: Option<usize>,
    /// The maximum number of visible suggestions; beyond that the list **scrolls**.
    max_visible: Option<usize>,
    on_input: Rc<dyn Fn(String) -> Msg>,
    on_pick: Rc<dyn Fn(String) -> Msg>,
    labels: Vec<String>,
    text_style: Option<TextStyle>,
    /// `[field]` or `[field, list]`.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Autocomplete<Msg> {
    /// Creates a field: the current value, `on_input(text)` on typing, and
    /// `on_pick(suggestion)` when a suggestion is chosen.
    pub fn new(
        value: impl Into<String>,
        on_input: impl Fn(String) -> Msg + 'static,
        on_pick: impl Fn(String) -> Msg + 'static,
    ) -> Self {
        let mut ac = Self {
            value: value.into(),
            width: DEFAULT_WIDTH,
            active: None,
            max_visible: None,
            on_input: Rc::new(on_input),
            on_pick: Rc::new(on_pick),
            labels: Vec::new(),
            text_style: None,
            children: Vec::new(),
        };
        ac.rebuild();
        ac
    }

    /// The suggestions' type, over the theme's and the reference's.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// The width of the field and the suggestions, in logical pixels (260 by default).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// The index of the **active** suggestion (highlighted; chosen from the keyboard).
    /// The app moves it along (arrows) and commits it (Enter) — the state stays with it.
    pub fn active(mut self, index: usize) -> Self {
        self.active = Some(index);
        self.rebuild();
        self
    }

    /// Limits the number of **visible** suggestions; beyond that the floating list
    /// **scrolls** (a viewport bounded to `n` rows) instead of stretching forever.
    pub fn max_visible(mut self, rows: usize) -> Self {
        self.max_visible = Some(rows.max(1));
        self.rebuild();
        self
    }

    /// Adds a suggestion to the floating list.
    pub fn suggestion(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        // The field: rebuilt on every setting (width, value). The shared `on_input`
        // callback (an Rc) is captured by the field.
        let on_input = self.on_input.clone();
        let input = TextField::new(self.value.clone())
            .width(self.width)
            .on_input(move |text| on_input(text));
        self.children = vec![Box::new(input)];

        if !self.labels.is_empty() {
            let mut list = Flex::column().gap(ROW_GAP);
            for (index, label) in self.labels.iter().enumerate() {
                list = list.child(Suggestion {
                    label: label.clone(),
                    query: self.value.clone(),
                    width: self.width,
                    active: self.active == Some(index),
                    text_style: self.text_style,
                    message: (self.on_pick)(label.clone()),
                });
            }
            // Past the threshold, the list scrolls in a viewport bounded to `n` rows.
            match self.max_visible {
                Some(n) if self.labels.len() > n => {
                    // The rows' own height, not the floor: a viewport counted at `ROW_H`
                    // while the rows are taller shows `n` rows minus a sliver of each —
                    // which is what the reader's font setting does to them, and it is
                    // applied here because `resolved()` applies it.
                    //
                    // `None` for the theme, and it is the one thing a builder cannot have:
                    // this list is built before any theme exists. So an application that
                    // retypesets suggestions through `widgets.autocomplete` and *also*
                    // caps them with `max_visible` should say it on the widget instead,
                    // where the same number reaches both the rows and their viewport.
                    let row = row_height(&label_style(self.text_style, None));
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
}

impl<Msg: Clone> Widget<Msg> for Autocomplete<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Input(String),
        Pick(String),
    }

    #[test]
    fn no_suggestions_no_overlay() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick);
        assert!(Widget::<Msg>::overlay(&ac).is_none());
    }

    #[test]
    fn suggestions_float_and_pick() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot");
        assert!(Widget::<Msg>::overlay(&ac).is_some());
        // The list holds both suggestions; the 1st emits Pick("apple").
        let list = &Widget::<Msg>::children(&ac)[1];
        assert_eq!(list.children().len(), 2);
        assert_eq!(
            list.children()[0].on_click(),
            Some(Msg::Pick("apple".to_string()))
        );
    }

    #[test]
    fn match_range_is_case_insensitive_substring() {
        assert_eq!(match_range("Apricot", "ap"), Some((0, 2)));
        assert_eq!(match_range("pineapple", "APPLE"), Some((4, 9)));
        assert_eq!(match_range("apple", ""), None);
        assert_eq!(match_range("apple", "xyz"), None);
    }

    #[test]
    fn matched_portion_is_drawn_as_its_own_segment() {
        // The query "ap" on "apricot" → segments "ap" (brought out) + "ricot".
        let ac = Autocomplete::new("ap", Msg::Input, Msg::Pick).suggestion("apricot");
        let (list, _) = Widget::<Msg>::overlay(&ac).unwrap();
        let ui = build_ui(
            list,
            Size::new(280.0, 80.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let texts: Vec<String> = ui
            .scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.contains(&"ap".to_string()),
            "the matching portion is isolated: {texts:?}"
        );
        assert!(
            texts.contains(&"ricot".to_string()),
            "the remainder is isolated: {texts:?}"
        );
    }

    #[test]
    fn active_suggestion_is_highlighted() {
        let ac = Autocomplete::new("ap", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot")
            .active(1);
        let (list, _) = Widget::<Msg>::overlay(&ac).unwrap();
        let ui = build_ui(
            list,
            Size::new(280.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let theme = Theme::default();
        let tint = theme.scheme.surface_container.lerp(theme.primary, 0.14);
        let has_tint = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Rect { color, .. } if color.fade(1.0) == tint.fade(1.0)
            )
        });
        assert!(has_tint, "the active suggestion is highlighted");
    }

    #[test]
    fn long_list_scrolls_when_capped() {
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .max_visible(2)
            .suggestion("a1")
            .suggestion("a2")
            .suggestion("a3")
            .suggestion("a4");
        let (overlay, _) = Widget::<Msg>::overlay(&ac).unwrap();
        // The overlay is a SingleChildScrollView bounded to 2 rows (viewport = 2*ROW_H + 1 gap).
        let expected = 2.0 * ROW_H + ROW_GAP;
        assert!(
            matches!(Widget::<Msg>::style(overlay).height, Dimension::Length(v) if (v - expected).abs() < 0.5),
            "viewport bounded to 2 rows",
        );
        // It does scroll over all 4 suggestions.
        assert_eq!(overlay.children()[0].children().len(), 4);
    }

    #[test]
    fn short_list_is_not_wrapped_in_scroll() {
        // Below the threshold: a bare list, with no bounded viewport.
        let ac = Autocomplete::new("a", Msg::Input, Msg::Pick)
            .max_visible(5)
            .suggestion("a1")
            .suggestion("a2");
        let (overlay, _) = Widget::<Msg>::overlay(&ac).unwrap();
        // A direct list: its children are the 2 suggestions, not a SingleChildScrollView one level down.
        assert_eq!(overlay.children().len(), 2);
        assert_eq!(
            overlay.children()[0].on_click(),
            Some(Msg::Pick("a1".to_string()))
        );
    }

    #[test]
    fn field_and_suggestions_are_keyboard_reachable() {
        // Keyboard descent goes through the focus system: the field, then the
        // suggestions, enter the Tab cycle (the down arrow from the single-line field
        // moves the focus to the 1st suggestion).
        let ac = Autocomplete::new("ap", Msg::Input, Msg::Pick)
            .suggestion("apple")
            .suggestion("apricot");
        let ui = build_ui(
            &ac,
            Size::new(280.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let first = ui.focus_next(None, true);
        assert!(first.is_some(), "the field is focusable");
        let second = ui.focus_next(first, true);
        assert!(
            second.is_some() && second != first,
            "a suggestion follows the field in the cycle"
        );
    }
}
