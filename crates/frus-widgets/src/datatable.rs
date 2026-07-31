//! [`DataTable`] : un [`Table`](crate::Table) qui **trie ses propres données**.
//!
//! Le `Table` de base est purement contrôlé : il n'émet que la colonne cliquée (`on_sort`) et
//! affiche l'indicateur de l'état `sorted` qu'on lui passe — c'est l'application qui réordonne ses
//! lignes. `DataTable` encapsule ce réordonnancement d'**affichage** : on lui donne les lignes
//! brutes et l'état de tri `(colonne, sens)`, il reconstruit un `Table` avec les lignes déjà
//! **triées** (tri numérique-aware, insensible à la casse) et l'indicateur. Le modèle reste
//! contrôlé — l'état de tri vit dans l'app —, mais la logique de tri n'est plus recopiée à la main.

use std::cmp::Ordering;
use std::rc::Rc;

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::table::Table;
use crate::theme::Theme;
use crate::widget::Widget;

/// Compare deux cellules texte : **numériquement** si les deux se lisent comme des nombres, sinon
/// lexicalement en **insensible à la casse**. Base du tri d'un [`DataTable`].
pub fn compare_cells(a: &str, b: &str) -> Ordering {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

/// Renvoie une **copie** de `rows` triée par la colonne `col` (`ascending` = croissant) via
/// [`compare_cells`]. Une cellule absente (ligne trop courte) compte comme vide. Réutilisable hors
/// widget : un reducer peut trier ses données exactement de la même façon.
pub fn sort_rows(rows: &[Vec<String>], col: usize, ascending: bool) -> Vec<Vec<String>> {
    let empty = String::new();
    let mut out = rows.to_vec();
    out.sort_by(|a, b| {
        let ord = compare_cells(a.get(col).unwrap_or(&empty), b.get(col).unwrap_or(&empty));
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
    out
}

/// Un tableau de données **texte** qui trie ses propres lignes selon l'état de tri fourni, puis
/// délègue le rendu à un [`Table`](crate::Table).
///
/// ```
/// use frus_widgets::DataTable;
/// let rows = vec![vec!["Bob".to_string(), "9".to_string()], vec!["Ada".to_string(), "10".to_string()]];
/// let table: DataTable = DataTable::new(["Name", "Score"], rows).sorted(1, true);
/// ```
pub struct DataTable<Msg = ()> {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    widths: Vec<f32>,
    sort: Option<(usize, bool)>,
    on_sort: Option<Rc<dyn Fn(usize) -> Msg>>,
    inner: Table<Msg>,
}

impl<Msg: Clone + 'static> DataTable<Msg> {
    /// Crée un tableau depuis ses **en-têtes** et ses **lignes** (texte). Sans état de tri, les
    /// lignes sont affichées dans l'ordre fourni.
    pub fn new(
        headers: impl IntoIterator<Item = impl Into<String>>,
        rows: impl IntoIterator<Item = Vec<String>>,
    ) -> Self {
        let headers: Vec<String> = headers.into_iter().map(Into::into).collect();
        let cols = headers.len().max(1);
        let mut me = Self {
            headers,
            rows: rows.into_iter().collect(),
            widths: vec![0.0; cols],
            sort: None,
            on_sort: None,
            inner: Table::new(cols),
        };
        me.rebuild();
        me
    }

    /// Largeur **fixe** de chaque colonne, en pixels (`0` ou moins = colonne flexible).
    pub fn column_widths(mut self, widths: &[f32]) -> Self {
        for (i, w) in widths.iter().enumerate().take(self.widths.len()) {
            self.widths[i] = *w;
        }
        self.rebuild();
        self
    }

    /// Colonne triée et sens (`true` = croissant) : **trie** les lignes pour l'affichage et montre
    /// l'indicateur de sens.
    pub fn sorted(mut self, column: usize, ascending: bool) -> Self {
        self.sort = Some((column, ascending));
        self.rebuild();
        self
    }

    /// Rend les en-têtes **cliquables** : `on_sort(colonne)` au clic (l'application bascule alors le
    /// sens et repasse `sorted(...)`).
    pub fn on_sort(mut self, on_sort: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_sort = Some(Rc::new(on_sort));
        self.rebuild();
        self
    }

    /// (Re)construit le `Table` interne : lignes triées selon `sort`, en-têtes, largeurs, indicateur.
    fn rebuild(&mut self) {
        let display = match self.sort {
            Some((col, asc)) => sort_rows(&self.rows, col, asc),
            None => self.rows.clone(),
        };
        let hrefs: Vec<&str> = self.headers.iter().map(|s| s.as_str()).collect();
        let mut t = Table::new(self.headers.len().max(1)).header(&hrefs);
        for row in &display {
            let refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
            t = t.row(&refs);
        }
        if self.widths.iter().any(|w| *w > 0.0) {
            t = t.column_widths(&self.widths);
        }
        if let Some(f) = &self.on_sort {
            let f = f.clone();
            t = t.on_sort(move |c| f(c));
        }
        if let Some((col, asc)) = self.sort {
            t = t.sorted(col, asc);
        }
        self.inner = t;
    }
}

impl<Msg: Clone> Widget<Msg> for DataTable<Msg> {
    fn style(&self) -> Style {
        Widget::<Msg>::style(&self.inner)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        Widget::<Msg>::children(&self.inner)
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        Widget::<Msg>::stack(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Vec<String>> {
        vec![
            vec!["Bob".to_string(), "9".to_string()],
            vec!["alice".to_string(), "10".to_string()],
            vec!["Carol".to_string(), "2".to_string()],
        ]
    }

    #[test]
    fn sort_rows_is_numeric_aware_and_case_insensitive() {
        let rows = sample();
        // Colonne 1 **numérique** : 2 < 9 < 10 (et non le tri lexical "10" < "2" < "9").
        let by_num = sort_rows(&rows, 1, true);
        assert_eq!(by_num.iter().map(|r| r[1].as_str()).collect::<Vec<_>>(), ["2", "9", "10"]);
        // Colonne 0 **texte**, insensible à la casse : alice < Bob < Carol.
        let by_name = sort_rows(&rows, 0, true);
        assert_eq!(by_name.iter().map(|r| r[0].as_str()).collect::<Vec<_>>(), ["alice", "Bob", "Carol"]);
        // Sens **décroissant** : ordre inversé.
        let desc = sort_rows(&rows, 1, false);
        assert_eq!(desc.iter().map(|r| r[1].as_str()).collect::<Vec<_>>(), ["10", "9", "2"]);
    }

    #[test]
    fn compare_cells_prefers_numbers_then_text() {
        assert_eq!(compare_cells("2", "10"), Ordering::Less, "numérique : 2 < 10");
        assert_eq!(compare_cells("Bob", "alice"), Ordering::Greater, "texte : b > a (insensible casse)");
    }

    #[test]
    fn data_table_builds_a_non_empty_tree() {
        let dt = DataTable::<()>::new(["Name", "Score"], sample())
            .column_widths(&[120.0, 80.0])
            .sorted(1, true)
            .on_sort(|_| ());
        // En-tête + lignes → arbre de rendu non vide.
        assert!(!Widget::<()>::children(&dt).is_empty(), "le DataTable produit un arbre");
    }
}
