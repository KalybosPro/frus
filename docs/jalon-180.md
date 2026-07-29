# Jalon 180 — Formulaires : validation croisée & récapitulatif d'erreurs

## Analyse

Le module `form` validait chaque champ **isolément** (`Rule` ne voit qu'une valeur) et
exposait déjà `is_valid` / `error(key)` / `first_invalid()` (de quoi focaliser le premier champ
fautif). Deux besoins courants manquaient : (1) la **validation croisée** (un champ comparé à
un autre — « confirmer le mot de passe », « date de fin ≥ début ») ; (2) un **récapitulatif
d'erreurs** en tête de formulaire après une soumission invalide.

## Décisions techniques

- **Valeurs mémorisées → validation croisée.** `Form` retient désormais la **valeur** de
  chaque champ. `field_with(key, value, |value, form| …)` valide via une fonction qui reçoit le
  formulaire **partiel** (champs déjà déclarés) et peut consulter `form.value(other)`.
  `matches(key, value, other, message)` en est le raccourci (égalité stricte — confirmation).
  Le champ référencé doit être déclaré **avant** (validation en un passage, ordre de déclaration).

- **`errors()` + widget `ErrorSummary`.** `Form::errors()` renvoie `(clé, message)` dans
  l'ordre. Le widget `ErrorSummary::new(messages)` en fait une **carte teintée « erreur »**
  (titre « Please fix N error(s) » + une puce par message), **inerte** (aucun clic), avec
  `is_empty()` pour ne rien afficher quand tout est valide. Le module `form` reste **pur** pour
  la logique ; seul `ErrorSummary` dessine (widget dédié).

- **Focus au premier invalide : déjà outillé.** `first_invalid()` donne la clé à focaliser ;
  l'application la passe à `Command::focus(key)` à la soumission (le shell résout la clé contre
  l'arbre — jalon focus existant). Rien à ajouter côté framework.

## Implémentation

- `form.rs` : `Form` stocke `(clé, valeur, erreur)` ; `field_with` / `matches` / `value` /
  `errors` ; widget `ErrorSummary` (fond `surface.lerp(error)` + bord, lignes de texte).
- `lib.rs` : `pub use form::ErrorSummary`.
- `goldens.rs` : `form_error_summary` (récapitulatif au-dessus d'un champ en erreur).

## Vérification

- **Unitaire** : `cross_field_confirm_password` (`matches` + `field_with` avec `form.value`) ;
  `errors_lists_all_messages_in_order` (valides omis, ordre conservé) ; `error_summary_lists_messages`
  (titre + une puce par message, vide → `is_empty`). Les tests existants (`field`, `first_invalid`)
  et le doctest du module restent **verts**.
- **Golden** `form_error_summary` **inspecté** : carte « Please fix 2 errors » + puces, au-dessus
  du champ Email en erreur — aucune régression.
- `cargo test --workspace` **vert**.

## Reste

- **Récapitulatif cliquable** : cliquer une puce pour focaliser le champ correspondant
  (l'`ErrorSummary` porterait un message par item) — extension.
- **Règles inter-champs riches** au-delà de l'égalité (dépendances multiples) : déjà couvertes
  par `field_with`, à documenter par des recettes.
