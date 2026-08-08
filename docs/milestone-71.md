# Milestone 71 — Leaf→root key handling (3 states): Escape closes everywhere

The second item of §6: **key routing** on the way up, with the brief's
three-state result — and its immediate payoff: **Escape closes the topmost modal,
menu, drawer or sheet, from anywhere** (there was no Escape key at all until
now).

## The infrastructure

- **`KeyResponse<Msg> { Ignored, Handled(Option<Msg>), Skip }`** — `Ignored`
  keeps propagating up, `Handled` consumes (emitting the message if there is
  one), `Skip` stops the propagation **without** a fallback.
- **`Widget::on_key(&Key) -> KeyResponse`** (a hook, delegated by
  `Box`/`Keyed`/`Responsive`) — the focused widget receives it first, then each
  ancestor.
- **`find_path(root, id) -> Vec<&dyn Widget>`** — the root→target path (the same
  `child_id` identities as every other walk), traversed **in reverse** for the
  climb.
- **`Key::Escape`** added (never routed to editing: a text field ignores it, so
  it climbs).

## Routing Escape (shell)

1. **Climb** along the focus path: the first `Handled`/`Skip` stops it. `Portal`
   consumes Escape (`Handled(on_dismiss)`) — the "focus inside the dialogue"
   case.
2. **Fallback** if the whole path ignored it (or there is no focus): closing the
   **topmost** overlay — `Ui::top_dismiss()`, recorded while the overlays are
   rendered (the last rendered is the topmost; nested portals follow).

The lesson the test caught: under an open modal, the full-screen scrim
intercepts the hit-test — anything behind is unreachable by pointer. So **both**
paths are needed: the climb from focus *inside* the dialogue, and the
topmost-overlay fallback for every other case.

## Demo payoff (with no demo code change)

Every existing overlay already declares its dismissal (`.dismiss(...)`): the
confirmation modal, the menus, the drawer and the sheet all answer Escape for
free.

## Validation

- **248 tests**, all green — the new test pins: closing the topmost
  (`top_dismiss`), the root→content path crossing the portal, the portal
  consuming Escape on the climb, an empty path for an unknown target, and no
  dismissal when there is no overlay.
- A warning-free build; the demo did not panic.

## What's next (§6)

A regularised keyboard model (physical + logical + character), scrolling in 4
pieces (`Position/Controller/Physics/Activity`), the `padding`/`viewInsets`
split, focus scopes (trapping Tab inside a modal).
