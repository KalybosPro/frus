# Milestone 528 — A page of its own

The owner, on the phone: **"when I scroll another page, the one I just left scrolls too."**

## What was happening

Everything a widget keeps from one frame to the next — a scroll offset, a caret, a sheet's height,
a value on its way somewhere — is kept by the runtime under a `WidgetId`, and a `WidgetId` is a
hash of the widget's **position**: the index of each child on the way down from the root, unless a
`Keyed` on the way says otherwise. `Navigator` held its pages as plain children, `[screen]` at
rest and `[outgoing, incoming]` during a transition. Two things followed from that.

- **Two pages of the same shape were one page.** The page on show is always the first child,
  whichever page it is, so two pages built the same way have the same identities all the way
  down, their scroll regions included, and read one offset between them. Printing the regions of
  every demo page found two such families: the data table, the editable grid and the tour share a
  region, and so do the charts and the board. The table and the grid both scroll sideways — 246 px
  and 278 px on a 420 × 360 surface — so a table scrolled sideways opened the grid scrolled
  sideways too.
- **A page changed identity when it arrived.** During a transition the page arriving is the second
  child, and once the transition is over it is the first. So a page returned to by a pop was drawn
  with nothing retained for the whole slide — at the top of its list — and jumped to where it had
  been left in the frame the pop ended. A back gesture's preview is a pop held by the finger, and
  showed the same thing for as long as the finger was down.

## Reproduced first

Five tests were written against the old API and failed:

- Two pages of one shape, one scrolled to 400: the other was **drawn at 400**, where it had never
  been scrolled.
- A transition between two pages scrolled to 400 and 700: both settled pages had one region, so
  both were **drawn at 700**.
- The same route at two depths, the lower scrolled to 400: the upper was **drawn at 400**.
- On the demo, the table scrolled sideways, then back and into the grid: the grid's region
  **read the table's offset**.
- On the demo, settings scrolled to 300, then the journal pushed on top: the back gesture's preview
  and the pop's slide both drew settings from a region **with no offset at all**.

## What the reference does

Every route builds its page inside a subtree of its own: a `ModalRoute` keeps a `GlobalKey` for its
scope and for its subtree, and a `PageStorageBucket` of its own that the page's scrollables store
their offsets in (`routes.dart:2267`). A route's state belongs to the route, wherever its overlay
entry sits among the others, so neither of these failures can happen there.

## The decision

Three ways were weighed. **A fix in the demo alone** — wrap each screen in `keyed` — fixes this
application and leaves the next one to find the same bug on its own phone. **An optional key** on
`Navigator` leaves the default exactly what the bug is: every application that has not read the
note keys nothing, and it is the default everyone meets first. **The owner chose a mandatory key,**
and it is the safer of the three because a page's identity stops being something an application
can forget to give: the compiler asks for it at every call.

- **`Navigator::new(key, screen)` and `.from(key, previous, progress, forward)`**, each key any
  `Hash`. Each page is wrapped in a `Keyed`, so everything inside it takes its identity from the key
  and not from the index — through `child_id`, which is the one place every walk, lookup and
  animation derives a child's identity, so none of them needed touching. `Keyed` is transparent and
  forwards every hook a page needs, which the macro's own test enforces; the walk still reads the
  outgoing and incoming pages by index, and the index is still what orders them.
- **The key names an entry of the stack, not a route.** The same route pushed twice — a thread
  opened from a thread — is two pages, and a key made of the route alone would give them one offset
  again. `(depth, route)` is the recommended shape, and the documentation says so.
- **Two pages of one transition under one key** would be one identity drawn twice in a frame. A
  debug build refuses it with a message naming the shape to use.
- **A key a page's root declares is replaced** by the page's key, which the documentation says:
  something inside a page named for a request by key is keyed inside the page. No demo screen keys
  its root.
- The builders now ask `Msg: 'static`, as `OverflowBar`'s do: a page is boxed inside its key.

**What a page keeps while it is not in the tree** was read rather than assumed: nothing prunes the
runtime's scroll offsets, so a page below the top comes back under its key where it was left. What
it was animating is pruned and adopts its target when it is seen again, as a widget seen for the
first time does. Pruning was not changed here.

Every caller moved: the demo's `build_view` keys the page on show `(depth, route)`, the page a push
left `(depth - 1, route)`, the page a pop left `(depth + 1, route)`, and the back gesture's pair
`(depth, top)` over `(depth - 1, below)` — a gesture only starts on a stack with something below —
with `Hash` added to `Route`. The hero and navigator tests and the navigator golden's test moved
too. No example crate, guide or README builds a `Navigator`.

## Verification

**Tests.** The five reproductions above pass, and each checks what is **drawn** or what the frame's
own region reads, not a registry the walk might ignore. On the navigator: two pages of one shape
keep 400 and nothing; a transition between pages scrolled to 400 and 700 draws each page at its own
offset, on a push and on a pop, whichever of the two is arriving; the same route at depths 1 and 2
keeps 400 and nothing; and two pages under one key panic in a debug build. On the demo: the grid no
longer reads the table's offset; settings scrolled to 300 keep it while a push slides them out,
while a back gesture previews them, and while a pop slides them back in. The hero, overlay, clip
and parallax tests of the navigator run unchanged apart from their keys.

The whole widget crate's 1594 tests pass, the demo's 61, the shell's 106, the harness's suites
with the navigator golden unchanged, and the widget crate's 52 doctests, the new example on
`Navigator` included.

**Mutations** — six, all killed, each applied to a copy of the fixed tree and restored after:

- **The pages fall back to their index** (no key on either page, no assertion): four navigator tests
  and both demo tests fail.
- **The incoming page keyed with the outgoing page's key**, after the assertion: the transition test
  and the demo's pop test fail.
- **The same-key assertion removed**: its `should_panic` test fails.
- **The back gesture's page below keyed at the top's depth**: the demo's pop test fails, at the
  gesture.
- **The page a push left keyed as if a pop had left it**: the demo's pop test fails, at the push.
  That check was added for this mutation; the reproduction as first written did not look at a push.
- **The page on show keyed without its depth**: the demo's pop test fails.

**On the device** — the Huawei STK-L21, release builds of the demo, the same steps on a build of
the commit before this milestone and on this one. The data table opened from the drawer, scrolled
sideways to its Role, Score and Level columns, then back with the bar's arrow and the editable grid
opened: **before**, the grid opened scrolled sideways, its Name column out of sight and its Role
column cut to "ineer" and "ptographer"; **after**, it opened at its start, Name and Role in full.
And on this build, with the home page scrolled and the table scrolled sideways, a back gesture held
half-way showed the table sliding out still scrolled and the home page behind it at its own scroll,
where before it would have been drawn at the top.

**Found on the way, not caused here.** A back gesture started from the left edge at the height of a
task row of the page below (the home page's list) does not slide the page: the table stays put and a
row's green outline is drawn over it until the finger lifts. The build before this milestone does the
same (an empty outline there, the row's own content here, since the row now keeps its identity). A
gesture started at another height works. It belongs to the shell's gesture arbitration, and is left
for its own milestone.

## What is left

- **The back gesture over a row of the page below**, above: its own milestone.
- **A popped entry is never forgotten.** Nothing prunes the scroll offsets of a page that left the
  stack, so the same entry pushed again — the same depth and route — opens where the last one was
  left, where the reference starts a new route fresh. It was so before this milestone too, by
  position; a key makes it a decision to take rather than an accident.
