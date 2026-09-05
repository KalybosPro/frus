//! [`PopupMenuButton`]: a **floating** action menu — an anchor plus a list of items that opens
//! over it, through the overlay, and closes on an outside click.

use frus_core::{
    BorderRadius, Color, Insets, Point, Rect, ResolvedTextStyle, Scene, ShapeBorder, TextStyle,
};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::disabled_content;
use crate::flex::Flex;
use crate::interaction::Status;
use crate::portal::Placement;
use crate::theme::Theme;
use crate::widget::Widget;

const WIDTH: f32 = 220.0;
/// **One row's height** — the smallest box a control worked by a finger reserves for
/// it, which is what the reference gives a menu item (`popup_menu.dart:279`). It was 38,
/// ten pixels under the number the accessibility scanners on both mobile platforms check
/// for, on a widget whose entire purpose is to be tapped.
const ROW_H: f32 = crate::theme::MIN_TAP_TARGET;
/// The room either side of a row's label (`popup_menu.dart:1876`).
const PAD_X: f32 = 12.0;
/// The room above and below the rows, inside the panel (`popup_menu.dart:1872`). It is
/// twice the panel's own corner, which is why a row's highlight can never reach a curve
/// and the panel needs no clip.
const PAD_Y: f32 = 8.0;
/// How far off the page the panel sits (`popup_menu.dart:1839`).
const ELEVATION: f32 = 3.0;

/// The panel's surface: the caller's word, then the theme's, then the reference's
/// — `surface_container`, a menu being a **distinct area within** the surface rather
/// than something floating above it in a colour of its own (`popup_menu.dart:1858`).
fn panel_background(own: Option<Color>, theme: &Theme) -> Color {
    own.or(theme.widgets.menu.background)
        .unwrap_or(theme.scheme.surface_container)
}

/// The style the items are drawn in: what the caller said, else what the theme says, else
/// the reference's — a popup menu's items are `labelLarge` (`popup_menu.dart:1849`).
///
/// **Resolved once**, so that the number the box is measured with is the number the glyphs
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403); a size that never passes through it is a size the reader cannot change.
fn label_style(over: Option<TextStyle>, theme: Option<&Theme>) -> ResolvedTextStyle {
    over.or(theme.and_then(|t| t.widgets.menu.text_style))
        .unwrap_or_else(|| crate::theme::type_scale(theme).label_large)
        .resolved()
}

/// One menu action, a clickable row.
///
/// **Not a button.** It has no surface and no outline of its own: it is a strip of the
/// panel that lights up under a pointer, which is what a row in a list of actions is
/// everywhere it appears. It carries the panel's resolved colour down so its state layer
/// has the right thing to sit on.
struct Item<Msg> {
    label: String,
    /// The menu's availability, handed down to every row.
    enabled: bool,
    text_style: Option<TextStyle>,
    /// The caller's panel colour, so a row's highlight tints the surface it is drawn on
    /// and not the one the framework would have picked.
    background: Option<Color>,
    /// The caller's row padding and row height, if either was named.
    padding: Option<Insets>,
    height: Option<f32>,
    message: Msg,
}

impl<Msg> Item<Msg> {
    /// The room either side of the label: the caller's, then the theme's, then the
    /// reference's twelve.
    fn padding(&self, theme: Option<&Theme>) -> Insets {
        self.padding
            .or(theme.and_then(|t| t.widgets.menu.item_padding))
            .unwrap_or(Insets::new(0.0, PAD_X, 0.0, PAD_X))
    }

    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let height = self
            .height
            .or(theme.and_then(|t| t.widgets.menu.item_height))
            .unwrap_or(ROW_H);
        Style {
            width: Dimension::Length(WIDTH),
            // The row grows if the reader's type does not fit in it — the height is a
            // floor, not a promise.
            height: Dimension::Length(frus_text::line_box(
                height,
                &label_style(self.text_style, theme),
                0.0,
            )),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Item<Msg> {
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
        // A row draws **nothing at rest**. The panel behind it is the surface; all this
        // adds is the state layer that says a pointer is over it. No state layer while
        // disabled: a hover tint promises that a press would do something.
        //
        // It used to draw a filled, outlined, rounded rectangle per row — which made a
        // menu a stack of buttons with two-pixel gutters showing the page through, where
        // the reference has one panel with rows inside it.
        let base = panel_background(self.background, theme);
        if self.enabled {
            let tinted = theme.state_layer(base, theme.on_surface, &status);
            if tinted != base {
                scene.fill_rect(bounds, tinted.fade(o));
            }
        }
        let ink = if self.enabled {
            theme.on_surface
        } else {
            disabled_content(theme)
        };
        let style = label_style(self.text_style, Some(theme));
        let ty = bounds.y + (bounds.height - style.line_height()) * 0.5;
        scene.text(
            Point::new(bounds.x + self.padding(Some(theme)).left, ty),
            self.label.clone(),
            &style,
            ink.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A menu row said nothing to a reader before this.
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Button).label(self.label.clone());
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }
}

/// **The thing a menu actually is**: one surface, off the page, with the rows inside it.
///
/// This did not exist. `rebuild` handed the overlay a bare column of rows, so a menu was
/// a stack of outlined buttons with the page showing through two-pixel gutters — which
/// is not what a menu looks like anywhere, and is not what the reference draws
/// (`popup_menu.dart:1837`).
struct Panel<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    background: Option<Color>,
    shape: Option<ShapeBorder>,
    elevation: Option<f32>,
    padding: Option<Insets>,
}

impl<Msg> Panel<Msg> {
    /// What shape the panel is: the caller's word, then the theme's shape, then the
    /// theme's plain radius, then the framework's own corner.
    ///
    /// The framework's is `theme.radius` and **not** the reference's four. The reference
    /// gives every component its own corner — four for a menu, twelve for a card,
    /// twenty-eight for a sheet — where this framework collapses them into one number an
    /// application sets once. Reaching into that collapse for one widget would make the
    /// menu the only thing on screen that ignores it; `MenuTheme::radius` is there for an
    /// application that wants the reference's number.
    fn shape_of(&self, theme: &Theme) -> ShapeBorder {
        crate::resolve_shape(
            self.shape,
            theme.widgets.menu.shape,
            theme.widgets.menu.radius.map(BorderRadius::uniform),
            ShapeBorder::rounded(theme.radius),
        )
    }

    /// The room above and below the rows: eight, which is twice the corner and so keeps a
    /// row's highlight clear of the curve without a clip.
    fn padding(&self, theme: &Theme) -> Insets {
        self.padding
            .or(theme.widgets.menu.padding)
            .unwrap_or(Insets::new(PAD_Y, 0.0, PAD_Y, 0.0))
    }
}

impl<Msg: Clone> Widget<Msg> for Panel<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            padding: Insets::new(PAD_Y, 0.0, PAD_Y, 0.0),
            ..Default::default()
        }
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        Style {
            flex_direction: FlexDirection::Column,
            padding: self.padding(theme),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let shape = self.shape_of(theme);
        let radius = shape
            .as_rounded(bounds)
            .map(|(_, r)| r)
            .unwrap_or(BorderRadius::ZERO);
        let depth = self
            .elevation
            .or(theme.widgets.menu.elevation)
            .unwrap_or(ELEVATION);
        if depth > 0.0 {
            let blur = depth * 4.0 + 8.0;
            scene.shadow(
                Rect::new(
                    bounds.x - blur,
                    bounds.y + depth * 2.0 - blur,
                    bounds.width + 2.0 * blur,
                    bounds.height + 2.0 * blur,
                ),
                theme.scheme.shadow.with_alpha(0.30).fade(o),
                radius.inflate(blur),
                blur,
            );
        }
        // Opaque, and **no outline**: a panel that is off the page says so with its
        // shadow. A shadow and a hairline together is the mash-up milestone 279 took out
        // of the card.
        scene.draw_shape(
            bounds,
            shape,
            panel_background(self.background, theme).fade(o),
        );
    }

    /// The panel itself answers nothing: the rows do. It **traps** the press all the
    /// same, by being an opaque thing under the pointer, which is what keeps a click on
    /// the gap between two rows from reaching the page and dismissing the menu.
    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A controlled action menu, opened and closed by the application.
pub struct PopupMenuButton<Msg> {
    open: bool,
    enabled: bool,
    /// `[anchor]`, or `[anchor, list]` when the menu is showing.
    children: Vec<Box<dyn Widget<Msg>>>,
    items: Vec<(String, Msg)>,
    text_style: Option<TextStyle>,
    /// The panel's look, and the rows'. Every builder that writes one of these calls
    /// [`rebuild`](Self::rebuild), so **the order they are written in does not matter**
    /// — unlike [`BottomSheet`](crate::BottomSheet), where the panel is built by
    /// `body` and anything said after it is dropped.
    background: Option<Color>,
    shape: Option<ShapeBorder>,
    elevation: Option<f32>,
    menu_padding: Option<Insets>,
    item_padding: Option<Insets>,
    item_height: Option<f32>,
    dismiss: Option<Msg>,
    on_dismiss: Option<Msg>,
}

impl<Msg: Clone + 'static> PopupMenuButton<Msg> {
    /// Creates a menu around an anchor. When `open`, the list floats above it;
    /// `on_dismiss` is emitted on a click **outside** the menu.
    pub fn new(anchor: impl Widget<Msg> + 'static, open: bool, on_dismiss: Msg) -> Self {
        Self {
            open,
            enabled: true,
            children: vec![Box::new(anchor)],
            items: Vec::new(),
            text_style: None,
            background: None,
            shape: None,
            elevation: None,
            menu_padding: None,
            item_padding: None,
            item_height: None,
            dismiss: Some(on_dismiss.clone()),
            on_dismiss: if open { Some(on_dismiss) } else { None },
        }
    }

    /// The items' type, over the theme's and the reference's.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self.rebuild();
        self
    }

    /// **The panel's surface**, over the theme's and the reference's `surface_container`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.rebuild();
        self
    }

    /// **What shape the panel is**, over the theme's and the framework's corner.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.shape = Some(shape);
        self.rebuild();
        self
    }

    /// The shorthand for a rounded rectangle — `radius(4.0)` is the reference's own
    /// menu corner, which this framework does not use by default because it keeps **one**
    /// corner for the whole interface, in `Theme::radius`. Reaching past that for a single
    /// widget would make the menu the only thing on screen ignoring the number an
    /// application set.
    #[must_use]
    pub fn radius(self, radius: impl Into<BorderRadius>) -> Self {
        self.shape(ShapeBorder::rounded(radius.into()))
    }

    /// **How far off the page the panel sits**, in pixels. Three by default.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self.rebuild();
        self
    }

    /// The room kept **above and below** the rows, inside the panel.
    #[must_use]
    pub fn menu_padding(mut self, padding: Insets) -> Self {
        self.menu_padding = Some(padding);
        self.rebuild();
        self
    }

    /// The room kept either side of a row's label.
    #[must_use]
    pub fn item_padding(mut self, padding: Insets) -> Self {
        self.item_padding = Some(padding);
        self.rebuild();
        self
    }

    /// **How tall one row is**, as a floor — a row still grows to fit the reader's
    /// type. The default is the smallest box a finger can be asked to hit.
    #[must_use]
    pub fn item_height(mut self, height: f32) -> Self {
        self.item_height = Some(height);
        self.rebuild();
        self
    }

    /// Whether the menu can be used. Disabled it is **inert** and, like a disabled
    /// [`DropdownButton`](crate::DropdownButton), **never open**: a floating panel over an anchor that
    /// answers nothing traps a press and returns no message.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.on_dismiss = if self.open && enabled {
            self.dismiss.clone()
        } else {
            None
        };
        self.rebuild();
        self
    }

    /// Adds an action: a label plus a message on click. Ignored when the menu is closed.
    pub fn item(mut self, label: impl Into<String>, message: Msg) -> Self {
        if self.open {
            self.items.push((label.into(), message));
            self.rebuild();
        }
        self
    }

    /// (Re)builds the floating panel (child 1) from the items.
    ///
    /// A disabled menu keeps only its anchor, so there is no overlay to return and no
    /// panel to trap a press.
    fn rebuild(&mut self) {
        if !self.enabled {
            self.children.truncate(1);
            return;
        }
        // No gap. The rows are contiguous strips of one surface; the two-pixel gutter
        // this used to leave showed the page through the middle of the menu.
        let mut list = Flex::column();
        for (label, message) in &self.items {
            list = list.child(Item {
                label: label.clone(),
                enabled: self.enabled,
                text_style: self.text_style,
                background: self.background,
                padding: self.item_padding,
                height: self.item_height,
                message: message.clone(),
            });
        }
        let panel: Box<dyn Widget<Msg>> = Box::new(Panel {
            children: vec![Box::new(list)],
            background: self.background,
            shape: self.shape,
            elevation: self.elevation,
            padding: self.menu_padding,
        });
        if self.children.len() > 1 {
            self.children[1] = panel;
        } else {
            self.children.push(panel);
        }
    }
}

impl<Msg: Clone> Widget<Msg> for PopupMenuButton<Msg> {
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
            .map(|panel| (panel.as_ref(), Placement::Below))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }

    fn overlay_traps_focus(&self) -> bool {
        // An open menu **traps** keyboard focus in its items — Escape or an outside
        // click closes it through `on_dismiss` — the keyboard pattern menus are expected
        // to follow.
        self.open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Point as P, Runtime, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
        A,
        B,
    }

    fn anchor() -> Container<Msg> {
        Container::<Msg>::new().width(40.0).height(30.0)
    }

    /// Every filled rectangle in the frame, innermost layers included, as
    /// `(rect, colour, radius, border width)`.
    fn rects(scene: &frus_core::Scene) -> Vec<(Rect, frus_core::Color, BorderRadius, f32)> {
        fn walk(
            primitives: &[frus_core::Primitive],
            out: &mut Vec<(Rect, frus_core::Color, BorderRadius, f32)>,
        ) {
            for p in primitives {
                match p {
                    frus_core::Primitive::Rect {
                        rect,
                        color,
                        radius,
                        border_width,
                        ..
                    } => out.push((*rect, *color, *radius, *border_width)),
                    frus_core::Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(scene.primitives(), &mut out);
        out
    }

    fn open_menu() -> PopupMenuButton<Msg> {
        PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("A", Msg::A)
            .item("B", Msg::B)
    }

    fn frame(menu: &PopupMenuButton<Msg>, theme: &Theme) -> crate::Ui<Msg> {
        build_ui(menu, Size::new(400.0, 400.0), &Runtime::default(), theme)
    }

    /// **A menu is one panel, not a stack of buttons.**
    ///
    /// It used to draw a filled, outlined, rounded rectangle **per row**, with a
    /// two-pixel gutter between them showing the page through the middle of the menu.
    /// That is not what a menu looks like anywhere, and the reference draws one surface
    /// with rows inside it (`popup_menu.dart:1837`).
    ///
    /// So: one opaque rectangle the width of the menu, in `surface_container`, and
    /// **not one border anywhere** — a panel that is off the page says so with its
    /// shadow, and a shadow with a hairline is the mash-up milestone 279 took out of the
    /// card.
    #[test]
    fn a_menu_is_one_panel_and_not_a_stack_of_buttons() {
        let theme = Theme::default();
        let ui = frame(&open_menu(), &theme);
        let painted = rects(ui.scene());

        let panels: Vec<_> = painted
            .iter()
            .filter(|(rect, color, ..)| {
                rect.width >= WIDTH && *color == theme.scheme.surface_container
            })
            .collect();
        assert_eq!(
            panels.len(),
            1,
            "one surface, not one per row: {painted:#?}"
        );
        assert!(
            panels[0].2 != BorderRadius::ZERO,
            "and it is rounded: {:?}",
            panels[0].2
        );

        assert!(
            painted.iter().all(|(.., border)| *border == 0.0),
            "nothing in an open menu draws an outline: {painted:#?}"
        );
    }

    /// **A row is at least a tap target.** It was 38 pixels tall — ten under the
    /// number the accessibility scanners on both mobile platforms check for, on a widget
    /// whose entire purpose is to be tapped. The reference gives a menu item
    /// `kMinInteractiveDimension` (`popup_menu.dart:279`).
    ///
    /// The panel is measured too: two rows of 48 plus 8 above and 8 below.
    #[test]
    fn a_row_is_at_least_a_tap_target() {
        let theme = Theme::default();
        let ui = frame(&open_menu(), &theme);
        let panel = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| find_panel(p, &theme))
            .expect("a panel");
        assert!(
            panel.height >= 2.0 * crate::theme::MIN_TAP_TARGET + 2.0 * PAD_Y,
            "two rows of a tap target each, plus the panel's own room: {panel:?}"
        );
    }

    fn find_panel(p: &frus_core::Primitive, theme: &Theme) -> Option<Rect> {
        match p {
            frus_core::Primitive::Rect { rect, color, .. }
                if rect.width >= WIDTH && *color == theme.scheme.surface_container =>
            {
                Some(*rect)
            }
            frus_core::Primitive::Layer { primitives, .. } => {
                primitives.iter().find_map(|p| find_panel(p, theme))
            }
            _ => None,
        }
    }

    /// **The panel answers to a theme, and to its caller over the theme** — the
    /// surface, the corner, the room inside it and how tall a row is. None of these were
    /// reachable at all: a menu picked every one of them for itself.
    #[test]
    fn a_menu_answers_to_its_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        theme.widgets.menu.background = Some(frus_core::Color::rgb(0.2, 0.4, 0.6));
        theme.widgets.menu.radius = Some(3.0);
        theme.widgets.menu.item_height = Some(60.0);
        theme.widgets.menu.padding = Some(Insets::new(20.0, 0.0, 20.0, 0.0));

        let ui = frame(&open_menu(), &theme);
        let panel = rects(ui.scene())
            .into_iter()
            .find(|(rect, color, ..)| {
                rect.width >= WIDTH && *color == frus_core::Color::rgb(0.2, 0.4, 0.6)
            })
            .expect("the theme's surface");
        assert_eq!(panel.2, BorderRadius::uniform(3.0), "the theme's corner");
        assert_eq!(
            panel.0.height, 160.0,
            "two rows of 60 and 20 above and below"
        );

        // The caller outranks it, all four.
        let told = open_menu()
            .background(frus_core::Color::rgb(0.9, 0.1, 0.1))
            .radius(9.0)
            .item_height(50.0)
            .menu_padding(Insets::ZERO);
        let ui = frame(&told, &theme);
        let panel = rects(ui.scene())
            .into_iter()
            .find(|(rect, color, ..)| {
                rect.width >= WIDTH && *color == frus_core::Color::rgb(0.9, 0.1, 0.1)
            })
            .expect("the caller's surface");
        assert_eq!(panel.2, BorderRadius::uniform(9.0));
        assert_eq!(panel.0.height, 100.0);
    }

    /// **The order the builders are written in does not matter.** Every one of them
    /// rebuilds the panel, so `.item(..)` after `.background(..)` and `.background(..)`
    /// after `.item(..)` produce the same menu.
    ///
    /// This is the trap milestone 458 found in [`BottomSheet`](crate::BottomSheet), where
    /// the panel is built by `body` and anything said after it is silently dropped. It is
    /// worth a test here precisely because it is the kind of thing that is true when
    /// written and quietly stops being true later.
    #[test]
    fn the_builders_can_be_written_in_any_order() {
        let theme = Theme::default();
        let colour = frus_core::Color::rgb(0.9, 0.1, 0.1);
        let first = PopupMenuButton::new(anchor(), true, Msg::Close)
            .background(colour)
            .item("A", Msg::A)
            .item("B", Msg::B);
        let last = open_menu().background(colour);
        assert_eq!(
            rects(frame(&first, &theme).scene()),
            rects(frame(&last, &theme).scene())
        );
    }

    #[test]
    fn closed_has_no_overlay() {
        let menu = PopupMenuButton::new(anchor(), false, Msg::Close).item("A", Msg::A);
        assert!(Widget::<Msg>::overlay(&menu).is_none());
        assert_eq!(Widget::<Msg>::children(&menu).len(), 1);
    }

    #[test]
    fn open_floats_items_and_dismisses_outside() {
        let menu = PopupMenuButton::new(anchor(), true, Msg::Close)
            .item("A", Msg::A)
            .item("B", Msg::B);
        assert!(Widget::<Msg>::overlay(&menu).is_some());
        assert_eq!(Widget::<Msg>::overlay_dismiss(&menu), Some(Msg::Close));

        let ui = build_ui(
            &menu,
            Size::new(400.0, 300.0),
            &Runtime::default(),
            &Theme::default(),
        );
        // A click far from the anchor (the bottom-right corner) closes the menu.
        let corner = ui.hit(P::new(390.0, 290.0)).expect("hit de fermeture");
        assert_eq!(ui.msg_for(corner), Some(Msg::Close));
    }
}
