# Milestone 713 — A field being edited is found once

## Objective

Milestone 711 put a real `<input>` over a web field while it has the focus, so the browser can
type, compose and dictate into it. It left the field with two nodes in the page's accessibility
tree for as long as it was edited: the accessibility bridge's node for it, in the canvas
(milestone 564), and the element. A reader browsing the page met the field twice, once of them
an empty-looking copy that took no typing. The browser check of milestone 711 shows it on master:
the add field, focused, is found as `['element', 'bridge']`.

## What was done

- **The bridge hides the field's node while the element stands for it.** The editor says which
  field it is editing (`Editor::editing`), and the bridge sets `hidden` on that field's node —
  out of the accessibility tree, and out of the focus's reach — and takes it off when the
  editing ends. The element is the field's one node: named like it, holding its text and
  selection, with the browser's focus.
- **The editor goes first each frame.** It used to follow the bridge, which focused the
  field's node in the frame the field took the shell's focus, before the element existed. Now
  the element is made and focused first, and the bridge hides the node in the same frame — a
  hidden element takes no focus, so the bridge's own focusing of the field does nothing. When
  the editing ends while the field keeps the shell's focus (it stopped taking typing), the
  node is shown again and the bridge gives it the focus.

## Checked in a browser

`scripts/web-text-editing-check.py` now asks the accessibility tree, as a reader would, for the
textboxes named *New task*, and tells them apart; and it logs every element the focus lands on
once the check has focused the field's node, as a reader moving to the field does:

| | master | this milestone |
|---|---|---|
| The field focused, being edited | `['element', 'bridge']` | `['element']` |
| After Tab | `['bridge']` | `['bridge']` |
| Where the focus went | | the node, then the element, once (`['node', 'element']`) |

All six web checks pass on the demo built from this branch: five rounds of the six, and the
text-editing check four times more. One run of the browser-shortcuts check never reached the
browser — its DevTools port was not up yet while a build ran beside it — and passed the two
times it was run again. On master, the first of the new lines fails, as above.

## Tests

- `scripts/web-text-editing-check.py`, three lines added (above). The bridge and the editor
  are the web's only and have no native test host; what they do is the browser's to show.

## Mutation testing

Three mutants, each built for the web and run through the browser check.

| Mutant | Result |
|---|---|
| The field's node not hidden | killed: `['element', 'bridge']` |
| The bridge not focusing the field being edited (a filter on its focus) removed | survived, with every focus move logged: equivalent — a hidden element takes no focus. The filter went. |
| The end of an editing not a reason to give the focus again | survived: the demo has no field that stops taking typing while it keeps the focus. Left in, unchecked. |
