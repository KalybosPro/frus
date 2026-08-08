# Jalon 33 — New widgets: Collapsible, Menu, Chip

Three more widgets (disclosure / actions / labels).

## Widgets

- **`Collapsible::new(title, open, on_toggle).content(w)`** — a controlled
  **collapsible** section. A `[header, content?]` composite: the content is only
  realised when open → so its appearance and disappearance get the mount/unmount
  fades **for free**. Clickable header (title + chevron ▸/▾), focusable.
- **`Menu::new(anchor, open, on_dismiss).item(label, msg)`** — a **floating**
  action menu (through `Portal`, `Below` placement). Closes on an **outside
  click**. The items are clickable and focusable.
- **`Chip::new(label).on_remove(msg)`** — a compact label (tag / filter) with an
  optional remove cross (clickable, focusable).

## A useful generalisation

Closing on an outside click (`overlay_dismiss`) only applied to `Center` modals;
it now applies to **every overlay** (including `Below` menus): a full-screen hit,
underneath the content, emits the dismiss message. The dark scrim, on the other
hand, stays reserved for modals.

## Demo (integration)

- A "⋯" **Menu** in the header (actions: Save, Clear completed).
- A **Chip** for the active filter (other than "All"), removable → back to "All".
- A **Collapsible** "Advanced options" in the Settings "About" tab, containing
  `Chip`s ("beta", "experimental").

## Tests

- `Collapsible`: `[header]` when closed, `[header, content]` when open; the
  header emits the toggle.
- `Menu`: closed → no overlay; open → overlay + `overlay_dismiss`; a click far
  from the anchor emits the dismiss.
- `Chip`: label alone, or label + a clickable cross that emits the removal.
- 62 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Menu`: fixed width, no submenus and no item separators.
- `Collapsible`: no height animation (the content fades, it does not "slide").
