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
use frus_layout::{Align, Style};

use crate::flex::Flex;
use crate::interaction::Status;
use crate::pagination::Pagination;
use crate::segmented::SegmentedControl;
use crate::table::Table;
use crate::text::Text;
use crate::textinput::TextInput;
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

/// `true` si une cellule de `row` **contient** `query` (sous-chaîne **insensible à la casse**). Une
/// requête vide ou blanche laisse tout passer. Base du filtre d'un [`DataTable`] ; réutilisable hors
/// widget (un reducer peut filtrer ses données de la même façon).
pub fn row_matches(row: &[String], query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    row.iter().any(|cell| cell.to_lowercase().contains(&q))
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

/// Libellé « N–M of T » de la tranche courante (`0 of 0` si vide) — jalon 236. Réutilisable.
pub fn page_range_label(current: usize, per_page: usize, total: usize) -> String {
    if total == 0 {
        return "0 of 0".to_string();
    }
    let per = per_page.max(1);
    let current = current.clamp(1, page_count(total, per));
    let start = (current - 1) * per + 1;
    let end = (current * per).min(total);
    format!("{start}\u{2013}{end} of {total}")
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
    /// Tailles de page proposées + rappel au changement (sélecteur du pied) — jalon 236.
    page_sizes: Vec<usize>,
    on_page_size: Option<Rc<dyn Fn(usize) -> Msg>>,
    /// Sélection de ligne (jalon 239) : rappel au clic + lignes surlignées. Les index sont ceux
    /// des **lignes source** (avant tri/pagination) — le `DataTable` fait la traduction avec la
    /// tranche affichée, exactement comme il le fait déjà pour le tri et la page.
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    selected: Vec<usize>,
    /// Comparateur **personnalisé** par colonne (jalon 240) : `None` = tri par défaut
    /// ([`compare_cells`]). Permet d'ordonner des cellules que le défaut trie mal — dates
    /// formatées, montants (« $1.2M »), priorités (« High »/« Medium »/« Low »).
    comparators: Vec<Option<Rc<dyn Fn(&str, &str) -> Ordering>>>,
    /// Sélection **multiple** (jalon 241) : colonne de cases à cocher. `on_check(ligne_source)`
    /// bascule une ligne, `on_check_all` bascule la case de tête. L'état coché suit
    /// [`selected`](Self::selected) (mêmes index source).
    on_check: Option<Rc<dyn Fn(usize) -> Msg>>,
    on_check_all: Option<Msg>,
    /// Recherche (jalon 242) : requête courante + rappel à la frappe. Quand `on_query` est posé, un
    /// champ de recherche coiffe le tableau et les lignes source sont **filtrées** ([`row_matches`])
    /// avant tri et pagination — tout reste en index source (sélection/cases inchangées).
    query: Option<String>,
    on_query: Option<Rc<dyn Fn(String) -> Msg>>,
    /// Barre d'**actions groupées** (jalon 243) : fabrique de widgets d'action (boutons…), rappelée à
    /// chaque reconstruction. Rendue **au-dessus** du tableau **seulement** quand des lignes sont
    /// sélectionnées, précédée d'un « N selected ». L'app câble ses boutons (variantes, messages).
    bulk_actions: Option<Rc<dyn Fn() -> Vec<Box<dyn Widget<Msg>>>>>,
    /// Texte de l'**état vide** (jalon 244) : affiché centré, sous l'en-tête, quand aucune ligne n'est
    /// visible (données vides ou filtre sans résultat). Surchargeable via [`empty_text`](DataTable::empty_text).
    empty_text: String,
    /// Rendu : le `Table` (lignes triées/paginées), éventuellement coiffé d'un pied (libellé de
    /// tranche + `Pagination` + sélecteur de taille) dessous.
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
            page_sizes: Vec::new(),
            on_page_size: None,
            on_select: None,
            selected: Vec::new(),
            comparators: Vec::new(),
            on_check: None,
            on_check_all: None,
            query: None,
            on_query: None,
            bulk_actions: None,
            empty_text: "No results".to_string(),
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

    /// Ajoute un **sélecteur de taille de page** (un `SegmentedControl` des `sizes` proposées) dans
    /// le pied. `on_page_size(taille)` au changement (l'app met à jour la taille et, en général,
    /// revient à la page 1). Sans effet si le tableau n'est pas paginé.
    pub fn page_sizes(mut self, sizes: &[usize], on_page_size: impl Fn(usize) -> Msg + 'static) -> Self {
        self.page_sizes = sizes.iter().copied().filter(|&s| s > 0).collect();
        self.on_page_size = Some(Rc::new(on_page_size));
        self.rebuild();
        self
    }

    /// Rend les lignes **cliquables** : `on_select_row(ligne_source)` au clic sur une ligne.
    /// L'index passé est celui de la **ligne source** (avant tri/pagination) — le `DataTable`
    /// traduit la position affichée en index d'origine.
    pub fn on_select_row(mut self, on_select: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self.rebuild();
        self
    }

    /// Lignes **sélectionnées** (surlignées / cases cochées), désignées par leur index de **ligne
    /// source**. Le `DataTable` ne surligne (et ne coche) que celles visibles dans la tranche
    /// courante.
    pub fn selected(mut self, rows: &[usize]) -> Self {
        self.selected = rows.to_vec();
        self.rebuild();
        self
    }

    /// Active la **sélection multiple** : une colonne de cases à cocher coiffée d'un « tout cocher ».
    /// `on_check(ligne_source)` bascule une ligne (index de la **ligne source**, traduit depuis la
    /// position affichée), `on_check_all` bascule la case de tête. L'état coché reflète
    /// [`selected`](Self::selected). Se combine avec [`on_select_row`](Self::on_select_row) : la case
    /// gère la sélection groupée, un clic sur le corps de la ligne reste un clic de ligne.
    pub fn checkboxes(mut self, on_check: impl Fn(usize) -> Msg + 'static, on_check_all: Msg) -> Self {
        self.on_check = Some(Rc::new(on_check));
        self.on_check_all = Some(on_check_all);
        self.rebuild();
        self
    }

    /// Rend le tableau **cherchable** : un champ de recherche (valeur `query`) coiffe le tableau, et
    /// les lignes source sont **filtrées** ([`row_matches`], sous-chaîne insensible à la casse sur
    /// toutes les colonnes) avant tri/pagination. `on_query(texte)` à chaque frappe (l'application met
    /// à jour `query`, et en général revient à la page 1). Le filtre agit en amont du tri, de la page
    /// **et** de la sélection : cases et surlignage restent en index source, sur le sous-ensemble visible.
    pub fn searchable(mut self, query: impl Into<String>, on_query: impl Fn(String) -> Msg + 'static) -> Self {
        self.query = Some(query.into());
        self.on_query = Some(Rc::new(on_query));
        self.rebuild();
        self
    }

    /// Ajoute une **barre d'actions groupées** au-dessus du tableau, visible **seulement** quand des
    /// lignes sont [`selected`](Self::selected). La fabrique `make` produit les widgets d'action
    /// (typiquement des [`Button`](crate::Button) — l'app choisit variantes et messages) ; la barre les
    /// précède d'un libellé « N selected ». Rappelée à chaque reconstruction (widgets frais). Le
    /// nombre affiché est celui des lignes **sélectionnées** (toutes pages confondues).
    pub fn bulk_actions(mut self, make: impl Fn() -> Vec<Box<dyn Widget<Msg>>> + 'static) -> Self {
        self.bulk_actions = Some(Rc::new(make));
        self.rebuild();
        self
    }

    /// Surcharge le texte de l'**état vide** (défaut « No results ») — p. ex. « No people match your
    /// search ». Affiché centré sous l'en-tête quand aucune ligne n'est visible (jalon 244).
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self.rebuild();
        self
    }

    /// Donne un **comparateur personnalisé** à la colonne `col` : `cmp(a, b)` ordonne deux de ses
    /// cellules (texte). Remplace le tri par défaut ([`compare_cells`]) pour cette colonne — utile
    /// quand les valeurs se trient mal telles quelles : dates formatées (« Mar 2024 »), montants
    /// (« $1.2M »), priorités (« High »/« Medium »/« Low »). Le sens (`sorted(_, ascending)`)
    /// s'applique par-dessus (le comparateur définit l'ordre **croissant**).
    pub fn sort_with(mut self, col: usize, cmp: impl Fn(&str, &str) -> Ordering + 'static) -> Self {
        if self.comparators.len() <= col {
            self.comparators.resize_with(col + 1, || None);
        }
        self.comparators[col] = Some(Rc::new(cmp));
        self.rebuild();
        self
    }

    /// Ordre des **index de lignes source** après **filtre** (recherche) puis tri (stable) ;
    /// l'identité filtrée si aucun tri. Le tri utilise le comparateur **personnalisé** de la colonne
    /// s'il en a un, sinon [`compare_cells`].
    fn sorted_order(&self) -> Vec<usize> {
        // Filtre de recherche en amont : ne garde que les lignes source correspondantes.
        let mut order: Vec<usize> = (0..self.rows.len())
            .filter(|&i| match &self.query {
                Some(q) => row_matches(&self.rows[i], q),
                None => true,
            })
            .collect();
        if let Some((col, asc)) = self.sort {
            let empty = String::new();
            let custom = self.comparators.get(col).and_then(|c| c.as_ref());
            order.sort_by(|&a, &b| {
                let (x, y) = (self.rows[a].get(col).unwrap_or(&empty), self.rows[b].get(col).unwrap_or(&empty));
                let ord = match custom {
                    Some(cmp) => cmp(x, y),
                    None => compare_cells(x, y),
                };
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }
        order
    }

    /// (Re)construit le rendu : lignes triées selon `sort`, découpées à la page `page` le cas
    /// échéant, dans un `Table` (en-têtes, largeurs, indicateur) éventuellement coiffé d'un
    /// `Pagination` dessous.
    fn rebuild(&mut self) {
        // On raisonne sur des **index de lignes source** (triés puis découpés) : cela préserve
        // l'identité d'origine de chaque ligne à travers le tri et la pagination, pour traduire
        // la sélection (index affiché ↔ index source) dans les deux sens.
        let order = self.sorted_order();
        let total = order.len();
        let page_indices: Vec<usize> = match self.page {
            Some((current, per)) => {
                let per = per.max(1);
                let current = current.clamp(1, page_count(total, per));
                let start = (current - 1) * per;
                let end = (start + per).min(total);
                order[start..end].to_vec()
            }
            None => order.clone(),
        };
        let hrefs: Vec<&str> = self.headers.iter().map(|s| s.as_str()).collect();
        let mut t = Table::new(self.headers.len().max(1)).header(&hrefs);
        for &i in &page_indices {
            let refs: Vec<&str> = self.rows[i].iter().map(|s| s.as_str()).collect();
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
        // Sélection : `Table` raisonne en **positions affichées** (0..tranche). On câble le clic
        // pour renvoyer l'index **source** et on traduit les lignes sélectionnées source → position.
        if let Some(f) = &self.on_select {
            let f = f.clone();
            let indices = page_indices.clone();
            t = t.on_select_row(move |d| f(indices.get(d).copied().unwrap_or(d)));
        }
        // Sélection multiple : même traduction position affichée → index source pour les cases.
        if let (Some(f), Some(all)) = (&self.on_check, &self.on_check_all) {
            let f = f.clone();
            let indices = page_indices.clone();
            let all = all.clone();
            t = t.checkboxes(move |d| f(indices.get(d).copied().unwrap_or(d)), all);
        }
        if !self.selected.is_empty() {
            let display_sel: Vec<usize> = page_indices
                .iter()
                .enumerate()
                .filter(|(_, orig)| self.selected.contains(orig))
                .map(|(d, _)| d)
                .collect();
            if !display_sel.is_empty() {
                t = t.selected(&display_sel);
            }
        }
        // État vide (données vides ou filtre sans résultat) : en-tête + message centré, **sans** pied
        // (un pager « 0 of 0 » sous un corps vide n'apporte rien). Le message est surchargeable.
        let block: Box<dyn Widget<Msg>> = if total == 0 {
            let message = Text::new(self.empty_text.clone()).size(15.0);
            let empty = Flex::column().align(Align::Center).padding(24.0).child(message);
            Box::new(Flex::column().gap(8.0).child(t).child(empty))
        } else {
            match (self.page, &self.on_page) {
            (Some((current, per)), Some(on_page)) => {
                let pages = page_count(total, per);
                let current = current.clamp(1, pages);
                // Libellé « N–M of T » de la tranche courante (jalon 236).
                let label = Text::new(page_range_label(current, per, total)).size(13.0);
                let on_page = on_page.clone();
                let pager = Pagination::new(current, pages, move |p| on_page(p));
                let mut footer =
                    Flex::row().align(Align::Center).gap(12.0).child(label).child(Flex::row().flex(1.0)).child(pager);
                // Sélecteur de taille de page, si proposé (jalon 236).
                if let (Some(on_size), false) = (&self.on_page_size, self.page_sizes.is_empty()) {
                    let sizes = self.page_sizes.clone();
                    let sel = sizes.iter().position(|&s| s == per).unwrap_or(0);
                    let on_size = on_size.clone();
                    let mut seg = SegmentedControl::new(sel, move |i| on_size(sizes[i]));
                    for s in &self.page_sizes {
                        seg = seg.segment(s.to_string());
                    }
                    footer = footer.child(seg);
                }
                Box::new(Flex::column().gap(12.0).child(t).child(footer))
            }
            _ => Box::new(t),
            }
        };
        // Actions groupées : une barre au-dessus du tableau, uniquement si une sélection existe.
        let mut block = block;
        if let Some(make) = &self.bulk_actions {
            if !self.selected.is_empty() {
                let label = Text::new(format!("{} selected", self.selected.len())).size(14.0);
                let mut bar =
                    Flex::row().align(Align::Center).gap(8.0).child(label).child(Flex::row().flex(1.0));
                for w in make() {
                    bar = bar.child(w);
                }
                block = Box::new(Flex::column().gap(12.0).child(bar).child(block));
            }
        }
        // Recherche : coiffe le tableau d'un champ (sinon on garde le bloc tel quel).
        self.inner = if let Some(on_query) = &self.on_query {
            let on_query = on_query.clone();
            let field = TextInput::new(self.query.clone().unwrap_or_default())
                .placeholder("Search")
                .width(240.0)
                .on_input(move |s| on_query(s));
            Box::new(Flex::column().gap(12.0).child(field).child(block))
        } else {
            block
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
        // inner = colonne [table, pied] → deux enfants.
        assert_eq!(Widget::<()>::children(&dt).len(), 2, "table + pied de pagination");
    }

    #[test]
    fn page_range_label_describes_the_slice() {
        assert_eq!(page_range_label(1, 3, 7), "1\u{2013}3 of 7");
        assert_eq!(page_range_label(2, 3, 7), "4\u{2013}6 of 7");
        assert_eq!(page_range_label(3, 3, 7), "7\u{2013}7 of 7", "dernière page partielle");
        assert_eq!(page_range_label(1, 3, 0), "0 of 0", "vide");
        assert_eq!(page_range_label(99, 3, 7), "7\u{2013}7 of 7", "page hors bornes ramenée");
    }

    /// Collecte, en ordre d'arbre, tous les messages `on_click` non nuls d'un sous-arbre.
    fn collect_clicks(w: &dyn Widget<usize>, out: &mut Vec<usize>) {
        if let Some(m) = w.on_click() {
            out.push(m);
        }
        for c in w.children() {
            collect_clicks(c.as_ref(), out);
        }
    }

    #[test]
    fn selection_click_reports_the_source_row_through_sort_and_page() {
        // Trois lignes, clé numérique en colonne 1. Tri croissant → ordre source [1, 2, 0].
        let rows = vec![
            vec!["A".to_string(), "3".to_string()],
            vec!["B".to_string(), "1".to_string()],
            vec!["C".to_string(), "2".to_string()],
        ];
        let make = |page: usize| -> DataTable<usize> {
            DataTable::new(["N", "K"], rows.clone()).sorted(1, true).paginated(page, 2, |_| 0).on_select_row(|i| i)
        };
        // `children()[0]` = le Table ; `[1]` = le pied (pager) qu'on ignore.
        let clicks_of = |dt: &DataTable<usize>| {
            let mut v = Vec::new();
            collect_clicks(Widget::<usize>::children(dt)[0].as_ref(), &mut v);
            v.dedup(); // une cellule cliquable par colonne → répétitions consécutives
            v
        };
        // Page 1 (taille 2) des lignes triées [1, 2, 0] : le clic renvoie l'index **source** 1 puis 2.
        assert_eq!(clicks_of(&make(1)), vec![1, 2], "le clic renvoie l'index de la ligne source");
        // Page 2 : la dernière ligne triée, index source 0 — la pagination n'altère pas l'identité.
        assert_eq!(clicks_of(&make(2)), vec![0], "la traduction survit à la pagination");
    }

    #[test]
    fn empty_filter_drops_rows_and_pager() {
        // Deux lignes, filtre « zzz » qui ne matche rien → état vide : aucune ligne cliquable, et le
        // **pied de pagination est retiré** (sinon son unique bouton de page émettrait `0`).
        let rows = vec![vec!["Ada".to_string()], vec!["Bob".to_string()]];
        let dt: DataTable<usize> =
            DataTable::new(["N"], rows).searchable("zzz", |_| 0).paginated(1, 5, |_| 0).on_select_row(|i| i);
        let mut clicks = Vec::new();
        for c in Widget::<usize>::children(&dt) {
            collect_clicks(c.as_ref(), &mut clicks);
        }
        assert!(clicks.is_empty(), "ni ligne cliquable ni pager quand le filtre ne matche rien");
        // L'arbre reste non vide : en-tête + message d'état vide.
        assert!(!Widget::<usize>::children(&dt).is_empty(), "en-tête + message rendus");
    }

    #[test]
    fn bulk_actions_bar_shows_only_with_a_selection() {
        use crate::button::Button;
        let rows = vec![vec!["A".to_string()], vec!["B".to_string()]];
        // `777` = message d'une action de la barre (sentinelle).
        let make = |sel: &[usize]| -> DataTable<usize> {
            DataTable::new(["N"], rows.clone())
                .checkboxes(|i| i, 900usize)
                .selected(sel)
                .bulk_actions(|| vec![Box::new(Button::new("Clear").on_press(777usize)) as Box<dyn Widget<usize>>])
        };
        let has_action = |dt: &DataTable<usize>| {
            let mut v = Vec::new();
            for c in Widget::<usize>::children(dt) {
                collect_clicks(c.as_ref(), &mut v);
            }
            v.contains(&777)
        };
        assert!(!has_action(&make(&[])), "aucune barre sans sélection");
        assert!(has_action(&make(&[0])), "barre d'actions présente dès qu'une ligne est sélectionnée");
    }

    #[test]
    fn row_matches_is_case_insensitive_substring_over_all_cells() {
        let row = vec!["Ada Lovelace".to_string(), "Engineer".to_string()];
        assert!(row_matches(&row, ""), "requête vide = tout passe");
        assert!(row_matches(&row, "  "), "requête blanche = tout passe");
        assert!(row_matches(&row, "ENGIN"), "insensible à la casse, sous-chaîne");
        assert!(row_matches(&row, "love"), "autre colonne");
        assert!(!row_matches(&row, "zzz"), "aucune correspondance");
    }

    #[test]
    fn search_filters_rows_before_sort_and_keeps_source_indices() {
        // Quatre lignes ; on cherche « a » → seules Ada et Cal correspondent (index source 0, 2).
        let rows = vec![
            vec!["Ada".to_string(), "3".to_string()],
            vec!["Bob".to_string(), "1".to_string()],
            vec!["Cal".to_string(), "2".to_string()],
            vec!["Eve".to_string(), "4".to_string()],
        ];
        // Tri croissant par clé parmi {Ada(3), Cal(2)} → Cal(2), Ada(3) = index source [2, 0].
        let dt: DataTable<usize> =
            DataTable::new(["N", "K"], rows).searchable("a", |_| 0).sorted(1, true).on_select_row(|i| i);
        let mut clicks = Vec::new();
        for c in Widget::<usize>::children(&dt) {
            collect_clicks(c.as_ref(), &mut clicks);
        }
        clicks.dedup();
        assert_eq!(clicks, vec![2, 0], "filtré puis trié, en index source");
    }

    #[test]
    fn checkbox_click_reports_the_source_row_through_sort_and_page() {
        // Mêmes données que la sélection simple : tri croissant → ordre source [1, 2, 0].
        let rows = vec![
            vec!["A".to_string(), "3".to_string()],
            vec!["B".to_string(), "1".to_string()],
            vec!["C".to_string(), "2".to_string()],
        ];
        // `999` = message de la case « tout cocher » (sentinelle, filtrée).
        let dt: DataTable<usize> =
            DataTable::new(["N", "K"], rows).sorted(1, true).paginated(2, 2, |_| 0).checkboxes(|i| i, 999);
        let mut v = Vec::new();
        // `children()[0]` = le Table ; `[1]` = le pied (pager) ignoré.
        collect_clicks(Widget::<usize>::children(&dt)[0].as_ref(), &mut v);
        v.retain(|&m| m != 999); // enlève la case de tête
        v.dedup();
        // Page 2 (taille 2) des lignes triées [1, 2, 0] → la case renvoie l'index **source** 0.
        assert_eq!(v, vec![0], "la case renvoie l'index de la ligne source, page comprise");
    }

    #[test]
    fn custom_comparator_orders_a_column_semantically() {
        // Colonne de **priorité** que le tri texte classerait par ordre alphabétique
        // (High < Low < Medium) — sémantiquement faux. Un comparateur maison impose Low < Medium < High.
        let rows = vec![
            vec!["A".to_string(), "High".to_string()],
            vec!["B".to_string(), "Low".to_string()],
            vec!["C".to_string(), "Medium".to_string()],
        ];
        let rank = |s: &str| match s {
            "Low" => 0,
            "Medium" => 1,
            "High" => 2,
            _ => 3,
        };
        let dt: DataTable<usize> = DataTable::new(["N", "Prio"], rows)
            .sorted(1, true)
            .sort_with(1, move |a, b| rank(a).cmp(&rank(b)))
            .on_select_row(|i| i);
        let mut clicks = Vec::new();
        for c in Widget::<usize>::children(&dt) {
            collect_clicks(c.as_ref(), &mut clicks);
        }
        clicks.dedup();
        // Croissant sémantique Low(1) < Medium(2) < High(0) → index source [1, 2, 0], pas l'ordre
        // alphabétique [High(0), Low(1), Medium(2)] du tri par défaut.
        assert_eq!(clicks, vec![1, 2, 0], "le comparateur personnalisé ordonne par priorité");
    }

    #[test]
    fn page_size_selector_appears_in_the_footer() {
        let rows: Vec<Vec<String>> = (1..=7).map(|i| vec![i.to_string()]).collect();
        // inner = [table, pied] ; le pied est une Flex row [libellé, spacer, pager, (sélecteur)].
        let footer_len = |dt: &DataTable<()>| Widget::<()>::children(dt)[1].children().len();
        let base = DataTable::<()>::new(["N"], rows.clone()).paginated(1, 3, |_| ());
        let sized = DataTable::<()>::new(["N"], rows).paginated(1, 3, |_| ()).page_sizes(&[3, 5], |_| ());
        assert_eq!(footer_len(&base), 3, "libellé + spacer + pager");
        assert_eq!(footer_len(&sized), 4, "+ sélecteur de taille");
    }
}
