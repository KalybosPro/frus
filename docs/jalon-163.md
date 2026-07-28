# Jalon 163 — Réordonnancement : inertie douce & en-têtes annoncés

## Analyse

Le coulissement des colonnes (jalon 161) **collait au curseur** : réactif, mais un peu
sec (aucune détente quand le doigt s'arrête ou saute). Deux points du « Reste » : une
**inertie** (ressort) et l'**accessibilité** du réordonnancement.

## Décisions techniques

- **Ressort à un seul état, pas par colonne.** Plutôt qu'un offset animé par colonne
  (lourd), on **lisse l'abscisse du curseur** : un état `reorder_x` rejoint la position
  réelle par un **ressort exponentiel** (constante de temps ~70 ms), et c'est **lui** qui
  nourrit le réagencement géométrique. Les colonnes coulissent donc avec une **inertie
  douce**, tandis que le **fantôme colle au curseur réel** (il « précède », le fond
  « rattrape ») — sensation Material, pour un coût minime. Le ressort est
  **cadence-indépendant** (`1 − e^{−dt/τ}`) et sans dépassement ; la frame reste
  « animée » tant qu'il n'est pas stabilisé.

- **En-têtes annoncés.** Chaque en-tête porte désormais une **sémantique** (rôle bouton +
  libellé) ; s'il est réordonnable, sa **valeur** indique « column N of M ». Un lecteur
  d'écran énonce donc la colonne **et sa position** : en re-parcourant après un
  déplacement (souris ou Ctrl+Flèches), l'utilisateur **perçoit** le nouvel ordre. Les
  cellules de données restent muettes (pas de bruit).

## Implémentation

- `app.rs` (shell) : champ `reorder_x` ; initialisé au curseur au début du glissement ;
  **ressort** avancé dans la boucle d'animation (`spring_toward`, fonction pure) et injecté
  dans `reflow_reorder_columns` ; la frame reste animée jusqu'à stabilisation.
- `table.rs` : `Cell::semantics` (en-têtes : rôle + libellé + « column N of M » si
  réordonnable).

## Vérification

- **Unitaire** : `spring_toward` — approche **monotone**, **bornée** (pas de dépassement),
  quasi atteinte après ~0,5 s. `Cell::semantics` — en-tête « B » annoncé
  `label="B"`, `value="column 2 of 3"` ; cellule de données **muette**.
- L'inertie est un effet **temporel interactif** (boucle de rendu), non golden-able ; sa
  loi est isolée et testée. Le golden `table_reorder_preview` (réagencement direct) reste
  inchangé.
- `cargo test --workspace` **vert**, sans avertissement.

## Reste

- **Annonce vocale « live »** du déplacement (« déplacé en position 3 ») : demande une
  **région live** AccessKit dédiée (au-delà de l'arbre sémantique passif actuel).
- **Détente au dépôt** : petit ressort de la carte fantôme vers sa position finale.
