# Milestone 671 — The daemon's emulators, platforms and messages

## Objective

`frus daemon` answered for the devices only: the list, and the events as they come and go. The
reference's daemon, which its editors are built on, also starts and makes emulators, says which
platforms a project builds for, takes a device's screenshot, and passes messages for the
developer. Its VS Code picker offers "Start …" for each emulator and "Create …". This milestone
brings frus's daemon and extension to the same place.

## What was done

- **`emulator.getEmulators`**: every Android AVD (and the iOS simulators on a Mac), as `id`,
  `name`, `category`, `platformType`.
- **`emulator.launch`** (`emulatorId`, `coldBoot`): starts it and answers at once; it arrives as
  `device.added` when it is up. It is started from the emulator's own folder: it outlives the
  daemon and must hold no one else's directory.
- **`emulator.create`** (`name`): makes an AVD, installing what it needs first (the SDK's tools,
  the emulator, a system image if none is installed; the newest installed one otherwise, the
  plain Google APIs one before the Play Store one at the same level). Answered as the reference
  does: never an error, `{success, emulatorName, error}`.
- **`daemon.getSupportedPlatforms`** (`projectRoot`): `platforms`, and for each platform
  whether the application builds for it and, if not, why and what to do — a desktop program, the
  Android sections of the manifest, `web/index.html`; iOS, until its shell exists.
- **`device.takeScreenshot`** (`deviceId`): an Android device's screen, a PNG in base64. Elsewhere
  it says why not (`s` in `frus run` takes the application's own).
- **`daemon.logMessage`** for what the tool does on the way (installing, creating, starting), and
  **`daemon.showMessage`** for what the developer must see now. In the daemon, every message of
  the tool's goes out as one of these: standard output stays JSON only.
- **The extension**:
  - the picker lists the devices, then an **Emulators** section — "Start …" for each one not
    running, "Create an Android emulator";
  - an emulator started from it is **selected once it is up** (matched by name, which a running
    emulator takes from its AVD);
  - a device whose platform the application does not build for says why;
  - **frus: Launch Emulator** and **frus: Create Emulator** in the palette;
  - the daemon's messages in a **frus** output channel, and the ones to show as notifications;
  - the daemon client answers each request with its own reply (`request`), and fails the ones
    waiting when the daemon stops.

## Verified

- Unit tests: base64; the platforms of an application (with and without `web/`); a free emulator
  name; the newest system image chosen; the extension's client (answers matched out of order, an
  error answer, messages and device events handed on).
- The real daemon on this machine: `getSupportedPlatforms` on the counter example (windows, macos,
  linux, android, web; iOS with its reason); `emulator.create` made `frus_test_emulator` from
  the installed Android 16 image, saying so as a `daemon.logMessage`; `getEmulators` listed it;
  `launch` started it cold, and it arrived as `device.added` "frus test emulator", Android 16;
  `takeScreenshot` returned its home screen; on the desktop it said why not.
- VS Code 1.142, isolated, looked at: the picker with the devices, "Start frus test emulator" and
  "Create an Android emulator"; after **frus: Launch Emulator**, the status bar showed "frus test
  emulator" by itself, with the notification that it is running and selected. Then F5, the log,
  a reload, debug paint and stopping, as before.
- The test emulator was deleted afterwards.

## Left

- Creating an emulator when no system image is installed downloads one (about 1.5 GB): not run
  here, where one was installed.
- iOS simulators are listed and booted on a Mac; not tried.
