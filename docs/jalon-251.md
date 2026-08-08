# Jalon 251 — Drag ghost including a rich card's content

## Analysis

The drag-and-drop ghost (milestones 248, 250) copies, translated and un-clipped, the primitives of the
grabbed element. The filter kept the primitives whose **owner** is exactly the grabbed card
(`p.owner() == id`). That is enough for a **text** card (the card paints its own background and label),
but **not** for a **rich** card: its content (a label, tags, an × button) is painted by **children**,
under other owners. As a result, a rich card's ghost showed only an **empty tile** — the limitation
noted in milestone 250.

## Technical decisions

- **The ghost = the whole subtree's primitives.** Rather than the card alone, the shell gathers the
  primitives of **every** widget in the grabbed element's subtree (the card **and** its children). The
  paint order is preserved (the background first, the content next), so the ghost is faithful.

- **A new `subtree_ids(widget, root_id)` utility** in `frus-widgets`: the identities of the subtree
  rooted at `root_id`, derived by the **same positional scheme** as `collect_ids`
  (`child_id(id, index, child)`). The shell turns it into a set of owners and filters the scene on it.

- **A safe fallback**: if the tree is unavailable, we fall back on the old behaviour (owner = the card
  alone).

## Implementation

- `frus-widgets/src/ui.rs`: `pub fn subtree_ids` (+ the export in `lib.rs`). The
  `subtree_ids_covers_a_widget_and_its_descendants` test (from the root ≡ `collect_ids`; from a child:
  starts with its identity, a strict subset of the tree).
- `frus-shell/src/app.rs`: `paint_reorder_preview` computes the owner set through `subtree_ids` and
  filters the ghost's primitives on it.

## Verification

- **Widgets 387**; **shell 26**. The mechanism (`subtree_ids`) is covered by a unit test: the ghost's
  owner set does contain the card **and** its descendants.
- **No regression**: no static rendering changed — the ghost only exists during an engaged drag,
  outside the goldens; the goldens unchanged.

## Notes

- The ghost remains **runtime** state (a drag in progress), not inspected on a GPU in this environment.
  This milestone fixes the ghost's **composition** (the source of its primitives); its verification
  covers the subtree collection logic.

## What's left

- A **between-cards** insertion cue (above/below depending on the hovered half).
