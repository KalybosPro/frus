# frus examples

Complete frus applications, each a standalone Cargo project you can copy, and a gallery of every
widget. Read them in this order:

| Project | What it teaches |
|---|---|
| [`hello_world`](hello_world) | the smallest application: one entry point, one `StatelessWidget` page |
| [`counter`](counter) | state: a `StatefulWidget`, its `State`, handlers, a timer, unit tests |
| [`widgets`](widgets) | **the widget gallery**: every widget with a live example and its code |
| [`forms`](forms) | text fields, choices, pickers, validation and submission |
| [`navigation`](navigation) | routes, parameters, query strings, nested pages, guards, navigation bars |
| [`http`](http) | calling a JSON API: loading, errors, retries, posting data |
| [`websocket`](websocket) | a long-lived connection: messages over time, sending, reconnecting |
| [`authentication`](authentication) | sign-in, a shared auth service, route guards, sessions |
| [`forge`](forge) | a larger real application (front end and back end) |

## One layout for every project

Every example is organised the same way, so what you learn in one carries to the next:

```
<project>/
  Cargo.toml            one dependency: frus
  res/                  Android resources (launch theme, launcher icon)
  src/
    lib.rs              the module list and frus::main!(..)
    bin/<project>.rs    the desktop binary
    common/             routes, the router, colours, shared widgets, services
    features/<feature>/
      pages/            one page per file
      states/           what a page remembers, and the methods that change it
      controllers/      logic that is not a widget: validation, API calls
      widgets/          pieces of a page, pulled out
    models/             plain data types
  tests/                the tests, one file per module tested (`counterstate_test.rs`)
```

The tests live in `tests/`, beside `src/`, apart from the code they test. Each file there is built against the project as a library, so it tests what `src/` makes public.

## Running

From any project folder:

```sh
frus run                        # desktop (Windows, macOS, Linux)
frus run -d android             # Android: the connected phone, or an emulator
frus run -d web                 # in the browser
frus test                       # the tests in tests/, without a window
```

`frus` comes from `cargo install frus-cli`, and installs what a platform needs the first time
(the Android SDK, a Rust target, wasm-bindgen). Every project has a `web/index.html`, and its
README's "Web" section also gives the commands by hand (build for `wasm32-unknown-unknown`, run
`wasm-bindgen`, serve `web/`).

The projects depend on the frus checkout they sit in (`frus = { path = "../../crates/frus" }`).
Copied elsewhere, depend on the published crate instead: `frus = "0.3"`.

To share one build cache between all the examples (frus is then compiled once, not once per
project), point them at the same target directory:

```sh
CARGO_TARGET_DIR=../../target frus run
```
