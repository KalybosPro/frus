# Jalon 193 — Snackbar : sortie animée + file branchée

## Analyse

Le démo n'affichait qu'**une** notification à la fois (`toast: Option<String>`), sans
transition de **sortie** : elle disparaissait d'un coup après 2 s. `SnackbarQueue` (jalon 185)
existait mais n'était pas branchée, et `ToastHost` (188) ne savait qu'entrer en fondu. Il manquait
la **sortie animée** et la **file réellement câblée** (plusieurs notifications qui s'enchaînent).

## Décisions techniques

- **Phase « en sortie » dans la file (framework).** `SnackbarQueue` gagne un drapeau par entrée :
  `start_leaving()` marque la tête en sortie, `is_leaving()` l'expose — la notification **reste
  visible** pendant que l'hôte joue son fondu, puis `dismiss()` la retire. `tick`/`dismiss`/`push`
  gardent leur API (jalon 185 intact) ; seul le tuple interne passe à trois champs.

- **`ToastHost::fade_out` (framework), symétrique de `fade_in`.** Anime l'opacité de groupe vers
  **0** (via `AnimatedOpacity`, couche d'animation existante) — le toast s'efface avant son
  retrait. Les deux passent par un `wrap_opacity(target, duration)` commun.

- **File branchée au démo, pilotée par commandes minutées.** `app.toast: Option<String>` devient
  `app.snackbars: SnackbarQueue<String>` (`#[derive(Default)]` couvre l'init). Le cycle est mené
  par trois messages :
  `show_toast` empile et, si la notification devient tête, programme `ToastExpire` (~2 s) →
  `start_leaving` + programme `DismissToast` (~0,3 s, le temps du fondu) → `dismiss` puis, s'il
  reste des notifications, reprogramme `ToastExpire`. Le rendu choisit `fade_out` quand
  `is_leaving()`, `fade_in` sinon. Plusieurs `Save`/inscriptions **s'empilent** et défilent une à
  une.

## Implémentation

- `toast.rs` : `SnackbarQueue` — `start_leaving` / `is_leaving`, tuple `(T, f32, bool)`.
- `toasthost.rs` : `fade_out` + `wrap_opacity` partagé.
- `frus-demo/src/lib.rs` : champ `snackbars`, `Msg::ToastExpire`, helpers `show_toast` /
  `toast_expire_after`, arms `Save`/`ToastExpire`/`DismissToast`/`WizardSubmit`, rendu via
  `current()`/`is_leaving()`.

## Vérification

- **Unitaire (framework)** : `leaving_phase_precedes_dismissal` (marque en sortie sans retirer,
  puis retrait) ; `fade_out_wraps_children`. Le test `queue_shows_one_at_a_time_and_expires`
  (jalon 185) reste **vert**.
- **Intégration (démo)** : `snackbar_queue_orders_and_exits` — deux notifications empilées, la
  tête passe en sortie puis cède la place à la suivante, file vidée. Les 17 autres tests démo
  restent verts (18 au total).
- `cargo test -p frus-demo -p frus-widgets` **vert, zéro warning**.

## Reste

- **Auto-tick** (souscription temps réel) au lieu des commandes minutées — plus fluide pour des
  durées variables, mais l'approche minutée suffit ici.
- **Sortie par glissement** (translation + fondu) — via la couche d'animation (translation
  animée), en plus de l'opacité.
