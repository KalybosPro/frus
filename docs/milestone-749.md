# Milestone 749 — The page fits the phone's screen, and its keyboard

## Objective

The widget gallery, opened in Chrome on an Android phone (Huawei STK-L21, Android 10, drawing
through WebGL 2 since milestone 745), showed a dark band at the bottom of the screen, under the
application. The page had to fill what the phone shows, as a native application does: no band,
nothing of the application below the edge, through rotation and the browser's bars.

## What was measured

The gallery served from this machine (`adb reverse`), the phone's Chrome driven over its
DevTools, portrait, address bar shown:

| | value |
|---|---|
| `innerWidth` × `innerHeight` | 423 × 788 |
| `visualViewport` | 423.53 × 787.84, offset 0, scale 1 |
| `devicePixelRatio` | 2.55 |
| `100vh` / `100dvh` / `100svh` | 843.92 / 787.84 / 787.84 |
| canvas, CSS box | 423.53 × **843.92** |
| canvas, `width` × `height` | 1080 × 2152 (the box × 2.55, exact) |
| document `scrollHeight` | 845 |

Landscape: `innerHeight` 339, canvas 395.29 tall (`100vh` again), 2224 × 1008 attributes.

Two separate faults came out of it.

**The canvas was taller than the screen.** Every `index.html` sized the canvas
`width: 100vw; height: 100vh`. On a phone, `100vh` is the height with the address bar *hidden*
(the "large" viewport); the page is `overflow: hidden` and does not scroll, so the bar never
hides, and the canvas ran 56 CSS pixels (143 device pixels) past the bottom of the screen. The
application laid itself out for 844 pixels and the last 56 were never seen — a bottom navigation
bar, a floating button or the end of a list cut off, in both orientations. The canvas's
attributes matched its box exactly: winit's resize observer was right, it was observing a box
that was wrong. The overflow also made the document 845 pixels tall in a 788-pixel window, so
anything that scrolls the page programmatically could shift it.

**The band was the phone's navigation bar, painted by the browser.** The pixels of the band were
exactly `#121418`, the page's background — the loading screen's colour — while the gallery is
drawn light (`#F5F5F7`). Chrome on Android paints the system navigation bar, under its own
window, with the background of the document's root element, and follows it when it changes:
setting the root's background to red over DevTools turned the bar red at once. The page's
background was never seen anywhere else, so nothing had kept it in step with the application.

## What was done

**The canvas fills the window the page is given.** Every page — the `frus create` template,
`frus-hello` and every example (`authentication`, `counter`, `forge`, `forms`, `hello_world`,
`http`, `navigation`, `websocket`, `widgets`) — now pins the canvas:

```css
canvas { display: block; position: fixed; inset: 0; width: 100%; height: 100%; }
```

A fixed box of 100% is the layout viewport: as tall as what is seen, and it follows the browser's
bars when they come and go, the way a full-page application is laid over a page by the leading
toolkits. Being out of the flow, it no longer makes the document taller than the window. The
status box, fixed too and earlier in the page, gets `z-index: 1` so that "Loading…" and an error
stay over the canvas. `frus-demo` already sized its column with `100dvh` and is unchanged.

**The page's background follows the application.** The shell already decides, every frame, the
colour of each system bar on Android: the region drawn at the bar's edge of the frame
(`SystemUiOverlayStyle` on an `AnnotatedRegion`, an app bar...), then the theme's background
(#46). On the Web the same answer for the navigation bar now becomes the root element's
background, set inline (`web_page::paint_behind`), and only when it changes. A translucent bar
colour is laid over the theme's background, in linear light as the renderer composites, so the
page is always opaque. The page keeps its own colour until the first frame, so the loading
screen is unchanged.

## Found on the phone after that, and fixed

The application was then gone through on the same phone — fields, the keyboard, scrolling,
the back button, leaving and coming back, rotation, a lost GPU context. Three faults were in the
web shell.

**The keyboard panned the whole page.** A phone's browser lays its keyboard over the page and
shrinks only the *visual* viewport. Focusing the Search field of the TextField demo, low on the
screen, made Chrome pan the page 394 CSS pixels up to keep the editing element (milestone 711,
which sits over the field) in sight: the field was visible, but the application's own top bar —
the title and the tabs — had gone off the top of the screen, and nothing told the application a
keyboard was there. A phone's own applications keep their bars and lift their content.

- `web_page::keyboard_cover` measures the keyboard as the part of the canvas below the visual
  viewport (nothing while the page is pinch-zoomed), and `compute_insets` reports it as the
  window's bottom inset on the Web. The shell's existing baseline turns it into the keyboard's
  `view_insets`, and its existing "reveal after keyboard" scrolls the focused field into view:
  the same path Android takes.
- A listener on the visual viewport's `resize` asks for a frame: the keyboard resizes nothing a
  resize observer sees.
- On a touch screen (`pointer: coarse`), the editing element stays in the top quarter of the
  canvas until the keyboard is up, then within what the keyboard leaves visible
  (`web_editing::element_top`), so the browser never has a reason to pan. It is over the field
  again once the field has been scrolled above the keyboard.

Measured after: `visualViewport` 423 × 322, offset **0**; the bar and tabs stay; the Search field
and the "Code" field at the bottom of the demo both come up just above the keyboard; closing it
gives the application its full height back.

**What the browser typed skipped the field's filter and its limit.** The demo's "Code (upper
case)" field, whose filter upper-cases, held `Xyab` after `xyab` was typed on the phone: the
browser's change went in through `replace_value`, the path undo uses, which applies neither a
filter nor a length limit. It now goes in as the field's own typing
(`web_editing::browser_change`): the stretch the browser replaced, found from the two values,
is typed over through `on_edit`, so the filter, `max_length` and read-only all apply. The
browser's caret and composition are kept when the field took the change as it came (a
substitution keeps the length), the field's own caret otherwise.

**A lost WebGL context stayed dark.** Taking the canvas's context away
(`WEBGL_lose_context.loseContext()`, as a GPU process restart or memory pressure does) left the
page dark for good: a browser gives a context back only to a page that cancels
`webglcontextlost`, and nothing did. The shell now cancels it, and on `webglcontextrestored`
makes a new renderer, as for a lost device.

## Checked on the phone and found right

- **Typing**: the keyboard opens on a tap, the caret blinks where it should, `adb input text`
  types, Backspace deletes, the arrow keys move the caret and an insertion lands there.
- **Back closes the keyboard first**: the field keeps its focus and the route stays; a second
  Back goes back a page.
- **Scrolling** (a real touch through DevTools' `Input.dispatchTouchEvent`): drags follow the
  finger, a quick one flings to the end of the list. `adb shell input swipe` does **not**
  scroll: its events reach Chrome with an empty `pointerType`, which winit takes for a mouse,
  and a mouse drag does not scroll — a test artefact, not a fault.
- **Leaving and coming back** (Home, five seconds, Chrome again): drawn at once, same route,
  same scroll position.
- **Rotation**: landscape gives a 872 × 339 canvas (2224 × 865 device pixels), the navigation
  bar on the right painted light; portrait again gives 423.5 × 787.8 back.
- **Navigation**: a tap opens a category, then a demo; Back returns.

## How it was tested

- `create::tests::every_page_fills_the_window_it_is_given` (frus-cli) reads every page of the
  checkout and checks the canvas rule: no `vh`, fixed, inset 0, 100% by 100%, and a z-index on the
  status box. It fails on any page before this change (checked with the `counter` page restored).
- `web_page` tests: the CSS spelling of a colour, opaque and see-through bars, the keyboard's
  cover (with rounding and a pinch-zoomed page).
- `web_editing` tests: the stretch a change replaced; the browser's typing through an
  upper-casing filter (`Xyz` becomes `XYZ`, the browser's caret kept), a dropped character (the
  field's caret), a paste over `max_length` (cut to the limit), a deletion; where the editing
  element goes on a touch screen and off one.
- On the phone (Huawei STK-L21, Android 10, Chrome, WebGL 2), the gallery built as the Pages
  workflow builds it, served over `adb reverse` and driven over DevTools: every measurement and
  check above, with screenshots.
- In headless Edge as a 412 × 780 phone: the canvas is 412 × 780, the page's background follows
  the application, a touch drag scrolls, with the Windows and with an Android user agent.

## Not done

- **A press outside a field ends the editing**, on the Web as on Android. On the Web that is
  also what the leading toolkit does (a touch outside unfocuses there, the browser's input losing
  its focus anyway), so a scroll that starts outside the focused field closes the keyboard. On
  Android's own applications a touch outside does not unfocus; frus's native shell does — not
  checked on a native build here, and not changed.
- **Safe areas** (`env(safe-area-inset-*)`, a notch in landscape) are not read on the Web: the
  side and top insets stay zero there. The test phone has no notch to check it on.
- The status bar's colour (`<meta name="theme-color">`, which Chrome uses for its toolbar) is not
  set from the application: the toolbar is the browser's, and colouring it is the page's choice.
- The page's background follows the theme's background where no region says otherwise; the
  gallery's scaffold paints `#F5F5F7` over a theme background of `#F5F6F8`, one step apart.
- Opening a deep link to a category (`#/category/inputs`) shows it without a back arrow, and a
  demo left and come back to through the browser's Back and Forward is built afresh (its fields
  empty). Router behaviour, not the page's; noted, not changed.
- No iPhone was available: Safari's toolbar, which collapses as a page scrolls, was not seen.
