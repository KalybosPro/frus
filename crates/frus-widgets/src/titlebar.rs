//! **The title bar's line**, shared with the system (milestone 640), and **the window's menu
//! bar** on it (milestone 642).
//!
//! On a desktop whose system allows it, a [`WindowMenuBar`] puts the menu bar on the window's
//! title bar line, as a code editor does. The system keeps its own window: its icon still
//! opens the window menu, its three buttons still minimize, maximize and close, and the
//! empty part of the line still moves the window and maximizes it on a double click. Where
//! it can, the system also paints the line — its caption backdrop and its own buttons — and
//! frus draws the window's icon and the menu bar over it (milestone 643); elsewhere frus
//! paints the line, in the caption's colours.
//!
//! The shell says when the line is shared, and what the system keeps on it, through
//! [`MediaQuery::title_bar`](crate::MediaQuery::title_bar).
//!
//! ## The window's, not a page's
//!
//! A desktop application has one window, and its pages take turns inside it. The title bar
//! and the menu bar on it belong to the window: they stay put while a page slides in, and
//! they do not come and go with the pages. So the menu bar goes **above the pages** — around
//! the router, with `frus_shell::FrusApp::builder` — and not in a page's scaffold, where it
//! would leave with the page, slide with the page's transition, and hand the line back to
//! the system on every page that has none.

use frus_core::{Color, Insets, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::container::Container;
use crate::expanded::Expanded;
use crate::interaction::Status;
use crate::media::{Edges, MediaQuery};
use crate::mediascope::MediaScope;
use crate::menubar::MenuBar;
use crate::theme::Theme;
use crate::themed::Themed;
use crate::widget::Widget;
use crate::widgettheme::TitleBarTheme;

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
    /// The system's surface for the line, as it would draw its own caption for the window
    /// now — active or not; `None` where the shell cannot tell (milestone 642).
    pub background: Option<Color>,
    /// The system's words on that caption, likewise.
    pub foreground: Option<Color>,
    /// **The system paints the line** (milestone 643): its backdrop — on Windows 11 the
    /// wallpaper-tinted one — and its three buttons, with their own hover and press, show
    /// through wherever the application leaves the line transparent. The application then
    /// paints neither, unless it was told a background; it paints the window's icon and
    /// its menu bar.
    pub system_paints: bool,
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
        // The line's surface and ink, as its scope resolved them (`WindowMenuBar`).
        let surface = theme
            .widgets
            .menu_bar
            .background
            .unwrap_or(theme.scheme.surface_container);
        let ink = theme
            .widgets
            .menu_bar
            .foreground
            .unwrap_or(theme.scheme.on_surface);
        let glyph = if self.bar.active {
            t.glyph_color.unwrap_or(ink)
        } else {
            t.inactive_glyph_color
                .unwrap_or_else(|| surface.lerp(t.glyph_color.unwrap_or(ink), 0.45))
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
                (false, _, true) => {
                    Some(t.pressed_color.unwrap_or_else(|| surface.lerp(glyph, 0.04)))
                }
                (false, true, _) => {
                    Some(t.hover_color.unwrap_or_else(|| surface.lerp(glyph, 0.08)))
                }
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

/// The row a [`WindowMenuBar`] puts its menu bar in, which says it wants the title bar's
/// line. Under a top intrusion — a tablet's status bar — it starts below it, and paints
/// behind it.
struct TitleBarRow<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    width: f32,
    height: f32,
    top: f32,
}

impl<Msg> Widget<Msg> for TitleBarRow<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: frus_layout::FlexDirection::Row,
            align: frus_layout::Align::Center,
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.top + self.height),
            padding: Insets::new(self.top, 0.0, 0.0, 0.0),
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

/// A menu bar's height off the title bar's line, unless it says otherwise: a desktop's.
const MENU_BAR_HEIGHT: f32 = 30.0;

/// The window's icon on the title bar's line, a side.
const WINDOW_ICON: f32 = 16.0;

/// **The window's menu bar**, above everything the window shows (milestone 642).
///
/// It goes around the application's pages, so that it stays put while they change — with
/// `FrusApp::builder`, which hands it the pages:
///
/// ```ignore
/// FrusApp::router(router).builder(|cx, pages| {
///     let wide = MediaQuery::of().size_class() == SizeClass::Expanded;
///     let bar = wide.then(|| menu_bar(cx)); // `None` on a narrow window
///     Box::new(WindowMenuBar::new(bar, pages))
/// })
/// ```
///
/// On a desktop whose system allows it — Windows — the bar goes **on the title bar's line**,
/// as a code editor's does: the system keeps the window's icon, which opens the window menu,
/// and its three buttons, which minimize, maximize and close the window, and the empty part
/// of the line moves the window and maximizes it on a double click. On Windows 11 the system
/// paints the line — its backdrop and its buttons — and frus the icon, from the
/// application's own, and the bar; where the system cannot, frus paints the line too, and the
/// buttons where the system has them. Elsewhere the bar is the first line under the system's
/// title bar. On the title bar's line it is as tall as
/// the system's line; elsewhere as tall as the bar's own [`height`](MenuBar::height), or a
/// desktop's 30.
///
/// **`child` gets the rest of the window**: below it, [`MediaQuery::of`] is the window less
/// the bar's line — its size shorter by it, its top intrusion consumed — so a
/// [`Scaffold`](crate::Scaffold) built there fits under the bar. That holds for what is built
/// where it sits: the router's pages, a component. A scaffold made before, outside, has
/// already read the whole window.
///
/// **With no bar** — `None`, a narrow window that folds its menus away — `child` gets the
/// whole window and the system gets its title bar back. The tree keeps its shape either
/// way, so the pages below keep their state when the bar comes and goes: keep the
/// `WindowMenuBar` and pass it `None`, rather than leaving it out.
///
/// **Its colours are the system's** on the title bar's line: the caption the desktop would
/// draw for the window — light or dark, the accent colour where the person asked for it,
/// quieter when the window is not the active one. [`background`](Self::background),
/// [`foreground`](Self::foreground) and [`style`](Self::style) say otherwise for this bar, and
/// [`TitleBarTheme`] (`theme.widgets.title_bar`) for every bar; what this bar says outranks
/// the theme, which outranks the system. Off the line, unset, the bar's own
/// [`MenuBarTheme`](crate::MenuBarTheme).
pub struct WindowMenuBar<Msg = crate::callback::Callback> {
    /// The bar's row (an empty box with no bar), then `child`.
    children: Vec<Box<dyn Widget<Msg>>>,
    width: f32,
    height: f32,
    /// The colours this bar was given, read by the row's scope when the walk reaches it — a
    /// setter called after `new` still reaches the row.
    own: std::rc::Rc<std::cell::Cell<TitleBarTheme>>,
}

impl<Msg: Clone + 'static> WindowMenuBar<Msg> {
    /// `bar` on the window's first line, or none, and `child` under it.
    pub fn new(bar: impl Into<Option<MenuBar<Msg>>>, child: impl Widget<Msg> + 'static) -> Self {
        let surface = MediaQuery::of();
        let own = std::rc::Rc::new(std::cell::Cell::new(TitleBarTheme::default()));
        let (row, taken): (Box<dyn Widget<Msg>>, f32) = match bar.into() {
            Some(bar) => {
                let line = surface.title_bar;
                // On the title bar's line the system has the window's top: no intrusion
                // there to start below.
                let top = if line.is_some() {
                    0.0
                } else {
                    surface.padding.top
                };
                let row = title_bar_row(bar, line, surface.size.width, top);
                let taken = row.top + row.height;
                // The line's colours, for everything on it: resolved under the theme the
                // walk brings, so the theme's say is the one in force where the bar is.
                let colours = own.clone();
                let row = Themed::tweak(
                    move |theme: &mut Theme| paint_line(theme, colours.get(), line),
                    row,
                );
                (Box::new(row), taken)
            }
            None => (Box::new(Container::new().height(0.0)), 0.0),
        };
        let content = MediaScope::tweak(
            move |media: &mut MediaQuery| {
                if taken > 0.0 {
                    *media = media.remove_padding(Edges {
                        top: true,
                        ..Edges::NONE
                    });
                    media.size.height = (media.size.height - taken).max(0.0);
                    // The line is this bar's: nothing below is on it.
                    media.title_bar = None;
                }
            },
            child,
        );
        Self {
            children: vec![row, Box::new(Expanded::new(content))],
            width: surface.size.width,
            height: surface.size.height,
            own,
        }
    }

    /// **The line's surface**, active or not, for this bar.
    #[must_use]
    pub fn background(self, color: Color) -> Self {
        self.restyle(|t| t.background = Some(color))
    }

    /// **The words and glyphs on the line**, active or not, for this bar.
    #[must_use]
    pub fn foreground(self, color: Color) -> Self {
        self.restyle(|t| t.foreground = Some(color))
    }

    /// **Every colour of the line** for this bar, the inactive window's included. What it
    /// leaves unset is the theme's, then the system's.
    #[must_use]
    pub fn style(self, style: TitleBarTheme) -> Self {
        self.restyle(|t| *t = style)
    }

    fn restyle(self, change: impl FnOnce(&mut TitleBarTheme)) -> Self {
        let mut style = self.own.get();
        change(&mut style);
        self.own.set(style);
        self
    }
}

/// How far the bar's highlight moves its surface toward its words, on the system's line: a
/// menu bar's open word's.
const HIGHLIGHT: f32 = 0.12;

/// **`ink` at the opacity that reads, over `base`, as `base` moved `strength` of the way
/// to `ink`** (milestone 643).
///
/// Over its own surface the bar's highlight is that colour, opaque. Over the system's
/// backdrop it has to be a wash, and frus blends a wash in linear light, where 12 % of white
/// over a dark caption reads as 40 %: measured on the system's line, the open word's box was
/// `#7C7E7D` on `#1E2120`. So the opacity is the one that, blended in linear light over the
/// system's caption colour, gives the colour the opaque highlight would have.
fn wash(base: Color, ink: Color, strength: f32) -> Color {
    let target = base.lerp(ink, strength).to_linear();
    let (base, linear_ink) = (base.to_linear(), ink.to_linear());
    let mut sum = 0.0;
    let mut channels = 0.0;
    for (target, base, ink) in [
        (target.r, base.r, linear_ink.r),
        (target.g, base.g, linear_ink.g),
        (target.b, base.b, linear_ink.b),
    ] {
        if (ink - base).abs() > 1e-3 {
            sum += (target - base) / (ink - base);
            channels += 1.0;
        }
    }
    let alpha = if channels > 0.0 {
        (sum / channels).clamp(0.0, 1.0)
    } else {
        strength
    };
    ink.with_alpha(ink.a * alpha)
}

/// One of the line's colours for the window as it is: active, or what is said for an
/// inactive window, else the active one's.
fn for_window(active: bool, color: Option<Color>, inactive: Option<Color>) -> Option<Color> {
    if active {
        color
    } else {
        inactive.or(color)
    }
}

/// **Says the line's colours to everything on it**: what the bar was given, then the theme's
/// [`TitleBarTheme`], then the system's caption ([`TitleBar::background`]). The bar's words
/// and highlight read them as the menu bar's surface and ink, and the window's buttons as
/// their glyphs; what the menu bar itself was given still outranks them.
fn paint_line(theme: &mut Theme, own: TitleBarTheme, line: Option<TitleBar>) {
    let active = line.is_none_or(|line| line.active);
    let said = theme.widgets.title_bar;
    let background = for_window(active, own.background, own.inactive_background)
        .or(for_window(
            active,
            said.background,
            said.inactive_background,
        ))
        .or(line.and_then(|line| line.background.filter(|_| !line.system_paints)))
        // Where the system paints the line, nothing of the application's is in its way.
        .or(line
            .filter(|line| line.system_paints)
            .map(|_| Color::TRANSPARENT));
    let foreground = for_window(active, own.foreground, own.inactive_foreground)
        .or(for_window(
            active,
            said.foreground,
            said.inactive_foreground,
        ))
        .or(line.and_then(|line| line.foreground));
    if let Some(color) = background {
        theme.widgets.menu_bar.background = Some(color);
    }
    // Over the system's own backdrop, the bar's highlight is a wash of its words that reads
    // as it would over the system's caption (milestone 643).
    if let Some(line) = line.filter(|line| line.system_paints) {
        let bar = &mut theme.widgets.menu_bar;
        if let (None, Some(base), Some(ink)) = (bar.highlight, line.background, foreground) {
            bar.highlight = Some(wash(base, ink, HIGHLIGHT));
        }
    }
    if let Some(color) = foreground {
        theme.widgets.menu_bar.foreground = Some(color);
        // Already the inactive window's when the window is not active: the glyphs take it
        // as it is, rather than quietened a second time.
        if !active {
            let buttons = &mut theme.widgets.caption_buttons;
            buttons.inactive_glyph_color = buttons.inactive_glyph_color.or(Some(color));
        }
    }
}

/// The bar's row: on the title bar's line, the system's height, with the window's icon at
/// its start and its three buttons where the system has them; elsewhere, the bar alone.
fn title_bar_row<Msg: Clone + 'static>(
    bar: MenuBar<Msg>,
    line: Option<TitleBar>,
    width: f32,
    top: f32,
) -> TitleBarRow<Msg> {
    let height = line
        .map(|line| line.height)
        .or(bar.given_height())
        .unwrap_or(MENU_BAR_HEIGHT);
    let mut cells: Vec<Box<dyn Widget<Msg>>> = Vec::new();
    if let Some(line) = line {
        if line.leading > 0.0 {
            cells.push(Box::new(
                Container::new().width(line.leading).height(height),
            ));
        }
        if let Some(png) = line.icon {
            cells.push(Box::new(
                Container::new()
                    .padding_each(0.0, 6.0, 0.0, 10.0)
                    .child(WindowIcon::new(png, WINDOW_ICON)),
            ));
        }
    }
    cells.push(Box::new(bar.height(height)));
    // The empty part of the line: nothing of the application's, so the system's to move the
    // window by.
    cells.push(Box::new(Container::new().flex(1.0).height(height)));
    if let Some(line) = line {
        if let Some(buttons) = line.buttons {
            if line.system_paints {
                // The system's own buttons show there: the room is kept, and left clear.
                cells.push(Box::new(
                    Container::new().width(buttons.width).height(height),
                ));
            } else {
                cells.push(Box::new(CaptionButtons::new(line, buttons.width)));
            }
        }
    }
    TitleBarRow {
        children: cells,
        width,
        height,
        top,
    }
}

impl<Msg> Widget<Msg> for WindowMenuBar<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: frus_layout::FlexDirection::Column,
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn debug_name(&self) -> &'static str {
        "WindowMenuBar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_ui, BuildContext, Callback, Component, MenuBar, MenuPath, Runtime, Scaffold, Size,
        SubmenuButton, UseState,
    };
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    type Msg = Callback;

    const SIZE: Size = Size::new(1000.0, 700.0);
    /// The frus logo, a real PNG.
    const ICON: &[u8] = include_bytes!("../../frus-shell/assets/icon.png");

    fn bar() -> MenuBar<Msg> {
        MenuBar::new(&MenuPath::closed(), |_| Callback::new(|| {})).menu(SubmenuButton::new("File"))
    }

    /// What a page read of the window when it was built.
    type Seen = Rc<Cell<Option<MediaQuery>>>;

    /// A page as the router makes one: a component, built where it sits, whose scaffold has
    /// a title, a footer and a floating button.
    fn page(seen: &Seen) -> Component {
        let seen = seen.clone();
        Component::stateless(move |_: &BuildContext| {
            seen.set(Some(MediaQuery::of()));
            Scaffold::new()
                .app_bar(crate::Text::new("Title"))
                .persistent_footer(crate::Text::new("Foot"))
                .fab(crate::Text::new("Fab"))
                .build()
        })
    }

    /// One frame of the shell's, less the drawing, on `surface`: `tree` is made under it, as
    /// the shell calls `view`, then built and laid out.
    fn frame(
        surface: MediaQuery,
        runtime: &Runtime,
        tree: impl FnOnce() -> Box<dyn Widget<Msg>>,
    ) -> crate::Ui<Msg> {
        frame_themed(surface, runtime, &windows(), tree)
    }

    fn windows() -> Theme {
        Theme::default().with_platform(frus_core::TargetPlatform::Windows)
    }

    /// [`frame`], under `theme`.
    fn frame_themed(
        surface: MediaQuery,
        runtime: &Runtime,
        theme: &Theme,
        tree: impl FnOnce() -> Box<dyn Widget<Msg>>,
    ) -> crate::Ui<Msg> {
        let theme = theme.clone();
        surface.scope(|| {
            runtime.states.begin_build();
            let tree = tree();
            crate::build_deferred(tree.as_ref(), &theme, runtime);
            let ui = build_ui(tree.as_ref(), SIZE, runtime, &theme);
            runtime.states.end_frame();
            ui
        })
    }

    /// The window's menu bar around a page, on `surface`.
    fn window(surface: MediaQuery, seen: &Seen) -> crate::Ui<Msg> {
        let page = page(seen);
        frame(surface, &Runtime::default(), || {
            Box::new(WindowMenuBar::new(bar(), page))
        })
    }

    fn text_at(ui: &crate::Ui<Msg>, label: &str) -> Rect {
        find_text(ui, label).unwrap_or_else(|| panic!("{label:?} is painted"))
    }

    fn find_text(ui: &crate::Ui<Msg>, label: &str) -> Option<Rect> {
        ui.scene().primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Text { text, .. } if text == label => Some(p.bounds()),
            _ => None,
        })
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
            background: None,
            foreground: None,
            system_paints: false,
        }
    }

    /// **The window's menu bar is its first line, and the page gets the rest**: off the
    /// title bar's line a desktop's 30 px; the page is built for the window less them, so
    /// its scaffold fits under the bar — its footer and floating button still at the
    /// window's bottom — and nothing below is on the line.
    #[test]
    fn the_menu_bar_is_the_window_s_first_line_and_the_page_gets_the_rest() {
        let seen = Seen::default();
        let ui = window(MediaQuery::new(SIZE), &seen);
        let file = text_at(&ui, "File");
        let title = text_at(&ui, "Title");
        assert!(file.y + file.height <= 30.0, "in the first 30 px: {file:?}");
        assert!(title.y >= 30.0, "the app bar under it: {title:?}");
        assert!(ui.wants_title_bar(), "and it asks for the line");
        let page = seen.get().expect("the page was built");
        assert_eq!(
            page.size,
            Size::new(SIZE.width, SIZE.height - 30.0),
            "the page is built for the window less the bar"
        );
        assert_eq!(page.title_bar, None, "the line is the bar's");
        for part in ["Foot", "Fab"] {
            let r = text_at(&ui, part);
            assert!(
                r.y + r.height <= SIZE.height && r.y + r.height > SIZE.height - 80.0,
                "{part} is at the window's bottom: {r:?}"
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
        let seen = Seen::default();
        let surface = MediaQuery::new(SIZE).with_title_bar(Some(line(Some(buttons), Some(ICON))));
        let ui = window(surface, &seen);
        let title = text_at(&ui, "Title");
        assert!(title.y >= 40.0, "the app bar under the line: {title:?}");
        let page = seen.get().expect("the page was built");
        assert_eq!(page.size.height, SIZE.height - 40.0, "the line taken off");
        assert_eq!(page.title_bar, None, "the line is the bar's");
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

    /// **Under a top intrusion** — a tablet's status bar — the bar starts below it, and the
    /// page is told the intrusion is dealt with.
    #[test]
    fn under_a_status_bar_the_menu_bar_starts_below_it() {
        let seen = Seen::default();
        let surface = MediaQuery::new(SIZE).with_insets(frus_core::WindowInsets {
            padding: frus_core::Insets::new(24.0, 0.0, 0.0, 0.0),
            ..Default::default()
        });
        let ui = window(surface, &seen);
        let file = text_at(&ui, "File");
        assert!(file.y >= 24.0, "below the status bar: {file:?}");
        let page = seen.get().expect("the page was built");
        assert_eq!(page.size.height, SIZE.height - 24.0 - 30.0);
        assert_eq!(page.padding.top, 0.0, "the status bar is dealt with");
        let title = text_at(&ui, "Title");
        assert!(title.y >= 54.0, "the app bar under both: {title:?}");
    }

    /// **With no bar** — a narrow window — the page has the whole window, the system gets
    /// its line back, and **the page keeps its state** while the bar comes and goes: the
    /// tree keeps its shape.
    #[test]
    fn with_no_bar_the_page_has_the_window_and_keeps_its_state() {
        let runtime = Runtime::default();
        let count: Rc<RefCell<Option<UseState<i32>>>> = Rc::default();
        let tree = |with_bar: bool| {
            let count = count.clone();
            move || -> Box<dyn Widget<Msg>> {
                let page = Component::stateless(move |cx: &BuildContext| -> Box<dyn Widget> {
                    let n = cx.use_state(|| 0);
                    *count.borrow_mut() = Some(n.clone());
                    Box::new(crate::Text::new(format!("count {}", n.get())))
                });
                Box::new(WindowMenuBar::new(with_bar.then(bar), page))
            }
        };
        let ui = frame(MediaQuery::new(SIZE), &runtime, tree(true));
        assert!(ui.wants_title_bar());
        count.borrow().as_ref().expect("built").set(5);

        let ui = frame(MediaQuery::new(SIZE), &runtime, tree(false));
        assert!(find_text(&ui, "File").is_none(), "no bar");
        assert!(!ui.wants_title_bar(), "the line is the system's again");
        let text = text_at(&ui, "count 5");
        assert!(text.y < 30.0, "the page from the window's top: {text:?}");

        let ui = frame(MediaQuery::new(SIZE), &runtime, tree(true));
        assert!(find_text(&ui, "count 5").is_some(), "kept, both ways");
    }

    /// The system's caption, as the shell reports it for an active dark window.
    const CAPTION: Color = Color::rgb(0.13, 0.12, 0.16);
    const CAPTION_INK: Color = Color::WHITE;

    /// The line as the shell reports it: the system's buttons, and its caption's colours.
    fn system_line(active: bool, background: Color, foreground: Color) -> TitleBar {
        TitleBar {
            active,
            background: Some(background),
            foreground: Some(foreground),
            ..line(Some(Rect::new(862.0, 0.0, 138.0, 40.0)), None)
        }
    }

    /// The window's menu bar around an empty page, on `surface`, under `theme`; `dress`
    /// gives the bar its colours.
    fn dressed(
        surface: MediaQuery,
        theme: &Theme,
        dress: impl FnOnce(WindowMenuBar<Msg>) -> WindowMenuBar<Msg>,
    ) -> crate::Ui<Msg> {
        frame_themed(surface, &Runtime::default(), theme, || {
            Box::new(dress(WindowMenuBar::new(bar(), crate::Text::new("page"))))
        })
    }

    /// The line's surface, the bar's words and the window's glyphs, as painted.
    fn colours(ui: &crate::Ui<Msg>) -> (Color, Color, Vec<Color>) {
        let prims = ui.scene().primitives();
        let surface = prims
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. }
                    if rect.x == 0.0 && rect.y == 0.0 && rect.width == SIZE.width =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .expect("the line's surface");
        let words = prims
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { text, color, .. } if text == "File" => Some(*color),
                _ => None,
            })
            .expect("the bar's words");
        let glyphs = prims
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Path {
                    stroke: Some(stroke),
                    ..
                } => Some(stroke.color),
                _ => None,
            })
            .collect();
        (surface, words, glyphs)
    }

    /// **The system's caption is the line's by default**: its surface, and its ink on the
    /// words and the window's glyphs — for an inactive window the inactive caption's, taken
    /// as it is and not quietened a second time.
    #[test]
    fn the_line_takes_the_system_s_caption_colours() {
        let surface =
            MediaQuery::new(SIZE).with_title_bar(Some(system_line(true, CAPTION, CAPTION_INK)));
        let (fill, words, glyphs) = colours(&dressed(surface, &windows(), |w| w));
        assert_eq!(fill, CAPTION, "the system's surface");
        assert_eq!(words, CAPTION_INK, "its ink on the words");
        assert_eq!(glyphs, vec![CAPTION_INK; 3], "and on the glyphs");

        let (grey, quiet) = (
            Color::rgb(0.125, 0.125, 0.125),
            Color::rgb(0.47, 0.47, 0.47),
        );
        let surface = MediaQuery::new(SIZE).with_title_bar(Some(system_line(false, grey, quiet)));
        let (fill, words, glyphs) = colours(&dressed(surface, &windows(), |w| w));
        assert_eq!(fill, grey, "the inactive caption's surface");
        assert_eq!(words, quiet, "its ink");
        assert_eq!(glyphs, vec![quiet; 3], "the glyphs as the system has them");
    }

    /// **The theme outranks the system, and the bar outranks the theme** — for an active
    /// window and, with what is said for one, an inactive window.
    #[test]
    fn the_bar_and_the_theme_say_otherwise() {
        let red = Color::rgb(1.0, 0.0, 0.0);
        let green = Color::rgb(0.0, 1.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let yellow = Color::rgb(1.0, 1.0, 0.0);
        let mut themed = windows();
        themed.widgets.title_bar = TitleBarTheme {
            background: Some(red),
            foreground: Some(green),
            inactive_background: None,
            inactive_foreground: Some(blue),
        };
        let on = |active| {
            MediaQuery::new(SIZE).with_title_bar(Some(system_line(active, CAPTION, CAPTION_INK)))
        };

        let (fill, words, glyphs) = colours(&dressed(on(true), &themed, |w| w));
        assert_eq!(
            (fill, words),
            (red, green),
            "the theme's, over the system's"
        );
        assert_eq!(glyphs, vec![green; 3]);

        let (fill, words, glyphs) = colours(&dressed(on(false), &themed, |w| w));
        assert_eq!(fill, red, "no inactive surface said: the active one's");
        assert_eq!(words, blue, "the inactive window's ink");
        assert_eq!(glyphs, vec![blue; 3]);

        let (fill, words, glyphs) = colours(&dressed(on(true), &themed, |w| {
            w.background(blue).foreground(yellow)
        }));
        assert_eq!((fill, words), (blue, yellow), "the bar's, over the theme's");
        assert_eq!(glyphs, vec![yellow; 3]);
    }

    /// **Where the system paints the line**, the application paints neither its surface nor
    /// the window's buttons — the system's show through — and the room the buttons need is
    /// kept; the words take the system's ink, and their highlight is a wash that reads as it
    /// would over the system's caption.
    #[test]
    fn where_the_system_paints_the_line_the_application_leaves_it_clear() {
        let painted = |system_paints| {
            let line = TitleBar {
                system_paints,
                ..system_line(true, CAPTION, CAPTION_INK)
            };
            dressed(
                MediaQuery::new(SIZE).with_title_bar(Some(line)),
                &windows(),
                |w| w,
            )
        };
        let ui = painted(true);
        let opaque_line = ui.scene().primitives().iter().any(|p| {
            matches!(p, frus_core::Primitive::Rect { rect, color, .. }
                if rect.y < 40.0 && rect.width >= 100.0 && color.a > 0.0)
        });
        assert!(!opaque_line, "nothing of the application's under the line");
        let ink = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text { text, color, .. } if text == "File" => Some(*color),
                _ => None,
            })
            .expect("the bar's words");
        assert_eq!(ink, CAPTION_INK, "the system's ink on the words");
        let strokes = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    frus_core::Primitive::Path {
                        stroke: Some(_),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(strokes, 0, "the system's own buttons, not painted ones");
        let file = text_at(&ui, "File");
        let painted_file = text_at(&painted(false), "File");
        assert_eq!(file, painted_file, "the bar where it was");

        // The open word's highlight is the wash that reads as on the system's caption.
        let line = TitleBar {
            system_paints: true,
            ..system_line(true, CAPTION, CAPTION_INK)
        };
        let open = frame_themed(
            MediaQuery::new(SIZE).with_title_bar(Some(line)),
            &Runtime::default(),
            &windows(),
            || {
                let bar = MenuBar::new(&MenuPath::from_indices([0]), |_| Callback::new(|| {}))
                    .menu(SubmenuButton::new("File"));
                Box::new(WindowMenuBar::new(bar, crate::Text::new("page")))
            },
        );
        let file = text_at(&open, "File");
        let lit = open
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Rect { rect, color, .. }
                    if rect.contains(frus_core::Point::new(file.x + 1.0, file.y + 4.0))
                        && color.a > 0.0 =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .expect("the open word's highlight");
        assert_eq!(lit, wash(CAPTION, CAPTION_INK, HIGHLIGHT));
    }

    /// **The wash reads as the opaque highlight would**: blended in linear light over the
    /// system's caption, it gives the caption moved 12 % toward the words — on a dark
    /// caption, where white at 12 % would read far stronger, and on a light one, where black
    /// needs more than 12 % to read as much.
    #[test]
    fn the_highlight_on_the_system_s_line_reads_as_on_its_caption() {
        for (base, ink, lighter) in [
            (Color::rgb8(0x20, 0x20, 0x20), Color::WHITE, true),
            (Color::rgb8(0xF3, 0xF3, 0xF3), Color::BLACK, false),
        ] {
            let washed = wash(base, ink, HIGHLIGHT);
            assert!(washed.a > 0.0 && washed.a < 1.0, "a wash: {washed:?}");
            assert_eq!(washed.a < HIGHLIGHT, lighter, "{base:?}: {washed:?}");
            let (b, i) = (base.to_linear(), ink.to_linear());
            let blended = b.r + (i.r - b.r) * washed.a;
            let wanted = base.lerp(ink, HIGHLIGHT).to_linear().r;
            assert!(
                (blended - wanted).abs() < 1e-4,
                "{base:?}: {blended} for {wanted}"
            );
        }
    }

    /// **Off the title bar's line, with nothing said**, the row is the menu bar's own:
    /// `surface_container` and `on_surface`, as before.
    #[test]
    fn off_the_line_the_row_is_the_menu_bar_s_own() {
        let theme = windows();
        let (fill, words, glyphs) = colours(&dressed(MediaQuery::new(SIZE), &theme, |w| w));
        assert_eq!(fill, theme.scheme.surface_container);
        assert_eq!(words, theme.scheme.on_surface);
        assert!(glyphs.is_empty(), "no buttons of the system's");
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
