# Screens, navigation and dialogs

An application of more than one screen is a **router**: each screen is a route, named by a
path, and the router keeps the stack of screens the person has opened. The back button, the
back gesture, the page transitions and, on the web, the browser's address bar all follow it.

## Routes

```rust
use frus::{
    component, BuildContext, Button, Column, FrusApp, GoRoute, GoRouter, Text, Widget,
};

fn home(cx: &BuildContext) -> Box<dyn Widget> {
    let router = cx.router();
    let settings = router.clone();
    Box::new(
        Column::new()
            .spacing(12.0)
            .child(Text::new("Home"))
            .child(Button::new("Open user 42").on_press(move || router.go("/users/42")))
            .child(Button::new("Settings").on_press(move || settings.push("/settings"))),
    )
}

fn user(id: String) -> Box<dyn Widget> {
    Box::new(Text::new(format!("User {id}")))
}

fn app() -> FrusApp {
    let router = GoRouter::new(vec![
        GoRoute::new("/", |_, _| component(home)).routes(vec![
            GoRoute::new("users/:id", |_, state| {
                user(state.param("id").unwrap_or_default().to_string())
            })
            .name("user"),
        ]),
        GoRoute::new("/settings", |_, _| Text::new("Settings")),
    ]);
    FrusApp::router(router).title("People")
}
```

`frus::main!(app())` starts it. From any screen, `cx.router()` gives the router:

| Call | What it does |
| --- | --- |
| `go("/users/42")` | Shows that location, with the routes it is made of underneath (here, home) so there is a page to go back to. |
| `push("/settings")` | Puts a screen on top of the current one. |
| `pop()` | Goes back one screen. |
| `replace("/login")` | Swaps the top screen for another. |
| `go_named("user", &[("id", "42")], &[])` | Goes by the route's name, so the paths can change without breaking the callers. |

A screen reads what its location said from the `GoRouterState` its builder receives:
`param("id")` for `:id`, `query("tab")` for `?tab=`, `extra::<T>()` for a value passed along
with the navigation.

## Keeping someone out: redirects

A redirect is asked before every navigation and can send it elsewhere — to the sign-in screen
while nobody is signed in:

```rust
use std::cell::Cell;
use std::rc::Rc;

use frus::{GoRoute, GoRouter, Text};

fn guarded(signed_in: Rc<Cell<bool>>) -> GoRouter {
    GoRouter::new(vec![
        GoRoute::new("/", |_, _| Text::new("Your account")),
        GoRoute::new("/login", |_, _| Text::new("Sign in")),
    ])
    .redirect(move |state| {
        let to_login = !signed_in.get() && state.location() != "/login";
        to_login.then(|| "/login".to_string())
    })
}
```

`.refresh_listenable(&auth)` asks the redirect again whenever `auth` changes — so that signing
out sends the person back to the sign-in screen at once.

## The shape of a screen: `Scaffold`

`Scaffold` puts the usual parts of a screen where people expect them: an app bar at the top,
the body, a floating action button, a navigation bar or a drawer. It also keeps the body clear
of the status bar, the notch and the keyboard.

```rust
use frus::{
    component, AppBar, Center, FloatingActionButton, IconButton, Icons, Scaffold, Text, Widget,
};

fn notes() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let count = cx.use_state(|| 0u32);
        let tab = cx.use_state(|| 0usize);
        let (add, pick) = (count.clone(), tab.clone());
        Box::new(
            Scaffold::new()
                .app_bar(
                    AppBar::new()
                        .title(Text::new("Notes"))
                        .action(IconButton::new(Icons::SEARCH).on_press(|| {})),
                )
                .body(Center::new(Text::new(format!("{} notes", count.get()))))
                .fab(FloatingActionButton::new(Icons::ADD).on_press(move || add.update(|n| *n += 1)))
                .nav(tab.get(), move |i: usize| pick.set(i))
                .destination(Icons::NOTE, "Notes")
                .destination(Icons::ALARM, "Reminders"),
        )
    }))
}
```

A screen the router pushed over another gets the way back on its own: its app bar shows a
back arrow (a chevron on Apple's systems) that pops the router, unless the screen has a drawer
or gives the bar a `leading` of its own. The system's back button and the back gesture pop it
too.

## A message at the bottom: snack bars

`host::show_snack_bar` shows a short message at the bottom of the window, over whatever screen
is showing, for four seconds. Several wait their turn. An action runs and hides the bar:

```rust
use frus::{host, Button, SnackBar, Widget};

fn archive() -> Box<dyn Widget> {
    Box::new(Button::new("Archive").on_press(|| {
        // … archive the message …
        host::show_snack_bar(SnackBar::new("Archived").action("Undo", (|| {
            // … put it back …
        }).into()));
    }))
}
```

`host::show_snack_bar_for(bar, duration)` chooses how long; `host::hide_current_snack_bar()`
hides it from code, and `.close_icon(host::hide_current_snack_bar)` gives it a cross.

## Dialogs and sheets

A dialog is part of the screen's state: it is open while a field of the state says so. The
dialog's buttons change that field, and the screen it covers is its `body`:

```rust
use frus::{
    AlertDialog, Button, State, StateContext, StatefulWidget, Text, Variant, Widget,
};

struct Draft;

#[derive(Default)]
struct DraftState {
    asking: bool,
    discarded: bool,
}

impl StatefulWidget for Draft {
    type State = DraftState;
    fn create_state(&self) -> DraftState {
        DraftState::default()
    }
}

impl State for DraftState {
    type Widget = Draft;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let screen = Button::new(if self.discarded { "Discarded" } else { "Discard draft" })
            .on_press(cx.callback(|s: &mut Self| s.asking = true));
        Box::new(
            AlertDialog::new(self.asking)
                .title("Discard draft?")
                .content("Your message has not been sent.")
                .action(
                    Button::new("Keep editing")
                        .variant(Variant::Text)
                        .on_press(cx.callback(|s: &mut Self| s.asking = false)),
                )
                .action(Button::new("Discard").variant(Variant::Text).on_press(cx.callback(
                    |s: &mut Self| {
                        s.discarded = true;
                        s.asking = false;
                    },
                )))
                .on_dismiss(cx.callback(|s: &mut Self| s.asking = false))
                .body(screen),
        )
    }
}

```

Because it is state, a dialog survives a rotation, can be opened from a test by setting the
field, and is never left open by a screen that has gone. `BottomSheet::new(open)` is the same
shape for a sheet that rises from the bottom (`.sheet(content)`, `.body(screen)`,
`.on_dismiss(..)`), and so are `SimpleDialog`, `Dialog`, `MenuAnchor` and `PopupMenuButton`.

## Tabs and drawers

`TabBar` with a `TabBarView`, `NavigationBar`, `NavigationRail` and `NavigationDrawer` switch
between sections of one screen; the selected index is the state's, like any other control. The
widget gallery (`frus-examples/widgets`, under Navigation) has each one, and
`frus-examples/navigation` is a shop of eight screens built on the router: nested routes,
names, queries, a guard and a page that is not found.
