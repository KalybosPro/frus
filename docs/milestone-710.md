# Milestone 710 — A screen reader can tick a box and fill a field

## Objective

Two gaps found while checking milestone 708 through Windows' UI Automation: a task's checkbox
did not change on a **Toggle**, and the new-task field stayed empty on a **SetValue**, while an
**Invoke** on a button worked. A person who uses a screen reader, dictation or a voice-control
tool could press frus's buttons, but not tick its checkboxes or fill its fields.

## What was found

- **The checkbox: a request nobody woke up for.** The checkbox was right — marked clickable,
  with its message — and the shell's path for an assistive technology's press ticked it, as a
  test through the windowless driver showed at once, on the demo's own row. The difference was
  the event loop: AccessKit calls frus's action handler whenever the assistive technology asks,
  and the handler only queued the action. The queue is drained when the loop next starts — and
  an idle window's loop does not start until something else happens. The Invoke on a button
  that "worked" had had something else happen; the checkbox, on a still screen, waited forever.
- **The field: a request nobody answered.** The bridge forwarded *click* and *focus* and
  dropped everything else, *set value* included — and no field said it could be set.
- **And the field had no name.** It had no label, so a reader announced it as "edit", with
  nothing to say what it was for.

## What was done

- **The action handler wakes the loop**: after queuing an action it asks the window for a
  redraw, which brings the loop round on every system, and the queue is drained at its start.
  Every action an assistive technology asks for now takes effect at once, on a still window too.
- **Set value**: an editable field (a text input, not disabled) now offers it, and the request
  is forwarded with its text. The shell puts the text in the field through the message typing
  it would have produced — the path an autofill and an undo already take — so a field that
  refuses typing refuses this too, with the caret after what arrived.
- **A field with no label is named by its placeholder**, as the reference reads a field's hint
  out when it has no label; a label still wins, and an error is still appended.
- **The demo's new-task field** has a placeholder, *New task*: what the field is for, shown while
  it is empty, and its name for a reader.
- **Two driver hooks** for tests: `assistive_click(label)` and `assistive_set_value(label,
  text)`, the shell's own path for those requests, on the node a reader knows by that name.

## Checked on the machine

Windows 11, the demo with one task, driven through UI Automation:

| Request | Before | After |
|---|---|---|
| Toggle on the task's checkbox | stays *Off* | **On**, the task struck through |
| The new-task field's name | *(none)* | **New task** |
| SetValue "Buy bread" on it | stays empty | **Buy bread** |
| Then Invoke on **Add** | — | the field empties, the Tasks tab's count goes from 0 to **1** active task |

## Left

- The web's bridge (milestone 564) forwards a click and a focus too; whether a browser's
  assistive technology sets a value through it, and how, was not looked at.
- Rows out of a list's view are not in the tree, so a reader reaches them by scrolling; whether
  they should be, as the reference keeps a few beyond the edge, is a question of its own.

## Tests

- `a11y.rs`: an editable field offers set value, a disabled one and a label, button or checkbox
  do not; a request becomes the shell's action — click, focus, set value with its text — and set
  value with no text, the root, or an action the shell does not answer, nothing.
- `app.rs`: activate ticks a checkbox, and again unticks it; activate ticks a checkbox merged
  with its caption; set value fills a field.
- `textinput.rs`: a field with no label is named by its placeholder; a label wins; an error is
  appended.
- The demo: a screen reader ticks a task; a screen reader adds a task, filling the field it knows
  by its placeholder and pressing Add.

The wake itself is not reachable from a test without a window — the driver runs the loop
itself — and was checked on the machine above.

## Mutation testing

Six mutants, all killed:

| Mutant | Killed by |
|---|---|
| Set value not offered | `an_editable_field_offers_set_value` |
| A disabled field offers it | `an_editable_field_offers_set_value` |
| Set value dropped by the bridge | `a_request_becomes_the_shells_action` |
| The value not applied to the field | `set_value_fills_a_field` |
| The placeholder not a name | `a_field_with_no_label_is_named_by_its_placeholder` |
| The placeholder winning over the label | `a_field_with_no_label_is_named_by_its_placeholder` |
