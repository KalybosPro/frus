# Jalon 129 — Cible Web (wasm + WebGPU)

## Analyse

frus visait bureau et Android via winit + wgpu — les deux backends natifs. Manquait la
**plateforme la plus universelle** : le navigateur. winit et wgpu supportent tous deux
`wasm32-unknown-unknown` + **WebGPU** ; l'app (`view`/`update`) est déjà pure et
indépendante de la plateforme. Le travail est **entièrement dans la couche shell**.

Objectif de ce jalon : que toute la pile (**core → widgets → gpu → shell → app**)
**compile** pour `wasm32-unknown-unknown` et qu'une app pilotée par l'entrée
(`frus-hello`, le compteur) tourne dans le navigateur, sans régression sur natif.

## Décisions techniques

- **Trois plateformes, pas deux.** Le shell distinguait bureau (`not(android)`) et
  Android. Le Web devient une 3ᵉ cible : les sous-systèmes **bureau-seuls** (presse-
  papier `arboard`, accessibilité AccessKit, logger `env_logger`, live-reload) passent
  de `not(android)` à `not(any(android, wasm32))` ; leurs no-op couvrent Android **et**
  Web.

- **Horloge portable.** `std::time::Instant` **panique** sur `wasm32-unknown-unknown`.
  On bascule tout le shell sur `web-time::Instant` (alias `std` en natif,
  `performance.now()` sur le Web) — une seule API, zéro `cfg` dans le corps.

- **Init GPU asynchrone.** Le Web ne peut pas **bloquer** (`pollster::block_on`). Sur
  wasm, `resumed` lance `Renderer::new` (déjà `async`) via `wasm_bindgen_futures::
  spawn_local` ; le renderer prêt est déposé dans un `Rc<RefCell<Option<…>>>` et
  récupéré à la première frame. Natif garde `block_on`.

- **Canvas géré par winit.** `WindowAttributesExtWebSys::with_append(true)` : winit crée
  et ajoute un `<canvas>` au `<body>`. Aucune plomberie DOM manuelle.

- **Point d'entrée navigateur.** `frus_shell::run_web` confie la boucle au navigateur
  (`EventLoopExtWebSys::spawn_app`, pilotée par `requestAnimationFrame`). L'app expose
  `#[wasm_bindgen(start)] fn start()` — aucune autre différence.

## Implémentation

- `frus-shell` : `Cargo.toml` (deps Web : `wasm-bindgen(-futures)`, `web-sys`,
  `console_error_panic_hook`, `console_log` ; `web-time` partout ; `pollster` restreint
  au natif ; bureau-seuls exclus du wasm) ; `run_web` ; `resumed`/`RedrawRequested`
  asynchrones ; horloge `web-time` dans `app`/`gesture`/`subscription`/`reload`.
- `frus-hello` : entrée `#[wasm_bindgen(start)]`, `run_desktop` restreint au natif,
  binaire no-op hors natif ; dossier `web/` (`index.html`, `README.md` de build,
  `.gitignore`).

## Vérification

- **Compile** pour `wasm32-unknown-unknown` (debug **et** release) : toute la pile, y
  compris WebGPU. `frus_hello.wasm` ≈ 7,5 Mo brut (avant `wasm-bindgen` + `wasm-opt` +
  gzip).
- **Natif intact** : `cargo test --workspace` reste **vert** (aucune régression des
  ~330 tests), le bureau se construit comme avant.
- Build navigateur : `wasm-bindgen --target web` → `web/pkg/`, servi en `localhost`
  (voir `crates/frus-hello/web/README.md`).

## Reste

- **Vérification en navigateur réel** (l'étape *voir* : je ne peux pas lancer de
  navigateur ici) — Chrome/Edge 113+ sur `localhost`.
- **Effets & souscriptions** au Web : les threads natifs (`std::thread::spawn`) ne
  s'exécutent pas sur wasm → une app à souscription (animation `every`) ne tickerait pas
  encore. À porter via `spawn_local` + timers navigateur.
- **Presse-papier / IME / accessibilité** Web (APIs navigateur) — chantiers distincts.
- Amincir le `.wasm` (`wasm-opt -Oz`, `panic=abort`, split des features wgpu).
