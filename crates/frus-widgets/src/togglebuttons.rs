//! [`ToggleButtons`]: a bank of buttons that **share their edges**, of which any number
//! can be on at once.
//!
//! ```ignore
//! ToggleButtons::new(vec![true, false, true], Msg::Toggle)
//!     .child(Text::new("B").weight(FontWeight::Bold))
//!     .child(Text::new("I").italic())
//!     .child(Text::new("U").underline())
//! ```
//!
//! It is **not** [`SegmentedButton`](crate::SegmentedButton), and the difference is worth
//! knowing before reaching for either. A segmented button is one control with one answer:
//! the caller hands it labels and gets back an index. This hands the caller the buttons
//! themselves — each child is an arbitrary widget — and asks a separate yes or no about
//! every one of them. A bold/italic/underline bank is three answers, not one, and no
//! amount of styling turns a single-selection control into that.
//!
//! ## The edges are shared
//!
//! Three buttons touching have **four** vertical lines between and around them, not six.
//! Each button draws the edge facing the button before it — its *leading* border — and
//! only the last one draws the edge at the far end. So a seam is drawn once, at one
//! border's width, rather than twice at two, which is what a row of outlined buttons
//! pushed together gives and what makes that arrangement read as three controls instead
//! of one.
//!
//! A seam belongs to the two buttons either side of it, so **either** of them being on
//! colours it (`toggle_buttons.dart:596`). The reference's own defaults make all three
//! border colours the same, so nothing shows until a caller names
//! [`selected_border_color`](ToggleButtons::selected_border_color) — which is the point at
//! which getting the rule right starts to matter.
//!
//! ## Colour reaches the children through the theme
//!
//! The reference paints its children by handing the button a `foregroundColor` that
//! descendant `Text` and `Icon` widgets inherit. This does the same thing through the
//! mechanism this framework already has: each child is wrapped in a
//! [`Themed`](crate::Themed) that sets the text and icon colour for its subtree. A caller
//! who put a `Text` in a button gets the selected colour on it without saying so, and one
//! who set a colour on that `Text` themselves still wins — which is the same order of
//! precedence, expressed with the framework's own parts.

use std::cell::{OnceCell, RefCell};

use frus_core::{BorderRadius, Color, Insets, Rect, Scene, TextDirection, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::disabled::{disabled_content, over_surface};
use crate::interaction::Status;
use crate::rowcolumn::VerticalDirection;
use crate::theme::Theme;
use crate::themed::Themed;
use crate::widget::Widget;
use crate::widgetstate::{WidgetState, WidgetStateProperty, WidgetStates};
use crate::widgettheme::ToggleButtonsTheme;

/// The smallest a toggle button may be, either way: the framework's tap target
/// (`toggle_buttons.dart:762`, which reads `kMinInteractiveDimension`).
pub const TOGGLE_BUTTON_MIN_SIZE: f32 = 48.0;

/// The hairline around and between the buttons (`toggle_buttons.dart:243`).
pub const TOGGLE_BUTTONS_BORDER_WIDTH: f32 = 1.0;

/// The opacity of a selected button's fill over the surface (`toggle_buttons.dart:929`).
const SELECTED_FILL_OPACITY: f32 = 0.12;

/// The opacity of an unselected button's content (`toggle_buttons.dart:751`).
const CONTENT_OPACITY: f32 = 0.87;

/// The opacity of the hairline, untold (`toggle_buttons.dart:601`).
const BORDER_OPACITY: f32 = 0.12;

/// **Which way a bank runs.**
///
/// Two variants and not three: [`crate::Axis`] carries a `Both`, which is a scrollable's
/// answer and not a direction anything can be laid out along. The crate already names an
/// axis per family for this reason ([`crate::ReorderAxis`], [`crate::IntrinsicAxis`]); that
/// there are now five of them is on the roadmap rather than solved here.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ToggleAxis {
    /// A row. The default.
    #[default]
    Horizontal,
    /// A column.
    Vertical,
}

/// Everything a caller may say about a bank's appearance, each part `None` until they do.
///
/// One struct rather than fourteen fields threaded one at a time: the buttons are built
/// from it, and a property added here is a property every button already reads.
#[derive(Clone, Debug, PartialEq)]
struct ToggleStyle {
    render_border: bool,
    border_width: Option<f32>,
    border_radius: Option<BorderRadius>,
    border_color: Option<Color>,
    selected_border_color: Option<Color>,
    disabled_border_color: Option<Color>,
    color: Option<Color>,
    selected_color: Option<Color>,
    disabled_color: Option<Color>,
    fill_color: Option<WidgetStateProperty<Color>>,
    text_style: Option<TextStyle>,
    min_width: Option<f32>,
    min_height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
}

impl Default for ToggleStyle {
    fn default() -> Self {
        Self {
            // The one field whose untold answer is not `None`: the reference renders the
            // border unless told otherwise (`toggle_buttons.dart:233`), and a bank with no
            // edges is a row of loose children.
            render_border: true,
            border_width: None,
            border_radius: None,
            border_color: None,
            selected_border_color: None,
            disabled_border_color: None,
            color: None,
            selected_color: None,
            disabled_color: None,
            fill_color: None,
            text_style: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        }
    }
}

impl ToggleStyle {
    fn themed(theme: &Theme) -> &ToggleButtonsTheme {
        &theme.widgets.toggle_buttons
    }

    /// The hairline's width — zero when the bank was told to draw none, so that every
    /// caller of this gets the *effective* number and the flag is tested once.
    fn border_width(&self, theme: &Theme) -> f32 {
        if !self.render_border {
            return 0.0;
        }
        self.border_width
            .or(Self::themed(theme).border_width)
            .unwrap_or(TOGGLE_BUTTONS_BORDER_WIDTH)
            .max(0.0)
    }

    /// The bank's corners — **the bank's**, not a button's. Each button takes the pair at
    /// its own end of this and squares off the rest; see [`ToggleButton::corners`].
    fn radius(&self, theme: &Theme) -> BorderRadius {
        self.border_radius
            .or(Self::themed(theme).border_radius)
            .unwrap_or(BorderRadius::ZERO)
    }

    /// A hairline's colour, for a button in the state given.
    ///
    /// All three rungs default to the same value (`toggle_buttons.dart:601`, `:635`,
    /// `:651`), which is the reference's M3 answer and is deliberate: the bank is one
    /// object, and its outline does not break up because one of its buttons is on. The
    /// three are still named apart because a caller who wants the seam to move — a
    /// selected pair marked out from the rest — needs somewhere to say so.
    fn border(&self, theme: &Theme, enabled: bool, selected: bool) -> Color {
        let t = Self::themed(theme);
        let told = if !enabled {
            self.disabled_border_color.or(t.disabled_border_color)
        } else if selected {
            self.selected_border_color.or(t.selected_border_color)
        } else {
            self.border_color.or(t.border_color)
        };
        told.unwrap_or_else(|| over_surface(theme, BORDER_OPACITY))
    }

    /// What is drawn **in** a button: its label, its glyph, whatever it holds.
    fn ink(&self, theme: &Theme, enabled: bool, selected: bool) -> Color {
        let t = Self::themed(theme);
        if !enabled {
            return self
                .disabled_color
                .or(t.disabled_color)
                // `on_surface` at 38 %, resolved opaque rather than handed to the GPU as
                // an alpha — the crate's one answer for unavailable.
                .unwrap_or_else(|| disabled_content(theme));
        }
        if selected {
            self.selected_color
                .or(t.selected_color)
                .unwrap_or(theme.scheme.primary)
        } else {
            self.color
                .or(t.color)
                .unwrap_or_else(|| over_surface(theme, CONTENT_OPACITY))
        }
    }

    /// A button's ground.
    ///
    /// Untold, a selected button takes the accent at 12 % **resolved in sRGB** rather than
    /// left translucent (`toggle_buttons.dart:929`). The reference's opacity tokens are a
    /// design language written in the space the colours are written in; this one blends in
    /// linear light, where 12 % paints at roughly what a third would give. Same reasoning
    /// as [`over_surface`], with the accent as the ink instead of `on_surface`.
    ///
    /// An unselected button has **no** ground at all, which is not the same as a
    /// transparent one being painted: nothing is drawn, so whatever the bank was put on
    /// shows through.
    fn fill(&self, theme: &Theme, enabled: bool, selected: bool) -> Color {
        let states = WidgetStates::EMPTY
            .set(WidgetState::Selected, enabled && selected)
            .set(WidgetState::Disabled, !enabled);
        // A colour the caller gave for *every* state answers here too: the reference's
        // field is a `Color?` that may hold a state property, and a plain colour there
        // fills selected and unselected alike (`toggle_buttons.dart:912`).
        if let Some(told) = self
            .fill_color
            .as_ref()
            .and_then(|own| own.resolve(states))
            .or_else(|| {
                Self::themed(theme)
                    .fill_color
                    .as_ref()
                    .and_then(|w| w.resolve(states))
            })
        {
            return *told;
        }
        if enabled && selected {
            theme
                .scheme
                .surface
                .lerp(theme.scheme.primary, SELECTED_FILL_OPACITY)
        } else {
            Color::TRANSPARENT
        }
    }

    /// The type a button's text takes (`toggle_buttons.dart:759`).
    fn text_style(&self, theme: &Theme) -> TextStyle {
        self.text_style
            .or(Self::themed(theme).text_style)
            .unwrap_or_else(|| crate::theme::type_scale(Some(theme)).body_medium)
    }

    fn min_width(&self, theme: &Theme) -> f32 {
        self.min_width
            .or(Self::themed(theme).min_width)
            .unwrap_or(TOGGLE_BUTTON_MIN_SIZE)
    }

    fn min_height(&self, theme: &Theme) -> f32 {
        self.min_height
            .or(Self::themed(theme).min_height)
            .unwrap_or(TOGGLE_BUTTON_MIN_SIZE)
    }

    fn max_width(&self, theme: &Theme) -> Option<f32> {
        self.max_width.or(Self::themed(theme).max_width)
    }

    fn max_height(&self, theme: &Theme) -> Option<f32> {
        self.max_height.or(Self::themed(theme).max_height)
    }
}

/// A bank of buttons sharing their edges, any number of which may be on.
pub struct ToggleButtons<Msg> {
    selected: Vec<bool>,
    on_pressed: Box<dyn Fn(usize) -> Msg>,
    enabled: bool,
    axis: ToggleAxis,
    vertical_direction: VerticalDirection,
    style: ToggleStyle,
    /// What the caller handed over, before it was wrapped. Taken once, on assembly.
    declared: RefCell<Vec<Box<dyn Widget<Msg>>>>,
    built: OnceCell<Vec<Box<dyn Widget<Msg>>>>,
}

impl<Msg: Clone + 'static> ToggleButtons<Msg> {
    /// A bank whose buttons are on or off as `selected` says, and which emits
    /// `on_pressed(index)` when one of them is pressed.
    ///
    /// The list is the application's, not the widget's: a bank does not remember which of
    /// its buttons are down, it draws the answer it was given. That is the shape every
    /// control in this framework has, and it is why there is no controller here where the
    /// reference has one — the model already holds the booleans.
    ///
    /// `selected` decides how many buttons there are as much as
    /// [`child`](Self::child) does; a child with no boolean beside it is not drawn, and a
    /// boolean with no child is nothing. The reference asserts the two lists are the same
    /// length, and this takes the shorter of them rather than panicking in a `view`.
    pub fn new(selected: Vec<bool>, on_pressed: impl Fn(usize) -> Msg + 'static) -> Self {
        Self {
            selected,
            on_pressed: Box::new(on_pressed),
            enabled: true,
            axis: ToggleAxis::Horizontal,
            vertical_direction: VerticalDirection::Down,
            style: ToggleStyle::default(),
            declared: RefCell::new(Vec::new()),
            built: OnceCell::new(),
        }
    }

    /// Adds a button holding `child` — typically a [`Text`](crate::Text) or an
    /// [`Icon`](crate::Icon), but anything at all (`toggle_buttons.dart:254`).
    #[must_use]
    pub fn child(self, child: impl Widget<Msg> + 'static) -> Self {
        self.child_boxed(Box::new(child))
    }

    /// The same, for a child that is already boxed.
    #[must_use]
    pub fn child_boxed(self, child: Box<dyn Widget<Msg>>) -> Self {
        self.declared.borrow_mut().push(child);
        self
    }

    /// Whether the bank can be pressed at all.
    ///
    /// The reference disables a bank by passing a null `onPressed`
    /// (`toggle_buttons.dart:269`), which is not expressible here — the callback is how
    /// the bank knows what to emit, and taking it away would take the type with it. So the
    /// flag is separate, and everything downstream of it is the same: no message, no tab
    /// stop, no splash, the disabled colours, and **no fill on a selected button**, which
    /// is the part that is easy to miss (a disabled bank never enters the selected state
    /// at all, `toggle_buttons.dart:739`).
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Which way the bank runs (`toggle_buttons.dart:454`). A row by default.
    #[must_use]
    pub fn direction(mut self, axis: ToggleAxis) -> Self {
        self.axis = axis;
        self
    }

    /// For a vertical bank, whether the first button is at the top or at the bottom
    /// (`toggle_buttons.dart:458`). Ignored by a row, whose order is the reading
    /// direction's to decide and which mirrors with the rest of the frame.
    #[must_use]
    pub fn vertical_direction(mut self, direction: VerticalDirection) -> Self {
        self.vertical_direction = direction;
        self
    }

    /// Whether the hairlines are drawn at all (`toggle_buttons.dart:400`).
    #[must_use]
    pub fn render_border(mut self, render: bool) -> Self {
        self.style.render_border = render;
        self
    }

    /// The hairline's width, around the bank and between its buttons
    /// (`toggle_buttons.dart:441`). Untold, [`TOGGLE_BUTTONS_BORDER_WIDTH`].
    #[must_use]
    pub fn border_width(mut self, width: f32) -> Self {
        self.style.border_width = Some(width);
        self
    }

    /// The **bank's** corners (`toggle_buttons.dart:449`): the first button takes the pair
    /// at its end and the last one the pair at the other, and the buttons between them are
    /// square. Untold, square all round, as the reference is.
    #[must_use]
    pub fn border_radius(mut self, radius: impl Into<BorderRadius>) -> Self {
        self.style.border_radius = Some(radius.into());
        self
    }

    /// The hairline around and beside a button that is off (`toggle_buttons.dart:409`).
    #[must_use]
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    /// The hairline around and beside a button that is on (`toggle_buttons.dart:417`).
    /// A seam between an on button and an off one takes **this**, from either side.
    #[must_use]
    pub fn selected_border_color(mut self, color: Color) -> Self {
        self.style.selected_border_color = Some(color);
        self
    }

    /// The hairline when the bank cannot be pressed (`toggle_buttons.dart:425`).
    #[must_use]
    pub fn disabled_border_color(mut self, color: Color) -> Self {
        self.style.disabled_border_color = Some(color);
        self
    }

    /// What is drawn in a button that is off (`toggle_buttons.dart:310`) — its text, its
    /// glyph, anything in its subtree that reads the theme for a colour.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    /// The same, for a button that is on (`toggle_buttons.dart:322`).
    #[must_use]
    pub fn selected_color(mut self, color: Color) -> Self {
        self.style.selected_color = Some(color);
        self
    }

    /// The same, when the bank cannot be pressed (`toggle_buttons.dart:333`).
    #[must_use]
    pub fn disabled_color(mut self, color: Color) -> Self {
        self.style.disabled_color = Some(color);
        self
    }

    /// A button's ground, per state (`toggle_buttons.dart:348`).
    ///
    /// The reference's field is a `Color?` that may secretly hold a state property, tested
    /// for at run time; here it is the state property, and
    /// `WidgetStateProperty::all(colour)` is the plain-colour case — which, as there, then
    /// fills every button rather than only the ones that are on.
    #[must_use]
    pub fn fill_color(mut self, fill: WidgetStateProperty<Color>) -> Self {
        self.style.fill_color = Some(fill);
        self
    }

    /// The type any text in the bank takes (`toggle_buttons.dart:291`). Its colour is
    /// ignored, as the reference ignores it: that is [`color`](Self::color)'s to answer,
    /// and it depends on the state the button is in.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.style.text_style = Some(style);
        self
    }

    /// The smallest a button may be (`toggle_buttons.dart:299`). Untold,
    /// [`TOGGLE_BUTTON_MIN_SIZE`] both ways — the tap target, which is a floor and not a
    /// size: a wider child makes a wider button.
    #[must_use]
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.style.min_width = Some(width);
        self.style.min_height = Some(height);
        self
    }

    /// The largest a button may be. Untold, unbounded.
    #[must_use]
    pub fn max_size(mut self, width: f32, height: f32) -> Self {
        self.style.max_width = Some(width);
        self.style.max_height = Some(height);
        self
    }

    /// How many buttons there actually are: a child needs a boolean beside it.
    fn count(&self) -> usize {
        self.declared.borrow().len().min(self.selected.len())
    }

    /// Wraps each declared child in its button, once.
    ///
    /// The colour cascade is a [`Themed`](crate::Themed) rather than a value computed
    /// here, because the theme is not known when a tree is built — the walk resolves it on
    /// the way down. The closure runs with the theme the button will actually be under,
    /// which is also what lets [`ToggleButtonsTheme`] have the last word.
    fn assemble(&self) -> Vec<Box<dyn Widget<Msg>>> {
        let count = self.count();
        let mut declared = self.declared.borrow_mut();
        declared.truncate(count);
        declared
            .drain(..)
            .enumerate()
            .map(|(index, child)| {
                let selected = self.selected[index];
                let enabled = self.enabled;
                let style = self.style.clone();
                let cascade = style.clone();
                let painted = Themed::tweak(
                    move |theme| {
                        let ink = cascade.ink(theme, enabled, selected);
                        let mut text = cascade.text_style(theme);
                        text.color = Some(ink);
                        theme.widgets.text.style = text;
                        theme.widgets.icon.color = Some(ink);
                    },
                    child,
                );
                Box::new(ToggleButton {
                    child: vec![Box::new(painted)],
                    index,
                    count,
                    selected,
                    // The seam faces the button before this one, and belongs to both.
                    previous_selected: index > 0 && self.selected[index - 1],
                    enabled: self.enabled,
                    axis: self.axis,
                    reversed: self.vertical_direction == VerticalDirection::Up,
                    style,
                    message: (self.on_pressed)(index),
                }) as Box<dyn Widget<Msg>>
            })
            .collect()
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for ToggleButtons<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: match (self.axis, self.vertical_direction) {
                (ToggleAxis::Horizontal, _) => FlexDirection::Row,
                (ToggleAxis::Vertical, VerticalDirection::Down) => FlexDirection::Column,
                (ToggleAxis::Vertical, VerticalDirection::Up) => FlexDirection::ColumnReverse,
            },
            // **Every button is as tall as the tallest**, which is what makes the bank one
            // object: the reference wraps the row in an `IntrinsicHeight`
            // (`toggle_buttons.dart:856`) to the same end. Stretching is the flexbox
            // spelling of it and costs no extra pass.
            align: Align::Stretch,
            justify: Justify::Start,
            // No gap. The buttons touch, and the line where they meet is a border.
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.built.get_or_init(|| self.assemble())
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// One button of a bank: the child, the ground under it, and the edges it owns.
struct ToggleButton<Msg> {
    child: Vec<Box<dyn Widget<Msg>>>,
    index: usize,
    count: usize,
    selected: bool,
    /// Whether the button **before** this one is on — the other half of the shared-edge
    /// rule, and the only thing a button needs to know about its neighbours.
    previous_selected: bool,
    enabled: bool,
    axis: ToggleAxis,
    /// Whether the buttons appear in the reverse of the order they were declared in. For a
    /// column this is [`VerticalDirection::Up`]; for a row it is the reading direction,
    /// which is not known until paint.
    reversed: bool,
    style: ToggleStyle,
    message: Msg,
}

impl<Msg> ToggleButton<Msg> {
    /// Whether this button appears first — leftmost in a row, topmost in a column.
    fn visually_first(&self, flipped: bool) -> bool {
        if flipped {
            self.index + 1 == self.count
        } else {
            self.index == 0
        }
    }

    /// Whether it appears last.
    fn visually_last(&self, flipped: bool) -> bool {
        if flipped {
            self.index == 0
        } else {
            self.index + 1 == self.count
        }
    }

    /// **This button's** corners, out of the bank's.
    ///
    /// The pair at the bank's near end goes to whichever button is drawn there and the
    /// pair at the far end to the one at the other; everything between is square. A bank
    /// of one keeps all four (`toggle_buttons.dart:503`).
    fn corners(&self, theme: &Theme, flipped: bool) -> BorderRadius {
        let r = self.style.radius(theme);
        if self.count <= 1 {
            return r;
        }
        let first = self.visually_first(flipped);
        let last = self.visually_last(flipped);
        match self.axis {
            ToggleAxis::Horizontal if first => BorderRadius {
                top_left: r.top_left,
                bottom_left: r.bottom_left,
                top_right: 0.0,
                bottom_right: 0.0,
            },
            ToggleAxis::Horizontal if last => BorderRadius {
                top_left: 0.0,
                bottom_left: 0.0,
                top_right: r.top_right,
                bottom_right: r.bottom_right,
            },
            ToggleAxis::Vertical if first => BorderRadius {
                top_left: r.top_left,
                top_right: r.top_right,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
            ToggleAxis::Vertical if last => BorderRadius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_left: r.bottom_left,
                bottom_right: r.bottom_right,
            },
            _ => BorderRadius::ZERO,
        }
    }

    /// The strip this button's **leading** border occupies: the edge facing the button
    /// before it, which is the start side in a row and the top in a column.
    fn leading_strip(&self, bounds: Rect, flipped: bool, width: f32) -> Rect {
        match (self.axis, flipped) {
            (ToggleAxis::Horizontal, false) => Rect::new(bounds.x, bounds.y, width, bounds.height),
            (ToggleAxis::Horizontal, true) => Rect::new(
                bounds.x + bounds.width - width,
                bounds.y,
                width,
                bounds.height,
            ),
            (ToggleAxis::Vertical, false) => Rect::new(bounds.x, bounds.y, bounds.width, width),
            (ToggleAxis::Vertical, true) => Rect::new(
                bounds.x,
                bounds.y + bounds.height - width,
                bounds.width,
                width,
            ),
        }
    }

    /// The box the outline is drawn over, **grown by one hairline** past the seam when
    /// there is a next button.
    ///
    /// This is the whole trick of a shared edge, and it is worth writing down. A rounded
    /// rectangle is one call and gets its corners right; four strips are four calls and do
    /// not. So each button draws a whole outline — and every button but the last puts its
    /// far edge exactly where the next button's leading border will land, so the next one,
    /// painted after, covers it. The bank ends up with one hairline per seam, every corner
    /// an arc, and no button drawing an edge that is not its own.
    fn outline_box(&self, bounds: Rect, flipped: bool, width: f32) -> Rect {
        if self.index + 1 == self.count {
            return bounds;
        }
        match (self.axis, flipped) {
            (ToggleAxis::Horizontal, false) => {
                Rect::new(bounds.x, bounds.y, bounds.width + width, bounds.height)
            }
            (ToggleAxis::Horizontal, true) => Rect::new(
                bounds.x - width,
                bounds.y,
                bounds.width + width,
                bounds.height,
            ),
            (ToggleAxis::Vertical, false) => {
                Rect::new(bounds.x, bounds.y, bounds.width, bounds.height + width)
            }
            (ToggleAxis::Vertical, true) => Rect::new(
                bounds.x,
                bounds.y - width,
                bounds.width,
                bounds.height + width,
            ),
        }
    }
}

impl<Msg: Clone> Widget<Msg> for ToggleButton<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    /// The child's box plus the edges this button owns.
    ///
    /// The borders are **padding**, not decoration drawn over the content: the reference's
    /// render object deflates its child by exactly the sides it draws
    /// (`toggle_buttons.dart:1254`), so a hairline never crosses a letter. A row keeps the
    /// leading side on the left and lets the frame's mirror move it in a right-to-left
    /// reading; a column has no mirror, so it swaps the two ends itself.
    fn style_themed(&self, theme: &Theme) -> Style {
        let width = self.style.border_width(theme);
        let leading = width;
        let trailing = if self.index + 1 == self.count {
            width
        } else {
            0.0
        };
        let padding = match (self.axis, self.reversed) {
            (ToggleAxis::Horizontal, _) => Insets::new(width, trailing, width, leading),
            (ToggleAxis::Vertical, false) => Insets::new(leading, width, trailing, width),
            (ToggleAxis::Vertical, true) => Insets::new(trailing, width, leading, width),
        };
        let cross = width * 2.0;
        let main = leading + trailing;
        let (extra_w, extra_h) = match self.axis {
            ToggleAxis::Horizontal => (main, cross),
            ToggleAxis::Vertical => (cross, main),
        };
        Style {
            padding,
            min_width: Dimension::Length(self.style.min_width(theme) + extra_w),
            min_height: Dimension::Length(self.style.min_height(theme) + extra_h),
            max_width: self
                .style
                .max_width(theme)
                .map_or(Dimension::Auto, |w| Dimension::Length(w + extra_w)),
            max_height: self
                .style
                .max_height(theme)
                .map_or(Dimension::Auto, |h| Dimension::Length(h + extra_h)),
            flex_direction: FlexDirection::Row,
            justify: Justify::Center,
            align: Align::Center,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A row mirrors with the frame; a column is told which way it runs.
        let flipped = match self.axis {
            ToggleAxis::Horizontal => theme.direction == TextDirection::Rtl,
            ToggleAxis::Vertical => self.reversed,
        };
        let corners = self.corners(theme, flipped);

        // The ground, and the state layer over it. An unselected button has no ground of
        // its own, so the layer is measured from the surface it stands on — and when
        // nothing is happening, nothing is painted at all, rather than a transparent
        // rectangle rubbing out what the bank was put over.
        let base = self.style.fill(theme, self.enabled, self.selected);
        let ground = if base.a > 0.0 {
            base
        } else {
            theme.scheme.surface
        };
        let fill = if self.enabled {
            let lit =
                theme.state_layer(ground, self.style.ink(theme, true, self.selected), &status);
            if lit == ground {
                base
            } else {
                lit
            }
        } else {
            base
        };
        if fill.a > 0.0 {
            scene.draw_rect(bounds, fill.fade(o), corners, 0.0, Color::TRANSPARENT);
        }

        let width = self.style.border_width(theme);
        if width <= 0.0 {
            return;
        }
        let side = self.style.border(theme, self.enabled, self.selected);
        scene.draw_rect(
            self.outline_box(bounds, flipped, width),
            Color::TRANSPARENT,
            corners,
            width,
            side.fade(o),
        );

        // **The shared edge.** It belongs to this button and the one before it, so either
        // being on colours it (`toggle_buttons.dart:596`) — which the outline above cannot
        // express, drawing one colour on four sides. Redrawn only when the two answers
        // differ, and when they differ the edge is always an inner seam: the outer end of
        // a bank is the first button's leading side, whose neighbour does not exist, so
        // there the two rules give the same colour and the arc the outline drew survives.
        let shared =
            self.style
                .border(theme, self.enabled, self.selected || self.previous_selected);
        if shared != side {
            scene.fill_rect(self.leading_strip(bounds, flipped, width), shared.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.message.clone())
    }

    fn ink(&self, theme: &Theme) -> Option<crate::InkStyle> {
        if !self.enabled {
            return None;
        }
        let splash = self.style.ink(theme, true, self.selected).fade(0.10);
        Some(crate::InkStyle::of(theme).color(splash))
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    /// A button, and whether it is on (`toggle_buttons.dart:836`).
    ///
    /// No label: the child is the caller's and could be anything, so the name a reader
    /// hears is whatever that child announces. What this adds is the part the child cannot
    /// know — that it is pressable, and that it is currently down.
    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Button).toggled(self.selected);
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size, Text};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle(usize),
    }

    fn bank(selected: Vec<bool>) -> ToggleButtons<Msg> {
        let mut buttons = ToggleButtons::new(selected, Msg::Toggle);
        for label in ["B", "I", "U"] {
            buttons = buttons.child(Text::new(label));
        }
        buttons
    }

    fn scene_of(buttons: ToggleButtons<Msg>) -> Vec<Primitive> {
        let root = crate::flex::Flex::column()
            .width(400.0)
            .height(120.0)
            .child(buttons);
        build_ui(
            &root,
            Size::new(400.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .to_vec()
    }

    /// Every rectangle drawn with a border, and the colour of that border.
    fn outlines(primitives: &[Primitive]) -> Vec<(Rect, Color)> {
        primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    border_width,
                    border_color,
                    ..
                } if *border_width > 0.0 => Some((*rect, *border_color)),
                _ => None,
            })
            .collect()
    }

    fn buttons_of(bank: &ToggleButtons<Msg>) -> &[Box<dyn Widget<Msg>>] {
        Widget::<Msg>::children(bank)
    }

    /// A child with no boolean beside it is not a button. The reference asserts the two
    /// lists match; a `view` is not a place to panic, so the shorter one wins.
    #[test]
    fn a_bank_is_as_long_as_its_shorter_list() {
        assert_eq!(buttons_of(&bank(vec![false, false, false])).len(), 3);
        assert_eq!(buttons_of(&bank(vec![false])).len(), 1);
        let extra = ToggleButtons::new(vec![false, false], Msg::Toggle).child(Text::new("B"));
        assert_eq!(buttons_of(&extra).len(), 1);
    }

    /// Three buttons, four lines — not six. Each button owns the edge facing the one
    /// before it, and only the last owns the far end; the outlines overlap by exactly one
    /// hairline so that a seam is drawn once.
    #[test]
    fn the_buttons_share_their_edges() {
        let primitives = scene_of(bank(vec![false, false, false]));
        let drawn = outlines(&primitives);
        assert_eq!(drawn.len(), 3, "one outline per button");
        let mut boxes: Vec<Rect> = drawn.iter().map(|(r, _)| *r).collect();
        boxes.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        for pair in boxes.windows(2) {
            let (left, right) = (pair[0], pair[1]);
            assert!(
                (left.x + left.width - right.x - TOGGLE_BUTTONS_BORDER_WIDTH).abs() < 0.01,
                "the outlines meet on the seam, overlapping by one hairline: {left:?} {right:?}"
            );
        }
        // And the last one stops at its own edge rather than reaching past it.
        let last = boxes.last().unwrap();
        let bank_right = boxes[0].x + boxes.iter().map(|b| b.width).sum::<f32>()
            - TOGGLE_BUTTONS_BORDER_WIDTH * 2.0;
        assert!((last.x + last.width - bank_right).abs() < 0.01);
    }

    /// The rule a row of separate buttons cannot express: a seam belongs to both of the
    /// buttons it separates, so **either** being on colours it.
    #[test]
    fn a_seam_takes_the_selected_colour_from_either_side() {
        let marked = Color::rgb(1.0, 0.0, 0.0);
        let plain = Color::rgb(0.0, 0.0, 1.0);
        // What is actually drawn on each button's leading edge, in order: the strip when
        // one was needed, and otherwise the outline that would have been covered by it.
        // Both are the same answer, which is the point — the extra strip exists only
        // where the shared rule and the button's own rule disagree.
        let seams = |selected: Vec<bool>| -> Vec<Color> {
            let primitives = scene_of(
                bank(selected)
                    .selected_border_color(marked)
                    .border_color(plain),
            );
            let mut drawn = outlines(&primitives);
            drawn.sort_by(|a, b| a.0.x.partial_cmp(&b.0.x).unwrap());
            let strips: Vec<(Rect, Color)> = primitives
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect {
                        rect,
                        color,
                        border_width,
                        ..
                    } if *border_width == 0.0 && rect.width == TOGGLE_BUTTONS_BORDER_WIDTH => {
                        Some((*rect, *color))
                    }
                    _ => None,
                })
                .collect();
            drawn
                .into_iter()
                .map(|(rect, outline)| {
                    strips
                        .iter()
                        .find(|(strip, _)| (strip.x - rect.x).abs() < 0.01)
                        .map_or(outline, |(_, colour)| *colour)
                })
                .collect()
        };
        // Nothing on: every edge is the plain one.
        assert_eq!(seams(vec![false, false, false]), vec![plain, plain, plain]);
        // The middle button on marks the seams on **both** sides of it — its own leading
        // edge, which its outline already carries, and the leading edge of the button
        // after it, which needs the strip.
        assert_eq!(seams(vec![false, true, false]), vec![plain, marked, marked]);
        // The first button on marks the bank's own end and the seam after it.
        assert_eq!(seams(vec![true, false, false]), vec![marked, marked, plain]);
        // And the last one marks the seam before it, from its own side.
        assert_eq!(seams(vec![false, false, true]), vec![plain, plain, marked]);
    }

    /// Only the two ends of a bank are rounded; the buttons between them are square, or
    /// the corners would appear four times across one control.
    #[test]
    fn only_the_ends_of_a_bank_are_rounded() {
        let primitives = scene_of(bank(vec![false, false, false]).border_radius(8.0));
        let mut drawn: Vec<(Rect, BorderRadius)> = primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    rect,
                    radius,
                    border_width,
                    ..
                } if *border_width > 0.0 => Some((*rect, *radius)),
                _ => None,
            })
            .collect();
        drawn.sort_by(|a, b| a.0.x.partial_cmp(&b.0.x).unwrap());
        let radii: Vec<BorderRadius> = drawn.into_iter().map(|(_, r)| r).collect();
        assert_eq!(radii[0].top_left, 8.0);
        assert_eq!(
            radii[0].top_right, 0.0,
            "the first button's inner end is square"
        );
        assert_eq!(radii[1], BorderRadius::ZERO, "and the middle one entirely");
        assert_eq!(radii[2].top_right, 8.0);
        assert_eq!(radii[2].top_left, 0.0);
    }

    /// A bank of one is not a first button with no last: it keeps all four corners.
    #[test]
    fn a_bank_of_one_keeps_every_corner() {
        let one = ToggleButtons::new(vec![false], Msg::Toggle)
            .child(Text::new("B"))
            .border_radius(8.0);
        let primitives = scene_of(one);
        let radii: Vec<BorderRadius> = primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect {
                    radius,
                    border_width,
                    ..
                } if *border_width > 0.0 => Some(*radius),
                _ => None,
            })
            .collect();
        assert_eq!(radii, vec![BorderRadius::uniform(8.0)]);
    }

    /// The colour reaches the child through the theme, which is what makes a plain `Text`
    /// in a button take the selected colour without being told.
    #[test]
    fn the_state_colours_the_children() {
        let theme = Theme::default();
        let text_colours = |selected: Vec<bool>, enabled: bool| -> Vec<Color> {
            scene_of(bank(selected).enabled(enabled))
                .into_iter()
                .filter_map(|p| match p {
                    Primitive::Text { color, .. } => Some(color),
                    _ => None,
                })
                .collect()
        };
        let live = text_colours(vec![true, false, false], true);
        assert_eq!(live[0], theme.scheme.primary, "the button that is on");
        assert_eq!(
            live[1],
            over_surface(&theme, CONTENT_OPACITY),
            "and the ones that are not"
        );
        for colour in text_colours(vec![true, false, false], false) {
            assert_eq!(
                colour,
                disabled_content(&theme),
                "a disabled bank flattens to one grey, its selected button included"
            );
        }
    }

    /// Disabled is the four-part contract: no message, no tab stop, no splash, and the
    /// disabled colours — plus the part specific to this widget, that a disabled bank
    /// never enters the selected state, so nothing is filled.
    #[test]
    fn a_disabled_bank_is_inert_but_still_readable() {
        let theme = Theme::default();
        let dead = bank(vec![true, false, false]).enabled(false);
        for button in buttons_of(&dead) {
            assert_eq!(button.on_click(), None);
            assert!(!button.focusable());
            assert!(button.ink(&theme).is_none());
            assert!(button.semantics().expect("still announced").disabled);
        }
        // Still says which one is on: a disabled control is read-only, not blank.
        assert_eq!(
            buttons_of(&dead)[0].semantics().unwrap().toggled,
            frus_core::Toggled::True
        );
        let accent = theme
            .scheme
            .surface
            .lerp(theme.scheme.primary, SELECTED_FILL_OPACITY);
        assert!(
            !scene_of(bank(vec![true, false, false]).enabled(false))
                .into_iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if color == accent)),
            "and never fills with the accent"
        );
        assert!(
            scene_of(bank(vec![true, false, false]))
                .into_iter()
                .any(|p| matches!(p, Primitive::Rect { color, .. } if color == accent)),
            "which a live one does"
        );
    }

    /// A button is at least the tap target however small its child, and the borders are
    /// added to that rather than eaten out of it.
    #[test]
    fn a_button_is_at_least_the_tap_target() {
        let theme = Theme::default();
        for (i, button) in buttons_of(&bank(vec![false, false, false]))
            .iter()
            .enumerate()
        {
            let style = button.style_themed(&theme);
            let last = i == 2;
            let expected = TOGGLE_BUTTON_MIN_SIZE
                + TOGGLE_BUTTONS_BORDER_WIDTH
                + if last {
                    TOGGLE_BUTTONS_BORDER_WIDTH
                } else {
                    0.0
                };
            assert_eq!(style.min_width, Dimension::Length(expected));
            assert_eq!(
                style.min_height,
                Dimension::Length(TOGGLE_BUTTON_MIN_SIZE + TOGGLE_BUTTONS_BORDER_WIDTH * 2.0)
            );
        }
    }

    /// The hairline is padding, so it never crosses the child — and only the last button
    /// pads the far end, which is the layout half of the shared edge.
    #[test]
    fn the_hairline_is_room_and_not_decoration() {
        let theme = Theme::default();
        let buttons = bank(vec![false, false, false]);
        let padding = |i: usize| buttons_of(&buttons)[i].style_themed(&theme).padding;
        assert_eq!(padding(0).left, TOGGLE_BUTTONS_BORDER_WIDTH);
        assert_eq!(padding(0).right, 0.0, "the seam is the next button's");
        assert_eq!(padding(2).right, TOGGLE_BUTTONS_BORDER_WIDTH);
        assert_eq!(padding(1).top, TOGGLE_BUTTONS_BORDER_WIDTH);
        // Told to draw no border, a button takes no room for one either.
        let bare = bank(vec![false, false, false]).render_border(false);
        assert_eq!(
            buttons_of(&bare)[0].style_themed(&theme).padding,
            Insets::ZERO
        );
    }

    /// A column runs the other way, and told to run upwards it swaps which end of each
    /// button carries its leading hairline — there being no mirror for a column the way
    /// there is for a row.
    #[test]
    fn a_vertical_bank_stacks() {
        let theme = Theme::default();
        let down = bank(vec![false, false, false]).direction(ToggleAxis::Vertical);
        assert_eq!(
            Widget::<Msg>::style(&down).flex_direction,
            FlexDirection::Column
        );
        assert_eq!(
            buttons_of(&down)[0].style_themed(&theme).padding.top,
            TOGGLE_BUTTONS_BORDER_WIDTH
        );
        let up = bank(vec![false, false, false])
            .direction(ToggleAxis::Vertical)
            .vertical_direction(VerticalDirection::Up);
        assert_eq!(
            Widget::<Msg>::style(&up).flex_direction,
            FlexDirection::ColumnReverse
        );
        assert_eq!(
            buttons_of(&up)[0].style_themed(&theme).padding.bottom,
            TOGGLE_BUTTONS_BORDER_WIDTH
        );
        assert_eq!(buttons_of(&up)[0].style_themed(&theme).padding.top, 0.0);
    }

    /// The theme is the last word before the framework's own, for every property the
    /// builders answer.
    #[test]
    fn the_theme_answers_what_the_caller_did_not() {
        let mut theme = Theme::default();
        let told = Color::rgb(0.0, 1.0, 0.0);
        theme.widgets.toggle_buttons.border_color = Some(told);
        theme.widgets.toggle_buttons.border_width = Some(3.0);
        let style = ToggleStyle::default();
        assert_eq!(style.border(&theme, true, false), told);
        assert_eq!(style.border_width(&theme), 3.0);
        // And the caller still outranks it.
        let mine = Color::rgb(1.0, 0.0, 1.0);
        let own = ToggleStyle {
            border_color: Some(mine),
            ..ToggleStyle::default()
        };
        assert_eq!(own.border(&theme, true, false), mine);
    }

    /// A message per index, and the bank itself emits nothing: it is the buttons that are
    /// pressed.
    #[test]
    fn each_button_emits_its_own_index() {
        let buttons = bank(vec![false, false, false]);
        assert_eq!(Widget::<Msg>::on_click(&buttons), None);
        for (i, button) in buttons_of(&buttons).iter().enumerate() {
            assert_eq!(button.on_click(), Some(Msg::Toggle(i)));
        }
    }
}
