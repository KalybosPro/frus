//! [`SearchBar`]: the raised pill an application searches from.
//!
//! It is a [`TextField`](crate::TextField) with no container of its own inside a container
//! of the framework's — which is the whole design. A search bar is not a form field: it
//! does not float a label, it does not sit on a line, and it is raised off the page rather
//! than sunk into it, because it is a control that goes *over* the content it filters
//! rather than one more row of a form.
//!
//! ```ignore
//! SearchBar::new(&app.query)
//!     .hint("Search mail")
//!     .leading(Icon::new(Icons::Menu))
//!     .trailing(IconButton::new(Icons::Close).on_press(Msg::Clear))
//!     .on_input(Msg::Query)
//!     .on_submit(Msg::Search)
//! ```
//!
//! Everything it paints is a default and nothing is a rule: the fill, the shadow and how
//! far off the page it sits, the outline, the shape, the room inside it, the two type
//! steps, and the box it asks for — each resolved from the instance, then
//! [`SearchBarTheme`](crate::widgettheme::SearchBarTheme), then the scheme's role.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use frus_core::{BorderRadius, BorderSide, Color, Insets, Rect, Scene, ShapeBorder, TextStyle};
use frus_layout::{Align, Dimension, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgettheme::resolve_shape;

/// The box a search bar asks for (`search_anchor.dart:1908`).
///
/// The floor is the interesting one: **360 wide**, which is a phone's width and not a
/// coincidence — a search bar narrower than that is a text field wearing a shadow. A
/// caller who has less room says so; the widget does not quietly become something else.
pub const SEARCH_BAR_MIN_WIDTH: f32 = 360.0;
/// And the ceiling: past this a search bar stops being a control and becomes a banner.
pub const SEARCH_BAR_MAX_WIDTH: f32 = 800.0;
/// Its height (`search_anchor.dart:1908`).
pub const SEARCH_BAR_HEIGHT: f32 = 56.0;
/// How far off the page it sits (`search_anchor.dart:1863`).
const ELEVATION: f32 = 6.0;
/// The room inside it, either side (`search_anchor.dart:1896`).
///
/// The reference applies it **twice** — once around the whole row and once around the
/// field inside it — so the gap between a leading icon and the first letter is sixteen,
/// and the gap from the bar's edge to that icon is eight. Both are that one number.
const PADDING_X: f32 = 8.0;
/// What a disabled search bar fades to (`search_anchor.dart:48`).
///
/// This is the one place the framework's own rule gives way. A disabled control here
/// [flattens rather than fades](crate::disabled) — but the reference dims the whole bar,
/// icons and shadow and all, and there is nothing here to flatten *to*: a raised pill's
/// unavailability is the raising going away, not a grey rectangle.
const DISABLED_OPACITY: f32 = 0.38;

/// A raised pill holding a search field, with a slot before it and any number after.
pub struct SearchBar<Msg> {
    value: String,
    hint: Option<String>,
    leading: RefCell<Option<Box<dyn Widget<Msg>>>>,
    trailing: RefCell<Vec<Box<dyn Widget<Msg>>>>,
    on_input: Option<Rc<dyn Fn(String) -> Msg>>,
    on_submit: Option<Msg>,
    on_tap: Option<Msg>,
    enabled: bool,
    read_only: bool,
    background: Option<Color>,
    shadow_color: Option<Color>,
    elevation: Option<f32>,
    shape: Option<ShapeBorder>,
    radius: Option<BorderRadius>,
    side: Option<BorderSide>,
    padding: Option<Insets>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    height: Option<f32>,
    text_style: Option<TextStyle>,
    hint_style: Option<TextStyle>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> SearchBar<Msg> {
    /// A search bar showing `value`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            hint: None,
            leading: RefCell::new(None),
            trailing: RefCell::new(Vec::new()),
            on_input: None,
            on_submit: None,
            on_tap: None,
            enabled: true,
            read_only: false,
            background: None,
            shadow_color: None,
            elevation: None,
            shape: None,
            radius: None,
            side: None,
            padding: None,
            min_width: None,
            max_width: None,
            height: None,
            text_style: None,
            hint_style: None,
            built: OnceCell::new(),
        }
    }

    /// What the bar says while it is empty (`search_anchor.dart:1454`).
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self.rebuild();
        self
    }

    /// The widget before the field — usually a menu button, a back arrow or a magnifier.
    pub fn leading(self, leading: impl Widget<Msg> + 'static) -> Self {
        self.leading_boxed(Box::new(leading))
    }

    /// [`Self::leading`], for a widget already boxed.
    pub fn leading_boxed(mut self, leading: Box<dyn Widget<Msg>>) -> Self {
        *self.leading.borrow_mut() = Some(leading);
        self.rebuild();
        self
    }

    /// Adds a widget **after** the field. The reference asks for no more than two
    /// (`search_anchor.dart:1465`) and does not enforce it; neither does this, because a
    /// bar that silently dropped the third would be harder to debug than a crowded one.
    pub fn trailing(self, trailing: impl Widget<Msg> + 'static) -> Self {
        self.trailing_boxed(Box::new(trailing))
    }

    /// [`Self::trailing`], for a widget already boxed.
    pub fn trailing_boxed(mut self, trailing: Box<dyn Widget<Msg>>) -> Self {
        self.trailing.borrow_mut().push(trailing);
        self.rebuild();
        self
    }

    /// What to emit on every keystroke.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Rc::new(on_input));
        self.rebuild();
        self
    }

    /// What to emit when the search is confirmed.
    pub fn on_submit(mut self, message: Msg) -> Self {
        self.on_submit = Some(message);
        self.rebuild();
        self
    }

    /// What to emit when the bar itself is pressed — how a bar that only *looks* like a
    /// field opens the thing that really searches (`search_anchor.dart:1470`). It goes
    /// with [`Self::read_only`].
    pub fn on_tap(mut self, message: Msg) -> Self {
        self.on_tap = Some(message);
        self.rebuild();
        self
    }

    /// Whether the bar can be used at all. A disabled one is inert and fades.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// The bar takes no typing, only presses — a bar whose real search happens somewhere
    /// else. It is **not** [`disabled`](Self::enabled): it still lights, still answers a
    /// press, and still reads as available, because it is.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self.rebuild();
        self
    }

    /// The bar's fill. Unset, the scheme's `surface_container_high`
    /// (`search_anchor.dart:1859`).
    pub fn background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The colour its shadow is cast in. Unset, the scheme's `shadow`.
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = Some(color);
        self
    }

    /// How far off the page it sits. Unset, `6` — and `0.0` is the flat bar an
    /// application that separates by colour wants.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Its shape. Unset, a stadium (`search_anchor.dart:1892`).
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Its corners, for the ordinary case of a rounded rectangle. See [`Self::shape`].
    pub fn radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// An outline around it. Unset, none at all — the reference has no default side
    /// (`search_anchor.dart:1888`), a raised surface being told apart by its shadow.
    pub fn side(mut self, width: f32, color: Color) -> Self {
        self.side = Some(BorderSide { width, color });
        self
    }

    /// The room inside it. Unset, `8` either side.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self.rebuild();
        self
    }

    /// The narrowest it will go. Unset, [`SEARCH_BAR_MIN_WIDTH`].
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// The widest. Unset, [`SEARCH_BAR_MAX_WIDTH`].
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Its height. Unset, [`SEARCH_BAR_HEIGHT`].
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The value's type. Unset, `bodyLarge` in `on_surface`
    /// (`search_anchor.dart:1900`).
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// The hint's type. Unset, the value's, then `bodyLarge` in `on_surface_variant`
    /// (`search_anchor.dart:1903`) — **the reference falls back to the value's style
    /// before its own default**, so a bar told to use one type does not say its hint in
    /// another.
    pub fn hint_style(mut self, style: TextStyle) -> Self {
        self.hint_style = Some(style);
        self.rebuild();
        self
    }

    /// Throws the assembled subtree away, so the builders stay order-independent.
    fn rebuild(&mut self) {
        self.built.take();
    }

    /// The room inside it, resolved.
    fn room(&self, theme: &Theme) -> Insets {
        self.padding
            .or(theme.widgets.search_bar.padding)
            .unwrap_or(Insets::new(0.0, PADDING_X, 0.0, PADDING_X))
    }

    /// Its shape, resolved.
    fn outline(&self, theme: &Theme) -> ShapeBorder {
        let t = &theme.widgets.search_bar;
        let shape = resolve_shape(
            self.shape,
            t.shape,
            self.radius.or(t.radius),
            ShapeBorder::stadium(),
        );
        match self.side.or(t.side) {
            Some(side) => shape.with_side(side),
            None => shape,
        }
    }

    /// The value's type, resolved.
    fn value_style(&self, theme: &Theme) -> TextStyle {
        self.text_style
            .or(theme.widgets.search_bar.text_style)
            .unwrap_or_else(|| crate::theme::type_scale(Some(theme)).body_large)
    }

    /// **The hint's type, resolved on five rungs rather than three.**
    ///
    /// The caller's hint style, then the theme's hint style, then the caller's *value*
    /// style, then the theme's value style, then the default
    /// (`search_anchor.dart:1727`). The two middle rungs are the point: a bar told to say
    /// its value in one type would otherwise say its hint in another, which is the kind of
    /// thing nobody notices until the field is empty.
    fn hint_style_of(&self, theme: &Theme) -> TextStyle {
        let t = &theme.widgets.search_bar;
        self.hint_style
            .or(t.hint_style)
            .or(self.text_style)
            .or(t.text_style)
            .unwrap_or_else(|| crate::theme::type_scale(Some(theme)).body_large)
    }

    /// The bar's subtree: the leading slot, the field, the trailing ones.
    fn assemble(&self, theme: &Theme) -> Vec<Box<dyn Widget<Msg>>> {
        let text_style = self.value_style(theme);
        let hint_style = self.hint_style_of(theme);

        let mut field = crate::TextField::<Msg>::new(self.value.clone())
            .borderless()
            .dense(true)
            .enabled(self.enabled)
            .style(crate::TextFieldStyle {
                text_color: text_style.color.or(Some(theme.scheme.on_surface)),
                // A field's hint and its floating label are one colour here, and a search
                // bar has no label — so this is the hint's colour and nothing else's.
                label_color: hint_style.color.or(Some(theme.scheme.on_surface_variant)),
                // The reference pads the field **again**, inside the row it already padded
                // (`search_anchor.dart:1790`), so the gap from a leading icon to the first
                // letter is twice the gap from the bar's edge to that icon.
                padding_x: Some(self.room(theme).left),
                ..Default::default()
            });
        if let Some(size) = text_style.size {
            field = field.size(size);
        }
        if let Some(hint) = &self.hint {
            field = field.placeholder(hint.clone());
        }
        if self.read_only {
            field = field.read_only();
        }
        if let Some(on_input) = &self.on_input {
            let on_input = on_input.clone();
            field = field.on_input(move |value| on_input(value));
        }
        if let Some(message) = &self.on_submit {
            field = field.on_submit(message.clone());
        }
        let mut row = Flex::<Msg>::row().align(Align::Center);
        if let Some(leading) = self.leading.borrow_mut().take() {
            row = row.child_boxed(leading);
        }
        row = row.child(crate::Expanded::new(field));
        for trailing in std::mem::take(&mut *self.trailing.borrow_mut()) {
            row = row.child_boxed(trailing);
        }
        vec![Box::new(row)]
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for SearchBar<Msg> {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(self.height.unwrap_or(SEARCH_BAR_HEIGHT)),
            min_width: Dimension::Length(self.min_width.unwrap_or(SEARCH_BAR_MIN_WIDTH)),
            max_width: Dimension::Length(self.max_width.unwrap_or(SEARCH_BAR_MAX_WIDTH)),
            align: Align::Center,
            padding: Insets::new(0.0, PADDING_X, 0.0, PADDING_X),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let t = &theme.widgets.search_bar;
        let room = self.room(theme);
        Style {
            height: Dimension::Length(self.height.or(t.height).unwrap_or(SEARCH_BAR_HEIGHT)),
            min_width: Dimension::Length(
                self.min_width
                    .or(t.min_width)
                    .unwrap_or(SEARCH_BAR_MIN_WIDTH),
            ),
            max_width: Dimension::Length(
                self.max_width
                    .or(t.max_width)
                    .unwrap_or(SEARCH_BAR_MAX_WIDTH),
            ),
            padding: room,
            ..Widget::<Msg>::style(self)
        }
    }

    fn build_themed(&self, theme: &Theme) {
        let _ = self.built.set(self.assemble(theme));
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble(&Theme::default()))
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        // **A disabled bar fades, shadow and all.** See [`DISABLED_OPACITY`].
        let o = status.opacity * if self.enabled { 1.0 } else { DISABLED_OPACITY };
        let t = &theme.widgets.search_bar;

        let depth = self.elevation.or(t.elevation).unwrap_or(ELEVATION);
        if depth > 0.0 {
            let blur = depth * 2.0 + 4.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 0.5 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                self.shadow_color
                    .or(t.shadow_color)
                    .unwrap_or(theme.scheme.shadow)
                    .with_alpha(0.30)
                    .fade(o),
                BorderRadius::uniform(blur),
                blur,
            );
        }

        let base = self
            .background
            .or(t.background_color)
            .unwrap_or(theme.scheme.surface_container_high);
        // Nothing lights on a bar that cannot be used. **Nor on one with no surface**: a
        // state layer is a lerp *from the ground* toward the ink, and a transparent ground
        // lerped toward `on_surface` is a grey wash over whatever is behind the bar rather
        // than a highlight on the bar. That is the case a search **view**'s header is
        // — a bar painted onto a surface that is already there — and it is why the
        // reference sets that header's overlay to transparent (`search_anchor.dart:1166`)
        // rather than leaving it to the default.
        //
        // Otherwise the framework's one rule: a lerp from the ground toward the ink,
        // resolved opaquely, answering hover, focus and press at once
        // (`search_anchor.dart:1874`).
        let fill = if self.enabled && base.a > 0.0 {
            theme.state_layer(base, theme.scheme.on_surface, &status)
        } else {
            base
        };
        scene.draw_shape(bounds, self.outline(theme), fill.fade(o));
    }

    /// A press on the bar is a press on the bar, wherever it lands — the reference wraps
    /// the whole thing in an ink well rather than only the field
    /// (`search_anchor.dart:1772`), so that the eight pixels beside a leading icon are
    /// not dead.
    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.on_tap.clone()).flatten()
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    /// **Announced as a search field**, which is a role in its own right on both
    /// platforms — the reference says so explicitly (`search_anchor.dart:1792`) rather
    /// than letting a text input inside a pill be read as a text input.
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let mut props = frus_core::SemanticsProperties::new(frus_core::Role::TextInput)
            .value(self.value.clone())
            .disabled(!self.enabled);
        if let Some(hint) = &self.hint {
            props = props.label(hint.clone());
        }
        if self.enabled {
            props = props.clickable();
        }
        Some(props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Open,
        Query(String),
    }

    fn bar() -> SearchBar<Msg> {
        SearchBar::new("").hint("Search mail")
    }

    fn scene_of(widget: &dyn Widget<Msg>, status: Status, theme: &Theme) -> Scene {
        let mut scene = Scene::new();
        widget.paint(
            Rect::new(0.0, 0.0, 400.0, SEARCH_BAR_HEIGHT),
            status,
            theme,
            &mut scene,
        );
        scene
    }

    fn lit(status: Status) -> Status {
        Status {
            opacity: 1.0,
            ..status
        }
    }

    /// **The box the reference asks for**, floor and ceiling both. The floor is the one
    /// that surprises people: a search bar will not go below 360 unless it is told to,
    /// because a search bar narrower than a phone is a text field wearing a shadow.
    #[test]
    fn it_asks_for_the_box_the_reference_asks_for() {
        let theme = Theme::default();
        let style = Widget::<Msg>::style_themed(&bar(), &theme);
        assert_eq!(style.min_width, Dimension::Length(SEARCH_BAR_MIN_WIDTH));
        assert_eq!(style.max_width, Dimension::Length(SEARCH_BAR_MAX_WIDTH));
        assert_eq!(style.height, Dimension::Length(SEARCH_BAR_HEIGHT));

        let narrow = Widget::<Msg>::style_themed(&bar().min_width(240.0).height(40.0), &theme);
        assert_eq!(narrow.min_width, Dimension::Length(240.0));
        assert_eq!(narrow.height, Dimension::Length(40.0));
    }

    /// **The hint's type falls back through the value's.** Five rungs, and the two in the
    /// middle are the whole point: a bar told to say its value in one type must not say
    /// its hint in another.
    #[test]
    fn the_hint_takes_the_values_type_before_its_own_default() {
        let theme = Theme::default();
        let plain = bar();
        assert_eq!(
            plain.hint_style_of(&theme),
            crate::theme::type_scale(Some(&theme)).body_large,
            "with nothing said, both are the default step"
        );

        let told = bar().text_style(TextStyle::new(22.0));
        assert_eq!(
            told.hint_style_of(&theme).size,
            Some(22.0),
            "the hint follows the value it stands in for"
        );

        let both = bar()
            .text_style(TextStyle::new(22.0))
            .hint_style(TextStyle::new(12.0));
        assert_eq!(
            both.hint_style_of(&theme).size,
            Some(12.0),
            "and a hint told its own type keeps it"
        );
        assert_eq!(both.value_style(&theme).size, Some(22.0));
    }

    /// A disabled bar is inert to the tap and to the tab, announced as unavailable, and
    /// **fades rather than flattening** — the one place the framework's own disabled rule
    /// gives way, because a raised pill's unavailability is the raising going away.
    #[test]
    fn a_disabled_bar_is_inert_and_faded() {
        let theme = Theme::default();
        let off = bar().on_tap(Msg::Open).enabled(false);
        assert_eq!(Widget::<Msg>::on_click(&off), None);
        assert!(!Widget::<Msg>::focusable(&off));
        let semantics = Widget::<Msg>::semantics(&off).expect("still announced");
        assert!(semantics.disabled);
        assert!(!semantics.clickable);

        let on = bar().on_tap(Msg::Open);
        assert_eq!(Widget::<Msg>::on_click(&on), Some(Msg::Open));
        assert!(Widget::<Msg>::focusable(&on));

        // And the pill is painted at the reference's 38 %.
        let alpha = |bar: &SearchBar<Msg>| {
            scene_of(bar, lit(Status::default()), &theme)
                .primitives()
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(color.a),
                    _ => None,
                })
                .expect("the bar paints its pill")
        };
        assert!(
            (alpha(&off) - DISABLED_OPACITY).abs() < 0.01,
            "{}",
            alpha(&off)
        );
        assert!((alpha(&on) - 1.0).abs() < 0.01);
    }

    /// Nothing lights on a bar that cannot be used: a state layer is the promise of an
    /// interaction, and there is none.
    #[test]
    fn a_disabled_bar_does_not_light_under_a_pointer() {
        let theme = Theme::default();
        let hovered = Status {
            opacity: 1.0,
            hover_progress: 1.0,
            ..Default::default()
        };
        let fill = |bar: &SearchBar<Msg>, status: Status| {
            scene_of(bar, status, &theme)
                .primitives()
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(*color),
                    _ => None,
                })
                .expect("the bar paints its pill")
        };
        let resting = fill(&bar(), lit(Status::default()));
        assert_ne!(
            fill(&bar(), hovered),
            resting,
            "an available bar answers a pointer"
        );
        let off = bar().enabled(false);
        assert_eq!(
            fill(&off, hovered).fade(1.0),
            fill(&off, lit(Status::default())).fade(1.0),
            "an unavailable one has nothing to say to it"
        );
    }

    /// **The field inside has no container of its own.** Two containers deep is what a
    /// search bar looks like when the field keeps its own box: the pill's corners cut by a
    /// rounded rectangle sitting inside them.
    #[test]
    fn the_field_inside_paints_no_container() {
        let theme = Theme::default();
        let boxed = crate::TextField::<Msg>::new("x").filled();
        let bare = crate::TextField::<Msg>::new("x").borderless();
        let rects = |field: &crate::TextField<Msg>| {
            let mut scene = Scene::new();
            field.paint(
                Rect::new(0.0, 0.0, 200.0, 48.0),
                lit(Status::default()),
                &theme,
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { .. }))
                .count()
        };
        assert!(rects(&boxed) > 0, "a filled field draws its container");
        assert_eq!(rects(&bare), 0, "a borderless one draws none of it");
    }

    /// A press anywhere on the bar is a press on the bar — the reference wraps the whole
    /// pill rather than only the field, so the eight pixels beside a leading icon are not
    /// dead.
    #[test]
    fn the_whole_pill_answers_a_press() {
        let bar = bar().read_only(true).on_tap(Msg::Open);
        assert_eq!(Widget::<Msg>::on_click(&bar), Some(Msg::Open));
        assert_eq!(
            Widget::<Msg>::on_click(&SearchBar::<Msg>::new("")),
            None,
            "and a bar with nothing to open answers nothing"
        );
    }

    /// It is announced as a field with a value, not as a pill.
    #[test]
    fn it_announces_its_value() {
        let bar = SearchBar::<Msg>::new("mail").hint("Search mail");
        let semantics = Widget::<Msg>::semantics(&bar).expect("announced");
        assert_eq!(semantics.role, frus_core::Role::TextInput);
        assert_eq!(semantics.value.as_deref(), Some("mail"));
        assert_eq!(semantics.label.as_deref(), Some("Search mail"));
    }

    /// The builders are order-independent: the subtree is thrown away by each of them.
    #[test]
    fn saying_it_afterwards_says_it() {
        let theme = Theme::default();
        let after = SearchBar::<Msg>::new("")
            .on_input(Msg::Query)
            .hint("Search");
        after.build_themed(&theme);
        let mut found = 0;
        fn walk<Msg: Clone + 'static>(node: &dyn Widget<Msg>, found: &mut usize) {
            if node.debug_name() == "TextField" {
                *found += 1;
            }
            for child in node.children() {
                walk(&**child, found);
            }
        }
        for child in Widget::<Msg>::children(&after) {
            walk(&**child, &mut found);
        }
        assert_eq!(found, 1, "one field, built once");
    }

    /// Its shape carries the outline, so a bar told to have one is still whatever shape it
    /// was told to be.
    #[test]
    fn an_outline_rides_on_the_shape() {
        let theme = Theme::default();
        assert_eq!(bar().outline(&theme), ShapeBorder::stadium());
        let edged = bar().side(2.0, Color::rgb8(200, 0, 0));
        assert_eq!(edged.outline(&theme).side().width, 2.0);
        let squared = bar().radius(4.0).side(1.0, Color::rgb8(0, 0, 0));
        assert!(matches!(
            squared.outline(&theme),
            ShapeBorder::RoundedRectangle { .. }
        ));
    }
}
