# Milestone 563 — Links from outside the web

## Objective

The last of what issue #31 named: a route that comes from outside the application, on the two
platforms that are not a browser. `myapp://orders/42` and `https://example.com/orders/42` on
Android; `myapp://orders/42` on a desktop, where the operating system starts the program with the
link as an argument, and a second copy of a running program should hand it over rather than open
a second window.

## What it does

- **One translation, for every platform** (`frus_shell::location_of_link`): a link's path and
  query are the location for a web link; for a custom scheme the "host" is the first segment,
  because `myapp://orders/42` means `orders/42` to whoever wrote it, not a server called `orders`.
  A fragment is dropped. `link_among` finds the link among a program's arguments.
- **Android**: the activity's launch intent is read (`getIntent().getDataString()`), best-effort
  and with every failure's pending exception cleared, as the settings read is. The location goes
  to `Application::open_location` before the first frame, the way the address does on the web.
  The demo declares an intent filter for `frusdemo://`.
- **A desktop**: the link among the arguments is handed to `open_location` at start.
  **`Application::instance_id`** (`FrusApp::single_instance`) makes the application one window: a
  second process hands its link to the first and ends, and the first opens the location and comes
  to the front.

## Decisions

**A link is a location, not a message.** The application already understands locations, and
`open_location` is the one way in from the web's address bar; a link is another source of the
same thing, so nothing new is asked of an application that handles the address.

**The single-instance channel is a socket on the loopback interface and a lock file**, in
`std` alone. A named pipe, a Unix socket and a mailbox would each be right on one platform.
The cost is that a local socket is open to anything on the machine — including a web page that
writes to `127.0.0.1:<port>`. So the listener answers only a first line of exactly
`frus-link <token> <link>`, the token being in a file only the user can read on Unix, and closes
anything else without a word; a request from a browser starts with `POST / HTTP/1.1`. A second
instance leaves only if it is answered `ok`, so a lock left behind by a crash — or a port some
other program has since taken — makes the new process the first, not a process that vanishes.
It is opt-in, because an application with a window per document is right to have many instances.

## Not done, and why

- **A link opened while an Android application is running** reaches the activity as
  `onNewIntent`, which the native activity does not forward; catching it takes a class of the kind
  the input bridge is (a dex), and is not made here. Only the launch is covered.
- **The operating system's registration of a scheme** — a registry key on Windows, a `.desktop`
  file with `x-scheme-handler` on Linux — is an installer's job, and is documented, not done.
  **macOS** delivers a URL to a running program as an Apple Event, not as an argument, and winit
  does not surface it: links do not reach a macOS build.
- **Not run on a device or on Linux/macOS.** The translation and the whole single-instance
  protocol are tested (seven tests each, the latter over real loopback sockets, on Linux); the JNI
  read is written against the pattern of the settings read that runs on a device, and compiled
  and linted for `aarch64-linux-android`; nothing here has been started with an intent, and the
  demo's APK was built to see the manifest merge, not run.
