# Milestone 751 — The page around a web application, and the way back through it

## Objective

Milestone 749 went through the widget gallery on a phone's browser and left five faults
written down, all at the edge between the application and the page or the browser it runs
in:

1. The safe areas — a notch, rounded corners, a home indicator — were not read on the Web:
   the top and side insets stayed zero there.
2. The browser's toolbar (and the status bar over it), which a phone's browser colours with
   the page's `<meta name="theme-color">`, was never told the application's colour.
3. The page's background — which Chrome on Android paints the phone's navigation bar with —
   followed the theme's background (`#F5F6F8`), while the phone's screenshots showed the
   gallery's canvas as `#F5F5F7`: one step apart.
4. A link straight to a category (`#/category/inputs`) opened it with no back arrow.
5. A demo left and come back to through the browser's Back and Forward was built afresh, its
   fields empty.

## What was done

### 1. Safe areas on the Web

A browser exposes the safe area to style sheets only, as `env(safe-area-inset-*)`. The shell
now keeps a probe element in the page that takes no room and is never seen, padded by the four
insets, and reads its computed padding (`web_page::safe_area`). The answer is kept for the
window's size: a notch moves to a side only with a rotation, and a rotation resizes the window.

`compute_insets` reports them as the window's insets on the Web, as Android reports its
content rect, with the bottom one raised to the on-screen keyboard's cover
(`web_page::window_insets`): the keyboard is laid over the bottom of the page, the home
indicator with it. The shell's existing baseline then splits the bottom into padding and the
keyboard's `view_insets`, exactly as on a phone's own build: with the keyboard up,
`view_insets.bottom` is the keyboard and `padding.bottom` is zero; without it, `padding` is the
safe area, which `SafeArea` and the scaffold already honour.

The insets are non-zero only on a page that asks for `viewport-fit=cover`. Every page of the
checkout already did; a test now keeps it so (`every_page_asks_for_the_whole_screen_and_its_safe_areas`
in frus-cli, over the template, `frus-hello`, `frus-demo`, `forge` and every example).

### 2. `theme-color`, and 3. what the bars stand against

On Android the shell colours the system bars from the regions under them
(`SystemUiOverlayStyle` on an `AnnotatedRegion`), then the theme's background (#46). On the Web
the bars are the browser's and sit against the page's very edges, so the shell now asks, for
each edge:

1. the colour a region at that edge **states** — an explicit request wins;
2. otherwise the colour the frame **actually paints** there;
3. otherwise the theme's background;

laid over the theme's background when translucent, since nothing is behind a page
(`web_page::bar_color`). The bottom edge's answer is the root element's background (milestone
749's mechanism); the top edge's is the page's `<meta name="theme-color">`, created in the
`<head>` when the page has none (`web_page::paint_toolbar`), as a `#rrggbb` triplet — a
translucent `theme-color` is not honoured by browsers. Both are written only when they change.

What is painted is read from the frame's scene by a new `Scene::flat_color_at(point)`
(frus-core): the solid rectangles over the point composited in drawing order, in linear light
as the renderer does, a border band counted as the border's colour, a group's opacity applied.
Text is left out (glyphs leave the colour behind them showing), and so are soft shadows. Where
what shows is not one colour — a picture, a gradient, a vector shape, a transformed or filtered
layer — it says so (`None`), and the theme's background is used: the answer is never a guess.

What the one step was. The gallery paints nothing of its own colour at its edges: its scaffold
has no background, and what shows there is the root's fill of the theme's background,
`#F5F6F8` — which is what `flat_color_at` finds, and what the page was already given. In
headless Edge, through WebGPU and through the WebGL 2 fallback alike, the canvas's pixels at the
top, the middle and the bottom are `#F5F6F8`, the page's background `rgb(245, 246, 248)`, the
`theme-color` `#f5f6f8`: the three agree to the byte. The `#F5F5F7` of milestone 749's
screenshots is the phone's own path from the canvas to the screen (in J749's captures the
navigation bar, painted by the system from the page's colour, reads `#F5F6F8` exactly, while the
canvas beside it reads one step lower), not a colour the application paints. No colour the page
is given can make the two meet on that phone: the system paints the bar with the colour as it
is, and the canvas goes through the browser's compositor. The rule above makes the page follow
what is painted wherever an application does paint its edges in a colour of its own — a
coloured app bar, a bottom bar, a page background of its own — which the theme alone did not.

### 4. A link to a nested page has its parent beneath

The router already builds the stack from the location the way mature routers do: a location
that a **sub-route** answers to gets one page per route of the chain, so `/users/42` under `/`
has `/` beneath it, and its app bar shows Back. The gallery declared its three routes side by
side at the top level, so `/category/inputs` matched a route with no parent. They are now
sub-routes of the home page (`category/:id`, `widget/:id`), the addresses unchanged.

A demo's address does not say which category it was reached from, so a link straight to a demo
has the home page beneath it, not the category.

The router had a second fault on the same path. The browser's Back and Forward reach the
application as a location, which the router took through `go`: the stack rebuilt from the
location. A page that had been **pushed** — every page of the gallery is — was rebuilt as a new
page, with nothing it held, and with only its parent routes beneath it: from home → category →
demo, the browser's Back gave a category page with no way back at all, the gallery's routes
being flat. `GoRouter::open`, the entry for locations from outside, now goes back to a page that
is already beneath the top with that location, keeping it and what is under it, as a browser
history restores the stack it had rather than one derived from the address. A redirect still
has its say: a location it would send elsewhere is navigated to as before.

### 5. Back, then Forward, finds the page as it was left

A browser's Forward returns to a page that was popped. Its state — a form's fields, typed into
the application's own state — had been let go with it, as every popped page's is.

The router can now **keep popped pages** (`GoRouter::keep_popped_pages(limit)`): the pages
popped off the top, the nearest last, are kept in the tree as hidden pages of the navigator
(built, not laid out, drawn or given input — what a page covered by another already is), under
the very key they had on the stack. A location handed back in through `open` that is one of
them puts them back on the stack, and the page comes back as it was: its fields, its scroll,
everything its components hold. Any move that is not back — a push, a `go` elsewhere, a
replace — lets them go, as a browser drops the entries ahead when a new one is made. A pop
sliding several pages out slides the top one out under its kept key too, so it is the same page
throughout.

It is the general mechanism, not one for text fields: whatever a page's components hold is
kept, because the components themselves are. The cost is a build of each kept page per
rebuild, and its timers keep running while it is kept. So it is off by default — a phone's back
button expects a popped page to be let go — and `FrusApp::router` turns it on, for four pages,
on the Web only, unless the application set its own number.

## How it was tested

- `Scene::flat_color_at` (frus-core): the last opaque surface; a translucent one composited in
  linear light (half black over white is a linear 0.5); a group's opacity; a border band; text
  ignored; a shape or a picture is no answer, and an opaque surface over them is again; a
  rectangle clipped away from the point does not count.
- `web_page` (frus-shell): the `theme-color` spelling; a bar is what is stated, then what is
  painted, then the theme, and always opaque; computed
  lengths read in CSS pixels; the safe area and the keyboard make the window's insets (a notch and
  a home indicator, then the keyboard over the indicator, then landscape).
- Router (frus-widgets): an address that is a pushed page beneath goes back to that page, the
  same key, home still beneath — it failed before (`["/category/inputs"]`, no way back); a
  redirect still sends a page beneath elsewhere; a kept page comes back as it was left, through
  the browser's Back and through the app bar's, one step at a time and two at once; a new move
  lets kept pages go; a router keeps nothing unless asked; the nearest are the ones kept.
- Tester (frus-shell): a router with a pushed form, a name typed, the browser's Back and Forward
  handed in as locations: `Typed: Ada` is there again, and after the app bar's Back too; a router
  that keeps nothing shows the field empty, the behaviour every page had before.
- The gallery (`frus-examples/widgets/tests/routers_test.rs`): a link to a category has the home
  page beneath and Back lands on it — it failed before; a link to a demo keeps its query.
- frus-cli: every page asks for `viewport-fit=cover` with the device width and an initial scale
  of one.
- In headless Edge as a 412 × 780 phone (touch, mobile), the gallery built as the Pages workflow
  builds it (`web-release`, `wasm-bindgen --target web`), driven over DevTools:
  - `#/category/inputs` opened straight: the bridge offers `Back`, and tapping the app bar's
    arrow lands on the list of categories (`#/`) — before, the page had no arrow.
  - `<meta name="theme-color">` was created with `#f5f6f8`; the root's background
    `rgb(245, 246, 248)`; the canvas's pixels the same, through WebGPU and through WebGL 2
    (`navigator.gpu` removed).
  - DevTools' `Emulation.setSafeAreaInsetsOverride` (47 at the top, 34 at the bottom): the probe
    reads `47px 0px 34px 0px` after a rotation and back, and the application's app bar comes
    down 47 pixels under the "notch". With no override, the probe reads zero, as a desktop
    browser does.
  - Home → Inputs & forms → TextField, `Ada Lovelace` typed into Name (a tap, then DevTools'
    `Input.insertText`), then `history.back()` and `history.forward()`: the field holds
    `Ada Lovelace`. The same after the app bar's Back then the browser's Forward, and after
    `history.go(-2)` then `history.go(2)`.

## Not done

- **Android's own bars** still take the region, then the theme: the painted colour is used on
  the Web only. The phone was not available to this milestone, and a change to what the system
  bars of every Android application look like is not one to ship unseen.
- **No device with a notch** was available, and the phone was not used: the safe-area path was
  checked in headless Edge with DevTools' safe-area override, and the arithmetic by tests.
  Safari on an iPhone, where the insets matter most, was not seen; nor was the toolbar's colour
  on a real phone's Chrome.
- **The phone's one-step difference** between its canvas and its navigation bar (item 3) is in
  the device's path from canvas to screen and is not addressed; it was not investigated on the
  phone.
- **A demo reached by a link has the home page beneath it, not its category**: the address does
  not carry the category. Changing the gallery's addresses to `/category/<id>/widget/<id>` would
  do it, and break the links already shared.
- **A page kept for Forward is kept in memory only**: a reload, or a Forward after the router was
  told to go somewhere new, builds it afresh, as before. State restoration across a reload —
  serialising what pages hold into the history entry — is not done; `State::save` exists for
  `frus run`'s restarts and could carry it, for the states that opt in.
- The browser's own `theme-color` set by a page's author is overwritten by the application's
  colour once the first frame is drawn.
