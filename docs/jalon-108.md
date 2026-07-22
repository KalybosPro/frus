# Jalon 108 — `AlignmentGeometry` : l'ancrage unifié

## Analyse

Les J106–J107 ont livré deux types d'ancrage — physique ([`Alignment`]) et
directionnel ([`AlignmentDirectional`]) — mais exposés en **double** : deux builders
sur `Container` (`.alignment` / `.alignment_directional`), **deux** méthodes de trait
(`alignment` / `alignment_directional`), une résolution qui filtrait les deux. Or
Flutter place ces deux types sous une abstraction commune, `AlignmentGeometry`, que
`Container.alignment` accepte indifféremment. Ce jalon adopte cette forme : **un**
point d'entrée, résolu en un seul endroit.

## Décisions techniques

- **`AlignmentGeometry` (frus-core).** Enum `Physical(Alignment) |
  Directional(AlignmentDirectional)` avec `resolve(direction) -> Alignment` (le
  physique est renvoyé tel quel). `From<Alignment>` et `From<AlignmentDirectional>`
  → n'importe quel ancrage se convertit implicitement.

- **Un seul builder, signature de Flutter.**
  `Container::alignment(impl Into<AlignmentGeometry>)` accepte physique **ou**
  directionnel — `.alignment(Alignment::CENTER)` comme
  `.alignment(AlignmentDirectional::CENTER_START)`. Le builder `.alignment_directional`
  disparaît (redondant).

- **Une seule méthode de trait.** `Widget::alignment_geometry() ->
  Option<AlignmentGeometry>` remplace les deux précédentes ; `align_offset` résout
  une fois selon `self.rtl`, puis applique la mécanique physique inchangée
  (correction RTL comprise). Surface de trait réduite, forwarders allégés
  (`Box`/`Keyed`/`Responsive`/nommés : une ligne au lieu de deux).

## Implémentation

- `frus-core` : enum `AlignmentGeometry` + `resolve` + deux `From` (geometry.rs),
  ré-export.
- `frus-widgets` : trait `alignment_geometry()` (remplace `alignment` +
  `alignment_directional`) + forwarders ; `Container` (champ unique
  `alignment: Option<AlignmentGeometry>`, builder `impl Into`) ; `align_offset`
  résout la géométrie unifiée.

## Tests

- `alignment_geometry_unifies_physical_and_directional` (core) : un ancrage
  physique est invariant à la direction ; un directionnel suit le sens (LTR →
  gauche, RTL → droite), les deux construits via `Into`.
- Tests d'ancrage existants (centrage, coin, fractionnel, retournement RTL)
  inchangés — le directionnel passe désormais par `.alignment(...)`, prouvant que
  le builder unique accepte les deux.
- Suites vertes : frus-core 88, frus-widgets 198 ; workspace complet vert.

## Reste

- Idiome shell / démo animant `align_tween.animate(&ctrl).value()`.
- Ancrage à **enfants multiples** (aujourd'hui : enfant unique, façon Flutter).
