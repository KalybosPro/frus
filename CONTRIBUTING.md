# Contributing to frus

Thanks for looking. frus is early — the core is solid but the surface is wide open, so a single well-scoped PR can genuinely shape a subsystem. This document tells you how to build it, what a good change looks like here, and how to get it merged.

> **Language.** The repository is **English** — code, comments, documentation and commit messages. Issues and discussions are welcome in English or French. The one deliberate exception is [`README.fr.md`](README.fr.md), the French translation of the README.

---

## Table of contents

- [Ways to help](#ways-to-help)
- [Setting up](#setting-up)
- [Running things](#running-things)
- [The workflow](#the-workflow)
- [What we expect from a change](#what-we-expect-from-a-change)
- [Testing](#testing)
- [Code style](#code-style)
- [Design principles](#design-principles)
- [Commits and pull requests](#commits-and-pull-requests)
- [Platform notes and known gotchas](#platform-notes-and-known-gotchas)
- [Getting help](#getting-help)

---

## Ways to help

You don't need to know `wgpu` to be useful here.

| If you know… | Good places to start |
|---|---|
| **Rust, no graphics** | Widgets, layout behaviour, `frus-core` types, tests, examples, API ergonomics |
| **Another UI toolkit** | Port a widget or interaction pattern we're missing. Tell us where our API is worse than the one you know — that's a valid issue |
| **GPU / graphics** | `frus-gpu`: batching, the glyph atlas, path tessellation, shader work, the compositor |
| **Mobile** | Android shell hardening; iOS shell is unclaimed and self-contained |
| **Web** | wasm target: clipboard, accessibility, live-reload are all unwired |
| **Accessibility** | AccessKit coverage across widgets; semantics tree correctness |
| **i18n** | RTL edge cases, Fluent bundles, locale negotiation |
| **Docs / writing** | English documentation, `docs/` translation, tutorials, rustdoc on public APIs |

Browse [good first issues](https://github.com/KalybosPro/frus/labels/good%20first%20issue) and [help wanted](https://github.com/KalybosPro/frus/labels/help%20wanted), or read [ROADMAP.md](ROADMAP.md) for the larger open areas.

**Nothing is claimed until someone says so.** Comment on the issue before you start, so two people don't build the same thing.

## Setting up

**Required:**

- Rust **current stable** (`rustup update stable`). No MSRV is pinned yet — establishing one is [on the roadmap](ROADMAP.md).
- A GPU with Vulkan, Metal, or DX12 drivers — frus renders through `wgpu`, there is no software fallback for the demos
- `rustfmt` and `clippy` components: `rustup component add rustfmt clippy`

**Per-platform extras:**

| Target | Needs |
|---|---|
| Linux desktop | `libxkbcommon-dev`, `libwayland-dev` (or X11 headers), plus Vulkan drivers (`mesa-vulkan-drivers`) |
| Android | Android SDK + NDK, `ANDROID_HOME` / `ANDROID_NDK_ROOT`, `cargo install cargo-apk`, `rustup target add aarch64-linux-android` |
| Web | `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli`, a WebGPU browser (Chrome/Edge 113+) |

```sh
git clone https://github.com/KalybosPro/frus
cd frus
cargo test --workspace
```

If that's green, you're set.

## Running things

```sh
cargo run -p frus-hello           # minimal counter — read this first
cargo run -p frus-demo            # broad app: todos, kanban, tables, pickers
cargo run -p frus-transforms      # animation / transform showcase
cargo run -p frus-fetch-example   # async HTTP + RemoteData states
```

Android:

```sh
cargo apk run -p frus-demo
```

Web: see [`crates/frus-hello/web/README.md`](crates/frus-hello/web/README.md).

Logging is `env_logger`-based:

```sh
RUST_LOG=frus_shell=debug,frus_gpu=info cargo run -p frus-demo
```

## The workflow

frus is built in **milestones**. Each one is a single coherent step: a feature, its tests, and a design note in `docs/milestone-N.md` ([index](docs/README.md)). There are 277 of them, and together they are the project's real memory — they record not just what was built but which alternatives were rejected and why.

You do **not** need to write a milestone note for a bug fix, a test, or a small ergonomic improvement. You **do** for anything that adds public API, changes a subsystem's shape, or makes a trade-off a future reader would question.

A milestone note follows the shape the existing ones use:

```markdown
# Milestone N — <title>

## Goal
Why this step, what problem it solves, why it comes now.

## API           (or: The type / Architecture)
The public surface, with code.

## Technical decisions
Alternatives considered, their trade-offs, and why this one won.

## Implementation
What changed, crate by crate.

## Verification
How it was tested. What's green.

## What's left
What this deliberately leaves for later.
```

Pick the next free number (`ls docs/milestone-*.md | sort -V | tail -1`). If your PR is a milestone, say so in the description and the maintainers will confirm the number.

## What we expect from a change

Four things, in rough order of importance:

1. **Tests.** No module lands without them. If it's logic, it's a unit test that runs without a GPU. If it's rendering, it's a golden.
2. **Cross-platform.** A change is not done if it only works on your OS. If it can't be, gate it and say so.
3. **Overridable styling.** Every visual property a widget uses must be settable by the app. Themed defaults are fine; hardcoded-only values are not — this is a hard rule, see [Design principles](#design-principles).
4. **Documentation.** Public items get rustdoc. Non-obvious decisions get a comment explaining the *why*, not the *what*.

Changes that will be sent back: new `unsafe` without justification, a widget with baked-in colors or sizes, a public API added without a doc comment, platform-specific code outside `frus-shell`, and anything that lands untested.

## Testing

```sh
cargo test --workspace                     # everything
cargo test -p frus-widgets                 # one crate
cargo test -p frus-widgets scroll          # one area
cargo test --workspace --features net,json # feature-gated code
```

**Two kinds of tests.**

*Logic tests* need no GPU and no window — this is the payoff of the pure-`update` architecture, and most of the suite is here. Prefer them:

```rust
#[test]
fn counting_is_pure() {
    let mut app = Counter::default();
    app.update(Msg::Increment);
    app.update(Msg::Decrement);
    assert_eq!(app.count, 0);
}
```

*Golden tests* render offscreen through [`frus-test`](crates/frus-test) and compare against a reference PNG in `crates/frus-test/tests/goldens/`:

```rust
snapshot.assert_golden(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/card.png"));
```

- A missing golden is **created** on first run — inspect it by eye, then commit it. Never commit a golden you haven't looked at.
- A failing golden writes `<name>.actual.png` next to the reference so you can diff them. Those files are gitignored.
- Goldens depend on the rasterizer; text antialiasing varies across drivers. `assert_golden_with` takes a channel tolerance and a pixel budget for that reason. If a golden fails only on your machine, say so in the PR rather than loosening it silently.

**Run the goldens yourself when you change what pixels come out.** `cargo test --workspace` includes them, but the headless CI job does not, and the CI job that does is advisory for the goldens, for want of a pinned rasteriser version on the runner. That gap once let a deliberate change to text metrics leave 47 goldens red for five milestones with nobody looking (milestone 294). If your change touches text measurement, layout, painting or the renderer:

```sh
cargo test -p frus-test --test goldens --test widgets
```

Both files are pixel tests: `goldens.rs` holds the screens — tables, charts, forms,
pickers — and `widgets.rs` holds one image per group of widgets, which is what says
whether a change to painting moved a checkbox, an icon or a drawer (milestone 296).

If they change, **look at the `.actual.png` files** before deciding. A shift of a pixel or two on every glyph is a text-metric change and probably intended; a shape that has moved, vanished or changed colour is not. Accept them with `FRUS_UPDATE_GOLDENS=1` only once you have looked, and say in the commit message what moved and why.

## Code style

- `cargo fmt --all` before committing. The tree is rustfmt-clean and CI enforces it (milestone 298).
- `cargo clippy --workspace --all-targets` must be silent — CI runs it with `-D warnings` (milestone 298). Where a lint is wrong for the code, write the `#[allow]` at the site with the reason beside it; don't widen it to a module or a crate.
- **`unsafe` requires a justification comment.** The framework core is safe Rust; the only legitimate uses are FFI at the platform edge.
- Comments explain *why*. The code already says what.
- Public items get a `///` doc comment. Widgets should show a usage snippet.
- Keep the crate boundaries honest: if `frus-widgets` needs to know about the platform, the design is wrong.

## Design principles

These are the standing rules the codebase is built on. A PR that violates one needs a very good argument.

**1. Everything in Rust.** No framework logic in another language, ever. The only non-Rust code allowed is a paper-thin native adapter (Kotlin/Swift/Win32/JS) that exposes a platform API and contains no logic of its own.

**2. Learn from mature toolkits, don't copy them.** Spring physics, the widget/element split, gesture arenas, lifecycle contracts — these are solved problems, and the solutions are worth studying. Port the *shape* of what works. But where a prior design has a weakness, fix it rather than inheriting it. Every departure should be defensible.

**3. Customizable to the last detail.** Every widget's styling and every slot must be overridable by the app. Themed defaults, yes. Hardcoded-only, never. If you find yourself writing a literal color or dimension inside a widget's paint path, it belongs in the theme or in a builder method.

**4. Pure `update`.** State transitions are a pure function of state and message. Side effects go through `Command`; external events come in through `Subscription`. This is what makes the framework testable without a GPU, and it is not negotiable.

**5. Dependencies point downward.** `core` → `layout`/`text`/`gpu` → `widgets` → `shell` → app. Nothing below `shell` may know what platform it's running on.

**6. cargo-native.** No bespoke CLI, no code generation step, no project format. If a feature needs a tool, it should be an existing one.

**7. Small, sharp dependencies.** We lean on best-in-class Rust crates (`taffy`, `cosmic-text`, `wgpu`, `lyon`, `fluent`) rather than reinventing them — and write everything else ourselves. Adding a dependency needs a reason.

**8. Nothing lands untested.** See above.

## Commits and pull requests

**Commit messages** follow Conventional Commits, and milestone commits name their milestone:

```
feat: Milestone 277 — <what the milestone delivers>
fix: Scroll no longer clips its last child when height is Auto
test: golden coverage for DataTable sort indicators
docs: translate milestone-101 to English
refactor: extract glyph atlas eviction into its own module
perf: batch path fills sharing a paint
```

Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `chore`.

**Pull requests:**

1. Fork, then branch from `master`: `git checkout -b feat/my-thing`.
2. Make the change. Add tests. Update or add docs.
3. Verify locally:
   ```sh
   cargo test --workspace
   cargo clippy --workspace --all-targets
   cargo fmt --all -- --check   # on your files
   ```
4. Open the PR against `master` and fill in the template. Say what you changed, why, how you tested it, and which platforms you actually ran it on.
5. Screenshots or a short clip for anything visual. Before/after if you changed existing rendering.

Keep PRs focused. One subsystem, one concern. A large PR that mixes a refactor with a feature will be asked to split.

Reviews aim to be substantive rather than fast — expect questions about trade-offs, not nitpicks about formatting.

## Platform notes and known gotchas

Things that have cost previous contributors real time:

- **Scroll sizing.** `Scroll::new()` defaults to 200px with `Auto` sizing. Unsized inside a flexible parent it collapses and clips its content — the symptom looks like a blank page. Give it an explicit width/height or make it flex.
- **RTL text.** If a text buffer is wider than the text itself, cosmic-text right-aligns into the empty space and the glyphs land off-screen. Size the buffer to the text.
- **Interactive widgets and hit-testing.** `Ui::widget_rect` resolves rects through per-kind registries. A new interactive widget that isn't wired into a registry will pass its unit tests and still silently no-op on live drag or hover paths.
- **Android fonts.** Fonts are bundled, not taken from the system — an Android-only text bug is often a font-loading problem, not a shaping one.
- **Windows + Smart App Control.** With SAC in Enforce mode, freshly built test executables are intermittently blocked (`os error 4551`). Compilation succeeds, running fails. Retry, run under WSL2 (see [`scripts/wsl-run.sh`](scripts/wsl-run.sh)), or turn SAC off.
- **WSL2 display.** Force the X11 backend (`WINIT_UNIX_BACKEND=x11`); WSLg's Wayland connection is unstable under root.

Found a new one? Add it here in your PR.

## How decisions get made

frus is small and maintainer-led today. That means:

- **Maintainers merge.** Every PR needs an approving review from a maintainer. Currently that is [@KalybosPro](https://github.com/KalybosPro).
- **Architecture changes go through discussion first.** For anything marked 🔴 in [ROADMAP.md](ROADMAP.md), or anything that changes a crate boundary or a public trait, open an issue or discussion before writing code. You'll get a faster answer and avoid rewriting.
- **Disagreements are settled by argument, not seniority.** The [design principles](#design-principles) are the shared ground. If you think one of them is wrong for a particular case, say so — several existing milestones exist because someone pushed back.
- **"Framework X does it this way" starts a discussion, it doesn't end one.** We port what works and fix what doesn't.
- **Sustained, high-quality contribution in an area earns review rights in it.** There's no formal process yet; if the project grows to need one, it will be written down here.

Issue labels you'll see: `good first issue`, `help wanted`, `needs triage`, `needs design`, `bug`, `enhancement`, plus area labels (`area: widgets`, `area: gpu`, `area: shell`, `area: web`, `area: android`, `area: docs`).

## Getting help

- **Questions and ideas** → [Discussions](https://github.com/KalybosPro/frus/discussions)
- **Bugs and feature requests** → [Issues](https://github.com/KalybosPro/frus/issues)
- **Security** → do not open a public issue, see [SECURITY.md](SECURITY.md)

Before asking, `docs/milestone-*.md` very often already answers "why is it like this?" — `grep` them.

By contributing you agree that your work is dual-licensed under MIT and Apache-2.0, and to abide by the [Code of Conduct](CODE_OF_CONDUCT.md).
