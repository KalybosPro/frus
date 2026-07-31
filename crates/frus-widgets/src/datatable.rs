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

use crate::flex::Flex;
use crate::interaction::Status;
use crate::pagination::Pagination;
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

/// Nombre de pages pour `len` lignes découpées par tranches de `per_page` (au moins **1**).
pub fn page_count(len: usize, per_page: usize) -> usize {
    let per = per_page.max(1);
    (len.div_ceil(per)).max(1)
}

/// La **tranche** de lignes de la page `current` (1-indexée) de taille `per_page`. La page est
/// ramenée dans `[1, page_count]` si elle déborde. Réutilisable hors widget.
pub fn page_rows(rows: &[Vec<String>], current: usize, per_page: usize) -> Vec<Vec<String>> {
    let per = per_page.max(1);
    let current = current.clamp(1, page_count(rows.len(), per));
    let start = (current - 1) * per;
    let end = (start + per).min(rows.len());
    rows[start..end].to_vec()
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
    /// Pagination `(page courante 1-indexée, taille de page)` — jalon 233.
    page: Option<(usize, usize)>,
    on_page: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// Rendu : le `Table` (lignes triées/paginées), éventuellement coiffé d'un `Pagination` dessous.
    inner: Box<dyn Widget<Msg>>,
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
            page: None,
            on_page: None,
            inner: Box::new(Flex::<Msg>::column()),
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

    /// **Pagine** le tableau : n'affiche que la tranche de la page `current` (1-indexée) de taille
    /// `per_page`, et pose un sélecteur [`Pagination`](crate::Pagination) dessous. `on_page(page)`
    /// au clic sur une page (l'application met à jour `current`). Le découpage suit le **tri**.
    pub fn paginated(
        mut self,
        current: usize,
        per_page: usize,
        on_page: impl Fn(usize) -> Msg + 'static,
    ) -> Self {
        self.page = Some((current.max(1), per_page.max(1)));
        self.on_page = Some(Rc::new(on_page));
        self.rebuild();
        self
    }

    /// (Re)construit le rendu : lignes triées selon `sort`, découpées à la page `page` le cas
    /// échéant, dans un `Table` (en-têtes, largeurs, indicateur) éventuellement coiffé d'un
    /// `Pagination` dessous.
    fn rebuild(&mut self) {
        let sorted = match self.sort {
            Some((col, asc)) => sort_rows(&self.rows, col, asc),
            None => self.rows.clone(),
        };
        // Découpe la page sur les lignes **déjà triées** (le cas échéant).
        let display = match self.page {
            Some((current, per)) => page_rows(&sorted, current, per),
            None => sorted.clone(),
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
        // Table seul, ou table + sélecteur de page (calculé sur le nombre **total** de lignes triées).
        self.inner = match (self.page, &self.on_page) {
            (Some((current, per)), Some(on_page)) => {
                let pages = page_count(sorted.len(), per);
                let current = current.clamp(1, pages);
                let on_page = on_page.clone();
                let pager = Pagination::new(current, pages, move |p| on_page(p));
                Box::new(Flex::column().gap(12.0).child(t).child(pager))
            }
            _ => Box::new(t),
        };
    }
}

impl<Msg: Clone> Widget<Msg> for DataTable<Msg> {
    fn style(&self) -> Style {
        self.inner.style()
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.inner.children()
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn stack(&self) -> bool {
        self.inner.stack()
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

    #[test]
    fn pagination_slices_rows_and_counts_pages() {
        let rows: Vec<Vec<String>> = (1..=7).map(|i| vec![i.to_string()]).collect();
        assert_eq!(page_count(7, 3), 3, "7 lignes / 3 = 3 pages");
        assert_eq!(page_count(0, 3), 1, "au moins une page");
        let col0 = |rs: Vec<Vec<String>>| rs.iter().map(|r| r[0].clone()).collect::<Vec<_>>();
        // Page 1 : 1,2,3 ; page 3 : 7 (dernière, partielle).
        assert_eq!(col0(page_rows(&rows, 1, 3)), ["1", "2", "3"]);
        assert_eq!(col0(page_rows(&rows, 3, 3)), ["7"]);
        // Page hors bornes ramenée dans l'intervalle.
        assert_eq!(col0(page_rows(&rows, 99, 3)), ["7"]);
    }

    #[test]
    fn data_table_with_pagination_builds_table_and_pager() {
        let rows: Vec<Vec<String>> =
            (1..=10).map(|i| vec![format!("R{i}"), i.to_string()]).collect();
        let dt = DataTable::<()>::new(["Name", "Score"], rows)
            .sorted(1, true)
            .paginated(1, 4, |_| ());
        // inner = colonne [table, pagination] → deux enfants.
        assert_eq!(Widget::<()>::children(&dt).len(), 2, "table + sélecteur de page");
    }
}
