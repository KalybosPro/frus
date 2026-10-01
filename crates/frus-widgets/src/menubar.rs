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

use frus_core::{Point, Rect, Scene};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::interaction::Status;
use crate::menu::{menu_panel, MenuItem, PanelStyle, RowKeys, RowLook};
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
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> MenuBar<Msg> {
    /// A bar with `path` open, sending `on_path` the path it should have next.
    pub fn new(path: &MenuPath, on_path: impl Fn(MenuPath) -> Msg + 'static) -> Self {
        Self {
            path: path.clone(),
            on_path: Rc::new(on_path),
            menus: Vec::new(),
            children: Vec::new(),
        }
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
                open,
                enabled: menu.enabled,
                message: on_path(press),
                left: arrow(-1),
                right: arrow(1),
            }),
            hover,
        );
        if !open {
            return word;
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
        let panel = panel(&menu.entries, 1, &self.path, on_path, &across);
        Box::new(
            OverlayPortal::new_boxed(word)
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
                let row = on_enter(row, hover);
                if open {
                    let inner = panel(&sub.entries, level + 1, path, on_path, across);
                    Box::new(OverlayPortal::new_boxed(row).overlay_boxed(inner, Placement::Beside))
                } else {
                    row
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
            look: PanelStyle::default(),
            text_style: None,
            item_padding: None,
            item_height: None,
            enabled: true,
        },
        &decorate,
    )
}

/// `row`, sending `hover` when the pointer comes onto it.
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
        let base = theme.scheme.surface_container;
        // An open menu's word stays lit, so the eye can find what the panel belongs to.
        let lit = if self.open {
            theme.state_layer(
                base,
                theme.on_surface,
                &Status {
                    interaction: crate::interaction::Interaction::Hovered,
                    ..status
                },
            )
        } else if self.enabled {
            theme.state_layer(base, theme.on_surface, &status)
        } else {
            base
        };
        if lit != base {
            scene.fill_rect(bounds, lit.fade(o));
        }
        let style = crate::theme::type_scale(Some(theme)).label_large.resolved();
        let ink = if self.enabled {
            theme.on_surface
        } else {
            crate::disabled::disabled_content(theme)
        };
        let y = bounds.y + (bounds.height - style.line_height()) * 0.5;
        scene.text(
            Point::new(bounds.x + BAR_PAD_X, y),
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
        let style = crate::theme::type_scale(theme).label_large.resolved();
        let width = frus_text::measure_resolved(&self.label, &style).width + 2.0 * BAR_PAD_X;
        Style {
            width: Dimension::Length(width.ceil()),
            height: Dimension::Length(crate::theme::MIN_TAP_TARGET),
            ..Style::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for MenuBar<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            align: Align::Center,
            ..Style::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, _status: Status, theme: &Theme, scene: &mut Scene) {
        scene.fill_rect(bounds, theme.scheme.surface_container);
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
}
