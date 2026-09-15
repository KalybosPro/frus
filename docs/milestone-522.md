# Milestone 522 — A bar with no title

`AppBar::new` took the title as a string. Everything since milestone 397 had already made the
title a widget underneath: the string was wrapped in a plain `Text` and put in a
`Box<dyn Widget>` field, and `.title(widget)` replaced it. What was left of the string was the
constructor's argument, and with it one thing the reference does not do: **a bar had to have a
title.**

That showed in two ways. A bar with nothing between its leading and its actions — a back button
and a Save — was written `AppBar::new("")`. And a bar whose title was not text, a logo or a
composed row, was written `AppBar::new("").title(logo)`, with a string thrown away. The demo's
own sheet screen was written that way, and so were five tests.

## What an empty title cost

Checked before anything was changed, on a bar built with `new("")`:

- **Nothing was announced.** The worry was an empty heading for a screen reader to land on; there
  was none — the heading wrapper merges its child's words, and an empty label is dropped. So the
  accessibility half of the question is answered *no*, and it is answered by looking.
- **An empty text was laid out and painted.** One text primitive, with no words, sitting in the
  title's place in the row, and counted by the fold as a child of the row like any title.

## What the reference does

Its `title` is a `Widget?` (`app_bar.dart:1067`). When it is null nothing is wrapped, no heading
is added, and the toolbar is handed a null middle, which builds nothing there. There is no string
form of the title at all.

## The decision

Two shapes were offered to the owner: keep `new(title)` and add an `AppBar::empty()` for a bar
without one — nothing breaks, two constructors — or do what the reference does, breaking every
call. **The owner chose the second.**

- **`AppBar::new()` takes nothing**, and `AppBar` implements `Default` as its sibling
  `BottomAppBar` does.
- **`.title(widget)` is the only way to give a bar a title.** No string convenience beside it:
  the reference has none, and `.title(Text::new("Inbox"))` is one call longer than it was.
- **Without a title the row holds no title child**, so nothing is laid out, painted or announced
  between the leading and the actions.
- **The fold charges nothing for a title that is not there.** The budget counts the row's
  children before the actions and the gaps joining them; the title was always one of them, and
  now it is one only when it exists. Its reserve is nought, and so is its floor.

Every caller moved: the demo's task list and sheet screens, the doc examples in `appbar.rs` and
`themebuilder.rs`, the scaffold's and the bar's tests, and the app bar golden.

## Verification

**Tests.** A bar with a back button, an action and no title draws exactly those two words,
announces no heading, keeps its action at the end and keeps its height. A bar sized to the pixel
for its margins and one action shows the action with no overflow button — charged a join for a
title it has not got, it would fold it away. The sweep that checks a bar's inline actions fit in
the bar at every width up to 1600 px now sweeps untitled bars too, flush and centred. Every
earlier bar test runs unchanged apart from its constructor.

The sweep starts each configuration at the narrowest bar it can be — below that every action is
folded and what is left is wider than the bar on its own. For a bar without a title that floor
is the titled one's less the title's 64 px floor and one 8 px gap: 199 px flush, 207 centred. The
first run started both at 200, and the centred one overflowed at 200 px by **7 px** — exactly the
distance to its floor, which is the budget's arithmetic confirmed to the pixel rather than a slip.

**Goldens.** Every golden passes unchanged, the app bar's included: its bar now says
`AppBar::new().title(…)` and draws the same pixels. The demo's tests pass with its task list and
sheet screens on the new constructor, and the whole widget crate's 1586 tests with every earlier
bar test on it.

**Mutations** — six, all killed: an untitled bar still building an empty text, a title set and
dropped, the budget counting a title always, the budget counting one never, an untitled bar
still reserving a title's floor, and the title no longer announced as a heading.

**On the device** — not run. Nothing a titled bar draws changed, which the golden checks to the
pixel, and no screen of the demo has a bar without a title.

## What is left

- The remaining `AppBar` properties the roadmap lists: `clipBehavior`, `scrolledUnderElevation`,
  `animateColor`, `systemOverlayStyle`, and a back button implied from the route.
