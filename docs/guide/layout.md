# Laying out a screen

A screen is a tree of widgets. Most of it is made of five things: a **row** and a
**column** to put children in a line, a **container** to give one child a size, a colour and
some room, **expanded** to share what is left, and a **stack** to put things on top of each
other. Every example on this page is a complete function you can paste into an application.

## Rows and columns

`Row` lays its children out across, `Column` down. `spacing` puts the same gap between every
pair of children; `main_axis_alignment` says where the free room goes along the line, and
`cross_axis_alignment` lines the children up across it.

```rust
use frus::{Align, Column, Icon, Icons, Justify, Row, Text, Widget};

fn profile_header() -> Box<dyn Widget> {
    Box::new(
        Row::new()
            .spacing(12.0)
            .child(Icon::new(Icons::PERSON).size(40.0))
            .child(
                Column::new()
                    .spacing(2.0)
                    .cross_axis_alignment(Align::Start)
                    .child(Text::new("Ada Lovelace").size(18.0))
                    .child(Text::new("Analyst").size(13.0)),
            ),
    )
}

fn toolbar() -> Box<dyn Widget> {
    Box::new(
        Row::new()
            .main_axis_alignment(Justify::SpaceBetween)
            .child(Text::new("Inbox"))
            .child(Text::new("12 unread")),
    )
}
```

The `row!` and `column!` macros are the short form when the children are known in advance:

```rust
use frus::{button, column, row, text, Widget};

fn short_form() -> Box<dyn Widget> {
    Box::new(column![
        text("Two buttons below"),
        row![button("Cancel", || {}), button("OK", || {})],
    ])
}
```

## Sharing the room: `Expanded`, `Flexible`, `Spacer`

A child wrapped in `Expanded` takes the room the others leave; several share it by their
`flex` factor. `Flexible` may take that room but keeps its own size when it needs less — what
long text wants, so that it wraps instead of running off the edge. `Spacer` is an empty
`Expanded`: it pushes what follows to the far end.

```rust
use frus::{Expanded, Flexible, Icon, Icons, Row, Spacer, Text, TextField, Widget};

fn search_row() -> Box<dyn Widget> {
    Box::new(
        Row::new()
            .spacing(8.0)
            .child(Icon::new(Icons::SEARCH))
            .child(Expanded::new(TextField::new("").placeholder("Search")))
            .child(Icon::new(Icons::MIC)),
    )
}

fn message_row() -> Box<dyn Widget> {
    Box::new(
        Row::new()
            .spacing(8.0)
            .child(Icon::new(Icons::EMAIL))
            .child(Flexible::new(
                Text::new("A subject long enough to need a second line on a phone").wrap(),
            ))
            .child(Spacer::new())
            .child(Text::new("09:41")),
    )
}
```

## One box: `Container`

`Container` is the all-purpose box: a size, padding inside, margin outside, a colour, rounded
corners, a border, a shadow, and one child.

```rust
use frus::{BoxShadow, Color, Colors, Container, Text, Widget};

fn badge() -> Box<dyn Widget> {
    Box::new(
        Container::new()
            .padding(16.0)
            .margin(8.0)
            .radius(12.0)
            .color(Colors::WHITE)
            .border(1.0, Color::hex("#E4E4E7"))
            .shadows([BoxShadow::new(0.0, 4.0, 12.0, Color::rgba(0.0, 0.0, 0.0, 0.15))])
            .child(Text::new("A card-like box")),
    )
}
```

When you only need one of these, the single-purpose widgets say so more plainly:
`Padding::new(Insets::uniform(16.0), child)`, `SizedBox::empty().height(24.0)`, `Center::new(child)`,
`Align::new(Alignment::BOTTOM_RIGHT, child)`, `AspectRatio::new(16.0 / 9.0, child)`.

```rust
use frus::{Center, Insets, Padding, SizedBox, Text, Widget};

fn centred_message() -> Box<dyn Widget> {
    Box::new(Center::new(Padding::new(
        Insets::uniform(24.0),
        Text::new("Nothing here yet"),
    )))
}

fn gap() -> Box<dyn Widget> {
    Box::new(SizedBox::empty().height(24.0))
}
```

## On top of each other: `Stack`

A `Stack` paints its layers in order, the last on top. `Positioned` pins a layer to the
stack's edges; `Positioned::fill` covers it.

```rust
use frus::{Color, Colors, Container, Positioned, Stack, Text, Widget};

fn photo_with_caption() -> Box<dyn Widget> {
    Box::new(
        Stack::new()
            .layer(Container::new().width(320.0).height(180.0).color(Color::hex("#6366F1")))
            .layer(
                Positioned::new(
                    Container::new()
                        .padding(8.0)
                        .color(Color::rgba(0.0, 0.0, 0.0, 0.5))
                        .child(Text::new("Lisbon, at dusk").color(Colors::WHITE)),
                )
                .left(0.0)
                .right(0.0)
                .bottom(0.0),
            ),
    )
}
```

## When it does not fit: `Wrap` and scrolling

`Wrap` starts a new line when the next child does not fit — tags, chips, filters. A page taller
than the screen goes in a `SingleChildScrollView`.

```rust
use frus::{Chip, Column, SingleChildScrollView, Text, Widget, Wrap};

fn tags() -> Box<dyn Widget> {
    let mut wrap = Wrap::new().gap(8.0).run_gap(8.0);
    for tag in ["rust", "ui", "android", "web", "desktop", "accessibility"] {
        wrap = wrap.child(Chip::new(tag));
    }
    Box::new(wrap)
}

fn long_page() -> Box<dyn Widget> {
    let mut column = Column::new().spacing(12.0);
    for i in 1..=50 {
        column = column.child(Text::new(format!("Paragraph {i}")));
    }
    Box::new(SingleChildScrollView::new().child(column))
}
```

A long list of similar rows is not a column in a scroll view: it is a `ListView`, which builds
only the rows on screen. See [Lists](lists.md).

## Adapting to the screen

`MediaQuery::of()` says how big the window is, how dense its pixels are, and where the system's
bars and notches sit. A layout that changes with the width reads it while it builds:

```rust
use frus::{component, Column, MediaQuery, Row, Text, Widget};

fn adaptive() -> Box<dyn Widget> {
    Box::new(component(|_cx| {
        let wide = MediaQuery::of().size.width >= 600.0;
        let list = Text::new("The list");
        let detail = Text::new("The detail");
        if wide {
            Box::new(Row::new().spacing(24.0).child(list).child(detail))
        } else {
            Box::new(Column::new().spacing(12.0).child(list).child(detail))
        }
    }))
}
```

`SafeArea::new(child)` keeps content clear of the status bar, the notch and the gesture bar;
a `Scaffold` (see [Navigation](navigation.md)) does it for you.
