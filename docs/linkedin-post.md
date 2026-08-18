# LinkedIn announcement

The launch post for LinkedIn, kept here for the same reason as
[hn-post.md](hn-post.md): a copy pasted into a browser and lost is a copy nobody
can check. Attach [`docs/media/tour.gif`](media/tour.gif) as the image.

LinkedIn renders no Markdown — `**bold**`, `#` and `-` bullets appear literally,
and a `>` copied out of this file goes into the post as a `>`. So the post lives
in [linkedin-post.txt](linkedin-post.txt) as plain text with nothing to strip:
**copy that file, not the block below**, which is quoted only so this document
reads as a document. The two are kept identical by hand.

**It measures 740 characters.** The first draft ran to 1,616 and the composer
refused it by 820, so the ceiling in play is roughly 800 — not the 3,000 the
documentation advertises. Measure before pasting, and treat 780 as the practical
budget:

```powershell
(Get-Content docs\linkedin-post.md -Raw).Length   # or the body alone
```

The first two lines are what shows before "see more".

---

## Body

> I spent the last months building a UI framework in Rust. Here is what it looks
> like running.
>
> frus draws all of it on the GPU — layout, type, charts, spring transitions. One
> codebase, running today on desktop, Android and the web. iOS is next.
>
> The whole framework is Rust: renderer, layout, widgets, gestures, theming,
> animation, accessibility. No embedded VM, no second language for app logic.
>
> It's cargo-native — no DSL, no codegen, no bespoke CLI. Apps are Elm-shaped, so
> update is pure and ~970 of the tests run headless, with no GPU and no window.
>
> Honestly: pre-alpha, the API isn't stable, nothing on crates.io yet. 327 design
> notes say why every decision went the way it did.
>
> github.com/KalybosPro/frus
>
> #rustlang #opensource #ui

### What the trim removed

Kept: the hook, the three claims, the honest limits, the link. Those are the post.

Cut: the "no platform channel between your code and the pixels" clause, whose
qualifier needs more room than the budget allows and which reads as a boast
without it; the themed-defaults paragraph; the web's specific gaps, now folded
into "pre-alpha"; the closing call for an iOS shell; and the "Three things I'd
point at" scaffolding, which cost a line to announce what the next lines said
anyway. The design-notes sentence lost its clause about judging the foundations.

If the ceiling turns out to be higher than 800 elsewhere, the themed-defaults
paragraph and the iOS ask are the first two things worth restoring — in that
order. Both are in the git history of this file.

---

## Image alt text

Paste into LinkedIn's "Add alt text" field on the GIF — it also gives the post
something readable when the image doesn't load.

> Four screens of the frus sample application — a task list, a data table, a
> Kanban board and charts — entered and left through spring route transitions.

---

## Posting notes

- Tuesday to Thursday morning, in your own working hours. LinkedIn's feed favours
  posts that gather comments in the first hour, so stay reachable.
- The GIF goes in as the post image, not as a link. LinkedIn deprioritises posts
  whose main content is an outbound URL; the repository link sits in the body
  because the post has to stand on its own without it, and it can also go in the
  first comment if reach matters more than convenience.
- The hashtags are three deliberate ones. A wall of them reads as reach-chasing
  and doesn't help.

---

## Claims and their sources

Verified 2026-08-17 against the tree.

| Claim in the post | Source |
| --- | --- |
| Desktop, Android and web today; iOS next | README "Project status", ROADMAP platform matrix |
| Whole framework in Rust, no VM, no second language for app logic, no platform channel to the pixels | Same three claims as the Show HN post, with the same load-bearing qualifiers — see [hn-post.md](hn-post.md) for the sourcing and the prepared replies |
| ~970 headless tests | README, "Pure `update`, testable core" and the `cargo test --workspace` line |
| Pre-alpha, API unstable, nothing on crates.io, web missing clipboard/a11y/live-reload | README "Project status" |
| 327 design notes | `docs/milestone-*.md` file count |
