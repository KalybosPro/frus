# Milestone 729 — A component's whole tree is found by its key

## Objective

Milestone 719 found that `host::scroll_to("list", ..)` did nothing when the keyed list was a
component's whole tree — `component(|cx| Box::new(keyed("list", ListView::new(..))))`, or a
`FrusApp::from_fn` whose page is the list. The same lookup serves `host::focus`, so a field a
component returns as its whole tree could not be focused by its key either.

## Why

A component stands in the tree where its built widget stands, under the same identity: it is
transparent, its children are its built widget's children. Its **key**, though, is its own and
never its built widget's — deliberately: the parent reads it to place the component before the
component is built, and a key that appeared once the child arrived would move the place under
the state kept there. So the layout gave the list the component's identity, and the lookup by
key, which asks each widget for its key and then descends into its children, asked the
component (no key) and then the list's own children: the `Keyed` the component had built was
never asked.

## What was done

- `Widget::built_child`, a default method answering `None`, which a `Component` answers with
  what it built (once built). It is the one thing a lookup needs that the transparent
  component hides; layout and identity are untouched.
- `carries_key`: a widget carries a key when it is its own, or when what a component built
  carries it — through any chain of components that build one another.
- `find_by_key` (scroll and focus requests) and `find_sheet_by_key` (sheet requests) use it. A
  match on a component returns the component's identity, which is the one the built widget was
  laid out under.

## Tests

- `tester.rs`: a 500-row list that is a component's whole tree, keyed, is scrolled to 400
  pixels by `host::scroll_to` and shows row 10, not row 0. With the lookup as it was, the test
  fails (the list stays at its top) — checked.
