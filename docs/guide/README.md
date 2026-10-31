# The frus guide

How to build an application with frus, one topic at a time. Every page is made of examples
you can paste: each one is a complete piece of Rust, and each is compiled — and the ones with
assertions run — by frus's own test suite, so none of them is out of date.

Start with [Starting a frus application](../getting-started.md): the smallest application,
the three ways to keep state, `async` work, and how to run on a phone. Then:

| Page | What it covers |
| --- | --- |
| [Laying out a screen](layout.md) | Rows and columns, sharing the room, boxes, stacks, wrapping, scrolling, adapting to the screen |
| [Text, colour and the theme](style.md) | Text, the theme's colours and type, light and dark, changing one widget or every widget of a kind |
| [Taking input](input.md) | Text fields, controllers, checkboxes, switches, sliders, dropdowns, checking a form |
| [Lists and grids](lists.md) | Long lists, the data behind them, pull to refresh, swipe to remove, grids, scrolling from code |
| [Screens, navigation and dialogs](navigation.md) | Routes, redirects, the `Scaffold`, dialogs and sheets |
| [Talking to a server](network.md) | Requests, JSON, loading a screen's data, sending a form |
| [Animation](animation.md) | Values that move to their new value, switching widgets, curves, what is animated for you |
| [Testing](testing.md) | Testing the state and the logic, and what the screen looks like |
| [Structuring an application](../app-structure.md) | Where each part of a growing application goes |

Beyond the guide:

- **The widget gallery** — `frus-examples/widgets` — shows every widget with the code that
  builds it: 173 demos, in a browser at <https://kalybospro.github.io/frus/>.
- **The example applications** — `frus-examples/` — are complete projects: a counter, forms,
  sign-in, navigation, a client for a REST server, a WebSocket chat.
- **The API reference** — <https://docs.rs/frus> — has every type and method, most with an
  example.
