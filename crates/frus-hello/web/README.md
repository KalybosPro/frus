# frus-hello — cible Web (wasm + WebGPU)

`frus-hello` tourne dans le navigateur via **WebGPU**, sans changer une ligne de l'app :
seul le point d'entrée diffère (`#[wasm_bindgen(start)] fn start()` appelle
`frus_shell::run_web`).

## Prérequis (une fois)

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli   # version alignée sur wasm-bindgen 0.2
```

Un navigateur avec **WebGPU** : Chrome/Edge 113+, ou Firefox avec WebGPU activé. WebGPU
exige un **contexte sécurisé** (`https` ou `localhost`).

## Construire

```bash
# 1) Compiler la lib en wasm (release conseillé pour la taille/perf).
cargo build -p frus-hello --target wasm32-unknown-unknown --release

# 2) Générer la glu JS + le .wasm bindé dans web/pkg/.
wasm-bindgen \
  --target web --no-typescript \
  --out-dir crates/frus-hello/web/pkg \
  target/wasm32-unknown-unknown/release/frus_hello.wasm
```

## Servir

Un serveur statique suffit (les modules ES et le `.wasm` doivent être servis en HTTP,
pas en `file://`) :

```bash
cd crates/frus-hello/web && python3 -m http.server 8080
# → http://localhost:8080
```

## Notes

- **Rendu / entrée / animation** partagent le même code que bureau/Android ; winit
  ajoute un `<canvas>` au `<body>`, l'horloge passe par `web-time` (`performance.now()`),
  et l'init GPU est **asynchrone** (voir `run_web` / `resumed`).
- **Effets & souscriptions** sont portés au Web (jalon 130) : les `Command` passent par
  `spawn_local`, les souscriptions `every` par un `setInterval` navigateur (annulé au
  drop). Le bouton **Start auto** du compteur déclenche une souscription `every(1s)` qui
  l'incrémente — l'exemple à observer en navigateur. Un effet réellement **asynchrone**
  (fetch réseau) reste à venir (le `Task` actuel est synchrone).
- **Presse-papier / accessibilité / live-reload** sont désactivés sur le Web (chantiers
  distincts).
