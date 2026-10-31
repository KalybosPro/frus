# Milestone 716 — Writing frus easily

## Objective

People trying frus 0.5 asked for code that is easy to write. Writing the guide (milestone
715) against the compiler showed exactly where it was not: seven places where an example had
to be longer than the idea it showed. This milestone removes all seven.

## What was done

### 1. A plain closure wherever a widget calls back

`on_input`, `on_toggle` and `on_change` already took a closure that returns nothing, so that a
function component writes `move |text| name.set(text)`. Forty-five other setters took only
`impl Fn(..) -> Msg`, among them `Scaffold::nav`, `TabBar`, `SegmentedButton`, `NavigationBar`,
`NavigationRail`, `RadioGroup`, a dropdown's options, `Pagination`, `Stepper`, `Rating`, the
date and time pickers, `GestureDetector`'s pans and `MouseRegion`'s hovers. A function
component had to build a `Callback` inside the closure by hand.

Each now takes `impl Fn(..) -> R` with `R: IntoMsg<Msg>`, wrapped by the helper `on_input`
already used (`handler1`; `handler3` is new, for the time pickers' three arguments). A closure
returning a message is a message, as before; one returning `()` becomes a `Callback` that runs
it. The change was made by a script over every such parameter, so none was missed.

One thing this costs, the same as `on_input` already did: a closure returning `()` given to a
widget **whose message type nothing else names** is ambiguous between the message `()` and a
`Callback`. One of frus's own tests built a lone `DatePicker` that way and needed its type
written out. An application's widgets are named by where they go (`Box<dyn Widget>`), so it
does not meet it.

### 2. A snack bar in one line

`host::show_snack_bar(SnackBar::new("Saved"))`. frus keeps the queue — one bar at a time, the
next when the current one goes —, the four seconds (`show_snack_bar_for` for another
duration), and the action, which also hides the bar. `hide_current_snack_bar` and
`clear_snack_bars` complete it.

The bar is shown by the application's root. `FrusApp` lays its pages under an
`OverlayPortal` whose overlay is the current bar, at a new placement, `Notification`: across
the bottom of the window, above the system's bars, with no scrim, so a press anywhere else
still reaches the page. The portal is there **with or without a bar** — an empty box when there
is none — so that a bar coming or going never moves the pages in the tree, and they keep their
state. A bar shown before a navigation is still there after it. `SnackBar` became `Clone` (its
controls are made from its fields) so the queue can hand the root a copy every frame.

### 3. The way back, on its own

The router puts every page in a `ScaffoldScope` that says how to go back — `router.pop()` —
when there is a page beneath it. `ScaffoldInfo` gained that message, and the app bar, which
already grew a menu button for a drawer from the same ambient, grows the back arrow from it: the
platform's (a chevron on iOS and macOS), labelled for screen readers, unless the screen has a
drawer or the bar a `leading` of its own — the reference's order.

Two rules make it reach the bar. A scope says only the way back and keeps the drawers of the
shell around it (a page is not a shell), and a nested shell keeps the way back of the scope
around it: without the second, a page whose root is a `Scaffold` would have the `Scaffold`'s
own scope replace the router's and lose the arrow.

### 4. `AppBar` and `Scaffold` are widgets

Forgetting `.build()` was a type error that named neither type. Both now keep their parts in
a `Lazy` — the parts until the walk first asks the widget anything, then the widget they
assemble — and implement `Widget` through the transparent-wrapper macro, so they forward every
structural hook. `.build()` stays, for those who want the box now.

### 5. A lazy list of rows of any height

`ListView::builder(count, build)` lays each row out at its own height and still builds only
the rows in view. A row is measured the first time it is built, which is when it scrolls in;
the extents are kept between frames in the runtime, and the rows not seen yet are counted at
the mean of those that were (`.estimated_extent(..)` before any is). The rows above the window
are the ones the reader came past, already measured, so the window does not jump while one
scrolls. A measurement that differs from what the frame assumed asks for one more frame.

### 6. An animation a component drives

`cx.use_animation_controller(AnimationController::unit)` gives an `AnimationHandle`: start it
with `forward`, `reverse`, `repeat` or `animate_to`, and every frame moves it and rebuilds what
reads `value()`, until it stops. The frames reach it through `tick_animations`, which
`FrusApp::tick` calls; a controller nobody holds any more is dropped from the list. A
`StatefulWidget` keeps one with `AnimationHandle::new`. It replaces a `use_ref`, a
`use_interval` ticking at a guessed rate and a `use_state` mirroring the value.

### 7. Testing a page as it is used

`frus::Tester` (feature `testing`) wraps the shell's windowless driver in the words a test is
written in: `tap("Send")`, `enter_text("Title", "Hello")`, `wait(seconds)`, `back()`,
`shows("Created")`, `texts()`. Each action draws frames until the screen is still. A tap on
what is not there fails and lists what was.

Making it usable found two things the driver did not do:

- **Timers never fired.** A `Command::after` (what `host::after` becomes) was sent through the
  event loop, which a driver has none of. Under a driver it is now due at a time of the
  driver's own clock and dispatched by the frame that reaches it — the clock gestures have used
  since milestone 584.
- **"Still" did not count the application's animations**, only the widgets'. A router's first
  page arriving through its transition looked still, and the test read an empty screen. The
  driver now reports what `Application::tick` answered.

## The guide

Each page that showed the long form shows the short one: the `Scaffold` example is a function
component again, with no `.build()` and a plain closure for its tabs; the navigation page has
snack bars; the lists page has `ListView::builder`; the animation page drives a controller; the
testing page tests a page with `Tester`. The testing page now needs the `testing` feature, so
CI's optional-features step runs `cargo test -p frus --features "net json testing"`.

## Tests

- `frus-shell/src/tester.rs`, end to end through `FrusApp` and the tester: typing and a tap
  change the page; a tap on nothing fails; a snack bar shows over the page and goes after four
  seconds; its action runs and hides it; a pushed page's app bar offers the way back and the
  first does not, and pressing it goes back; a function component picks a destination with a
  plain closure; rows of two heights are built only in view and stacked by their own height; an
  animation a component starts reaches its end through the frames.
- `messenger.rs`: bars are shown one at a time in order, only the one on screen is timed; a
  timer leaves alone a bar shown after its own; the action runs and hides.
- `animationhandle.rs`: the frames move a started controller to its end and stop; a controller
  nobody holds is not driven; a change asks for a rebuild.
- The guide's 48 doctests, with every feature on.

## Not done

- The widget gallery's snack bar demo still shows the queue by hand: it is a demo of
  `SnackBarQueue` too, which stays for applications that want their own.
- `ListView::builder` does not keep the reader's place when rows **above** the window change
  height (a picture that loads in a row already scrolled past): the rows below move by the
  difference. The reference corrects the scroll offset; that is a further step.
- A typed `Application` (not `FrusApp`) gets neither the snack bar layer nor the driven
  controllers; it owns its view and its tick, and can call `tick_animations` itself.
