//! [`Table`] : un tableau de données texte à **rangées `Flex`** (colonnes de largeur
//! **fixe ou flexible**). En-tête **triable** au clic (indicateur de sens), lignes
//! **sélectionnables**, et **sélection multiple** optionnelle via une colonne de cases à
//! cocher coiffée d'un « tout cocher ».
//!
//! Comme le reste de frus, tri et sélection sont **décidés par l'application** : le
//! tableau émet un message au clic (`on_sort`, `on_select_row`, `on_check`,
//! `on_check_all`) et n'affiche que l'état qu'on lui passe (`sorted`, `selected`).

use frus_core::{Color, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::flex::Flex;
use crate::icons::IconName;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 34.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 15.0;
/// Largeur de la colonne des cases à cocher (sélection multiple).
const CHECK_W: f32 = 40.0;
/// Côté de la case à cocher dessinée.
const BOX: f32 = 18.0;

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

/// Style d'une cellule : largeur de colonne (fixe ou flexible) et hauteur de rangée.
fn cell_style(width: Dimension) -> Style {
    let flex_grow = if matches!(width, Dimension::Length(_)) { 0.0 } else { 1.0 };
    Style {
        width,
        height: Dimension::Length(ROW_H),
        flex_grow,
        ..Default::default()
    }
}

/// Une cellule texte (en-tête ou donnée), thémée au rendu.
struct Cell<Msg> {
    label: String,
    width: Dimension,
    header: bool,
    selected: bool,
    /// Indicateur de tri de l'en-tête : `Some(true)` = ▲, `Some(false)` = ▼.
    sort: Option<bool>,
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Cell<Msg> {
    fn style(&self) -> Style {
        cell_style(self.width)
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

        let color = if self.header { theme.muted } else { theme.on_surface };
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(Point::new(bounds.x + PAD_X, ty), self.label.clone(), SIZE, color.fade(o));

        if let (true, Some(ascending)) = (self.header, self.sort) {
            let lw = frus_text::measure(&self.label, SIZE).width;
            let cx = bounds.x + PAD_X + lw + 8.0;
            let cy = bounds.y + ROW_H * 0.5;
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
        let by = bounds.y + (ROW_H - BOX) * 0.5;
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
}

/// Un tableau de données à colonnes fixes ou flexibles (voir le module).
pub struct Table<Msg> {
    columns: usize,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    /// Largeur par colonne : `> 0` = fixe (px), `<= 0` = flexible (part égale).
    widths: Vec<f32>,
    total_width: Option<f32>,
    sort: Option<(usize, bool)>,
    selected: Vec<usize>,
    on_sort: Option<Box<dyn Fn(usize) -> Msg>>,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    on_check: Option<Box<dyn Fn(usize) -> Msg>>,
    on_check_all: Option<Msg>,
    root: Flex<Msg>,
}

impl<Msg: Clone + 'static> Table<Msg> {
    /// Crée un tableau de `columns` colonnes (flexibles, largeurs égales par défaut).
    pub fn new(columns: usize) -> Self {
        let columns = columns.max(1);
        Self {
            columns,
            headers: Vec::new(),
            rows: Vec::new(),
            widths: vec![0.0; columns],
            total_width: None,
            sort: None,
            selected: Vec::new(),
            on_sort: None,
            on_select: None,
            on_check: None,
            on_check_all: None,
            root: Flex::column().gap(2.0),
        }
    }

    /// Définit la ligne d'en-tête (une étiquette par colonne).
    pub fn header(mut self, labels: &[&str]) -> Self {
        self.headers = labels.iter().map(|s| s.to_string()).collect();
        self.rebuild();
        self
    }

    /// Ajoute une ligne de données (une valeur par colonne).
    pub fn row(mut self, cells: &[&str]) -> Self {
        self.rows.push(cells.iter().map(|s| s.to_string()).collect());
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
        self.on_select = Some(Box::new(on_select));
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

    /// Indique les lignes sélectionnées (surlignées, cases cochées).
    pub fn selected(mut self, rows: &[usize]) -> Self {
        self.selected = rows.to_vec();
        self.rebuild();
        self
    }

    /// Dimension de la colonne `c` : fixe si `widths[c] > 0`, flexible sinon.
    fn col_width(&self, c: usize) -> Dimension {
        match self.widths.get(c).copied().unwrap_or(0.0) {
            w if w > 0.0 => Dimension::Length(w),
            _ => Dimension::Auto,
        }
    }

    /// Une rangée `Flex`, à la largeur totale du tableau si fixée.
    fn new_row(&self) -> Flex<Msg> {
        let row = Flex::row().gap(2.0);
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
    fn rebuild(&mut self) {
        let checks = self.on_check.is_some();
        let mut col = Flex::column().gap(2.0);

        // Rangée d'en-tête (si étiquettes ou cases à cocher).
        if !self.headers.is_empty() || checks {
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
            for (c, label) in self.headers.iter().enumerate() {
                let sort = self.sort.filter(|(col, _)| *col == c).map(|(_, asc)| asc);
                let message = self.on_sort.as_ref().map(|f| f(c));
                hrow = hrow.child(Cell {
                    label: label.clone(),
                    width: self.col_width(c),
                    header: true,
                    selected: false,
                    sort,
                    message,
                });
            }
            col = col.child(hrow);
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
            for (c, label) in row.iter().enumerate() {
                let message = self.on_select.as_ref().map(|f| f(r));
                drow = drow.child(Cell {
                    label: label.clone(),
                    width: self.col_width(c),
                    header: false,
                    selected,
                    sort: None,
                    message,
                });
            }
            col = col.child(drow);
        }

        self.root = col;
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
