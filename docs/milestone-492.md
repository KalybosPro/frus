# Milestone 492 — An application's licences

Closes [#43](https://github.com/KalybosPro/frus/issues/43).

`LicensePage`, `AboutDialog` and `AboutListTile`, a registry behind them, and
`scripts/gen_licenses.py`, which reads an application's **actual** dependency graph.

Every application distributed through a store, and every application depending on anything
under Apache-2.0 or MIT, has to show the licences of what it links. This framework gave it
nothing to show them with, so an application wrote its own screen or — more likely —
shipped without one. The obligation is silent: nothing warns anybody it is missing.

## The part with the decision in it

**Where the licence text comes from.** The reference keeps a registry that packages add
themselves to at startup, and that shape does not survive the crossing: a Rust crate cannot
run code before `main`, so nothing a dependency ships would ever register itself and the
list would be hand-maintained — which is to say wrong the first time somebody adds a
dependency. The issue said as much and asked for the argument before the code.

What Rust has instead is better, and it is the whole design: **cargo knows the answer.**
`scripts/gen_licenses.py` asks it, three ways:

- `cargo tree -p <app> -e no-dev --target all` for **which** packages are linked. `no-dev`
  because a test harness is not shipped; `--target all` because one generated file has to
  serve every platform an application builds for, and a notice for something a given build
  did not link is harmless where a missing one is not.
- `cargo metadata` for each package's manifest directory and its declared SPDX expression.
- the licence files each package **actually ships**, read from that directory.

The output is a plain text file the application embeds and registers in one line:

```rust
frus_widgets::licenses::add_all(include_str!("../assets/licenses.txt"));
```

The list cannot then drift from what is linked without the file changing, which is the one
failure mode that matters here.

### Three smaller decisions inside it

**Grouped by text, not by licence.** 442 packages ship 214 distinct licence files, because
the same MIT text differs by a copyright line and the same Apache-2.0 file differs by
whether its appendix was filled in. Forty of those 214 are Apache-2.0, and collapsing them
to one canonical copy would save about 340 KB of the 619 KB the file weighs. The generator
does not do it: "close enough to Apache-2.0" is the first step towards a list that does not
say what the application ships, and the size is not a cost of the feature — it is what four
hundred packages' licences weigh, and an application that ships them is paying for its
dependencies rather than for a widget.

**Sixty of those packages ship no licence file at all.** They declare an SPDX expression in
their manifest and leave it at that. They are not silently dropped and no text is invented
for them: they are gathered per declaration into one entry that says exactly that, with the
repository to go and look at. That is the honest report, and it is also the list to take to
those projects.

**A crate of a workspace keeps its licences at the root**, beside the manifest that
declares the workspace rather than beside its own — so frus's nine crates would have looked
like nine packages shipping nothing. The generator looks upwards for a workspace member,
which is how `frus-widgets 0.1.0` ends up under this repository's own two files.

## Nothing is registered by default, deliberately

The issue suggested including frus's own notices by default. Writing it made the case
against: a framework can only know its own subtree, an application's list has to cover the
**application**, and a token entry for frus would be a list that looks complete and is not.
Since the application generates its list from its own graph, frus's crates and everything
under them are in it anyway — the demo's file lists `frus-widgets`, `wgpu`, `winit` and
`taffy` because the demo links them, not because the framework said so.

What replaces it is louder: **an empty page says so, and says what to do about it.** The
obligation is a silent one; a page that renders nothing where the licences should be is
exactly how it stays silent.

## The widgets

`LicensePage` is the reference's master-detail, controlled the way everything here is: the
application owns which package is open, so the page fits a route as easily as a panel.
The list is a row per package with the count of licences covering it (the reference's
`licensesPackageDetailText`); opening one shows the whole text.

Two things about the text. It is **grouped by text and read by package**, so the page
transposes what the generator writes — the generator groups to deduplicate, and a reader
looks for `wgpu`. And the paragraphs are **re-flowed**: licence files are hard-wrapped at
seventy-odd columns for a terminal, which on a phone is a paragraph that runs off the side.
The reference re-flows for the same reason.

`AboutDialog` composes over `AlertDialog`, `AboutListTile` over `ListTile`. Both are thin,
and both are worth their names for the reason the reference gives them theirs: every
application writes this row, and every application gets it slightly differently.

**The page does not scroll itself and does not guess its height.** Four hundred packages
are taller than any screen, and a widget that decided its own height would be wrong in a
panel — milestone 263's rule, the same one that gives `Kanban` an explicit card height.
The screen puts it in the scroll, which is where a screen's content belongs anyway.

## The format, and why it is length-prefixed

```text
frus-licenses 1
@ <byte length of the text>
- <package> <version>
.
<exactly that many bytes>
```

A delimiter would have been shorter and wrong: a licence text can contain any line at all,
including one that looks like a separator, and several real ones do. The test writes an
entry whose text contains `@ 9`, a lone full stop and a line that looks like a package
entry, and reads it back whole.

A truncated file stops the walk and returns what was read rather than panicking. A page
missing its last notice is better than no page, and much better than a crash on the screen
somebody opened in order to comply with a licence.

## In the demo

An `AboutListTile` on the settings screen, the `AboutDialog` over that screen, and a
`Licences` route behind its button. The list is generated from the demo's own graph — 442
packages, 214 notices, 619 KB — and registered in `Application::init`, which is where it
belongs because there are three entry points and only one of them is a `main`.

## Verification

Eight tests on the widgets and the format, one on the demo's own list — that it parses,
that every notice has a text, and that `wgpu`, `winit`, `taffy`, `cosmic-text` and the
framework's own crates are in it — and a golden of the page both ways round.

What no test can prove is that the file was regenerated after the last dependency changed.
That is what running the script is for, and it is written down in the script's own
documentation. A staleness check would need the registry sources, which CI does not have.
