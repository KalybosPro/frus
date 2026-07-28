# Jalon 161 — Réordonnancement : clavier & coulissement continu

## Analyse

Le réordonnancement de colonnes (jalons 153/155/159) était **souris seule** et son
coulissement **sautait** d'une colonne à l'autre (le réagencement suivait la colonne cible,
pas le curseur). Deux manques du « Reste » : l'**accès clavier** et un coulissement **doux**.

## Décisions techniques

### Coulissement continu (regroupé par cellule)

L'ancien réagencement classait chaque **primitive** par sa position et décalait d'un bloc
la bande entre source et cible — d'où un **saut** quand le curseur changeait de colonne, et
un risque de **cisaillement** (fond et texte d'une cellule décalés différemment pendant une
transition).

Nouveau `reflow_reorder_columns(prims, src, cursor_x, lifted_owner)` : on **regroupe les
primitives par propriétaire** (une cellule = un `owner` : fond + texte + icône). Chaque
cellule coulisse **d'un bloc** (plus de cisaillement) d'une quantité **continue** fonction
de `cursor_x` — le coulissement **suit le curseur** au lieu de sauter. La cible n'est plus
nécessaire à l'aperçu (seulement au dépôt). Les blocs plus larges qu'une colonne (fonds de
page/ligne) restent en place ; les cellules sans fond (réduites à leur texte) prennent la
largeur d'un cran comme échelle de transition.

### Réordonnancement clavier

Le routage `on_key` (jalon 160) propose déjà les flèches au widget focalisé. Un en-tête
focalisé consomme **Ctrl+Gauche/Droite** (`Key::Left/Right { word: true }`) pour déplacer sa
colonne d'un cran (`on_reorder(from, to)`), **borné** au nombre de colonnes ; au bord, il
**ignore** (le focus navigue alors). Les flèches **nues** restent ignorées (navigation du
focus entre en-têtes). Le tri au clic/Entrée est intact.

## Implémentation

- `reorder.rs` (frus-widgets) : `reflow_reorder_columns` regroupé par `owner` + décalage
  continu (nouvelle signature `cursor_x`) ; tests mis à jour (coulissement partiel).
- `table.rs` : `Cell.reorder` porte aussi le **nombre de colonnes** ; `Cell::on_key`
  (Ctrl+Flèches → `on_reorder`, borné).
- `app.rs` (shell) : `paint_reorder_preview` réagence selon `self.cursor.x` (plus de cible).
- `goldens.rs` : `table_reorder_preview` mis à jour (curseur au-delà de « Score », coulissement plein).

## Vérification

- **Unitaire** : `reflow_reorder_columns` — curseur loin → coulissement **plein** (col 1 → 0,
  col 2 → 100) ; curseur au **centre** d'une colonne → coulissement **à mi-course** (−50), la
  suivante **immobile** ; sens gauche symétrique. `Cell::on_key` — Ctrl+Gauche/Droite sur la
  colonne du milieu → `Reorder(1,0)`/`Reorder(1,2)` ; aux bords → `Ignored` ; flèche nue →
  `Ignored`.
- **Golden** `table_reorder_preview` **inspecté** : « Role » soulevé, « Score » (5/3)
  **coulissé** à la place de « Role », trou ouvert, fantôme « Role » flottant à droite.
- `cargo test --workspace` **vert**.

## Reste

- **Easing temporel** (ressort) en plus du suivi-curseur : demanderait un état d'animation
  par colonne (offset animé) dans le runtime.
- **Réordonnancement clavier annoncé** (sémantique/accessibilité) et **PgUp/PgDn** pour aller
  au bord.
