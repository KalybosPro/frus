# Jalon 270 — Effets **asynchrones** (`perform_async` / `run_async`)

## Objectif

Jusqu'ici, une `Command` ne portait que des tâches **synchrones** (`FnOnce() -> Option<Msg>`) : sur le
Web (mono-thread), elles s'exécutaient en microtâche sans pouvoir `await` — donc **pas de vrai
`fetch`**. Ce jalon ajoute une forme **asynchrone** : une `Command` peut porter une **future** qui
s'attend réellement, pilotée par le navigateur sur le Web et menée à terme sur un thread en natif.

## API

- `Command::perform_async(future)` — la valeur de la future devient un message.
- `Command::run_async(future)` — future à effet de bord ; `Option<Msg>` (`None` = aucun message).

```rust
fn update(&mut self, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Load => Command::perform_async(async {
            let body = fetch("/api/data").await;   // await réel (fetch sur le Web)
            Msg::Loaded(body)
        }),
        Msg::Loaded(_) => Command::none(),
    }
}
```

## Exécution par plateforme

- **Web** (`wasm32`) : `wasm_bindgen_futures::spawn_local` pilote la future — le navigateur est le
  réacteur, un `fetch` (`JsFuture`) `await` sans bloquer la boucle. Le message revient par le proxy.
- **Natif** : la future part sur son **propre thread** et est menée à terme par `pollster::block_on`.
  Parfait pour une future **autonome** (calcul, canal, minuterie pilotée). Une **E/S réseau réelle**
  (qui exige un réacteur) s'appuie sur le **runtime async de l'application** — le framework n'impose
  aucun runtime.

### Bornes `Send` par plateforme

Le type de tâche asynchrone est **conditionnel** : `Future + Send + 'static` en natif (elle traverse
un thread), `Future + 'static` sur le Web (les futures du navigateur — `JsFuture` — ne sont **pas**
`Send`, et n'en ont pas besoin en mono-thread). Les deux signatures de `perform_async` / `run_async`
sont donc `#[cfg]`-gardées.

## Implémentation

- **`frus-shell/src/command.rs`** : champ `async_tasks: Vec<AsyncTask<Msg>>` (alias `#[cfg]`-gardé),
  méthodes `perform_async` / `run_async` (deux variantes par plateforme), `batch` / `is_empty` /
  `into_parts` étendus.
- **`frus-shell/src/app.rs`** (`run_command`) : draine `async_tasks` — `thread::spawn` +
  `pollster::block_on` en natif, `spawn_local` sur le Web ; message renvoyé par le proxy, comme les
  tâches synchrones.

## Vérification

- **Compilation** : `frus-shell` compile (tests inclus, `--no-run`).
- **Tests** (natif) : `perform_async_yields_a_message` (`block_on(async { 7 })` → `Some(7)`),
  `run_async_may_produce_nothing`, `batch_combines_sync_and_async_tasks`.
  *(L'exécution locale des binaires de test est bloquée par SAC cette session — os error 4551,
  environnement ; la compilation, elle, passe. Voir la note SAC du projet.)*
- **Web** : chemin `spawn_local` structurellement identique à l'ancien (déjà en place au jalon 130) ;
  vérifiable en navigateur.

## Reste

- Un **helper `fetch` cross-plateforme** (web-sys `fetch` ↔ un client natif) — pour l'instant, l'app
  fournit la future ; le framework ne fait que la piloter.
