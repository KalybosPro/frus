//! [`SearchAnchor`]: the view a [`SearchBar`](crate::SearchBar) opens.
//!
//! Milestone 469 built the bar and stopped there, which is where the reference also stops
//! if you only reach for `SearchBar` — a bar alone is a useful thing. This is the other
//! half: press the bar and a surface grows out of it holding the same query, a rule, and
//! whatever the application thinks you meant.
//!
//! ```ignore
//! SearchAnchor::new(app.searching, &app.query)
//!     .hint("Search mail")
//!     .on_open(Msg::OpenSearch)
//!     .on_close(Msg::CloseSearch)
//!     .on_input(Msg::Query)
//!     .on_clear(Msg::Query(String::new()))
//!     .suggestion(ListTile::new("flight to lisbon").on_tap(Msg::Pick(0)))
//!     .suggestion(ListTile::new("flights from lisbon").on_tap(Msg::Pick(1)))
//! ```
//!
//! ## It is open because the application says so
//!
//! The reference pushes a `PopupRoute` and keeps `isOpen` inside a controller, because a
//! Flutter widget owns its state. Nothing here owns state: the application holds `open` in
//! its model and gets a message back, the way [`Drawer`](crate::Drawer) has since milestone
//! 46 and [`ExpansionTile`](crate::ExpansionTile) since 463. So `on_open` is what the bar
//! emits when pressed, and `on_close` is what the back arrow and the scrim emit — and a
//! test that wants the view open just says so.
//!
//! ## Two views, one widget
//!
//! On a phone the view **is** the screen; on a desktop it hangs under the bar. The
//! reference decides by platform (`search_anchor.dart:554`) and so does this, at compile
//! time, with [`full_screen`](SearchAnchor::full_screen) to say otherwise. The difference
//! is four things and no more: how big it is, whether its corners are rounded, how tall its
//! header is, and whether anything shows around it.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use frus_core::{BorderRadius, BorderSide, Color, Insets, Rect, Scene, ShapeBorder, TextStyle};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;
use crate::widgettheme::resolve_shape;

/// The floating view's corner (`search_anchor.dart:1949`). A full-screen one is square:
/// it has no corners to round, being the screen.
pub const SEARCH_VIEW_RADIUS: f32 = 28.0;
/// The narrowest a floating view goes, and the shortest (`search_anchor.dart:1958`).
pub const SEARCH_VIEW_MIN_WIDTH: f32 = 360.0;
/// See [`SEARCH_VIEW_MIN_WIDTH`].
pub const SEARCH_VIEW_MIN_HEIGHT: f32 = 240.0;
/// The header's height once the view is the screen (`search_anchor.dart:1933`).
///
/// Sixteen more than a bar's 56, and the sixteen is the point: a header that is the top of
/// a screen has a thumb reaching past it and needs the room, where one hanging under a bar
/// on a desktop does not.
pub const SEARCH_VIEW_FULL_SCREEN_HEADER: f32 = 72.0;
/// How far off the page the view sits (`search_anchor.dart:1939`).
const ELEVATION: f32 = 6.0;
/// The room either side of the header's contents (`search_anchor.dart:1961`).
const BAR_PADDING_X: f32 = 8.0;

/// **Whether a search view takes the whole screen**, decided the way the reference decides
/// it (`search_anchor.dart:554`): by platform, at compile time here rather than from a
/// `TargetPlatform` read at run time.
///
/// A phone opens the screen; a desktop hangs a panel under the bar. It is a question about
/// how much room there is and how a search is got out of, not about taste — but
/// [`SearchAnchor::full_screen`] overrules it, because a tablet in landscape is a phone
/// that should be answering *no*.
const fn full_screen_by_default() -> bool {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        true
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        false
    }
}

/// A search bar and the view it opens.
pub struct SearchAnchor<Msg> {
    open: bool,
    query: String,
    hint: Option<String>,
    view_hint: Option<String>,
    anchor: RefCell<Option<Box<dyn Widget<Msg>>>>,
    suggestions: RefCell<Vec<Box<dyn Widget<Msg>>>>,
    view_leading: RefCell<Option<Box<dyn Widget<Msg>>>>,
    view_trailing: RefCell<Vec<Box<dyn Widget<Msg>>>>,
    on_open: Option<Msg>,
    on_close: Option<Msg>,
    on_clear: Option<Msg>,
    on_input: Option<Rc<dyn Fn(String) -> Msg>>,
    on_submit: Option<Msg>,
    enabled: bool,
    full_screen: Option<bool>,
    background: Option<Color>,
    elevation: Option<f32>,
    shape: Option<ShapeBorder>,
    radius: Option<BorderRadius>,
    side: Option<BorderSide>,
    divider_color: Option<Color>,
    header_height: Option<f32>,
    header_text_style: Option<TextStyle>,
    header_hint_style: Option<TextStyle>,
    min_width: Option<f32>,
    min_height: Option<f32>,
    bar_padding: Option<Insets>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> SearchAnchor<Msg> {
    /// An anchor showing `query`, with its view open or shut.
    pub fn new(open: bool, query: impl Into<String>) -> Self {
        Self {
            open,
            query: query.into(),
            hint: None,
            view_hint: None,
            anchor: RefCell::new(None),
            suggestions: RefCell::new(Vec::new()),
            view_leading: RefCell::new(None),
            view_trailing: RefCell::new(Vec::new()),
            on_open: None,
            on_close: None,
            on_clear: None,
            on_input: None,
            on_submit: None,
            enabled: true,
            full_screen: None,
            background: None,
            elevation: None,
            shape: None,
            radius: None,
            side: None,
            divider_color: None,
            header_height: None,
            header_text_style: None,
            header_hint_style: None,
            min_width: None,
            min_height: None,
            bar_padding: None,
            built: OnceCell::new(),
        }
    }

    /// What the **bar** says while it is empty.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self.rebuild();
        self
    }

    /// What the **view's header** says, when it differs. Unset, the bar's — a view that
    /// renamed the thing being searched the moment it opened would be answering a
    /// different question.
    pub fn view_hint(mut self, hint: impl Into<String>) -> Self {
        self.view_hint = Some(hint.into());
        self.rebuild();
        self
    }

    /// The widget in the flow, in place of the [`SearchBar`](crate::SearchBar) this builds
    /// by default — the reference's `builder` (`search_anchor.dart:151`). A bar is the
    /// usual anchor and not the only one: an icon button in an app bar is a search anchor
    /// too.
    pub fn anchor(self, anchor: impl Widget<Msg> + 'static) -> Self {
        self.anchor_boxed(Box::new(anchor))
    }

    /// [`Self::anchor`], for a widget already boxed.
    pub fn anchor_boxed(mut self, anchor: Box<dyn Widget<Msg>>) -> Self {
        *self.anchor.borrow_mut() = Some(anchor);
        self.rebuild();
        self
    }

    /// Adds a row to the view — the reference's `suggestionsBuilder`
    /// (`search_anchor.dart:152`), which is a builder there because the widget owns the
    /// query and has to re-ask on every keystroke. Here the application owns it, so these
    /// are simply what it decided the query meant.
    pub fn suggestion(self, suggestion: impl Widget<Msg> + 'static) -> Self {
        self.suggestion_boxed(Box::new(suggestion))
    }

    /// [`Self::suggestion`], for a widget already boxed.
    pub fn suggestion_boxed(mut self, suggestion: Box<dyn Widget<Msg>>) -> Self {
        self.suggestions.borrow_mut().push(suggestion);
        self.rebuild();
        self
    }

    /// The widget before the view's field, in place of the back arrow it builds by
    /// default (`search_anchor.dart:1040`).
    pub fn view_leading(self, leading: impl Widget<Msg> + 'static) -> Self {
        self.view_leading_boxed(Box::new(leading))
    }

    /// [`Self::view_leading`], for a widget already boxed.
    pub fn view_leading_boxed(mut self, leading: Box<dyn Widget<Msg>>) -> Self {
        *self.view_leading.borrow_mut() = Some(leading);
        self.rebuild();
        self
    }

    /// Adds a widget after the view's field, in place of the clear cross it builds by
    /// default.
    pub fn view_trailing(self, trailing: impl Widget<Msg> + 'static) -> Self {
        self.view_trailing_boxed(Box::new(trailing))
    }

    /// [`Self::view_trailing`], for a widget already boxed.
    pub fn view_trailing_boxed(mut self, trailing: Box<dyn Widget<Msg>>) -> Self {
        self.view_trailing.borrow_mut().push(trailing);
        self.rebuild();
        self
    }

    /// What the bar emits when it is pressed. Without it the anchor opens nothing, which
    /// is a bar and not an anchor.
    pub fn on_open(mut self, message: Msg) -> Self {
        self.on_open = Some(message);
        self.rebuild();
        self
    }

    /// What the back arrow and the scrim emit.
    pub fn on_close(mut self, message: Msg) -> Self {
        self.on_close = Some(message);
        self.rebuild();
        self
    }

    /// What the clear cross emits. **The cross only exists when there is something to
    /// clear** (`search_anchor.dart:1048`) — and only when there is somewhere for it to
    /// say so.
    pub fn on_clear(mut self, message: Msg) -> Self {
        self.on_clear = Some(message);
        self.rebuild();
        self
    }

    /// What the view's field emits on every keystroke.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Rc::new(on_input));
        self.rebuild();
        self
    }

    /// What it emits when the search is confirmed.
    pub fn on_submit(mut self, message: Msg) -> Self {
        self.on_submit = Some(message);
        self.rebuild();
        self
    }

    /// Whether the anchor can be used at all.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Whether the view takes the whole screen.
    ///
    /// Unset, the platform decides, the way the reference decides it
    /// (`search_anchor.dart:554`): a phone opens the screen, a desktop hangs a panel under
    /// the bar. It is a question about how much room there is and how a search is got out
    /// of, not about taste — but a tablet in landscape is a phone that should be answering
    /// *no*, which is what this is for.
    pub fn full_screen(mut self, full_screen: bool) -> Self {
        self.full_screen = Some(full_screen);
        self.rebuild();
        self
    }

    /// The view's fill. Unset, the scheme's `surface_container_high`
    /// (`search_anchor.dart:1936`) — the same rung the bar takes, so the one grows out of
    /// the other rather than changing colour on the way.
    pub fn view_background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.rebuild();
        self
    }

    /// How far off the page the view sits. Unset, `6`.
    pub fn view_elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// The view's shape. Unset, corners of [`SEARCH_VIEW_RADIUS`] floating and square
    /// full-screen.
    pub fn view_shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Its corners, for the ordinary case. See [`Self::view_shape`].
    pub fn view_radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// An outline around it. Unset, none (`search_anchor.dart:1944`).
    pub fn view_side(mut self, width: f32, color: Color) -> Self {
        self.side = Some(BorderSide { width, color });
        self
    }

    /// The rule under the header. Unset, the scheme's `outline`
    /// (`search_anchor.dart:1967`).
    pub fn divider_color(mut self, color: Color) -> Self {
        self.divider_color = Some(color);
        self.rebuild();
        self
    }

    /// The header's height. Unset, [`SEARCH_VIEW_FULL_SCREEN_HEADER`] full-screen and the
    /// bar's own height floating.
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = Some(height);
        self.rebuild();
        self
    }

    /// The header's type. Unset, `bodyLarge` in `on_surface`.
    pub fn header_text_style(mut self, style: TextStyle) -> Self {
        self.header_text_style = Some(style);
        self.rebuild();
        self
    }

    /// The header hint's type. Unset, `bodyLarge` in `on_surface_variant`.
    pub fn header_hint_style(mut self, style: TextStyle) -> Self {
        self.header_hint_style = Some(style);
        self.rebuild();
        self
    }

    /// The narrowest the view goes. Unset, [`SEARCH_VIEW_MIN_WIDTH`].
    pub fn view_min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self.rebuild();
        self
    }

    /// The shortest. Unset, [`SEARCH_VIEW_MIN_HEIGHT`].
    pub fn view_min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self.rebuild();
        self
    }

    /// The room either side of the header's contents. Unset, `8`.
    pub fn view_bar_padding(mut self, padding: Insets) -> Self {
        self.bar_padding = Some(padding);
        self.rebuild();
        self
    }

    fn rebuild(&mut self) {
        self.built.take();
    }

    /// Whether this view is the screen.
    fn is_full_screen(&self) -> bool {
        self.full_screen.unwrap_or_else(full_screen_by_default)
    }

    /// The view's shape, resolved. **A full-screen view has no corners to round**: it is
    /// the screen, and a rounded screen is a rounded rectangle with the wallpaper showing
    /// through the corners (`search_anchor.dart:1947`).
    fn view_outline(&self, theme: &Theme) -> ShapeBorder {
        let t = &theme.widgets.search_view;
        let shape = resolve_shape(
            self.shape,
            t.shape,
            self.radius.or(t.radius),
            if self.is_full_screen() {
                ShapeBorder::rounded(BorderRadius::ZERO)
            } else {
                ShapeBorder::rounded(BorderRadius::uniform(SEARCH_VIEW_RADIUS))
            },
        );
        match self.side.or(t.side) {
            Some(side) => shape.with_side(side),
            None => shape,
        }
    }

    /// The header's height, resolved.
    fn header(&self, theme: &Theme) -> f32 {
        self.header_height
            .or(theme.widgets.search_view.header_height)
            .unwrap_or(if self.is_full_screen() {
                SEARCH_VIEW_FULL_SCREEN_HEADER
            } else {
                crate::SEARCH_BAR_HEIGHT
            })
    }

    /// The bar in the flow: the caller's, or one this builds.
    fn build_anchor(&self) -> Box<dyn Widget<Msg>> {
        if let Some(anchor) = self.anchor.borrow_mut().take() {
            return anchor;
        }
        let mut bar = crate::SearchBar::<Msg>::new(self.query.clone())
            .enabled(self.enabled)
            // **The bar an anchor builds does not take typing.** Pressing it opens the
            // view, and the view is where the words are entered — a bar that took both
            // would have two fields holding one query, and the reference's own anchor bar
            // is a tap target for exactly this reason (`search_anchor.dart:577`).
            .read_only(true);
        if let Some(hint) = &self.hint {
            bar = bar.hint(hint.clone());
        }
        if let Some(message) = &self.on_open {
            bar = bar.on_tap(message.clone());
        }
        Box::new(bar)
    }

    /// The view's header: the same field again, on the view's own surface.
    fn build_header(&self, theme: &Theme) -> Box<dyn Widget<Msg>> {
        let words = crate::localizations::of();
        let room = self
            .bar_padding
            .or(theme.widgets.search_view.bar_padding)
            .unwrap_or(Insets::new(0.0, BAR_PADDING_X, 0.0, BAR_PADDING_X));
        let mut header = crate::SearchBar::<Msg>::new(self.query.clone())
            // **Transparent and flat.** The view is already a raised surface; a raised
            // pill inside it would be a second one, with its own shadow falling on the
            // suggestions. The reference says the same three things
            // (`search_anchor.dart:1163`).
            .background_color(Color::TRANSPARENT)
            .elevation(0.0)
            .min_width(0.0)
            .height(self.header(theme))
            .padding(room);
        if let Some(hint) = self.view_hint.as_ref().or(self.hint.as_ref()) {
            header = header.hint(hint.clone());
        }
        if let Some(style) = self
            .header_text_style
            .or(theme.widgets.search_view.header_text_style)
        {
            header = header.text_style(style);
        }
        if let Some(style) = self
            .header_hint_style
            .or(theme.widgets.search_view.header_hint_style)
        {
            header = header.hint_style(style);
        }
        header = match self.view_leading.borrow_mut().take() {
            Some(leading) => header.leading_boxed(leading),
            None => {
                let mut back = crate::BackButton::<Msg>::new().icon_size(20.0);
                if let Some(message) = &self.on_close {
                    back = back.on_press(message.clone());
                }
                header.leading(back)
            }
        };
        let told = std::mem::take(&mut *self.view_trailing.borrow_mut());
        if told.is_empty() {
            // **A cross with nothing to clear is a cross that lies.** It appears with the
            // first character and goes when the last one does (`search_anchor.dart:1048`).
            if !self.query.is_empty() {
                if let Some(message) = &self.on_clear {
                    header = header.trailing(
                        crate::IconButton::new(crate::Icons::Close)
                            .icon_size(20.0)
                            .label(words.clear_button_label())
                            .on_press(message.clone()),
                    );
                }
            }
        } else {
            for trailing in told {
                header = header.trailing_boxed(trailing);
            }
        }
        Box::new(header)
    }

    /// The view: the header, the rule, the suggestions.
    fn build_view(&self, theme: &Theme) -> Box<dyn Widget<Msg>> {
        let t = &theme.widgets.search_view;
        let full = self.is_full_screen();
        let screen = crate::MediaQuery::of().size;

        // **The column fills the view, and the list fills the column.** Two flex factors
        // for one idea, and both are needed: the view has a floor of 240 that its content
        // does not reach, so a column sized by its content leaves the list nothing to grow
        // into and a scroll view with nothing to grow into collapses to nothing and clips
        // everything in it. The picture was a view with a header, a rule, and a blank.
        let mut column = Flex::<Msg>::column()
            .flex(1.0)
            .child_boxed(self.build_header(theme));
        column = column.child(
            crate::Divider::new().height(1.0).color(
                self.divider_color
                    .or(t.divider_color)
                    .unwrap_or(theme.scheme.outline),
            ),
        );
        let mut list = Flex::<Msg>::column();
        for suggestion in std::mem::take(&mut *self.suggestions.borrow_mut()) {
            list = list.child_boxed(suggestion);
        }
        column = column.child(
            crate::SingleChildScrollView::<Msg>::new()
                .flex(1.0)
                .child(list),
        );

        Box::new(SearchView {
            children: vec![Box::new(column)],
            full,
            screen: (screen.width, screen.height),
            min_width: self
                .min_width
                .or(t.min_width)
                .unwrap_or(SEARCH_VIEW_MIN_WIDTH),
            min_height: self
                .min_height
                .or(t.min_height)
                .unwrap_or(SEARCH_VIEW_MIN_HEIGHT),
            background: self
                .background
                .or(t.background_color)
                .unwrap_or(theme.scheme.surface_container_high),
            elevation: self.elevation.or(t.elevation).unwrap_or(ELEVATION),
            shape: self.view_outline(theme),
        })
    }

    /// The whole thing: the anchor, and the view over it while it is open.
    fn assemble(&self, theme: &Theme) -> Vec<Box<dyn Widget<Msg>>> {
        let anchor = self.build_anchor();
        let mut portal = crate::OverlayPortal::<Msg>::new_boxed(anchor);
        if self.open && self.enabled {
            // **Where the view goes.** Under the bar on a desktop, over everything on a
            // phone — the second one is `Center` and not a placement of its own because a
            // view that is the size of the screen is centred on it by arithmetic.
            let placement = if self.is_full_screen() {
                Placement::Center
            } else {
                Placement::Below
            };
            portal = portal.overlay_boxed(self.build_view(theme), placement);
            if let Some(message) = &self.on_close {
                portal = portal.dismiss(message.clone());
            }
        }
        vec![Box::new(portal)]
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for SearchAnchor<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    fn build_themed(&self, theme: &Theme) {
        let _ = self.built.set(self.assemble(theme));
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble(&Theme::default()))
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// The floating surface itself: a box of the right size, filled and raised.
///
/// Separate from [`SearchAnchor`] because the two answer different questions — the anchor
/// decides *whether* there is a view and what goes in it, and this decides how big it is
/// and what it looks like. It is private: an application asks the anchor.
struct SearchView<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    full: bool,
    /// The window, for a full-screen view to fill.
    screen: (f32, f32),
    min_width: f32,
    min_height: f32,
    background: Color,
    elevation: f32,
    shape: ShapeBorder,
}

impl<Msg: Clone> Widget<Msg> for SearchView<Msg> {
    /// **The screen, or a panel with a floor.**
    ///
    /// A full-screen view asks for the window exactly; a floating one asks for nothing in
    /// particular and refuses to go below 360 × 240. The reference expresses both as
    /// constraints on one box (`search_anchor.dart:1115`); here the two are the same two
    /// numbers said twice.
    fn style(&self) -> Style {
        if self.full {
            Style {
                width: Dimension::Length(self.screen.0),
                height: Dimension::Length(self.screen.1),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        } else {
            Style {
                min_width: Dimension::Length(self.min_width),
                min_height: Dimension::Length(self.min_height),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        if self.elevation > 0.0 {
            let blur = self.elevation * 2.0 + 4.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + self.elevation * 0.5 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                BorderRadius::uniform(blur),
                blur,
            );
        }
        scene.draw_shape(bounds, self.shape, self.background.fade(o));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Open,
        Close,
        Clear,
        Query(String),
    }

    fn anchor(open: bool, query: &str) -> SearchAnchor<Msg> {
        SearchAnchor::new(open, query)
            .hint("Search mail")
            .on_open(Msg::Open)
            .on_close(Msg::Close)
            .on_input(Msg::Query)
    }

    /// The portal the anchor built: `[bar]` shut, `[bar, view]` open.
    fn portal(anchor: &SearchAnchor<Msg>, theme: &Theme) -> Vec<String> {
        anchor.build_themed(theme);
        let built = Widget::<Msg>::children(anchor);
        assert_eq!(built.len(), 1, "one portal");
        built[0]
            .children()
            .iter()
            .map(|child| child.debug_name().to_string())
            .collect()
    }

    /// Every widget in a built subtree whose short type name matches.
    fn find<'a>(node: &'a dyn Widget<Msg>, name: &str, out: &mut Vec<&'a dyn Widget<Msg>>) {
        if node.debug_name() == name {
            out.push(node);
        }
        for child in node.children() {
            find(&**child, name, out);
        }
    }

    fn all<'a>(
        anchor: &'a SearchAnchor<Msg>,
        name: &str,
        theme: &Theme,
    ) -> Vec<&'a dyn Widget<Msg>> {
        anchor.build_themed(theme);
        let mut out = Vec::new();
        for child in Widget::<Msg>::children(anchor) {
            find(&**child, name, &mut out);
        }
        out
    }

    /// **It is open because the application says so.** No route, no controller: the model
    /// holds the flag and the widget draws what the flag says.
    #[test]
    fn the_view_is_there_when_the_application_says_it_is() {
        let theme = Theme::default();
        assert_eq!(portal(&anchor(false, ""), &theme), vec!["SearchBar"]);
        assert_eq!(
            portal(&anchor(true, ""), &theme),
            vec!["SearchBar", "SearchView"],
        );
        assert_eq!(
            portal(&anchor(true, "").enabled(false), &theme),
            vec!["SearchBar"],
            "and an anchor that cannot be used opens nothing, whatever the flag says"
        );
    }

    /// **The bar an anchor builds does not take typing.** Pressing it opens the view, and
    /// the view is where the words go — a bar that took both would be two fields holding
    /// one query.
    #[test]
    fn the_anchors_own_bar_is_a_tap_target() {
        let theme = Theme::default();
        let shut = anchor(false, "");
        let bars = all(&shut, "SearchBar", &theme);
        assert_eq!(bars.len(), 1);
        assert_eq!(
            bars[0].on_click(),
            Some(Msg::Open),
            "pressing it opens the view"
        );
    }

    /// **A cross with nothing to clear is a cross that lies.** It appears with the first
    /// character and goes when the last one does.
    #[test]
    fn the_clear_cross_comes_and_goes_with_the_query() {
        let theme = Theme::default();
        let empty = anchor(true, "").on_clear(Msg::Clear);
        assert_eq!(
            all(&empty, "IconButton", &theme).len(),
            0,
            "nothing to clear, nothing to press"
        );

        let typed = anchor(true, "lisbon").on_clear(Msg::Clear);
        let crosses = all(&typed, "IconButton", &theme);
        assert_eq!(crosses.len(), 1);
        assert_eq!(crosses[0].on_click(), Some(Msg::Clear));

        let nowhere = anchor(true, "lisbon");
        assert_eq!(
            all(&nowhere, "IconButton", &theme).len(),
            0,
            "and a cross with nowhere to say so is not offered either"
        );
    }

    /// **A full-screen view has no corners to round**: it is the screen, and a rounded
    /// screen is a rounded rectangle with the wallpaper showing through the corners.
    #[test]
    fn the_screen_is_square_and_the_panel_is_not() {
        let theme = Theme::default();
        assert_eq!(
            anchor(true, "").full_screen(true).view_outline(&theme),
            ShapeBorder::rounded(BorderRadius::ZERO)
        );
        assert_eq!(
            anchor(true, "").full_screen(false).view_outline(&theme),
            ShapeBorder::rounded(BorderRadius::uniform(SEARCH_VIEW_RADIUS))
        );
    }

    /// And a screen's header is taller than a panel's, by the sixteen a thumb needs.
    #[test]
    fn the_screens_header_is_the_taller_one() {
        let theme = Theme::default();
        assert_eq!(
            anchor(true, "").full_screen(true).header(&theme),
            SEARCH_VIEW_FULL_SCREEN_HEADER
        );
        assert_eq!(
            anchor(true, "").full_screen(false).header(&theme),
            crate::SEARCH_BAR_HEIGHT
        );
        assert_eq!(
            anchor(true, "").header_height(40.0).header(&theme),
            40.0,
            "and a caller still has the last word"
        );
    }

    /// The floating view refuses to go below the reference's floor; the full-screen one
    /// asks for the window exactly.
    #[test]
    fn the_panel_has_a_floor_and_the_screen_has_a_size() {
        let panel = SearchView::<Msg> {
            children: Vec::new(),
            full: false,
            screen: (390.0, 844.0),
            min_width: SEARCH_VIEW_MIN_WIDTH,
            min_height: SEARCH_VIEW_MIN_HEIGHT,
            background: Color::BLACK,
            elevation: 0.0,
            shape: ShapeBorder::stadium(),
        };
        let style = Widget::<Msg>::style(&panel);
        assert_eq!(style.min_width, Dimension::Length(SEARCH_VIEW_MIN_WIDTH));
        assert_eq!(style.min_height, Dimension::Length(SEARCH_VIEW_MIN_HEIGHT));
        assert_eq!(style.width, Dimension::Auto, "and no width of its own");

        let screen = SearchView::<Msg> {
            full: true,
            ..panel
        };
        let style = Widget::<Msg>::style(&screen);
        assert_eq!(style.width, Dimension::Length(390.0));
        assert_eq!(style.height, Dimension::Length(844.0));
    }

    /// **A bar with no surface has no state layer.** The view's header is a bar painted
    /// onto a surface that is already there; lerping from a transparent ground toward
    /// `on_surface` would put a grey wash over the view rather than a highlight on the bar.
    #[test]
    fn a_bar_with_no_surface_does_not_light() {
        let theme = Theme::default();
        let hovered = Status {
            opacity: 1.0,
            hover_progress: 1.0,
            ..Default::default()
        };
        let fill = |bar: &crate::SearchBar<Msg>, status: Status| {
            let mut scene = Scene::new();
            bar.paint(Rect::new(0.0, 0.0, 400.0, 56.0), status, &theme, &mut scene);
            scene
                .primitives()
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Rect { color, blur, .. } if *blur == 0.0 => Some(*color),
                    _ => None,
                })
                .expect("it paints its pill")
        };
        let bare = crate::SearchBar::<Msg>::new("").background_color(Color::TRANSPARENT);
        assert_eq!(
            fill(&bare, hovered),
            fill(
                &bare,
                Status {
                    opacity: 1.0,
                    ..Default::default()
                }
            ),
            "there is no ground to light"
        );

        let solid = crate::SearchBar::<Msg>::new("");
        assert_ne!(
            fill(
                &solid,
                Status {
                    opacity: 1.0,
                    ..Default::default()
                }
            ),
            fill(&solid, hovered),
            "and one with a surface still answers a pointer"
        );
    }

    /// The view's hint falls back to the bar's: a view that renamed the thing being
    /// searched the moment it opened would be answering a different question.
    #[test]
    fn the_views_hint_falls_back_to_the_bars() {
        let theme = Theme::default();
        let plain = anchor(true, "");
        plain.build_themed(&theme);
        let heard: Vec<String> = all(&plain, "SearchBar", &theme)
            .iter()
            .filter_map(|bar| bar.semantics()?.label)
            .collect();
        assert_eq!(heard.len(), 2, "the bar and the view's header");
        assert!(heard.iter().all(|hint| hint == "Search mail"));

        let told = anchor(true, "").view_hint("Search all mail");
        let heard: Vec<String> = all(&told, "SearchBar", &theme)
            .iter()
            .filter_map(|bar| bar.semantics()?.label)
            .collect();
        assert_eq!(heard, vec!["Search mail", "Search all mail"]);
    }

    /// The builders are order-independent, the subtree being thrown away by each of them.
    #[test]
    fn saying_it_afterwards_says_it() {
        let theme = Theme::default();
        let after = SearchAnchor::<Msg>::new(true, "")
            .on_close(Msg::Close)
            .full_screen(true)
            .hint("Search");
        assert_eq!(portal(&after, &theme).len(), 2);
        assert_eq!(after.header(&theme), SEARCH_VIEW_FULL_SCREEN_HEADER);
    }
}
