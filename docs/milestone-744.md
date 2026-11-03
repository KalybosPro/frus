# Milestone 744 — Permissions declared once

## Objective

Milestone 733 made macOS ask for the camera, the microphone and the location, and left the usage
strings to the application: macOS ends a process that asks without the `Info.plist` key saying
why, so frus refuses to ask when it is missing. But `frus build` and `frus run` made no bundle on
a Mac, so there was no `Info.plist` to put the keys in, and Android's permissions were declared
apart, in cargo-apk's own tables. An application had to say the same thing twice, in two formats,
and on macOS had nowhere to say it.

## What was done

- **One table**, read by the CLI from the application's `Cargo.toml`, next to the
  `[[package.metadata.frus.devices]]` it already reads:

  ```toml
  [package.metadata.frus.permissions]
  camera = "Scans the code on your ticket."
  microphone = "Records your voice notes."
  location = "Shows the shops near you."
  notifications = true
  ```

  The keys are the shell's `Permission`s. The text is the reason the person reads in the
  system's question. Notifications show none, so `true` declares them; `false` declares nothing.
- **Clear errors before anything is built** (`permissions.rs`): a camera, microphone or location
  without a reason (`true`, or empty text) names the key and what to write; an unknown key lists
  the four known; a key declared twice, a multi-line string, an unclosed string, an unknown escape
  and a control character an `Info.plist` cannot carry are each said. Every `frus build` and
  `frus run` checks, on every platform, so a declaration a Mac would end the application over is
  found on the machine that wrote it. The values are read as TOML strings: basic ones with their
  escapes (`\"`, `\\`, `\n`, `\uXXXX`…), literal ones, and a comment after.
- **macOS** (`bundle.rs`): `frus build` makes `name.app` beside the program, and `frus run`
  starts each run's copy from a bundle of its own, so `NSBundle.mainBundle` is the bundle and
  its `Info.plist` is read. The plist holds the bundle's identity (the identifier is the Android
  package, with `_` made `-`; the version is the package's), `NSHighResolutionCapable`, and the
  usage strings declared: `NSCameraUsageDescription`, `NSMicrophoneUsageDescription`,
  `NSLocationWhenInUseUsageDescription` with the older `NSLocationUsageDescription`. Every value
  is XML-escaped. The bundle is signed ad hoc with `codesign`, which every Mac has; a failure is
  a warning.
- **Android**: each permission the declarations need and the manifest does not name yet is
  appended to `Cargo.toml` as a `[[package.metadata.android.uses_permission]]` before cargo-apk
  runs, under a comment saying where it came from, in the file's own line endings. What is there
  stays; a second build adds nothing. An inline `uses_permission = [...]` cannot be added to by
  tables, so what it lacks is an error naming the permission.
- **The building is only files**: the plist, the bundle's layout and the Android additions are
  made by code that runs, and is tested, on Windows and Linux too. Only the ad-hoc signature
  waits for a Mac.
- The `frus create` template shows the table, commented out, with what it writes; its README has
  a Permissions section. The guide's Permissions section says how to declare them.
- The shell's warning on macOS, when a usage string is missing, now says where to declare it.

## Tests

- `permissions.rs`: the table read whole, reasons with both string kinds, escapes and comments;
  every error above; XML escaping of `& < > " '`; the Android additions (only what is missing,
  CRLF kept, an inline list completed by hand) and their writing to a `Cargo.toml`, once.
- `bundle.rs`: the `Info.plist` holds each entry, escaped, and no usage string for what is not
  declared; `make_app` lays out `Name.app/Contents/{MacOS/program, Info.plist, PkgInfo}` around a
  program, and making it again replaces the old one.
- `project.rs`: the table is read from a manifest, and an error names the `Cargo.toml`.
- `create.rs`: the template's table, uncommented as written, gives four permissions, a plist
  with the reasons, and the five Android permissions.

## Not checked

**This has not run on a Mac.** The bundle is made by code tested on Windows, but no bundle has
been opened by macOS: not the signature, not the question shown with the reason, not whether a
program started by `frus run` from a terminal is asked about as itself or as the terminal.

## Not done

- **iOS**: frus does not build for iOS yet. The keys are the same, and `usage_strings` gives
  them; the iOS build will write them into its `Info.plist`.
- **`frus run --hot`** builds through the hot-reload engine, which makes its own bundle and its
  own Android manifest: neither gets the declared permissions yet.
- The bundle has no icon (`.icns`) and no category; `LSMinimumSystemVersion` is fixed at 10.15.
- Permissions beyond the shell's four (contacts, photos, Bluetooth…) are not in the table:
  frus does not ask for them.
