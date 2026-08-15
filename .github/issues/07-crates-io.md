title: Publish the crates to crates.io
labels: help wanted, build

Everything resolves through local `path` dependencies, so using frus means cloning it.
This is the single biggest thing standing between the project and anybody trying it.

### What it needs

- Real versions, and a decision about whether the crates version together or apart.
- Per-crate `README.md` — that is [its own issue](../../issues) and can land first.
- Metadata on every crate: `repository`, `keywords`, `categories`, `documentation`,
  `readme`.
- A publish order that respects the dependency graph (`frus-core` first, `frus` last),
  and a dry run that proves it.
- `publish = false` on everything that should never go out: the examples, `frus-demo`,
  `frus-bench`.
- Once it is done, the `cargo generate` template drops its `{{frus_path}}` question and
  becomes `frus = "0.1"`.

### Notes

This is a meaty one and it is worth splitting: metadata and `publish = false` flags
make a good first pull request on their own, and the release script a second.

Talk to the maintainer before the first actual `cargo publish` — names on crates.io
cannot be taken back.
