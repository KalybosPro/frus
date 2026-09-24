#!/usr/bin/env bash
# Proves the crates can be published, without publishing anything (milestone 565, #13).
#
# `cargo publish --workspace --dry-run` packages every crate that is not `publish = false`
# in dependency order, builds each **from its packaged tarball** against the ones packaged
# before it, and stops just before the upload. That is the whole proof: a missing `version`
# on a path dependency, a file left out of a tarball, or a cycle all fail here and not on the
# day of the real thing.
#
# It compiles the graph three times over and takes the better part of an hour from a cold
# cache, which is why CI does not run it; run it before a release, from a clean checkout.
#
# Publishing for real is deliberately not in this script. A name on crates.io cannot be
# taken back, so the first `cargo publish` is the maintainer's to type.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo publish --workspace --dry-run "$@"
