title: Catch a size regression in CI
labels: good first issue, build, android

A minimal application is 4.9 MB installed (milestone 292), of which ~3.4 MB is the
bundled fonts — and nothing at all would notice if that doubled tomorrow.

### What to do

A CI job that builds `frus-hello` for `aarch64-linux-android` in release, measures the
stripped `.so`, compares it against a budget committed in the repository, and fails on
a jump.

- The budget belongs in a file someone can read and edit, not buried in the workflow.
- Print the measured size and the budget on every run, including a passing one. A check
  that only speaks when it is angry teaches nobody what normal looks like.
- Leave headroom: this should catch a doubling, not a 2% drift. Suggest a threshold in
  the pull request and argue for it.

### Where

`.github/workflows/ci.yml`, and see `docs/milestone-292.md` for how the floor was
measured the first time.
