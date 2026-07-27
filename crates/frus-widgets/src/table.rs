//! [`Table`] : un tableau de données texte, bâti sur [`crate::Grid`]. En-tête stylé
//! (colonnes **triables** au clic, avec indicateur de sens) et lignes **sélectionnables**
//! (surlignées, cliquables) ; colonnes égales.
//!
//! Comme le reste de frus, le tri et la sélection sont **décidés par l'application** :
//! le tableau émet un message au clic (`on_sort(colonne)` / `on_select_row(ligne)`) et
//! n'affiche que l'état qu'on lui passe (`sorted`, `selected`). Il ne trie ni ne
//! mémorise rien lui-même.

use frus_core::{Color, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::grid::Grid;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const ROW_H: f32 = 34.0;
const PAD_X: f32 = 10.0;
const SIZE: f32 = 15.0;

/// Une cellule (en-tête ou donnée), thémée au rendu.
struct Cell<Msg> {
    label: String,
    /// Cellule d'en-tête (fond teinté, texte discret, indicateur de tri éventuel).
    header: bool,
    /// Ligne sélectionnée (fond surligné). Ignoré pour l'en-tête.
    selected: bool,
    /// Indicateur de tri de la colonne : `Some(true)` = croissant (▲), `Some(false)` =
    /// décroissant (▼), `None` = colonne non triée. En-tête seulement.
    sort: Option<bool>,
    /// Message au clic (tri pour l'en-tête, sélection pour une ligne).
    message: Option<Msg>,
}

impl<Msg: Clone> Widget<Msg> for Cell<Msg> {
    fn style(&self) -> Style {
        Style {
            height: Dimension::Length(ROW_H),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let clickable = self.message.is_some();
        // Fond : en-tête teinté ; ligne sélectionnée surlignée ; survol des cellules
        // cliquables par-dessus (couche d'état). Sinon, transparent (fond du tableau).
        let base = if self.header {
            theme.surface.lerp(theme.on_surface, 0.06)
        } else if self.selected {
            theme.surface.lerp(theme.primary, 0.16)
        } else {
            theme.surface
        };
        let bg = if clickable {
            theme.state_layer(base, theme.on_surface, &status)
        } else {
            base
        };
        if self.header || self.selected || bg != theme.surface {
            scene.draw_rect(bounds, bg.fade(o), theme.radius, 0.0, Color::TRANSPARENT);
        }

        let color = if self.header { theme.muted } else { theme.on_surface };
        let ty = bounds.y + (ROW_H - frus_text::line_height(SIZE)) * 0.5;
        scene.text(Point::new(bounds.x + PAD_X, ty), self.label.clone(), SIZE, color.fade(o));

        // Indicateur de tri : petit triangle après le libellé de l'en-tête trié.
        if self.header {
            if let Some(ascending) = self.sort {
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
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }
}

/// Un tableau à `columns` colonnes égales, avec en-tête triable et lignes
/// sélectionnables (voir le module).
pub struct Table<Msg> {
    columns: usize,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    width: Option<f32>,
    /// Colonne triée et son sens (`true` = croissant), pour l'indicateur d'en-tête.
    sort: Option<(usize, bool)>,
    /// Indices des lignes sélectionnées (surlignées).
    selected: Vec<usize>,
    on_sort: Option<Box<dyn Fn(usize) -> Msg>>,
    on_select: Option<Box<dyn Fn(usize) -> Msg>>,
    grid: Grid<Msg>,
}

impl<Msg: Clone + 'static> Table<Msg> {
    /// Crée un tableau de `columns` colonnes.
    pub fn new(columns: usize) -> Self {
        let columns = columns.max(1);
        Self {
            columns,
            headers: Vec::new(),
            rows: Vec::new(),
            width: None,
            sort: None,
            selected: Vec::new(),
            on_sort: None,
            on_select: None,
            grid: Grid::new(columns).gap(2.0),
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

    /// Fixe la largeur du tableau, en pixels logiques.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self.rebuild();
        self
    }

    /// Rend les en-têtes **cliquables** : `on_sort(colonne)` est émis au clic sur un
    /// en-tête. À l'application de trier les lignes et de rappeler [`sorted`](Self::sorted).
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

    /// Rend les lignes **cliquables** : `on_select_row(ligne)` est émis au clic.
    pub fn on_select_row(mut self, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(on_select));
        self.rebuild();
        self
    }

    /// Indique les lignes sélectionnées (surlignées).
    pub fn selected(mut self, rows: &[usize]) -> Self {
        self.selected = rows.to_vec();
        self.rebuild();
        self
    }

    /// Régénère la grille à partir des données et de l'état courants. Appelé après
    /// chaque réglage : l'ordre des appels n'importe pas, l'état final est cohérent.
    fn rebuild(&mut self) {
        let mut grid = Grid::new(self.columns).gap(2.0);
        if let Some(width) = self.width {
            grid = grid.width(width);
        }
        for (c, label) in self.headers.iter().enumerate() {
            let sort = self.sort.filter(|(col, _)| *col == c).map(|(_, asc)| asc);
            let message = self.on_sort.as_ref().map(|f| f(c));
            grid = grid.cell(Cell {
                label: label.clone(),
                header: true,
                selected: false,
                sort,
                message,
            });
        }
        for (r, row) in self.rows.iter().enumerate() {
            let selected = self.selected.contains(&r);
            for label in row {
                let message = self.on_select.as_ref().map(|f| f(r));
                grid = grid.cell(Cell {
                    label: label.clone(),
                    header: false,
                    selected,
                    sort: None,
                    message,
                });
            }
        }
        self.grid = grid;
    }
}

impl<Msg: Clone> Widget<Msg> for Table<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style(&self.grid)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        Widget::<Msg>::children(&self.grid)
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
    fn header_and_rows_produce_cells() {
        let table = Table::<()>::new(2)
            .header(&["Nom", "Note"])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        // 2 colonnes × (1 en-tête + 2 lignes) = 6 cellules.
        assert_eq!(Widget::<()>::children(&table).len(), 6);

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
        // Colonne 1 de l'en-tête (première ligne) → tri de la colonne 1.
        assert_eq!(click(180.0, ROW_H * 0.5), Some(Msg::Sort(1)));
        // Deuxième ligne de données (r=1) : y au-delà de l'en-tête + 1re ligne.
        assert_eq!(click(30.0, ROW_H * 2.5), Some(Msg::Select(1)));
    }

    #[test]
    fn sort_indicator_and_selection_are_painted() {
        let table = Table::<Msg>::new(2)
            .width(240.0)
            .header(&["Name", "Score"])
            .sorted(0, true)
            .selected(&[1])
            .row(&["Ada", "5"])
            .row(&["Bob", "3"]);
        let ui = build_ui(&table, Size::new(240.0, 200.0), &Runtime::default(), &Theme::default());
        // Le tri dessine un triangle (chemin rempli).
        let has_path = ui
            .scene()
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Path { .. }));
        assert!(has_path, "l'indicateur de tri est un triangle");
        // La ligne sélectionnée est surlignée : un rect teinté primary existe.
        let theme = Theme::default();
        let sel = theme.surface.lerp(theme.primary, 0.16);
        let has_sel = ui.scene().primitives().iter().any(|p| matches!(
            p,
            Primitive::Rect { color, .. } if color.fade(1.0) == sel.fade(1.0)
        ));
        assert!(has_sel, "la ligne sélectionnée est surlignée");
    }
}
