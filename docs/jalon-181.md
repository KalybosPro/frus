# Jalon 181 — Formulaires : récapitulatif d'erreurs cliquable

## Analyse

Le widget `ErrorSummary` (jalon 180) listait les erreurs d'un formulaire mais restait **inerte** :
un utilisateur voyant « • Enter a valid email address » ne pouvait pas cliquer dessus pour
sauter au champ fautif — il devait le retrouver à la main. Sur un long formulaire, le
récapitulatif doit servir de **table des matières** : cliquer une puce focalise le champ.

## Décisions techniques

- **Puces = widgets, plus des `Text`.** Chaque puce devient un petit widget `Bullet` (privé) :
  même rendu qu'avant (texte `on_surface` sur la carte teintée) mais porteur d'un `on_click`.
  `ErrorSummary::new(messages)` garde des puces **inertes** (`message: None`) ;
  `ErrorSummary::links([(message, msg), …])` en fait des puces **cliquables** qui émettent
  `msg` — typiquement `Msg::FocusField(key)` que l'application traduit en `Command::focus(key)`.
  Les deux constructeurs partagent `assemble()` (titre « Please fix N error(s) » + puces).

- **Cliquable = focalisable + surbrillance.** Une puce cliquable est `focusable()` et expose une
  sémantique `Role::Button` (navigation clavier + lecteurs d'écran) ; elle peint une surbrillance
  discrète (`error.fade(0.12)`) pilotée par `status.hover_progress`/`focus_progress`. Une puce
  inerte n'est **ni** focalisable **ni** cliquable — le récapitulatif purement informatif reste
  identique au jalon 180 (golden inchangé à l'œil).

- **La liaison puce → champ reste applicative.** Le framework ne « connaît » pas les champs :
  l'application fournit le `Msg` par puce (souvent `Form::errors()` zippé avec les clés) et
  focalise via le mécanisme de focus existant. `ErrorSummary` reste un widget de présentation.

## Implémentation

- `form.rs` : `ErrorSummary::links` + `assemble()` ; widget privé `Bullet { label, message }`
  (`style` pleine largeur, `paint` texte + surbrillance conditionnelle, `on_click`, `focusable`,
  `semantics`).

## Vérification

- **Unitaire** : `error_summary_links_emit_focus_messages` — le titre n'est pas cliquable, chaque
  puce émet son `Msg` dans l'ordre et est focalisable ; la variante `new` reste inerte (ni clic,
  ni focus). Tests existants (`error_summary_lists_messages`, validation) **verts**.
- **Golden** `form_error_summary` régénéré et **inspecté** : carte « Please fix 2 errors » + deux
  puces claires au-dessus du champ Email — aucune régression visuelle.
- `cargo test -p frus-widgets form::` **vert**.

## Reste

- **État « soumis » vs « en cours d'édition »** : n'afficher le récapitulatif / les erreurs
  qu'après une première soumission — pilotage purement applicatif (un `bool submitted`), à
  documenter par une recette plutôt qu'à coder dans le framework pur.
- **Puce → surlignage du champ** (au-delà du focus) : l'application peut aussi teinter brièvement
  le champ ciblé (déjà possible via l'état d'interaction).
