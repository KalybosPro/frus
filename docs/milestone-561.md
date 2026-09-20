# Milestone 561 — The addresses, seen in a browser

## Objective

Milestone 560 connected the router's location to the browser's address bar and said, in its own
notes, that nothing of it had been seen in a real browser. This step sees it — and settles the
two costs 560 had left in the open.

## What was found by running it

A headless Edge, driven through the DevTools protocol, with the demo built for the web (milestone
562) and served. `scripts/web-address-check.py` is what was run, and stays.

**One real bug, that no test could have found.** The browser's back button changed the address
and the application did not follow: the screen stayed on Settings while the bar said `#/`. The
history was right, `popstate` fired, the shell drained it in the next frame and told the router —
and the router started a slide, which is ticked by the frames that come *after* it. Whether a
frame is followed by another is decided at its top, from whether the application was animating,
and the address is read a few lines later: a slide begun there was never ticked, and sat at its
first frame for good. The fix is the one call every other entry into the loop makes,
`request_redraw`. The state machine was correct and its tests green; the bug lived in the
seam between it and the loop.

**Everything else held**: a bare page gets its address filled in (`#/`, entry 0); a page opened at
`#/settings` opens there; the browser's back and forward move the application and the screen
follows; a reload comes back to the same page and makes no entry; a task pushed from the list is
`#/task/0`, one entry more, and the application's own Back goes back in the list instead of adding
to it.

## The two costs 560 left open

- **A page opened deep had no entry beneath it**, so the application's own Back added one
  (`/users/42`, then `/`) and the browser's Back returned to the page just left. The page that
  is opened as a **new visit** — not a reload, not a return to an entry made earlier — now gets
  entries made for the pages beneath it: the router's stack, `/` then `/users/42`. The entry
  the page opened at becomes the bottom of it (`replaceState`) and each page above is pushed. So
  the browser's Back from a deep page lands on the page beneath inside the application, and the
  application's Back is a step back in the list, not a copy on top. The cost: leaving the
  application by Back from a deep link takes one press more than it did.
  `Application::location_stack` is the hook that says what the stack is; its default is the
  location alone, which makes nothing.
- **A `replace` was reported as a new location**, so it added an entry where the router swapped
  one. `GoRouter::replaced` says the last move was a swap and `Application::location_replaces`
  carries it: the entry is swapped too. It is cleared by any other move.

## Verification

- The history: 21 tests, now 26; the application: 7, now 9; `GoRouter`: 4, now 6. The new ones
  are the deep page's entries, the application's own Back from it, the browser's Back from it, a
  reload making none, a swap replacing.
- In a browser: `scripts/web-address-check.py` — 15 checks, all passing, after the fix; the one
  that compares what is on screen before and after the back button fails without it.
- The state machine is not asked to have found the bug; the browser did.
