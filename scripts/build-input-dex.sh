#!/usr/bin/env bash
#
# Compile le pont de saisie Android (FrusTextBridge.java) en dex embarqué.
# À relancer seulement quand le fichier Java change ; le dex produit est
# versionné (crates/frus-shell/assets/frus_input.dex) pour que `cargo apk`
# n'ait jamais besoin de javac.
#
# Usage (dans WSL) :  bash scripts/build-input-dex.sh
set -eu

SDK="${ANDROID_HOME:-/root/android-sdk}"
BUILD_TOOLS="$SDK/build-tools/34.0.0"
PLATFORM="$SDK/platforms/android-34/android.jar"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JAVA_SRC="$ROOT/crates/frus-shell/java/dev/frus/input/FrusTextBridge.java"
OUT_DIR="$(mktemp -d)"
trap 'rm -rf "$OUT_DIR"' EXIT

javac --release 8 -cp "$PLATFORM" -d "$OUT_DIR/classes" "$JAVA_SRC"
mkdir -p "$OUT_DIR/dex"
"$BUILD_TOOLS/d8" --release --output "$OUT_DIR/dex" \
    "$OUT_DIR"/classes/dev/frus/input/*.class

mkdir -p "$ROOT/crates/frus-shell/assets"
cp "$OUT_DIR/dex/classes.dex" "$ROOT/crates/frus-shell/assets/frus_input.dex"
echo "OK : $(stat -c%s "$ROOT/crates/frus-shell/assets/frus_input.dex") octets"
