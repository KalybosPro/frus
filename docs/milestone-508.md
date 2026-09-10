# Milestone 508 — What the counter weighs, and a check that says so

Answers [#9](https://github.com/KalybosPro/frus/issues/9).

Milestone 292 found where 286 MB went (symbols, in a debug build), set a release profile,
and made the fonts a choice. What it could not do was keep the answer true: nothing in CI
would notice if the smallest application doubled tomorrow.

## Today's number

`frus-hello`, the counter, built for `aarch64-linux-android` with the workspace's release
profile — the stripped library an APK ships:

| | `.so` | gzip -9 |
|---|---|---|
| milestone 292 | 10,012,520 | — |
| milestone 508 | **10,427,568** | 4,794,223 |

Four per cent in two hundred and sixteen milestones, which is what drift looks like here.

## The check

- **`ci/size-budget.toml`** holds the budget, `budget_bytes = 13000000`, with the argument
  for it written above the number — a file someone can read and edit, not a constant in a
  workflow.
- **`scripts/check-size.sh <library> [budget]`** measures the file, prints its size, its
  compressed size (what it adds to a zip, for information), the budget and the share of it
  used — **on every run, a passing one included** — and fails over budget with a message
  saying what to do about it. It runs by hand as well: that is how it was checked here.
- **A `size` job** in CI builds the library and runs the script.

### Built without cargo-apk

The library is linked by the NDK's clang directly (API 24, `frus-hello`'s
`min_sdk_version`) rather than through cargo-apk. The `.so` is what is measured, and a
release APK needs a signing key the CI does not have and should not be given for this.

### The threshold, argued

About **25% of headroom**. The failures worth catching are jumps:

- the release profile going missing — the debug library was **300 MB**;
- LTO or stripping turned off;
- a dependency that arrives with half an ecosystem behind it;
- a font bundled by default — the four bundled groups are 3.4 MB together.

Each is tens of per cent or more. Drift has been 4% in two hundred milestones, so a quarter
leaves years of it; a margin of a few per cent would turn the check into a chore that
gets its budget raised without anyone looking, which is worse than no check.

### Blocking

Unlike the Android APK build next to it, which is advisory while its setup settles, this
job blocks. It uses only the NDK and the toolchain — no cargo-apk to install — and a check
that is allowed to be red is a check nobody reads (milestones 294, 298).

## Verification

- The library built and measured in WSL with the same commands the job runs.
- The script against the committed budget: passes, 80% used, 2,572,432 bytes of headroom.
- The script against a budget of 9,000,000: fails, with the overage and the instructions,
  exit status 1.
- The workflow's first run on GitHub is the pull request's: the NDK path comes from the
  runner's setup step, which cannot be exercised from here.
