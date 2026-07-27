# Jalon 141 — Flèches Haut/Bas dans le champ multi-lignes

## Analyse

Dans un champ multi-lignes, Haut/Bas devaient déplacer le caret d'une ligne à l'autre.
Or le shell traitait ces touches comme une **navigation géométrique du focus** — même
depuis un champ texte —, si bien qu'on quittait le champ au lieu d'y monter/descendre.

## Décisions techniques

- **Le champ décide, le shell arbitre.** Une méthode `Widget::caret_vertical(width,
  cursor, down)` rend le **nouvel index** si le caret peut changer de ligne **dans** le
  champ (même colonne visuelle), ou `None` s'il est déjà à la première (Haut) ou dernière
  (Bas) ligne — ou si ce n'est pas un champ multi-lignes. Le shell essaie d'abord ce
  déplacement ; sur `None`, il retombe sur la **navigation du focus** (on quitte le
  champ). Un même code gère donc « bouger dans le champ », « sortir par le haut/bas » et
  « champ mono-ligne » (toujours `None` → navigation).

- **Même colonne, via la layout 2D.** Le champ shape sa layout repliée, prend le caret
  courant `(x, y)`, vise le milieu de la ligne voisine à **la même `x`**, et `hit_test`
  y trouve l'index. La colonne visuelle est ainsi préservée à la montée/descente.

- **Sélection au Shift.** Comme pour Gauche/Droite, `Shift`+Haut/Bas **étend** la
  sélection (ancre posée au départ) ; sans Shift, simple déplacement (ancre effacée).
  Le déplacement **révèle le caret** (jalon 139), donc la ligne visée défile au besoin.

- **Retrouver la géométrie du champ focalisé.** Le déplacement vertical a besoin de la
  largeur du champ (pour le repli). Un accesseur `Ui::widget_rect(id)` la fournit depuis
  les focusables de la frame (pas seulement les zones défilables : un champ court non
  défilant navigue aussi ses lignes).

## Implémentation

- `widget.rs` (+ relais `Box`/`Keyed`/`Responsive`) : méthode `caret_vertical`.
- `textinput.rs` : impl `caret_vertical` (layout repliée → `hit_test` à même colonne,
  `None` aux bornes / hors multi-lignes).
- `ui.rs` : accesseur `Ui::widget_rect(id)`.
- `app.rs` : le bloc flèches tente d'abord `caret_vertical` (Haut/Bas), applique le
  déplacement (+ sélection au Shift, + `reveal_caret`), sinon navigue le focus.

## Vérification

- **Unitaire** : depuis la 1re ligne, Bas descend d'une ligne, Haut rend `None` ; depuis
  la dernière, Bas rend `None` ; depuis la 2e, Haut remonte ; un champ mono-ligne rend
  toujours `None`.
- **Non-régression** : la navigation du focus par flèches reste intacte hors champ
  multi-lignes ; `cargo test --workspace` vert.

## Reste

- **Colonne cible « mémorisée »** : en traversant des lignes plus courtes, la colonne
  idéale devrait être retenue (comportement d'éditeur) — ici on repart de la colonne
  courante à chaque saut.
- **Page préc./suiv.** (PgUp/PgDn) et **Ctrl+Début/Fin** dans le champ multi-lignes.
