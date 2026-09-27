# Milestone 590 — Viewports and fields take the room they are given

An application's login page was a scaffold whose body was a `SingleChildScrollView` holding a
column of text fields and a button. It filled a third of the screen. The scroll view was a 200-px
window, its width was the width of its content, and each field was 220 px wide. None of these
sizes were asked for. They were defaults, and the reference has no such defaults.

## How the reference sizes these

- **The scaffold** gives its body loose constraints: at most the full width, and at most the
  height between the app bar and the bottom bars.
- **A single-child scroll view** is `constraints.constrain(child.size)`. Along its axis it is as
  big as its child, up to the room, and it scrolls past that. Across its axis it passes its
  constraints on, so a column in it is as wide as the room when its children ask for that.
- **A list, a page view, a wheel and an interactive viewer** take the largest size offered. With
  nothing offered, they stop with an error.
- **A text field and a linear progress bar** take the full width offered.

## What changed

- The scaffold's body pane is an `Expanded` column, so its basis is nothing. It used to be a
  column that grew from the size of its content, and a body taller than the screen made the
  pane taller than the screen too. A scroll view inside it then had no bound to scroll within.
- `SingleChildScrollView::new()` is unsized. The layout measures it along the axis it scrolls:
  its content, clamped to the room on offer. Across that axis it asks to fill, through
  `fill_axes`. `.flex()`, `.width()` and `.height()` still override both.
- A new hook, **`Widget::soft_extent`**, names the axes on which a widget's size is only a
  default. When the widget asks to fill that axis and the layout can grant it, the size is
  cleared and the room replaces it. The layout can grant it when the widget is alone in a box,
  or on the cross axis of a row or a column. Along a line shared with other children the
  default stands. That is where the reference would have no size to give. The default keeps
  something on the screen instead of an error.
- `ListView`, `PageView`, `ListWheelScrollView` and `InteractiveViewer` fill both unsized axes.
  Their default heights (200, 180 and 300 px) are soft. `TextField` (220 px) and
  `LinearProgressIndicator` (200 px) fill the width, and their default widths are soft.
- Transparent wrappers forward `soft_extent`.

## Alternatives weighed

- **Dropping the defaults altogether.** A list in a column with a header would then be zero
  pixels tall, where the reference fails loudly. A soft default is the friendlier failure, and
  it gives way wherever the reference has an answer.
- **Filling along the shared line with `flex`.** That would give a list beside a header the
  rest of the column, which the reference does only when asked, with `Expanded`.

## Verification

- A 360 × 420 page with an app bar and a scroll view of six fields: the viewport is 360 wide and
  reaches the bottom of the page, below the bar, and the widest field is 360 px.
- The same page with one field: the viewport is as short as its content.
- An unsized list as the body: 360 × 420. The same list below a header in a column: 360 wide
  and 200 tall.
- A scroll view of one field in a `Center`: as wide as the page, as the field is, and as tall
  as the field.
- An unsized interactive viewer in a plain `Container` as the body: the whole body.
- The `search_bars` golden now shows the bars across the whole column, as the reference's
  search bar does. It was re-recorded.
- Mutations, each failing a test:
  - the soft size never cleared;
  - the vertical soft flag dropped;
  - the scroll view's clamp removed;
  - no clamp along the axis;
  - the body pane growing from its content;
  - the text field not filling;
  - the list not filling;
  - the scroll view not filling across.
- A mutation that also allowed a default size to pass a request up through a box survived. No
  widget with a soft default is laid out as a box: they are all measured leaves. The clause
  was removed.
