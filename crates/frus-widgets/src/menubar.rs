//! [`MenuBar`], [`SubmenuButton`] and [`MenuPath`]: the menus along the top of a desktop
//! window, and the submenus inside them (milestone 603).
//!
//! # Where "which menu is open" lives
//!
//! In the application, as a [`MenuPath`]: one field, and one message that carries the new
//! path. That is how every other menu here works — a [`PopupMenuButton`](crate::PopupMenuButton)
//! and a [`MenuAnchor`](crate::MenuAnchor) are told whether they are open — and it keeps
//! `update` the one place the interface's state changes, testable without a window. A menu
//! bar's state is a path rather than a flag, which is the whole difference, so the path is a
//! type of its own with the moves a menu makes on it.
//!
//! An application built from components keeps the path in a hook, as it would a flag.

use std::rc::Rc;

use frus_core::{Color, Insets, Point, Rect, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::menu::{menu_panel, MenuItem, PanelKind, PanelStyle, RowKeys, RowLook};
use crate::portal::{OverlayPortal, Placement};
use crate::theme::Theme;
use crate::widget::Widget;

/// **Which menu of a [`MenuBar`] is open, and how deep**: the index of the open menu on the
/// bar, then of the open submenu in it, and so on. Empty when every menu is closed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MenuPath(Vec<usize>);

impl MenuPath {
    /// Every menu closed.
    pub fn closed() -> Self {
        Self(Vec::new())
    }

    /// The path of indices, from the bar down.
    pub fn from_indices(indices: impl Into<Vec<usize>>) -> Self {
        Self(indices.into())
    }

    /// Whether any menu is open.
    pub fn is_open(&self) -> bool {
        !self.0.is_empty()
    }

    /// How many menus are open: `1` for a menu off the bar, `2` with a submenu in it.
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// Which entry is open at `level` — `0` the bar.
    pub fn at(&self, level: usize) -> Option<usize> {
        self.0.get(level).copied()
    }

    /// The indices, from the bar down.
    pub fn indices(&self) -> &[usize] {
        &self.0
    }

    /// The path with entry `index` open at `level`, and whatever was open below it closed.
    pub fn opened(&self, level: usize, index: usize) -> Self {
        let mut path = self.0[..level.min(self.0.len())].to_vec();
        path.push(index);
        Self(path)
    }

    /// The path with everything from `level` down closed: `truncated(0)` closes the lot.
    pub fn truncated(&self, level: usize) -> Self {
        Self(self.0[..level.min(self.0.len())].to_vec())
    }

    /// One level fewer open: what Escape does.
    pub fn up(&self) -> Self {
        self.truncated(self.0.len().saturating_sub(1))
    }
}

/// **What a caller says about a menu bar**, every word optional: what is unset is the
/// theme's ([`MenuBarTheme`](crate::MenuBarTheme)), then the platform's (milestone 636).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct BarStyle {
    height: Option<f32>,
    background: Option<Color>,
    padding: Option<Insets>,
    item_padding: Option<Insets>,
    text_style: Option<TextStyle>,
    foreground: Option<Color>,
    highlight: Option<Color>,
    item_radius: Option<f32>,
    item_inset: Option<f32>,
    menus: crate::MenuTheme,
}

/// A bar's look, resolved for one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BarLook {
    height: f32,
    background: Color,
    padding: Insets,
    item_padding: Insets,
    text_style: TextStyle,
    foreground: Color,
    highlight: Option<Color>,
    item_radius: f32,
    item_inset: f32,
}

impl BarStyle {
    /// The caller's word, then the theme's, then a desktop's bar on a desktop theme — 30 px,
    /// 13 px words 8 px either side, a 4 px rounded highlight 3 px inside the bar — or a
    /// phone's on a phone's theme, 48 px for a finger.
    fn resolve(&self, theme: &Theme) -> BarLook {
        let t = &theme.widgets.menu_bar;
        let desktop = theme.platform.is_desktop();
        BarLook {
            height: self.height.or(t.height).unwrap_or(if desktop {
                30.0
            } else {
                crate::theme::MIN_TAP_TARGET
            }),
            background: self
                .background
                .or(t.background)
                .unwrap_or(theme.scheme.surface_container),
            padding: self.padding.or(t.padding).unwrap_or(if desktop {
                Insets::new(0.0, 4.0, 0.0, 4.0)
            } else {
                Insets::ZERO
            }),
            item_padding: self.item_padding.or(t.item_padding).unwrap_or(if desktop {
                Insets::new(0.0, 8.0, 0.0, 8.0)
            } else {
                Insets::new(0.0, BAR_PAD_X, 0.0, BAR_PAD_X)
            }),
            text_style: self.text_style.or(t.text_style).unwrap_or(if desktop {
                TextStyle::new(13.0)
            } else {
                theme.text.label_large
            }),
            foreground: self
                .foreground
                .or(t.foreground)
                .unwrap_or(theme.scheme.on_surface),
            highlight: self.highlight.or(t.highlight),
            item_radius: self.item_radius.or(t.item_radius).unwrap_or(if desktop {
                4.0
            } else {
                0.0
            }),
            item_inset: self
                .item_inset
                .or(t.item_inset)
                .unwrap_or(if desktop { 3.0 } else { 0.0 }),
        }
    }
}

/// One entry of a menu: an action, a rule, or a submenu.
enum Entry<Msg> {
    Item(MenuItem<Msg>),
    Submenu(SubmenuButton<Msg>),
}

/// **A menu**: on a [`MenuBar`], the word along the top and what drops from it; inside
/// another menu, a row that opens a menu beside it.
///
/// Its rows are [`MenuItem`]s: actions, ticked rows, rows with a shortcut shown, rules.
///
/// ```
/// use frus_widgets::{MenuItem, SubmenuButton};
///
/// #[derive(Clone)]
/// enum Msg { New, Open, Recent(usize) }
///
/// let file: SubmenuButton<Msg> = SubmenuButton::new("File")
///     .item(MenuItem::new("New", Msg::New).shortcut("Ctrl+N"))
///     .item(MenuItem::new("Open…", Msg::Open))
///     .divider()
///     .submenu(
///         SubmenuButton::new("Open recent")
///             .item(MenuItem::new("notes.txt", Msg::Recent(0)))
///             .item(MenuItem::new("todo.md", Msg::Recent(1))),
///     );
/// ```
pub struct SubmenuButton<Msg = crate::callback::Callback> {
    label: String,
    entries: Vec<Entry<Msg>>,
    enabled: bool,
}

impl<Msg> SubmenuButton<Msg> {
    /// A menu called `label`, with nothing in it yet.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            entries: Vec::new(),
            enabled: true,
        }
    }

    /// Adds a row.
    pub fn item(mut self, item: MenuItem<Msg>) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// Adds a row that opens `submenu` beside this menu.
    pub fn submenu(mut self, submenu: SubmenuButton<Msg>) -> Self {
        self.entries.push(Entry::Submenu(submenu));
        self
    }

    /// Adds the rule between two groups.
    pub fn divider(mut self) -> Self {
        self.entries.push(Entry::Item(MenuItem::divider()));
        self
    }

    /// Whether the menu can be opened. A disabled one is still shown, greyed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// The message a path is turned into.
type OnPath<Msg> = Rc<dyn Fn(MenuPath) -> Msg>;

/// **The menus along the top of a window**: File, Edit, View.
///
/// The application holds which one is open as a [`MenuPath`] and is sent the new path
/// whenever it should change:
///
/// - a press on a menu's word opens it, and a second press closes it;
/// - once one is open, moving the pointer onto another word opens that one instead, with no
///   press;
/// - moving onto a row that opens a submenu opens it, and moving onto any other row of that
///   menu closes it again;
/// - a press outside every open menu closes them all.
///
/// A row's own message is sent when it is pressed. **Close the menus when you handle it**
/// (`self.menu = MenuPath::closed()`), as a [`PopupMenuButton`](crate::PopupMenuButton)'s
/// rows are closed: a press is one message, and it is the row's.
///
/// ```
/// use frus_widgets::{MenuBar, MenuItem, MenuPath, SubmenuButton};
///
/// #[derive(Clone)]
/// enum Msg { Menu(MenuPath), Quit, Wrap }
///
/// let path = MenuPath::closed();
/// let bar = MenuBar::new(&path, Msg::Menu)
///     .menu(SubmenuButton::new("File").item(MenuItem::new("Quit", Msg::Quit)))
///     .menu(SubmenuButton::new("View").item(MenuItem::checked("Word wrap", true, Msg::Wrap)));
/// # let _: MenuBar<Msg> = bar;
/// ```
pub struct MenuBar<Msg = crate::callback::Callback> {
    path: MenuPath,
    on_path: OnPath<Msg>,
    menus: Vec<SubmenuButton<Msg>>,
    style: BarStyle,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> MenuBar<Msg> {
    /// A bar with `path` open, sending `on_path` the path it should have next.
    pub fn new(path: &MenuPath, on_path: impl Fn(MenuPath) -> Msg + 'static) -> Self {
        Self {
            path: path.clone(),
            on_path: Rc::new(on_path),
            menus: Vec::new(),
            style: BarStyle::default(),
            children: Vec::new(),
        }
    }

    /// Builds the words again, so that a style said after the menus reaches them.
    fn rebuilt(mut self) -> Self {
        self.children = (0..self.menus.len()).map(|i| self.word(i)).collect();
        self
    }

    /// The height the caller gave the bar, if any.
    pub(crate) fn given_height(&self) -> Option<f32> {
        self.style.height
    }

    /// **How tall the bar is.** Unset, the theme's, then 30 on a desktop and 48 on a phone.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.style.height = Some(height);
        self.rebuilt()
    }

    /// **The bar's surface.** Unset, the theme's, then `surface_container`.
    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self.rebuilt()
    }

    /// **The room at the bar's ends**, around the words.
    #[must_use]
    pub fn padding(mut self, padding: Insets) -> Self {
        self.style.padding = Some(padding);
        self.rebuilt()
    }

    /// **The room either side of a word.**
    #[must_use]
    pub fn item_padding(mut self, padding: Insets) -> Self {
        self.style.item_padding = Some(padding);
        self.rebuilt()
    }

    /// **The words' type.** Unset, the theme's, then 13 px on a desktop and `label_large` on
    /// a phone.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.style.text_style = Some(style);
        self.rebuilt()
    }

    /// **The words' colour.** Unset, the theme's, then `on_surface`.
    #[must_use]
    pub fn foreground_color(mut self, color: Color) -> Self {
        self.style.foreground = Some(color);
        self.rebuilt()
    }

    /// **The highlight** behind the word under the pointer and the word whose menu is open.
    /// Unset, the theme's, then `on_surface` over the bar.
    #[must_use]
    pub fn highlight_color(mut self, color: Color) -> Self {
        self.style.highlight = Some(color);
        self.rebuilt()
    }

    /// **The highlight's corners.** Unset, the theme's, then 4 on a desktop.
    #[must_use]
    pub fn item_radius(mut self, radius: f32) -> Self {
        self.style.item_radius = Some(radius);
        self.rebuilt()
    }

    /// **The room the highlight keeps above and below it**, inside the bar.
    #[must_use]
    pub fn item_inset(mut self, inset: f32) -> Self {
        self.style.item_inset = Some(inset);
        self.rebuilt()
    }

    /// **The menus the bar opens**: their rows, their type, their panel. What it says
    /// outranks the theme's bar menus, a desktop's menus and the application's
    /// [`MenuTheme`](crate::MenuTheme).
    #[must_use]
    pub fn menu_style(mut self, menus: crate::MenuTheme) -> Self {
        self.style.menus = menus;
        self.rebuilt()
    }

    /// Adds a menu to the bar.
    pub fn menu(mut self, menu: SubmenuButton<Msg>) -> Self {
        self.menus.push(menu);
        // Built again whole: the arrows on each word and row name the menus beside it, and
        // a menu added last is beside the first.
        self.children = (0..self.menus.len()).map(|i| self.word(i)).collect();
        self
    }

    /// The enabled menu `step` places along from `index`, round the ends; `None` when
    /// there is no other.
    fn beside(&self, index: usize, step: isize) -> Option<usize> {
        let count = self.menus.len() as isize;
        (1..count)
            .map(|k| (index as isize + step * k).rem_euclid(count) as usize)
            .find(|&i| self.menus[i].enabled)
    }

    /// The word for menu `index`, with its menu floating under it when it is open.
    fn word(&self, index: usize) -> Box<dyn Widget<Msg>> {
        let menu = &self.menus[index];
        let on_path = &self.on_path;
        let open = menu.enabled && self.path.at(0) == Some(index);
        // A press opens the menu. While it is open, the word is under the press-outside
        // that closes it, so a second press on it closes it, as a bar's does.
        let press = self.path.opened(0, index);
        // Once a menu is open, the pointer moving onto another word opens that one.
        let hover = (menu.enabled && self.path.is_open() && !open)
            .then(|| on_path(self.path.opened(0, index)));
        // And so do the arrows, from a word with the focus (milestone 605).
        let arrow = |step| {
            self.path
                .is_open()
                .then(|| self.beside(index, step))
                .flatten()
                .map(|i| on_path(self.path.opened(0, i)))
        };
        let word = on_enter(
            Box::new(BarButton {
                label: menu.label.clone(),
                style: self.style,
                open,
                enabled: menu.enabled,
                message: on_path(press),
                left: arrow(-1),
                right: arrow(1),
            }),
            hover,
        );
        // In a portal open or shut: a portal takes a place in the tree, and one that came and
        // went with the menu moved the word under it, so that the focus could not find its
        // way back to the word when its menu closed (milestone 608).
        let portal = OverlayPortal::new_boxed(word);
        if !open {
            return Box::new(portal);
        }
        // Across the bar from inside a menu: the menus either side of this one.
        let across = Across {
            previous: self
                .beside(index, -1)
                .map(|i| on_path(MenuPath::closed().opened(0, i))),
            next: self
                .beside(index, 1)
                .map(|i| on_path(MenuPath::closed().opened(0, i))),
        };
        let panel = panel(
            &menu.entries,
            1,
            &self.path,
            on_path,
            &across,
            &self.style.menus,
        );
        Box::new(
            portal
                .overlay_boxed(panel, Placement::Below)
                .dismiss(on_path(MenuPath::closed())),
        )
    }
}

/// What the left and right arrows do when they leave a menu for the bar.
struct Across<Msg> {
    previous: Option<Msg>,
    next: Option<Msg>,
}

/// The panel of the menu open at `level`, with the submenu open in it, if any.
fn panel<Msg: Clone + 'static>(
    entries: &[Entry<Msg>],
    level: usize,
    path: &MenuPath,
    on_path: &OnPath<Msg>,
    across: &Across<Msg>,
    menus: &crate::MenuTheme,
) -> Box<dyn Widget<Msg>> {
    // The first row that can be used takes the focus when the menu opens, so the arrows
    // work in it from there (milestone 605).
    let first = entries.iter().position(|entry| match entry {
        Entry::Item(item) => item.is_action(),
        Entry::Submenu(sub) => sub.enabled,
    });
    // Left leaves a submenu for the row that opened it, and a menu off the bar for the
    // menu before it.
    let left = if level > 1 {
        Some(on_path(path.truncated(level - 1)))
    } else {
        across.previous.clone()
    };
    // The rows as the popup menu builds them: a submenu's is a row that opens it.
    let items: Vec<MenuItem<Msg>> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let mut row = match entry {
                Entry::Item(item) => item.clone_row(),
                Entry::Submenu(sub) => {
                    let mut row =
                        MenuItem::new(sub.label.clone(), on_path(path.opened(level, index)))
                            .enabled(sub.enabled);
                    row.submenu = true;
                    row.expanded = sub.enabled && path.at(level) == Some(index);
                    row
                }
            };
            // Right opens a submenu from its row, and goes on to the next menu from any
            // other row.
            let right = match entry {
                Entry::Submenu(sub) if sub.enabled => Some(on_path(path.opened(level, index))),
                _ => across.next.clone(),
            };
            row.keys = RowKeys {
                left: left.clone(),
                right,
                // Escape closes this menu, and the ones open from it, and no more.
                escape: Some(on_path(path.truncated(level - 1))),
                autofocus: Some(index) == first,
            };
            row
        })
        .collect();
    let decorate = |index: usize, row: Box<dyn Widget<Msg>>| -> Box<dyn Widget<Msg>> {
        match &entries[index] {
            Entry::Submenu(sub) if sub.enabled => {
                let open = path.at(level) == Some(index);
                let hover = (!open).then(|| on_path(path.opened(level, index)));
                // In a portal open or shut, as the word on the bar is.
                let portal = OverlayPortal::new_boxed(on_enter(row, hover));
                if open {
                    let inner = panel(&sub.entries, level + 1, path, on_path, across, menus);
                    Box::new(portal.overlay_boxed(inner, Placement::Beside))
                } else {
                    Box::new(portal)
                }
            }
            // Any other row of this menu closes a submenu open beside it.
            _ => {
                let hover = (path.depth() > level).then(|| on_path(path.truncated(level)));
                on_enter(row, hover)
            }
        }
    };
    menu_panel(
        &items,
        &RowLook {
            // What the caller said about the bar's menus; the theme's bar menus, a
            // desktop's and the application's menus answer the rest, at layout.
            look: PanelStyle {
                background: menus.background,
                shape: menus
                    .shape
                    .or(menus.radius.map(frus_core::ShapeBorder::rounded)),
                elevation: menus.elevation,
                shadow_color: menus.shadow_color,
                padding: menus.padding,
                border: menus.border,
            },
            text_style: menus.text_style,
            item_padding: menus.item_padding,
            item_height: menus.item_height,
            divider_height: menus.divider_height,
            enabled: true,
            kind: PanelKind::Bar,
        },
        &decorate,
    )
}

/// `row`, sending `hover` when the pointer comes onto it.
///
/// A region may come and go with the menus: it is transparent, and takes no place in the
/// tree of its own. A portal does, which is why the word and the rows that open submenus
/// are in theirs open or shut (milestone 608).
fn on_enter<Msg: Clone + 'static>(
    row: Box<dyn Widget<Msg>>,
    hover: Option<Msg>,
) -> Box<dyn Widget<Msg>> {
    match hover {
        Some(message) => Box::new(crate::MouseRegion::new(row).on_enter(move |_| message.clone())),
        None => row,
    }
}

/// A menu's word on the bar.
struct BarButton<Msg> {
    label: String,
    style: BarStyle,
    open: bool,
    enabled: bool,
    message: Msg,
    /// What the arrows send from this word: the menu beside it, while one is open.
    left: Option<Msg>,
    right: Option<Msg>,
}

/// Room either side of a word on the bar.
const BAR_PAD_X: f32 = 12.0;

impl<Msg: Clone> Widget<Msg> for BarButton<Msg> {
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
        let look = self.style.resolve(theme);
        // The highlight: under the pointer it fades in, and the word whose menu is open
        // keeps it, stronger, so the eye can find what the panel belongs to.
        let hover = if self.enabled {
            status
                .hover_progress
                .max(status.press_progress)
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let amount = if self.open { 1.0 } else { hover };
        if amount > 0.0 {
            let colour = match look.highlight {
                Some(c) => c.with_alpha(c.a * amount),
                None => {
                    let strength = if self.open { 0.12 } else { 0.08 * amount };
                    if look.background.a < 1.0 {
                        // A bar that lets what is behind it show — the system's own title
                        // bar under it (milestone 643): a wash of the words over it, since
                        // there is no surface of the bar's own to lean.
                        look.foreground.with_alpha(look.foreground.a * strength)
                    } else {
                        // Toward the words: on a bar of the system's accent with white
                        // words, a highlight toward `on_surface` would darken it
                        // (milestone 642).
                        look.background.lerp(look.foreground, strength)
                    }
                }
            };
            let inset = look.item_inset.min(bounds.height * 0.5);
            scene.draw_rect(
                Rect::new(
                    bounds.x,
                    bounds.y + inset,
                    bounds.width,
                    bounds.height - 2.0 * inset,
                ),
                colour.fade(o),
                look.item_radius,
                0.0,
                frus_core::Color::TRANSPARENT,
            );
        }
        let style = look.text_style.resolved();
        let ink = if self.enabled {
            look.foreground
        } else {
            crate::disabled::disabled_content(theme)
        };
        let y = bounds.y + (bounds.height - style.line_height()) * 0.5;
        scene.text(
            Point::new(bounds.x + look.item_padding.left, y),
            self.label.clone(),
            &style,
            ink.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.enabled.then(|| self.message.clone())
    }

    fn focusable(&self) -> bool {
        self.enabled
    }

    fn on_key(&self, key: &crate::interaction::Key) -> crate::interaction::KeyResponse<Msg> {
        use crate::interaction::{Key, KeyResponse};
        // A word that cannot be used answers no key, as it answers no press.
        if !self.enabled {
            return KeyResponse::Ignored;
        }
        let sent = match key {
            Key::Left { .. } => self.left.clone(),
            Key::Right { .. } => self.right.clone(),
            _ => None,
        };
        match sent {
            Some(message) => KeyResponse::Handled(Some(message)),
            None => KeyResponse::Ignored,
        }
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        let semantics =
            frus_core::SemanticsProperties::new(frus_core::Role::Button).label(self.label.clone());
        Some(if self.enabled {
            semantics.clickable()
        } else {
            semantics.disabled(true)
        })
    }

    fn debug_name(&self) -> &'static str {
        "MenuBarButton"
    }
}

impl<Msg> BarButton<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let fallback = Theme::default();
        let look = self.style.resolve(theme.unwrap_or(&fallback));
        let style = look.text_style.resolved();
        let width = frus_text::measure_resolved(&self.label, &style).width
            + look.item_padding.left
            + look.item_padding.right;
        Style {
            width: Dimension::Length(width.ceil()),
            height: Dimension::Length(look.height),
            ..Style::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for MenuBar<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style_themed(self, &Theme::default())
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        let look = self.style.resolve(theme);
        Style {
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            height: Dimension::Length(look.height),
            padding: look.padding,
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let look = self.style.resolve(theme);
        scene.fill_rect(bounds, look.background.fade(status.opacity));
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// A bar is a band across the top of the window.
    fn fill_axes(&self, _theme: &Theme) -> crate::widget::FillAxes {
        crate::widget::FillAxes {
            horizontal: true,
            vertical: false,
        }
    }

    fn debug_name(&self) -> &'static str {
        "MenuBar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime};
    use frus_core::Size;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Menu(MenuPath),
        New,
        Quit,
        Recent(usize),
        Wrap,
    }

    /// File (New, a rule, Open recent ▸ (two files), Quit) and View (Word wrap).
    fn bar(path: &MenuPath) -> MenuBar<Msg> {
        MenuBar::new(path, Msg::Menu)
            .menu(
                SubmenuButton::new("File")
                    .item(MenuItem::new("New", Msg::New).shortcut("Ctrl+N"))
                    .divider()
                    .submenu(
                        SubmenuButton::new("Open recent")
                            .item(MenuItem::new("notes.txt", Msg::Recent(0)))
                            .item(MenuItem::new("todo.md", Msg::Recent(1))),
                    )
                    .item(MenuItem::new("Quit", Msg::Quit)),
            )
            .menu(SubmenuButton::new("View").item(MenuItem::checked("Word wrap", true, Msg::Wrap)))
    }

    const SIZE: Size = Size::new(800.0, 600.0);

    fn ui(root: &MenuBar<Msg>) -> crate::Ui<Msg> {
        build_ui(root, SIZE, &Runtime::default(), &Theme::light())
    }

    /// What a press at `(x, y)` sends.
    fn press(root: &MenuBar<Msg>, x: f32, y: f32) -> Option<Msg> {
        let ui = ui(root);
        ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id))
    }

    /// What the pointer arriving at `(x, y)` sends.
    fn hover(root: &MenuBar<Msg>, x: f32, y: f32) -> Option<Msg> {
        let ui = ui(root);
        ui.hover_regions_at(Point::new(x, y))
            .into_iter()
            .find_map(|(id, _, _)| {
                crate::ui::find_widget(root as &dyn Widget<Msg>, id)?.on_hover_event(
                    crate::HoverEvent::Enter {
                        local: Point::new(1.0, 1.0),
                    },
                )
            })
    }

    /// Where the words `label` were painted, top left.
    fn text_at(root: &MenuBar<Msg>, label: &str) -> Option<Point> {
        let ui = ui(root);
        let found = ui.scene().primitives().iter().find_map(|p| match p {
            frus_core::Primitive::Text { position, text, .. } if text == label => Some(*position),
            _ => None,
        });
        found
    }

    fn path(indices: &[usize]) -> MenuPath {
        MenuPath::from_indices(indices.to_vec())
    }

    #[test]
    fn a_path_opens_and_closes_by_level() {
        let open = MenuPath::closed().opened(0, 2);
        assert_eq!(open.indices(), &[2]);
        let deeper = open.opened(1, 3);
        assert_eq!(deeper.indices(), &[2, 3]);
        assert_eq!(
            deeper.opened(0, 1).indices(),
            &[1],
            "another menu closes the submenu"
        );
        assert_eq!(deeper.truncated(1).indices(), &[2]);
        assert_eq!(deeper.up().indices(), &[2]);
        assert!(!deeper.truncated(0).is_open());
        assert_eq!(deeper.at(1), Some(3));
        assert_eq!(deeper.depth(), 2);
    }

    /// **Closed**, the bar shows its words and nothing else; pressing one opens it.
    #[test]
    fn pressing_a_word_opens_its_menu() {
        let closed = bar(&MenuPath::closed());
        let file = text_at(&closed, "File").expect("the word");
        assert!(text_at(&closed, "Quit").is_none(), "no menu is showing");
        assert_eq!(
            press(&closed, file.x + 5.0, 20.0),
            Some(Msg::Menu(path(&[0])))
        );
        let view = text_at(&closed, "View").expect("the word");
        assert_eq!(
            press(&closed, view.x + 5.0, 20.0),
            Some(Msg::Menu(path(&[1])))
        );
        // With nothing open, moving over a word opens nothing.
        assert_eq!(hover(&closed, view.x + 5.0, 20.0), None);
    }

    /// **Open**, its rows are under it; a row sends its own message, the word closes the
    /// menu, and a press anywhere else closes it too.
    #[test]
    fn an_open_menu_shows_its_rows_and_closes() {
        let open = bar(&path(&[0]));
        let file = text_at(&open, "File").expect("the word");
        let quit = text_at(&open, "Quit").expect("a row");
        assert!(quit.y > 48.0, "under the bar: {quit:?}");
        assert_eq!(press(&open, quit.x + 5.0, quit.y + 5.0), Some(Msg::Quit));
        assert_eq!(
            press(&open, file.x + 5.0, 20.0),
            Some(Msg::Menu(MenuPath::closed()))
        );
        assert_eq!(
            press(&open, 700.0, 500.0),
            Some(Msg::Menu(MenuPath::closed()))
        );
    }

    /// **Once one is open, the pointer moves between menus** with no press.
    #[test]
    fn hovering_another_word_switches_menus() {
        let open = bar(&path(&[0]));
        let view = text_at(&open, "View").expect("the word");
        assert_eq!(
            hover(&open, view.x + 5.0, 20.0),
            Some(Msg::Menu(path(&[1])))
        );
        let file = text_at(&open, "File").expect("the word");
        assert_eq!(hover(&open, file.x + 5.0, 20.0), None, "already open");
    }

    /// **A submenu** opens when the pointer comes onto its row, beside it; another row of
    /// the same menu closes it again.
    #[test]
    fn a_submenu_opens_on_hover_beside_its_row() {
        let open = bar(&path(&[0]));
        let recent = text_at(&open, "Open recent").expect("the row");
        assert!(text_at(&open, "notes.txt").is_none());
        assert_eq!(
            hover(&open, recent.x + 5.0, recent.y + 5.0),
            Some(Msg::Menu(path(&[0, 2])))
        );
        assert_eq!(
            press(&open, recent.x + 5.0, recent.y + 5.0),
            Some(Msg::Menu(path(&[0, 2])))
        );

        let deeper = bar(&path(&[0, 2]));
        let row = text_at(&deeper, "Open recent").expect("the row");
        let notes = text_at(&deeper, "notes.txt").expect("the submenu");
        assert!(
            notes.x > row.x + 100.0,
            "beside the row: {notes:?} vs {row:?}"
        );
        assert!(
            (notes.y - row.y).abs() < 10.0,
            "level with it: {notes:?} vs {row:?}"
        );
        assert_eq!(
            press(&deeper, notes.x + 5.0, notes.y + 5.0),
            Some(Msg::Recent(0))
        );
        let new = text_at(&deeper, "New").expect("a row");
        assert_eq!(
            hover(&deeper, new.x + 5.0, new.y + 5.0),
            Some(Msg::Menu(path(&[0])))
        );
    }

    /// **A disabled menu** is shown and does not open.
    #[test]
    fn a_disabled_menu_does_not_open() {
        let bar = MenuBar::new(&MenuPath::closed(), Msg::Menu).menu(
            SubmenuButton::new("Edit")
                .item(MenuItem::new("New", Msg::New))
                .enabled(false),
        );
        let edit = text_at(&bar, "Edit").expect("still shown");
        assert_eq!(press(&bar, edit.x + 5.0, 20.0), None);
        // Even a path that names it does not open it.
        let named = MenuBar::new(&path(&[0]), Msg::Menu).menu(
            SubmenuButton::new("Edit")
                .item(MenuItem::new("New", Msg::New))
                .enabled(false),
        );
        assert!(text_at(&named, "New").is_none());
    }

    /// **Near the window's end edge**, a submenu with no room beside its row opens on the
    /// other side instead of over the row.
    #[test]
    fn a_submenu_with_no_room_opens_on_the_other_side() {
        let crowded = |path: &MenuPath| {
            let filler = |word: &str| SubmenuButton::new(word).item(MenuItem::new("x", Msg::New));
            MenuBar::new(path, Msg::Menu)
                .menu(filler("Alpha"))
                .menu(filler("Bravo"))
                .menu(filler("Charlie"))
                .menu(filler("Delta"))
                .menu(
                    SubmenuButton::new("File").submenu(
                        SubmenuButton::new("Open recent")
                            .item(MenuItem::new("notes.txt", Msg::Recent(0))),
                    ),
                )
        };
        let root = crowded(&path(&[4, 0]));
        let ui = build_ui(
            &root,
            Size::new(560.0, 600.0),
            &Runtime::default(),
            &Theme::light(),
        );
        let at = |label: &str| {
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Text { position, text, .. } if text == label => {
                        Some(*position)
                    }
                    _ => None,
                })
                .expect(label)
        };
        let (row, notes) = (at("Open recent"), at("notes.txt"));
        assert!(notes.x < row.x, "on the start side: {notes:?} vs {row:?}");
        assert!(notes.x >= 0.0, "inside the window: {notes:?}");
    }

    /// The identity of the row whose words are `label`.
    fn row_id(root: &MenuBar<Msg>, label: &str) -> crate::interaction::WidgetId {
        let at = text_at(root, label).expect(label);
        ui(root)
            .hit(Point::new(at.x + 5.0, at.y + 5.0))
            .expect("a row")
    }

    /// What `key` sends from the row whose words are `label`.
    fn key_on(root: &MenuBar<Msg>, label: &str, key: crate::interaction::Key) -> Option<Msg> {
        let id = row_id(root, label);
        match crate::ui::find_widget(root as &dyn Widget<Msg>, id)?.on_key(&key) {
            crate::interaction::KeyResponse::Handled(message) => message,
            _ => None,
        }
    }

    /// What Escape sends from the row `label`, bubbling up from it as the shell does.
    fn escape_from(root: &MenuBar<Msg>, label: &str) -> Option<Msg> {
        let id = row_id(root, label);
        let path = crate::ui::find_path(root as &dyn Widget<Msg>, id);
        path.iter()
            .rev()
            .find_map(|w| match w.on_key(&crate::interaction::Key::Escape) {
                crate::interaction::KeyResponse::Handled(message) => Some(message),
                _ => None,
            })
            .flatten()
    }

    const LEFT: crate::interaction::Key = crate::interaction::Key::Left {
        shift: false,
        word: false,
    };
    const RIGHT: crate::interaction::Key = crate::interaction::Key::Right {
        shift: false,
        word: false,
    };

    /// **The first row that can be used takes the focus** when its menu opens: not a rule,
    /// not a disabled row.
    #[test]
    fn the_first_usable_row_takes_the_focus() {
        let open = bar(&path(&[0]));
        let focus: Vec<_> = ui(&open).autofocus_ids().collect();
        assert_eq!(focus, vec![row_id(&open, "New")]);

        let skipping = MenuBar::new(&path(&[0]), Msg::Menu).menu(
            SubmenuButton::new("File")
                .divider()
                .item(MenuItem::new("Gone", Msg::New).enabled(false))
                .item(MenuItem::new("Quit", Msg::Quit)),
        );
        let focus: Vec<_> = ui(&skipping).autofocus_ids().collect();
        assert_eq!(focus, vec![row_id(&skipping, "Quit")]);

        let deeper = bar(&path(&[0, 2]));
        let focus: Vec<_> = ui(&deeper).autofocus_ids().collect();
        assert!(
            focus.contains(&row_id(&deeper, "notes.txt")),
            "the submenu's too"
        );
    }

    /// **Right** opens a submenu from its row and goes on to the next menu from another;
    /// **left** goes back from a submenu, and to the menu before from a menu off the bar,
    /// round the ends.
    #[test]
    fn the_arrows_move_through_the_menus() {
        let open = bar(&path(&[0]));
        assert_eq!(
            key_on(&open, "Open recent", RIGHT),
            Some(Msg::Menu(path(&[0, 2])))
        );
        assert_eq!(key_on(&open, "New", RIGHT), Some(Msg::Menu(path(&[1]))));
        assert_eq!(
            key_on(&open, "New", LEFT),
            Some(Msg::Menu(path(&[1]))),
            "round the end"
        );

        let deeper = bar(&path(&[0, 2]));
        assert_eq!(
            key_on(&deeper, "notes.txt", LEFT),
            Some(Msg::Menu(path(&[0])))
        );
        assert_eq!(
            key_on(&deeper, "notes.txt", RIGHT),
            Some(Msg::Menu(path(&[1])))
        );
    }

    /// **From a word on the bar**, the arrows move to the menu beside it while one is open,
    /// and leave the focus to move as it does anywhere else while none is.
    #[test]
    fn the_arrows_on_the_bar_switch_menus_only_while_one_is_open() {
        let sent = |w: &dyn Widget<Msg>, key| match w.on_key(&key) {
            crate::interaction::KeyResponse::Handled(m) => m,
            _ => None,
        };
        // Open, the word is the anchor of the menu floating under it.
        let open = bar(&path(&[0]));
        let word = open.children()[0].children()[0].as_ref();
        assert_eq!(sent(word, RIGHT), Some(Msg::Menu(path(&[1]))));
        assert_eq!(
            sent(word, LEFT),
            Some(Msg::Menu(path(&[1]))),
            "round the end"
        );

        let closed = bar(&MenuPath::closed());
        assert_eq!(sent(closed.children()[0].as_ref(), RIGHT), None);
    }

    /// **Escape closes one level**: from a submenu, the submenu; from a menu, the menu.
    #[test]
    fn escape_closes_one_level() {
        let deeper = bar(&path(&[0, 2]));
        assert_eq!(
            escape_from(&deeper, "notes.txt"),
            Some(Msg::Menu(path(&[0])))
        );
        let open = bar(&path(&[0]));
        assert_eq!(
            escape_from(&open, "Quit"),
            Some(Msg::Menu(MenuPath::closed()))
        );
    }

    /// **A row whose submenu is shut is as wide as the others too**: it sits in the same
    /// portal open or shut, so that it keeps its identity (milestone 608), and a portal with
    /// nothing floating is laid out as any container, which must not shrink it either.
    #[test]
    fn a_row_with_its_submenu_shut_keeps_the_menus_width() {
        let open = bar(&path(&[0]));
        let recent = text_at(&open, "Open recent").expect("the row");
        assert_eq!(
            press(&open, recent.x + 120.0, recent.y + 5.0),
            Some(Msg::Menu(path(&[0, 2])))
        );
    }

    /// **A row with its submenu open is as wide as the others**, and its chevron is inside
    /// the menu. The open row is wrapped in the portal that floats the submenu, and the
    /// portal laid its anchor out at its label's width: the chevron was drawn off the menu's
    /// start edge, and a press near the row's end missed it (milestone 607).
    #[test]
    fn a_row_with_its_submenu_open_keeps_the_menus_width() {
        let deeper = bar(&path(&[0, 2]));
        let ui = ui(&deeper);
        let recent = text_at(&deeper, "Open recent").expect("the row");
        let quit = text_at(&deeper, "Quit").expect("another row");
        let id_at = |x: f32, y: f32| ui.hit(Point::new(x, y + 5.0));
        // Near the end of the menu, level with each row.
        let end = recent.x + 120.0;
        assert_eq!(
            press(&deeper, end, recent.y + 5.0),
            Some(Msg::Menu(path(&[0, 2])))
        );
        assert!(id_at(end, quit.y).is_some(), "the other row reaches as far");
        // The chevron: a path level with the row, past its label, inside the window.
        let chevron = ui
            .scene()
            .primitives()
            .iter()
            .find_map(|p| match p {
                p @ frus_core::Primitive::Path { .. } => {
                    let b = p.bounds();
                    ((b.y - recent.y).abs() < 16.0).then_some(b)
                }
                _ => None,
            })
            .expect("a chevron on the row");
        assert!(chevron.x > recent.x + 60.0, "at the row's end: {chevron:?}");
    }

    /// The scene of `root` under `theme`.
    fn scene_in(root: &MenuBar<Msg>, theme: &Theme) -> Vec<frus_core::Primitive> {
        build_ui(root, SIZE, &Runtime::default(), theme)
            .scene()
            .primitives()
            .to_vec()
    }

    /// The flat rectangles drawn at `(x, y)`, innermost last.
    fn rects_at(scene: &[frus_core::Primitive], x: f32, y: f32) -> Vec<(Rect, Color, f32)> {
        scene
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Rect {
                    rect,
                    color,
                    radius,
                    ..
                } if rect.contains(Point::new(x, y)) => Some((*rect, *color, radius.top_left)),
                _ => None,
            })
            .collect()
    }

    /// The words `label`: where, how big, in what colour.
    fn word(scene: &[frus_core::Primitive], label: &str) -> (Point, f32, Color) {
        scene
            .iter()
            .find_map(|p| match p {
                frus_core::Primitive::Text {
                    position,
                    text,
                    size,
                    color,
                    ..
                } if text == label => Some((*position, *size, *color)),
                _ => None,
            })
            .expect("the words are painted")
    }

    fn on(platform: frus_core::TargetPlatform) -> Theme {
        Theme::light().with_platform(platform)
    }

    /// **A desktop's bar is a desktop's**: 30 high, in 13 px words with 8 either side and
    /// 4 at the bar's start. A phone's keeps a finger's 48 and its label type.
    #[test]
    fn a_desktop_bar_is_a_desktops_and_a_phones_a_phones() {
        let closed = bar(&MenuPath::closed());
        let desk = scene_in(&closed, &on(frus_core::TargetPlatform::Windows));
        let surface = rects_at(&desk, 400.0, 2.0);
        assert_eq!(surface[0].0.height, 30.0, "the bar");
        let (file, size, _) = word(&desk, "File");
        assert_eq!(size, 13.0);
        assert_eq!(file.x, 4.0 + 8.0, "the bar's room, then the word's");
        let (view, _, _) = word(&desk, "View");
        let file_width = frus_text::measure_resolved("File", &TextStyle::new(13.0).resolved())
            .width
            .ceil();
        assert_eq!(
            view.x,
            file.x + file_width + 16.0,
            "8 after one, 8 before the next"
        );

        let phone = scene_in(&closed, &on(frus_core::TargetPlatform::Android));
        let surface = rects_at(&phone, 400.0, 2.0);
        assert_eq!(surface[0].0.height, crate::theme::MIN_TAP_TARGET);
        let (file, size, _) = word(&phone, "File");
        assert_eq!(size, Theme::light().text.label_large.resolved().size);
        assert_eq!(file.x, BAR_PAD_X);
    }

    /// **The open word keeps a highlight**, rounded and inside the bar, so the eye finds
    /// what the menu belongs to; a word at rest has none. The row whose submenu is open
    /// keeps its own, the panel's hover.
    #[test]
    fn the_open_word_and_the_open_row_keep_their_highlight() {
        let theme = on(frus_core::TargetPlatform::Linux);
        let open = scene_in(&bar(&path(&[0])), &theme);
        let (file, _, _) = word(&open, "File");
        let under = rects_at(&open, file.x + 2.0, 15.0);
        let (rect, colour, radius) = *under.last().expect("a highlight");
        assert_eq!((rect.y, rect.height, radius), (3.0, 24.0, 4.0));
        let surface = theme.scheme.surface_container;
        assert_eq!(colour, surface.lerp(theme.scheme.on_surface, 0.12));
        let (view, _, _) = word(&open, "View");
        assert_eq!(
            rects_at(&open, view.x + 2.0, 15.0).len(),
            1,
            "the bar alone under a word at rest"
        );

        let deeper = scene_in(&bar(&path(&[0, 2])), &theme);
        let (row, _, _) = word(&deeper, "Open recent");
        let (quit, _, _) = word(&deeper, "Quit");
        let lit = rects_at(&deeper, row.x, row.y + 4.0);
        let plain = rects_at(&deeper, quit.x, quit.y + 4.0);
        assert_eq!(
            lit.len(),
            plain.len() + 1,
            "the open row is lit, the others are not"
        );
    }

    /// **The highlight leans toward the bar's words** (milestone 642): white words on a dark
    /// accent bar — a title bar in the system's accent colour — get a lighter highlight, not
    /// one toward `on_surface`, which would darken it.
    #[test]
    fn the_highlight_leans_toward_the_words() {
        let theme = on(frus_core::TargetPlatform::Linux);
        let accent = Color::rgb(0.0, 0.2, 0.5);
        let open = scene_in(
            &bar(&path(&[0]))
                .background(accent)
                .foreground_color(Color::WHITE),
            &theme,
        );
        let (file, _, _) = word(&open, "File");
        let (_, colour, _) = *rects_at(&open, file.x + 2.0, 15.0)
            .last()
            .expect("a highlight");
        assert_eq!(colour, accent.lerp(Color::WHITE, 0.12));
    }

    /// **Every part of the bar is the caller's to say**: its height, surface, room,
    /// type, ink, highlight and the menus it opens.
    #[test]
    fn every_part_of_the_bar_can_be_said() {
        let red = Color::rgb(0.8, 0.1, 0.1);
        let green = Color::rgb(0.1, 0.7, 0.2);
        let blue = Color::rgb(0.1, 0.2, 0.9);
        let edge = frus_core::BorderSide {
            color: green,
            width: 2.0,
        };
        // Said after the menus: the words are built again.
        let styled = bar(&path(&[0]))
            .height(40.0)
            .background(red)
            .padding(Insets::new(0.0, 10.0, 0.0, 10.0))
            .item_padding(Insets::new(0.0, 20.0, 0.0, 20.0))
            .text_style(TextStyle::new(16.0))
            .foreground_color(green)
            .highlight_color(blue)
            .item_radius(0.0)
            .item_inset(0.0)
            .menu_style(crate::MenuTheme {
                item_height: Some(32.0),
                border: Some(edge),
                ..Default::default()
            });
        let scene = scene_in(&styled, &on(frus_core::TargetPlatform::Linux));
        let surface = rects_at(&scene, 400.0, 2.0);
        assert_eq!((surface[0].0.height, surface[0].1), (40.0, red));
        let (file, size, ink) = word(&scene, "File");
        assert_eq!((file.x, size, ink), (30.0, 16.0, green));
        let (rect, colour, radius) = *rects_at(&scene, file.x, 20.0).last().expect("lit");
        assert_eq!(
            (rect.y, rect.height, colour, radius),
            (0.0, 40.0, blue, 0.0)
        );
        let (new, _, _) = word(&scene, "New");
        let (quit, _, _) = word(&scene, "Quit");
        assert!(scene.iter().any(|p| matches!(
            p,
            frus_core::Primitive::Rect { border_width, border_color, .. }
                if *border_width == 2.0 && *border_color == green
        )));
        // New, a rule, Open recent, Quit: three rows of 32 and a rule apart.
        // Said last, a word still reaches the words already built.
        let last = bar(&MenuPath::closed()).text_style(TextStyle::new(18.0));
        let scene_last = scene_in(&last, &on(frus_core::TargetPlatform::Linux));
        assert_eq!(word(&scene_last, "File").1, 18.0);
        let rule = quit.y - new.y - 2.0 * 32.0;
        assert_eq!(
            rule,
            crate::MenuTheme::desktop(&Theme::light())
                .divider_height
                .unwrap()
        );
    }

    /// **The theme says what the caller did not**, and the caller outranks it.
    #[test]
    fn the_theme_answers_what_the_caller_left_unsaid() {
        let mut theme = on(frus_core::TargetPlatform::Linux);
        theme.widgets.menu_bar = crate::MenuBarTheme {
            height: Some(36.0),
            foreground: Some(Color::rgb(0.5, 0.0, 0.5)),
            menus: crate::MenuTheme {
                item_height: Some(28.0),
                ..Default::default()
            },
            ..Default::default()
        };
        let scene = scene_in(&bar(&path(&[0])), &theme);
        assert_eq!(rects_at(&scene, 400.0, 2.0)[0].0.height, 36.0);
        assert_eq!(word(&scene, "File").2, Color::rgb(0.5, 0.0, 0.5));
        let (new, _, _) = word(&scene, "New");
        let (rule_after, _, _) = word(&scene, "Open recent");
        let (quit, _, _) = word(&scene, "Quit");
        assert_eq!(quit.y - rule_after.y, 28.0, "the theme's rows");
        assert!(rule_after.y > new.y);

        let said = bar(&path(&[0])).height(44.0);
        let scene = scene_in(&said, &theme);
        assert_eq!(rects_at(&scene, 400.0, 2.0)[0].0.height, 44.0);
    }

    /// **A desktop's menus are a desktop's**: 26 px rows in 13 px type, ringed by a
    /// hairline, with rules that are a line and room. A phone's keep a finger's rows.
    #[test]
    fn a_desktop_bars_menus_are_a_desktops() {
        let open = bar(&path(&[0]));
        let theme = on(frus_core::TargetPlatform::MacOs);
        let scene = scene_in(&open, &theme);
        let (row, size, _) = word(&scene, "Open recent");
        let (quit, _, _) = word(&scene, "Quit");
        assert_eq!((quit.y - row.y, size), (26.0, 13.0));
        assert!(scene.iter().any(|p| matches!(
            p,
            frus_core::Primitive::Rect { border_width, border_color, .. }
                if *border_width == 1.0 && *border_color == theme.scheme.outline_variant
        )));
        let (new, _, _) = word(&scene, "New");
        let line = scene.iter().find_map(|p| match p {
            frus_core::Primitive::Rect { rect, .. }
                if rect.height == 1.0 && rect.y > new.y && rect.y < row.y =>
            {
                Some(*rect)
            }
            _ => None,
        });
        assert!(
            line.is_some_and(|l| l.width > 100.0),
            "the rule is a line across the menu"
        );

        let phone = scene_in(&open, &on(frus_core::TargetPlatform::Android));
        let (row, _, _) = word(&phone, "Open recent");
        let (quit, _, _) = word(&phone, "Quit");
        assert!(quit.y - row.y >= crate::theme::MIN_TAP_TARGET);
    }
}
