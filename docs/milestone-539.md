# Milestone 539 — The golden-image check stops being advisory

Issue #15. The GPU job's golden step has carried `continue-on-error: true` since it
existed, on the strength of a reason recorded in the roadmap for years: "lavapipe
rasterises differently from hardware." An advisory check went red for five milestones
without anyone noticing (milestone 294) before that reason was ever checked.

## The reason was wrong

Both sides — the maintainer's own blessing and CI — run **lavapipe**. The maintainer
blesses goldens from WSL, and llvmpipe (lavapipe's software rasteriser) is the only
Vulkan adapter WSL exposes; CI's `gpu-test` job installs `mesa-vulkan-drivers` and runs
under `LIBGL_ALWAYS_SOFTWARE=1`, which is the same llvmpipe. Nothing here is comparing
software rendering against a GPU.

What differs is which **version** of that one rasteriser each side has. WSL is Ubuntu
24.04 (noble), and `apt-get update && apt-get install mesa-vulkan-drivers` there
currently resolves `25.2.8-0ubuntu0.24.04.2` from `noble-updates`. CI's `gpu-test` job
ran on `ubuntu-latest` and installed whatever apt gave that runner on the day — a live
target that moves independently of the maintainer's machine, both because `ubuntu-latest`
itself can point at a different release over time and because the live archive keeps
only the current package version, not the one CI happened to bless against last time
anyone looked.

## The decision

**Pin, don't tolerate.** The issue names two ways and prefers the first: "A tolerance
guessed without measuring is a number nobody can defend, and it will be raised again the
first time it fails." A guessed `channel_tolerance` would have hidden exactly the kind of
drift milestone 294 already showed nobody was watching for.

Two things are pinned, both needed:

- **The Ubuntu release.** `runs-on: ubuntu-24.04`, not `ubuntu-latest`. The `-latest`
  alias is a moving target by design — GitHub repoints it at a new LTS on its own
  schedule — and a release pin is what makes "the same archive as WSL" a meaningful
  statement at all.
- **The apt snapshot.** Even pinned to `ubuntu-24.04`, plain `apt-get install
  mesa-vulkan-drivers` still resolves against the **live** `noble-updates` archive, which
  only serves the current package version — the exact drift this issue is about, just
  with the OS pinned instead of floating. [snapshot.ubuntu.com](https://snapshot.ubuntu.com)
  serves every historical state of the archive, addressable by timestamp, and never prunes
  one once served. `gpu-test` now points `/etc/apt/sources.list.d/ubuntu.sources` at
  `snapshot.ubuntu.com/ubuntu/20260910T000000Z` — confirmed against Launchpad's own
  publishing history to be a date `mesa-vulkan-drivers 25.2.8-0ubuntu0.24.04.2` was
  already published and not yet superseded — instead of the live mirrors. `apt-get
  install mesa-vulkan-drivers` (no version pin needed, since the snapshot serves only
  one) now always resolves to that exact package, on every run, indefinitely: a snapshot
  is immutable by construction, unlike a PPA's or a container registry's tag, which
  someone could still repoint.
- **`Acquire::Check-Valid-Until "false"`** is set alongside it: a snapshot's `Release`
  file carries the `Valid-Until` of the day it was taken, which is now always in the
  past, and apt refuses a stale archive by default.

**Considered and not taken: a container image.** The issue's other option, and its own
words call it "the honest one" in the abstract — but a maintainer-built image is a second
thing to keep in sync with WSL's own mesa (rebuild and repush by hand whenever one moves)
where a snapshot timestamp is one string to edit in a file already under version control.
A `kisak-mesa`-style PPA was also considered and set aside: it would still need the exact
version pinned by hand (PPAs don't prune either, but nothing stops a `mesa-vulkan-drivers`
with no version qualifier from picking up the PPA's next upload), and it packages mesa
itself rather than serving Ubuntu's own build, which is one more thing that could
theoretically differ from what WSL runs. The snapshot is the official archive, frozen —
nothing about the package changes, only when it is fetched from.

**The golden step's `continue-on-error: true` is gone.** With the rasteriser pinned,
a red golden is a real defect, not an artefact of drift, and hiding it is exactly what let
milestone 294 go unnoticed. A `dpkg-query` line after install prints the resolved version
in the CI log, so a future mismatch shows up as a wrong version number in a log rather
than as a silent one.

## Verification

This is CI infrastructure: there is no unit test for "does GitHub Actions' apt resolve
this correctly." Verified by dispatching the workflow against this branch directly
(`gh workflow run`, since this repository's own rule is that I never open a pull
request) rather than trusting the plan on paper:

- **The snapshot serves the exact expected version.** Both by fetching
  `snapshot.ubuntu.com/ubuntu/20260910T000000Z/dists/noble-updates/main/binary-amd64/Packages.gz`
  directly beforehand and by the CI run itself: `gpu-test`'s install step's `dpkg-query`
  line printed `mesa-vulkan-drivers 25.2.8-0ubuntu0.24.04.2` — the same version WSL's own
  `apt list --installed` reports.
- **Launchpad's publishing history** for that exact version and architecture shows it
  `Published`, with `date_superseded: null` — the snapshot date chosen was not sitting on
  a version already stale when taken.
- **The workflow YAML parses**, and the heredoc that writes the deb822 sources file
  produces flush-left field lines with no leading whitespace (checked by loading the
  parsed `run:` script back out of the YAML) — deb822 treats an indented line as a
  *continuation* of the field above it, so a stray leading space here would have silently
  merged `URIs:` into `Types:` and broken every field after it. Confirmed again by the
  run itself: the pin step and the install step both succeeded.

## What the first real run found

With the rasteriser genuinely pinned, the goldens step failed anyway —
`the_constraint_boxes_match_their_golden`, 587 pixels differ. This is the finding the
whole issue is for: an advisory check had already let something drift, and turning it
blocking was the first thing to notice.

**Not a rasteriser problem, in the end.** Ran the same test under WSL, the maintainer's
own blessing environment — no CI, no pin, nothing borrowed from this milestone — and it
failed too, by a different pixel count (1116). Ran it twice in a row on WSL: the two
renders are byte-identical (`md5sum` agrees), which rules out run-to-run nondeterminism
(thread-scheduling races in llvmpipe's rasteriser, the first suspect for "two numbers,
neither zero"). The committed golden itself was simply behind the code that draws it,
on the very machine that blesses it — the pin didn't create this, it's the first thing
that was ever positioned to notice it, because nothing before checked WSL's own output
against the file on disk except a human choosing to run the suite.

**Isolated to one golden's text, nothing it draws.** A pixel diff (bounding box of every
differing pixel) landed entirely inside the two vertical "OVERFLOWED BY 42 PIXELS"
labels; the hazard-stripe fills, the tiles, the two other constraint-box demonstrations
in the same image are bit-identical. The overflow amount itself is unchanged — both
images say "42 PIXELS" — so this is not a stale *number*, only the anti-aliased glyph
edges of the label rendering a fraction of a pixel apart from where they did when the
golden was blessed. The likely cause: `crates/frus-test/tests/goldens/constraint_boxes.png`
was committed 2026-09-08; `fix(layout): half a pixel, made impossible to get wrong`
(milestone 497, the day after) changed layout rounding and re-blessed thirty other
goldens in the same commit, but not this one — whether because it genuinely rendered
unaffected at the time and something later shifted it, or because it was missed by
whatever the re-blessing pass covered, is not something git history alone can settle.

**Re-blessed under WSL** (`FRUS_UPDATE_GOLDENS=1`), looked at, and it is right: the
picture matches the test's own doc comment exactly — three constraint-box
demonstrations, an overflow band on both edges of the first, still saying 42 pixels.
Ran the **whole** golden suite (`goldens`, `widgets`, `motion` — 102 + 47 + the motion
fixtures) under WSL after re-blessing, to ask whether anything else was quietly in the
same state: all green, so this was the one.

**Confirmed on a second CI dispatch** after pushing the re-blessed golden: `gpu-test`
green end to end (2m15s), `dpkg-query` still printing `mesa-vulkan-drivers
25.2.8-0ubuntu0.24.04.2`, and the goldens step itself — 102, then 16 (`motion`), then 47
(`widgets`) — all passing, all now blocking rather than advisory.

## What is left

- **This snapshot date will need updating eventually.** When the maintainer's own mesa
  next moves (a WSL `apt upgrade`, or a distro upgrade), the CI snapshot has to move with
  it by hand — there is no automation tying the two together, only this note and the one
  in `ci.yml` saying what the pinned version is.
- **The other platforms' GPU paths are untouched.** This pins only the `gpu-test` job's
  lavapipe; Android, iOS and the web target build but do not run their GPU-backed tests
  in CI at all, which is a pre-existing gap this issue never claimed to close.
