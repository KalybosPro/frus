# Milestone 698 — The system's file dialogs

## Objective

The second part of #219. A desktop application asks for a file through the system's own dialog
— open one, open several, choose a folder, choose where to save — and frus had no way to show
one: an application had to find a crate, learn which thread it may be called from on each
system, and work out how to tie the dialog to a window it was never given a handle to.

## What was done

- **`FileDialog`** (`frus-shell`, re-exported as `frus::FileDialog`), behind a new
  **`file-dialogs`** feature, off by default like `net`, `svg` and `lottie`:

  ```rust
  Command::perform(|| {
      Msg::Opened(
          FileDialog::new()
              .title("Open a picture")
              .filter("Images", &["png", "jpg", "jpeg"])
              .pick_file(),
      )
  })
  ```

  - A description — `title`, `directory`, `file_name` (what a save offers), `filter(name,
    extensions)`, one call per choice — and four ways to show it: `pick_file() ->
    Option<PathBuf>`, `pick_files() -> Vec<PathBuf>`, `pick_folder() -> Option<PathBuf>`,
    `save_file() -> Option<PathBuf>`. A cancel is `None`, or no paths.
  - Extensions are written bare; a leading `.` or `*.` — the two usual slips — is taken off, and
    an empty one dropped, rather than handed to a system that would then match nothing.
- **Blocking calls, made where waiting belongs.** The dialog waits for a person, so the four
  calls block, and are made from a background thread: `Command::perform` in an application,
  `frus_widgets::host::spawn` in a component. The alternatives were a `Command` per dialog
  (`Command::pick_file(dialog, Msg::Opened)`), which components cannot use — they have no
  `Command` — or `async` functions, which `spawn` cannot await. A blocking call fits both, in one
  line, through paths that already exist.
- **Refused on the interface's thread.** Asked for there, a dialog returns as a cancel and logs
  an error naming where to call it from. Shown there it would stop every frame until it closed —
  and on macOS, where the system runs the dialog on that very thread, the call would wait for a
  thread it is blocking, and never return. The shell records which thread is the interface's
  when it makes the window.
- **Owned by the application's window.** The shell keeps the window's raw handles when it makes
  it, and every dialog is given them as its parent: on top of the window and modal to it on
  Windows, a sheet attached to it on macOS, the portal's parent window on Linux. They are kept
  as raw handles because a background thread may not ask winit for them (winit refuses off the
  main thread on macOS); `Parent` lends them back through `HasWindowHandle`.
- **rfd 0.17.2** underneath — its asynchronous dialog, driven to its end on the calling thread
  (`pollster`). That is the form rfd hands to the main thread itself on macOS, and on Linux it
  talks to the **desktop portal** over D-Bus — the dialog sandboxed applications use, with no
  GTK to build against, so CI needs no new system package. Only in the desktop dependency table:
  Android, iOS and the web never build it.
- **The demo** (#219's "dropping a file on the demo's window shows its name"): a task's screen
  takes files dropped onto it — the J695 `on_files`, the whole body lighting up — and has an
  **Attach…** action that opens the dialog for several files. The names are listed under the
  task's state; the demo keeps names only, in memory, and reads nothing from the files.
- In passing: `textinput.rs` had a boolean clippy 1.96 asks to be written out
  (`nonminimal_bool`); the new stable toolchain fails on it with `-D warnings`.

## Left

- **The web's file picker.** A page cannot hand out paths: the browser gives a file's name and
  its bytes. Behind "the same API" that means a picked file that is a handle (a name and a way to
  read it) rather than a `PathBuf`, on every platform — a change to the four calls' result,
  better made with Android's picker (content URIs, also not paths) in view.
- **Android and iOS** have no dialog yet: Android's is the Storage Access Framework, through an
  activity result.
- **Run for real on Windows only.** The demo was started on a task's screen (Windows 11, 150 %),
  "Attach…" clicked, the system's dialog came up **owned by the demo's window** (`GetWindow(…,
  GW_OWNER)` is the demo's), a file was chosen, and the screen then listed its name (checked on
  a screenshot). On macOS and Linux the code type-checks (`clippy --target
  x86_64-apple-darwin` / `x86_64-unknown-linux-gnu`); how the dialog behaves there is rfd's, and
  on Linux it needs a desktop portal running (any GNOME or KDE desktop has one).
- **Found on the way, for the accessibility tree** (to be fixed on their own):
  - the bounds handed to AccessKit are **logical** pixels, and the system reads physical ones:
    at 150 % every node is reported two thirds of its size and off its place, which is where a
    screen reader's highlight and a magnifier go;
  - a `Container` with `on_click` has **no node at all**, so the demo's bottom-bar actions
    ("Delete", "Attach…", "Back") cannot be reached with a screen reader — the reference gives a
    tappable area a node with its tap action — and plain text is not in the tree either.
- **A frame seen once with two versions of the task's screen.** The first version of the
  demo's screen test attached the files with the screen already open; in one run out of about
  fifteen (a full run of three crates, right after a build), the frame read back carried the
  new names *and* the "drop files here" hint, which one build of the screen cannot produce
  together. It did not come back in twelve more runs, and the cause was not found; the test now
  opens the screen after attaching. A lead for whoever next sees a stale subtree in a frame.
- The rest of #219: system notifications, the tray icon, taskbar badges and progress, global
  shortcuts.

## Mutation testing

Seven hand mutants, each killed by a test:

| mutant | killed by |
|---|---|
| a filter's extensions kept with their `.` or `*.` | `a_filter_takes_the_dot_off` |
| an empty extension kept | the same |
| any thread counted as the interface's once one is known | `the_interface_thread_is_refused` |
| a name attached twice | `attachments_are_named_once_and_go_with_their_task` |
| a deleted task's attachments kept | the same |
| the whole path shown instead of the file's name | the same |
| the "drop files here" hint kept once files are attached | `a_task_screen_names_its_attached_files` |

Not mutated: turning the interface-thread guard off. The test would then open a real dialog
on the machine running it and wait for a person.
