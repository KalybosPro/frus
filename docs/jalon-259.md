# Jalon 259 — Contrat de cycle de vie de l'application (façon Flutter)

## Analyse

Le framework gérait le cycle de vie de la **surface** (winit `resumed`/`suspended` : (re)création et
destruction du renderer/fenêtre — indispensable sur Android où la surface GPU est invalide en
arrière-plan), mais **n'exposait rien à l'application**. Contrairement à Flutter
(`didChangeAppLifecycleState`), le `Application` ne pouvait pas réagir au passage premier plan ↔
arrière-plan (suspendre un minuteur/capteur, persister avant fermeture).

## Décisions techniques

- **Enum `Lifecycle`** (façon `AppLifecycleState`) : `Resumed` (premier plan, interactif), `Inactive`
  (visible mais non focalisé), `Paused` (arrière-plan, surface perdue), `Detached` (fermeture
  imminente).
- **Hook `Application::on_lifecycle(state)`** (défaut : rien).
- **Notification aux transitions seulement.** Le shell mémorise l'état courant (`lifecycle`) et
  n'appelle `on_lifecycle` qu'au **changement** (`set_lifecycle`).
- **Câblage** : `resumed` → `Resumed` ; `suspended` → `Paused` ; nouveau `exiting` → `Detached` ;
  `WindowEvent::Focused(true/false)` → `Resumed`/`Inactive` **sans** écraser `Paused`/`Detached`
  (le premier plan décide du focus, l'arrière-plan/fermeture décide du reste).

## Implémentation

- `frus-shell/src/application.rs` : `enum Lifecycle` + `fn on_lifecycle` ; export dans `lib.rs`.
- `frus-shell/src/app.rs` : champ `lifecycle` (init `Detached`), `set_lifecycle` (change-tracké),
  appels dans `resumed`/`suspended`/`exiting` et l'arm `WindowEvent::Focused`.
- `frus-demo/src/lib.rs` : `on_lifecycle` trace l'état et met `background = Paused|Detached` ; la
  **souscription** du chrono est gardée par `!background` → le minuteur se **suspend** en arrière-plan
  et **reprend** au retour (le framework arrête/relance la souscription par diff).

## Vérification

- **Desktop** : compile ; shell 27 ; démo (lib) 36.
- **Appareil** (Huawei STK-L21) : séquence **confirmée** en logcat en tâche de fond puis retour :
  `Resumed → Inactive → Paused` (appui HOME) puis `Resumed` (retour). Le chrono ne tourne plus en
  `Paused`.

## Notes

- `Inactive` s'appuie sur `WindowEvent::Focused` (fiable sur bureau ; sur Android on observe bien
  `Inactive` juste avant `Paused` lors du passage en tâche de fond).
- Pas encore : `Hidden` (état Flutter intermédiaire), ni restauration d'état **après mort du
  processus** (distinct du live-reload `save_state`/`restore_state`, réservé au dev).

## Reste

- Rework du **défilement Kanban** façon Flutter : scroll **horizontal** du board + scroll **vertical
  par colonne** (au lieu du pan 2D `Axis::Both` du jalon 258).
- Balayage overflow des autres écrans ; polish DnD (réagencement même-colonne, inertie verticale,
  ombre `Card`/`Toast`).
