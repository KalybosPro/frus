# Jalon 166 — Table: adaptive row height

## Analysis

Every cell imposed a **fixed** height (`ROW_H = 34`). Since widget cells (milestone 164), a
cell can hold **taller** content (a large avatar, a bulky chip, a button, eventually multi-line
text) — which was then **cropped** to 34 px. Rows had to **grow with their content**, while
keeping a minimum **comfort** height for short rows.

## Technical decisions

- **A minimum constraint in layout.** Rather than a table-local hack, we add the missing,
  general primitive: `Style.min_width` / `Style.min_height` (translated to taffy's `min_size`),
  in the spirit of a constrained box. A box can now grow with its content **without ever
  collapsing** below a floor.

- **A cell = `Auto` height, a `ROW_H` floor.** The cell goes from `height: Length(ROW_H)` to
  `height: Auto` + `min_height: ROW_H`. Since the row aligns its cells with `Stretch` (the
  default), **they all follow the tallest**: tall content in a single cell stretches the whole
  row, with no cropping, and short rows keep their 34 px.

- **Painting centred on the real height.** The text, the sort triangle and the checkbox
  centred on the `ROW_H` constant; they now use `bounds.height` (the cell's **effective**
  height) — correct centring whatever the row.

## Implementation

- `frus-layout/style.rs`: the `min_width` / `min_height` fields (defaulting to `Auto`), taken
  into account in `to_taffy` (`min_size`), `layout_hash` and `Default`.
- `frus-widgets/flex.rs`: the `Style` literal completed (`..Default::default()`).
- `frus-widgets/table.rs`: `cell_style` (`Auto` height + `min_height: ROW_H`); `Cell` and
  `CheckCell` centre on `bounds.height`.
- `goldens.rs`: `table_adaptive_rows` (a 48 px large avatar vs a text row).

## Verification

- **Unit**: `widget_row_grows_to_tall_content` — a 60 px widget in a cell is painted at **its
  full height** (the row followed it), well beyond `ROW_H`.
- **Golden** `table_adaptive_rows` **inspected**: the large-avatar row grows (the "admin" chip
  centred in it), the "Bo/editor" text row keeps its height — no cropping. The text / 26 px
  avatar goldens **unchanged** (no regression).
- `cargo test --workspace` **green**.

## What's left

- **Resizing + tall rows**: the handle layer positions itself on `n × ROW_H` (exact for a text
  table — the only case where every column is fixed); a taller widget row would make it an
  underestimate. A niche combination, to be dealt with if needed (a height measured after
  layout).
- **Sorting widget columns**: the sort key is still supplied by the application (the table does
  not compare widgets) — already possible, to be documented in the guide.
