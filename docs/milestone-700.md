# Milestone 700 — Words and tappable areas in the accessibility tree

## Objective

Found while checking milestone 698 through Windows UI Automation: of the demo's task screen —
a title, a state, a list of attached files, three actions along the bottom — a screen reader
could reach two things, the back arrow and the "✓" button. Two gaps, one in each layer:

- **Every plain text was silent on the desktop.** `Text` did put a `Label` node in the tree,
  with its words as the node's *label*. AccessKit names a label node from its **value** (on
  Windows the UI Automation name of a static text is the value; the label is not read), so each
  text was there, at its place, with no words. The live region that announces changes was a
  label node with its message as a label too: announcements came out empty.
- **A tappable area had no node at all.** A `Container` (or `GestureDetector`) with `on_click`
  states no semantics of its own, so the walk added nothing for it: "Delete", "Attach…" and
  "Back" on the demo's bottom bar could not be found, let alone activated. The reference gives
  anything with a tap handler a node carrying the tap, and a screen reader names it from the
  words inside.

## What was done

- **A label node's words are its value** (`frus-shell`'s AccessKit bridge): a `Label` entry's
  text — and the live region's message — go to the node's value; every other role keeps its
  label. Nothing changes in the widgets or in the tree frus builds, only in how it is handed
  over.
- **A tappable area is a button named by its words** (`Ui`'s walk): a widget with `on_click`
  and no semantics of its own gets, once its subtree is walked, a node with the button role and
  the click action, at its own box. The entries of its subtree that only *say* something —
  texts, headings, described images, none of them clickable — become its label, one per line,
  and leave the tree, so nothing is read twice; what can be operated on its own (a checkbox, a
  field, a button, another tappable area) keeps its entry, after the area's.
  - The **button role**, not a bare container with a click action: some platforms drop a
    generic container from the tree whatever it carries, and "button" is what says "this can
    be pressed" everywhere.
  - An area with **no words around exactly one clickable entry** adds nothing: that entry
    already answers for the press, and an unnamed "button" around it would only be noise.
  - The node keys on the area's own id, which is where its press is registered, so an
    assistive technology's "activate" reaches the area's message through the path every click
    from one already takes.
  - Done **after** the subtree's walk and inside `walk_node` rather than as a branch of its own
    that returns early, so an area that is also an opacity group, a barrier or a description
    loses none of that.
- **A button holding a widget is named by it.** A clickable widget that describes itself
  *without a name* — `Button::with_child(Text::new("Save"))` was a nameless button beside a
  separate "Save" — keeps its own entry and role and takes the words below it as its label,
  as the reference's button reads the text inside it. A named one (`Button::new("Submit")`)
  is left as it was.
- **The navigation bar's title is the screen's heading.** `NavigationBar` paints its title
  itself, so no text under it said it and the screen had no name in the tree; it now
  describes itself as a heading with the title, as `AppBar` already did for its own.
- Transparent wrappers forward `on_click`, `semantics` and `children` together, so a `Keyed`
  tappable container is one node, not two.

## Verification

On the real window (Windows 11, 150 %), the demo opened on a task's screen and its
accessibility tree listed through UI Automation:

| | what a screen reader could reach |
|---|---|
| before | `Button:Back`, `Button:✓` |
| after | `Text:Task` (the heading: UI Automation gives a heading the text control type, with a heading level), `Button:Back`, `Text:WT`, `Text:Water the plants`, `Text:Still to do`, `Text:Drop files here to attach them`, `Button:Delete`, `Button:Attach…`, `Button:Back`, `Button:✓` |

"Delete" was then **activated through UI Automation** — no pointer — and the task was deleted:
the screen read back "This task no longer exists."

## Left

- The tree is still **flat**: a tappable card's controls follow it rather than sitting inside
  it, so a reader cannot descend into the card as a unit.
- A tappable area that also has `on_long_press`, `on_double_tap` or `on_secondary_tap` exposes
  only the press; the reference adds those as custom actions.

## Mutation testing

Ten hand mutants; nine killed.

| mutant | killed by |
|---|---|
| a label node's words left as its label | `a_text_node_carries_its_words_as_its_value` |
| the live region's message left as its label | `announcement_adds_a_polite_live_region` |
| plain texts not taken as the area's words | `a_tappable_area_is_a_button_named_by_its_words`, `a_tappable_area_keeps_its_controls` |
| a wordless area around one control given a node anyway | `a_wordless_area_around_one_control_adds_nothing` |
| the area's controls dropped | `a_tappable_area_keeps_its_controls` |
| the area's node not clickable | `a_tappable_area_is_a_button_named_by_its_words` |
| a clickable entry taken as words | `a_clickable_entry_inside_an_area_is_not_taken_as_words` (added after it survived) |
| a self-described button's label not filled | `a_button_holding_a_text_is_named_by_it` |
| a self-described button not recognised as the area's own entry | the same |
| any widget with `on_click` treated as an area, named or not | **survived** (and survived in the rule's first form, which only took widgets with no self-description) |

The survivor: on a widget that already names itself, the change would take the words below it
into nothing. No widget in the crate is both named and holds words that are not hidden from the
tree (`Button::new` wraps its text in `ExcludeSemantics` for that reason), so no test can tell
the two apart today; it is recorded rather than covered by a widget made up for it.
