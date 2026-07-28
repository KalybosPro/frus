# Jalon 171 — Tableau : en-tête entièrement widget

## Analyse

L'en-tête savait porter un libellé texte (+ icône, + widget d'action à droite), mais restait
**structurellement** une cellule texte. Certaines grilles veulent un en-tête **entièrement
libre** : un bouton de tri maison, un filtre intégré, une puce, un titre sur deux lignes… Il
fallait pouvoir **remplacer** la ligne d'en-tête par des **widgets arbitraires** (comme
`widget_row` l'a fait pour les données au jalon 164).

## Décisions techniques

- **`widget_header` en miroir de `widget_row`.** `Table::widget_header(cells)` prend une
  **fabrique** par colonne (`Fn() -> Box<dyn Widget>`, en `Rc`), rappelée à chaque
  reconstruction. La ligne d'en-tête est alors bâtie de cellules-widgets à **fond d'en-tête**,
  au lieu des cellules texte.

- **Tri/réordonnancement laissés à l'application.** Le tableau ne peut pas deviner comment
  trier un widget d'en-tête arbitraire : le tri et le réordonnancement **automatiques** ne
  s'appliquent pas ici. L'application **câble** le comportement dans ses widgets (p.ex. un
  bouton d'en-tête émettant son propre message de tri). C'est le compromis assumé de la
  personnalisation totale — cohérent avec « tri décidé par l'application » (le tableau
  n'affiche que l'état qu'on lui passe).

- **`WidgetCell` réutilisé, avec fond d'en-tête.** La cellule-widget existante gagne un
  drapeau `header` (fond d'en-tête + peinture systématique du fond) ; les en-têtes widget
  sont des `WidgetCell { header: true, .. }`. Aucune nouvelle sorte de cellule.

## Implémentation

- `table.rs` : champ `header_widgets: Vec<CellFactory>` ; builder `widget_header` (vide
  `headers`, dernier appelé gagne) ; drapeau `WidgetCell.header` (fond d'en-tête) ; branche
  d'en-tête widget dans `rebuild` ; `header_present` inclut les en-têtes widget.
- `goldens.rs` : `table_widget_header` (puce « User » + bouton « Sort »).

## Vérification

- **Unitaire** : `widget_header_hosts_arbitrary_header_widgets` — la ligne d'en-tête héberge
  les widgets fournis (« Name », « Sort » peints) ; le bouton d'en-tête maison émet **son**
  message (`Sort(1)`), preuve que l'app câble le tri.
- **Golden** `table_widget_header` **inspecté** : puce + bouton en en-tête, sur fond
  d'en-tête, données dessous — aucune régression sur les autres goldens.
- `cargo test --workspace` **vert**.

## Reste

- **Mélange texte + widget par colonne** : `widget_header` remplace toute la ligne ; un mode
  « widget pour certaines colonnes seulement » (le reste en texte triable) serait une
  extension — non requis ici (l'app peut mettre un simple libellé-widget dans les autres).
- **Réordonnancement d'en-têtes widget** : possible à l'avenir en exposant les hooks
  `reorder_index`/`on_reorder` depuis les widgets fournis.
