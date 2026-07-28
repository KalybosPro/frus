//! [`Table`] : un tableau de données texte à **rangées `Flex`** (colonnes de largeur
//! **fixe ou flexible**). En-tête **triable** au clic (indicateur de sens), lignes
//! **sélectionnables**, et **sélection multiple** optionnelle via une colonne de cases à
//! cocher coiffée d'un « tout cocher ».
//!
//! Comme le reste de frus, tri et sélection sont **décidés par l'application** : le
//! tableau émet un message au clic (`on_sort`, `on_select_row`, `on_check`,
//! `on_check_all`) et n'affiche que l'état qu'on lui passe (`sorted`, `selected`).

use std::rc::Rc;

use frus_core::{Color, Insets, Path, Point, Rect, Scene};
use frus_layout::{Align, Dimension, Justify, Style};

use crate::flex::Flex;
use crate::icons::IconName;
use crate::list::List;
use crate::interaction::{Key, KeyResponse, Status};
use crate::stack::Stack;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 34.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 15.0;
/// Écart entre rangées et entre cellules (doit coïncider avec la géométrie des poignées).
const ROW_GAP: f32 = 2.0;
/// Largeur de la colonne des cases à cocher (sélection multiple).
const CHECK_W: f32 = 40.0;
/// Côté de la case à cocher dessinée.
const BOX: f32 = 18.0;
/// Largeur de la **zone de préhension** d'une poignée de redimensionnement.
const HANDLE_W: f32 = 8.0;

/// Fond commun d'une cellule selon son rôle et l'interaction (facteur partagé).
fn cell_background(header: bool, selected: bool, clickable: bool, theme: &Theme, status: &Status) -> Color {
    let base = if header {
        theme.surface.lerp(theme.on_surface, 0.06)
    } else if selected {
        theme.surface.lerp(theme.primary, 0.16)
    } else {
        theme.surface
    };
    if clickable {
        theme.state_layer(base, theme.on_surface, status)
    } else {
        base
    }
}

/// Style d'une cellule : largeur de colonne (fixe ou flexible), et hauteur **adaptative**
/// — un plancher `ROW_H` (confort minimal) qui grandit avec le contenu. Comme la rangée
/// aligne ses cellules en `Stretch`, toutes suivent la plus haute (aucun rognage).
fn cell_style(width: Dimension) -> Style {
    let flex_grow = if matches!(width, Dimension::Length(_)) { 0.0 } else { 1.0 };
    Style {
        width,
        height: Dimension::Auto,
        min_height: Dimension::Length(ROW_H),
        flex_grow,
        ..Default::default()
    }
}

/// Côté d'une icône d'en-tête (grille `24×24` mise à l'échelle).
const ICON: f32 = 16.0;
/// Écart entre l'icône d'en-tête et son libellé.
const ICON_GAP: f32 = 6.0;

/// Une cellule texte (en-tête ou donnée), thémée au rendu.
struct Cell<Msg> {
    label: String,
    width: Dimension,
    header: bool,
    selected: bool,
    /// Index de ligne (cellule de donnée) : sert à énoncer « Row N selected » au lecteur
    /// d'écran. `None` pour un en-tête.
    row: Option<usize>,
    /// Icône **de tête** (en-tête uniquement) : peinte avant le libellé (icône + texte).
    icon: Option<IconName>,
    /// Indicateur de tri de l'en-tête : `Some(true)` = ▲, `Some(false)` = ▼.
    sort: Option<bool>,
    message: Option<Msg>,
    /// En-tête **réordonnable** : `(index de colonne, nombre de colonnes, rappel
    /// on_reorder(from, to))`. Le nombre de colonnes borne le réordonnancement clavier.
    reorder: Option<(usize, usize, Rc<dyn Fn(usize, usize) -> Msg>)>,
    /// **Widget d'action** d'en-tête (0 ou 1) : un bouton (filtre, menu…) posé à droite,
    /// **enfant** de la cellule. Il capte son propre clic (hit-test du plus profond),
    /// le reste de l'en-tête continuant de trier — d'où un `Vec` pour l'exposer via
    /// [`children`](Widget::children).
    action: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for Cell<Msg> {
    fn style(&self) -> Style {
        let base = cell_style(self.width);
        // Avec un widget d'action, il se pose à **droite** (le libellé restant peint à
        // gauche) et centré verticalement.
        if self.action.is_empty() {
            base
        } else {
            Style {
                justify: Justify::End,
                align: Align::Center,
                padding: Insets::new(0.0, PAD_X, 0.0, 0.0),
                ..base
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.action
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        let bg = cell_background(self.header, self.selected, clickable, theme, &status);
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }

        let color = if self.header { theme.muted } else { theme.on_surface };
        let ty = bounds.y + (bounds.height - frus_text::line_height(SIZE)) * 0.5;

        // Icône de tête (en-tête) : peinte à gauche, le libellé décalé à sa suite.
        let mut text_x = bounds.x + PAD_X;
        if let Some(icon) = self.icon {
            let iy = bounds.y + (bounds.height - ICON) * 0.5;
            let path = icon.path().scaled(ICON / 24.0).translated(text_x, iy);
            scene.fill_path(&path, color.fade(o));
            text_x += ICON + ICON_GAP;
        }
        scene.text(Point::new(text_x, ty), self.label.clone(), SIZE, color.fade(o));

        if let (true, Some(ascending)) = (self.header, self.sort) {
            let lw = frus_text::measure(&self.label, SIZE).width;
            let cx = text_x + lw + 8.0;
            let cy = bounds.y + bounds.height * 0.5;
            let (w, h) = (4.0, 4.0);
            let tri = if ascending {
                Path::new()
                    .move_to(Point::new(cx, cy - h))
                    .line_to(Point::new(cx - w, cy + h))
                    .line_to(Point::new(cx + w, cy + h))
                    .close()
            } else {
                Path::new()
                    .move_to(Point::new(cx, cy + h))
                    .line_to(Point::new(cx - w, cy - h))
                    .line_to(Point::new(cx + w, cy - h))
                    .close()
            };
            scene.fill_path(&tri, theme.on_surface.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        // Seuls les en-têtes triables prennent le focus clavier (Entrée/Espace = trier) ;
        // les cellules de données restent cliquables à la souris sans encombrer le Tab.
        self.header && self.message.is_some()
    }

    fn reorder_index(&self) -> Option<usize> {
        self.reorder.as_ref().map(|(col, _, _)| *col)
    }

    fn on_reorder(&self, to: usize) -> Option<Msg> {
        self.reorder.as_ref().map(|(col, _, cb)| cb(*col, to))
    }

    fn announce(&self) -> Option<String> {
        // En-tête triable : énonce le tri **résultant** au lecteur d'écran (bascule du
        // sens courant : croissant par défaut, sinon on l'inverse — schéma Material usuel).
        if self.header && self.message.is_some() {
            let ascending = !matches!(self.sort, Some(true));
            let dir = if ascending { "ascending" } else { "descending" };
            return Some(format!("Sorted by {} {}", self.label, dir));
        }
        // Cellule de donnée sélectionnable : énonce l'état **résultant** de la ligne.
        if let (false, Some(row), true) = (self.header, self.row, self.message.is_some()) {
            let verb = if self.selected { "deselected" } else { "selected" };
            return Some(format!("Row {} {}", row + 1, verb));
        }
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // En-tête : annoncé aux lecteurs d'écran (libellé + position s'il est réordonnable,
        // pour que l'utilisateur perçoive un déplacement de colonne en re-parcourant).
        if !self.header {
            return None;
        }
        let mut sem = frus_core::Semantics::new(frus_core::Role::Button).label(self.label.clone());
        if let Some((col, columns, _)) = &self.reorder {
            sem = sem.value(format!("column {} of {}", col + 1, columns));
        }
        Some(sem)
    }

    fn on_key(&self, key: &Key) -> KeyResponse<Msg> {
        // Ctrl+Flèches (Left/Right avec `word`) sur un en-tête focalisé : déplace la
        // colonne d'un cran (borné). Les flèches nues laissent naviguer le focus.
        let Some((col, columns, cb)) = self.reorder.as_ref() else {
            return KeyResponse::Ignored;
        };
        let to = match key {
            Key::Left { word: true, .. } => col.checked_sub(1),
            Key::Right { word: true, .. } if col + 1 < *columns => Some(col + 1),
            _ => return KeyResponse::Ignored,
        };
        match to {
            Some(to) => KeyResponse::Handled(Some(cb(*col, to))),
            None => KeyResponse::Ignored, // au bord : laisse le focus naviguer
        }
    }
}

/// Une cellule case à cocher (colonne de sélection multiple).
struct CheckCell<Msg> {
    checked: bool,
    /// État **indéterminé** (certaines lignes cochées, pas toutes) — case du « tout
    /// cocher ». Prime l'affichage décoché ; ignoré si `checked`.
    indeterminate: bool,
    header: bool,
    selected: bool,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for CheckCell<Msg> {
    fn style(&self) -> Style {
        cell_style(Dimension::Length(CHECK_W))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        let bg = cell_background(self.header, self.selected, clickable, theme, &status);
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }

        let bx = bounds.x + (bounds.width - BOX) * 0.5;
        let by = bounds.y + (bounds.height - BOX) * 0.5;
        let box_rect = Rect::new(bx, by, BOX, BOX);
        if self.checked {
            scene.draw_rect(box_rect, theme.primary.fade(o), 4.0, 0.0, Color::TRANSPARENT);
            // Coche : l'icône Check remplie, centrée dans la case.
            let scale = (BOX - 4.0) / 24.0;
            let inset = (BOX - 24.0 * scale) * 0.5;
            let path = IconName::Check.path().scaled(scale).translated(bx + inset, by + inset);
            scene.fill_path(&path, theme.on_primary.fade(o));
        } else if self.indeterminate {
            // Indéterminé : case pleine barrée d'un tiret (façon Material).
            scene.draw_rect(box_rect, theme.primary.fade(o), 4.0, 0.0, Color::TRANSPARENT);
            let dash = Rect::new(bx + 4.0, by + BOX * 0.5 - 1.0, BOX - 8.0, 2.0);
            scene.draw_rect(dash, theme.on_primary.fade(o), 1.0, 0.0, Color::TRANSPARENT);
        } else {
            scene.draw_rect(box_rect, Color::TRANSPARENT, 4.0, 1.5, theme.muted.fade(o));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }

    fn announce(&self) -> Option<String> {
        // Case à cocher : énonce l'état **résultant** de la bascule (cochée → on
        // décoche). La case d'en-tête agit sur **toutes** les lignes.
        self.message.as_ref()?;
        let selecting = !self.checked;
        Some(match (self.header, selecting) {
            (true, true) => "All rows selected".into(),
            (true, false) => "All rows deselected".into(),
            (false, true) => "Row selected".into(),
            (false, false) => "Row deselected".into(),
        })
    }
}

/// Fabrique d'un widget de cellule : **rappelée à chaque reconstruction** (le tableau
/// se rebâtit après chaque réglage), donc elle produit un widget **frais** à chaque fois.
/// Permet des cellules riches (avatars, puces, boutons d'action) au-delà du texte.
pub type CellFactory<Msg> = std::rc::Rc<dyn Fn() -> Box<dyn Widget<Msg>>>;

/// Contenu d'une ligne de données : texte simple, ou widgets (par colonne).
enum RowKind<Msg> {
    Text(Vec<String>),
    Widgets(Vec<CellFactory<Msg>>),
}

/// Une cellule **widget** : un contenu arbitraire (centré, avec le fond de cellule et,
/// si la ligne est sélectionnable, le clic de sélection). Le contenu se peint par-dessus.
struct WidgetCell<Msg> {
    width: Dimension,
    selected: bool,
    /// Cellule d'**en-tête** (fond d'en-tête) plutôt qu'une cellule de donnée.
    header: bool,
    /// Index de ligne : pour énoncer « Row N selected » au lecteur d'écran.
    row: usize,
    message: Option<Msg>,
    content: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone> Widget<Msg> for WidgetCell<Msg> {
    fn style(&self) -> Style {
        Style {
            align: Align::Center,
            padding: Insets::new(0.0, PAD_X, 0.0, PAD_X),
            ..cell_style(self.width)
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.content
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        let bg = cell_background(self.header, self.selected, clickable, theme, &status);
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn announce(&self) -> Option<String> {
        // Ligne-widget sélectionnable : énonce l'état **résultant** de la sélection.
        self.message.as_ref()?;
        let verb = if self.selected { "deselected" } else { "selected" };
        Some(format!("Row {} {}", self.row + 1, verb))
    }
}

/// Un espace transparent et **inerte** (ni cliquable, ni glissable) : cale les
/// poignées de redimensionnement sur les bords de colonnes sans bloquer les clics
/// (le calque de poignées flotte au-dessus de la grille).
struct Spacer {
    width: f32,
    height: f32,
}

impl<Msg: Clone> Widget<Msg> for Spacer {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(self.width),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Une **poignée de redimensionnement** de colonne : fine barre verticale au bord
/// droit d'une colonne. Glissée horizontalement, elle émet `on_resize(col, dx)` où
/// `dx` est le déplacement (px) depuis le dernier événement — l'application
/// **accumule** la largeur. Le calque de poignées flotte au-dessus de la grille.
struct ResizeHandle<Msg> {
    col: usize,
    height: f32,
    on_resize: Rc<dyn Fn(usize, f32) -> Msg>,
}

impl<Msg: Clone> Widget<Msg> for ResizeHandle<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(HANDLE_W),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // Ligne de préhension centrée : discrète au repos, teintée `primary` et
        // épaissie au survol / pendant le glissement.
        let t = status.hover_progress;
        let color = theme.border.lerp(theme.primary, t);
        let lw = 1.0 + t; // 1 px → 2 px
        let x = bounds.x + (HANDLE_W - lw) * 0.5;
        scene.draw_rect(
            Rect::new(x, bounds.y, lw, bounds.height),
            color.fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn draggable(&self) -> bool {
        true
    }

    fn on_drag_delta(&self, dx: f32) -> Option<Msg> {
        // Un delta nul (appui, mouvement vertical pur) ne change rien.
        if dx == 0.0 {
            None
        } else {
            Some((self.on_resize)(self.col, dx))
        }
    }
}

/// Un tableau de données à colonnes fixes ou flexibles (voir le module).
pub struct Table<Msg> {
    columns: usize,
    headers: Vec<String>,
    /// Icône de tête par colonne d'en-tête (icône + libellé). Manquante = aucune.
    header_icons: Vec<Option<IconName>>,
    /// En-tête **entièrement widget** (par colonne) : remplace la ligne d'en-tête texte.
    /// Vide = en-tête texte classique. Le tri/réordonnancement automatique ne s'applique
    /// pas à ces en-têtes — l'application câble le comportement dans ses widgets.
    header_widgets: Vec<CellFactory<Msg>>,
    /// Widget d'action par colonne d'en-tête (bouton de filtre/menu), fabriqué à chaque
    /// reconstruction. `None` = aucun. Posé à droite de l'en-tête, il capte son clic.
    header_actions: Vec<Option<CellFactory<Msg>>>,
    rows: Vec<RowKind<Msg>>,
    /// Largeur par colonne : `> 0` = fixe (px), `<= 0` = flexible (part égale).
    widths: Vec<f32>,
    total_width: Option<f32>,
    sort: Option<(usize, bool)>,
    selected: Vec<usize>,
    on_sort: Option<Box<dyn Fn(usize) -> Msg>>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    on_check: Option<Box<dyn Fn(usize) -> Msg>>,
    on_check_all: Option<Msg>,
    /// Rappel de redimensionnement (colonne, delta px). Actif seulement si toutes
    /// les colonnes sont de largeur **fixe** (géométrie des bords connue).
    on_resize: Option<Rc<dyn Fn(usize, f32) -> Msg>>,
    /// Rappel de réordonnancement (`on_reorder(from, to)`) : glisser un en-tête sur
    /// un autre déplace la colonne. Nécessite aussi `on_sort` (en-têtes cliquables).
    on_reorder: Option<Rc<dyn Fn(usize, usize) -> Msg>>,
    /// Mode **virtualisé** : `(nombre de lignes, hauteur du viewport, fabrique de la ligne
    /// `index -> textes de cellules`)`. Seules les lignes **visibles** sont construites
    /// (défilement interne), pour des grilles de milliers de lignes. Exclut `rows`.
    virtual_data: Option<(usize, f32, Rc<dyn Fn(usize) -> Vec<String>>)>,
    root: Box<dyn Widget<Msg>>,
}

/// Dimension d'une colonne depuis les largeurs : fixe si `> 0`, flexible sinon
/// (facteur partagé entre la construction directe et la construction virtualisée).
fn col_dimension(widths: &[f32], c: usize) -> Dimension {
    match widths.get(c).copied().unwrap_or(0.0) {
        w if w > 0.0 => Dimension::Length(w),
        _ => Dimension::Auto,
    }
}

impl<Msg: Clone + 'static> Table<Msg> {
    /// Crée un tableau de `columns` colonnes (flexibles, largeurs égales par défaut).
    pub fn new(columns: usize) -> Self {
        let columns = columns.max(1);
        Self {
            columns,
            headers: Vec::new(),
            header_icons: Vec::new(),
            header_widgets: Vec::new(),
            header_actions: Vec::new(),
            rows: Vec::new(),
            widths: vec![0.0; columns],
            total_width: None,
            sort: None,
            selected: Vec::new(),
            on_sort: None,
            on_select: None,
            on_check: None,
            on_check_all: None,
            on_resize: None,
            on_reorder: None,
            virtual_data: None,
            root: Box::new(Flex::<Msg>::column().gap(ROW_GAP)),
        }
    }

    /// Définit la ligne d'en-tête (une étiquette par colonne).
    pub fn header(mut self, labels: &[&str]) -> Self {
        self.headers = labels.iter().map(|s| s.to_string()).collect();
        self.rebuild();
        self
    }

    /// Remplace la ligne d'en-tête par des **en-têtes entièrement widget** (un par
    /// colonne) — pour des grilles très personnalisées (bouton de tri maison, filtre
    /// intégré, deux lignes de titre…). Le tri et le réordonnancement **automatiques** ne
    /// s'appliquent pas ici : l'application câble le comportement dans les widgets fournis
    /// (p.ex. un bouton émettant son propre message de tri). Chaque fabrique est rappelée à
    /// la reconstruction (widget frais). Exclut [`header`](Self::header) (dernier appelé gagne).
    pub fn widget_header(mut self, cells: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>>) -> Self {
        self.header_widgets = cells.into_iter().map(std::rc::Rc::from).collect();
        self.headers.clear();
        self.rebuild();
        self
    }

    /// Donne une **icône de tête** à des colonnes d'en-tête (icône + libellé) :
    /// `None` laisse la colonne sans icône. L'en-tête reste **triable** et
    /// **réordonnable** comme un en-tête texte (l'icône est purement décorative).
    pub fn header_icons(mut self, icons: &[Option<IconName>]) -> Self {
        self.header_icons = icons.iter().copied().collect();
        self.rebuild();
        self
    }

    /// Pose un **widget d'action** (bouton de filtre, menu…) à droite de l'en-tête de la
    /// colonne `col`. C'est un **enfant** de la cellule : il capte son propre clic (hit-test
    /// du plus profond), tandis que le reste de l'en-tête **trie** et se **réordonne**
    /// comme d'habitude. La fabrique est rappelée à chaque reconstruction (widget frais).
    ///
    /// **Menu de colonne** : passez ici un [`Menu`](crate::Menu) ou un
    /// [`Dropdown`](crate::Dropdown). Son menu **flottant** est rendu même **imbriqué** dans
    /// l'en-tête (l'overlay est collecté à n'importe quelle profondeur), atteignable au
    /// **Tab** et piloté flèches/Entrée, et fermé par Échap / clic extérieur — sans code
    /// spécifique côté tableau (l'application pilote l'état ouvert/fermé du menu).
    pub fn header_action(mut self, col: usize, make: impl Fn() -> Box<dyn Widget<Msg>> + 'static) -> Self {
        if col < self.columns {
            if self.header_actions.len() <= col {
                self.header_actions.resize_with(col + 1, || None);
            }
            self.header_actions[col] = Some(std::rc::Rc::new(make));
        }
        self.rebuild();
        self
    }

    /// Ajoute une ligne de données (une valeur **texte** par colonne).
    pub fn row(mut self, cells: &[&str]) -> Self {
        self.rows.push(RowKind::Text(cells.iter().map(|s| s.to_string()).collect()));
        self.rebuild();
        self
    }

    /// Ajoute une ligne dont chaque cellule est un **widget** (avatar, puce, bouton
    /// d'action…), fourni par une **fabrique** rappelée à chaque reconstruction. La
    /// ligne reste sélectionnable (fond au clic hors des zones cliquables internes).
    ///
    /// **Tri d'une colonne-widget** : le tableau ne sait pas comparer des widgets — il
    /// n'émet que la colonne cliquée (`on_sort`). C'est l'**application** qui fournit la
    /// clé : au message de tri, elle ordonne ses données par le champ correspondant à la
    /// colonne (p.ex. le nom derrière un avatar), puis repasse les lignes déjà triées.
    pub fn widget_row(mut self, cells: Vec<Box<dyn Fn() -> Box<dyn Widget<Msg>>>>) -> Self {
        self.rows.push(RowKind::Widgets(cells.into_iter().map(std::rc::Rc::from).collect()));
        self.rebuild();
        self
    }

    /// Fixe la largeur totale du tableau, en pixels logiques (les colonnes flexibles se
    /// partagent l'espace restant).
    pub fn width(mut self, width: f32) -> Self {
        self.total_width = Some(width);
        self.rebuild();
        self
    }

    /// Largeur **fixe** de chaque colonne, en pixels (`0` ou moins = colonne flexible).
    /// Les entrées manquantes laissent la colonne flexible.
    pub fn column_widths(mut self, widths: &[f32]) -> Self {
        for (i, w) in widths.iter().enumerate().take(self.columns) {
            self.widths[i] = *w;
        }
        self.rebuild();
        self
    }

    /// Rend les en-têtes **cliquables** : `on_sort(colonne)` au clic sur un en-tête.
    pub fn on_sort(mut self, on_sort: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_sort = Some(Box::new(on_sort));
        self.rebuild();
        self
    }

    /// Indique la colonne triée et son sens (`true` = croissant) → affiche l'indicateur.
    pub fn sorted(mut self, column: usize, ascending: bool) -> Self {
        self.sort = Some((column, ascending));
        self.rebuild();
        self
    }

    /// Rend les lignes **cliquables** : `on_select_row(ligne)` au clic sur une ligne.
    pub fn on_select_row(mut self, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self.rebuild();
        self
    }

    /// Active la **sélection multiple** : une colonne de cases à cocher (à gauche) coiffée
    /// d'une case « tout cocher ». `on_check(ligne)` bascule une ligne, `on_check_all`
    /// bascule toutes les lignes. L'état coché reflète [`selected`](Self::selected).
    pub fn checkboxes(mut self, on_check: impl Fn(usize) -> Msg + 'static, on_check_all: Msg) -> Self {
        self.on_check = Some(Box::new(on_check));
        self.on_check_all = Some(on_check_all);
        self.rebuild();
        self
    }

    /// Passe le tableau en mode **virtualisé** : `count` lignes de données, dont seules les
    /// **visibles** sont construites/mises en page/peintes (défilement vertical interne dans
    /// un viewport de `viewport_height` px). `build(index)` fournit les **textes** de la
    /// ligne (une valeur par colonne). Indispensable pour les grandes grilles (milliers de
    /// lignes) : coût par frame ∝ lignes visibles, pas au total. L'en-tête reste **épinglé**.
    ///
    /// Sélection : [`on_select_row`](Self::on_select_row) et [`selected`](Self::selected)
    /// fonctionnent sur les lignes visibles. Non combiné avec cases à cocher /
    /// redimensionnement / réordonnancement / cellules-widgets (le contenu hors écran n'a pas
    /// d'état retenu) — ceux-ci sont ignorés en mode virtualisé. Exclut [`row`](Self::row).
    pub fn virtual_rows(
        mut self,
        count: usize,
        viewport_height: f32,
        build: impl Fn(usize) -> Vec<String> + 'static,
    ) -> Self {
        self.virtual_data = Some((count, viewport_height, Rc::new(build)));
        self.rows.clear();
        self.rebuild();
        self
    }

    /// Rend les colonnes **redimensionnables** à la souris : une fine poignée au
    /// bord droit de chaque colonne (sauf la dernière) émet `on_resize(colonne,
    /// delta_px)` quand on la glisse. L'application **accumule** la largeur, p.ex.
    /// `widths[col] = (widths[col] + delta).max(MIN)`, et la repasse via
    /// [`column_widths`](Self::column_widths). N'a d'effet que si **toutes** les
    /// colonnes ont une largeur fixe (bords connus).
    pub fn on_resize(mut self, on_resize: impl Fn(usize, f32) -> Msg + 'static) -> Self {
        self.on_resize = Some(Rc::new(on_resize));
        self.rebuild();
        self
    }

    /// Rend les colonnes **réordonnables** : glisser un en-tête (au-delà du seuil) et
    /// le déposer sur un autre émet `on_reorder(from, to)` ; l'application permute
    /// l'ordre de ses colonnes. Un simple **clic** trie toujours (`on_sort`). Sans
    /// effet si les en-têtes ne sont pas cliquables.
    pub fn on_reorder(mut self, on_reorder: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.on_reorder = Some(Rc::new(on_reorder));
        self.rebuild();
        self
    }

    /// Indique les lignes sélectionnées (surlignées, cases cochées).
    pub fn selected(mut self, rows: &[usize]) -> Self {
        self.selected = rows.to_vec();
        self.rebuild();
        self
    }

    /// Dimension de la colonne `c` : fixe si `widths[c] > 0`, flexible sinon.
    fn col_width(&self, c: usize) -> Dimension {
        col_dimension(&self.widths, c)
    }

    /// Une rangée `Flex`, à la largeur totale du tableau si fixée.
    fn new_row(&self) -> Flex<Msg> {
        let row = Flex::row().gap(ROW_GAP);
        match self.total_width {
            Some(w) => row.width(w),
            None => row,
        }
    }

    /// True si toutes les lignes sont sélectionnées (pour le « tout cocher »).
    fn all_selected(&self) -> bool {
        !self.rows.is_empty() && (0..self.rows.len()).all(|r| self.selected.contains(&r))
    }

    /// True si **certaines** lignes (pas toutes) sont sélectionnées → « tout cocher »
    /// indéterminé.
    fn some_selected(&self) -> bool {
        (0..self.rows.len()).any(|r| self.selected.contains(&r)) && !self.all_selected()
    }

    /// Régénère l'arbre (rangées + cellules) depuis les données et l'état courants.
    /// L'ordre des appels du builder n'importe pas : l'état final est cohérent.
    /// Calque de poignées (une par bord de colonne, sauf la dernière) à superposer
    /// à la grille. `None` si non redimensionnable ou colonnes non toutes fixes.
    fn resize_overlay(&self, total_h: f32) -> Option<Flex<Msg>> {
        let on_resize = self.on_resize.as_ref()?;
        // Géométrie des bords connue seulement si toutes les colonnes sont fixes.
        if !(0..self.columns).all(|c| self.widths.get(c).copied().unwrap_or(0.0) > 0.0) {
            return None;
        }
        let base = if self.on_check.is_some() { CHECK_W + ROW_GAP } else { 0.0 };
        // Rangée à gap nul : les écarts sont matérialisés par des cales.
        let mut row = Flex::row();
        let mut consumed = 0.0f32;
        let mut edge = base;
        for c in 0..self.columns {
            edge += self.widths[c]; // bord droit de la colonne c
            if c + 1 < self.columns {
                let handle_left = edge - HANDLE_W * 0.5;
                row = row
                    .child(Spacer {
                        width: (handle_left - consumed).max(0.0),
                        height: total_h,
                    })
                    .child(ResizeHandle {
                        col: c,
                        height: total_h,
                        on_resize: on_resize.clone(),
                    });
                consumed = handle_left + HANDLE_W;
            }
            edge += ROW_GAP; // écart inter-colonnes
        }
        Some(row)
    }

    fn rebuild(&mut self) {
        let checks = self.on_check.is_some();
        let mut col = Flex::column().gap(ROW_GAP);

        // Rangée d'en-tête (si étiquettes texte, en-têtes widget, ou cases à cocher).
        let widget_headers = !self.header_widgets.is_empty();
        if !self.headers.is_empty() || widget_headers || checks {
            let mut hrow = self.new_row();
            if checks {
                hrow = hrow.child(CheckCell {
                    checked: self.all_selected(),
                    indeterminate: self.some_selected(),
                    header: true,
                    selected: false,
                    message: self.on_check_all.clone(),
                });
            }
            if widget_headers {
                // En-têtes entièrement widget : chaque cellule héberge le widget fourni
                // (fond d'en-tête, contenu centré). Tri/réordonnancement câblés par l'app.
                for (c, make) in self.header_widgets.iter().enumerate() {
                    hrow = hrow.child(WidgetCell {
                        width: self.col_width(c),
                        selected: false,
                        header: true,
                        row: 0,
                        message: None,
                        content: vec![make()],
                    });
                }
            } else {
                for (c, label) in self.headers.iter().enumerate() {
                    let sort = self.sort.filter(|(col, _)| *col == c).map(|(_, asc)| asc);
                    let message = self.on_sort.as_ref().map(|f| f(c));
                    let reorder = self.on_reorder.as_ref().map(|cb| (c, self.columns, cb.clone()));
                    let action = self
                        .header_actions
                        .get(c)
                        .and_then(|a| a.as_ref())
                        .map(|make| vec![make()])
                        .unwrap_or_default();
                    hrow = hrow.child(Cell {
                        label: label.clone(),
                        width: self.col_width(c),
                        header: true,
                        selected: false,
                        row: None,
                        icon: self.header_icons.get(c).copied().flatten(),
                        sort,
                        message,
                        reorder,
                        action,
                    });
                }
            }
            col = col.child(hrow);
        }

        // Mode virtualisé : l'en-tête épinglé au-dessus d'une **liste virtualisée** de
        // rangées de données (seules les visibles sont construites). Les paramètres
        // nécessaires à chaque rangée sont **capturés** (clones) dans la fabrique de la
        // liste, qui reste `'static` — donc pas d'accès à `self` dans la closure.
        if let Some((count, viewport_height, build)) = self.virtual_data.clone() {
            let columns = self.columns;
            let widths = self.widths.clone();
            let total_width = self.total_width;
            let selected = self.selected.clone();
            let on_select = self.on_select.clone();
            let list = List::new(count, ROW_H, move |i| {
                let cells = build(i);
                let is_selected = selected.contains(&i);
                let mut row = Flex::row().gap(ROW_GAP);
                if let Some(w) = total_width {
                    row = row.width(w);
                }
                for c in 0..columns {
                    let label = cells.get(c).cloned().unwrap_or_default();
                    row = row.child(Cell {
                        label,
                        width: col_dimension(&widths, c),
                        header: false,
                        selected: is_selected,
                        row: Some(i),
                        icon: None,
                        sort: None,
                        message: on_select.as_ref().map(|f| f(i)),
                        reorder: None,
                        action: Vec::new(),
                    });
                }
                row
            });
            let list = match total_width {
                Some(w) => list.width(w).height(viewport_height),
                None => list.height(viewport_height),
            };
            col = col.child(list);
            self.root = Box::new(col);
            return;
        }

        // Rangées de données.
        for (r, row) in self.rows.iter().enumerate() {
            let selected = self.selected.contains(&r);
            let mut drow = self.new_row();
            if checks {
                drow = drow.child(CheckCell {
                    checked: selected,
                    indeterminate: false,
                    header: false,
                    selected,
                    message: self.on_check.as_ref().map(|f| f(r)),
                });
            }
            match row {
                RowKind::Text(cells) => {
                    for (c, label) in cells.iter().enumerate() {
                        let message = self.on_select.as_ref().map(|f| f(r));
                        drow = drow.child(Cell {
                            label: label.clone(),
                            width: self.col_width(c),
                            header: false,
                            selected,
                            row: Some(r),
                            icon: None,
                            sort: None,
                            message,
                            reorder: None,
                            action: Vec::new(),
                        });
                    }
                }
                RowKind::Widgets(factories) => {
                    for (c, make) in factories.iter().enumerate() {
                        drow = drow.child(WidgetCell {
                            width: self.col_width(c),
                            selected,
                            header: false,
                            row: r,
                            message: self.on_select.as_ref().map(|f| f(r)),
                            content: vec![make()],
                        });
                    }
                }
            }
            col = col.child(drow);
        }

        // Hauteur totale de la grille (rangées + écarts) : pour caler le calque de
        // poignées, qui court sur toute la hauteur. Basée sur `ROW_H` (le plancher
        // adaptatif) : exacte pour un tableau texte — le cas du redimensionnement, où
        // les colonnes sont toutes fixes ; une rangée-widget plus haute la sous-estime.
        let header_present = !self.headers.is_empty() || widget_headers || checks;
        let n = header_present as usize + self.rows.len();
        let total_h = if n > 0 {
            n as f32 * ROW_H + (n as f32 - 1.0) * ROW_GAP
        } else {
            0.0
        };

        self.root = match self.resize_overlay(total_h) {
            Some(handles) => {
                let base = if checks { CHECK_W + ROW_GAP } else { 0.0 };
                let cols_w: f32 = (0..self.columns).map(|c| self.widths[c]).sum();
                let computed = base + cols_w + ROW_GAP * self.columns.saturating_sub(1) as f32;
                let total_w = self.total_width.unwrap_or(computed);
                Box::new(
                    Stack::new()
                        .width(total_w)
                        .height(total_h)
                        .layer(col)
                        .layer(handles),
                )
            }
            None => Box::new(col),
        };
    }
}

impl<Msg: Clone> Widget<Msg> for Table<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style(&self.root)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        Widget::<Msg>::children(&self.root)
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        // Quand le tableau est redimensionnable, la racine est une pile (grille +
        // calque de poignées) : on relaie le drapeau pour que les couches se
        // superposent au lieu de s'aligner.
        Widget::<Msg>::stack(&self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::{Point, Primitive};

    #[test]
    fn header_and_rows_produce_rows_of_cells() {
        let table = Table::<()>::new(2)
            .header(&["Nom", "Note"])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // 1 en-tête + 2 rangées de données.
        assert_eq!(Widget::<()>::children(&table).len(), 3);
        // Chaque rangée a 2 cellules.
        assert_eq!(Widget::<()>::children(&table)[0].children().len(), 2);

        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Nom") && has("Ada") && has("Bob"));
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Sort(usize),
        Select(usize),
        Check(usize),
        CheckAll,
        Resize(usize, f32),
        Reorder(usize, usize),
        Filter,
    }

    #[test]
    fn header_click_sorts_and_row_click_selects() {
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .on_select_row(Msg::Select)
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        assert_eq!(click(180.0, ROW_H * 0.5), Some(Msg::Sort(1)));
        assert_eq!(click(30.0, ROW_H * 2.5), Some(Msg::Select(1)));
    }

    #[test]
    fn checkbox_column_toggles_rows_and_all() {
        let table = Table::<Msg>::new(2)
            .width(280.0)
            .header(&["Name", "Score"])
            .checkboxes(Msg::Check, Msg::CheckAll)
            .selected(&[0])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // Chaque rangée a 3 cellules : case + 2 colonnes.
        assert_eq!(Widget::<Msg>::children(&table)[0].children().len(), 3);

        let ui = build_ui(&table, Size::new(280.0, 200.0), &Runtime::default(), &Theme::default());
        let click = |x: f32, y: f32| ui.hit(Point::new(x, y)).and_then(|id| ui.msg_for(id));
        // Case « tout cocher » dans l'en-tête (colonne de gauche).
        assert_eq!(click(CHECK_W * 0.5, ROW_H * 0.5), Some(Msg::CheckAll));
        // Case de la 2e ligne de données (r=1).
        assert_eq!(click(CHECK_W * 0.5, ROW_H * 2.5), Some(Msg::Check(1)));
    }

    #[test]
    fn select_all_is_indeterminate_on_partial_selection() {
        // Case « tout cocher » de l'en-tête = 1re cellule de la 1re rangée.
        let header_check = |sel: &[usize]| {
            let table = Table::<Msg>::new(2)
                .header(&["A", "B"])
                .checkboxes(Msg::Check, Msg::CheckAll)
                .selected(sel)
                .row(&["x", "1"])
                .row(&["y", "2"]);
            let row0 = &Widget::<Msg>::children(&table)[0];
            // Peindre la cellule pour lire son état via les primitives serait lourd ; on
            // teste plutôt les helpers directement.
            let _ = row0;
            (table.all_selected(), table.some_selected())
        };
        assert_eq!(header_check(&[]), (false, false), "rien coché");
        assert_eq!(header_check(&[0]), (false, true), "partiel → indéterminé");
        assert_eq!(header_check(&[0, 1]), (true, false), "tout coché");
    }

    #[test]
    fn only_headers_take_keyboard_focus() {
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .on_select_row(Msg::Select)
            .row(&["Ada", "5"]);
        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        // Deux en-têtes triables sont focusables ; les cellules de données ne le sont pas.
        // On compte les focusables en parcourant le cycle Tab.
        let first = ui.focus_next(None, true);
        let mut count = 0;
        let mut cur = first;
        while let Some(id) = cur {
            count += 1;
            let next = ui.focus_next(Some(id), true);
            if next == first || count > 10 {
                break;
            }
            cur = next;
        }
        assert_eq!(count, 2, "seuls les 2 en-têtes prennent le focus (got {count})");
    }

    #[test]
    fn resize_handle_emits_accumulating_delta() {
        let table = Table::<Msg>::new(2)
            .column_widths(&[100.0, 100.0])
            .header(&["A", "B"])
            .on_resize(Msg::Resize)
            .row(&["x", "y"]);
        // Racine = pile ; le calque 1 est la rangée de poignées.
        assert_eq!(Widget::<Msg>::children(&table).len(), 2);
        let overlay = &Widget::<Msg>::children(&table)[1];
        // Une seule poignée (bord entre les 2 colonnes) parmi les cales.
        let handle = overlay
            .children()
            .iter()
            .find(|c| c.draggable())
            .expect("une poignée de redimensionnement");
        // Le glissement émet un delta accumulable ; un delta nul ne fait rien.
        assert_eq!(handle.on_drag_delta(12.0), Some(Msg::Resize(0, 12.0)));
        assert_eq!(handle.on_drag_delta(0.0), None);

        // La poignée est atteignable comme draggable au bord de la 1re colonne (x≈100).
        let ui = build_ui(&table, Size::new(220.0, 120.0), &Runtime::default(), &Theme::default());
        assert!(ui.draggable_at(Point::new(100.0, 20.0)).is_some(), "poignée saisissable au bord");
        // Hors des colonnes fixes, pas d'overlay : une largeur flexible le désactive.
        let flex = Table::<Msg>::new(2).header(&["A", "B"]).on_resize(Msg::Resize).row(&["x", "y"]);
        // Grille nue (en-tête + 1 rangée), aucun calque de poignées superposé.
        assert_eq!(Widget::<Msg>::children(&flex).len(), 2, "colonnes flexibles : pas de calque de poignées");
        assert!(
            Widget::<Msg>::children(&flex).iter().all(|r| r.children().iter().all(|c| !c.draggable())),
            "aucune poignée sans largeurs fixes",
        );
    }

    #[test]
    fn reorderable_headers_expose_index_and_message() {
        let table = Table::<Msg>::new(3)
            .width(300.0)
            .header(&["A", "B", "C"])
            .on_sort(Msg::Sort)
            .on_reorder(Msg::Reorder)
            .row(&["x", "y", "z"]);
        let hrow = &Widget::<Msg>::children(&table)[0];
        let cells = hrow.children();
        // Chaque en-tête connaît sa colonne (source/cible) et produit Reorder(from, to).
        assert_eq!(cells[0].reorder_index(), Some(0));
        assert_eq!(cells[2].reorder_index(), Some(2));
        assert_eq!(cells[0].on_reorder(2), Some(Msg::Reorder(0, 2)));
        // Le clic trie toujours (tap = tri, glissé = réordonner).
        assert_eq!(cells[1].on_click(), Some(Msg::Sort(1)));
        // Les cellules de données ne sont pas réordonnables.
        let drow = &Widget::<Msg>::children(&table)[1];
        assert_eq!(drow.children()[0].reorder_index(), None);
    }

    #[test]
    fn ctrl_arrows_reorder_focused_header() {
        use crate::interaction::{Key, KeyResponse};
        let table = Table::<Msg>::new(3)
            .width(300.0)
            .header(&["A", "B", "C"])
            .on_sort(Msg::Sort)
            .on_reorder(Msg::Reorder)
            .row(&["x", "y", "z"]);
        let cells = Widget::<Msg>::children(&table)[0].children();
        let ctrl_left = Key::Left { shift: false, word: true };
        let ctrl_right = Key::Right { shift: false, word: true };
        // Colonne du milieu (1) : Ctrl+Gauche → 0, Ctrl+Droite → 2.
        assert_eq!(cells[1].on_key(&ctrl_left), KeyResponse::Handled(Some(Msg::Reorder(1, 0))));
        assert_eq!(cells[1].on_key(&ctrl_right), KeyResponse::Handled(Some(Msg::Reorder(1, 2))));
        // Aux bords : ignoré (le focus navigue). Col 0 à gauche, col 2 à droite.
        assert_eq!(cells[0].on_key(&ctrl_left), KeyResponse::Ignored);
        assert_eq!(cells[2].on_key(&ctrl_right), KeyResponse::Ignored);
        // Flèche nue (sans Ctrl) : ignorée → navigation de focus.
        assert_eq!(cells[1].on_key(&Key::Left { shift: false, word: false }), KeyResponse::Ignored);
    }

    #[test]
    fn reorderable_header_is_announced_with_position() {
        let table = Table::<Msg>::new(3)
            .width(300.0)
            .header(&["A", "B", "C"])
            .on_sort(Msg::Sort)
            .on_reorder(Msg::Reorder)
            .row(&["x", "y", "z"]);
        let sem = Widget::<Msg>::children(&table)[0].children()[1]
            .semantics()
            .expect("en-tête annoncé");
        assert_eq!(sem.label.as_deref(), Some("B"));
        assert_eq!(sem.value.as_deref(), Some("column 2 of 3"));
        // Les cellules de données ne portent pas de sémantique d'en-tête.
        assert!(Widget::<Msg>::children(&table)[1].children()[0].semantics().is_none());
    }

    #[test]
    fn widget_rows_embed_arbitrary_widgets() {
        use crate::Text;
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Tag"])
            .on_select_row(Msg::Select)
            .widget_row(vec![
                Box::new(|| Box::new(Text::new("Ada"))),
                Box::new(|| Box::new(Text::new("admin"))),
            ]);
        // En-tête + 1 rangée de 2 cellules-widgets, chacune contenant son widget.
        let rows = Widget::<Msg>::children(&table);
        assert_eq!(rows.len(), 2);
        let drow = &rows[1];
        assert_eq!(drow.children().len(), 2);
        assert_eq!(drow.children()[0].children().len(), 1, "la cellule contient un widget");

        // Le contenu (« admin ») est bien rendu, et la ligne reste sélectionnable.
        let ui = build_ui(&table, Size::new(240.0, 120.0), &Runtime::default(), &Theme::default());
        let has_admin = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "admin"));
        assert!(has_admin, "le widget de cellule est peint");
        let click = ui.hit(Point::new(30.0, ROW_H * 1.5)).and_then(|id| ui.msg_for(id));
        assert_eq!(click, Some(Msg::Select(0)), "la ligne-widget est sélectionnable");
    }

    #[test]
    fn header_icon_shifts_label_and_paints() {
        // Une icône de tête recule le libellé de l'en-tête (icône + texte).
        let name_x = |icons: bool| {
            let mut t = Table::<Msg>::new(2).width(240.0).header(&["Name", "Score"]);
            if icons {
                t = t.header_icons(&[Some(IconName::Star), None]);
            }
            let ui = build_ui(&t, Size::new(240.0, 100.0), &Runtime::default(), &Theme::default());
            ui.scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Text { text, position, .. } if text == "Name" => Some(position.x),
                    _ => None,
                })
                .unwrap()
        };
        let (plain, iconed) = (name_x(false), name_x(true));
        assert!(iconed >= plain + ICON, "libellé décalé derrière l'icône : {iconed} vs {plain}");
        // La colonne sans icône (« Score ») n'est pas décalée.
    }

    #[test]
    fn sort_and_selection_are_announced() {
        // En-tête triable : le tri résultant énonce le sens basculé.
        let unsorted = Table::<Msg>::new(2).header(&["Name", "Score"]).on_sort(Msg::Sort);
        let head = |t: &Table<Msg>, c: usize| Widget::<Msg>::children(t)[0].children()[c].announce();
        assert_eq!(head(&unsorted, 0).as_deref(), Some("Sorted by Name ascending"));
        // Déjà croissant → un clic passe en décroissant.
        let asc = Table::<Msg>::new(2).header(&["Name", "Score"]).on_sort(Msg::Sort).sorted(0, true);
        assert_eq!(head(&asc, 0).as_deref(), Some("Sorted by Name descending"));

        // Cases à cocher : l'état résultant de la bascule.
        let table = Table::<Msg>::new(2)
            .header(&["Name", "Score"])
            .checkboxes(Msg::Check, Msg::CheckAll)
            .selected(&[0])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // Case « tout cocher » (en-tête, partiel) → cocher toutes.
        let all = Widget::<Msg>::children(&table)[0].children()[0].announce();
        assert_eq!(all.as_deref(), Some("All rows selected"));
        // Ligne 0 (cochée) → décocher ; ligne 1 (décochée) → cocher.
        let row0 = Widget::<Msg>::children(&table)[1].children()[0].announce();
        let row1 = Widget::<Msg>::children(&table)[2].children()[0].announce();
        assert_eq!(row0.as_deref(), Some("Row deselected"));
        assert_eq!(row1.as_deref(), Some("Row selected"));
    }

    #[test]
    fn header_action_widget_captures_its_click() {
        use frus_core::Color;
        // Un bouton d'action posé dans l'en-tête, cliquable indépendamment du tri.
        struct Btn;
        impl Widget<Msg> for Btn {
            fn style(&self) -> Style {
                Style {
                    width: Dimension::Length(24.0),
                    height: Dimension::Length(24.0),
                    ..Default::default()
                }
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                &[]
            }
            fn paint(&self, bounds: Rect, _s: Status, t: &Theme, scene: &mut Scene) {
                scene.draw_rect(bounds, t.primary, 4.0, 0.0, Color::TRANSPARENT);
            }
            fn on_click(&self) -> Option<Msg> {
                Some(Msg::Filter)
            }
        }
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .header_action(1, || Box::new(Btn));
        // La cellule d'en-tête de la colonne 1 porte le widget d'action.
        assert_eq!(
            Widget::<Msg>::children(&table)[0].children()[1].children().len(),
            1,
            "l'en-tête colonne 1 porte l'action",
        );
        let ui = build_ui(&table, Size::new(240.0, 100.0), &Runtime::default(), &Theme::default());
        let click = |x: f32| ui.hit(Point::new(x, ROW_H * 0.5)).and_then(|id| ui.msg_for(id));
        // Clic à droite (sur le bouton, centré ~218) → action ; ailleurs → tri.
        assert_eq!(click(218.0), Some(Msg::Filter), "le bouton capte son clic");
        assert_eq!(click(130.0), Some(Msg::Sort(1)), "le reste de l'en-tête trie");
    }

    #[test]
    fn row_click_selection_is_announced() {
        // Cliquer une ligne (texte ou widget) énonce l'état résultant, avec le numéro.
        let table = Table::<Msg>::new(2)
            .header(&["Name", "Tag"])
            .on_select_row(Msg::Select)
            .selected(&[1])
            .row(&["Ada", "admin"])
            .widget_row(vec![
                Box::new(|| Box::new(crate::Text::new("Bob"))),
                Box::new(|| Box::new(crate::Text::new("editor"))),
            ]);
        let rows = Widget::<Msg>::children(&table);
        // Ligne texte 0 (non sélectionnée) → « selected » ; cellule quelconque de la ligne.
        assert_eq!(rows[1].children()[0].announce().as_deref(), Some("Row 1 selected"));
        // Ligne widget 1 (sélectionnée) → « deselected ».
        assert_eq!(rows[2].children()[0].announce().as_deref(), Some("Row 2 deselected"));
        // Sans sélection possible, aucune annonce.
        let plain = Table::<Msg>::new(1).header(&["N"]).row(&["x"]);
        assert_eq!(Widget::<Msg>::children(&plain)[1].children()[0].announce(), None);
    }

    #[test]
    fn widget_row_grows_to_tall_content() {
        use frus_core::Color;
        // Un widget plus haut que ROW_H : la rangée s'adapte au lieu de le rogner.
        struct Tall(f32);
        impl Widget<Msg> for Tall {
            fn style(&self) -> Style {
                Style {
                    width: Dimension::Length(40.0),
                    height: Dimension::Length(self.0),
                    ..Default::default()
                }
            }
            fn children(&self) -> &[Box<dyn Widget<Msg>>] {
                &[]
            }
            fn paint(&self, bounds: Rect, _s: Status, _t: &Theme, scene: &mut Scene) {
                scene.fill_rect(bounds, Color::rgb(1.0, 0.0, 0.0));
            }
            fn on_click(&self) -> Option<Msg> {
                None
            }
        }
        let tall = 60.0;
        let table = Table::<Msg>::new(2).width(240.0).header(&["A", "B"]).widget_row(vec![
            Box::new(move || Box::new(Tall(tall))),
            Box::new(|| Box::new(crate::Text::new("x"))),
        ]);
        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        // La barre rouge (le widget haut) est peinte à sa pleine hauteur : la cellule,
        // et donc la rangée, l'ont suivie (pas de rognage à ROW_H).
        let bar_h = ui.scene().primitives().iter().find_map(|p| match p {
            Primitive::Rect { rect, color, .. } if color.r > 0.9 && color.g < 0.1 => Some(rect.height),
            _ => None,
        });
        assert!(bar_h.unwrap_or(0.0) >= tall - 1.0, "la rangée suit le contenu haut: {bar_h:?}");
        assert!(tall - 1.0 > ROW_H, "le contenu dépasse bien la hauteur nominale");
    }

    #[test]
    fn header_action_menu_opens_as_column_menu() {
        use crate::{Button, Menu};
        // Un Menu déposé en widget d'action d'en-tête = un menu de colonne : son overlay
        // flottant est collecté **même imbriqué** dans l'en-tête, et se ferme (Échap / clic
        // extérieur). Les items sont focusables → navigables au clavier.
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .on_sort(Msg::Sort)
            .header_action(1, || {
                Box::new(
                    Menu::new(Button::new("...").on_press(Msg::Filter), true, Msg::CheckAll)
                        .item("Sort ascending", Msg::Sort(1))
                        .item("Sort descending", Msg::Sort(1)),
                )
            });
        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        // L'overlay du menu (imbriqué) est bien collecté → fermable.
        assert_eq!(ui.top_dismiss(), Some(Msg::CheckAll), "l'overlay du menu de colonne est collecté");
        // Le menu flotte et se peint (ses items sont rendus par-dessus la grille).
        let painted = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(painted("Sort ascending"), "le menu de colonne flotte et se peint");
    }

    #[test]
    fn widget_header_hosts_arbitrary_header_widgets() {
        use crate::{Button, Text};
        let table = Table::<Msg>::new(2).width(240.0).widget_header(vec![
            Box::new(|| Box::new(Text::new("Name"))),
            Box::new(|| Box::new(Button::new("Sort").on_press(Msg::Sort(1)))),
        ]).row(&["Ada", "5"]);
        // En-tête = 1re rangée : 2 cellules-widgets, chacune hébergeant son widget.
        let rows = Widget::<Msg>::children(&table);
        assert_eq!(rows[0].children().len(), 2);
        assert_eq!(rows[0].children()[0].children().len(), 1, "l'en-tête widget contient son widget");

        let ui = build_ui(&table, Size::new(240.0, 120.0), &Runtime::default(), &Theme::default());
        // Le libellé « Name » (widget d'en-tête) est peint.
        let painted = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(painted("Name") && painted("Sort"), "les widgets d'en-tête sont peints");
        // Le bouton d'en-tête maison émet **son** message (pas de tri automatique).
        let click = ui.hit(Point::new(180.0, ROW_H * 0.5)).and_then(|id| ui.msg_for(id));
        assert_eq!(click, Some(Msg::Sort(1)), "l'app câble le tri dans son widget d'en-tête");
    }

    #[test]
    fn virtual_table_builds_only_visible_rows() {
        use std::cell::Cell as StdCell;
        use std::rc::Rc as StdRc;
        let built = StdRc::new(StdCell::new(0usize));
        let counter = built.clone();
        let table = Table::<Msg>::new(2)
            .width(200.0)
            .header(&["Name", "Score"])
            .on_select_row(Msg::Select)
            .virtual_rows(5000, 200.0, move |i| {
                counter.set(counter.get() + 1);
                vec![format!("R{i}"), format!("{i}")]
            });
        let ui = build_ui(&table, Size::new(200.0, 260.0), &Runtime::default(), &Theme::default());
        // Viewport 200 / ROW_H ≈ 6 lignes visibles (+ marge) — jamais 5000.
        assert!(built.get() < 20, "seules les lignes visibles sont construites : {}", built.get());
        assert!(built.get() >= 5, "au moins la fenêtre visible : {}", built.get());
        // En-tête **épinglé** + première ligne visible peints.
        let has = |t: &str| {
            ui.scene()
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has("Name"), "en-tête épinglé au-dessus de la liste");
        assert!(has("R0"), "première ligne virtualisée construite");
        // Le défilement couvre tout le contenu (5000 × ROW_H − viewport).
        let maxes = ui.scrollable_maxes();
        assert_eq!(maxes.len(), 1, "un viewport défilable");
        assert_eq!(maxes[0].2, 5000.0 * ROW_H - 200.0, "borne de défilement = contenu total");
        // Une ligne visible reste cliquable (sélection).
        let click = ui.hit(Point::new(20.0, ROW_H + 15.0)).and_then(|id| ui.msg_for(id));
        assert!(matches!(click, Some(Msg::Select(_))), "ligne virtualisée cliquable : {click:?}");
    }

    #[test]
    fn fixed_column_width_is_applied() {
        let table = Table::<()>::new(2)
            .width(300.0)
            .column_widths(&[80.0]) // 1re colonne fixe à 80, 2e flexible
            .header(&["A", "B"])
            .row(&["x", "y"]);
        let ui = build_ui(&table, Size::new(300.0, 100.0), &Runtime::default(), &Theme::default());
        // La 1re colonne d'en-tête ("A") occupe 80 px : "B" démarre au-delà de 80 + gap.
        let text_x = |t: &str| {
            ui.scene().primitives().iter().find_map(|p| match p {
                Primitive::Text { text, position, .. } if text == t => Some(position.x),
                _ => None,
            })
        };
        let (ax, bx) = (text_x("A").unwrap(), text_x("B").unwrap());
        assert!(bx >= ax + 80.0, "colonne fixe de 80 : bx={bx} ax={ax}");
    }
}
