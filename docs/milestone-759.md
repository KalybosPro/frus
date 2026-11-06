# Milestone 759 — Placeholders and grids take the width they are offered; an autocomplete's field can be dressed

## Objective

Seen on the phone (Huawei STK-L21, Android 10, release build of `frus-demo`), Settings → About:

1. The two loading placeholders (`Skeleton`) under the pagination were blank space: no bar, no
   shimmer.
2. The tags field beside "Info" was an empty box.
3. The three statistic tiles ("Total", "Active", "Done") huddled at the left, each as narrow as
   its number, instead of sharing the tab's width.

For each, the question was whether the widget or the demo was at fault, measured against what
the leading toolkit's equivalent widgets do.

## Root causes

### 1 and 3: two widgets that size to nothing they can measure

The same family as milestone 758's table. Both widgets sit in
`ConstrainedBox::new(..).max_width(480)`, a box that caps the width but does not size its
child, inside a column that does not stretch it. A widget there is as wide as its content says.

- **`Skeleton`** has a height, no width unless one is given, and no children: it paints its bar
  into whatever box it gets. Left to its content, that box was zero pixels wide.
- **`GridView`** divides its width into equal `1fr` columns, but nothing gave it a width: it was
  as wide as its tiles' content, so each column shrank to its widest label.

Through `Application::view` at a phone's logical size (411 x 869), before the fix:

```
placeholders: Rect { x: 20.0, y: 608.0, width: 0.0, height: 16.0 }
              Rect { x: 20.0, y: 632.0, width: 0.0, height: 14.0 }
```

In the leading toolkit, neither case comes out this way. A placeholder bar is an empty box of a
given height, and an empty box with no width expands to the width its parent allows. A grid is
a scroll view, and a scroll view takes the full extent across its axis; its tiles then divide
that extent. Both widgets here now ask for the width the same way: `Widget::fill_axes` answers
`FillAxes::WIDTH` when no width was given, the request milestone 758's table makes. The layout
walk grants it as it grants any other: it grows a lone child of a box and stretches the widget
across a column. Along a row the request is not granted (the row divides its width among its
children), which leaves a placeholder or a grid in a row as it was.

### 2: a field with nothing to say

The tags field is an `Autocomplete` with an empty value. The leading toolkit's autocomplete,
left to its defaults, also shows a bare field: that part is the demo's. But there the
application can dress the field (a field-view builder: label, hint, border, icons), and here it
could not: `Autocomplete` built its `TextField` itself and offered only a width. A widget whose
field cannot be given a label breaks the rule that every widget's parts can be styled.

## What was done

- **`Skeleton` asks for the width on offer when it was given none** (`skeleton.rs`). A
  placeholder with a `width` keeps it.
- **`GridView` asks for the width on offer when it was given none** (`grid.rs`), for all three
  kinds (`new`, `extent`, `builder`). A grid with a `width` keeps it.
- **`Autocomplete::field`** (`autocomplete.rs`): a function handed the field the autocomplete
  built (value, width and typing already wired) that returns it dressed, e.g.
  `.field(|field| field.label("Tags").placeholder("..."))`.
- **The demo labels its tags field** "Tags", with a hint of what to type.

## How it was tested

- `skeleton::tests::a_placeholder_takes_the_width_it_is_offered`: a `Skeleton` in a box capped
  at 300, in a start-aligned column, must draw a 300 x 16 bar. **Fails before the fix**.
- `grid::primary_tests::a_grid_takes_the_width_it_is_offered`: a three-column grid of one-letter cells
  in the same shape must start its columns 103.3 px apart ((300 - 2 x 10) / 3 + 10). **Fails
  before the fix**.
- `a_placeholder_given_a_width_keeps_it`, `a_grid_given_a_width_keeps_it`: the request is not
  made when a width was given.
- `autocomplete::tests::the_field_can_be_dressed`: the label given through `field` is painted,
  and the field is still a field.
- `tests::the_about_showcases_take_the_tabs_width_on_a_phone` (`frus-demo`): the demo itself,
  through `Application::view` at a phone's size, Settings → About. Both placeholders must be
  more than 300 px wide, the "Done" tile must start more than 220 px after the "Total" one, and
  the tags field must read "Tags". **Fails before the fix** (the placeholders were 0 px wide).
- `cargo test -p frus-widgets -p frus-demo --lib` after merging master (milestone 758's table
  fix included): 2139 + 88 passed. `cargo clippy --workspace --all-targets -D warnings` clean,
  `cargo fmt --check` clean.
- Goldens (`frus-test`, under WSL, `CARGO_INCREMENTAL=0`): all 237 tests passed with no image
  changed. No golden scene puts a width-less placeholder or grid where its parent does not
  already size it.
- On the phone (Huawei STK-L21, Android 10, 1080x2340 at density 408): a release APK of the
  demo built from this branch, Settings → About. The two placeholders are grey bars running
  the width of the tab, the three statistic tiles share the tab's width in equal thirds, and the
  tags field reads "Tags". The table from milestone 758 still shows its two columns apart.

## Not done

- **The example projects and the gallery were not rebuilt.** They live outside the workspace;
  the API change is an addition (`Autocomplete::field`), but a grid or a placeholder in them
  with no width now takes the width offered, which was not looked at screen by screen.

- **The autocomplete's field keeps a fixed width.** It is 260 px unless `width` says otherwise;
  the leading toolkit's field takes the width its parent offers. Changing the default would
  move every autocomplete already laid out around 260 px, and was left for its own step.
- **A placeholder or a grid along a row** still gets no width from the request, since a row
  divides its width among its children. Placed there without a width or a `flex`, they stay as
  wide as before (zero for a placeholder). The leading toolkit would report an unbounded width
  in the same place; nothing here warns about it.
