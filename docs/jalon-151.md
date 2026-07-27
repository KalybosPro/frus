# Jalon 151 — Tableau : redimensionnement de colonnes à la souris

## Analyse

Le tableau (jalons 145–149) triait, sélectionnait et cochait, mais ses colonnes
étaient **figées** : aucune poignée pour ajuster une largeur à la souris, alors que
c'est un geste attendu de toute grille de données (façon Flutter `DataTable` +
`ReorderableList`, tableurs).

Le blocage identifié dès le jalon 149 : cela demande un **glissement absolu** dans le
shell, or le seul mécanisme existant (`on_drag(fraction)`, utilisé par `Slider`) donne
une **fraction bornée** aux bornes du widget — sur une fine poignée, la fraction sature
aussitôt et ne peut pas **agrandir** une colonne.

## Décisions techniques

- **Glissement en delta, générique.** Nouveau `Widget::on_drag_delta(dx)` : `dx` est le
  déplacement horizontal (px) **depuis le dernier événement**. Le shell l'essaie **avant**
  `on_drag` ; un widget n'implémente que l'un des deux (`Slider` = fraction, poignée =
  delta). `Drag::Widget` mémorise `last_x` pour livrer le delta incrémental. Ce choix
  **incrémental** (et non absolu depuis le début) compose avec la reconstruction de
  l'arbre : chaque petit message est **accumulé** par l'application, sans double comptage
  quand la vue se rebâtit à mi-glissement.

- **Poignées en calque flottant.** Le tableau superpose (via `Stack`) un **calque de
  poignées** au-dessus de la grille : une fine barre verticale au bord droit de chaque
  colonne (sauf la dernière), calée par des **cales transparentes** (`Spacer`) sur la
  géométrie exacte des colonnes. Les cales sont **inertes** (ni cliquables ni
  focusables) : les clics de tri / sélection **traversent** le calque jusqu'à la grille
  (seuls les widgets *cliquables* peuplent la table de hit-test), tandis que
  `draggable_at` n'attrape que les poignées. Un glissement supprime le clic de relâchement
  (déjà le cas pour tout `Drag::Widget`), donc saisir une poignée ne déclenche pas de tri.

- **Contrôlé, comme le reste.** `on_resize(colonne, delta)` : l'application **accumule** la
  largeur (`widths[col] = (widths[col] + delta).max(MIN)`) et la repasse via
  `column_widths`. Le tableau ne stocke aucune largeur « vivante ».

- **Seulement si colonnes fixes.** Les poignées n'apparaissent que si **toutes** les
  colonnes ont une largeur fixe (bords connus) ; une colonne flexible désactive le calque
  (sa largeur rendue est inconnue du widget).

## Implémentation

- `widget.rs` : `on_drag_delta` (défaut `None`) + relais `Box` ; `keyed.rs`,
  `responsive.rs` : relais.
- `app.rs` (shell) : `Drag::Widget` gagne `last_x` ; `apply_widget_drag(id, rect, dx)`
  essaie `on_drag_delta(dx)` puis `on_drag(fraction)` ; le glissement calcule le delta
  incrémental.
- `table.rs` : `Spacer` (cale inerte) + `ResizeHandle` (poignée glissable, `on_drag_delta`
  → `on_resize(col, dx)`) ; champ `on_resize` (`Rc`) + `.on_resize()` ; `resize_overlay`
  construit le calque ; `rebuild` emballe la grille dans un `Stack` et relaie `stack()`.
- `goldens.rs` : golden `data_table_resizable` (3 colonnes fixes, poignées visibles).

## Vérification

- **Unitaire** : la poignée émet `Resize(0, 12.0)` pour un delta de 12 px et `None` pour
  un delta nul ; elle est **saisissable** (`draggable_at`) au bord de la 1re colonne
  (x≈100) ; des colonnes **flexibles** ne produisent **aucune** poignée. Tri / sélection /
  cases inchangés (tests verts).
- **Golden** `data_table_resizable` **inspecté** : fines barres verticales au bord droit de
  « Name » et « Role », aucune après « Score » (dernière colonne).
- `cargo test --workspace` **vert** (slider compris — le chemin `on_drag` fraction est
  préservé).

## Reste

- **Curseur `ew-resize`** au survol d'une poignée (indice visuel Material) — demande une
  notion de curseur par widget dans le shell.
- **Redimensionnement des colonnes flexibles** (mesurer la largeur rendue pour amorcer le
  delta) et **réordonnancement** des colonnes (glisser un en-tête).
