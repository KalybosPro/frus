# Changelog

All notable changes to frus are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project will follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it is published.

> **Nothing has been released yet.** frus is pre-alpha and not on crates.io; the workspace
> is versioned `0.0.0`. Until a first release, the authoritative record of what changed and
> *why* is the milestone notes in [`docs/jalon-*.md`](docs/) — one per step, 276 so far,
> each documenting the objective, the alternatives weighed, and the decision.

## [Unreleased]

### Added

- **Typed JSON over HTTP** (J275) — `Request::send_json::<T>()` and `Request::json_body(&value)`
  behind the `json` feature.
- **`RemoteData<T, E>`** (J274) — the Elm idiom for asynchronous data, replacing hand-rolled
  `Idle`/`Loading`/`Loaded`/`Failed` state machines in application code.
- **`frus-fetch-example`** (J273) — an end-to-end network example with loading, data, and
  error states.
- **`Request` builder** (J272) — POST bodies, headers, and timeouts on top of `fetch`.
- **Cross-platform `fetch`** (J271) — one signature for desktop, Android, and the web,
  behind the `net` feature (`ureq` natively, browser `fetch` on wasm).
- **Asynchronous effects** (J270) — `Command::perform_async` / `run_async`, making real
  `await` possible on the single-threaded web target.
- **`frus` facade crate** (J268) — an application declares exactly one dependency.
- **Single entry point** (J267) — `frus::main!(App::default())` generates the desktop,
  Android, and wasm entry points from one declaration.

### Changed

- **`compute_scroll` fills its constrained axis** (J269) — `Scroll` now behaves like a
  `ListView`; applications no longer need a filler container around it.

### Project

- Added `README.md`, `README.fr.md`, `CONTRIBUTING.md`, `ARCHITECTURE.md`, `ROADMAP.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, dual MIT/Apache-2.0 license files, issue and PR
  templates, and CI.

<!--
For releases, use this shape:

## [0.1.0] - YYYY-MM-DD

### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->

[Unreleased]: https://github.com/KalybosPro/frus/commits/master
