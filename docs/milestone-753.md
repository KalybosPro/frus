# Milestone 753 — State restoration

## Objective

On the user's phone, the system's low-memory killer ended a frus application twelve seconds
after it went to the background (130 MB). Coming back from the recent apps started it afresh:
the page, the scroll position and the half-typed draft were gone. A production application has
to survive that: the system keeps a small parcel of data for it — Android's saved instance
state, a browser tab's session storage — and hands it back to the new process. frus never wrote
anything into that parcel.

## What was done

### The API (`frus-widgets/src/restoration.rs`)

- **`RestorationBucket`**: a tree of values, each a short text under an id, each child bucket
  under an id of its own. `encode` writes it as UTF-8 text (a header line `frus-restoration 1`,
  then one line per value: its path and its text, `\`, `/`, tabs and line breaks escaped), so a
  browser's storage holds it as it is; `decode` reads it back and answers `None` for anything
  else — another version, another format, damaged bytes — so a page starts afresh rather than
  from a guess.
- **`RestorableValue`**: how a value becomes text and back. Numbers, `bool`, `char`, `String`
  and `Option`s of them implement it; an application implements it for its own types, or turns
  on the new `serde` feature for `use_restorable_serde` (JSON).
- **Restorable values**, each registered under an id in the scope it is built in:
  - `cx.use_restorable_state(id, init)`: `use_state`, restored;
  - `cx.use_restorable_text_controller(id, initial)`: a controller whose text is restored;
  - a `State` that returns a `restoration_id()`: what `State::save` returns is kept, and given
    back through `State::restore` before `init_state` — the same pair the hot restart uses;
  - `TextField::restoration_id(id)`: the caret and the selection (and a multi-line field's
    scroll);
  - `SingleChildScrollView::restoration_id(id)`, `ListView::restoration_id(id)`: the offset;
  - **rows of a `ListView`**: a row is built during the layout (milestone 748), after the
    page's build has returned, so the list keeps the scope it was made in and builds each row
    in a scope of its own, `row <index>`, below the list's restoration id when it has one. The
    same id can then be used in every row. A row that scrolls out of view and is not kept alive
    disposes its components, and what their hooks kept goes with them, as a row's state does;
    a row restored but not scrolled into view yet keeps its values until it is built;
  - the **router**: its stack of pages, by itself, as `g<location>` / `p<location>` lines —
    the pages a `go` made and the ones pushed on top, which is what replaying it needs.
- **Scopes**: what a component builds belongs to the scope the component was made in (the
  scope is captured when the component is constructed, and entered while it builds).
  `restoration_scope(id, build)` makes a component whose values are in the bucket `id` below.
  The router gives every page a scope, `route <index> <location>`, so ids only have to be unique
  within a page, and a page that leaves the stack has its scope forgotten — its values do not
  outlive it.
- **`Restoration`**, one per `Runtime`: whether the application asked for it, what the
  platform handed back and nothing has claimed yet, and every registered value. `data()` reads
  them all and encodes them under the application's scope id; values restored but not claimed
  yet (a page under another, whose fields were never built) are kept as they were. A widget's
  offset or caret, read from the runtime, keeps its last value once the widget is not laid out.
  Scroll offsets and carets restored for widgets are handed to the runtime by
  `Runtime::apply_restored`, which the shell calls right after a build and before the layout:
  the first frame of a restored page is already where it was.
- **Opt-in**: `Application::restoration_scope_id()`, `FrusApp::restoration_scope_id("app")`.
  Without it nothing is registered, claimed or written, and the cost is one branch per hook.

### The platforms (`frus-shell/src/restoration.rs`)

- **Android**: after every frame whose data changed, the bytes are published to a static.
  `FrusRestoration` (new, in the bundled dex) registers activity lifecycle callbacks and, in
  `onActivityPostSaveInstanceState`, puts them in the saved state under
  `android:native_state` — the key NativeActivity reads its native saved state from. On the next
  `onCreate` NativeActivity hands those bytes to the native side, and `android-activity` calls
  the optional `android_on_create` hook, which frus-shell now defines, with them; the shell takes
  them when it starts.
  - *Why after the activity's own save*: winit swallows `MainEvent::SaveState`, so the
    application thread is never asked, and NativeActivity writes the same key itself, from
    native code, with the state it was created with — a stale copy. Only a callback that runs
    after `onSaveInstanceState` returns can have the last word; that callback exists from
    Android 10 (API 29).
  - *Before Android 10* nothing is installed and the application starts afresh, rather than
    from a state that may be stale after the second recreation.
  - The bytes cross a binder transaction; past 256 KB a warning says to keep less.
- **Web**: the tab's `sessionStorage`, one entry per entry of the history
  (`frus-restoration:<index>`, the index the shell already keeps in `history.state`), read
  when the address is read. The router's stack is not restored there: the address and the
  browser's history already say which page is on show.
- **Desktop**: nothing. A desktop does not end applications for their memory.
- **iOS**: a TODO in the module — UIKit state restoration once the shell has a UIKit side.

### The demonstration

`frus-demo` opts in (`restoration_scope_id("demo")`): the task draft
(`use_restorable_text_controller` + the field's `restoration_id`), the tasks section's scroll
and the journal's list.

## How it was tested

- **Unit tests** (`frus-widgets`, `restoration::tests`, 10): the bucket's paths, claiming and
  pruning, scope removal, an encoding round trip with slashes, tabs, line breaks, backslashes,
  empty ids and non-Latin text, malformed bytes rejected, `RestorableValue` round trips, the
  caret and offset formats, nothing registered while off, live values read and gone ones
  dropped, unclaimed values kept, and a scroll offset restored, kept and forgotten.
- **Driver tests** (`frus-shell`, `restoration_tests`, 5): an application with a router is
  driven — two taps on a restorable count, a list dragged, a page pushed, a draft typed, a count
  in a `restoration_scope` raised — its data taken, and a **second application, made afresh**,
  handed it: it opens on `/compose` with the stack `["/", "/compose"]`, the draft, the scope's
  own count, and the caret at 11; back on `/`, the count is 2 and the list is at the offset it
  was dragged to; the popped page's scope is gone from the data. Also: nothing is kept without a
  scope id; another application's data or garbage starts afresh; a `State` with a restoration id
  gets its count back before `init_state`; and counts kept in the keyed rows of a `ListView` built on demand come back to their rows.

- The whole of `frus-widgets`' unit tests (2112, after merging milestone 748) and `frus-shell`'s (421) pass; clippy is clean
  for the workspace, for `frus-shell` on `aarch64-linux-android` and on `wasm32`, and for
  `frus-widgets` with `serde`; `cargo check -p frus-shell --target aarch64-apple-ios` passes.
- **On the Android emulator** (Android 16, API 36, x86_64, `-feature -Vulkan`), the demo as a
  release APK:
  - On the Tasks page, a draft typed (`B` — the emulator's keyboard dropped the rest of
    `adb shell input text`, unrelated to this change) and the page scrolled down. HOME, then
    `adb shell am kill com.frus.demo`: `pidof` empty. Opened again from the recent apps: a new
    process, logcat `android-activity: Creating: … saved state size = 125` then `restoring the
    state the system kept (125 bytes)`, and the page came back scrolled, with `B` in the field.
  - The drawer's Settings page pushed, then the same again: `saved state size = 138`, the
    application opened **on Settings**, and Back led to the Tasks page, still scrolled and with
    its draft. That second round is the one the timing matters for: the activity had been
    created with saved state, so NativeActivity wrote a stale copy of it first, and the state
    written after it won.

## Not done

- **Android before 10**: nothing is kept (see above).
- **Android 10 itself** (API 29, the first level the path needs: both
  `Activity.registerActivityLifecycleCallbacks` and `onActivityPostSaveInstanceState` arrived
  in it) was not run: the emulator here has an Android 16 image only, and the phone was in use.
  Nothing in the path is newer than API 29, and the key NativeActivity reads is the same since
  API 9, but it has to be checked on an Android 10 device.
- **iOS**: not implemented; TODO in `frus-shell/src/restoration.rs`.
- **Other widgets**: `PageView`'s page, a `TabController`'s tab, an `ExpansionTile`'s state,
  `Navigator` used without the router — none has a `restoration_id` yet. An application keeps
  them with `use_restorable_state`.
- **Rows follow their index, not their key**: a list's rows have restoration scopes named
  `row <index>`; a value kept in a keyed row comes back to the row at the same index, which is
  the same row as long as the data is the same after the restart.
- **"Don't keep activities"** could not be exercised: `settings put global
  always_finish_activities 1` took effect once on the emulator and not again. That once, the
  activity's `onDestroy` never returned and the system killed the application as not
  responding (`Force finishing activity`, `Application Not Responding: com.frus.demo`), so no
  saved state survived. This is not this change: `android-activity`'s `onDestroy` waits for
  `android_main` to return, and winit only logs `TODO: forward onDestroy` and keeps its loop
  running. An activity Android destroys while the process lives (the developer setting, or a
  configuration change the manifest does not declare) is a separate bug to fix.
- **`extra` objects** passed with `go_with_extra` / `push_with_extra` are not restorable: the
  page comes back from its location alone.
- **Desktop** keeps nothing, not even optionally to a file.
- **Web** was compiled and its code path reviewed, but not exercised in a browser.
