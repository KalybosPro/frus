# Milestone 733 — Permissions on macOS

## Objective

Milestone 730 left macOS out: the status of the camera, the microphone and the location was `Ask`
whatever the person had answered, and a request asked nothing. An application could not tell a
refusal from a question not yet put, nor send the person to the settings at the right moment.

## What was done

- **`macos_permission.rs`**: the status of the camera and the microphone is
  `AVCaptureDevice.authorizationStatusForMediaType:`, and that of the location
  `CLLocationManager.authorizationStatus` with `locationServicesEnabled`. A request is
  `requestAccessForMediaType:completionHandler:`, or `requestWhenInUseAuthorization` on the main
  thread, where a location manager must be made for its question to show.
- **A refusal is `Blocked`**: macOS does not ask twice, and only the settings change it. So is a
  restriction by the machine's policy, and the location with location services off.
- **Nothing is asked without a usage string.** macOS ends a process that asks for one of these
  when its `Info.plist` does not say why, with no error to catch. frus looks in the bundle first:
  the status is `Blocked` and the log names the key. A program run outside a bundle falls under
  this too.
- **The answer is read from the status**, polled from a thread made for waiting while it is "not
  determined", for five minutes at most; someone who walked away leaves it `Ask`.
- **Notifications stay `Granted`**: the notification centre asks by itself the first time one is
  shown, and frus does not read its answer yet.
- `objc2`, `block2` and `objc2-foundation` are no longer behind the `taskbar` feature on macOS. They
  were already in every macOS build through the windowing crate.

## Tests

- In `permission.rs`, on every platform: `AVAuthorizationStatus` and `CLAuthorizationStatus` as
  statuses.
- In `macos_permission.rs`, on CI's macOS runner: the classes are found, the messages are sent for
  real (objc2 checks their signatures in a debug build), no usage string is found in a test's
  program, and a request answers without asking. Had it asked, macOS would have ended the test
  process.

## Not checked

**The system's question has never been shown.** There was no Mac to run this on, and a test's
program has no bundle, so the asking path is compiled and linted but has not run: not the camera's
request, not the location's dispatch to the main thread, not the wait for the answer. It should
be tried in a bundled application before it is relied on.

## Not done

- **Notifications** do not report the person's answer on macOS.
- **`frus build` does not write the usage strings**: an application adds them to its `Info.plist`
  itself.
- `CLLocationManager.authorizationStatus` as a class method is deprecated since macOS 11. It still
  answers, and the instance property it was replaced by would need a manager for every read.
