# Talking to a server

frus has one HTTP client for every platform: the browser's `fetch` on the web, a native client
on the desktop and on phones, behind the same API. It is the `net` feature, and its typed JSON
half is `json`:

```toml
[dependencies]
frus = { version = "0.5", features = ["net", "json"] }
serde = { version = "1", features = ["derive"] }
```

A request is awaited. The interface goes on drawing while it waits; see
[Work that waits](../getting-started.md#work-that-waits) for how `async` code runs in frus.

## One request

`fetch(url)` gets a page as text. `Request` is the full form: a method, headers, a timeout, a
body, and an answer decoded into your own types with `send_json`.

```rust
use std::time::Duration;

use frus::{FetchError, Request};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct Post {
    pub id: u32,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct NewPost {
    pub title: String,
    pub body: String,
}

const BASE: &str = "https://jsonplaceholder.typicode.com";

pub async fn posts() -> Result<Vec<Post>, FetchError> {
    Request::get(format!("{BASE}/posts"))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(10))
        .send_json()
        .await
}

pub async fn create(post: NewPost) -> Result<Post, FetchError> {
    Request::post(format!("{BASE}/posts"))
        .json_body(&post)
        .send_json()
        .await
}
```

A failure is a `FetchError`: `Network` (no connection, no answer in time), `Status(code)` (the
server said no), or `Decode` (the answer was not what the type expects). An application
usually turns it into its own error once, with a message for the person, rather than in every
screen — `frus-examples/http/src/common/api.rs` does exactly that.

## Data a screen needs to show itself

`use_future(key, make)` starts the work the first time the screen is built, and again whenever
`key` changes. Until it ends, the snapshot it returns has no data; then it has the result.

```rust
use frus::{
    component, CircularProgressIndicator, FetchError, ListTile, ListView, Request, Text, Widget,
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
struct Post {
    title: String,
}

async fn posts() -> Result<Vec<Post>, FetchError> {
    Request::get("https://jsonplaceholder.typicode.com/posts").send_json().await
}

fn post_list() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let snapshot = cx.use_future((), posts);
        match snapshot.data {
            None => Box::new(CircularProgressIndicator::new()),
            Some(Err(error)) => Box::new(Text::new(format!("Could not load the posts: {error}"))),
            Some(Ok(posts)) => Box::new(ListView::new(posts.len(), 56.0, move |i| {
                ListTile::new().title(posts[i].title.clone())
            })),
        }
    }))
}
```

To load again — a "Retry" button, a pull to refresh — make the key a counter and add one to it.
`FutureBuilder::new(key, make, build)` is the same thing as a widget, for a part of a screen
built from data of its own; `use_stream` and `StreamBuilder` follow a stream of values, such as
the messages of a WebSocket.

## Sending: an `async` handler

When the person starts the request — a button that sends a form — the handler is `async`. It
says what happens in order, in one place: lock the form, send, show the answer.

```rust
use frus::{
    component, Button, Column, FetchError, Request, Text, TextField, Widget,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct NewPost {
    title: String,
}

#[derive(Clone, Deserialize)]
struct Post {
    id: u32,
}

async fn create(post: NewPost) -> Result<Post, FetchError> {
    Request::post("https://jsonplaceholder.typicode.com/posts")
        .json_body(&post)
        .send_json()
        .await
}

fn new_post() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let title = cx.use_state(String::new);
        let sending = cx.use_state(|| false);
        let answer = cx.use_state(|| None::<String>);

        let (typed, busy, said) = (title.clone(), sending.clone(), answer.clone());
        let send = cx.async_callback({
            let title = title.clone();
            move || {
                let (title, busy, said) = (title.clone(), busy.clone(), said.clone());
                async move {
                    busy.set(true);
                    let result = create(NewPost { title: title.get() }).await;
                    busy.set(false);
                    said.set(Some(match result {
                        Ok(post) => format!("Created post {}", post.id),
                        Err(error) => format!("Not sent: {error}"),
                    }));
                }
            }
        });

        let mut button = Button::new(if sending.get() { "Sending…" } else { "Send" });
        if !sending.get() {
            button = button.on_press(send);
        }
        Box::new(
            Column::new()
                .spacing(12.0)
                .child(
                    TextField::new(title.get())
                        .label("Title")
                        .on_input(move |text: String| typed.set(text)),
                )
                .child(button)
                .child(Text::new(answer.get().unwrap_or_default())),
        )
    }))
}
```

The task belongs to the component: if the person leaves the screen while the request is on
its way, the task ends with it and the answer changes nothing. A `StatefulWidget` writes the
same thing with `cx.async_callback(|state| async move { … })` and changes its state with
`state.set_state(..)`; the getting-started guide shows both, and how to cancel a task that a
newer one replaces.

## Work that is not waiting

Some work does not wait for anything: it computes — resizing a picture, parsing a large file.
That work goes to another thread with `host::spawn(work, then)`, so that the interface does not
stall while it runs; `then` is called on the interface's thread with the result.
