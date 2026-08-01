# Jalon 258 — Respect du viewport : board Kanban défilable + texte enroulé (fin du débordement)

## Analyse

Constat **sur appareil** : des éléments **sortent de l'écran**. Deux cas concrets sur l'écran Kanban
(téléphone ~393 px logiques de large) :
1. Le **board** est une rangée de colonnes de **largeur fixe** (`COL_W = 220` × 3 + gaps ≈ 452 px)
   **sans défilement** → la ou les dernières colonnes débordent à droite, hors écran.
2. Le **hint** est un texte **une seule ligne** non enroulé → il déborde à droite (« …drag a card to
   mo— » coupé).

Comme Flutter, le framework doit **borner le contenu au viewport** et le rendre **défilable** quand il
dépasse, et **enrouler** le texte.

## Décisions techniques

- **Board défilable en 2D.** Le board est placé dans un `Scroll { axis: Both, width: viewport,
  flex: 1 }` qui **remplit l'espace** sous la barre (même patron que l'écran Settings :
  `Scroll::new().width(width).flex(1.0)`). Contenu plus large **ou** plus haut que le viewport → il
  défile. Le `padding` est **dans** le contenu défilé (marge visuelle conservée).
- **Cohabitation glisser/défiler.** Inchangée et correcte : à l'appui sur une **carte**, le shell arme
  `Drag::Reorder` (jalon 250) **avant** le repli de défilement tactile (gardé par `drag.is_none()`,
  jalon 254) ; à l'appui sur une **zone vide**, c'est un défilement. Donc glisser une carte réordonne,
  glisser le vide fait défiler — comportement attendu.
- **Texte enroulé.** Le hint utilise `Text::wrap()` (déjà offert par le widget : `measure_wrapped` à la
  largeur proposée) ; posé dans un `Container` de la largeur de l'écran → il s'enroule sur 2 lignes au
  lieu de déborder.

## Implémentation

- `frus-demo/src/lib.rs` : `board_screen` — board dans `Scroll::new().axis(Axis::Both).width(width)
  .flex(1.0)`, hint `.wrap()` dans un `Container` pleine largeur ; import `Axis`.

## Vérification

- **Desktop** : compile ; démo (lib) 36 (logique de réduction inchangée).
- **Appareil** (Huawei STK-L21) : **confirmé par capture** — barre de défilement horizontale présente ;
  défiler révèle successivement les colonnes « To do », « Doing », « Done » (plus rien hors écran) ; le
  hint s'affiche sur **2 lignes**, non coupé.

## Notes

- La règle générale (« borner au viewport + défiler ») s'applique au-delà de cet écran ; ce jalon
  traite le cas **signalé** (Kanban). Balayage des autres écrans à la demande.
- Le patron `Scroll{Both, width, flex(1)}` est le pendant multi-axes du patron vertical déjà utilisé
  (Settings) — bon candidat à un helper « écran défilable » si le besoin se répète.

## Reste

- Balayer les autres écrans pour tout débordement résiduel (largeur des tableaux, longues étiquettes…).
- Contrat de **cycle de vie** façon Flutter (enum d'états + hook `on_lifecycle`, câblé sur
  `resumed`/`suspended` + Android `onPause`/`onStop`).
- Couverture réagencement même-colonne ; inertie verticale ; ombre `Card`/`Toast` sur le thème.
