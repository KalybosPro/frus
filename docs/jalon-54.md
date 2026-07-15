# Jalon 54 — Couche d'animation atteignable + transitions du démo dessus

Le jalon 53 a posé la couche `frus_core::animation` (physique, courbes, pilote),
mais **une application ne pouvait pas l'atteindre** : le démo dépend de
`frus-widgets`/`frus-shell`, qui ne réexportaient pas ces types. Le pilote
(`AnimationController`) restait donc théorique. Ce jalon le rend utilisable **et**
le prouve de bout en bout dans l'app réelle.

## `AnimationController` plus ergonomique + atteignable

- **`AnimationController::spring_to(target, spring, velocity)`** : anime la valeur
  courante vers `target` par un ressort amorcé par la vitesse courante — le chemin
  des transitions **interruptibles amorcées par un geste** (détente d'un
  glissement, retour amorcé par l'élan du doigt), sans que l'appelant ait à
  construire et boxer une `SpringSimulation` à la main.
- **`AnimationController` implémente `Default`** (contrôleur `[0,1]` au repos), pour
  vivre dans un modèle d'app `#[derive(Default)]`.
- **Ré-export** de toute la couche via `frus-widgets` (donc atteignable par les
  apps sans dépendre directement de `frus-core`). `frus_core::Status` (avancement
  d'animation) y est renommé **`AnimationStatus`** pour ne pas masquer le `Status`
  d'interaction (état de peinture).

## Le démo pilote ses transitions par `AnimationController`

Rappel du modèle Elm de Frus : **l'app possède son état d'animation et l'avance
dans `tick(dt)`** (option « B » du brief, adaptée). Le démo abandonne son
intégrateur d'Euler ad hoc (`spring_step`) et son bookkeeping manuel
(`nav_progress`/`nav_velocity`, `BackGesture.settling`) au profit du pilote
partagé :

- **Transition d'écran** : un `AnimationController` poussé de `0 → 1` par
  `spring_to` (ressort ~critique `k=220, c=30`). `tick` échantillonne, la vue lit
  `nav.value()` au paint.
- **Geste retour** : suivi au doigt pendant le glissement, puis **détente à
  ressort amorcée par l'élan du doigt** (`spring_to(cible, spring, velocity)`) — la
  forme close gère l'overshoot ; le contrôleur s'arrête net au bord (`[0,1]`), ce
  qui **valide** (`1`) ou **annule** (`0`) le dépilement.

Le ressort partagé est désormais exprimé une fois (`nav_spring()` →
`SpringDescription`), au lieu d'être dispersé en constantes passées à un stepper.

## Validation

- `frus-core` : **37 tests** (+`spring_to`).
- `frus-widgets` : **122 tests** ; `frus-demo` : **15 tests**, dont
  `back_gesture_flick_commits_pop` — le flick rapide **dépile toujours** l'écran
  après la migration (le pilote atteint la cible et valide).
- `cargo build --workspace` sans avertissement ; démo lancée sans panique
  (transition + geste opérationnels).

## Note d'architecture

Frus ne construit **pas** de registre de contrôleurs côté shell piloté par
`Command` (l'autre variante du brief) : son modèle « l'app avance ses animations
dans `tick` » atteint le même but — état d'animation retenu, hors de la vue pure,
lu au paint — sans nouvelle machinerie dans le shell. Les animations
d'interaction du framework (survol/focus/opacité/valeur/scroll) restent gérées par
le `Runtime` ; leur migration éventuelle vers `Simulation` changerait la sensation
et se fera sous goldens (cf. jalon 53).
