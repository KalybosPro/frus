//! **The title bar's line**, shared with the system (milestone 640).
//!
//! On a desktop whose system allows it, a [`Scaffold`](crate::Scaffold) with a
//! [`menu_bar`](crate::Scaffold::menu_bar) puts the menu bar on the window's title bar line,
//! as a code editor does. The system keeps its own window: its icon still opens the window
//! menu, its three buttons still minimize, maximize and close, and the empty part of the
//! line still moves the window and maximizes it on a double click. frus paints the line and
//! the menu bar on it.
//!
//! The shell says when the line is shared, and what the system keeps on it, through
//! [`MediaQuery::title_bar`](crate::MediaQuery::title_bar).

use frus_core::{Color, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// One of the window's three buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionButton {
    /// Minimize.
    Minimize,
    /// Maximize, or restore a maximized window.
    Maximize,
    /// Close.
    Close,
}

/// **The title bar's line, when the application's content shares it**, as the shell
/// reports it: how tall the line is, and what the system keeps on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBar {
    /// The line's height, in logical pixels: the system's.
    pub height: f32,
    /// Room the system keeps at the line's start, in logical pixels — buttons it draws
    /// itself there.
    pub leading: f32,
    /// Where the system's three buttons are, in logical pixels from the window's top left,
    /// when the application paints them; `None` when the system draws them itself.
    pub buttons: Option<Rect>,
    /// The window's icon, to paint at the line's start; `None` when the system shows none
    /// there.
    pub icon: Option<&'static [u8]>,
    /// The button the pointer is over.
    pub hovered: Option<CaptionButton>,
    /// The button being pressed.
    pub pressed: Option<CaptionButton>,
    /// Whether the window is maximized: the middle button then restores.
    pub maximized: bool,
    /// Whether the window is the active one: an inactive window's glyphs are quieter.
    pub active: bool,
}

/// What a part of the title bar's line is to the system, which acts on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleBarRole {
    /// The window's icon: the system opens the window menu on a press, and closes the window
    /// on a double press.
    Icon,
    /// The window's three buttons: the system acts on them.
    Buttons,
}

/// The window's three buttons, **painted where the system has them**, for the system to act
/// on — the minimize, maximize-or-restore and close glyphs of the desktop, in cells of equal
/// width, with the system's hover and press states. Their colours are
/// [`CaptionButtonsTheme`](crate::CaptionButtonsTheme)'s.
pub(crate) struct CaptionButtons {
    bar: TitleBar,
    width: f32,
}

impl CaptionButtons {
    pub(crate) fn new(bar: TitleBar, width: f32) -> Self {
        Self { bar, width }
    }
}

/// The glyph's side, in logical pixels.
const GLYPH: f32 = 10.0;

impl<Msg> Widget<Msg> for CaptionButtons {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.bar.height),
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let t = &theme.widgets.caption_buttons;
        let surface = theme.scheme.surface_container;
        let glyph = t.glyph_color.unwrap_or(theme.scheme.on_surface);
        let glyph = if self.bar.active {
            glyph
        } else {
            t.inactive_glyph_color
                .unwrap_or_else(|| surface.lerp(glyph, 0.45))
        };
        let cell = bounds.width / 3.0;
        for (i, button) in [
            CaptionButton::Minimize,
            CaptionButton::Maximize,
            CaptionButton::Close,
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(bounds.x + cell * i as f32, bounds.y, cell, bounds.height);
            let hovered = self.bar.hovered == Some(button);
            let pressed = self.bar.pressed == Some(button);
            let close = button == CaptionButton::Close;
            // The system's states: a wash under the pointer and a lighter one pressed; the
            // close button turns red, and its glyph white.
            let fill = match (close, hovered, pressed) {
                (true, _, true) => Some(
                    t.close_pressed_color
                        .unwrap_or(Color::rgb(0.70, 0.17, 0.12)),
                ),
                (true, true, _) => {
                    Some(t.close_hover_color.unwrap_or(Color::rgb(0.77, 0.17, 0.11)))
                }
                (false, _, true) => Some(
                    t.pressed_color
                        .unwrap_or_else(|| surface.lerp(theme.scheme.on_surface, 0.04)),
                ),
                (false, true, _) => Some(
                    t.hover_color
                        .unwrap_or_else(|| surface.lerp(theme.scheme.on_surface, 0.08)),
                ),
                _ => None,
            };
            if let Some(fill) = fill {
                scene.fill_rect(r, fill.fade(o));
            }
            let ink = if close && (hovered || pressed) {
                t.close_glyph_hover_color.unwrap_or(Color::WHITE)
            } else {
                glyph
            };
            let cx = r.x + r.width * 0.5;
            let cy = r.y + r.height * 0.5;
            let h = GLYPH * 0.5;
            let path = match button {
                CaptionButton::Minimize => Path::new()
                    .move_to(Point::new(cx - h, cy))
                    .line_to(Point::new(cx + h, cy)),
                CaptionButton::Maximize if self.bar.maximized => {
                    // Restore: a square, and a second one behind it, up and to the right.
                    let s = GLYPH - 2.0;
                    let (x, y) = (cx - h, cy - h + 2.0);
                    Path::new()
                        .move_to(Point::new(x, y))
                        .line_to(Point::new(x + s, y))
                        .line_to(Point::new(x + s, y + s))
                        .line_to(Point::new(x, y + s))
                        .close()
                        .move_to(Point::new(x + 2.0, y))
                        .line_to(Point::new(x + 2.0, y - 2.0))
                        .line_to(Point::new(x + s + 2.0, y - 2.0))
                        .line_to(Point::new(x + s + 2.0, y + s - 2.0))
                        .line_to(Point::new(x + s, y + s - 2.0))
                }
                CaptionButton::Maximize => Path::new()
                    .move_to(Point::new(cx - h, cy - h))
                    .line_to(Point::new(cx + h, cy - h))
                    .line_to(Point::new(cx + h, cy + h))
                    .line_to(Point::new(cx - h, cy + h))
                    .close(),
                CaptionButton::Close => Path::new()
                    .move_to(Point::new(cx - h, cy - h))
                    .line_to(Point::new(cx + h, cy + h))
                    .move_to(Point::new(cx + h, cy - h))
                    .line_to(Point::new(cx - h, cy + h)),
            };
            scene.stroke_path(&path, ink.fade(o), 1.0);
        }
    }

    fn title_bar_role(&self) -> Option<TitleBarRole> {
        Some(TitleBarRole::Buttons)
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "CaptionButtons"
    }
}

/// The window's icon on the title bar's line: the system opens the window menu from it.
pub(crate) struct WindowIcon<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: 'static> WindowIcon<Msg> {
    pub(crate) fn new(png: &'static [u8], side: f32) -> Self {
        Self {
            children: vec![Box::new(crate::Image::memory(png).width(side).height(side))],
        }
    }
}

impl<Msg> Widget<Msg> for WindowIcon<Msg> {
    fn style(&self) -> Style {
        Style::default()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn title_bar_role(&self) -> Option<TitleBarRole> {
        Some(TitleBarRole::Icon)
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "WindowIcon"
    }
}

/// The row a scaffold puts its menu bar in, which says it wants the title bar's line.
pub(crate) struct TitleBarRow<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    width: f32,
    height: f32,
}

impl<Msg> TitleBarRow<Msg> {
    pub(crate) fn new(children: Vec<Box<dyn Widget<Msg>>>, width: f32, height: f32) -> Self {
        Self {
            children,
            width,
            height,
        }
    }
}

impl<Msg> Widget<Msg> for TitleBarRow<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: frus_layout::FlexDirection::Row,
            align: frus_layout::Align::Center,
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let bg = theme
            .widgets
            .menu_bar
            .background
            .unwrap_or(theme.scheme.surface_container);
        scene.fill_rect(bounds, bg.fade(status.opacity));
    }

    fn wants_title_bar(&self) -> bool {
        true
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "TitleBarRow"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, MediaQuery, MenuBar, MenuPath, Runtime, Scaffold, Size, SubmenuButton};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Menu(MenuPath),
    }

    const SIZE: Size = Size::new(1000.0, 700.0);
    /// The frus logo, a real PNG.
    const ICON: &[u8] = include_bytes!("../../frus-shell/assets/icon.png");

    fn bar() -> MenuBar<Msg> {
        MenuBar::new(&MenuPath::closed(), Msg::Menu).menu(SubmenuButton::new("File"))
    }

    /// A scaffold with a title and a menu bar, built under `line`, and laid out.
    fn scaffold(line: Option<TitleBar>) -> crate::Ui<Msg> {
        let surface = MediaQuery::new(SIZE).with_title_bar(line);
        let tree = surface.scope(|| {
            Scaffold::<Msg>::new()
                .app_bar(crate::Text::new("Title"))
                .persistent_footer(crate::Text::new("Foot"))
                .fab(crate::Text::new("Fab"))
                .menu_bar(bar())
                .build()
        });
        let theme = Theme::default().with_platform(frus_core::TargetPlatform::Windows);
        surface.scope(|| build_ui(tree.as_ref(), SIZE, &Runtime::default(), &theme))
    }

    fn text_at(ui: &crate::Ui<Msg>, label: &str) -> Rect {
        ui.scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { text, .. } if text == label => Some(p.bounds()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{label:?} is painted"))
    }

    fn line(buttons: Option<Rect>, icon: Option<&'static [u8]>) -> TitleBar {
        TitleBar {
            height: 40.0,
            leading: 0.0,
            buttons,
            icon,
            hovered: None,
            pressed: None,
            maximized: false,
            active: true,
        }
    }

    /// **A scaffold's menu bar is its first line**, above the app bar, and asks for the
    /// title bar's line; off it, a desktop's 30 px.
    #[test]
    fn a_scaffold_s_menu_bar_is_its_first_line() {
        let ui = scaffold(None);
        let file = text_at(&ui, "File");
        let title = text_at(&ui, "Title");
        assert!(file.y + file.height <= 30.0, "in the first 30 px: {file:?}");
        assert!(title.y >= 30.0, "the app bar under it: {title:?}");
        assert!(ui.wants_title_bar(), "and it asks for the line");
        // The rest of the scaffold is the window less the row: its footer still ends on
        // the screen.
        for part in ["Foot", "Fab"] {
            let r = text_at(&ui, part);
            assert!(
                r.y + r.height <= SIZE.height,
                "{part} is in the window: {r:?}"
            );
        }
        assert!(
            ui.title_bar_regions().is_empty(),
            "nothing of the system's on it"
        );
    }

    /// **On the title bar's line**, the row is the system's height, the window's icon is
    /// at its start, and the three buttons are where the system has them.
    #[test]
    fn on_the_title_bar_line_the_system_s_parts_are_painted_where_it_has_them() {
        let buttons = Rect::new(862.0, 0.0, 138.0, 40.0);
        let ui = scaffold(Some(line(Some(buttons), Some(ICON))));
        let title = text_at(&ui, "Title");
        assert!(title.y >= 40.0, "the app bar under the line: {title:?}");
        let regions = ui.title_bar_regions();
        let (button_box, _) = regions
            .iter()
            .find(|(_, role)| *role == TitleBarRole::Buttons)
            .expect("the buttons");
        assert_eq!(*button_box, buttons, "where the system has them");
        let (icon_box, _) = regions
            .iter()
            .find(|(_, role)| *role == TitleBarRole::Icon)
            .expect("the icon");
        assert!(
            icon_box.x < 40.0 && icon_box.width >= 16.0,
            "at the start: {icon_box:?}"
        );
        let file = text_at(&ui, "File");
        assert!(
            file.x > icon_box.x + icon_box.width,
            "the menu after the icon"
        );
    }

    /// The three glyphs and the system's states, painted.
    fn painted(hovered: Option<CaptionButton>, maximized: bool) -> Vec<frus_core::Primitive> {
        let bar = TitleBar {
            hovered,
            maximized,
            ..line(Some(Rect::new(0.0, 0.0, 138.0, 40.0)), None)
        };
        let mut scene = Scene::new();
        Widget::<Msg>::paint(
            &CaptionButtons::new(bar, 138.0),
            Rect::new(0.0, 0.0, 138.0, 40.0),
            Status {
                opacity: 1.0,
                ..Default::default()
            },
            &Theme::dark(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    /// **Three glyphs at rest, a wash under the pointer, and red under it on close.**
    #[test]
    fn the_buttons_take_the_system_s_states() {
        let fills = |prims: &[frus_core::Primitive]| -> Vec<(Rect, Color)> {
            prims
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Rect { rect, color, .. } => Some((*rect, *color)),
                    _ => None,
                })
                .collect()
        };
        let strokes = |prims: &[frus_core::Primitive]| {
            prims
                .iter()
                .filter(|p| matches!(p, frus_core::Primitive::Path { .. }))
                .count()
        };
        let rest = painted(None, false);
        assert!(fills(&rest).is_empty(), "nothing behind them at rest");
        assert_eq!(strokes(&rest), 3, "three glyphs");

        let close = fills(&painted(Some(CaptionButton::Close), false));
        assert_eq!(close.len(), 1);
        assert_eq!(
            close[0].0,
            Rect::new(92.0, 0.0, 46.0, 40.0),
            "the last cell"
        );
        assert_eq!(close[0].1, Color::rgb(0.77, 0.17, 0.11), "red");

        let minimize = fills(&painted(Some(CaptionButton::Minimize), false));
        assert_eq!(minimize[0].0.x, 0.0, "the first cell");
        assert_ne!(minimize[0].1, close[0].1, "a wash, not red");

        // Maximized, the middle glyph is the restore glyph: two squares, not one.
        let path_len = |prims: Vec<frus_core::Primitive>| -> usize {
            prims
                .iter()
                .filter_map(|p| match p {
                    frus_core::Primitive::Path { path, .. } => Some(path.verbs().len()),
                    _ => None,
                })
                .nth(1)
                .expect("the middle glyph")
        };
        assert!(path_len(painted(None, true)) > path_len(painted(None, false)));
    }
}
