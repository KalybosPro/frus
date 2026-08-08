#!/usr/bin/env bash
#
# Run frus-demo inside WSL2 (displayed through WSLg).
#
# Context: on this machine, Smart App Control blocks Cargo build scripts from
# running on the Windows side. Development therefore happens in WSL2 (Ubuntu),
# where ELF binaries are not subject to that policy.
#
# winit's X11 backend is forced: under WSLg, the Wayland connection as root is
# unstable (Broken pipe), whereas X11 (DISPLAY=:0) works reliably. On a real
# Linux box, drop these two lines.
#
# Usage:  bash scripts/wsl-run.sh
set -eu

source "$HOME/.cargo/env" 2>/dev/null || true

# Build target directory on the Linux FS (far faster than /mnt/*).
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/frus-target}"

# WSLg display workaround.
export WINIT_UNIX_BACKEND=x11
unset WAYLAND_DISPLAY 2>/dev/null || true

exec cargo run -p frus-demo "$@"
