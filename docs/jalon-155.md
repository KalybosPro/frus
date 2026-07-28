# Jalon 155 — Réordonnancement des colonnes : aperçu glissant

## Analyse

Le réordonnancement (jalon 153) fonctionnait « à l'aveugle » : on saisissait un en-tête,
on relâchait, la colonne sautait. Sans **retour visuel** pendant le glissement, impossible
de viser une position — le jalon 153 l'avait laissé au « Reste ».

## Décisions techniques

- **Peint par le shell, par-dessus la scène.** Le glissement est un état du shell
  (`Drag::Reorder`), pas de l'application ; l'aperçu doit donc vivre **hors de l'arbre**
  contrôlé. On réutilise le patron de l'inspecteur : cloner la scène logique, **peindre
  l'aperçu**, puis mettre à l'échelle physique. Aucune donnée d'aperçu ne remonte dans
  `view`, cohérent avec l'architecture.

- **Trois repères Material.** ① Colonne **source estompée** (elle quitte sa place) ;
  ② **indicateur de dépôt** — barre verticale `primary` au bord d'insertion de la colonne
  cible (gauche si la cible précède la source, droite sinon) ; ③ **carte soulevée** suivant
  le curseur : la boîte de l'en-tête décalée de `dx`, avec **ombre portée** et **bord
  accentué** (élévation façon `ReorderableListView`).

- **Découpe neutralisée.** L'aperçu se dessine sous `Rect::UNBOUNDED` : la carte peut
  déborder de la colonne source sans être rognée par la découpe héritée.

- **Géométrie via le hit-test existant.** Les bornes de la source et de la cible viennent
  de `Ui::widget_rect` (les en-têtes triables sont focusables → indexés). La cible est
  résolue en direct par `reorderable_at(curseur)`. Zéro nouvel état.

- **Sans texte (assumé).** La carte reprend la boîte, pas le libellé (le shell n'a pas de
  `label()` sur les widgets) : un rectangle soulevé + l'indicateur suffisent à viser. La
  capture des primitives de l'en-tête pour un fantôme **texte compris** est notée au Reste
  (elle bute sur la découpe stockée par primitive).

## Implémentation

- `app.rs` (shell) : `draw_reorder_overlay(scene, theme, src, dx, drop)` — **fonction pure**
  (estompe + indicateur optionnel + ombre + carte) ; `paint_reorder_preview` calcule la
  géométrie (source, cible, `dx`) et l'appelle ; branche de rendu : si un
  `Drag::Reorder { moved: true }` est actif, cloner → peindre → mettre à l'échelle ;
  `handle_drag` redessine à chaque déplacement pour que la carte suive le curseur.

## Vérification

- **Unitaire** (`draw_reorder_overlay`, sans GPU) : avec cible → **4** primitives (estompe +
  indicateur + ombre + carte) ; sans cible (même colonne) → **3** (pas d'indicateur). La
  fonction pure isole la forme du chemin d'événements.
- **Non golden** : l'aperçu est **interactif** (piloté par le glissement), pas un rendu
  statique ; sa forme est couverte par le test ci-dessus, et le routage (jalon 153) l'est
  par les tests de contrat du tableau.
- `cargo test --workspace` **vert**, sans avertissement.

## Reste

- **Fantôme texte compris** : capturer les primitives de l'en-tête (`owner == id`), les
  translater et les **dé-découper** (aujourd'hui chaque primitive porte sa propre découpe).
- **Décalage animé des voisines** (elles s'écartent pour ouvrir la place de dépôt).
