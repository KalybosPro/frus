# Lists and grids

A list of a hundred thousand rows costs what the rows **on screen** cost: `ListView` builds
only those, as they scroll in. Give it how many rows there are, how tall each one is, and a
function that builds row `i`.

## A long list

```rust
use frus::{component, Icon, Icons, ListTile, ListView, Widget};

fn invoices() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let selected = cx.use_state(|| None::<usize>);
        let current = selected.get();
        let select = selected.clone();
        Box::new(ListView::new(10_000, 56.0, move |i| {
            let select = select.clone();
            ListTile::new()
                .leading(Icon::new(Icons::RECEIPT))
                .title(format!("Invoice #{}", i + 1))
                .subtitle(format!("{:.2} €", (i as f32 * 7.31) % 500.0))
                .selected(current == Some(i))
                .on_tap(move || select.set(Some(i)))
        }))
    }))
}
```

A list takes the room it is given: the whole page, or what a `Column` leaves it when it is
wrapped in `Expanded`. `.axis(Axis::Horizontal)` makes it run across, `.reverse()` start at the
end (a chat), `.padding(16.0)` keep its rows off the edges.

`ListTile` is the usual row: `leading`, `title`, `subtitle`, `trailing`, `on_tap`. Any widget
can be a row, as long as it is as tall as the list says. `CheckboxListTile`,
`SwitchListTile` and `RadioListTile` are a tile and a control in one.

### The data behind it

The rows usually come from a `Vec` the state holds. The builder is called with an index and
reads the item it needs:

```rust
use std::rc::Rc;

use frus::{component, ListTile, ListView, Widget};

#[derive(Clone)]
struct Contact {
    name: String,
    phone: String,
}

fn contacts(people: Vec<Contact>) -> Box<dyn Widget> {
    let people = Rc::new(people);
    Box::new(component(move |_cx| {
        let rows = people.clone();
        Box::new(ListView::new(people.len(), 64.0, move |i| {
            ListTile::new().title(rows[i].name.clone()).subtitle(rows[i].phone.clone())
        }))
    }))
}
```

## Rows of different heights

When the rows are not all the same height — posts of different lengths, messages with and
without a picture — `ListView::builder(count, build)` lays each one out at its own height. It
still builds only the rows in view: a row is measured the first time it scrolls in, and the
rows not seen yet are counted at the mean of those that were.

```rust
use std::rc::Rc;

use frus::{component, Column, ListView, Text, Widget};

struct Post {
    author: String,
    body: String,
}

fn feed(posts: Vec<Post>) -> Box<dyn Widget> {
    let posts = Rc::new(posts);
    Box::new(component(move |_cx| {
        let shown = posts.clone();
        Box::new(
            ListView::builder(posts.len(), move |i| {
                Column::new()
                    .spacing(4.0)
                    .child(Text::new(shown[i].author.clone()).size(13.0))
                    .child(Text::new(shown[i].body.clone()).wrap())
            })
            .estimated_extent(120.0)
            .padding(16.0),
        )
    }))
}
```

`.estimated_extent(..)` is the height to count a row at before any is measured; a guess close
to the real rows keeps the scrollbar steady. Each row that can move — be removed, reordered,
inserted above — is given a key with `keyed(id, row)`, so that its state stays with it rather
than with its position.

## Pull to refresh, swipe to remove

`RefreshIndicator` wraps a list and calls you when it is pulled down; it spins for as long as
you say it is refreshing:

```rust
use std::time::Duration;

use frus::{component, host, ListTile, ListView, RefreshIndicator, Widget};

fn headlines() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let refreshing = cx.use_state(|| false);
        let fetched = cx.use_state(|| 1u32);
        let (start, done, count) = (refreshing.clone(), refreshing.clone(), fetched.clone());
        let on_refresh = move || {
            start.set(true);
            let (done, count) = (done.clone(), count.clone());
            // Stands for the request that fetches the new rows.
            host::after(Duration::from_millis(1500), move || {
                count.update(|n| *n += 1);
                done.set(false);
            });
        };
        let shown = fetched.get();
        let list = ListView::new(20, 48.0, move |i| {
            ListTile::new().title(format!("Headline {} (fetch #{shown})", i + 1))
        });
        Box::new(
            RefreshIndicator::new(list)
                .on_refresh(on_refresh)
                .refreshing(refreshing.get()),
        )
    }))
}
```

`Dismissible` lets a row be swiped away, with what to show underneath while it moves and what
to do once it is gone:

```rust
use frus::{component, keyed, Align, Colors, Column, Container, Dismissible, ListTile, Text, Widget};

fn inbox() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let mail = cx.use_state(|| vec!["Lunch on Friday?", "Your invoice", "Weekly report"]);
        let rows = mail.get().into_iter().fold(
            Column::new().cross_axis_alignment(Align::Stretch),
            |column, subject| {
                let remove = mail.clone();
                let row = Dismissible::new(ListTile::new().title(subject))
                    .height(56.0)
                    .background(Container::new().color(Colors::RED).child(Text::new("Delete")))
                    .on_dismiss_to_start(move || remove.update(|m| m.retain(|s| *s != subject)));
                column.child(keyed(subject, row))
            },
        );
        Box::new(rows)
    }))
}
```

## Grids

`GridView::new(columns)` lays out the cells it is given; `GridView::extent(max, count, build)`
fits as many columns as it can of tiles no wider than `max`; `GridView::builder(columns, count,
build)` is the virtualised one, for thousands of tiles.

```rust
use frus::{Center, Color, Container, GridView, Text, Widget};

fn tile(i: usize) -> Container {
    Container::new()
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .child(Center::new(Text::new(format!("{}", i + 1))))
}

fn photos() -> Box<dyn Widget> {
    Box::new(GridView::extent(120.0, 60, tile).gap(6.0).aspect(1.0))
}

fn catalogue() -> Box<dyn Widget> {
    Box::new(GridView::builder(3, 5_000, tile).gap(4.0).tile_height(120.0))
}
```

## Moving a list from code

A list wrapped in `keyed(key, list)` can be scrolled from anywhere with `host::scroll_to`: to
its start, its end, or an offset. It animates unless told `.instant()`.

```rust
use frus::{host, keyed, Button, Column, Expanded, ListTile, ListView, ScrollTo, Widget};

const MESSAGES: &str = "messages";

fn conversation() -> Box<dyn Widget> {
    Box::new(
        Column::new()
            .child(Expanded::new(keyed(
                MESSAGES,
                ListView::new(500, 48.0, |i| ListTile::new().title(format!("Message {i}"))),
            )))
            .child(
                Button::new("Latest").on_press(|| host::scroll_to(MESSAGES, ScrollTo::end())),
            ),
    )
}
```

`.on_scroll(|position| ..)` tells you where the list is — to load the next page when the reader
nears the end, for instance: `position.offset.1 > position.max.1 - 400.0`.
