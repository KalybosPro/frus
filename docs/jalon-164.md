# Jalon 164 — Tableau : cellules-widgets (au-delà du texte)

## Analyse

Le tableau ne savait afficher que du **texte** (`.row(&[&str])`). Or une grille réelle
mêle des puces d'état, des avatars, des boutons d'action en ligne… Il fallait des
**cellules-widgets**.

Le nœud : le tableau **se reconstruit** après chaque réglage (rebuild à chaque builder,
pour l'indépendance d'ordre). Or un `Box<dyn Widget>` n'est **pas clonable** — impossible
de le stocker puis de le rejouer à chaque reconstruction.

## Décisions techniques

- **Cellule par fabrique.** Une cellule-widget est fournie par une **fabrique**
  `Fn() -> Box<dyn Widget<Msg>>` (stockée en `Rc`, donc partageable) : `rebuild` la
  **rappelle** pour produire un widget **frais** à chaque reconstruction — compatible avec
  l'architecture existante, sans forwarder `Rc<dyn Widget>` (≈ 50 méthodes). Les lignes de
  données deviennent un `RowKind::{Text, Widgets}` ; **le texte reste inchangé** (aucune
  régression sur les tableaux existants).

- **Cellule = conteneur thémé.** `WidgetCell` occupe la largeur de colonne × la hauteur de
  rangée, centre son contenu (padding horizontal), peint le **fond de cellule** (survol /
  sélection) et reste **cliquable** pour la sélection de ligne — le contenu (bouton,
  puce…) se peint **par-dessus**, et un widget interne cliquable capte le clic là où il est
  (hit-test du plus haut), la zone libre sélectionnant la ligne.

## Implémentation

- `table.rs` : type public `CellFactory<Msg>` ; `enum RowKind` ; `WidgetCell` (fond +
  contenu centré + clic de sélection) ; `rows: Vec<RowKind>` ; `.widget_row(cells)` ;
  `rebuild` gère les deux variantes.
- `goldens.rs` : `table_widget_cells` (colonne d'avatars + colonne de `Chip`).

## Vérification

- **Unitaire** : une `widget_row` produit une rangée de cellules contenant **chacune un
  widget** ; le contenu (« admin ») est **peint** ; la ligne-widget reste **sélectionnable**
  (`on_select_row`). Tri / sélection / redimensionnement / réordonnancement des tableaux
  texte : inchangés.
- **Golden** `table_widget_cells` **inspecté** : colonne d'**avatars** (« A », « B ») et
  colonne de **puces** (« admin », « editor »), centrées dans leurs cellules.
- `cargo test --workspace` **vert**.

## Reste

- **Tri de colonnes-widgets** : l'app fournit la clé de tri (le tableau ne sait pas
  comparer des widgets) — déjà possible côté application, à documenter.
- **Cellules d'en-tête widget** (icône + libellé) et **hauteur de rangée adaptative** au
  contenu (aujourd'hui `ROW_H` fixe : un contenu plus grand est rogné).
