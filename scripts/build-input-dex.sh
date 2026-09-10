#!/usr/bin/env bash
#
# Compile the Android bridge classes (FrusTextBridge.java, the input bridge;
# FrusSystemBars.java, the system bars of #46; FrusClipboard.java, the clipboard of
# #22) into one embedded dex.
# Only re-run this when a Java file changes; the resulting dex is checked in
# (crates/frus-shell/assets/frus_input.dex) so that `cargo apk` never needs javac.
#
# Usage (inside WSL):  bash scripts/build-input-dex.sh
set -eu

SDK="${ANDROID_HOME:-/root/android-sdk}"
BUILD_TOOLS="$SDK/build-tools/34.0.0"
PLATFORM="$SDK/platforms/android-34/android.jar"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JAVA_DIR="$ROOT/crates/frus-shell/java/dev/frus/input"
OUT_DIR="$(mktemp -d)"
trap 'rm -rf "$OUT_DIR"' EXIT

javac --release 8 -cp "$PLATFORM" -d "$OUT_DIR/classes" "$JAVA_DIR"/*.java
mkdir -p "$OUT_DIR/dex"
"$BUILD_TOOLS/d8" --release --output "$OUT_DIR/dex" \
    "$OUT_DIR"/classes/dev/frus/input/*.class

mkdir -p "$ROOT/crates/frus-shell/assets"
cp "$OUT_DIR/dex/classes.dex" "$ROOT/crates/frus-shell/assets/frus_input.dex"
echo "OK: $(stat -c%s "$ROOT/crates/frus-shell/assets/frus_input.dex") bytes"
