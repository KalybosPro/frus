# Show HN announcement

The launch post for Hacker News, kept here so the claims stay in one place and can
be re-checked against the repository before posting. Every factual statement in the
body is sourced from [README.md](../README.md), [ROADMAP.md](../ROADMAP.md) or the
tree itself; the checklist at the bottom records what was verified and when.

If the post is edited, edit it here — a copy pasted into a browser and lost is a
copy nobody can check.

---

## Title — pick one (Hacker News limit is 80 characters)

**A.** `Show HN: Frus – A Rust UI framework for desktop, Android and the web` (74)

**B.** `Show HN: Frus – A Rust UI framework for desktop, Android, web and iOS` (75)

**A is recommended.** iOS is planned and the architecture is built for it, but the
README and the roadmap both list it as not started, and a title lists what runs
today. The first reader to clone the repository and find no iOS target turns the
discussion into an argument about credibility rather than about the framework —
and that argument cannot be won, even when the roadmap is entirely sincere.

The body names the iOS intention in its opening paragraph, so a reader learns it
within the first ten seconds either way. That carries the signal without the
exposure. Use **B** only if reaching iOS developers matters more than the tone of
the first reply.

---

## Body

Two constraints. Hacker News does not render Markdown — blank lines separate
paragraphs and `*asterisks*` produce italics, while `#`, `-` bullets and backticks
appear literally. And **the text field is capped at 4000 characters**, which the
first draft of this post exceeded. The version below is written for both; paste it
unchanged.

It measures **3523 characters** as written, 3552 once the browser converts newlines
to CRLF on submit. The placeholder on the "why" line accounts for 21 of those, so
there are roughly **440 characters of safe headroom** for it — three sentences.
Count before pasting:

```powershell
$t = [System.IO.File]::ReadAllText("docs\hn-post.md")   # or the body alone
$t.Length
```

> Hi HN. Frus is a UI framework written entirely in Rust. Renderer, layout,
> widgets, gestures, theming, animation and accessibility are all Rust — no
> embedded VM, no second language for app logic, no platform channel between your
> code and the pixels. One source tree runs on desktop (Windows/Linux/macOS),
> Android and the web. iOS is next, and I say below how far along that is.
>
> Why I built it: [YOUR SENTENCES HERE]
>
> What it does today: flex/grid/wrap layout, 1D and 2D scrolling, text input with
> a real IME, drag-and-drop with live reflow, data tables, editable grids, charts,
> date/time pickers, trees, toasts, modals, drawers, navigation with spring
> transitions and a back gesture, an overridable theme generated from a seed
> colour, i18n and RTL, async HTTP with typed JSON, and golden-image testing.
> Three sample apps exercise it.
>
> How it works: apps follow the Elm architecture — a pure `update(&mut self, msg)`
> and a `view` returning a widget tree. Because update is pure, roughly a thousand
> tests run headless, with no GPU and no window. Below that: taffy for flexbox,
> cosmic-text for shaping, lyon for tessellation, and wgpu, which covers Vulkan,
> Metal, DX12 and WebGPU from one path. Everything platform-specific — window
> creation, IME, screen readers, the Android activity — lives in one crate,
> frus-shell. Everything above it is portable.
>
> One honest exception: NativeActivity provides no InputConnection, so frus-shell
> carries a 209-line Java shim, loaded as a bundled dex, purely so the IME has
> something real to talk to — composition, swipe typing and CJK depend on it. It's
> the only non-Rust code, it sits on the input path rather than the render path,
> and an app never touches it.
>
> A counter app is ~30 lines and ends with `frus::main!(App)`, which wires up the
> desktop, Android and web entry points. The source doesn't change between
> targets: `cargo run`, `cargo apk run`, or wasm-bindgen.
>
> What makes it different: it's cargo-native. No DSL, no codegen, no generated
> build directory, no bespoke CLI to install and keep in sync — just cargo build,
> cargo test, cargo apk run. And widgets ship themed defaults rather than
> hardcoded ones: if a widget paints it, you can restyle it or swap the slot
> without forking the library.
>
> &nbsp;&nbsp;git clone https://github.com/KalybosPro/frus
> &nbsp;&nbsp;cd frus
> &nbsp;&nbsp;cargo run -p frus-demo
>
> Needs stable Rust and a GPU with Vulkan, Metal or DX12 drivers. There's a
> cargo-generate template for starting your own app, wired for desktop and
> Android.
>
> https://github.com/KalybosPro/frus
>
> Limits, plainly: pre-alpha, and the API is not stable. Nothing is on crates.io
> yet, so dependencies resolve by path or git revision — the single biggest thing
> between the project and anyone trying it. Web works for rendering, input,
> animation and async, but clipboard, accessibility and live-reload aren't wired
> up there. Accessibility is full on desktop (AccessKit), partial on Android,
> absent on the web. No MSRV is pinned.
>
> On iOS: everything platform-specific lives in that one crate and the other
> fourteen don't know what they're running on, so adding a target should be
> writing one shell, not threading a port through the codebase. Should. Nobody has
> tested that bet, and I'd rather say so now than have you find out after cloning.
> If you've built a Rust iOS shell before, that's the highest-leverage place to
> help.
>
> Design decisions are written up one note per milestone, 318 so far, each with
> the alternatives weighed and why the choice was made. That's the project's real
> memory.
>
> Happy to answer anything.

### What the trim removed

Kept whole: the three claims, the Java-shim disclosure, the limits, and the iOS
paragraph — the honesty is the load-bearing part and cutting it would have saved
characters at the cost of the post's credibility.

Cut instead: the "Trying it:" and "GitHub:" labels, which said nothing a bare URL
and an indented command block don't; the docs and architecture URLs, both one click
from the repository page; and a layer of adjectives throughout. The design-notes
paragraph lost its closing clause about judging the foundations — readers who care
about that will reach the notes anyway.

### The one blank

The "why I built it" line is deliberately unwritten. It is the strongest paragraph
in any Show HN and the one readers can tell was ghost-written. Two or three honest
sentences: what you set out to build, and what stopped you.

---

## Prepared replies

The questions below arrive within the first hour. Response latency matters more
than the wording of the post, so the answers are drafted in advance.

**"How is this different from the mainstream cross-platform frameworks?"**

The mainstream answer to this problem ships a language runtime inside the app and
talks to the platform across a serialization boundary. Frus has neither: app logic,
widgets and renderer are one language with no FFI in the hot path, and the
toolchain is cargo rather than a bespoke CLI with its own package manager and
generated build directory. Concede the other side of it in the same breath — those
frameworks are vastly more mature and have an ecosystem, and frus does not.
Conceding the obvious is what buys credibility for the rest of the reply.

**"You claim no second language, but there's a .java file and a jni dependency."**

Both are real, and the post names the Java file first precisely so this arrives as
a detail rather than as a catch. The shim is
[`FrusTextBridge.java`](../crates/frus-shell/java/dev/frus/input/FrusTextBridge.java),
209 lines, reached from
[`android_ime.rs`](../crates/frus-shell/src/android_ime.rs) through `jni`. It
exists because `NativeActivity` exposes no `InputConnection`: without it every IME
falls back to `TYPE_NULL`, Latin keys only, with no composition, no swipe typing
and no CJK. It is compiled to a bundled dex and loaded at runtime, so packaging
never changes, and it sits on the input path rather than the render path.

The claim is *no second language **for app logic***, and those three words carry
it: an application developer writes no Java, no JavaScript and no Objective-C.
Never let the phrase shorten to "no second language" in a reply — the shim surface
grows rather than shrinks. It is wasm-bindgen glue and `index.html` on the web
today; it becomes Objective-C interop when iOS lands, and DOM nodes when web
accessibility is wired up.

**"No VM? Android apps run on ART."**

They do, and frus's do too, because `NativeActivity` is a Java class — that is the
platform's runtime, present in every Android app, and it is not something frus
ships. The claim is that frus embeds no runtime to *interpret application code*:
Rust compiles ahead of time to native code on desktop and Android, and to wasm on
the web.

Worth volunteering, because it is the strongest evidence the claim is structural
rather than aspirational: the usual reason a framework eventually embeds a VM is
hot reload, and [`reload.rs`](../crates/frus-shell/src/reload.rs) already delivers
state-preserving reload without one. Elm state is a single struct, so the shell
serialises it, relaunches the freshly built binary and rehydrates. No ABI to
stabilise, no interpreter, pure cargo.

**"No platform channel? You just said there's JNI."**

The claim is scoped to the render path — *between your code and the pixels* — and
that path is app → widget tree → scene → wgpu → Vulkan/Metal/DX12/WebGPU, with no
serialisation boundary in it. wgpu keeps that true on new targets. The JNI is on
the input path.

Keep the scope in every restatement. Paraphrased as "no platform channel", the
claim is simply false, and `jni = "0.21"` in
[`Cargo.toml`](../crates/frus-shell/Cargo.toml) is one grep away from any reader
who wants to check.

**"Why not <another Rust UI crate>?"**

The harder question, and it must be answered from current knowledge rather than
assumption. What can be stated as fact about frus: Android is a first-class target
validated on a physical device with IME composition and swipe typing, and the
widget set reaches data tables, charts, and drag-and-drop with live reflow.
Whether any of that is a gap in a neighbouring project is a claim to verify before
making it. An inaccurate comparison to a sibling crate is the one mistake that
reliably turns the thread hostile, and its maintainers read Hacker News too.

**"When iOS?"**

The most-asked question under any cross-platform launch. Answer with the
architecture — one shell crate, fourteen platform-blind crates, and wgpu already
speaking Metal — and with a real constraint rather than a date. wgpu covering
Metal is the strongest concrete point available: the renderer is not the hard
part, the shell is. A missed date costs more than "next, and here is what it
depends on."

**"Is this AI-generated?"**

434 commits, 318 design notes, Android validated on hardware. Link a milestone
note: they are specific in a way that settles the question better than a denial.

**"Performance?"**

`frus-bench` exists, and the O(n²) batch planner is already named in the roadmap.
Volunteering a known weakness before someone finds it reads as confidence, and the
benchmark is included so the claim is checkable.

---

## Posting notes

- Tuesday to Thursday, roughly 09:00–11:00 ET (13:00–15:00 UTC). Weekday US
  morning is the deepest front-page window.
- Stay at the keyboard for the first two hours.
- Never solicit upvotes. Voting rings are detected and the post is penalised.
- [`docs/media/tour.gif`](media/tour.gif) sits at the top of the README, so the
  click-through lands on a moving picture within a second. Keep it there.

---

## Claims and their sources

Verified 2026-08-16. Re-check before posting if the tree has moved.

| Claim in the post | Source |
| --- | --- |
| Repository is public | `github.com/KalybosPro/frus` → HTTP 200 |
| 318 design notes | `docs/milestone-*.md` file count |
| 434 commits | `git log --oneline` |
| "Roughly a thousand" headless tests | ~1,050 `#[test]` attributes in the workspace; the README states ~970 run with no GPU and no window |
| Desktop working, Android device-validated, web functional without clipboard/a11y/live-reload | README "Project status", ROADMAP platform matrix |
| iOS and native macOS not started | Same two documents. Planned, per the maintainer; no code exists as of this date |
| taffy, cosmic-text, lyon, wgpu, AccessKit, Fluent | README crate table |
| "No embedded VM" | Rust compiles AOT to native and to wasm; `reload.rs` achieves state-preserving hot reload by serialising state and relaunching the binary, not by interpreting. Android hosts ART because `NativeActivity` is a Java class — the platform's runtime, not one frus ships |
| "No second language **for app logic**" | One non-Rust source file in the framework: `crates/frus-shell/java/dev/frus/input/FrusTextBridge.java`, 209 lines, plus `jni = "0.21"` in `crates/frus-shell/Cargo.toml`. Shell plumbing on the input path; an app writes none of it. The qualifier is load-bearing — see the prepared replies |
| "No platform channel **between your code and the pixels**" | Render path is app → widget tree → scene → wgpu → native API, with no serialisation boundary. The JNI bridge is on the input path. The scope qualifier is load-bearing |

Two documents linked from [README.md](README.md) in this directory — `brief.md` and
`status.html` — are not present in the tree, and that index still reads "305
milestone notes" against 318 on disk. Both are worth fixing before the post drives
traffic here, and neither is fixed by this file.
