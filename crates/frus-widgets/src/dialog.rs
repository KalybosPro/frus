//! [`Dialog`] and [`AlertDialog`]: the **modal** the framework did not have.
//!
//! A drawer, a bottom sheet and a menu were all here; the one that asks a question and
//! waits for the answer was not — and the name `AlertDialog` was taken by a widget that is
//! not a dialog at all. That one is [`Alert`](crate::Alert) now, which is what it is.
//!
//! Both are **controlled**, like every other overlay here: the application holds the open
//! flag and says what closes it. There is no `showDialog`, because there is no route stack
//! the framework owns to push onto — `open` is a field of the application's state, and
//! that is the whole difference.
//!
//! ```ignore
//! AlertDialog::new(app.confirm_open)
//!     .title("Delete this task?")
//!     .content("This cannot be undone.")
//!     .action(button("Cancel", Msg::CloseConfirm))
//!     .action(button("Delete", Msg::Delete))
//!     .on_dismiss(Msg::CloseConfirm)
//!     .body(screen)
//! ```
//!
//! [`Dialog`] is the surface on its own — rounded, elevated, held off the window's edges,
//! at least 280 wide — for content that is not a title-and-actions box.

use frus_core::{BorderRadius, Color, Insets, Rect, Scene, TextAlign, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

/// How far the dialog is held off the window's edges — the reference's
/// `_defaultInsetPadding` (`dialog.dart:32`).
pub const DIALOG_INSET_PADDING: Insets = Insets {
    top: 24.0,
    right: 40.0,
    bottom: 24.0,
    left: 40.0,
};
/// The narrowest a dialog gets, whatever its content (`dialog.dart:275`).
pub const DIALOG_MIN_WIDTH: f32 = 280.0;
/// The Material 3 corner (`dialog.dart:1967`).
pub const DIALOG_RADIUS: f32 = 28.0;
/// The Material 3 elevation (`dialog.dart:1966`).
pub const DIALOG_ELEVATION: f32 = 6.0;
/// The margin around the title / content / actions column.
const EDGE: f32 = 24.0;
/// The gap above content that follows a title or an icon (`dialog.dart:859`).
const CONTENT_GAP: f32 = 16.0;
/// The gap under a title with nothing below it (`dialog.dart:829`).
const TITLE_ONLY_GAP: f32 = 20.0;
/// The space between two action buttons — half the reference's `buttonPadding.horizontal`,
/// which is 16 (`dialog.dart:882`).
const ACTION_GAP: f32 = 8.0;

/// The title's type: what the caller said, else the theme, else the reference's
/// `headlineSmall` (`dialog.dart:1988`).
fn title_style(over: Option<TextStyle>, theme: Option<&Theme>) -> TextStyle {
    over.or(theme.and_then(|t| t.widgets.dialog.title_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).headline_small)
}

/// The content's type: `bodyMedium` (`dialog.dart:1991`).
fn content_style(over: Option<TextStyle>, theme: Option<&Theme>) -> TextStyle {
    over.or(theme.and_then(|t| t.widgets.dialog.content_text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_medium)
}

/// Hands a subtree a default text style, which is how the reference styles a dialog's
/// title and content — a `DefaultTextStyle` around the widget the caller passed, never a
/// style forced onto it (`dialog.dart:842`).
fn texted<Msg: Clone + 'static>(
    style: TextStyle,
    align: Option<TextAlign>,
    child: Box<dyn Widget<Msg>>,
) -> crate::Themed<Msg> {
    crate::Themed::tweak(
        move |t: &mut Theme| {
            t.widgets.text.style = style;
            if let Some(align) = align {
                t.widgets.text.align = Some(align);
            }
        },
        child,
    )
}

/// The panel itself: a rounded, elevated surface, at least [`DIALOG_MIN_WIDTH`] wide and
/// held off the window's edges by its margin.
struct DialogSurface<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    background: Option<Color>,
    elevation: Option<f32>,
    shadow_color: Option<Color>,
    surface_tint: Option<Color>,
    shape: Option<BorderRadius>,
    inset_padding: Option<Insets>,
    min_width: Option<f32>,
    max_width: Option<f32>,
}

impl<Msg> DialogSurface<Msg> {
    fn elevation(&self, theme: Option<&Theme>) -> f32 {
        self.elevation
            .or(theme.and_then(|t| t.widgets.dialog.elevation))
            .unwrap_or(DIALOG_ELEVATION)
    }

    fn radius(&self, theme: Option<&Theme>) -> BorderRadius {
        self.shape
            .or(theme.and_then(|t| t.widgets.dialog.shape))
            .unwrap_or_else(|| DIALOG_RADIUS.into())
    }

    /// The surface's colour, tinted for its elevation **only where a tint is named**.
    ///
    /// The reference's Material 3 dialog makes both the shadow and the tint transparent
    /// (`dialog.dart:1982`): the container tone already carries the height. A caller or a
    /// theme that names a tint is asking for the older look, and gets it.
    fn background(&self, theme: &Theme) -> Color {
        let base = self
            .background
            .or(theme.widgets.dialog.color)
            .unwrap_or(theme.scheme.surface_container_high);
        match self.surface_tint.or(theme.widgets.dialog.surface_tint) {
            Some(tint) => base.surface_tint(tint, self.elevation(Some(theme))),
            None => base,
        }
    }

    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let margin = self
            .inset_padding
            .or(theme.and_then(|t| t.widgets.dialog.inset_padding))
            .unwrap_or(DIALOG_INSET_PADDING);
        Style {
            flex_direction: FlexDirection::Column,
            height: Dimension::Auto,
            min_width: Dimension::Length(self.min_width.unwrap_or(DIALOG_MIN_WIDTH)),
            max_width: self.max_width.map_or(Dimension::Auto, Dimension::Length),
            margin,
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for DialogSurface<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let radius = self.radius(Some(theme));
        let depth = self.elevation(Some(theme));
        // The shadow, drawn the way `Card` draws one: the blur grows with the depth and
        // the drop is half of it. The reference's Material 3 shadow colour is transparent
        // — the tone carries the height — so a shadow appears only where one is named.
        let shadow = self
            .shadow_color
            .or(theme.widgets.dialog.shadow_color)
            .unwrap_or(Color::TRANSPARENT);
        if depth > 0.0 && shadow.a > 0.0 {
            let blur = depth * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                shadow.fade(o),
                radius.inflate(blur),
                blur,
            );
        }
        scene.draw_rect(
            bounds,
            self.background(theme).fade(o),
            radius,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A **modal surface** over the screen: rounded, elevated, held off the window's edges.
///
/// Controlled, like every overlay here: `open` says whether it is showing, and
/// [`Dialog::on_dismiss`] is what a click on the scrim sends. The screen behind stays
/// built and visible.
pub struct Dialog<Msg> {
    open: bool,
    on_dismiss: Option<Msg>,
    content: Option<Box<dyn Widget<Msg>>>,
    panel: Option<Box<dyn Widget<Msg>>>,
    children: Vec<Box<dyn Widget<Msg>>>,
    background: Option<Color>,
    elevation: Option<f32>,
    shadow_color: Option<Color>,
    surface_tint: Option<Color>,
    shape: Option<BorderRadius>,
    inset_padding: Option<Insets>,
    min_width: Option<f32>,
    max_width: Option<f32>,
}

impl<Msg: Clone + 'static> Dialog<Msg> {
    /// Creates a dialog; `open` says whether it is showing.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            on_dismiss: None,
            content: None,
            panel: None,
            children: Vec::new(),
            background: None,
            elevation: None,
            shadow_color: None,
            surface_tint: None,
            shape: None,
            inset_padding: None,
            min_width: None,
            max_width: None,
        }
    }

    /// What a click on the scrim sends. Without it the scrim is inert, which is the
    /// reference's `barrierDismissible: false` — for a dialog that must be answered.
    #[must_use]
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// What the dialog holds.
    #[must_use]
    pub fn child(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// The surface's colour. The scheme's `surfaceContainerHigh` by default, as in the
    /// reference (`dialog.dart:1979`).
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// How far off the page it sits. `6` by default (`dialog.dart:1966`).
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// The shadow's colour. **Transparent by default**, as the reference's Material 3
    /// dialog is: the container tone carries the height instead of a drop shadow.
    #[must_use]
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = Some(color);
        self
    }

    /// The colour the surface is tinted towards for its elevation. Unset — the default —
    /// nothing is tinted, which is the reference's Material 3 answer.
    #[must_use]
    pub fn surface_tint(mut self, color: Color) -> Self {
        self.surface_tint = Some(color);
        self
    }

    /// The corner. `28` by default (`dialog.dart:1967`).
    #[must_use]
    pub fn shape(mut self, shape: impl Into<BorderRadius>) -> Self {
        self.shape = Some(shape.into());
        self
    }

    /// How far the dialog is held off the window's edges. 40 across and 24 down by
    /// default (`dialog.dart:32`).
    #[must_use]
    pub fn inset_padding(mut self, padding: Insets) -> Self {
        self.inset_padding = Some(padding);
        self
    }

    /// The narrowest it gets, whatever its content. `280` by default
    /// (`dialog.dart:275`).
    #[must_use]
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// The widest it gets. Unbounded by default, as the reference's `constraints` is.
    #[must_use]
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Sets the screen behind it and finalises the dialog.
    #[must_use]
    pub fn body(mut self, body: impl Widget<Msg> + 'static) -> Self {
        self.panel = self.content.take().map(|content| {
            Box::new(DialogSurface {
                children: vec![content],
                background: self.background,
                elevation: self.elevation,
                shadow_color: self.shadow_color,
                surface_tint: self.surface_tint,
                shape: self.shape,
                inset_padding: self.inset_padding,
                min_width: self.min_width,
                max_width: self.max_width,
            }) as Box<dyn Widget<Msg>>
        });
        self.children = vec![Box::new(body)];
        self
    }
}

impl<Msg: Clone> Widget<Msg> for Dialog<Msg> {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Percent(1.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// The width it was **offered**, not the width its parent came out at.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes::WIDTH
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn overlay(&self) -> Option<(&dyn Widget<Msg>, Placement)> {
        self.panel.as_ref().map(|p| (p.as_ref(), Placement::Center))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn anim_target(&self) -> Option<f32> {
        Some(if self.open { 1.0 } else { 0.0 })
    }
}

/// Where the action buttons sit along the bottom row.
pub type ActionsAlignment = Justify;

/// A dialog that **asks something**: an optional icon, an optional title, some content,
/// and the buttons that answer it.
///
/// The paddings are the reference's, and conditional the way the reference's are: a title
/// under an icon loses its top padding, a title with content below it loses its bottom
/// padding, and a title with nothing below it keeps twenty of it (`dialog.dart:824`).
/// An icon also **centres the title**, which is the one place the presence of one slot
/// changes how another is aligned (`dialog.dart:844`).
pub struct AlertDialog<Msg> {
    dialog: Dialog<Msg>,
    icon: Option<Box<dyn Widget<Msg>>>,
    icon_color: Option<Color>,
    title: Option<Box<dyn Widget<Msg>>>,
    title_text_style: Option<TextStyle>,
    content: Option<Box<dyn Widget<Msg>>>,
    content_text_style: Option<TextStyle>,
    actions: Vec<Box<dyn Widget<Msg>>>,
    actions_alignment: Option<ActionsAlignment>,
}

impl<Msg: Clone + 'static> AlertDialog<Msg> {
    /// Creates one; `open` says whether it is showing.
    pub fn new(open: bool) -> Self {
        Self {
            dialog: Dialog::new(open),
            icon: None,
            icon_color: None,
            title: None,
            title_text_style: None,
            content: None,
            content_text_style: None,
            actions: Vec::new(),
            actions_alignment: None,
        }
    }

    /// What a click on the scrim sends. Without it the scrim is inert — the reference's
    /// `barrierDismissible: false`, for a question a button has to answer.
    #[must_use]
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.dialog = self.dialog.on_dismiss(message);
        self
    }

    /// A glyph above the title, centred. Having one **centres the title too**.
    #[must_use]
    pub fn icon(mut self, icon: impl Widget<Msg> + 'static) -> Self {
        self.icon = Some(Box::new(icon));
        self
    }

    /// The icon's colour. The scheme's `secondary` by default (`dialog.dart:1976`).
    #[must_use]
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// The heading, as text.
    #[must_use]
    pub fn title(self, title: impl Into<String>) -> Self {
        let text = crate::Text::new(title);
        self.title_widget(text)
    }

    /// The heading, as a widget.
    #[must_use]
    pub fn title_widget(mut self, title: impl Widget<Msg> + 'static) -> Self {
        self.title = Some(Box::new(title));
        self
    }

    /// The heading's type. `headlineSmall` by default.
    #[must_use]
    pub fn title_text_style(mut self, style: TextStyle) -> Self {
        self.title_text_style = Some(style);
        self
    }

    /// What the dialog says, as text.
    #[must_use]
    pub fn content(self, content: impl Into<String>) -> Self {
        let text = crate::Text::new(content);
        self.content_widget(text)
    }

    /// What the dialog says, as a widget.
    #[must_use]
    pub fn content_widget(mut self, content: impl Widget<Msg> + 'static) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// The content's type. `bodyMedium` by default.
    #[must_use]
    pub fn content_text_style(mut self, style: TextStyle) -> Self {
        self.content_text_style = Some(style);
        self
    }

    /// Adds a button to the row along the bottom.
    #[must_use]
    pub fn action(mut self, action: impl Widget<Msg> + 'static) -> Self {
        self.actions.push(Box::new(action));
        self
    }

    /// Where the buttons sit along that row. The trailing end by default, as in the
    /// reference (`MainAxisAlignment.end`).
    #[must_use]
    pub fn actions_alignment(mut self, alignment: ActionsAlignment) -> Self {
        self.actions_alignment = Some(alignment);
        self
    }

    /// The surface's colour — see [`Dialog::background`].
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.dialog = self.dialog.background(color);
        self
    }

    /// How far off the page it sits — see [`Dialog::elevation`].
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.dialog = self.dialog.elevation(elevation);
        self
    }

    /// The shadow's colour — see [`Dialog::shadow_color`].
    #[must_use]
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.dialog = self.dialog.shadow_color(color);
        self
    }

    /// The tint for its elevation — see [`Dialog::surface_tint`].
    #[must_use]
    pub fn surface_tint(mut self, color: Color) -> Self {
        self.dialog = self.dialog.surface_tint(color);
        self
    }

    /// The corner — see [`Dialog::shape`].
    #[must_use]
    pub fn shape(mut self, shape: impl Into<BorderRadius>) -> Self {
        self.dialog = self.dialog.shape(shape);
        self
    }

    /// How far off the window's edges — see [`Dialog::inset_padding`].
    #[must_use]
    pub fn inset_padding(mut self, padding: Insets) -> Self {
        self.dialog = self.dialog.inset_padding(padding);
        self
    }

    /// The narrowest it gets — see [`Dialog::min_width`].
    #[must_use]
    pub fn min_width(mut self, width: f32) -> Self {
        self.dialog = self.dialog.min_width(width);
        self
    }

    /// The widest it gets — see [`Dialog::max_width`].
    #[must_use]
    pub fn max_width(mut self, width: f32) -> Self {
        self.dialog = self.dialog.max_width(width);
        self
    }

    /// Sets the screen behind it and finalises the dialog.
    ///
    /// The column is composed by a [`ThemeBuilder`](crate::ThemeBuilder): the title's type
    /// comes from the theme and its alignment from whether there is an icon, so what the
    /// column *contains* cannot be decided before the theme is known.
    #[must_use]
    pub fn body(self, body: impl Widget<Msg> + 'static) -> Dialog<Msg> {
        let AlertDialog {
            dialog,
            icon,
            icon_color,
            title,
            title_text_style,
            content,
            content_text_style,
            actions,
            actions_alignment,
        } = self;
        let has_icon = icon.is_some();
        let has_title = title.is_some();
        let has_content = content.is_some();
        let column = crate::ThemeBuilder::boxed(move |theme| {
            // **The column stretches** (`dialog.dart:928`) and the centring is the
            // *text's* own: a slot laid out at its natural width has nothing for a text
            // alignment to move it within, so an icon would centre nothing.
            let mut column = crate::Flex::column().align(Align::Stretch);
            if let Some(icon) = icon {
                // 24 all round, less at the bottom when something follows
                // (`dialog.dart:795`).
                let below = if has_title {
                    CONTENT_GAP
                } else if has_content {
                    0.0
                } else {
                    EDGE
                };
                let color = icon_color
                    .or(theme.widgets.dialog.icon_color)
                    .unwrap_or(theme.scheme.secondary);
                column = column.child(
                    crate::Flex::column()
                        .align(Align::Center)
                        .padding_each(EDGE, EDGE, below, EDGE)
                        .child(crate::Themed::tweak(
                            move |t: &mut Theme| t.widgets.icon.color = Some(color),
                            icon,
                        )),
                );
            }
            if let Some(title) = title {
                let above = if has_icon { 0.0 } else { EDGE };
                let below = if has_content { 0.0 } else { TITLE_ONLY_GAP };
                let align = if has_icon {
                    TextAlign::Center
                } else {
                    TextAlign::Start
                };
                // The row is what the alignment moves the title within. A text declares
                // the width of its own words, so a text alignment alone has nothing to
                // move it inside — that is the difference between this and the
                // reference's `textAlign`, which works because its column stretches the
                // text to the box. Both are set: the row places a single line, the
                // alignment places the lines of a title that wrapped.
                column = column.child(
                    crate::Flex::column()
                        .padding_each(above, EDGE, below, EDGE)
                        .child(
                            crate::Flex::row()
                                .justify(if has_icon {
                                    Justify::Center
                                } else {
                                    Justify::Start
                                })
                                .child(texted(
                                    title_style(title_text_style, Some(theme)),
                                    Some(align),
                                    title,
                                )),
                        ),
                );
            }
            if let Some(content) = content {
                let above = if has_icon || has_title {
                    CONTENT_GAP
                } else {
                    EDGE
                };
                column = column.child(
                    crate::Flex::column()
                        .padding_each(above, EDGE, EDGE, EDGE)
                        .child(texted(
                            content_style(content_text_style, Some(theme)),
                            None,
                            content,
                        )),
                );
            }
            if !actions.is_empty() {
                // 24 at the sides and the bottom, nothing on top: whatever is above has
                // already spaced itself (`dialog.dart:1994`).
                let mut row = crate::Flex::row()
                    .justify(actions_alignment.unwrap_or(Justify::End))
                    .gap(ACTION_GAP);
                for action in actions {
                    row = row.child_boxed(action);
                }
                column = column.child(
                    crate::Flex::column()
                        .padding_each(0.0, EDGE, EDGE, EDGE)
                        .child(row),
                );
            }
            Box::new(column) as Box<dyn Widget<Msg>>
        });
        dialog.child(column).body(body)
    }
}

/// The padding around a [`SimpleDialog`]'s title (`dialog.dart:1168`).
const SIMPLE_TITLE_PADDING: Insets = Insets {
    top: 24.0,
    right: 24.0,
    bottom: 0.0,
    left: 24.0,
};
/// The padding around its list of options (`dialog.dart:1171`) — **no side padding**,
/// because each option is a full-width row that pads itself and lights up across the
/// whole dialog when it is pressed.
const SIMPLE_CONTENT_PADDING: Insets = Insets {
    top: 12.0,
    right: 0.0,
    bottom: 16.0,
    left: 0.0,
};
/// A [`SimpleDialogOption`]'s own padding (`dialog.dart:1082`).
const OPTION_PADDING: Insets = Insets {
    top: 8.0,
    right: 24.0,
    bottom: 8.0,
    left: 24.0,
};

/// One row of a [`SimpleDialog`]: a full-width option that answers the question by being
/// chosen.
///
/// It is a widget of its own rather than a closure argument for the reason the reference
/// makes it one — the row is the tappable thing, and its ink has to run the full width of
/// the dialog, which only a widget that *is* the row can do.
pub struct SimpleDialogOption<Msg> {
    child: Option<Box<dyn Widget<Msg>>>,
    on_press: Option<Msg>,
    padding: Option<Insets>,
}

impl<Msg: Clone + 'static> SimpleDialogOption<Msg> {
    /// An option showing `label`, sending `message` when it is chosen.
    pub fn new(label: impl Into<String>, message: Msg) -> Self {
        Self {
            child: Some(Box::new(crate::Text::new(label))),
            on_press: Some(message),
            padding: None,
        }
    }

    /// The same, with a widget in place of the text.
    pub fn with_child(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            child: Some(Box::new(child)),
            on_press: None,
            padding: None,
        }
    }

    /// What choosing it sends. Without one it cannot be chosen, which is the reference's
    /// null `onPressed`.
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The space around the row's content. 8 down and 24 across by default.
    #[must_use]
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Assembles the option.
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let padding = self.padding.unwrap_or(OPTION_PADDING);
        let mut row = crate::Flex::row().align(Align::Center).padding_each(
            padding.top,
            padding.right,
            padding.bottom,
            padding.left,
        );
        if let Some(child) = self.child {
            row = row.child_boxed(child);
        }
        let mut ink = crate::InkWell::new();
        if let Some(message) = self.on_press {
            ink = ink.on_click(message);
        }
        Box::new(ink.child(row))
    }
}

/// A dialog that **offers a choice**: a title, and a list of options to pick from.
///
/// The one the reference calls simple, and the difference from [`AlertDialog`] is what it
/// is for: an alert dialog asks a question and puts the answers in a row of buttons at the
/// bottom, this one lists them and each row *is* an answer.
pub struct SimpleDialog<Msg> {
    dialog: Dialog<Msg>,
    title: Option<Box<dyn Widget<Msg>>>,
    title_text_style: Option<TextStyle>,
    title_padding: Option<Insets>,
    content_padding: Option<Insets>,
    content_text_style: Option<TextStyle>,
    options: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> SimpleDialog<Msg> {
    /// Creates one; `open` says whether it is showing.
    pub fn new(open: bool) -> Self {
        Self {
            dialog: Dialog::new(open),
            title: None,
            title_text_style: None,
            title_padding: None,
            content_padding: None,
            content_text_style: None,
            options: Vec::new(),
        }
    }

    /// What a click on the scrim sends — see [`Dialog::on_dismiss`].
    #[must_use]
    pub fn on_dismiss(mut self, message: Msg) -> Self {
        self.dialog = self.dialog.on_dismiss(message);
        self
    }

    /// The heading, as text.
    #[must_use]
    pub fn title(self, title: impl Into<String>) -> Self {
        let text = crate::Text::new(title);
        self.title_widget(text)
    }

    /// The heading, as a widget.
    #[must_use]
    pub fn title_widget(mut self, title: impl Widget<Msg> + 'static) -> Self {
        self.title = Some(Box::new(title));
        self
    }

    /// The heading's type. `headlineSmall` by default, as [`AlertDialog`]'s is.
    #[must_use]
    pub fn title_text_style(mut self, style: TextStyle) -> Self {
        self.title_text_style = Some(style);
        self
    }

    /// The space around the heading.
    #[must_use]
    pub fn title_padding(mut self, padding: Insets) -> Self {
        self.title_padding = Some(padding);
        self
    }

    /// The space around the list of options. **No side padding** by default: an option is
    /// a full-width row that pads itself.
    #[must_use]
    pub fn content_padding(mut self, padding: Insets) -> Self {
        self.content_padding = Some(padding);
        self
    }

    /// What the options are set in. `bodyMedium` by default.
    #[must_use]
    pub fn content_text_style(mut self, style: TextStyle) -> Self {
        self.content_text_style = Some(style);
        self
    }

    /// Adds an option — usually a [`SimpleDialogOption`].
    #[must_use]
    pub fn option(mut self, option: impl Widget<Msg> + 'static) -> Self {
        self.options.push(Box::new(option));
        self
    }

    /// The surface's colour — see [`Dialog::background`].
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.dialog = self.dialog.background(color);
        self
    }

    /// How far off the page it sits — see [`Dialog::elevation`].
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.dialog = self.dialog.elevation(elevation);
        self
    }

    /// The corner — see [`Dialog::shape`].
    #[must_use]
    pub fn shape(mut self, shape: impl Into<BorderRadius>) -> Self {
        self.dialog = self.dialog.shape(shape);
        self
    }

    /// How far off the window's edges — see [`Dialog::inset_padding`].
    #[must_use]
    pub fn inset_padding(mut self, padding: Insets) -> Self {
        self.dialog = self.dialog.inset_padding(padding);
        self
    }

    /// Sets the screen behind it and finalises the dialog.
    #[must_use]
    pub fn body(self, body: impl Widget<Msg> + 'static) -> Dialog<Msg> {
        let SimpleDialog {
            dialog,
            title,
            title_text_style,
            title_padding,
            content_padding,
            content_text_style,
            options,
        } = self;
        let has_options = !options.is_empty();
        let column = crate::ThemeBuilder::boxed(move |theme| {
            let mut column = crate::Flex::column().align(Align::Stretch);
            if let Some(title) = title {
                let pad = title_padding.unwrap_or(SIMPLE_TITLE_PADDING);
                // The reference drops the title's **bottom** padding when options follow:
                // the list has a top padding of its own, and two of them stacked would be
                // a gap nobody asked for (`dialog.dart:1316`).
                let bottom = if has_options { 0.0 } else { pad.bottom };
                column = column.child(
                    crate::Flex::column()
                        .padding_each(pad.top, pad.right, bottom, pad.left)
                        .child(texted(
                            title_style(title_text_style, Some(theme)),
                            None,
                            title,
                        )),
                );
            }
            if has_options {
                let pad = content_padding.unwrap_or(SIMPLE_CONTENT_PADDING);
                let mut list = crate::Flex::column().align(Align::Stretch);
                for option in options {
                    list = list.child_boxed(option);
                }
                column = column.child(
                    crate::Flex::column()
                        .padding_each(pad.top, pad.right, pad.bottom, pad.left)
                        .child(texted(
                            content_style(content_text_style, Some(theme)),
                            None,
                            Box::new(list),
                        )),
                );
            }
            Box::new(column) as Box<dyn Widget<Msg>>
        });
        dialog.child(column).body(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size, Text};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
        Delete,
    }

    fn texts(dialog: &Dialog<Msg>) -> Vec<String> {
        build_ui(
            dialog,
            Size::new(400.0, 800.0),
            &Runtime::default(),
            &Theme::default(),
        )
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            frus_core::Primitive::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
    }

    /// Closed, it is an overlay with nothing showing: the animation target is what the
    /// runtime reads, and the screen behind is untouched either way.
    #[test]
    fn a_closed_dialog_is_an_overlay_at_rest() {
        let dialog = Dialog::<Msg>::new(false)
            .child(Text::new("Question"))
            .body(Container::new());
        assert_eq!(Widget::<Msg>::anim_target(&dialog), Some(0.0));
    }

    #[test]
    fn an_open_dialog_is_a_centred_overlay_that_the_scrim_closes() {
        let dialog = Dialog::<Msg>::new(true)
            .on_dismiss(Msg::Close)
            .child(Text::new("Question"))
            .body(Container::new());
        let (_, placement) = Widget::<Msg>::overlay(&dialog).expect("an open dialog is shown");
        assert_eq!(placement, Placement::Center);
        assert_eq!(Widget::<Msg>::overlay_dismiss(&dialog), Some(Msg::Close));
        assert_eq!(Widget::<Msg>::anim_target(&dialog), Some(1.0));
    }

    /// **A dialog that must be answered has an inert scrim.** Saying nothing about
    /// dismissal is the reference's `barrierDismissible: false`, and it has to be the
    /// answer rather than a default, because a dialog that closes itself on a stray click
    /// has not been answered.
    #[test]
    fn a_dialog_that_must_be_answered_has_an_inert_scrim() {
        let dialog = Dialog::<Msg>::new(true)
            .child(Text::new("Question"))
            .body(Container::new());
        assert_eq!(Widget::<Msg>::overlay_dismiss(&dialog), None);
    }

    /// The surface's own geometry is the reference's: never narrower than 280, and held
    /// off the window's edges by 40 across and 24 down.
    #[test]
    fn the_surface_is_the_reference_s_box() {
        let dialog = Dialog::<Msg>::new(true)
            .child(Text::new("."))
            .body(Container::new());
        let (panel, _) = Widget::<Msg>::overlay(&dialog).expect("shown");
        assert_eq!(panel.style().min_width, Dimension::Length(DIALOG_MIN_WIDTH));
        assert_eq!(panel.style().margin, DIALOG_INSET_PADDING);
    }

    /// Everything the alert dialog was given is drawn.
    #[test]
    fn an_alert_dialog_shows_its_title_its_content_and_its_actions() {
        let dialog = AlertDialog::<Msg>::new(true)
            .title("Delete this task?")
            .content("This cannot be undone.")
            .action(crate::dsl::button("Cancel", Msg::Close))
            .action(crate::dsl::button("Delete", Msg::Delete))
            .body(Container::new());
        let found = texts(&dialog);
        for wanted in [
            "Delete this task?",
            "This cannot be undone.",
            "Cancel",
            "Delete",
        ] {
            assert!(
                found.iter().any(|t| t == wanted),
                "{wanted:?} is missing from {found:?}"
            );
        }
    }

    /// **The title is a heading, and the content is not.** The reference gives them
    /// `headlineSmall` and `bodyMedium`, handed down as a default text style rather than
    /// forced onto the widget the caller passed — so a caller's own `Text::size` still
    /// wins, and a plain string gets the type the specification asks for.
    #[test]
    fn the_title_and_the_content_take_the_reference_s_two_roles() {
        let dialog = AlertDialog::<Msg>::new(true)
            .title("Heading")
            .content("Body")
            .body(Container::new());
        let ui = build_ui(
            &dialog,
            Size::new(400.0, 800.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let size_of = |wanted: &str| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { text, size, .. } if text == wanted => Some(*size),
                    _ => None,
                })
                .expect("drawn")
        };
        let theme = Theme::default();
        assert_eq!(
            size_of("Heading"),
            theme.text.headline_small.resolved().size
        );
        assert_eq!(size_of("Body"), theme.text.body_medium.resolved().size);
        assert!(size_of("Heading") > size_of("Body"));
    }

    /// A simple dialog lists its options, and each row **is** an answer: choosing one
    /// sends its message, which is the difference from an alert dialog's row of buttons.
    #[test]
    fn a_simple_dialog_lists_options_that_answer_it() {
        let dialog = SimpleDialog::<Msg>::new(true)
            .title("Set backup account")
            .option(SimpleDialogOption::new("someone@example.com", Msg::Close).build())
            .option(SimpleDialogOption::new("Add account", Msg::Delete).build())
            .body(Container::new());
        let found = texts(&dialog);
        for wanted in ["Set backup account", "someone@example.com", "Add account"] {
            assert!(
                found.iter().any(|t| t == wanted),
                "{wanted:?} is missing from {found:?}"
            );
        }
        // And the rows are clickable. The panel is the dialog's **overlay**, not one of
        // its children — a modal is drawn over the screen, not in it — so the walk has to
        // start there. `texts` above has already built it.
        fn walk(widget: &dyn Widget<Msg>, out: &mut Vec<Msg>) {
            if let Some(message) = widget.on_click() {
                out.push(message);
            }
            for child in widget.children() {
                walk(child.as_ref(), out);
            }
        }
        let (panel, _) = Widget::<Msg>::overlay(&dialog).expect("an open dialog is shown");
        let mut clicks = Vec::new();
        walk(panel, &mut clicks);
        assert!(
            clicks.contains(&Msg::Close) && clicks.contains(&Msg::Delete),
            "an option that cannot be chosen is not an option: {clicks:?}"
        );
    }

    /// **An option's row runs the full width of the dialog**, which is what the reference's
    /// zero side padding on the list is for: the row is the tappable thing, and its ink has
    /// to reach both edges. Padding the *list* instead would inset the ripple.
    #[test]
    fn an_option_s_row_reaches_both_edges_of_the_dialog() {
        let dialog = SimpleDialog::<Msg>::new(true)
            .title("Pick one")
            .option(SimpleDialogOption::new("An option", Msg::Close).build())
            .body(Container::new());
        let ui = build_ui(
            &dialog,
            Size::new(400.0, 800.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // The dialog's own surface, and the option's text: the text starts one option
        // padding in from the surface's edge, not one option padding **plus** a list one.
        let surface = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, radius, .. } if radius.top_left > 0.0 => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the dialog's surface is drawn");
        let option = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { text, bounds, .. } if text == "An option" => {
                    Some(*bounds)
                }
                _ => None,
            })
            .expect("the option is drawn");
        assert!(
            (option.x - (surface.x + OPTION_PADDING.left)).abs() < 0.5,
            "the option is inset twice: surface {surface:?}, option {option:?}"
        );
    }

    /// **An icon centres the title.** It is the one place where the presence of one slot
    /// changes how another is aligned, and the reference does it in as many words
    /// (`dialog.dart:844`).
    #[test]
    fn an_icon_centres_the_title_under_it() {
        let title_of = |icon: bool| {
            let mut dialog = AlertDialog::<Msg>::new(true).title("Heading");
            if icon {
                dialog = dialog.icon(Text::new("!"));
            }
            let dialog = dialog.body(Container::new());
            build_ui(
                &dialog,
                Size::new(400.0, 800.0),
                &Runtime::default(),
                &Theme::default(),
            )
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text {
                    text,
                    align,
                    bounds,
                    ..
                } if text == "Heading" => Some((*align, *bounds)),
                _ => None,
            })
            .expect("the title is drawn")
        };
        let (with, with_box) = title_of(true);
        let (without, without_box) = title_of(false);
        assert_eq!(with, TextAlign::Center, "an icon must centre the title");
        assert_eq!(
            without,
            TextAlign::Start,
            "without one it starts at the leading edge"
        );
        // And it is where it says it is. The dialog is centred in a 400-wide window, so
        // its own centre is 200; a centred title's box is centred on that, and one that
        // starts at the leading edge sits to the left of it.
        let centre = |b: frus_core::Rect| b.x + b.width / 2.0;
        assert!(
            (centre(with_box) - 200.0).abs() < 1.0,
            "the title is not on the dialog's centre line: {with_box:?}"
        );
        assert!(
            centre(without_box) < centre(with_box) - 10.0,
            "the title did not move when an icon was put above it: {without_box:?}"
        );
    }
}
