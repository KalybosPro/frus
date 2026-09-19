# Milestone 553 — A router

## Objective

An application with more than one screen has to say what the screens are called, how to get
to one, and what "back" is. Until now that was the application's business: a `Vec<Route>` in
the state, a transition controller, a back-gesture struct, and a `view` that assembled a
`Navigator` from them — about two hundred lines a screen-holding application wrote for itself,
the demo included.

The objective is a **router**: routes named by a pattern of path, a stack of pages moved by
*locations* — the string a link, a deep link or a button carries — redirects, and the motion
and the back gesture handled once.

## What was added

- **`GoRoute`** — a path (`/users/:id`, `files/*`), a builder, an optional name, sub-routes and
  a redirect. A top-level path starts with `/`; a sub-route's is relative to its parent's.
- **`GoRouter`** — the routes and the stack. `go`, `push`, `replace`, `pop`, the named forms
  (`go_named`, `push_named`, `location_for`), `go_with_extra`, `refresh`, `location`, `state`,
  `depth`, `can_pop`. A handle: clone it into a handler.
- **`GoRouterState`** — what a page is told: the location, the route's pattern and name, the
  path parameters, the query, and an optional object that came along.
- **Redirects** — one for the router, one per route, asked on every navigation and again on a
  `refresh_listenable`'s change; followed up to a limit, then the error page.
- **`FrusApp::router(router)`** — the application whose interface *is* the router; it also
  answers the back gesture. `BuildContext::router()` reaches it from anywhere below.

## Decisions

**Two verbs, as two questions.** `go` says *where*: the stack becomes the routes the location
is made of, so `/users/42/posts/7` has the user and home beneath it and there is somewhere to
go back to at each step. `push` says *on top of what is here*: one page, and the same page can
be open twice. A `go` that reaches a place the stack already passes through keeps the page
there — its key is the part of the location it answered — and so keeps what the page holds.

**A covered page is kept.** A stack of pages that lost what each held the moment another was
pushed would make every screen a fresh one on return: a half-typed form, an expanded panel, a
scroll. So `Navigator` gained `retain`: pages *under* the one on show are built, which is what
keeps the state of their components alive, and are not laid out, drawn or given input. What
identifies a page is its key, not its place among the navigator's children, so a page that goes
from kept to shown — or leaves as the transition ends — is the same page throughout. A page that
is popped is no longer built, and its state is disposed when the transition that carries it away
is done.

**The motion is the one the demo had.** The same spring, the same commit rule for the back
gesture (position plus projected momentum), moved into the router so that an application no
longer holds it. A pop that a back gesture carried out goes without a second transition: the
finger already played it.

**Redirects close over what they need.** A redirect is a function of the state of the
navigation, not of a build context — there is none outside a frame — so it captures a handle to
whatever it decides on (a signed-in flag, a notifier). Wiring the same object to
`refresh_listenable` is what makes signing out move the page with nobody navigating.

**No browser history yet.** The router keeps the location; it does not write it to the address
bar or read a deep link at start. That is a platform edge and belongs in the shell, in its own
step.

## Verification

- 21 tests on the router: initial location, the chain a `go` builds (with the parameters each
  page sees), query decoding, percent-decoding, wildcard, the error page, `push`/`pop`/`replace`,
  page identity across `go`, named routes and their locations, redirects (global, per route,
  loop, refresh), `extra`, the direction of a transition, back gestures past and short of
  halfway, and that a malformed route path is refused.
- 2 tests on `Navigator::retain` — the order of its children and that the retained count is
  reported — and end-to-end tests through the shell's windowless driver: a tap pushes a page and
  the transition runs; a covered page keeps its hook state and the same state is there after the
  pop; a covered page is not painted; a back gesture pops.

## Left out

Browser history and deep links; shell routes (a persistent frame around changing pages);
per-route transitions; typed route parameters; a pop that returns a value.
