//! [`DropdownButton`]: a **controlled** dropdown list whose options float above
//! everything else (through the overlay mechanism), below the header.
//!
//! Adjustable width ([`width`](DropdownButton::width)), the **selected** option highlighted
//! and ticked ([`selected`](DropdownButton::selected)), and **keyboard** navigation: the
//! header and the options take focus (Enter opens or picks, the arrows move through).
//!
//! A choice is a word in the ordinary case, and [`DropdownOption`] is what it is when it is
//! not: a swatch, a flag, two lines with a subtitle — or a choice that is **there and
//! cannot be picked**, which a list that simply left it out could not say.
//!
//! [`crate::DropdownMenu`] is the same closed set behind a **field** rather than a button:
//! it filters as it is typed into, and it draws its choices with the rows in this module so
//! that the two lists cannot drift apart.

use std::rc::Rc;

use frus_core::{Insets, Path, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::flex::Flex;
use crate::icons::Icons;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::transparent::Shared;
use crate::widget::Widget;

const DEFAULT_WIDTH: f32 = 240.0;
const ROW_H: f32 = 40.0;
const PAD_X: f32 = 12.0;

/// The style the value and the options are drawn in: what the caller said, else what the
/// theme says, else the reference's — a dropdown is `titleMedium`.
///
/// **Resolved once**, so that the number the box is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
pub(crate) fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.dropdown.text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).title_medium)
        .resolved()
}

/// The room the tick takes on the right of a row, its gap included — kept clear on every
/// row rather than on the ticked one, so that a choice the caller drew does not change
/// width when it becomes the selected one.
const TICK: f32 = 18.0;

/// One row: the header, or an option.
struct Row<Msg> {
    /// The row's own words — `None` when the caller drew the choice, in which case their
    /// widget is child 0.
    label: Option<String>,
    /// `[the caller's widget]`, or empty for a row of words.
    children: Vec<Box<dyn Widget<Msg>>>,
    width: f32,
    is_header: bool,
    /// The currently selected option (highlighted + ticked). Ignored for the header.
    selected: bool,
    /// Whether **this row** can be used: the list's availability and the choice's own.
    enabled: bool,
    text_style: Option<TextStyle>,
    on_click: Option<Msg>,
}

impl<Msg> Row<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let line = frus_text::line_box(ROW_H, &label_style(self.text_style, theme), 0.0);
        if self.children.is_empty() {
            return Style {
                width: Dimension::Length(self.width),
                height: Dimension::Length(line),
                ..Default::default()
            };
        }
        Style {
            width: Dimension::Length(self.width),
            // A choice the caller drew **grows**: a two-line entry is half of why the
            // form exists, and a fixed height would cut it in two.
            height: Dimension::Auto,
            min_height: Dimension::Length(line),
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            // The columns are kept clear by padding rather than by empty siblings, so the
            // caller's widget gets exactly the room the words would have had.
            padding: Insets::new(0.0, PAD_X + TICK, 0.0, PAD_X),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Row<Msg> {
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
        // A menu panel is a distinct area within the surface, the `surface_container`
        // role (`menu_anchor.dart:4035`). Selected option: a primary-tinted background;
        // hover on top (the state layer).
        let panel = theme.scheme.surface_container;
        let base = if self.selected && self.enabled {
            panel.lerp(theme.primary, 0.14)
        } else {
            panel
        };
        // No state layer while disabled: a hover tint is a promise that a press would do
        // something. The outline is the row's **container**, so it takes the container
        // opacity rather than the content one.
        let bg = if self.enabled {
            theme.state_layer(base, theme.on_surface, &status)
        } else {
            base
        };
        let outline = if self.enabled {
            theme.border
        } else {
            disabled_container(theme)
        };
        scene.draw_rect(bounds, bg.fade(o), theme.radius, 1.0, outline.fade(o));

        let ink = if self.enabled {
            theme.on_surface
        } else {
            disabled_content(theme)
        };
        let style = label_style(self.text_style, Some(theme));
        let ty = bounds.y + (bounds.height - style.line_height()) * 0.5;
        if let Some(label) = &self.label {
            scene.text(
                Point::new(bounds.x + PAD_X, ty),
                label.clone(),
                &style,
                ink.fade(o),
            );
        }

        if self.is_header {
            // A vector "▾" chevron (a downward-pointing triangle), on the right.
            let cx = bounds.x + self.width - PAD_X - 4.0;
            let cy = bounds.y + bounds.height * 0.5;
            let (w, h) = (5.0, 3.0);
            let tri = Path::new()
                .move_to(Point::new(cx - w, cy - h))
                .line_to(Point::new(cx + w, cy - h))
                .line_to(Point::new(cx, cy + h))
                .close();
            let chevron = if self.enabled {
                theme.muted
            } else {
                disabled_content(theme)
            };
            scene.fill_path(&tri, chevron.fade(o));
        } else if self.selected {
            // The selected option's tick, on the right.
            let size = 18.0;
            let x = bounds.x + self.width - PAD_X - size;
            let y = bounds.y + (bounds.height - size) * 0.5;
            let path = Icons::CHECK.placed(size, x, y, theme.direction);
            // The tick stays: which option is chosen is still owed to a reader who cannot
            // choose another.
            let check = if self.enabled {
                theme.primary
            } else {
                disabled_content(theme)
            };
            scene.fill_path(&path, check.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        self.on_click.clone()
    }

    fn focusable(&self) -> bool {
        self.enabled && self.on_click.is_some()
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A dropdown row said nothing to a reader before this. Announcing that a row is
        // unavailable without ever announcing the row would have been announcing an
        // absence, the same hole `RadioOption` had in milestone 322.
        //
        // A choice the caller drew announces nothing here and leaves that to whatever they
        // drew — a row with no words and a `Button` role would announce an empty button.
        let semantics = frus_core::SemanticsProperties::new(frus_core::Role::Button);
        let semantics = match &self.label {
            Some(label) => semantics.label(label.clone()),
            None if !self.is_header => return None,
            None => semantics,
        };
        let semantics = if self.is_header {
            semantics
        } else {
            semantics.toggled(self.selected)
        };
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// **One choice in a [`DropdownButton`]'s list.**
///
/// [`options`](DropdownButton::options) takes plain strings and is still the right call
/// for the common case; this is what a choice is when it is not one. A colour swatch
/// beside a name, a flag, a two-line entry with a subtitle, a heading — and a choice that
/// is **there but cannot be picked**, which no string can say.
///
/// A choice carries no message: [`options`](DropdownButton::options) and
/// [`options_widgets`](DropdownButton::options_widgets) both map the chosen **index** to
/// one, so that a list built from an application's own vector says what it means once.
///
/// ```
/// use frus_widgets::{Container, DropdownButton, DropdownOption};
/// use frus_core::Color;
///
/// # #[derive(Clone)] enum Msg { Toggle, Pick(usize) }
/// let swatch = |c: Color| Container::<Msg>::new().width(14.0).height(14.0).color(c);
/// let menu = DropdownButton::new("Amber", Msg::Toggle).options_widgets(
///     true,
///     vec![
///         DropdownOption::widget(swatch(Color::rgb8(255, 193, 7))),
///         DropdownOption::widget(swatch(Color::rgb8(3, 169, 244))),
///         DropdownOption::new("Out of stock").enabled(false),
///     ],
///     Msg::Pick,
/// );
/// ```
pub struct DropdownOption<Msg> {
    label: Option<String>,
    child: Option<Rc<dyn Widget<Msg>>>,
    enabled: bool,
}

impl<Msg> DropdownOption<Msg> {
    /// A choice of words — what [`options`](DropdownButton::options) builds.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            child: None,
            enabled: true,
        }
    }

    /// A choice **the caller draws**.
    ///
    /// It is laid out in the room the words would have had, with the tick's column kept
    /// clear, and it **grows**: a choice of words is a row tall, a choice of anything else
    /// is at least that and as much more as it needs.
    pub fn widget(child: impl Widget<Msg> + 'static) -> Self {
        Self {
            label: None,
            child: Some(Rc::new(child)),
            enabled: true,
        }
    }

    /// Whether **this choice** can be picked, over the list's own availability.
    ///
    /// A choice that cannot be picked is still drawn, still announced, and still ticked if
    /// it is the one selected — which happens, and is the reason it is drawn rather than
    /// left out: a list that hides what it cannot offer cannot show what was already
    /// chosen.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The choice's words, if it has any. `None` for a choice the caller drew.
    pub(crate) fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Builds one choice's row.
///
/// Shared with [`crate::DropdownMenu`], which is the same closed set behind a field rather
/// than behind a button and must draw its choices identically — two lists that look nearly
/// alike are worse than one that looks the same.
pub(crate) fn option_row<Msg: Clone + 'static>(
    option: &DropdownOption<Msg>,
    width: f32,
    selected: bool,
    enabled: bool,
    text_style: Option<TextStyle>,
    on_click: Option<Msg>,
) -> impl Widget<Msg> {
    Row {
        label: option.label.clone(),
        children: option
            .child
            .clone()
            .map(|inner| Box::new(Shared::new(inner)) as Box<dyn Widget<Msg>>)
            .into_iter()
            .collect(),
        width,
        is_header: false,
        selected,
        enabled: enabled && option.enabled,
        text_style,
        on_click,
    }
}

/// A single-selection dropdown list (a floating menu).
pub struct DropdownButton<Msg> {
    header_label: String,
    on_toggle: Msg,
    width: f32,
    selected: Option<usize>,
    open: bool,
    enabled: bool,
    options: Vec<DropdownOption<Msg>>,
    text_style: Option<TextStyle>,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> DropdownButton<Msg> {
    /// Creates a list: the current label + the toggle message (open/close).
    pub fn new(selected_label: impl Into<String>, on_toggle: Msg) -> Self {
        let mut dropdown = Self {
            header_label: selected_label.into(),
            on_toggle,
            width: DEFAULT_WIDTH,
            selected: None,
            open: false,
            enabled: true,
            options: Vec::new(),
            text_style: None,
            on_select: None,
            children: Vec::new(),
        };
        dropdown.rebuild();
        dropdown
    }

    /// The value's and the options' type, over the theme's and the reference's.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// Whether the list can be opened or chosen from. Disabled it is **inert** - the
    /// header takes no press, no row takes focus - and it still shows the current choice.
    ///
    /// A disabled list is also **never open**: whatever `options` was told, the menu is
    /// not built, because a floating menu over a control that cannot be chosen from is a
    /// menu that traps a press and returns nothing.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Width of the header and the menu, in logical pixels (240 by default).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self.rebuild();
        self
    }

    /// Index of the **selected** option (highlighted + ticked in the menu).
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self.rebuild();
        self
    }

    /// Sets the options; if `open`, they float below the header. `on_select` maps
    /// the chosen index to a message.
    pub fn options(
        self,
        open: bool,
        labels: &[&str],
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        let options = labels.iter().map(|s| DropdownOption::new(*s)).collect();
        self.options_widgets(open, options, on_select)
    }

    /// The same, for choices that are **not** words: a swatch, a flag, two lines, or a
    /// choice that is there and cannot be picked. See [`DropdownOption`].
    ///
    /// `on_select` is given the index into **this** list, so it is the caller's own order
    /// whatever the widget does with it.
    pub fn options_widgets(
        mut self,
        open: bool,
        options: Vec<DropdownOption<Msg>>,
        on_select: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        self.open = open;
        self.options = options;
        self.on_select = Some(Box::new(on_select));
        self.rebuild();
        self
    }

    /// Rebuilds the header (and the menu if open) from the current state.
    fn rebuild(&mut self) {
        let header = Row {
            label: Some(self.header_label.clone()),
            children: Vec::new(),
            width: self.width,
            is_header: true,
            selected: false,
            enabled: self.enabled,
            text_style: self.text_style,
            on_click: Some(self.on_toggle.clone()),
        };
        self.children = vec![Box::new(header)];

        if self.open && self.enabled && !self.options.is_empty() {
            let mut menu = Flex::column().gap(4.0);
            for (index, option) in self.options.iter().enumerate() {
                let on_click = self
                    .on_select
                    .as_ref()
                    .filter(|_| self.enabled && option.enabled)
                    .map(|f| f(index));
                menu = menu.child(option_row(
                    option,
                    self.width,
                    self.selected == Some(index),
                    self.enabled,
                    self.text_style,
                    on_click,
                ));
            }
            self.children.push(Box::new(menu));
        }
    }
}

impl<Msg: Clone> Widget<Msg> for DropdownButton<Msg> {
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
            .map(|menu| (menu.as_ref(), Placement::Below))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle,
        Select(usize),
    }

    /// **A choice can be a widget the caller drew.** A colour swatch beside a name, a
    /// flag, a two-line entry — none of which a `&str` can say, and all of which a list of
    /// choices is routinely asked for.
    ///
    /// It is laid out in the room the words would have had, with the tick's column kept
    /// clear, and it **grows**: the row is a row tall or as much more as the drawing needs.
    #[test]
    fn a_choice_can_be_a_widget_the_caller_drew() {
        let mark = frus_core::Color::rgb(0.9, 0.1, 0.1);
        let open = DropdownButton::new("Pick", Msg::Toggle)
            .width(200.0)
            .options_widgets(
                true,
                vec![
                    DropdownOption::widget(
                        crate::Container::<Msg>::new()
                            .width(40.0)
                            .height(60.0)
                            .color(mark),
                    ),
                    DropdownOption::new("B"),
                ],
                Msg::Select,
            );
        let (menu, _) = Widget::<Msg>::overlay(&open).unwrap();
        let ui = build_ui(
            menu,
            Size::new(220.0, 200.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let drawn = rects(ui.scene())
            .into_iter()
            .find(|(_, color)| *color == mark)
            .expect("the caller's own widget is drawn");
        assert_eq!(drawn.0.x, PAD_X, "it starts where the words would have");
        assert_eq!(drawn.0.height, 60.0, "and keeps the height it asked for");
    }

    /// **One choice can be unavailable while the list is not.** It was the whole list or
    /// nothing, so an application with one choice it could not offer had to leave it out —
    /// and a list that hides what it cannot offer cannot show what was already chosen.
    ///
    /// It is still drawn, still announced, and still ticked when it is the selected one.
    #[test]
    fn one_choice_can_be_unavailable_while_the_list_is_not() {
        let open = DropdownButton::new("Pick", Msg::Toggle).options_widgets(
            true,
            vec![
                DropdownOption::new("A"),
                DropdownOption::new("B").enabled(false),
            ],
            Msg::Select,
        );
        let menu = &Widget::<Msg>::children(&open)[1];
        assert_eq!(menu.children()[0].on_click(), Some(Msg::Select(0)));
        assert_eq!(
            menu.children()[1].on_click(),
            None,
            "the choice that said it was unavailable answers nothing"
        );
        assert!(
            !menu.children()[1].focusable(),
            "and does not stop the keyboard on the way past"
        );
        let announced = menu.children()[1].semantics().expect("still announced");
        assert!(announced.disabled, "as unavailable");
        assert_eq!(announced.label.as_deref(), Some("B"), "and by name");
    }

    /// Every filled rectangle in the frame, innermost layers included.
    fn rects(scene: &frus_core::Scene) -> Vec<(Rect, frus_core::Color)> {
        fn walk(primitives: &[Primitive], out: &mut Vec<(Rect, frus_core::Color)>) {
            for p in primitives {
                match p {
                    Primitive::Rect { rect, color, .. } => out.push((*rect, *color)),
                    Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(scene.primitives(), &mut out);
        out
    }

    #[test]
    fn closed_has_no_overlay_open_floats_options() {
        let closed =
            DropdownButton::new("Pick one", Msg::Toggle).options(false, &["A", "B"], Msg::Select);
        assert!(
            Widget::<Msg>::overlay(&closed).is_none(),
            "closed: no overlay"
        );

        let open =
            DropdownButton::new("Pick one", Msg::Toggle).options(true, &["A", "B"], Msg::Select);
        assert!(
            Widget::<Msg>::overlay(&open).is_some(),
            "open: a floating menu"
        );
        let menu = &Widget::<Msg>::children(&open)[1];
        assert_eq!(menu.children().len(), 2);
        assert_eq!(menu.children()[1].on_click(), Some(Msg::Select(1)));
    }

    #[test]
    fn header_and_options_are_keyboard_focusable() {
        let open = DropdownButton::new("Pick", Msg::Toggle).options(true, &["A", "B"], Msg::Select);
        // A focusable header (opens from the keyboard) + 2 options.
        assert!(Widget::<Msg>::children(&open)[0].focusable());
        let menu = &Widget::<Msg>::children(&open)[1];
        assert!(menu.children()[0].focusable() && menu.children()[1].focusable());
    }

    #[test]
    fn selected_option_is_highlighted_and_checked() {
        let open = DropdownButton::new("Pick", Msg::Toggle)
            .selected(1)
            .options(true, &["A", "B"], Msg::Select)
            .width(200.0);
        // The menu is an overlay: render it on its own to read its primitives.
        let (menu, _) = Widget::<Msg>::overlay(&open).unwrap();
        let ui = build_ui(
            menu,
            Size::new(220.0, 120.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let theme = Theme::default();
        // The selected option's tick (a filled path).
        let has_check = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Path { .. }));
        assert!(has_check, "the selected option is ticked");
        // The selected option's primary-tinted background.
        let sel = theme.scheme.surface_container.lerp(theme.primary, 0.14);
        let has_tint = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Rect { color, .. } if color.fade(1.0) == sel.fade(1.0)
            )
        });
        assert!(has_tint, "the selected option is highlighted");
    }

    #[test]
    fn a_disabled_list_is_inert_and_cannot_be_open() {
        let dead = DropdownButton::new("Option B", Msg::Toggle)
            .selected(1)
            .options(true, &["A", "B"], Msg::Select)
            .enabled(false);
        // Told to be open, and not open: a floating menu over a control that cannot be
        // chosen from would trap a press and return nothing.
        assert!(Widget::<Msg>::overlay(&dead).is_none(), "no floating menu");
        assert_eq!(Widget::<Msg>::children(&dead).len(), 1, "header only");

        let header = &Widget::<Msg>::children(&dead)[0];
        assert_eq!(header.on_click(), None, "the header takes no press");
        assert!(!header.focusable(), "and no focus");
        let semantics = header.semantics().expect("still announced");
        assert!(semantics.disabled, "announced as unavailable");
        assert_eq!(
            semantics.label.as_deref(),
            Some("Option B"),
            "and still says which option is current"
        );
    }

    #[test]
    fn a_live_list_is_untouched_by_it() {
        let live = DropdownButton::new("Pick one", Msg::Toggle)
            .options(true, &["A", "B"], Msg::Select)
            .enabled(true);
        assert!(Widget::<Msg>::overlay(&live).is_some());
        assert_eq!(
            Widget::<Msg>::children(&live)[0].on_click(),
            Some(Msg::Toggle)
        );
    }
}
