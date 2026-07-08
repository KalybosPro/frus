#!/usr/bin/env bash
#
# Lance frus-demo dans WSL2 (affichage via WSLg).
#
# Contexte : sur cette machine, Smart App Control bloque l'exécution des
# build scripts Cargo côté Windows. Le développement se fait donc dans WSL2
# (Ubuntu), où les binaires ELF ne sont pas soumis à cette politique.
#
# On force le backend X11 de winit : sous WSLg, la connexion Wayland en root
# est instable (Broken pipe), alors que X11 (DISPLAY=:0) fonctionne de façon
# fiable. Sur un vrai Linux, retirez ces deux lignes.
#
# Usage :  bash scripts/wsl-run.sh
set -eu

source "$HOME/.cargo/env" 2>/dev/null || true

# Cible de build sur le FS Linux (bien plus rapide que /mnt/*).
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/frus-target}"

# Workaround affichage WSLg.
export WINIT_UNIX_BACKEND=x11
unset WAYLAND_DISPLAY 2>/dev/null || true

exec cargo run -p frus-demo "$@"
