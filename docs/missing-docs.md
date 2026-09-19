# Rust documentation checks

The public Frus crates use `#![deny(missing_docs)]`. New public modules,
types, methods, and fields therefore need rustdoc before they can be merged.
Run the affected crate's documentation build together with the workspace
tests:

```bash
cargo test -p frus-image
cargo doc -p frus-image --no-deps
```

Keep examples small and make them compile as doctests when they describe a
public API.
