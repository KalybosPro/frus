# CLAUDE.md — frus

Greenfield Rust cross-platform UI framework (Flutter-like). Elm architecture.

## Stack
Rust workspace (edition 2021). wgpu 22 · winit 0.30 · taffy 0.7 (flex/grid) · glyphon+cosmic-text. `Application` trait: `update(msg) -> Command`, `view -> Box<dyn Widget>`, `subscription`.

## Crate layering (respect this dep order)
`frus-core` (geometry/Color/Scene/SizeClass, zero-dep) ← `frus-gpu` (wgpu renderer, no winit), `frus-layout` (taffy wrapper), `frus-text`, `frus-widgets` (←layout+text, NO gpu) ← `frus-shell` (winit host, owns `run(app)`) ← `frus-demo` (bin). `frus-test` (←gpu+widgets) = headless snapshot/golden harness, dev-only.

## ⚠ Dev runs in WSL2, NOT native Windows
Windows Smart App Control is enforced and blocks Cargo build scripts (os error 4551) — native builds are a dead end. Build/test/run inside WSL Ubuntu-24.04 as root (repo is at `/mnt/d/...`):
```
wsl -d Ubuntu-24.04 -u root bash -lc '. ~/.cargo/env; cd /mnt/d/.../frus && cargo test'
```
- Set `CARGO_TARGET_DIR` to a **Linux** path (not `/mnt/d`) — far faster.
- Run the app: `bash scripts/wsl-run.sh` (forces winit X11 backend; WSLg Wayland-as-root breaks with Broken pipe).
- GUI check: `timeout 8 cargo run -p frus-demo` — **exit 124 = success** (ran without crash). For visual proof, prefer an offscreen render + pixel-readback test (WSLg GPU is software llvmpipe).

## Conventions (non-obvious)
- **UI-visible strings must be English.** Docs/comments may stay French.
- Widgets **theme themselves at paint time**, not at construction (paint receives theme/`Status`).
- UI world is **logical px**; DPI scale applied only at shell boundaries. Colors authored **sRGB**, converted to linear at the GPU edge.
- **Controlled widgets**: app owns state; composites use the "rebuild pattern" (`children = [...]`, realize only what's shown).
- Stable identity via `child_id`/`Keyed`; keep id-deriving walks consistent or retained state (hover/focus/edit/anim) breaks on reorder.
- `Container::child` is mono-child (clear+push). A `Percent(1.0)` widget needs a parent with a defined size (Auto parent collapses it to 0).

## Workflow
Work proceeds in numbered "jalons" (milestones): one clean commit per jalon on `master` (no remote), each builds; write `docs/jalon-N.md`. **No `Co-Authored-By` trailer in commits.**
