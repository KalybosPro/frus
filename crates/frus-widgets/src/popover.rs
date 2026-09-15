//! [`MenuAnchor`]: a floating panel with **free content**, anchored and controlled, that
//! closes on an outside click. It generalises [`crate::PopupMenuButton`] to any content,
//! and floats that content on the same panel.

use std::rc::Rc;

use frus_core::{BorderRadius, Color, Insets, Rect, Scene, ShapeBorder};
use frus_layout::{FlexDirection, Style};

use crate::interaction::Status;
use crate::menu::{Panel, PanelKind, PanelStyle};
use crate::portal::Placement;
use crate::theme::Theme;
use crate::transparent::Shared;
use crate::widget::Widget;

/// A popover: an anchor plus an optional floating panel.
///
/// **The content floats on a panel**: one surface in `surface_container`, the menu's
/// corner, eight pixels of room above and below, and a shadow three high — the reference's
/// menu defaults, and the panel [`PopupMenuButton`](crate::PopupMenuButton) floats its
/// rows on. Each of those answers to the builders below, then to
/// [`MenuTheme`](crate::MenuTheme). Content that brings a surface of its own — a `Card` —
/// would be a surface on a surface; hand the anchor what goes *on* the panel instead.
pub struct MenuAnchor<Msg> {
    open: bool,
    /// `[anchor]`, or `[anchor, panel]` when open with content.
    children: Vec<Box<dyn Widget<Msg>>>,
    /// The caller's content, shared so the panel can be built again when a builder is
    /// written after [`content`](Self::content).
    content: Option<Rc<dyn Widget<Msg>>>,
    look: PanelStyle,
    on_dismiss: Option<Msg>,
}

impl<Msg: Clone + 'static> MenuAnchor<Msg> {
    /// Creates a popover around an anchor. When `open`, the content floats;
    /// `on_dismiss` is emitted on a click **outside** the popover.
    pub fn new(anchor: impl Widget<Msg> + 'static, open: bool, on_dismiss: Msg) -> Self {
        Self {
            open,
            children: vec![Box::new(anchor)],
            content: None,
            look: PanelStyle::default(),
            on_dismiss: if open { Some(on_dismiss) } else { None },
        }
    }

    /// Sets the floating content, which is shown only when open.
    pub fn content(mut self, content: impl Widget<Msg> + 'static) -> Self {
        if self.open {
            let content: Rc<dyn Widget<Msg>> = Rc::new(content);
            self.content = Some(content);
            self.rebuild();
        }
        self
    }

    /// **The panel's surface**, over the theme's and the reference's `surface_container`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.look.background = Some(color);
        self.rebuild();
        self
    }

    /// **What shape the panel is**, over the theme's and the framework's corner.
    #[must_use]
    pub fn shape(mut self, shape: ShapeBorder) -> Self {
        self.look.shape = Some(shape);
        self.rebuild();
        self
    }

    /// The shorthand for a rounded rectangle — `radius(4.0)` is the reference's own menu
    /// corner. See [`PopupMenuButton::radius`](crate::PopupMenuButton::radius) for why it
    /// is not the default.
    #[must_use]
    pub fn radius(self, radius: impl Into<BorderRadius>) -> Self {
        self.shape(ShapeBorder::rounded(radius.into()))
    }

    /// **How far off the page the panel sits**, in pixels. Three by default.
    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.look.elevation = Some(elevation);
        self.rebuild();
        self
    }

    /// The colour of the panel's shadow, over the theme's and the scheme's shadow at 30 %.
    /// [`Color::TRANSPARENT`] casts none.
    #[must_use]
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.look.shadow_color = Some(color);
        self.rebuild();
        self
    }

    /// The room kept **around** the content, inside the panel. Eight above and below by
    /// default, and nothing either side.
    #[must_use]
    pub fn menu_padding(mut self, padding: Insets) -> Self {
        self.look.padding = Some(padding);
        self.rebuild();
        self
    }

    /// (Re)builds the floating panel (child 1) around the content.
    fn rebuild(&mut self) {
        self.children.truncate(1);
        if let (true, Some(content)) = (self.open, &self.content) {
            self.children.push(Box::new(Panel::new(
                PanelKind::Menu,
                self.look,
                vec![Box::new(Shared::new(content.clone()))],
            )));
        }
    }
}

impl<Msg: Clone> Widget<Msg> for MenuAnchor<Msg> {
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
            .map(|content| (content.as_ref(), Placement::Below))
    }

    fn overlay_dismiss(&self) -> Option<Msg> {
        self.on_dismiss.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::probe::{blurred, crisp};
    use crate::{build_ui, Container, Runtime, Size, Text};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
    }

    fn anchor() -> Container<Msg> {
        Container::<Msg>::new().width(40.0).height(30.0)
    }

    const MARK: Color = Color::rgb(0.9, 0.1, 0.1);

    /// An open anchor floating a 100 × 50 box in a colour of its own.
    fn open() -> MenuAnchor<Msg> {
        MenuAnchor::new(anchor(), true, Msg::Close).content(
            Container::<Msg>::new()
                .width(100.0)
                .height(50.0)
                .color(MARK),
        )
    }

    fn frame(menu: &MenuAnchor<Msg>, theme: &Theme) -> crate::Ui<Msg> {
        build_ui(menu, Size::new(400.0, 400.0), &Runtime::default(), theme)
    }

    #[test]
    fn closed_has_no_overlay() {
        let p = MenuAnchor::new(anchor(), false, Msg::Close).content(Text::new("hi"));
        assert!(Widget::<Msg>::overlay(&p).is_none());
    }

    #[test]
    fn open_floats_content_and_dismisses() {
        let p = MenuAnchor::new(anchor(), true, Msg::Close).content(Text::new("hi"));
        assert!(Widget::<Msg>::overlay(&p).is_some());
        assert_eq!(Widget::<Msg>::overlay_dismiss(&p), Some(Msg::Close));
    }

    /// **The content floats on a panel.** It floated bare: no surface, no corner, no room
    /// around it, and so nothing a shadow could be the shadow of, where the reference's
    /// anchor always wraps what it is given in one.
    ///
    /// One rectangle in `surface_container`, drawn before the content, as wide as the
    /// content and eight taller above and below, rounded, with no outline — and under it
    /// one shadow, three high: a blur of twenty, dropped six, in the scheme's shadow at
    /// 30 %.
    #[test]
    fn the_content_floats_on_one_panel_with_a_shadow_three_high() {
        let theme = Theme::default();
        let ui = frame(&open(), &theme);
        let painted = crisp(ui.scene());
        let panels: Vec<_> = painted
            .iter()
            .filter(|r| r.color == theme.scheme.surface_container)
            .collect();
        assert_eq!(panels.len(), 1, "one surface: {painted:#?}");
        let panel = panels[0];
        let content = painted
            .iter()
            .position(|r| r.color == MARK)
            .expect("the content is drawn");
        let behind = painted
            .iter()
            .position(|r| r.color == theme.scheme.surface_container)
            .unwrap();
        assert!(behind < content, "the panel is painted behind the content");
        let content = painted[content];
        assert_eq!(
            panel.rect.width, content.rect.width,
            "as wide as its content"
        );
        assert_eq!(
            panel.rect.height,
            content.rect.height + 16.0,
            "and eight more each way"
        );
        assert_eq!(content.rect.y, panel.rect.y + 8.0);
        assert!(panel.radius != BorderRadius::ZERO, "rounded");
        assert!(painted.iter().all(|r| r.border_width == 0.0), "no outline");

        let shadows = blurred(ui.scene());
        assert_eq!(shadows.len(), 1, "one shadow: {shadows:#?}");
        let shadow = shadows[0];
        assert_eq!(shadow.blur, 20.0, "three high: four a step from eight");
        assert_eq!(
            shadow.rect.y,
            panel.rect.y + 6.0 - 20.0,
            "dropped two a step"
        );
        assert_eq!(shadow.color, theme.scheme.shadow.with_alpha(0.30));
    }

    /// **Every word about the panel is the caller's to say**, over the theme's, and the
    /// theme's over the framework's: surface, corner, height, shadow colour and room.
    #[test]
    fn the_panel_answers_to_its_theme_and_to_its_caller() {
        let mut theme = Theme::default();
        let (surface, shade) = (Color::rgb(0.2, 0.4, 0.6), Color::rgba(0.0, 0.0, 0.5, 0.5));
        theme.widgets.menu.background = Some(surface);
        theme.widgets.menu.radius = Some(3.0);
        theme.widgets.menu.elevation = Some(1.0);
        theme.widgets.menu.shadow_color = Some(shade);
        theme.widgets.menu.padding = Some(Insets::new(4.0, 4.0, 4.0, 4.0));

        let ui = frame(&open(), &theme);
        let panel = *crisp(ui.scene())
            .iter()
            .find(|r| r.color == surface)
            .expect("the theme's surface");
        assert_eq!(
            panel.radius,
            BorderRadius::uniform(3.0),
            "the theme's corner"
        );
        assert_eq!(
            (panel.rect.width, panel.rect.height),
            (108.0, 58.0),
            "its room"
        );
        let shadow = blurred(ui.scene())[0];
        assert_eq!(
            (shadow.blur, shadow.color),
            (12.0, shade),
            "its height and colour"
        );

        let (own, own_shade) = (Color::rgb(0.9, 0.9, 0.1), Color::rgba(0.5, 0.0, 0.0, 0.4));
        let told = open()
            .background(own)
            .radius(9.0)
            .elevation(2.0)
            .shadow_color(own_shade)
            .menu_padding(Insets::ZERO);
        let ui = frame(&told, &theme);
        let panel = *crisp(ui.scene())
            .iter()
            .find(|r| r.color == own)
            .expect("the caller's surface");
        assert_eq!(panel.radius, BorderRadius::uniform(9.0));
        assert_eq!((panel.rect.width, panel.rect.height), (100.0, 50.0));
        let shadow = blurred(ui.scene())[0];
        assert_eq!((shadow.blur, shadow.color), (16.0, own_shade));
    }

    /// **A transparent shadow colour casts nothing**, on the anchor or on its theme — the
    /// rule of milestone 529 — and a flat panel casts nothing in any colour.
    #[test]
    fn a_transparent_shadow_or_a_flat_panel_casts_nothing() {
        let theme = Theme::default();
        let clear = open().shadow_color(Color::TRANSPARENT);
        assert!(blurred(frame(&clear, &theme).scene()).is_empty());
        assert!(blurred(frame(&open().elevation(0.0), &theme).scene()).is_empty());
        assert!(
            crisp(frame(&open().elevation(0.0), &theme).scene())
                .iter()
                .all(|r| r.color != theme.scheme.shadow.with_alpha(0.30)),
            "not even an unblurred box of it"
        );

        let mut themed = Theme::default();
        themed.widgets.menu.shadow_color = Some(Color::TRANSPARENT);
        assert!(blurred(frame(&open(), &themed).scene()).is_empty());
    }

    /// **The builders can be written in any order**: a look said before the content is the
    /// look said after it.
    ///
    /// Each builder is written **alone** after the content. Written together, the last
    /// one rebuilds the panel for all of them, and a builder that forgot to rebuild would
    /// pass behind it — which is how the first version of this test let exactly that
    /// mutation through.
    #[test]
    fn the_builders_can_be_written_in_any_order() {
        let theme = Theme::default();
        let colour = Color::rgb(0.1, 0.7, 0.3);
        type Builder = fn(MenuAnchor<Msg>) -> MenuAnchor<Msg>;
        let builders: [(&str, Builder); 5] = [
            ("background", |m| m.background(Color::rgb(0.1, 0.7, 0.3))),
            ("shape", |m| m.radius(2.0)),
            ("elevation", |m| m.elevation(5.0)),
            ("shadow_color", |m| {
                m.shadow_color(Color::rgba(0.0, 0.5, 0.0, 0.5))
            }),
            ("menu_padding", |m| {
                m.menu_padding(Insets::new(1.0, 2.0, 3.0, 4.0))
            }),
        ];
        for (name, say) in builders {
            let first = say(MenuAnchor::new(anchor(), true, Msg::Close)).content(
                Container::<Msg>::new()
                    .width(100.0)
                    .height(50.0)
                    .color(MARK),
            );
            let last = say(open());
            let (before, after) = (frame(&first, &theme), frame(&last, &theme));
            assert_eq!(crisp(before.scene()), crisp(after.scene()), "{name}");
            assert_eq!(blurred(before.scene()), blurred(after.scene()), "{name}");
            let untold = frame(&open(), &theme);
            assert_ne!(
                (crisp(after.scene()), blurred(after.scene())),
                (crisp(untold.scene()), blurred(untold.scene())),
                "{name} after the content changed something"
            );
        }
        assert!(crisp(frame(&open().background(colour), &theme).scene())
            .iter()
            .any(|r| r.color == colour));
    }

    /// **A press on the panel's own room does not close it**, and one outside still does.
    #[test]
    fn a_press_on_the_panel_does_not_close_it() {
        let ui = frame(&open(), &Theme::default());
        let press = |x, y| {
            ui.hit(frus_core::Point::new(x, y))
                .and_then(|id| ui.msg_for(id))
        };
        assert_eq!(press(50.0, 34.0), None, "the room above the content");
        assert_eq!(press(390.0, 390.0), Some(Msg::Close));
    }
}
