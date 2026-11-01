# Milestone 730 — Permissions

## Objective

An application that scans a code, records a voice note or shows the nearest shop needs the
camera, the microphone or the location, and those are the person's to give. frus asked for one
permission only — notifications, inside `SystemNotification::show` (milestones 703, 704) — and
had no way to ask for the others, to know where they stood, or to send the person to the
settings once they had said no for good. The milestone 718 guide page said so.

## What was done

- **`Permission::{Camera, Microphone, Location, Notifications}`**, with `.status().await`,
  which asks nothing, and `.request().await`, which asks when there is something to ask; a
  **`PermissionStatus`** of `Granted`, `Ask` (not decided), `Denied` (refused, may ask again),
  `Blocked` (only the settings can change it) or `Unavailable`; and
  **`open_permission_settings()`**. Both calls are `async` because the web answers no other way;
  natively a request waits on a thread made for waiting (`blocking::unblock`), never on the
  interface's. Exported by `frus`, with no feature to turn on.
- **Android** (`android_permission.rs`, and `FrusPermissions` in the bundled dex, rebuilt). The
  status is `checkSelfPermission`, told apart by `shouldShowRequestPermissionRationale` and by
  whether this application asked before (kept in its preferences): not asked → `Ask`, refused
  once → `Denied`, refused for good → `Blocked`, missing from the manifest → `Blocked` with a
  log line naming the permission to add. A request is `requestPermissions`, and — as a
  NativeActivity is not told the answer — waits the way notifications do: a yes read from the
  permission, a no from the dialog going away. That waiting logic (`question`) moved from
  `notification.rs` to `permission.rs`, shared by both. The location is granted by **either** of
  its two permissions (below).
- **The web** (`web_permission.rs`): the status is `navigator.permissions.query`; a request is
  what the browser asks on — the camera or the microphone opened for a moment and let go, the
  position read once, `Notification.requestPermission`. A page refused once is `Blocked` (only
  the site's settings change it); camera, microphone and location are `Unavailable` on a page
  that is not secure. Reached through `js_sys::Reflect`, as browsers disagree on what they
  support (Firefox will not query the camera; it reads as `Ask`).
- **Windows** asks no desktop application: the status reads the privacy settings in the
  registry (the machine's policy, the user's switch, the desktop applications' switch) —
  `Blocked` when any says `Deny`. `open_permission_settings` opens `ms-settings:privacy`.
- **Linux**: `Granted`, nothing to ask. **macOS**: the system asks itself on first use (given the
  bundle's usage strings); frus does not ask ahead yet, so the status is `Ask`, documented.
- **The widget gallery** has a *Permission* demo — each permission looked at or asked for, and
  the settings — and its Android manifest names the five permissions.
- **The guide**'s platform page has a *Permissions* section with an example and the five
  statuses.

## Checked on a device and in a browser

On an Android 16 emulator, through the gallery's demo:

- the camera asked for → the system's dialog → allowed → `Granted`;
- the microphone refused once → `Denied`; refused again → `Blocked` (Android 11+ stops asking);
- notifications asked for (Android 13+) → `Granted`;
- *Open the settings* opened the application's *App info*; coming back, the application was
  alive with its state;
- **the location allowed as *approximate* only read as `Blocked`**: the first version required
  both location permissions, and from Android 12 the person may grant only the approximate one.
  Now either grants the location, and one the manifest leaves out does not count against the
  other (`combine`, tested); rebuilt and checked again on the emulator: `Granted`.

In headless Edge, `scripts/web-permissions-check.py` drives the same demo with a fake camera and
microphone and the DevTools protocol granting or refusing: the location not decided at first;
the camera and the microphone, opened for a moment, granted; the location granted once the
browser grants it; notifications refused by the browser read as blocked. All checks pass.

The emulator's emulated Vulkan hung wgpu's device creation after a restart (a black window,
nothing logged past the Vulkan instance); with `-feature -Vulkan` frus ran on its GLES path and
the checks went on. That is the emulator's, not this milestone's.

## Tests

- `permission.rs`: the Android codes as statuses; the location granted by either of its
  permissions, a missing one not counting; the camera needing all of its own; the browser's
  states; the Windows privacy values; the dialog question, moved here from `notification.rs`.

## Not done

- **macOS** does not ask ahead of first use (`AVCaptureDevice.requestAccess` and
  `CLLocationManager`); its status is `Ask` until it does.
- **iOS**: `Unavailable`, as the platform itself is not built yet.
- **More permissions** — contacts, photos, Bluetooth, background location — follow the same
  shape when an application needs them.
