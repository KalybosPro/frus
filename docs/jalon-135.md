# Jalon 135 — Validation de formulaire groupée (pure, côté app)

## Analyse

Les jalons 132–134 ont donné au champ de quoi **afficher** une erreur (`error(...)`),
mais rien pour la **décider**. Chaque `view` devait recalculer sa validité à la main.
Il manquait la contrepartie logique : de quoi valider un **ensemble** de champs, savoir
si tout passe, récupérer l'erreur de chaque champ (pour alimenter `error(...)`) et
repérer le premier champ en échec (à focaliser).

Flutter place ça dans `FormState.validate()` avec un `validator` par champ. Mais son
modèle repose sur un état mutable (`GlobalKey<FormState>`). En architecture Elm, la
validité est une **fonction pure de l'état** : on fournit des *combinateurs* purs, pas un
objet à muter.

## Décisions techniques

- **Deux briques pures, zéro dessin.** `Rule` (une règle `&str -> Option<String>`) et
  `Form` (un rapport sur un ensemble de champs). Le module ne connaît ni widget ni GPU ;
  l'application appelle `Form::error(key)` pour nourrir le `error(...)` d'un `TextInput`.

- **Des règles composables.** Constructeurs prêts (`required`, `min_len`, `max_len`,
  `email`) et un combinateur `Rule::all([...])` où la **première** règle en échec gagne —
  l'ordre porte le sens (« obligatoire » avant « format »).

- **Un rapport ordonné et interrogeable.** `Form::field(key, value, rule)` valide sur le
  champ et empile `(key, erreur?)` dans l'ordre déclaré. On interroge ensuite :
  `is_valid()`, `error(key)`, `first_invalid()` (la clé du premier en échec — à focaliser
  ou mettre en avant).

- **Clés `&'static str`.** Identifiants de champ stables et lisibles, sans allocation ;
  l'application relie chaque clé à son état et à son widget.

- **`email` = heuristique, pas la RFC.** `local@domaine`, partie locale non vide, domaine
  avec au moins un point et aucune étiquette vide. Suffisant pour un champ de saisie,
  sans le piège d'une regex RFC 5322.

## Implémentation

- `crates/frus-widgets/src/form.rs` : `Rule` (+ constructeurs, `all`), `is_email`, `Form`
  (`field`/`is_valid`/`error`/`first_invalid`). Tests des règles, du combinateur, du
  rapport, et un doctest d'usage.
- `crates/frus-widgets/src/lib.rs` : `pub mod form;` (accès namespacé `form::{Rule, Form}`
  — noms trop génériques pour la racine du crate).
- `crates/frus-test/tests/goldens.rs` : golden `validated_signup_form` — un `Form` valide
  des valeurs saisies et **pilote le `error(...)` de chaque champ**, rendu après une
  soumission invalide (bout-en-bout jalons 132→135).

## Vérification

- **Bout-en-bout à l'œil** : « ada » déclenche « Enter a valid email address » (non vide
  mais pas un e-mail → `all` renvoie la 2ᵉ règle) ; le mot de passe masqué « short »
  déclenche « At least 8 characters ». Les deux champs en rouge, labels flottés. Figé en
  golden `validated_signup_form.png`.
- **Unitaires + doctest** : règles (blanc, longueurs, e-mail), `all` (première erreur),
  rapport (`is_valid`/`error`/`first_invalid`), formulaire vide valide.
- **Suites** : `frus-widgets` + `frus-test` verts.

## Reste

- **Focaliser le premier champ invalide** : `first_invalid()` donne la clé ; commander le
  focus depuis l'application (mapper clé → `WidgetId`) reste à câbler côté shell.
- Règles supplémentaires au besoin (numérique, plage, correspondance de deux champs pour
  « confirmer le mot de passe »).
