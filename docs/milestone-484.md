# Milestone 484 — Four thousand rows, ten of them built

Closes [#41](https://github.com/KalybosPro/frus/issues/41).

The issue asks for a widget that composes `DataTable` and `Pagination`. That one exists —
milestone 233 gave the table a page and milestone 236 the footer under it. What did not
exist is the part the issue is really about: **a table that does not hold its rows.**

`DataTable::new` takes every row. Showing ten of four thousand meant building four
thousand, sorting four thousand, and throwing away three thousand nine hundred and ninety
of them, on every frame.

## `DataTable::lazy`

`lazy(headers, count, row)`: how many rows there are, and how to build one. Only the page's
rows are ever asked for.

**What a lazy table gives up is the work it cannot do without the rows.** Sorting,
searching and filtering belong to whoever owns the data:

- `sorted` still draws the arrow and `on_sort` still fires, but the table does not reorder
  anything.
- `searchable` still draws the field and still emits; the application filters, which is
  what changes `count`.
- Every index — `selected`, `on_select_row`, the checkboxes — is an index **into the set as
  it stands**, the one `row` was asked for. In an ordinary table it is the index of the row
  as it was given, before sorting; here there is no *before*, because the table never saw
  the other rows.

That is the reference's `DataTableSource` split, and stating it is most of the work: a
lazy table that quietly sorted the ten rows it could see would be wrong on every page but
the first, and it would look right in a test.

## Composed when someone looks

Writing the first test found the fault the feature would have shipped with. A table is
built by a chain of eight or nine builder methods, and **each of them composed the whole
widget again** and threw the last one away. For a held table that is wasted work; for a
lazy one it is *four thousand rows built at `lazy(…)` and ten at `paginated(…)`* — the
exact cost the issue exists to remove, moved one line up.

So `inner` is a `OnceCell` now: a builder method throws the rendering away, and it is
composed once, when something finally asks. Every `DataTable` is cheaper for it, and the
test says so in the plainest way available — after the chain and before anyone looks, the
row builder has not been called at all.

## A page that no longer exists

The set shrinks under the reader all the time: type a letter into the search field on page
40 of 400 and there are two pages left. The answer is **the last page that does exist**,
and it is now written down rather than being what the clamp happened to do.

The alternatives are worse. An empty table with a pager under it says the filter matched
nothing, which is a lie. Jumping to page 1 loses the reader's place for a filter that
shortened the set by one row. The clamp keeps them as close to where they were as the set
allows — and the slice, the pager and the line all say the same clamped number, so nothing
on screen disagrees with anything else. The application's own `current` catches up on the
next click.

## The line under it

"11–20 of 4,000" now comes from the reader's language, and so do the numbers in it. Four
new entries in the table milestone 483 opened up:

- `page_range_label(start, end, total)` — a whole sentence, because the pieces are not in
  the same order in every language.
- `rows_per_page_label` — and the chooser is **named** in the footer now, where it used to
  be three bare numbers in a corner saying nothing about what they counted.
- `selected_row_count_label(count)` — takes the count, because French has to agree the
  participle with it where English does not.
- `group_digits` — a comma in English, a no-break space in French. Four thousand written
  `4000` is a number a reader has to count the digits of.

`French` answers all four, which the milestone-483 tripwire enforces.

## The pictures

`data_table_lazy_page`: page 2 of 400, four thousand rows, ten built. The footer is as much
the point as the rows — "11–20 of 4,000" with the thousand grouped, and "Rows per page"
beside the chooser.

`data_table_paginated` moved, for that name in the footer and nothing else. It was the one
picture that moved; the other eleven table goldens were checked and did not.
