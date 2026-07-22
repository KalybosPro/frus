# Jalon 109 — Container : marge extérieure (`margin`)

## Analyse

Dernière pièce de parité `Container` de Flutter : la **marge extérieure**. Le
conteneur savait espacer son contenu **à l'intérieur** (padding, J102) mais pas
réserver d'espace **autour** de lui — impossible d'écarter deux cartes sans insérer
un widget d'espacement. Flutter le fait via `Container(margin:)`.

Bilan de parité `Container` : padding ✓ (J102), décoration composite ✓ (J105),
alignement ✓ (J105–J108), **marge ✓ (ce jalon)**.

## Décisions techniques

- **`margin` dans `frus_layout::Style`.** taffy gère nativement la marge
  (`LengthPercentageAuto`) ; il manquait le champ dans notre `Style` mince. Ajouté :
  champ `margin: Insets`, mêlé au `layout_hash` (il change la géométrie → doit
  invalider le cache de relayout) et mappé vers `taffy::Rect` dans `to_taffy`.

- **Marge = extérieure, indépendante de la décoration.** taffy pose la boîte
  **insérée** de sa marge ; le fond, la bordure et l'ombre se peignent dans cette
  boîte réduite (aucun changement de peinture — `paint` reçoit déjà `bounds` insérés).
  La marge **pousse les frères** sans agrandir la décoration.

- **`Container::margin(f32)` / `margin_each(...)`**, parallèles à `padding` /
  `padding_each`. `Flex` (Row/Column) n'expose **pas** de marge (comme Flutter), il
  passe `Insets::ZERO`.

## Implémentation

- `frus-layout/style.rs` : champ `margin`, défaut `ZERO`, `layout_hash`, `to_taffy`.
- `frus-widgets` : `Container` (champ `margin`, builders `.margin`/`.margin_each`,
  `style()` renseigne `margin`) ; `flex.rs` passe `Insets::ZERO` (constructeur de
  `Style` énuméré).

## Tests

- `margin_pushes_siblings_and_insets` : dans une colonne, un 2e enfant (haut 20) de
  marge 10 démarre à `y = 30` (frère 20 + marge 10) et est inséré à `x = 10`, sans
  que sa boîte grandisse (haut 20).
- Suites vertes : frus-layout 4, frus-widgets 199 ; workspace complet vert.

## Reste

- `Transform` (rotation/échelle/translation d'un enfant), `AspectRatio`,
  `FractionallySizedBox` — autres widgets de disposition de Flutter.
- Idiome shell / démo rassemblant l'arsenal (animations + alignement + marge).
