# authentication

An application with sign-in. It shows how to:

- keep **who is signed in** in one service (`Auth`) that every page shares;
- **guard routes**: signed-out people are sent to `/login`, signed-in people are kept off the
  auth pages, and the page someone was heading to is kept in a `?from=` query and restored
  after sign-in;
- make the router **follow the service**, so signing in or out moves the person without any
  page navigating by hand;
- validate forms with `frus::form::{Form, Rule}`, showing errors only after the first submit;
- show a **loading state** on submit buttons and the **server's errors** inline;
- **remember the session** across restarts.

The server is a fake one, in memory, that takes 0.9 s to answer. It knows one account:

    demo@frus.dev / password123

The login page shows this account and has a button that fills it in.

## Run it

    cargo run                      # desktop
    cargo run -- /profile          # start at /profile: signed out, you land on /login?from=/profile
    cargo apk run --lib            # Android
    cargo test                     # the guard, the fake server, the forms, the session

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/authentication.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

The guard works the same in the browser: opening `http://localhost:8080/#/profile` signed out
lands on `#/login?from=/profile`.


## How it fits together

```
lib.rs  ── Auth::new() ──► router(auth) ──► FrusApp::router(..)
                              │
            redirect: guard_redirect(auth.is_signed_in(), location)
            refresh_listenable(auth.changes())  ◄── sign_in / register / sign_out
                              │
              pages get a clone of `auth` from their route builder
```

1. `Auth` holds the user in a `ValueNotifier<Option<User>>`. A `ValueNotifier` is a
   `Listenable`: whoever listens is told when the value changes.
2. The router's `redirect` calls `guard_redirect` on every navigation. It is a plain
   function of `(signed_in, location)`, so all four cases (signed in or not, public page
   or not) and the `from` round trip are unit-tested without a window.
3. `refresh_listenable(auth.changes())` runs the guard again on the current page whenever
   the user changes. That is why the login page does nothing when sign-in succeeds: the
   router sees the new user and leaves for the `from` page, or for home.
4. A request (`sign_in`, `register`, `request_password_reset`, `update_name`) answers
   later through `frus::host::after`, which works the same natively and on the Web. The page
   gets the answer through its state handle (`cx.handle()`), turns its spinner off, and
   shows the error if there is one.

## Files

```
src/
  lib.rs                         creates the service and the app (theme from one seed colour)
  common/
    auth.rs                      Auth: the user (ValueNotifier) + requests with simulated latency
    session.rs                   saves and loads the session id (a file on desktop)
    routes.rs                    the paths, PUBLIC_ROUTES, the `from` query key
    routers.rs                   GoRouter + guard_redirect + `from` encoding (tested)
    submission.rs                submitted / loading / server_error, shared by every form
    shared.rs                    logo, page frame, submit button with spinner, error alert
    colors.rs                    the brand seed colour
  models/user.rs                 User { id, name, email }: no password on the client
  features/
    auth/
      controllers/user_store.rs  the fake server: accounts, errors, reset "outbox" (tested)
      pages/                     loginpage.rs, register.rs, forgotpassword.rs
      states/                    each page's state and its Form rules (tested)
    home/
      pages/homepage.rs          welcome + AppBar actions (profile, sign out)
      widgets/user_card.rs       a StatelessWidget configured with a User
    profile/
      pages/profilepage.rs       edit the display name, sign out
      states/profilestate.rs     its rules (a name equal to the current one is refused)
```

## Remembering the session

frus has no storage API of its own. An application stores data the way any Rust program
does. This example writes the session id (never the password) to a file in the user's
configuration folder:

- Windows: `%APPDATA%\frus-authentication-example\session`
- macOS: `~/Library/Application Support/frus-authentication-example/session`
- Linux: `$XDG_CONFIG_HOME` (or `~/.config`)`/frus-authentication-example/session`

At start-up, `Auth::new` reads the id and asks the server whether it still names an
account. If it does, the app opens signed in. If it does not, the file is deleted. The fake
server forgets accounts registered during a run when the app restarts, so only the demo
account (always id `u1`) survives a restart.

On **Android** and the **Web** the session is kept in memory only. To add persistence there:

- **Web**: add `web-sys = { version = "0.3", features = ["Window", "Storage"] }` under the
  `wasm32` dependencies. In `session.rs`, read and write
  `web_sys::window()?.local_storage().ok()??` with `get_item` / `set_item` / `remove_item`.
  `crates/frus-demo/src/storage.rs` in the frus repository does exactly this.
- **Android**: write to the app's internal files directory
  (`/data/data/<package>/files`, the `internalDataPath` of the activity). It is private to
  the app, so no permission is needed.

A real application would store an opaque token from its backend instead of the user id. It
would check that token at start-up with a request and show a splash screen while it waits.

## Swapping in a real backend

Only `features/auth/controllers/user_store.rs` and the four request methods in
`common/auth.rs` change. Send the same requests with `frus::fetch` (or `host::spawn` around
a blocking client), map the HTTP errors to `AuthError`, and call `done` with the result.
The pages, the guard and the router stay exactly as they are.
