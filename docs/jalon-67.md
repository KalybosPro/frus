# Jalon 67 — Adoption du par-coin (feuille, segments) + la bordure réserve sa place

Jalon de finition : les rayons par-coin du jalon 66 entrent dans les widgets où ils
sont l'aspect **correct**, et la règle `content_padding` du brief (§5) est câblée
dans la mise en page.

## `Button::radius(…)` (règle « personnalisable comme Flutter »)

Le rayon du bouton était un emprunt direct du thème, sans surcharge. Nouveau
builder `radius(impl Into<BorderRadius>)` (défaut : rayon du thème) — l'ombre
suit (`inflate`). C'est la brique des groupes de boutons connectés.

## `SegmentedControl` : segments réellement « connectés »

Chaque segment n'est plus une pilule uniforme : seuls les coins **extérieurs** du
groupe sont arrondis (premier à gauche, dernier à droite, jointures droites,
segment unique = uniforme). Rayon extérieur surchargable (`.radius(f32)`, défaut
10). Épinglé par `segments_round_only_the_outer_corners` (les remplissages des
trois segments portent exactement gauche-arrondi / droit / droite-arrondie).

## `BottomSheet` : coins hauts arrondis

Le panneau de la feuille peignait une surface carrée ; il a désormais les coins
**hauts** arrondis (`BorderRadius::top(theme.radius + 6)`, le bord bas restant
collé à la fenêtre), avec le liseré haut en retrait des arrondis. Les tests
épinglés de la feuille (glissement à mi-course, géométrie) passent inchangés.

## La bordure réserve sa place (`content_padding` → taffy)

Une `Container` bordée **réserve l'épaisseur du trait** dans son padding de mise
en page : le contenu n'est plus mangé par la bordure (la règle que
`BoxDecoration::content_padding` documentait sans être câblée). Une bordure
invisible (épaisseur nulle ou alpha nul) ne change rien. Épinglé par
`visible_border_reserves_layout_padding`. Impact : les trois conteneurs bordés de
la démo voient leur contenu s'écarter d'1 px — le comportement correct.

## Validation

- **240 tests**, tout vert — dont les tests épinglés de BottomSheet/Drawer
  inchangés, et les 2 nouveaux (coins extérieurs des segments, réserve de
  bordure). Build sans avertissement ; démo sans panique.

## Suite (§5 restants)

Consolidation `ColorScheme` (+ `from_seed` HCT), décorations de texte,
`letter_spacing`/`line_height`, `Alignment`, RTL (§14).
