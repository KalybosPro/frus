# Jalon 130 — Effets & souscriptions au Web

## Analyse

Le jalon 129 a fait **compiler** toute la pile pour `wasm32-unknown-unknown` + WebGPU,
mais la couche framework restait bornée aux plateformes natives sur un point : les
**effets** (`Command`) et les **souscriptions** (`Subscription::every`) s'exécutaient
sur `std::thread::spawn`. Or wasm32 est **mono-thread** — un `thread::spawn` y panique.
Conséquence : sur le Web, une app purement pilotée par l'entrée (le compteur à boutons)
tournait, mais toute app à effet ou à animation par souscription se serait effondrée au
premier `update` renvoyant une `Command`, ou à la première souscription active.

Objectif : porter les deux mécanismes au Web **sans toucher l'API applicative** —
`Command::perform`/`run` et `Subscription::every` restent identiques ; seule leur
exécution diffère par plateforme.

## Décisions techniques

- **Effets → `spawn_local`.** Une `Command` est une liste de tâches synchrones
  (`FnOnce() -> Option<Msg>`). Sur le Web, chaque tâche est planifiée sur la boucle via
  `wasm_bindgen_futures::spawn_local` (une microtask) au lieu d'un thread ; son message
  revient par le même `EventLoopProxy` que sur natif. Le travail reste **synchrone** (le
  type `Task` ne peut pas `await`) — suffisant pour un effet de calcul ; un effet
  réellement asynchrone (fetch réseau) demandera plus tard une variante `Command` dédiée.

- **Souscriptions → `setInterval`.** La souscription `every` tournait dans un thread
  bouclant sur `recv_timeout`. Sur le Web, elle devient un **`setInterval` navigateur**
  (`web-sys`) : à chaque tick, la callback émet le message via le proxy. L'**annulation**
  — pierre angulaire du diff des souscriptions — est préservée par une poignée
  `web_timer::Interval` dont le **drop** appelle `clearInterval` (et libère la closure
  retenue). Symétrie exacte avec le natif, où le drop du `Sender` fait sortir le thread.

- **`SubHandle`, une poignée par plateforme.** `running_subs` mappe désormais chaque id
  vers un `SubHandle` : `Sender<()>` en natif, `Option<web_timer::Interval>` sur le Web.
  Le diff (`retain` + `insert`) et l'arrêt-par-drop restent identiques ; toute la logique
  de `sync_subscriptions` est inchangée.

- **`web-time` pour l'horloge du tick.** Le `Instant` passé à la fabrique de message
  (`make(Instant::now())`) vient de `web-time` — déjà en place depuis J129 — donc valide
  sur les trois plateformes.

- **Vitrine : mode auto dans `frus-hello`.** Le compteur gagne un bouton **Start/Stop
  auto** : en mode auto, `subscription()` renvoie `every(1s, |_| Tick)` ; sinon
  `none()`. C'est le plus petit exemple qui rend une souscription **visible** en
  navigateur — et il se teste sans GPU (le diff de souscription est pur).

## Implémentation

- `frus-shell/src/app.rs` : module `web_timer` (Web) — `Interval` retenu, `clearInterval`
  au drop ; type `SubHandle` par plateforme ; `run_command` et `start_subscription`
  dédoublés natif/Web (`spawn_local` / `setInterval`) ; import `Sender`/`RecvTimeoutError`
  restreint au natif.
- `frus-shell/src/reload.rs` : `restore_from_env` (et l'import `Path`) restreints à leur
  seul appelant, le `run` de bureau — supprime le code mort sur Android/Web.
- `frus-hello/src/lib.rs` : état `auto`, messages `ToggleAuto`/`Tick`, `subscription()`
  selon l'état, bouton de bascule, test `auto_mode_drives_the_subscription`.

## Vérification

- **Compile** pour `wasm32-unknown-unknown` (effets + souscriptions inclus), **sans
  warning**.
- **Natif intact** : `cargo test --workspace` reste **vert** (aucune régression), y
  compris le nouveau test du mode auto.
- Souscription **testée purement** : absente au repos, présente une fois `auto` activé,
  un `Tick` incrémente, disparue à la coupure.

## Reste

- **Vérification en navigateur réel** (l'étape *voir*) : Start auto → le compteur
  s'incrémente une fois par seconde, Stop → il s'arrête. Je ne peux pas lancer de
  navigateur ici.
- **Effet réellement asynchrone au Web** (fetch réseau) : demandera une variante
  `Command` capable d'`await` (le `Task` actuel est synchrone).
- Presse-papier / IME / accessibilité Web restent des chantiers distincts.
