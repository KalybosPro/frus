# Milestone 304 — Pictures, and somewhere to start

Two requests, one purpose: the repository has 300 milestones of work in it and its
front page showed none of it, and its issue tracker was empty, so there was nowhere for
anyone to begin.

## The pictures are rendered, not photographed

The obvious way to put screenshots in a README is to run the demo, screenshot the
window, and crop. That produces pictures nobody can regenerate: they are the size of
whatever monitor was to hand, they are taken at whatever the application looked like
that afternoon, and six months later they are quietly a lie.

So the pictures come out of the framework itself. `Application` is public — `init`,
`update`, `view`, `theme`, `tick` — and `frus-test` already renders a widget tree
offscreen through the same pipeline a window uses. Between them there is nothing left
to invent:

```rust
let mut app = TodoApp::default();
let _ = app.init();
let _ = app.update(Msg::Push(Route::Charts));
while app.tick(DT) {}                        // let the spring settle
let theme = app.theme();
let root = app.view(&theme, width as f32, height as f32);
stage.settle(root.as_ref());
stage.render(root.as_ref()).unwrap().write_png(&path);
```

`cargo run -p frus-demo --features shots --bin shots -- docs/media`, behind an optional
feature so that a GIF encoder and the test harness stay out of the demo's normal
dependency tree. New public API, one function: `Snapshot::write_png`.

The GIF is the same loop with the clock running: push a route, capture every frame the
application says it is still animating, hold, pop, repeat. Four screens, the real
spring transitions, no video capture and no screen recorder.

### Making a GIF that is not four megabytes

The first cut was 3.3 MB, which at the top of a README is an insult to anyone on a slow
connection. Three things brought it to 1.1 MB:

- **One frame in three.** A GIF stores every frame whole, so the frame count *is* the
  file size. 10 fps on a spring is honest enough.
- **Identical frames are folded into the previous frame's delay.** The tour spends half
  its length holding still on a screen, and holding still is the one thing a GIF
  expresses for free.
- **Three stops instead of four.** Each stop costs two transitions. Settings moved to
  the stills, where a screen costs a PNG.

### The one photograph

The Android picture is a real screenshot of a real phone, and it is the only file here
that is not rendered — a rendering could not honestly claim what that image is there to
claim. Its status bar and navigation bar are cropped off: neither is the framework, and
the first one showed which applications the maintainer has notifications from.

The first hero attempt is worth recording as a mistake: the home screen at 1440×900,
which sounded like a hero and rendered as a narrow column of content with a third of the
frame empty on each side, because the layout centres its content at a maximum width. The
useful size for a screenshot is the size the content actually wants.

## Fourteen issues, written to be startable

`.github/issues/`, one markdown file each, and `scripts/seed-issues.sh` to create them
and their labels through the GitHub CLI. They live in the repository so that they can be
reviewed like any other change, and so that a fork does not start with an empty tracker.

The rule they are written to: an issue exists so that somebody who has never read this
codebase can finish the work without asking three questions first. Each says why it
matters, where to look — crate, file, function — how to know it is done, and where the
obvious wrong turn is. Several of them say *"one crate is a perfectly good pull
request"*, because the most common reason a good first issue goes untaken is that it
looks like a weekend.

Most are drawn from the roadmap, which already tagged them 🟢 / 🟡 / 🔴; the labels use
the same three words, so a roadmap line and a label mean the same thing. One
(`NavBar` collapsing around its back button) is a real bug that milestone 296 found and
nobody has fixed.

They are live: [issues 7–20](https://github.com/KalybosPro/frus/issues), with the
labels the script creates.

### The failure that nearly happened

The script was written and shipped as bash, on a machine developed on in PowerShell.
Run there as `./scripts/seed-issues.sh` it printed **nothing at all** and returned
success — PowerShell does not execute a `.sh` and does not say so. Silence reads as
"it worked"; `gh issue list` afterwards showed an empty tracker.

Two fixes, and the first is the interesting one:

- `scripts/seed-issues.ps1`, a wrapper, rather than a line in a README telling people
  to type `bash` first. The failure mode was silence, and a note nobody reads does not
  compete with a command that works. It turned out to need more than finding a bash:
  on this machine the only `bash` on `PATH` is **WSL's**, whose filesystem view puts
  the repository at `/mnt/d/...` and for which `D:/...` is not a path at all — so it
  answered "No such file or directory" naming a file that plainly exists. The wrapper
  prefers Git for Windows' bash and translates the path when it has to settle for
  WSL's. It also drops `$ErrorActionPreference` back to `Continue` first: under `Stop`,
  Windows PowerShell 5.1 turns every line written to stderr into a terminating error,
  so a file skipped exactly as designed would have been reported as a crash.
- The script skips an issue whose **exact title** is already open. It originally
  documented the opposite, on the reasoning that guessing which of two similar issues
  is "the same one" is worse than an obvious duplicate. That reasoning stands for
  *similar*; a title match is not a guess, and with the issues now posted the next run
  would otherwise have made fourteen duplicates. Re-running it creates nothing, which
  is how it was checked.

## Left

- The README's gallery is four stills and a GIF. There is no picture of the framework on
  the **web**, which is the target a reader is most likely to doubt.
