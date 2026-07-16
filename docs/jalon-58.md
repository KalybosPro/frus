# Jalon 58 — Thème : state-layers Material bakées + rôles M3 étendus

Suite du système de design (§5). Le thème de Frus était un **sac plat de couleurs**
et chaque widget réinventait ses états d'interaction (survol/pression) à la main —
`Button` éclaircissait au survol, assombrissait à la pression, et codait sa variante
`Danger` **en dur** (hors thème). Ce jalon introduit la **règle d'états bakée** de
Material et commence l'**élargissement des rôles**, de façon strictement additive
(les ~130 accès aux champs plats existants restent intacts).

## State-layer bakée dans le thème

`Theme::state_layer(base, on, status)` superpose la couleur de contenu `on` sur
`base` à faible opacité, selon l'état : **survol 8 %**, **focus 10 %**, **pression
12 %**, en tenant compte des progressions animées (`hover_progress`/
`focus_progress`). C'est la règle « rôle→couleur selon `Status` » du brief, **bakée
dans le thème** : le widget reste déclaratif (il fournit sa couleur de base et sa
couleur de contenu ; le thème décide de l'overlay), et tous les widgets partagent la
même sensation d'états.

## Rôles M3 étendus (clair/sombre écrits à la main)

Cinq rôles ajoutés au `Theme`, interpolés au fondu de thème (`lerp`) :
`primary_container` / `on_primary_container` (surfaces tonales douces), `error` /
`on_error` (danger thémé), `outline_variant` (contour discret). Les champs plats
existants (`background`, `surface`, `primary`, `on_surface`, `muted ≈
on_surface_variant`, `border ≈ outline`, …) gardent **exactement** leurs valeurs :
zéro régression visuelle sur les 60+ widgets qui les utilisent.

## Adoption : `Button`

`Button` abandonne sa logique d'états ad hoc au profit de `theme.state_layer(base,
on, &status)`, et sa variante `Danger` référence désormais les rôles `error` /
`on_error` au lieu d'une couleur codée en dur. Les rôles et la state-layer sont donc
**réellement employés**, pas de l'infrastructure morte. Les tests de `Button`
(qui vérifient le message de clic, pas les couleurs) passent inchangés.

## Validation

- `frus-widgets` : **130 tests** (+1 : `state_layer` — repos neutre, survol tire de
  8 % vers le contenu, pression plus forte que le survol).
- Reste vert : `frus-core` 46, `frus-demo` 15, shell 7, gpu 4 (readback offscreen),
  layout 3, text 2. `cargo build --workspace` sans avertissement.
- Rendu non observable sous WSLg-root ; le changement d'aspect de `Button` (états
  Material corrects) est intentionnel et conforme au spec, non pinné par un test.

## Suite (§5, vers la `ColorScheme` complète)

- Généraliser l'adoption de `state_layer` aux autres widgets à survol thémé
  (checkbox, switch, list rows, menu items…), au fil de l'eau.
- Compléter les rôles (`secondary`/`tertiary`, `surface_container*`,
  `inverse_surface`, `scrim`) puis regrouper sous une vraie `ColorScheme` ; ajouter
  `TextTheme` (15 slots) avec `TextStyle`, et `from_seed` (HCT) plus tard.
- Câbler `BoxDecoration::content_padding` (jalon 57) dans taffy.
