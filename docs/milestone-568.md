# Milestone 568 — The bar over a selection

Towards [#23](https://github.com/KalybosPro/frus/issues/23): copy and paste without a keyboard.
Milestone 511 gave a finger a selection — a hold selects the word, two handles move its ends —
and left the thing that acts on it. This is that: **Cut, Copy, Paste and Select all**, floating
over the selection, and on a desktop what a right-click opens.

## What it does

- **A hold opens the bar.** After the hold selects a word, and after a handle has been dragged
  and let go, the field's bar floats over the selection: **above** it, centred, a gap clear; or
  **below** it, clear of the handles that hang there, when there is no room above; held inside
  the window either way. It is put away by the next press elsewhere, by a key, and when the
  field stops being the focused one. While a handle is being dragged it is away, and comes back
  when the finger lifts.
- **A hold with no word to take still opens it** — an empty field, a hold past the end of the
  text — at the caret, with Paste and whatever else applies.
- **The list is decided by conditions.** Cut and Copy want a selection; Paste wants text on the
  clipboard and a field that can take it; Select all wants something left unselected. So a
  caret with an empty clipboard offers Select all alone, and an empty field with nothing to
  paste offers nothing and shows no bar. A **masked** field offers neither Cut nor Copy; a
  **read-only** one neither Cut nor Paste.
- **The application has the last word.** `TextField::selection_toolbar(|context, defaults| …)`
  is handed the [`ToolbarContext`] and the default list and returns the list to show: an item
  dropped, an item of its own added (`ToolbarItem::custom(label, message)`, which sends its
  message like any button and closes the bar), or an empty list for no bar at all.
- **Copy collapses the selection to its end**, as the platforms' own fields do, and puts the
  handles and the bar away. **Cut** copies and deletes — only if it copied. **Paste** types the
  clipboard's text and closes the bar. **Select all** leaves the bar open on the larger
  selection, with the list that state calls for.
- **A right-click on a text field opens the same bar** on a desktop: it focuses the field, puts
  the caret where the click landed unless it landed inside the selection there already is, and
  opens it. A right-click on anything else puts an open bar away.
- **Without a pointer**: the **context-menu key**, or **Shift+F10**, opens it on the focused
  text field. The bar's buttons are in the accessibility tree (a role, a label), and an
  assistive technology's click on one performs its action — on the field the bar is open on,
  which an AT's *focus* on the button does not take the focus from.
- **Back and Escape close it first**, before anything behind it: Back closes the bar, then
  the page. On Android the keyboard takes the first Back, so a bar over a field with the
  keyboard up needs two.

## Faults found on the way

- **Ctrl+C in a masked field copied the password**, and Ctrl+X copied it and deleted it.
  `selected_text` read the real value, not the dots. A masked field now gives its text to
  nobody: no Copy, no Cut, nothing on the clipboard. The fix is in the field, so the keyboard
  and the bar share it.
- **That fix would have made Cut destructive**: Cut was "copy, then delete", and with the copy
  refused it would have deleted the selection and put it nowhere. Cut now deletes only what it
  copied.
- And one that did no harm but would have: `Status` was fingerprinted for the paint cache
  without `handles`, so a cached subtree could replay a stale answer about them. Both flags are
  in the fingerprint now.

## How it is built

**A real overlay, not a paint in the field's pass.** The handles are painted inside the field's
own pass, which is why one overhanging the field is covered by whatever paints after it and cut
by whatever scrolls it. A bar needs neither: it goes through the overlay machinery that menus,
tooltips and dialogs already use, above everything and unclipped. The field asks the runtime
nothing; the shell **marks** the field as showing its bar (`Runtime::selection_toolbar`, with
whether the clipboard held text when it opened — asked once, on opening, not every frame), and
`Ui` floats the field's bar when it walks a marked, focused field.

**A new placement, `Placement::Selection`.** Every existing placement is against a widget's
box, and the box a bar is against is a *selection's*, which the field alone can say —
`Widget::selection_anchor`, the union of the selection's line boxes, or the caret's when nothing
is selected. The variant is a public addition to `Placement`. Its arithmetic is one pure function,
`selectiontoolbar::place`, that the tests pin: above by a gap; below past the handles; clamped to
the window's margins; pinned to the left margin when the window is narrower than the bar.

**The bar is a widget, and its buttons send no message.** Cut, Copy, Paste and Select all are
things the *shell* does — it holds the clipboard and the editing state — and a button that had
to send the application's `Msg` for them would make every application declare four messages
that mean the same everywhere. So a built-in button carries an `EditAction` (`Widget::edit_action`)
and is a message-less hit target (`opaque`); the shell resolves a press to it
(`Ui::edit_action_for`) and performs it on the field the bar is open on. Only an application's
own item carries a message. No button is focusable: a bar that took the focus would end the
selection it acts on, and a press on the bar (`Ui::toolbar_contains`) touches neither the focus,
the selection nor the handles.

**One bar per situation, built once.** The walk borrows the widget it floats for a whole frame,
and the situation changes between frames with nothing rebuilding the tree, so the field keeps
one bar for each of the eight combinations of the three conditions, built the first time it is
asked for. Each has an identity of its own (`WidgetId::toolbar`): layout caches are kept by
identity, and two lists under one would be measured as whichever came first.

**The three hooks are forwarded** through the transparent wrappers' macro, `Responsive` (by
hand) and `Box<dyn Widget>`, and a test holds all three to it — the place `Responsive` has cost
three silent bugs (milestones 477–495).

## Found by using it

The bar was then used the way the app is: on the phone, across pages, and in a browser through
the accessibility tree. It found four things no test had.

- **Back left the bar open.** With the bar up, the first Back closed the keyboard (Android's, as
  it should) and the second went to the *page*, leaving the bar over whatever was next. Back and
  Escape now close the bar first. Confirmed: wizard's password field, Back — keyboard away, bar
  still there; Back — bar and handles away, page kept; Back — the page.
- **An assistive technology could see the bar and not press it.** Its click resolved to a
  message, and a built-in button has none. It resolves to the button's action now, and an AT's
  *focus* on a button no longer takes the focus from the field (which would have closed the bar
  before the click).
- **On the web, an Enter on a button also reached the shell as a key.** The browser's own
  "activate" fires a `click`, and the same Enter went on to the shell's key handler, which read
  it as typing in the focused field: pressing *Select all* **submitted the text field**. The
  bridge's buttons now keep an Enter or a Space from propagating; `click` is the one way in.
  (`scripts/web-accessibility-check.py` had pressed a button with a key event that carries no
  character, which a browser does not activate a button with; it passed only because the shell
  also read the key. It now sends a real Enter.)
- **On the web, activating a bar button dropped the browser's focus on the page**, so the next
  key — an Escape — reached no one. The button's element is replaced when the bar's list
  changes, and the browser gives the focus to the document when the focused element leaves it.
  The bridge now puts the focus back on the shell's focused widget when it removes the element
  that had it.

## What it does not do

- **A field in a virtualised list has no bar.** A list item is built on the fly and cannot
  defer an overlay (it says so in `render_item`), and the bar is one. A form in an ordinary
  scroll area is fine.
- **The web cannot say whether the clipboard holds text** without asking, and asking is a
  promise that may prompt the reader; Paste is always offered there.
- **The handles are still painted in the field's pass**, with the limits milestone 511 wrote
  down; the bar no longer needs them to move, but they have not moved to the overlay.
- **The bar's buttons are not focus stops inside the application**: a keyboard user opens the
  bar and has Ctrl+C/X/V/A; on the web the buttons are real `<button>`s a screen reader or Tab
  reaches. **Shift+F10 was not seen to open it in the browser check** — a synthetic key
  carrying a modifier flag gives the page no Shift press to read, so `shift` stayed false —
  where the context-menu key did; the rule itself is a pure function with its own test.
- **No single handle under a caret**, no magnifier, no extending by words after the hold: still
  as 511 left them.
- **Right to left** was not looked at: the row of buttons flows as the layout does, and nothing
  was checked.

## Verification

- Pure and widget tests: the placement (above; below and clear of the handles; against both
  edges; a window narrower than the bar; below and still on the window); the eight contexts each
  have their own number; the default list for every condition, for a masked field and a
  read-only one; the application's list replacing, dropping, adding and emptying; an
  application item sending its message and a built-in one sending nothing; the anchor on the
  selection's box, on the caret, and agreeing with the handles.
- Through `Ui`: a marked, focused field floats one bar **above** its selection with the gap the
  placement says, and every action is reachable by pressing along it; below with no room above;
  none unless the field is both marked and focused; a press on the bar is known for one; the
  wrappers pass the hooks on.
- The shell: what a copy leaves behind, whichever way the selection was made.
- Two mutations, each failing the test meant for it: Paste offered to a read-only field; the
  bar shown on whichever field is focused rather than the one the shell marked. The second one
  **passed the first time round** — the two-field test focused the marked field, so the mark's
  identity was never the thing separating them — and the test now also focuses the other one.
- The routine battery, all green: `cargo fmt --all --check`; clippy `-D warnings` on the whole
  workspace, on `frus-demo --features shots`, and for `aarch64-linux-android` on the shell and
  the widgets (the Android-only lines are linted by nobody otherwise); rustdoc `-D warnings`
  over every feature; `cargo test --workspace` (1,733 tests in `frus-widgets`, 194 in the
  shell's library, the rest of the workspace and the doctests besides); the shell checked for
  `wasm32-unknown-unknown`; and `frus-hello` for Android at 11,343,144 bytes, 87% of the
  budget. Goldens are advisory and were not run — nothing here paints into a golden.
- **On a device**, the demo built from this branch (release, arm64) on a Huawei STK-L21,
  Android 10, SwiftKey, in the task list's *add* field, `Hello world` typed:
  - a hold on `Hello` selected it, put two handles under it and floated **Cut, Copy, Select all**
    above it — no Paste, the clipboard being empty;
  - **Copy** put the bar and the handles away and collapsed the selection to a caret after the
    word, and the keyboard's own clipboard strip then offered `Hello`: it was on the system
    clipboard;
  - a hold on `world` now floated **Cut, Copy, Paste, Select all**, and **Paste** replaced the
    word: `Hello Hello`;
  - a hold and **Select all** selected the whole text and left the bar open with **Cut, Copy,
    Paste** — no Select all, since everything was selected;
  - a handle **held down and dragged** left: the bar was **away** and the selection followed the
    handle; the finger lifted, the bar came back, with Select all again;
  - a press on the page **elsewhere** put the bar and handles away and unfocused the field;
  - a hold on `Hello` and **Cut** left `Hello`, caret at the start, bar closed.
  No panic in the log at any point. The press on the bar, which lives in the shell's event loop
  and no test reaches, is what this confirms.
- **Across pages, on the same device:** the bar open, a tap on the *Stats* tab — the page
  changes, the keyboard closes, no bar is left; back on *Tasks* the field keeps its text and no
  bar comes back; *Home* and back into the app restores the field, the selection, the handles,
  the bar and the keyboard as they were; the *About* page, the navigation drawer and Back out of
  it; the *Sign-up wizard* through its two steps. In its password field a hold selected the
  masked text and the bar offered **Paste** only — no Cut, no Copy, no Select all (everything
  was selected). Leaving the wizard made the phone's autofill service (*Password Vault*) offer
  to save the credentials — it had been handed the form, which is evidence for #45's open
  question, not the offer of a saved one.
- **In a real browser** (headless Edge over the DevTools protocol, the demo built for the web):
  `scripts/web-selection-bar-check.py` — the context-menu key opens the bar with **Paste and
  Select all**, activating *Select all* through the accessibility tree leaves the bar open
  offering **Cut, Copy, Paste**, the field is not submitted, and Escape closes the bar; and
  `scripts/web-accessibility-check.py`, still ten of ten, with a real Enter.
- **Not seen on a device:** the bar *below* the selection (nothing was near the top), an
  application's own item, a masked field (the demo has none in this screen), and the
  right-click, which is a desktop path and was not run on one.
