# Milestone 564 — Accessibility on the web

## Objective

Issue #18: the semantic tree already exists and reaches AccessKit on desktop, and on the web it
was built and dropped. AccessKit has no web adapter to bridge to — its own README lists the web
as a **planned** platform, not a shipped one — so this is closer to the writing than the
bridging the issue expected, done straight against the frus semantic tree the desktop bridge
already reads.

## What it does

- **`frus-shell/src/a11y_web.rs`**: one real DOM element per semantic node, built as **canvas
  fallback content** — a child of the `<canvas>` winit attaches the window to. Unpainted when the
  canvas renders (the platform's own contract for that position), but walked into the browser's
  accessibility tree, `Tab`-reachable, and readable by a screen reader — the documented way a
  canvas-drawn interface stays accessible. A clickable node is a `<button>`, so `Enter`/`Space`
  activate it without a hand-rolled key handler; everything else is a `<div>`. Diffed against
  last frame's set rather than rebuilt from nothing. Role, name, checked state, numeric range and
  disabled all map to the matching ARIA attribute; a value with no ARIA slot of its own becomes
  `aria-description` (or, for `Role::TextInput`, the element's own text, which is how a browser
  resolves a plain `role="textbox"`'s value with nothing editable inside it).
- **The live region is not fallback content** — the one deliberate exception. `aria-live`
  machinery watches the *render* tree, and a subtree the canvas never paints has nothing in it
  for that machinery to watch; found by the announcement never reaching Chrome's own computed
  tree until the element moved to a real, laid-out (`position: absolute; width: 1px; height: 1px;
  clip: rect(0,0,0,0)`) sibling in `<body>`.
- **`TextField::announce`** (`frus-widgets/src/textinput.rs`): the two existing places the shell
  reads a widget's `announce()` — a click, and keyboard `Enter`/`Space` on a *clickable*
  focusable — never fire for a field's own `Enter`-submits-instead-of-typing path, so nothing a
  field did could ever announce anything, on any platform. A third call site in `apply_key`
  reads it the same way, before the dispatch that rebuilds the tree.
- **The demo's checkbox is named** (`crates/frus-demo/src/screens/todo.rs`): `Checkbox::new(..)`
  carried no label — the caption is a separately animated `Text` beside it — so it read as
  "checkbox, ticked" with no way to tell which task. `Semantics::merge` joins the two into one
  control now that it is safe to (below), and the field submitting a task carries its own
  announcement, `"{text} added"`.

## The bug this found, not in the web bridge

`Semantics::merge` keyed its surviving node on the **wrapper's own id** — correct for the labels,
which is all the existing tests checked, and silently wrong for anything clickable underneath:
nothing routes a click through a wrapper's id, so a control merged with its caption announced
correctly and did **nothing** on activation. Desktop AccessKit too, not just the web bridge here
— the bug was in `Ui::apply_description` (`frus-widgets/src/ui.rs`), which both read. Fixed by
keying the merged node on the one clickable child's id when there is exactly one; two clickable
children have no single answer, so nothing is guessed and the wrapper's id is kept, as before.
Two tests cover it (`crates/frus-widgets/src/semantics.rs`): a merged button stays operable
(`ui.msg_for` resolves through the merged node's id), and a merge of two buttons is left alone.

Found the way this project usually finds these: not a unit test, a real browser driven over the
DevTools protocol, clicking a real accessibility node and reading back whether the task actually
ticked.

## Decisions

**Canvas fallback content over a positioned DOM overlay.** No bounding boxes to keep in step with
layout, no `pointer-events` games to keep the overlay from stealing real clicks — the browser
already does not paint it. The cost is exactly what "unpainted" means: `getBoundingClientRect` on
any of it is empty, so a touch screen reader's spatial "explore by touch" has nothing to work
from. A keyboard or virtual-cursor reader does not need one.

**Verified against the computed accessibility tree, not by listening.** Headless Edge, driven
over the DevTools protocol's `Accessibility` and `DOM` domains — `Accessibility.getFullAXTree` is
the same computed tree a screen reader consumes, not a heavier DOM query. `scripts/
web-accessibility-check.py`: the canvas carries `role="application"` and the app's title; the
"Menu" and "Delete task" buttons are named; adding a task exposes a checkbox named after it and
an `aria-live` announcement; `DOM.focus` on the checkbox's `backendDOMNodeId` followed by a real
`Enter` keydown ticks it, and the change round-trips back into the computed tree.

## Not done, and why

- **Not heard.** Verified against the computed accessibility tree Chrome hands assistive
  technology, not against NVDA, JAWS, Narrator or VoiceOver with the screen off, which is what
  the issue actually asked for and is a claim this milestone cannot make. What is checked is
  precise (role, name, state, the two-way action loop) and is not the same claim.
- **Touch "explore by touch."** Canvas fallback content is never laid out, so it has no bounding
  box a touch screen reader could use — recorded in `a11y_web.rs`'s own doc comment rather than
  worked around.
- **Firefox and Safari.** Only Chromium (Edge) was driven; the technique is the HTML canvas
  specification's own, not a Chromium extension, but nothing here has run on the other engines.
