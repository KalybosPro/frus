# Milestone 549 — Raising the MSRV floor for wgpu 30

Follows milestone 507 (#8), which pinned the workspace's `rust-version` at 1.88.
The wgpu 22→30 upgrade (this branch) pulled in dependencies that outgrew that
floor: CI's MSRV job (`cargo check --workspace --all-targets` on the pinned
toolchain) failed with

```
error: rustc 1.88.0 is not supported by the following packages:
  cosmic-text@0.19.0 requires rustc 1.89
  ordered-float@5.5.0 requires rustc 1.90
  smol_str@0.3.6 requires rustc 1.89
```

Per the rule milestone 507 set — raising the floor is a deliberate edit of the
manifest and the CI job together, never something a dependency bump does
quietly — `rust-version` in `[workspace.package]` and the `msrv` job's pinned
toolchain both move to **1.90**, the highest of the three requirements above.
