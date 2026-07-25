# Jalon 131 — Amincir le `.wasm`

## Analyse

Le bundle Web du jalon 129 pesait ~7,9 Mo de `.wasm` **brut** en `--release` par défaut
— soit, une fois passé par `wasm-bindgen` et servi en gzip, **~2,86 Mo téléchargés**.
C'est le premier octet que voit un visiteur : la taille de transfert est la métrique qui
compte (pas le brut sur disque). Le profil release par défaut optimise la **vitesse**
(`opt-level = 3`), pas la taille, et conserve les tables de déroulement des panics —
inutiles pour une cible où l'on veut d'abord un petit téléchargement.

Objectif : réduire le `.wasm` **téléchargé** sans dégrader le rendu ni toucher au
release **natif** (qui, lui, doit rester réglé pour la vitesse).

## Décisions techniques

- **Un profil `web-release` dédié.** Cargo ne permet pas de régler un profil *par cible*,
  mais permet des **profils nommés**. On ajoute `[profile.web-release]` (héritant de
  `release`), activé explicitement par `--profile web-release`. Le release natif n'est
  jamais affecté.
  - `opt-level = "z"` — priorité à la **taille** du code (vs `3` = vitesse).
  - `lto = true` — inlining inter-crates + élagage agressif du code mort.
  - `codegen-units = 1` — une seule unité de génération : meilleure optimisation.
  - `panic = "abort"` — supprime les **tables de déroulement** des panics (unwinding),
    du poids mort sur le Web (un panic y va de toute façon dans la console).
  - `strip = true` — retire symboles et debuginfo.

- **`wasm-opt` : mesuré, pas supposé.** La passe `wasm-opt -Oz` (binaryen) rétrécit le
  `.wasm` **brut**, mais avec le binaryen ancien disponible ici (v108) elle **gonfle
  légèrement le gzip** (réordonnancement qui compresse moins bien). Comme c'est la taille
  gzip qui est téléchargée, on ne l'inscrit pas en dur dans le flux de build : le README
  la documente comme passe **optionnelle**, à n'adopter qu'avec un binaryen récent et
  après avoir mesuré le gzip.

## Implémentation

- `Cargo.toml` (workspace) : profil `[profile.web-release]` (`inherits = "release"`).
- `crates/frus-hello/web/README.md` : build via `--profile web-release` (et chemin
  `target/wasm32-unknown-unknown/web-release/…`), tableau des tailles, mise en garde
  `wasm-opt`, rappel de servir le `.wasm` compressé.

## Vérification

Mesures via `wasm-bindgen --target web` puis `gzip -9` (taille réellement téléchargée) :

| build                     | après `wasm-bindgen` | gzip (transfert) |
| ------------------------- | -------------------: | ---------------: |
| `--release` (défaut)      |          6 662 015 o |      2 864 956 o |
| `--profile web-release`   |          5 644 007 o |  **2 556 282 o** |

- **Transfert : −308 674 o ≈ −10,8 %** ; brut post-bindgen : −15,3 %.
- **Natif intact** : le profil est additif ; `cargo test --workspace` reste vert, le
  release natif garde `opt-level = 3`.

## Reste

- L'essentiel du poids restant est **`wgpu` + `naga`** (le pilote WebGPU) — incompressible
  sans perdre le rendu.
- Leviers plus lourds, non retenus ici : `-Z build-std` + `panic_immediate_abort`
  (recompile la `std` optimisée taille, mais exige nightly) ; `wasm-opt -Oz` avec un
  binaryen récent (gain supplémentaire à vérifier au gzip).
