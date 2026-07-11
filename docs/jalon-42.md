# Jalon 42 — Responsivité par défaut

Rend l'adaptation à la taille **facile et par défaut**, en trois primitives
complémentaires (chacune construite et testée séparément).

## Lot A — Classes de taille (`SizeClass`)

`frus-core` : `SizeClass { Compact, Medium, Expanded }` (breakpoints Material 3,
px logiques : < 600 / 600–840 / ≥ 840), `SizeClass::from_width(w)`, `rank()`.

`frus-widgets` : le widget `Responsive` — `responsive(width).compact(a).medium(b)
.expanded(c)` — choisit un sous-arbre selon le palier, avec **repli gracieux**
(palier le plus proche, en préférant plus petit à égalité d'écart). Il **délègue
tout** à la variante choisie (comme `Keyed`), donc s'insère n'importe où.

## Lot B — `Wrap` (flex-wrap)

`Style.flex_wrap: bool` → `taffy::FlexWrap::Wrap`. `Flex::wrap()` l'active ;
`Wrap::new()` est le point d'entrée nommé (rangée qui passe à la ligne). Les
enfants qui débordent l'axe principal **refluent** sur une nouvelle ligne, sans
breakpoint. Hauteur pilotée par le contenu (vraie mise en page multi-lignes) :
c'est le bon outil pour « barre d'actions / tuiles 3→2→1 ».

## Lot C — `LayoutBuilder`

`LayoutBuilder::new(|size| widget)` construit son contenu **à partir de sa boîte
réelle** (façon Flutter `LayoutBuilder`), pas seulement selon la fenêtre : un
composant s'adapte quel que soit l'endroit où il est placé. Même mécanique que la
liste virtualisée — feuille de layout, contenu construit à la volée, rendu via
`render_item` — donc **pas d'état retenu** (survol/clic OK, pas de focus persistant
ni d'overlay différé) et **taille propre = son style** (fixez hauteur/`flex`).

Choisir le bon primitif : **`Wrap`** pour un reflow à hauteur automatique,
**`Responsive`** pour brancher sur la classe de la fenêtre, **`LayoutBuilder`**
pour brancher sur la boîte réelle mesurée (à hauteur fixe).

## Démo

Carte de tâches responsive : largeur par palier (Lot A), en-tête dont les
boutons d'action **refluent** en `Wrap` (Lot B), et ligne de résumé en
`LayoutBuilder` qui raccourcit son texte quand la boîte est étroite (Lot C). Les
champs internes (saisie, barre de progression) suivent la largeur de carte.

## Tests

- `frus-core` : seuils + ordre de `SizeClass`.
- `frus-widgets` : `Responsive` (choix par largeur, repli), `Wrap` (style), et
  `LayoutBuilder` (reçoit sa boîte réelle, adapte le nombre de tuiles).
- `frus-layout` : `flex_wrap` déplace réellement l'enfant qui déborde à la ligne
  suivante (test fonctionnel de reflow).

## Limites (v1)

- `LayoutBuilder` a une hauteur fixe (feuille) : il ne mesure pas son propre
  contenu — utilisez `Wrap` quand la hauteur doit suivre le reflow.
- `Wrap` : lignes de même hauteur (pas de packing type masonry).
