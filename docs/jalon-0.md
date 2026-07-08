# Jalon 0 — Une fenêtre + un quad coloré

Premier jalon de **frus**. Il établit la boucle fondamentale de tout framework
UI : `événement système → mise à jour → frame GPU → présentation`.

## Ce qui est livré

- Workspace Cargo (`crates/*`).
- `frus-gpu` : contexte GPU + moteur de rendu minimal (wgpu), **sans dépendance
  au fenêtrage**. Dessine un quad coloré via un pipeline WGSL.
- `frus-shell` : couche plateforme (winit 0.30, `ApplicationHandler`). Crée la
  fenêtre et pilote le renderer.
- `frus-demo` : binaire de démonstration.

## Architecture

```
        winit (event loop)                 wgpu (GPU)
      ┌───────────────────┐          ┌──────────────────────┐
OS ─► │  frus-shell        │ ─frame─► │  frus-gpu (Renderer) │ ─► écran
event │  ApplicationHandler│ ─resize► │  pipeline + quad     │
      └───────────────────┘          └──────────────────────┘
                    └──── frus-demo (bin: cargo run) ────┘
```

Le seam clé : `frus-gpu::Renderer::new` reçoit une `wgpu::SurfaceTarget` et non
un type winit. La plateforme est ainsi remplaçable (Web/mobile plus tard) sans
toucher au moteur de rendu.

## Environnement de dev : WSL2

**Important** : sur la machine de dev Windows, **Smart App Control** est activé
et bloque l'exécution des *build scripts* que Cargo compile (erreur système
4551). SAC ne se désactive qu'une fois et de façon irréversible ; on a donc
choisi de développer dans **WSL2 (Ubuntu 24.04)**, où les binaires Linux ne sont
pas soumis à cette politique. L'affichage passe par **WSLg**.

Setup (déjà réalisé) :

```sh
wsl --install -d Ubuntu-24.04
# dans Ubuntu :
sudo apt-get install -y build-essential curl pkg-config \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 wayland-protocols \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libvulkan1 mesa-vulkan-drivers vulkan-tools libgl1-mesa-dri libegl1
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

## Lancer

Depuis WSL2, à la racine du projet :

```sh
bash scripts/wsl-run.sh
```

Le script force le backend **X11** de winit (sous WSLg, Wayland en root est
instable ; X11 via `DISPLAY=:0` est fiable) et met le répertoire de build sur
le FS Linux.

Résultat attendu : une fenêtre « frus — Jalon 0 » avec un carré au dégradé
(rouge/vert/bleu/jaune) sur fond bleu nuit.

> Note GPU : sous WSLg sans passthrough matériel, wgpu tombe sur le backend
> Vulkan logiciel `llvmpipe` (Mesa). Le rendu est correct, simplement non
> accéléré. Sur une machine Linux native, un vrai GPU sera utilisé.

## Tests

```sh
cargo test
```

`frus-gpu` inclut un test qui énumère les adaptateurs GPU sans planter
(exécutable en CI sans écran).

## Notes de version

`wgpu` évolue vite : les versions sont épinglées dans `Cargo.toml`
(`wgpu = "22"`, `winit = "0.30"`). Le premier `cargo build` (une fois Rust
installé) validera l'API et ajustera les éventuels détails mineurs.
