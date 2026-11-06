# Milestone 756 — Restorable pages, wheels and grids

## Objective

Milestone 753 brought an application back after the system ended its process: the router's
pages, scroll offsets, text fields, and the values hooks and states keep. Its "Not done" listed
the widgets that still came back afresh: a `PageView`'s page, a tab bar's tab, an
`ExpansionTile`'s open flag. This milestone goes through every widget that holds a state of its
own and makes it restorable, opt-in by a `restoration_id`, the way milestone 753 did for scroll
areas and fields.

## Which widgets hold a state of their own

Each widget the list named was read, and each of its neighbours:

| Widget | Who holds the state | Restored by |
|---|---|---|
| `PageView` | the **runtime** (the offset; `.page(n)` is only a request) | `PageView::restoration_id` — new |
| `ListWheel` | the runtime (the same paged machinery) | `ListWheel::restoration_id` — new |
| `GridView::builder` | the runtime (the offset) | `GridView::restoration_id` — new |
| `SingleChildScrollView`, `ListView`, `TextField` | the runtime | already, milestone 753 |
| `TabBar` | the application (`TabBar::new(selected, on_select)`) | the application's value |
| `ExpansionTile`, `ExpansionPanelList` | the application (`open`, `on_toggle`) | the application's value |
| `BottomNavigationBar`, `NavigationRail`, `NavigationScaffold`, `SegmentedButton` | the application (`current_index`/`selected`) | the application's value |
| `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `Stepper` | the application | the application's value |
| `DatePicker`, `TimePicker`, `DateTimePicker`, `CarouselView`, `Drawer`, `SearchView` | the application | the application's value |
| `Navigator` | the application (its stack, as keys) | the application's value |

frus has no tab controller, page controller or expansion controller that keeps an index to
itself: every widget in the second half of the table is **told** what it shows and sends a
message to change it. There is nothing in those widgets to restore — the value lives in the
application, and the application keeps it with `use_restorable_state`, or with a `State` that
names a `restoration_id`, both from milestone 753. A test now holds that path for a tab bar and
an expansion tile, and the demonstration's settings screen does it for its tab, its advanced
section and its reminder wheel.

The three widgets in the first half keep a position in the runtime that the application never
sees as a value — `.page(n)` is applied when the view first appears and when the number
changes, and the finger owns the offset in between — so they need restoring by themselves.

## What was done

### `PageView::restoration_id`, `ListWheel::restoration_id`

- **The page, not the offset, is kept.** A phone turned on its side between the two processes
  has pages of another width; what the reader was on is a page. The value is the page the view
  last reported as shown (`Runtime::page_shown`, kept current by the shell's page-change
  reports), written as an integer.
- `Restoration::register_page(path, id, requested)` (crate-private) claims the page restored
  at `path` when the view is built, with the page the application asks for in that same build,
  and hands both to the runtime as a pending `Page`. `Restoration::apply` became
  `take_pending`, and `Runtime::apply_restored` puts each pending value in place, since a page
  touches more of the runtime than an offset does:
  - `page_requested` takes the application's request, so the view's first sighting does not
    move it to that request;
  - `page_shown` takes the request too, so the restored page reads as a **change** on the first
    frame, and `on_page_changed` (or `ListWheel::on_selected`) tells the application where the
    view is — an application following the page with a plain `use_state` then asks for that
    page from then on;
  - `page_restored` (new, crate-private) keeps the page until the view's pages are measured.
- The walk opens a view with no offset yet on its restored page rather than on the requested
  one, so the **first frame is already on that page** — no page 0 flashing past;
  `Runtime::sync_pages` then turns the page into the view's offset, with the measured extent,
  and `page_changes` reads the same page before the offset exists.
- The demonstration's guided tour gives its panels an id (`"panels"`).

### `GridView::restoration_id`

A grid made by `GridView::builder` scrolls with the list machinery; its offset is now
registered like a `ListView`'s. The other two kinds of grid do not scroll by themselves, and an
id on them keeps nothing.

### The demonstration

- Tour: `PageView::restoration_id("panels")`.
- Settings: the screen's `State` names a `restoration_id` (`"settings"`) and saves the tab, the
  advanced section's open flag and the reminder wheel's minutes; its scroll area has a
  `restoration_id` too.

## How it was tested

- **Driver tests** (`frus-shell`, `restoration_tests`, 5 new, each driving one application,
  taking its restoration data, and handing it to a second application made afresh):
  - `a_page_view_opens_on_the_page_it_was_left_on`: three taps on "Next" (the view glides to
    page 3); the data holds `pages = 3`; the second application's **first frame** draws page 3
    and not page 0, the application's own `use_state` hears 3 through `on_page_changed`, and a
    further "Next" turns to page 4. Without the id the second application opens on page 0.
  - `a_restored_page_survives_another_window_width`: restored into a window 700 wide instead of
    400, the view opens on page 2 at two of the new page widths.
  - `a_list_wheel_opens_on_its_row`: a wheel moved to row 10 opens there (400 px down) and
    `on_selected` brings the application's value to 10.
  - `a_grid_opens_where_it_was_scrolled`: a windowed grid dragged up opens at the same offset.
  - `told_widgets_come_back_through_the_application_values`: a `TabBar` on its second tab and
    an open `ExpansionTile`, both held in `use_restorable_state`, come back as they were.

  The first four fail before the change (the method does not exist; with the registration and
  the runtime's handling taken out, the second application opens on page 0, row 0, offset 0).
  The four were run with the registration taken out of the three widgets' `build_in` (a
  temporary mutation): all four failed, and the other six restoration tests passed.
- **The test driver now reports page changes.** `Driver::frame` never called
  `Runtime::page_changes`, so `on_page_changed` and `ListWheel::on_selected` never reached an
  application under test, while the window's frame has always reported them. It now does, as
  the window does; the first version of the tests passed nothing through it and failed for that
  reason alone.
- The whole of `frus-widgets`' unit tests (2124) and `frus-shell`'s (441) pass; clippy is clean
  for the workspace with all targets.
- **On a phone** (Huawei STK-L21, Android 10, EMUI), the demo as a release APK built from this
  branch:
  - On the Tasks page a draft typed (`Restore`) and the page scrolled; the guided tour opened
    from the drawer and swiped to its third panel. HOME, then `adb shell am kill com.frus.demo`
    (`pidof` empty), then opened again from the recent apps: a new process, logcat
    `android-activity: Creating: … saved state size = 169` then `frus_shell::app: restoring the
    state the system kept (169 bytes)`. The tour came back on its third panel, its dots, number
    buttons and "Panel 3 of 4" agreeing (the screen heard the page through `on_page_changed`).
    Back led to the Tasks page, scrolled as it was, with `Restore` in the field.
  - Settings: the reminder wheel turned to 3 minutes, the About tab chosen, "Advanced options"
    opened, the page scrolled down. The same round (`saved state size = 257`, `restoring the
    state the system kept (257 bytes)`): the screen came back on the About tab, the section
    open, scrolled where it was, and the Controls tab showed the wheel on 3 minutes.
  - Android 10 was the level milestone 753 had not been able to run; on this phone the whole
    path works there, EMUI included.
  - Seen in passing, not from this change: the About tab's two-column `Table` ("Metric",
    "Value") draws both columns on top of each other on the phone.

## Not done

- **No restoration inside told widgets**, by design: a tab bar, a tile, a navigation bar, a
  checkbox or a date picker is given its state, so the application restores it (see the table).
  A controller type that keeps an index by itself, which a `restoration_id` would then apply to,
  does not exist in frus and was not added here.
- **A page view's fractional position** is not kept: a view the process was ended in
  mid-swipe comes back on the page that swipe read as.
- **`Navigator` without the router** is the application's stack and is restored by it; there is
  still no restorable push of a page that carries an `extra`.
- What milestone 753 left (Android before 10, iOS, desktop, the web path in a browser, rows
  following their index rather than their key) is unchanged, except that Android 10 itself has
  now been run (above).
- The overlapping columns of the demo's `Table` on the phone were not looked into.
