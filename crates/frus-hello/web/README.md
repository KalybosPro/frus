# frus-hello — cible Web (wasm + WebGPU)

`frus-hello` tourne dans le navigateur via **WebGPU**, sans changer une ligne de l'app :
le **point d'entrée unique** `frus_shell::main!(Counter::default())` engendre lui-même
l'entrée Web `#[wasm_bindgen(start)]` (qui appelle `frus_shell::run_web`), comme il engendre
les entrées bureau et Android — l'app n'écrit **rien** de spécifique à la plateforme.

## Prérequis (une fois)

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli   # version alignée sur wasm-bindgen 0.2
```

Un navigateur avec **WebGPU** : Chrome/Edge 113+, ou Firefox avec WebGPU activé. WebGPU
exige un **contexte sécurisé** (`https` ou `localhost`).

## Construire

```bash
# 1) Compiler la lib en wasm avec le profil Web (optimisé TAILLE — jalon 131 :
#    opt-level=z, lto, codegen-units=1, panic=abort, strip).
cargo build -p frus-hello --target wasm32-unknown-unknown --profile web-release

# 2) Générer la glu JS + le .wasm bindé dans web/pkg/.
wasm-bindgen \
  --target web --no-typescript \
  --out-dir crates/frus-hello/web/pkg \
  target/wasm32-unknown-unknown/web-release/frus_hello.wasm
```

### Taille (jalon 131)

Le profil `web-release` réduit le `.wasm` **téléchargé** (gzip) d'environ **11 %**
par rapport au `--release` par défaut :

| build                     | après `wasm-bindgen` | gzip (transfert) |
| ------------------------- | -------------------: | ---------------: |
| `--release` (défaut)      |            ~6,65 Mio |        ~2,86 Mio |
| `--profile web-release`   |            ~5,64 Mio |    **~2,56 Mio** |

L'essentiel du poids restant est `wgpu` + `naga` (le pilote WebGPU), incompressible
sans perdre le rendu. Servez toujours le `.wasm` **compressé** (`Content-Encoding:
gzip`/`br`) — c'est la taille gzip qui est téléchargée.

> Passe optionnelle `wasm-opt -Oz` (binaryen) : elle rétrécit le `.wasm` brut, mais
> **n'aide le gzip qu'avec un binaryen récent** — une version ancienne peut réordonner
> le code d'une façon qui compresse *moins* bien. Mesurez le gzip avant de l'adopter.

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
