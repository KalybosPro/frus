# Milestone 622 — The system's Back is a pop

## Objective

When the system asked to go back — Android's back button, the browser's Back, a back key —
the shell played it as a back swipe let go at once: `back_gesture(0.0)` then
`back_gesture_end(5.0)`. It looked like a pop on a spring, and since milestone 621, under an
iOS theme, like an iOS page let go: 350 ms on the swipe's curve. The reference pops the route
(`Navigator.maybePop`), and the page leaves with its own transition — 500 ms for the slide,
300 for the zoom, 450 for the fade — as it leaves for any button that pops it.

## What was done

- **`Application::go_back()`**: the system asked to go back, with no finger on the page.
  Called only when `can_go_back()` says yes. The default plays it as a back swipe let go at
  once, which is all an application answering only the gesture hooks can do.
- An application built from components (`FrusApp`) answers it with its router's `pop()`: the
  page leaves at once from the stack, with the transition the theme's platform draws.
- The shell's Back goes through `go_back`. What it does short of leaving the application is
  now `system_back_step`, which says whether it went back from anything; with nothing left,
  the application exits as before. The test driver can press it: `Driver::system_back`.

## Tests

- `the_system_back_pops_the_page`: after Back the page is off the stack at once — a pop, not a
  swipe still settling — and with nothing left, Back says so.
