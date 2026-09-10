# Milestone 506 — An emoji drawn as an empty box on Android

Towards [#56](https://github.com/KalybosPro/frus/issues/56), whose emoji half this is.

Found on the device while checking milestone 504: the demo's guided tour opens on a panel
whose glyph is 👋, and on the phone it was an **empty box**.

## Where the emoji should have come from

`new_font_system` builds every font system in the framework — the measurement's and the
renderer's — as "the system fonts, which provide the emoji and script fallbacks, plus the
bundled faces". The bundled faces are DejaVu and Noto Naskh: they answer for text, and
neither has an emoji.

On Android the first half was **nothing**, for two reasons that compound:

- **fontdb reads no font directory on Android.** Its `load_system_fonts` has branches for
  Windows, macOS and the other Unixes, and the Unix one explicitly leaves Android out. So
  the database held the bundled faces and not one of the platform's.
- **cosmic-text has no fallback list for Android.** Its per-platform lists name the faces
  to try for a character the run's font lacks; the list it uses on Android is empty.

The bundled faces were already there because of the second point — the Arabic face is
bundled, and routed to by script, for exactly this reason. An emoji had no such route.

## Only the emoji faces

The platform's emoji faces are now loaded explicitly: every font file in `/system/fonts`
whose name says `emoji` — `NotoColorEmoji.ttf` on stock Android, and a manufacturer's own
under the same kind of name — and nothing else from that directory.

- **Not the whole directory.** It is hundreds of files, and the bundled faces already
  answer for text; taking the platform's text faces as well would make an application's
  text depend on the phone it runs on, which is what bundling was for.
- **Not a bundled emoji font.** Colour emoji are the platform's own look, and its largest
  font — ten megabytes and more. The files are mapped, not read.
- **No fallback list needed.** cosmic-text's last pass tries every face in the database,
  and a face whose name says `Emoji` is a candidate **whatever style was asked for**. So an
  emoji inside bold or italic text is found too, where a text face would have needed an
  exact style match.

A missing directory, or one with no emoji file in it, loads nothing and costs nothing.

## Verification

- A test of the loader against a scratch directory: a file named for emoji, a text face,
  and a non-font file named for emoji — each holding a perfectly good face, so that a
  filter letting one through shows as a second face. One file taken, one face loaded; a
  missing directory takes nothing.
- A test of the shaping, in the arrangement Android is left with — the bundled face, no
  system font, the emoji face loaded on its own: the glyph for 👋 comes from the emoji face
  and is a real glyph. It needs a colour emoji font on the machine running it, and Windows
  ships one; elsewhere it says so and passes.
- `frus-text` checked for `aarch64-linux-android`, where the loading is compiled.
- Two mutations, each failing the loader's test: taking every font file, and taking a file
  whatever its extension.
- **On the device** — Huawei STK-L21, Android 10 — once it was back on the wire: the
  guided tour opens on the wave **in colour, at the size asked for**, centred over its
  title. That settles the one thing no test here could: the platform's emoji font is made
  of colour bitmaps, not outlines, and the renderer scales and draws them.

## What stays open in #56

Bidirectional runs, complex scripts, per-character versus per-run fallback for text, the
emoji sequences that are one grapheme and several codepoints, and grapheme-aware editing.
