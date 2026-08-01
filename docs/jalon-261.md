# Jalon 261 — Finitions DnD : ombres `Card`/`Toast` thémées + test de réagencement même-colonne

## Analyse

Reliquats relevés aux jalons 255/256 : (1) `Card` et `Toast` peignaient leur ombre avec un **noir codé
en dur** (`Color::rgba(0,0,0,0.3)`), alors que `Button` et le fantôme de glisser (jalon 255) tirent la
leur de `theme.scheme.shadow` — incohérent avec la règle « customizable like Flutter » ; (2) le
réagencement vertical (`reflow_reorder_cards`) n'avait **pas** de test pour le cas **même colonne**
(source et cible dans la même bande x).

## Décisions techniques

- **Ombres thémées.** `Card` et `Toast` prennent `theme.scheme.shadow.with_alpha(0.30)` au lieu d'un
  noir littéral. `scheme.shadow` **étant** noir dans les thèmes fournis, le rendu est **identique** —
  dé-codage-en-dur, surchargeable via le thème (les widgets deviennent thémables).
- **Test même-colonne.** Documente le comportement correct d'un réagencement intra-colonne : la carte
  soulevée en haut, insertion après la 2e carte → la carte **au-dessus de la ligne remonte** d'un cran
  (comble le trou), celle **sous la ligne reste** (décalage `+cran`/`−cran` net **nul**), la place de
  dépôt s'ouvrant juste au-dessus d'elle. La colonne voisine ne bouge pas.
- **Inertie verticale : non retenue.** Le ressort horizontal (`reorder_x`) lisse le coulissement des
  colonnes voisines ; l'équivalent vertical serait un pur agrément **runtime** (non inspectable au GPU
  ici) au bénéfice marginal — laissé de côté (voir Reste) plutôt qu'ajouter du code non vérifiable.

## Implémentation

- `frus-widgets/src/card.rs` : ombre `theme.scheme.shadow` (import `Color` retiré, devenu inutile).
- `frus-widgets/src/toast.rs` : ombre `theme.scheme.shadow`.
- `frus-widgets/src/reorder.rs` : test `same_column_reflow_lifts_upper_cards_and_holds_the_rest`.

## Vérification

- **Widgets 393** (+1) ; **goldens 77 inchangés** (les ombres `Card`/`Toast` apparaissent dans des
  goldens — le dé-codage est pixel-identique, aucune régression) ; aucun avertissement.

## Reste

- Inertie/ressort **vertical** du coulissement des cartes (parité avec l'horizontal) — agrément runtime.
- Défilement **vertical par colonne** du Kanban (patron Flutter complet).
- Balayage overflow des autres écrans (Data table, Grid, Charts, Wizard — audit fait).
