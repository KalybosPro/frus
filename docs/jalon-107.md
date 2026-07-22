# Jalon 107 — Ancrage : listes virtualisées + `AlignmentDirectional`

## Analyse

Le J106 a livré l'ancrage fractionnel, mais laissait deux trous listés dans son
« Reste » :

1. **Listes virtualisées / `layout_builder`.** Leur rendu passe par `render_item`
   (chemin propre, sans les branches spéciales du walk), qui **n'appliquait pas** le
   décalage d'ancrage : un conteneur ancré comme élément de liste ou enfant de
   `LayoutBuilder` restait collé en haut-gauche.
2. **Ancrage directionnel.** L'`Alignment` physique ne suit pas le sens de lecture ;
   il manquait l'équivalent de `AlignmentDirectional` de Flutter (début/fin résolus
   en RTL).

## Décisions techniques

- **`render_item` réutilise `align_offset`.** Même calcul que le walk principal
  (espace libre × fraction, décalage cascadé par la translation). Un seul point de
  vérité pour l'ancrage, deux chemins de rendu.

- **`AlignmentDirectional` (frus-core).** Struct `{ x_start, y }`, `x_start` exprimé
  **début → fin** (`-1` = début, `+1` = fin), neuf constantes
  (`CENTER_START`, `TOP_END`…). `resolve(direction) -> Alignment` inverse `x_start`
  en RTL (début ↔ droite) ; le `y` est direction-invariant. Type géométrique pur,
  sur le modèle d'`InsetsDirectional`.

- **Résolution à l'endroit qui connaît le sens.** `Container` stocke l'ancrage
  directionnel tel quel ; c'est `Builder::align_offset` (qui tient `self.rtl`) qui le
  **résout** en `Alignment` physique, puis le traite comme n'importe quel ancrage —
  la correction RTL existante fait le reste (double passage cohérent : un ancrage
  directionnel résolu suit exactement la mécanique physique). L'ancrage directionnel
  **prime** sur le physique.

- **Trait `Widget::alignment_directional()`** (défaut `None`) + forwarders
  (`Box`/`Keyed`/`Responsive`/nommés). `Container::alignment_directional(...)` +
  `style()` pose `Start`/`Start` si l'un **ou** l'autre ancrage est posé.

## Implémentation

- `frus-core` : `AlignmentDirectional` + constantes + `resolve` (geometry.rs),
  ré-export.
- `frus-widgets` : trait `alignment_directional()` + forwarders ; `Container`
  (champ `alignment_dir`, builder, `style()`, accès) ; `align_offset` résout le
  directionnel selon `self.rtl` ; `render_item` applique l'ancrage.

## Tests

- `directional_alignment_resolves_by_direction` (core) : `CENTER_START` → gauche
  en LTR, droite en RTL ; `TOP_CENTER` invariant.
- `directional_alignment_flips_the_child_in_rtl` (widgets) : même arbre, l'enfant
  ancré `CENTER_START` est à x≈0 en LTR, x≈80 en RTL.
- Suites vertes : frus-core 87, frus-widgets 198 ; workspace complet vert.

## Reste

- Idiome shell / démo animant `align_tween.animate(&ctrl).value()`.
- Ancrage à **enfants multiples** (aujourd'hui : enfant unique, façon Flutter).
