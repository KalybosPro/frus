# Jalon 24 — `Command` / effets depuis `update`

Le modèle Elm de frus gagne son **canal d'effets** : `update` peut désormais
déclencher un travail hors du cycle (I/O, tâche de fond) dont le résultat revient
sous forme de message. C'est ce qui manquait pour des apps réelles.

## API

```rust
fn update(&mut self, msg: Msg) -> Command<Msg>;   // renvoie les effets
fn init(&mut self) -> Command<Msg> { Command::none() }  // effet de démarrage

Command::none()                          // aucun effet
Command::batch([a, b, c])                // plusieurs
Command::perform(|| calcul() -> Msg)     // tâche → message réinjecté
Command::run(|| { effet(); None })       // effet de bord, message optionnel
```

`Application::Message` est désormais `Clone + Send + 'static` (les effets
traversent des threads).

## Exécution (framework)

- `run` ouvre la boucle avec **événements utilisateur** :
  `EventLoop::<Message>::with_user_event()`, et garde un `EventLoopProxy<Message>`.
- `update` renvoie une `Command` ; le pilote **spawn un thread par tâche** ; au
  retour, `proxy.send_event(msg)` **réveille la boucle** → `user_event(msg)` →
  `dispatch(msg)` (qui réapplique `update`, pouvant produire d'autres effets).
- `init()` s'exécute une fois au démarrage (dans `resumed`).

Tous les points d'entrée (clic, clavier, drag, événement utilisateur) passent par
un `dispatch` central qui exécute la `Command` renvoyée.

## Décisions techniques (alternatives)

- **Threads bruts vs runtime async (tokio/smol)** → **threads** : zéro dépendance
  lourde, suffisant pour l'I/O et la latence d'UI. Un exécuteur async reste une
  évolution possible (les tâches sont déjà des `FnOnce() -> Option<Msg> + Send`).
- **Retour des résultats** via `EventLoopProxy` (mécanisme natif winit pour
  réveiller la boucle depuis un autre thread) plutôt qu'un canal mpsc maison.

## Démo — persistance des tâches

- **Sauver** → `Command::run` écrit les tâches dans un fichier temporaire
  (`done<TAB>texte`, **sans serde**).
- **Charger** / **démarrage** → `Command::perform` lit le fichier →
  `Msg::Loaded(Vec<(bool, String)>)` remplace les tâches (ids réattribués).
- Boutons « Charger » / « Sauver » dans le pied.

## Tests

- `Command` : `none`/`perform`/`run`/`batch` (structure + drain des tâches).
- Persistance : round-trip `save → load` (fichier temporaire déterministe),
  `Msg::Loaded` remplace les tâches avec ids uniques, `Save` produit bien un effet
  (`!is_empty`) là où une mutation simple n'en produit pas.
- Total : 35 frus-widgets + 4 frus-shell (Command) + 7 frus-demo + doctest.

## Limites (v1)

- Threads bruts : pas d'annulation ni de pool ; pas d'`async`/`await`.
- Persistance format texte maison (pas de migration/robustesse), fichier unique.
- Un effet immédiat fait tout de même un aller-retour par la boucle (léger délai).
