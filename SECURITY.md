# Security Policy

## Supported versions

frus is **pre-alpha** and nothing is published to crates.io yet. Only the `master` branch is supported. There are no maintained release branches and no backports.

Once versioned releases exist, this table will be filled in.

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Report it privately through GitHub's [private vulnerability reporting](https://github.com/KalybosPro/frus/security/advisories/new) on this repository. If that is unavailable to you, email **mitoaboudou@gmail.com** with `[frus security]` in the subject line.

Please include:

- what the issue is, and which crate and file it affects;
- how to reproduce it — a minimal example is ideal;
- what an attacker could actually do with it;
- the platform, GPU/driver, and toolchain version if relevant.

**What to expect:** an acknowledgement within a few days, an assessment of impact, and a fix on `master` with credit to you unless you'd rather stay anonymous. Because there are no releases yet, a fix means a commit on `master` — not a patch release.

This is a small project maintained in people's own time. There is no bug bounty.

## Scope

Things that are in scope:

- Memory-safety issues — any `unsafe` block that can be made to misbehave, and any soundness hole in a safe API.
- Crashes or undefined behaviour reachable from untrusted input: decoding a malicious PNG/JPEG (`frus-image`), shaping hostile text (`frus-text`), parsing a malformed HTTP or JSON response (`frus-shell` `net`/`json` features).
- Anything in the `net` feature that could leak credentials, ignore TLS validation, or be induced to send a request somewhere unintended.
- Resource exhaustion that a remote input can trigger — unbounded allocation from a hostile response or asset.
- Vulnerabilities in the platform adapters (Android IME/activity, wasm bindings) reachable from another app or a web page.

Things that are **not** in scope:

- Bugs in upstream dependencies — report those to `wgpu`, `winit`, `taffy`, `cosmic-text`, `ureq`, `image`, etc. directly. Tell us too if frus's usage makes it worse.
- GPU driver bugs and vendor-specific rendering artifacts.
- Anything that requires an attacker to already control the application's own source code or build environment.
- Denial of service caused by an application's own code — e.g. an unbounded list in `view`.

## For contributors

Two rules keep this surface small, and they're part of the [contribution guidelines](CONTRIBUTING.md):

- **`unsafe` requires a written justification.** The framework core is safe Rust; the only legitimate uses are FFI at the platform edge. A PR adding `unsafe` without a comment explaining why it is sound will be sent back.
- **Untrusted input must be bounded.** Anything decoding an image, parsing a response, or shaping text from the network is parsing attacker-controlled data. Fuzz targets for these paths are a very welcome contribution.
