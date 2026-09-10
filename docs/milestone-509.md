# Milestone 509 — The clipboard on Android

Answers [#22](https://github.com/KalybosPro/frus/issues/22).

The shell's clipboard was `arboard` on the desktop and an empty struct everywhere else.
On Android `get_text` returned `None` and `set_text` dropped what it was handed: a copy
did not happen, a paste found nothing, and nothing failed or logged. The platform table in
the roadmap said ✅ all the same.

## The platform's clipboard, behind the same interface

`android.content.ClipboardManager`, reached the way the system bars are (milestone 503): a
third class in the bridge's dex, `FrusClipboard`, loaded through the same in-memory class
loader and **on its own terms** — a failure to load it costs the clipboard, not the
keyboard. The shell's `clip::Clipboard` keeps the interface the desktop satisfies, so no
caller changed; the stub remains for iOS and the web, gated on what is *not* implemented so
that a new target is never left without the type.

- **The read is synchronous**, because a paste wants its text now, and it needs no hop to
  the UI thread: the clipboard is a binder call, and the manager is built on the main
  thread's handler whichever thread asks for it.
- **Nothing, or nothing that reads as text, is `None`.** An empty clipboard, no clip, an
  item with no text: the Java side answers `null` and the Rust side reads that as `None`,
  not as an error. A clip that is not plain text — a link, a URI — is asked for its text
  form, which is what a paste into a text field means.
- **Nothing on the path can panic**, and every Java exception is cleared: one left pending
  takes the process down on the next JNI call anybody makes.
- **Android 12's toast** when an application reads the clipboard is left alone — it is the
  platform telling the user something true.

## The keys a keyboard has for it

The shortcuts were Ctrl+C/X/V, matched on the logical key. A keyboard may also have keys of
its own for the same thing, and Android has keycodes for them — which winit reports **by
physical code only**, with no logical key at all. A check of the logical key alone ignored
them on the one platform where they are a keycode of their own.

So what a key asks of the clipboard is decided in one function, `clipboard_command`, from
both halves of the key: the named Copy, Cut and Paste keys a desktop keyboard reports,
Ctrl with C, X or V in either case, and the physical Copy, Cut and Paste codes. Being a
function of the key alone, it is tested without a window.

## Verification

- Two tests on `clipboard_command`: Ctrl+C/X/V in either case are the clipboard and the
  same letters without Ctrl are only letters; the physical Copy, Cut and Paste codes are
  the clipboard with no logical key and no modifier, and so is the named Paste key.
- Three mutations, each failing the test meant for it: the physical keys ignored, a letter
  taken for the clipboard without Ctrl, the named Paste key ignored.
- `frus-shell` checked and linted for `aarch64-linux-android`; the dex rebuilt with the new
  class; the APK built.
- **On the device** — Huawei STK-L21, Android 10, once it was back on the wire. A word
  typed into the home screen's field, selected by a finger dragged across it, copied with
  Android's own **Copy** key (`KEYCODE_COPY`, sent by `adb shell input keyevent 278`);
  the application **force-stopped and started again**; the field tapped and Android's
  **Paste** key sent: the word came back. Only the system clipboard survives the process
  being killed, so this is the round trip through it — and the physical keys, which the
  logical-key check used to ignore, driving both ends. Before pasting, the log was read
  for the bridge's failure lines (none) and the process's own log checked to reach
  logcat at all, so that a failed copy could not have put someone else's clipboard on the
  screen.

  The issue also asks for text copied in *another application*. That was not done on a
  phone that is somebody's own; the restart is the same test of the same thing — the
  text living in the platform's clipboard rather than in the process.

## Found on the way

Typing `frusclip` into the field through `adb shell input text`, with SwiftKey as the
keyboard, left **`FFFrusclip`** in the field while the keyboard's own suggestion strip read
`FFrusclip`: the field holds one character more than the keyboard believes it wrote. That
may be the keyboard re-composing injected key events, or the field applying a composition
twice. Either way it is a disagreement between the two, and it is its own investigation.
