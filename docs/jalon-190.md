# Jalon 190 — Assistant d'inscription intégré (démo bout en bout)

## Analyse

Beaucoup de briques récentes n'existaient que « en vitrine » (goldens isolés) : indicateur
`Steps` cliquable (182/183), formulaire `Form` + récapitulatif d'erreurs cliquable `ErrorSummary`
(180/181), notification `Toast`/`SnackbarQueue`/`ToastHost` (185/188). Il fallait **prouver
qu'elles s'assemblent en une app réelle** — pas juste côte à côte dans un test, mais reliées à un
état, une navigation et des messages. C'est le rôle de cette intégration dans `frus-demo`.

## Décisions techniques

- **Un assistant multi-étapes comme nouvelle route.** `Route::Wizard` s'ajoute à la pile
  d'écrans (accessible depuis le tiroir). L'état tient dans quelques champs de `TodoApp`
  (`wizard_step`, quatre valeurs, `wizard_submitted`) — `#[derive(Default)]` couvre l'init, aucun
  site de construction à toucher.

- **Chaque brique à sa place, reliée par des messages.**
  - `Steps(["Account","Security","Review"]).current(step).on_tap(Msg::WizardStep)` : l'indicateur
    **pilote** la navigation (marqueur cliqué → saut d'étape, jalon 183).
  - `Form` (pur) est **reconstruit à la volée** depuis l'état à chaque rendu **et** à la
    soumission — mêmes règles, une seule source de vérité (`wizard_form`). La validation croisée
    `matches` relie `confirm` à `password`.
  - Les erreurs des champs ne s'affichent **qu'après** une première soumission (`wizard_submitted`)
    — l'état « soumis vs en cours d'édition » évoqué au jalon 181, ici concret.
  - Sur l'étape Review, `ErrorSummary::links` transforme chaque erreur en **puce cliquable** qui
    saute à l'étape du champ fautif (`wizard_step_of` → `Msg::WizardStep`), reliant 181 et 183.
  - À la soumission valide, une **notification** de succès s'affiche via `ToastHost`
    (bas-centre, fondu d'entrée, jalon 188) et l'assistant se réinitialise.

- **La logique de flux est pure et testée.** `reduce` gère `WizardStep/Input/Back/Next/Submit` ;
  `Submit` bifurque sur `wizard_form(app).is_valid()` (notifier + reset, ou révéler les erreurs et
  aller à Review). Aucun état caché : tout dérive des champs.

- **Bonus vitrine.** Le toast existant du démo (« Saved ») passe lui aussi par `ToastHost`.

## Implémentation

- `frus-demo/src/lib.rs` : `Route::Wizard` (+ `save_state`/`restore_state`) ; 5 `Msg` ; champs
  `wizard_*` ; arms `reduce` ; `wizard_form` / `wizard_step_of` / `wizard_input` / `wizard_screen` ;
  entrée tiroir ; rendu du toast via `ToastHost`.
- `goldens.rs` : `wizard_review_errors` (étape Review avec récapitulatif d'erreurs — l'assemblage
  réel).

## Vérification

- **Intégration** (`wizard_flow_validates_navigates_and_notifies`) : l'écran se rend ;
  soumission vide → `submitted`, saut à Review, pas de toast ; remplissage valide → toast
  « Account created » + assistant réinitialisé ; navigation par étapes bornée. Les 16 tests démo
  existants restent **verts** (17 au total).
- **Golden** `wizard_review_errors` **inspecté** : `Steps` (Review), « Please fix 2 errors »
  cliquable, résumé, Back / Create account.
- `cargo build -p frus-demo` **propre** (zéro warning).

## Reste

- **Focus du champ fautif** (au-delà du saut d'étape) : câbler `Command::focus(key)` sur clic de
  puce — nécessite des clés de focus stables sur les `TextInput`.
- **Validation par étape** (bloquer « Next » tant que l'étape courante est invalide) — variante
  d'ergonomie.
- **Masquage du mot de passe** (`TextInput` obscurci) — fonctionnalité widget distincte.
