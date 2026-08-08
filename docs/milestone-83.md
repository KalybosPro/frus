# Milestone 83 — One-command start (`cargo generate`): closing §13

## Analysis

§13's last DX link: "`cargo new --template frus-app` works, stay inside cargo as
much as possible". A newcomer has to get a running frus app **with no proprietary
tooling**, using only the standard cargo ecosystem.

## Deliverables

### 1. `crates/frus-hello` — the canonical minimal app
The smallest complete frus app: a **counter** (~60 lines) — state, a pure
`update`, a `view`, and both desktop **and** Android entry points. Being a
workspace member, it **compiles and is tested on every `cargo test
--workspace`**: the reference cannot rot. It is the framework's "Hello, world!"
and the source of the template.

### 2. `templates/app` — the `cargo generate` template
The same counter, parameterised (Liquid):
- `{{project-name}}` / `{{crate_name}}` for naming;
- `{{frus_path}}`: the path to the frus checkout (frus not yet being published on
  crates.io, the dependencies are `path = …`; a comment shows the switch to
  `frus-shell = "0.1"` once published).
- `src/bin/{{project-name}}.rs`: the desktop binary delegates to the lib.
- `cargo-apk` metadata included (Android ready).

Excluded from the workspace (`exclude = ["templates"]`): its `{{…}}` files are
not compilable Rust.

### 3. `docs/getting-started.md`
The complete path: the smallest app, `cargo generate`, `cargo run`,
`cargo apk run`, `cargo test`.

## Usage

```sh
cargo install cargo-generate
cargo generate --path templates/app --name my-app
cd my-app && cargo run
```

## Validation

- `cargo test --workspace`: 21 suites green (frus-hello added; its
  `counting_is_pure` test illustrates the Elm argument — logic tested with no
  GPU).
- **Template rendered → a buildable project**: placeholders substituted
  (`hello-app`), `cargo build` OK and `cargo test` green inside the generated
  project (outside the workspace, with `path` dependencies onto frus).
- `frus-hello` runs like the demo (the same `frus_shell::run` path, an exit
  identical to the WSL smoke run).
- An unused `Dimension` warning (a leftover from the Alert-paragraph milestone)
  cleaned up along the way.

## §13 closed

Headless/golden testing (`frus-test`, J77), the runtime inspector (J78), live
reload (J79), and now a cargo-native start: §13's DX is covered. Outside §13
there remain: §14 (RTL/i18n) and AccessKit (a11y).
