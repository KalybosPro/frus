# Milestone 762 — Restored rows follow their key; restoration before Android 10

## Objective

State restoration (milestones 753 and 756) left three things in its "Not done":

1. **Rows follow their index, not their key.** A list's rows had restoration scopes named
   `row <index>`. A value kept in a keyed row came back to whatever row had the same index in
   the next process: the right one only if the data was in the same order.
2. **Android before 10 kept nothing.** The path wrote the bytes in
   `onActivityPostSaveInstanceState`, which exists from Android 10 (API 29). frus supports
   Android 7 (API 24) and later.
3. **A page view's fractional position** was not kept.

## 1. Rows and cells follow their key

### Root cause

`ListView::new` built every row inside `scope_below(list scope, "row <index>")`. The key a
row is given (`keyed(..)`, `Keyed::new`, `Component::with_key`) was never looked at for
restoration, although the row's *state* has followed that key since milestone 748. So:

- after a restart with the data in another order, the values went to the row at the old
  index — another item;
- worse, within one process, a keyed row that moved kept the state it was made with (its
  component follows its key) but its restorable values stayed registered under the index it
  was made at; the row that later took that index registered under the same path, and what was
  saved for both was whichever wrote last.

The cells of a `GridView::builder` (and `GridView::extent`) had no scope at all: they are built
during the layout, outside any component's scope, so a restorable value in a cell was kept at
the root of the application's bucket, and the same id in two cells was one value.

### Design

`restoration::ItemScopes` (crate-private) builds the items of a list or a grid built on demand,
each in a bucket of its own below the list's (or grid's) scope:

- an item with a key is kept under `key <16 hex digits>` — the key as frus reduces it
  (`Widget::key`, the hash `ChildKey` also uses);
- an item without one under `row <index>` (a list) or `cell <index>` (a grid), as before for
  lists.

The key is only known once the item is built, and what the item holds takes its scope when it
is *made* (a component captures the scope at construction; `TextField::restoration_id` and
nested lists compute their path then). So the scope is **guessed** from the key the same index
had the last time it was built; when the built item's key is not the guess — the first time an
index is built, or after the data moved — the item is dropped and made again in the right scope,
and the guess is updated. Making an item is cheap (building, laying out and running hooks all
happen later, and only for the item that is kept), and rows without a key are made once, as
before. The guess lives in the list value, which is made anew by each rebuild of the page, so a
keyed row is made twice on the first frame after a rebuild that reaches it; an index whose key
did not change is made once afterwards.

A bucket per item, named by an id the item carries, is the usual shape for restoring the
items of a list; here the key frus rows already carry is that id, so nothing new has to be given
to a row.

*Considered*: making a scope a shared cell that is renamed once the key is known — every
restorable value computes its path at construction from the scope, so each would have to hold
the cell and read it late; a larger change for the same result. Asking the application for an
id-per-index function — the key already is that, and `find_child_index` already maps it back.

The key is a hash of the application's key value. It is the same in the next process of the
same build; saved state does not survive an update of the application anyway.

## 2. Restoration before Android 10

### Why it needed API 29

NativeActivity's `onSaveInstanceState` calls `super`, then asks the native side for its state
(the `onSaveInstanceState` callback of `ANativeActivityCallbacks`) and stores the answer under
`android:native_state`, the key the next `onCreate` reads. android-activity answers that callback
by sending `SaveState` to the application's thread and returning whatever the application stored
— winit ignores the event (`TODO: forward saveState notification to application`), so the answer
is the state the activity was **created** with. Milestone 753 therefore wrote its bytes *after*
NativeActivity, in `onActivityPostSaveInstanceState`, which only exists from Android 10.

### Design

The answer itself is replaced. The native callback exists on every Android version (since
API 9), and NativeActivity copies and frees whatever it returns.

- `frus_native_activity_on_create` (frus-shell, exported) calls android-activity's
  `ANativeActivity_onCreate`, then puts frus's own function in the activity's
  `onSaveInstanceState` slot, keeping android-activity's.
- That function returns a `malloc`ed copy of the bytes the last frame published — or nothing,
  so NativeActivity writes no state rather than a stale one. When the application did not ask
  for restoration, it calls android-activity's callback, as before.
- NativeActivity calls the function the manifest names in `android.app.func_name` (default
  `ANativeActivity_onCreate`), so an application opts in with three lines:

  ```toml
  [[package.metadata.android.application.activity.meta_data]]
  name = "android.app.func_name"
  value = "frus_native_activity_on_create"
  ```

  The demonstration, the four sample crates and the nine example projects declare it.
- With the line, the shell installs nothing else; without it (an application made before this
  milestone, the hot-reload project of `frus run`), the Android 10 path of milestone 753 is
  used, and before Android 10 the log says which line is missing.

So one mechanism serves every version for an application that declares the entry, and the
callback runs on the Java main thread without waiting for the application's thread (android-
activity's own answer blocks the main thread until the application's loop has handled
`SaveState`).

*Considered*: reading `NativeActivity.mNativeHandle` by reflection to reach the callback table
without a manifest line — a private field, subject to the non-SDK interface restrictions from
Android 9; `Application.registerActivityLifecycleCallbacks`' `onActivitySaveInstanceState` (API
14) — it runs inside `super.onSaveInstanceState`, before NativeActivity writes the key, so the
stale copy would still win after the second restart; a fragment that saves state — restored by
class name through the application's class loader, which does not hold frus's bundled dex.

**The template** (`frus create`) is not changed: it depends on the published `frus = "0.5"`,
which does not define the function, and NativeActivity cannot start an activity whose entry is
missing. The line goes into the template with the release that ships the function.

## 3. A page view's fractional position

What a paged view should keep is where it rests, **in pages** (a fraction when between two),
as of the last scroll that ended — not a pixel offset, and not a position mid-gesture. So:

- a view that snaps ends every scroll on a page: the page is kept. A process ended mid-swipe
  comes back on the page of the last scroll that ended — frus keeps the page on show, which
  at rest is the same page. Unchanged;
- a view that does **not** snap (`page_snapping(false)`) rests between pages, and that
  fraction is where the reader left it. frus kept the rounded page; it now keeps the fraction too
  (`Runtime::page_position`, updated where page changes are read; written only for views that
  do not snap), and opens on it at the new page extent.

## How it was tested

(filled in below)

## Not done
