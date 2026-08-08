# Milestone 198 — TextInput: clickable suffix (positional click)

## Analysis

A `TextInput`'s **suffix** icon was decorative. Yet many fields want an action *inside* the field: a
"✕" to clear, an eye to reveal a password, a magnifier to search. frus's clicking is
**position-independent** (`msg_for`, precomputed from `on_click`): there was no way to tell "a click
on the icon" from "a click in the text" without positional routing.

## Technical decisions

- **A positional click hook in the `Widget` trait.** A new
  `positional_click(local_x, local_y, width) -> Option<Msg>` method (default `None`), taking
  **priority** over `on_click`. On release over the clicked widget, the shell computes the **local**
  coordinates (cursor − the widget's corner, through `ui.widget_rect`) and asks it; if it returns
  `Some`, that message wins, otherwise we fall back to `on_click` (behaviour unchanged for every
  widget). The method is **forwarded** by `Box<dyn Widget>`, `Keyed` and `Responsive` so it crosses
  the wrappers.

- **`TextInput::on_suffix(msg)`.** Makes the suffix icon clickable: `positional_click` emits `msg`
  when the click falls in the **suffix zone** (the box's right edge, `suffix_hit`); and `cursor_at`
  returns `None` there so the caret is **not** placed. Without `on_suffix`, the icon stays purely
  decorative.

- **Demo: a clear button.** The task input field, when **non-empty**, carries a "✕" icon
  (`IconName::Close`) emitting `ClearDraft` → clears the field. It also unblocks, in time, the
  reveal eye (with an eye icon).

## Implementation

- `widget.rs`: the `positional_click` method (the trait + the `Box` forwarder).
- `keyed.rs` / `responsive.rs`: forwarders.
- `textinput.rs`: the `suffix_action` field + `on_suffix`; `suffix_hit`; `positional_click`; a guard
  in `cursor_at`.
- `frus-shell/src/app.rs`: on release, `positional_click` (local coordinates) takes priority over
  `msg_for`.
- `frus-demo/src/lib.rs`: `Msg::ClearDraft` + a conditional suffix icon on the input field.
- `goldens.rs`: `textinput_clear`.

## Verification

- **Unit**: `clickable_suffix_emits_and_blocks_caret` — a click on the suffix emits the message and
  places no caret; a click in the body places a caret and emits nothing; with no `on_suffix`, no
  positional click at all. `clear_draft_empties_the_field` (demo). The existing tests stay **green**
  (19 demo tests).
- **Golden** `textinput_clear` **inspected**: a "Buy milk" field with the "✕" icon on the right.
- `cargo build -p frus-shell` **clean**.

## What's left

- **A reveal eye icon**: reuses `on_suffix` to toggle `obscure` — an (outline) eye icon still has to
  be added to the set.
- **Hovering the suffix** (a hand cursor / highlighting the icon) — through a sub-region hover
  state.
