# Milestone 724 — A simulated device in the browser

## Objective

Step 5 of simulated devices (milestone 720 has the plan): the phone a desktop window stands in
for, shown in a browser instead, to look at an application's web build on a phone's screen or
to show it to someone. Chosen with the owner: `frus run -d iphone-15 --web`.

## What was done

- **`frus run -d <simulated device> --web`** builds the web application and serves it, and
  opens the first browser found (or serves it for any browser when there is none), the way
  `-d edge` does, with `r`, `R` and `--hot`. `--web` with a device that is not simulated says
  to use `-d web`. A release build is refused, as in milestone 723.
- **How the page is told**: `frus run` adds `<script>window.__frusDevice = 'iphone-15';</script>`
  at the top of the page's `<head>`, before the application loads. It is not the address: on the
  web the address is the router's location (milestone 560), and a `?device=` in it would have
  become part of every route. A name that is not a device id is not written into a script.
- **The shell** reads that property on the web as it reads `FRUS_DEVICE` on a desktop. Everything
  else in milestones 721 and 722 is the same code: the canvas is the window, the device is
  fitted into it, the pointer is mapped into the screen.
- **The field the browser types into** (milestone 711) is put over the field where it is drawn:
  `Simulated::to_window`, the reverse of `to_screen`, takes the field's rectangle from the
  application's pixels to the canvas's, at one CSS pixel each.

## Checked

- `clippy -D warnings` for `wasm32-unknown-unknown`. The first run caught a borrow the host build
  cannot see (`placement()` called while the editor was borrowed). A deliberate type error in
  `cfg(web)` code was then caught too, to be sure the check reads that code.
- `cargo check` for `aarch64-apple-ios`.
- In a browser, `frus run -d iphone-15 --web --machine` in `frus-examples/counter`, driven
  through its JSON channel: the web build was served and Edge opened. The log says *simulating
  iPhone 15 (iOS, 393×852 at 3x)*, and `s` wrote the page: the counter on the iPhone's screen,
  centred in the page, island, status bar and home indicator drawn.

  **`l` found a bug.** Turning the device asked winit for the window's size transposed, which on
  the web writes that size on the canvas, so the page itself turned: a 842 × 1542 canvas, the
  phone in a band across it. On the web the page sizes the canvas, so the size request is now a
  desktop's only. Run again, `l` turned the phone inside the page as it was, larger, since a wide
  page has more room for it on its side. No Edge of `frus run`'s was left running.

## Tests

- `simulated::a_rect_goes_to_the_window_and_back`: at two zooms.
- `serve::a_page_is_told_its_simulated_device`: the script, at the top of `<head>`; an id that is
  not one is not written; a page with no `<head>`.
- `main`: `--web` is parsed, and the profile is left for `run` to set.

## Not done

- The accessibility tree's bounds on a desktop (AccessKit) are the application's, not the
  window's, while simulating. A screen reader's highlight would be off by the screen's offset.
