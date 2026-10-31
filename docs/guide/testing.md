# Testing

Most of what an application decides is plain Rust: what a button does to the state, whether a
form is valid, which page a link opens. Written as methods and functions, it is tested with
`cargo test` — no window, no GPU, no device. What the screen looks like is tested by
rendering it offscreen and comparing pixels.

## Tests live in `tests/`

A project made by `frus create` keeps its tests in a `tests/` folder beside `src/`, one file
per module it tests (`tests/counterstate_test.rs` for `src/…/counterstate.rs`). They use the
application as a library, the way any other crate would, so they test what is public.

```sh
cargo test
```

## The state: methods, and the tests of them

Keep what a handler does in a method of the state, and let the handler only call it. The
method is then tested on its own:

```rust
#[derive(Default)]
pub struct CartState {
    pub items: Vec<(String, u32)>,
}

impl CartState {
    pub fn add(&mut self, name: &str) {
        match self.items.iter_mut().find(|(n, _)| n == name) {
            Some((_, count)) => *count += 1,
            None => self.items.push((name.to_string(), 1)),
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.items.retain(|(n, _)| n != name);
    }

    pub fn total(&self) -> u32 {
        self.items.iter().map(|(_, count)| count).sum()
    }
}

// In the screen: `.on_press(cx.callback(|s: &mut CartState| s.add("tea")))`.

// tests/cartstate_test.rs
let mut cart = CartState::default();
cart.add("tea");
cart.add("tea");
cart.add("bread");
assert_eq!(cart.total(), 3);
cart.remove("tea");
assert_eq!(cart.items, vec![("bread".to_string(), 1)]);
```

The same goes for checking a form (`frus::form::Form` is a value: build it from the state and
ask it what is wrong), for the redirect of a router (write it as a function of the location
and test the function), and for turning a server's error into a message.

```rust
use frus::form::{Form, Rule};

fn sign_in(email: &str, password: &str) -> Form {
    Form::new()
        .field("email", email, Rule::email("Enter a valid email address"))
        .field("password", password, Rule::min_len(8, "At least 8 characters"))
}

fn redirect(signed_in: bool, location: &str) -> Option<String> {
    (!signed_in && location != "/login").then(|| "/login".to_string())
}

assert!(sign_in("ada@example.com", "correct horse").is_valid());
assert_eq!(sign_in("ada", "short").first_invalid(), Some("email"));
assert_eq!(redirect(false, "/orders"), Some("/login".to_string()));
assert_eq!(redirect(true, "/orders"), None);
```

## Work that waits

An `async` function that calls a server is tested in two halves. What it sends and how it reads
the answer are plain functions — the address it builds, the JSON it decodes from a sample
answer — and need no network. The call itself is left to the application's own runs, or to a
test that the CI is allowed to make over the network.

## A page, used as a person uses it: `Tester`

`frus::Tester` runs the application with no window — no GPU, no device — and drives it the
way a person does: it taps what reads "Add", types into the field labelled "Name", waits, and
reads what the screen says. It is frus's `testing` feature, for the tests only:

```toml
[dev-dependencies]
frus = { version = "0.5", features = ["testing"] }
```

```rust
use frus::{component, Button, Column, FrusApp, Tester, Text, TextField, Widget};

fn greeter() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let name = cx.use_state(String::new);
        let greeted = cx.use_state(|| false);
        let (typed, greet) = (name.clone(), greeted.clone());
        Box::new(
            Column::new()
                .child(TextField::new(name.get()).label("Name").on_input(move |t: String| typed.set(t)))
                .child(Button::new("Greet").on_press(move || greet.set(true)))
                .child(Text::new(if greeted.get() { format!("Hello, {}!", name.get()) } else { String::new() })),
        )
    }))
}

// tests/greeter_test.rs
let mut tester = Tester::new(FrusApp::from_fn(|_| greeter()));
tester.enter_text("Name", "Ada").tap("Greet");
assert!(tester.shows("Hello, Ada!"));
```

Every action draws frames until the screen is still: a page transition has ended, an animation
has run. `wait(seconds)` lets time pass — a snack bar's four seconds, a timer —, `back()` is
the system's back button, and `texts()` lists what the screen shows, which is what a failing
assertion should print. A tap on something that is not on screen fails the test and says what
was there.

## What the screen looks like: `frus-test`

`frus-test` renders a widget offscreen, through the same pipeline as a window, and gives back
its pixels. Add it for the tests only:

```toml
[dev-dependencies]
frus-test = "0.5"
```

```rust
use frus::{Color, Container, Theme};
use frus_test::render_widget;

let badge: Container = Container::new().width(40.0).height(40.0).color(Color::hex("#DC2626"));
// `None` where the machine has no graphics adapter: the test then has nothing to check.
if let Some(shot) = render_widget(&badge, 40, 40, &Theme::light()) {
    let [r, g, b, _] = shot.pixel(20, 20);
    assert!(r > 200 && g < 60 && b < 60, "the middle of the badge is red");
}
```

`shot.assert_golden("tests/goldens/badge.png")` compares the whole picture with a reference
image kept in the repository. The first run writes the reference — look at it, then commit it;
`FRUS_UPDATE_GOLDENS=1 cargo test` writes it again after a change you meant. Text is drawn a
little differently by different graphics drivers, so goldens are made and compared on the same
kind of machine (frus makes its own with a software renderer).
