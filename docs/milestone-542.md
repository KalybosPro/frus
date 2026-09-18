# Milestone 542 — `frus-hello` denies a missing doc comment

Issue #14, continuing the sweep. `frus-hello` is the framework's "Hello, world!" — a
counter in one state struct, a pure `update` and a `view`, the source of the
`cargo generate` template. Neither `Counter` nor `Msg` is `pub`; the only public
surface the crate has is `run()`, which `frus::main!(Counter::default())` generates
for `src/bin/frus-hello.rs` to call.

`#![warn(missing_docs)]` found nothing: the macro-generated `run()` is not flagged
(rustc does not require doc comments on items a macro invocation produces at the
call site), and every genuinely local item — the `Counter` fields, `Msg`'s variants,
the `Application` methods — was already documented. Straight to
`#![deny(missing_docs)]`.
