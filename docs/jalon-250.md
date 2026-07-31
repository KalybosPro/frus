# Jalon 250 — Registre des réordonnables (glisser des cartes fonctionnel)

## Analyse

Le glisser-déposer de réordonnancement du shell repérait ses cibles via le registre des widgets
**cliquables** (`ui.hit`). Les en-têtes de `Table` marchent car ils sont cliquables (tri), mais les
**cartes Kanban** et leurs **zones de dépôt** n'ont **pas** d'action de clic : elles n'entraient donc
dans aucun registre, et le shell ne pouvait ni les saisir ni les cibler. Le déplacement (`on_move`,
jalons 247–249) était correct en logique mais **ne s'engageait pas** à la souris.

Ce jalon ajoute un **registre des réordonnables** distinct du clic.

## Décisions techniques

- **Nouveau registre `reorderables: Vec<(WidgetId, Rect)>`** dans `Ui`, peuplé pour tout widget dont
  `reorder_index()` est `Some` — indépendamment de sa cliquabilité. Accès `Ui::reorderable_at(point)`.

- **Collecté comme `interactives`, pas caché.** Plutôt que d'étendre le cache de peinture
  (`BoundaryData`/`Snapshot`), on **désactive** la mise en cache d'un sous-arbre contenant un
  réordonnable (via `plain_subtree_len`), exactement comme pour un `InteractiveViewer` ou un `Scroll` :
  le registre est donc reconstruit à chaque frame, toujours à jour. Les blocs de **transformation**
  (échelle/rotation) transforment aussi les bornes des réordonnables (parité avec `draggables`).

- **Le shell utilise le registre.** `reorderable_at` (source à l'appui), la **cible** au dépôt, et la
  **ligne d'insertion** (aperçu vertical, jalon 248) lisent désormais `ui.reorderable_at` au lieu de
  `ui.hit`.

## Implémentation

- `frus-widgets/src/ui.rs` : champ `reorderables` (`Ui` + `Builder`) + init + assemblage ; collecte
  `if reorder_index().is_some()` dans les deux boucles de parcours ; transformation des bornes dans les
  deux blocs `Transform` ; `plain_subtree_len` exclut les réordonnables du cache ; accès
  `reorderable_at`. Test `kanban_cards_are_reorderable_without_being_clickable` (une carte est
  saisissable au point **et** absente du registre de clics).
- `frus-shell/src/app.rs` : `reorderable_at`, la cible de dépôt et `reorder_drop_line` passent par
  `ui.reorderable_at`.

## Vérification

- **Widgets** : la carte Kanban est enregistrée comme réordonnable sans être cliquable ; `reorderable_at`
  la retrouve là où `ui.hit` ne trouve rien.
- **Non-régression** : le registre n'émet aucune primitive (mêmes pixels) — goldens inchangés ; les
  en-têtes de `Table` (réordonnables **si** `on_reorder`) empruntent le même chemin qu'avant côté clic.
- Widgets 387 ; shell 26 ; goldens 77 ; démo 36.

## Notes

- L'**engagement** du glisser (source/cible/route) est désormais correct et couvert par des tests
  unitaires (registre + `reorderable_at` + logique `on_move`). Le rendu **live** du glisser (fantôme +
  ligne d'insertion) reste un état runtime non inspecté au GPU dans cet environnement ; le fantôme d'une
  carte **riche** ne capture que la tuile (le contenu, peint par des enfants, a un autre propriétaire) —
  affinage possible.

## Reste

- Fantôme d'aperçu incluant le **contenu** d'une carte riche.
- Indicateur d'insertion **inter-cartes** (au-dessus/au-dessous selon la moitié survolée).
