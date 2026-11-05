# Milestone 748 — Rows that keep their state

## Objective

Milestone 741 left two things undone about the rows of a list built on demand
(`ListView::new`, `ListView::builder`):

- **A row was its index.** The walk gave the row at index `i` the identity `list.child(i)`,
  whatever the row was. A key on the row (`keyed(..)`, `Keyed::new`, `Component::with_key`) was
  read everywhere else in the tree and ignored here. So when an item was inserted above a row, or
  one above it removed, everything kept by identity — the state of the components in the row, the
  hooks of a function component, a selection in its text — stayed at the index and passed to the
  row that came into it.
- **A row out of view was let go.** Only the rows in view are built, and a component's state is
  let go at the end of a rebuild that did not reach it. A row scrolled away and back came back
  new: a half-typed reply, an expanded section, a counter, gone. There was no way for a row to
  ask otherwise.

## What was done

- **A row with a key is its key.** The list identifies the row at index `i` as any parent
  identifies a child: by its key when it has one (`child_id`), by its index otherwise. Both
  kinds of list (fixed extent and any extent) do it at every step — the layout cache, the
  layout of the row, and the walk that paints it and registers what it holds. The state of the
  components in a keyed row, and a selection in its words, follow the row when items move above
  it.
- **`ListView::find_child_index`**: told a row's key (a `ChildKey`), says the index of its item
  now, or `None` once it has been removed. A row out of view that the list keeps alive is found
  with it: kept at its new index, or let go the frame its item is gone. `ChildKey::is(&value)`
  says whether it is the key made of `value`, with the hashing `Keyed::new` and
  `Component::with_key` use, so the callback compares the application's own ids.
- **Keep-alive.** `ListView::keep_alive(bool)` is the list's default (`false`: rows out of view
  are let go). A row decides for itself, from anywhere inside it:
  - `KeepAlive::new(child)` asks for its row to be kept, and `.keep(false)` declines a list that
    keeps by default;
  - `BuildContext::keep_alive(bool)` does the same from a component — the "automatic" kind: a
    row whose draft is not empty keeps itself, one whose draft is empty does not.

  A `true` from anything in the row wins; then a `false`; then the list's default.
- **What a kept row owns is held through the sweep.** The component store (`StateStore`) now
  knows which row each state was built in: a list enters a row while it builds, lays out and
  walks it, and every state made or reached meanwhile is owned by that row (the innermost, for a
  list inside a row of another). Each frame the list holds the rows it keeps but did not build,
  and the end-of-frame sweep spares a state whose row — or a row it was built in — is held. The
  rows it does not keep are not held, and their states are disposed as before: `State::dispose`
  runs and effects are cleaned up, so memory follows the rows kept, not every row seen.

- **A row's components are built before the row is laid out.** The walk before a frame
  (`build_deferred`) builds the components of the tree, but cannot reach a list's rows, which
  are made during the frame. The list now runs the same walk over each row it builds, under the
  row's identity, before the layout cache fingerprints it: every component in a row in view is
  built, and its state reached, every frame — the sweep's notion of "reached" depends on it, and
  so does `KeepAlive`, which asks as it is built.

## Decisions

**Identity by key, not a separate map from key to state.** Every retained thing in frus is kept
by `WidgetId`, and a parent's children already take their identity from their key. Giving a
list's rows the same rule moves all of it at once — component state, hooks, the layout cache, a
selection — where a map kept beside the list would have moved one of them.

**The callback is for rows out of view.** A row in view is found by building it: the builder
returns the row at index `i`, and its key says who it is. Only a row the list keeps and does not
build has to be looked for, which is what `find_child_index` answers. Without it, a kept row is
assumed to be where it was, and is let go only once the list is shorter than its index — which
keeps a keyed row's state under insertions that do not move it, and may keep a removed row's
state a while longer.

**Keys are asked, not read.** A key is reduced to a number when a widget is given it, so the
callback is handed a `ChildKey` and asks `key.is(&id)` rather than downcasting a key object.

**Ownership by scope, not by identity.** A state's place is a hash of its path, which says
nothing about which row it is under. Recording the row a state was built in, as it is built,
costs one push and pop per row and answers the question exactly, nested lists included.

**A `KeepAlive` wrapper as well as the hook.** A row built of components written elsewhere,
or of plain widgets around one, has no `BuildContext` of its own to ask with. The wrapper asks as it is expanded, so it works anywhere in the row, like the hook. It
is a transparent wrapper: the macro gained an `expand` variant for a wrapper that does something
as its child is built, and the forwarding of `expand` moved into a sub-macro that every other
variant includes.

## Tests

- `frus-widgets`, `list.rs` (`keep_tests`), each failing before the change:
  - a keyed function component's hooks stay with their row when an item is inserted above;
  - a keyed `StatefulWidget` (`with_key`) follows its item when one is inserted above and another
    removed, and only the removed item's state is disposed;
  - a row whose component calls `cx.keep_alive(true)` keeps its hooks when scrolled 1000 px away
    and back, while a row that does not comes back new and its state was disposed;
  - a list that keeps by default keeps every row but the one wrapped in `KeepAlive::keep(false)`;
    a list that does not keeps only the row wrapped in `KeepAlive`;
  - with `find_child_index`, a kept keyed row out of view comes back at its new index with its
    state after an insertion above, and is disposed the frame its item is removed;
  - a kept row holding a list keeps the inner list's rows' state while the outer row is out of
    view.
- `frus-shell`, through the driver with a mouse: in a `SelectionArea` around a
  `ListView::builder` of keyed rows, a row selected by a drag still copies its own words after an
  item is inserted above it, and has moved down; without keys the selection stays at the index
  and copies the row that came into it (the documented behaviour).
- The selection tests of milestones 575 to 578 and 738 to 742, the component tests of 551 to 556,
  and the transparent-wrapper forwarding tests pass.

## Not done

- A **grid** built on demand (`GridView::builder`) is a list of strips of cells, made by the grid;
  its rows are still their index, nothing in them is kept out of view, and a cell's key places it
  only within its strip.
- Only **component state** (states and hooks) is kept out of view. What the runtime keeps for a
  widget by identity — a hover or press animation, a scroll offset inside the row, an ink ripple —
  is let go with the row as before, and starts afresh when it comes back.
- The extents a list of rows of any extent has measured are kept **by index**: after an
  insertion above, the rows already measured are counted at their neighbours' extents until they
  are built again. The scroll offset is kept in pixels, so an insertion above the window moves
  the rows in view down, as it does in other toolkits.
- Not run on a phone or in a browser.
