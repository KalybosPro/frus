# Milestone 560 — On the web, the router's location is the address

## Objective

Issue #31: a screen the browser can address. Until now every visit to a web build started at
the root and nothing knew the address bar existed — no screen had a URL, the back button could
not be told apart from leaving the site, a page could not be linked to or reloaded where it was.
The router of milestone 553 already speaks in *locations*, the strings a link would carry; this
step connects them to the address.

## What it does

- **A page opened at `https://host/app/#/users/42` opens there**, with home underneath, before
  the first frame, and with no slide: nobody has seen a page to move away from.
- **Moving between pages changes the address** and adds an entry to the browser's history.
- **The back and forward buttons move the router**, with the slide that goes the right way.
- **A reload comes back to the same page.**
- **A bare page keeps the application's own start**, and the address is filled in from it.

An application made of a `FrusApp::router(..)` does all of it with nothing written.
`.location_strategy(LocationStrategy::Path)` writes plain paths instead of the hash.

## Decisions

**A router is not built for this; the router that exists is connected.** The issue argued that
a table of patterns to screens was the wrong shape and that the framework should not ship one.
The premise held when it was written; milestone 553 then made routes the way an application is
written, so a location is already something the application understands. What is added are the
two hooks the issue asked for and nothing that parses a location: `Application::location` (a way
out — where the application is) and `Application::open_location` (a way in — an address was
asked for), both defaulted, both on the trait so an application that is not a router can use
them.

**The hash is the default.** A static host serves the page for any address whose path it knows,
and it knows one; the part after `#` never reaches it. A reload works with nothing configured.
The path form reads better and needs the server to answer every address with the page, so it is
a choice the application makes (`LocationStrategy::Path`, with the document's `<base href>` as
its base).

**Who owns the history** — the question the issue said to argue before any code, because both
sides pushing makes one screen two entries. The application owns where it is; the browser owns
the list. The agreement, in `history.rs`:

- The application reports where it is and the history follows. A location the current entry
  holds is nothing. One an *earlier* entry holds is that entry: the browser goes back to it
  (`history.go(-n)`) rather than getting a copy on top. Any other is a new entry, and drops what
  lay ahead, as a browser does.
- **The first report replaces the entry the page opened at.** A redirect on the way in — `/`
  sent to `/login` — would otherwise leave `/` behind it, and the back button would bounce off it
  forever.
- The browser reports where it went and the application is told, unless the browser is only
  answering a `go` this side asked for. If the application then ends up somewhere else — it
  redirected — the address is **corrected in place**, not added to.
- **A `go` is answered later**, and a `pushState` made before the answer lands on the entry being
  left. Nothing is sent between the two.
- Each entry the shell makes carries its own index in the browser's `state`, so a page that was
  reloaded partway, or a jump the browser made, finds its place in the list.

It is a state machine with no browser in it: sixteen tests run it against a stand-in — a list of
entries, one current, a `go` answered later — with a stack of locations moved as a router moves
on the other side, and five more read and write addresses. The calls to the real browser
(`address.rs`) are a hundred lines around `pushState`, `replaceState`, `go` and a `popstate`
listener.

**`GoRouter::open`.** The router gained the one call an address needs: like `go`, except that a
location it is already at is left alone, and that before a tree has been built from it the
router starts there with no transition.

## What it costs

- **A page opened deep has no earlier entry.** The browser never held `/` behind
  `/users/42`, so the application's own back button there adds an entry (`/users/42`, then `/`)
  rather than going back in a list that has nothing behind. The browser's back button then
  returns to the deep page. It is what a browser does with a page that was opened deep, and the
  alternative — inventing an entry behind it — needs a second page load.
- **A `replace` is reported as a new location**, so it adds an entry where the router swapped
  one. The application's location is a string; whether the last move was a swap is not in it.
- **The application's location is asked for after every message and every frame** on the web —
  for a router, a clone of a string.

## Not done

- **Not seen in a real browser.** The state machine is tested; the calls into the browser are
  compiled for `wasm32-unknown-unknown` and read against the DOM's documentation, and that is
  all. Nothing here has run in one, and the build tooling for the web (`wasm-bindgen`, a served
  page, a driven tab) is not on this machine.
- **The issue's "done when" — the demo served on the web** — because the demo does not build for
  the web: it works on threads and files, and its entry point lacks the `wasm-bindgen`
  dependency the macro names.
- Android app links and intent filters, and a desktop URL scheme with the second-instance
  handoff: separate pull requests, as the issue says.

## Verification

- `cargo test -p frus-shell -p frus-widgets --lib` — 172 and 1,705 tests, all passing; the new
  ones are 21 for the history, 7 for the application and 4 for `GoRouter::open`.
- Six mutations of the history, each caught: waiting on a `go` in flight, telling the application
  the answer to its own `go`, pushing the first report instead of replacing, searching back
  through earlier entries, correcting a redirect in place, and dropping what lay ahead of a new
  entry — the last one was not caught until a test was written for it.
- `cargo clippy --workspace --all-targets -- -D warnings` (and the shell's Android target),
  `cargo check -p frus-shell --target wasm32-unknown-unknown --all-features`, `cargo doc`
  with `-D warnings`, `cargo fmt --all -- --check` — clean.
