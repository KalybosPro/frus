# Jalon 264 — Défilement vertical par colonne (façon Trello), via hauteur explicite

## Objectif

Compléter le patron Trello amorcé aux jalons 258/260 : le board défile **horizontalement** (rangée de
colonnes), et **chaque colonne** défile ses cartes **verticalement**, indépendamment. Le jalon 263
avait constaté que l'approche naturelle (`Scroll` en `flex(1)`) **s'effondre** faute d'une hauteur
d'ancêtre définie — les cartes disparaissent et ne sont plus réordonnables. Ce jalon livre la
fonctionnalité par le **stopgap documenté** : une hauteur **explicite** fournie par l'application.

## Décision : hauteur explicite fournie par l'app (façon Flutter)

Plutôt que d'attendre une primitive « fill-then-scroll » dans le moteur de layout, l'**application**
fournit la hauteur de la zone de cartes — comme Flutter demande souvent une contrainte de hauteur
définie pour un `ListView` imbriqué (`SizedBox`, `Expanded` dans une `Column` bornée…). C'est
**contrôlé** et surchargeable : sans l'appel, la colonne garde son comportement d'origine (hauteur du
contenu).

## Implémentation

- **`frus-widgets/src/kanban.rs`** :
  - `Kanban::card_area_height(h)` (nouveau) : rend les cartes de chaque colonne **défilables
    verticalement** dans une région de hauteur `h`. `build_column` compose alors la colonne en trois
    zones — **titre fixe** au-dessus, **cartes + zone de dépôt** dans un
    `Scroll { axis: Vertical, height: h }`, **bouton « + Add card » fixe** en dessous. Sans l'appel
    (`card_area_height == None`), les cartes restent des enfants directs de la colonne (inchangé).
  - Constante `COL_PAD = 12` (extraite du padding du panneau) : sert au calcul de la largeur
    intérieure `COL_W − 2·COL_PAD` donnée au `Scroll` et à sa liste.
- **`frus-widgets/src/flex.rs`** : `Flex::child_boxed(Box<dyn Widget>)` (nouveau) — ajoute un enfant
  déjà boxé, pour composer une liste construite dynamiquement (`Vec<Box<dyn Widget>>`).
- **`frus-demo/src/lib.rs`** (`board_screen`) : calcule `card_area = (height − BOARD_CHROME).max(160)`
  (réserve navbar + hint + paddings + titre + bouton d'ajout ; plancher pour ne jamais s'effondrer sur
  petit écran) et le passe via `.card_area_height(card_area)`.

## Vérification

- **Desktop** : compile ; widgets **395** (dont le nouveau garde-fou), kanban inchangé, goldens **77**
  inchangés.
- **Garde-fou (unitaire)** : `reorderables_inside_a_per_column_card_scroll_are_still_registered` — une
  colonne à `card_area_height` définie place ses cartes dans un `Scroll` **vertical à hauteur définie**
  (le cas même qui s'effondrait au jalon 263) ; les cartes visibles restent **réordonnables**
  (≥ 3 : 2 cartes + zone de dépôt). Complète `reorderables_inside_a_scroll_are_still_registered`
  (board dans un scroll horizontal, jalon 263).
- **Appareil** : à confirmer au doigt (défilement vertical d'une colonne + glisser d'une carte dans une
  colonne défilée). Le rendu et le défilement effectifs ne sont vérifiables que sur GPU/appareil.

## Limite connue

La hauteur est **explicite** (fournie par l'app), pas encore dérivée d'un « remplir la hauteur
disponible puis défiler » automatique. Les cartes défilées **hors** de la région visible ne sont pas
enregistrées comme réordonnables (attendu : on ne dépose pas sur une carte hors écran sans défiler
d'abord). Une primitive fill-then-scroll fiable dans le moteur de layout rendrait le stopgap inutile.

## Reste

- Primitive « fill-then-scroll » dans le layout (supprimerait le besoin d'une hauteur explicite).
- Inertie verticale du glisser (parité avec le ressort horizontal).
