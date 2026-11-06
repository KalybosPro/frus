# Milestone 755: an Android activity that ends no longer hangs the application

## What

An Android activity can be destroyed while its process lives: the developer setting "Don't keep
activities", Back at the root, the system finishing it, or a configuration change the manifest
does not declare. Until now a frus application froze at that moment and could not be opened
again until Android killed it as not responding. Now:

- when the activity is destroyed, the event loop ends, the application gets its `Detached`
  notification, and the process ends; the next launch starts a new process, from the state the
  system kept when the application asked for restoration (milestone 753);
- Back with nothing left to go back from does what the platform does: `Activity.onBackPressed()`,
  which finishes the activity (from Android 12, a root activity's task goes to the background
  instead). The process no longer ends under an activity the system still counts as running;
- the manifests (template, demo, sample crates, the eight example projects) declare the
  configuration changes an application follows in place, so a new font size, the night setting,
  a new language or a keyboard no longer recreate the activity; the shell reads the platform's
  settings again when one of them arrives.

## Why it hung: the root cause

`android-activity` (the glue winit runs on) implements `NativeActivity`'s `onDestroy` by sending
`Destroy` to the application's thread and **blocking the Java main thread until `android_main`
returns**. winit 0.30 receives that `Destroy`, logs `TODO: forward onDestroy notification to
application`, and keeps its loop running. Nothing ever returned from `android_main`: the main
thread stayed blocked in `onDestroy`, the next launch was queued behind it (a blank window on the
device), and the system eventually reported the application as not responding.

Returning from `android_main` is not enough on its own. winit builds **one event loop per
process** (`EventLoopBuilder::build` fails with `RecreationAttempt` the second time, a static that
only the web may reset), and `android-activity` starts a new `android_main` for every new activity
in the same process. So a frus application cannot live on in its process once its activity is
gone: the process has to end, and the next activity has to start in a new one. That is what
`run_android` already did after the loop; the loop just never ended.

## Design

- **Knowing the activity is going** (`frus-shell/src/android_lifecycle.rs`). winit swallows the
  event, so the shell asks the activity: `Activity.isDestroyed()` is set when the destruction
  begins, before `onDestroy` runs, on every API level frus supports. The native activity takes
  its surface away before it destroys anything (`TerminateWindow` always precedes `Destroy`), so
  the question is only asked in `about_to_wait` while there is no window: no JNI call per frame.
  When the answer is yes, the loop exits; `run_android` logs it and ends the process while
  `onDestroy` is still waiting.
  - *Considered*: an `Application.ActivityLifecycleCallbacks` in the bundled dex with
    `onActivityPreDestroyed` — Android 10 and later only, and a Java class for one boolean;
    wrapping `ANativeActivity`'s `onDestroy` callback — the pointer is not reachable from
    `android_on_create`, which runs before `android-activity` installs its callbacks; reading
    winit's log line — depends on the application keeping frus's logger.
  - *Why the process ends inside `onDestroy`*: the system already holds what the activity saved
    (it is reported when the activity stops), so a destroyed activity that is not finishing keeps
    its record and its saved state; a finishing one is removed either way. Ending after
    `onDestroy` returns would need a callback the process can still run once `android_main` has
    returned and `android-activity` has dropped what it holds.
- **Back at the root** calls `Activity.onBackPressed()` on the Java main thread
  (`AndroidApp::run_on_java_main_thread`), falling back to `finish()` if it throws. The activity
  then goes through its normal end, and the path above ends the loop.
- **Configuration changes** are declared in place (`config_changes` in each manifest's
  `[package.metadata.android.application.activity]`): `orientation|keyboardHidden|keyboard|
  screenSize|smallestScreenSize|locale|layoutDirection|fontScale|screenLayout|density|uiMode`.
  cargo-apk's default declared only the first, second and fourth. winit reports every
  configuration change as `ScaleFactorChanged`; the shell now reads the platform settings again
  there on Android (`refresh_platform_settings`, which marks the build dirty when the text
  scale, the brightness or the locales moved).

## How it was tested

- Unit test `app::tests::the_loop_ends_with_the_activity_and_only_asks_without_a_window`: the
  decision (`activity_ended`) ends the loop only without a window, and never makes the JNI
  question while there is one.
- On the phone (Huawei STK-L21, Android 10), the demo as a release APK, master first, then this
  branch:
  - **"Don't keep activities" on, master**: a draft typed, HOME, the launcher icon again: the
    same process, the activity record `RESUMED` but the screen blank (the splash colour) and
    nothing drawn — the main thread was stuck in `onDestroy`.
  - **Same, this branch**: HOME → logcat `the activity is being destroyed: the loop ends`, `the
    activity is gone: the process ends`, `Process com.frus.demo has died`; the activity record
    kept (`app=null`). Launcher icon → a new process, `android-activity: Creating: … saved state
    size = 105`, `restoring the state the system kept (105 bytes)`, and the Tasks page came back
    with `Draft755` in the field.
  - **Back at the root**: the same two log lines, no activity record left for the application
    (`dumpsys activity activities`), and the launcher icon starts a new process with `saved
    state size = 0` — a fresh start, as after any finished activity.
  - **`am kill` in the background** (milestone 753's path, setting off): HOME, `am kill`,
    reopened: a new process restoring 104 bytes, `Kill755` in the field.
  - **A font-size change with the application in front**, before the manifest change: the system
    relaunched the activity (`Config is relaunching resumed ActivityRecord`), the loop ended and
    the process with it, and the system removed the activity (`app died, no saved state`): the
    application closed instead of hanging. With the manifest change, the font size set to 1.3 and back to 1.0
    with a draft in the field and the keyboard open: the same process throughout, no relaunch,
    the text grew and shrank in place, and the draft and the keyboard stayed.
  - **The final build, all three paths again** (the setting switched in Developer options, not
    with `settings put`, which the activity manager does not notice on this phone):
    "Don't keep activities" on, `Dka755` typed, HOME → `the loop ends`, `the process ends`,
    `has died`; the launcher icon → a new process restoring 103 bytes, `Dka755` in the field.
    Setting off, Back at the root → the same two lines, no activity record left; the icon → a
    new process with `saved state size = 0`, the Tasks page drawn. `Kill755` typed, HOME,
    `am kill` → a new process restoring 104 bytes, `Kill755` in the field. No ANR in any run.
    The phone was left as found: the setting unset, the original demo APK reinstalled.
- `cargo fmt --all --check`, `cargo clippy -p frus-shell --target aarch64-linux-android
  --no-deps -- -D warnings`, the workspace clippy and the frus-shell and frus-cli tests.

## Not done

- **A new activity in a process whose loop already ran** still cannot run: winit builds one event
  loop per process. That case remains for a configuration change no manifest declares (rare:
  `mcc`, `mnc`, `touchscreen`, `navigation`, `colorMode`) and for a second activity instance
  (intents must keep `FLAG_ACTIVITY_SINGLE_TOP | NEW_TASK`). The activity is destroyed cleanly
  now and the application closes rather than freezes, but it does not come back by itself: the
  system removes a resumed activity whose process ends without saved state. Running again in the
  same process needs winit to allow a second event loop, or a shell that keeps one loop across
  activities.
- **Android 12 and later** were not run: `onBackPressed` at the root moves the task to the
  background there instead of finishing it, by the platform's choice; that path was not seen on a
  device.
- **Existing applications** keep their manifest: an application created before this milestone
  gets the loop and Back fixes, but must add the `config_changes` line itself to follow font,
  night and language changes in place.
- The hot-reload Android project (`frus run --hot`, milestone 680) writes its own manifest through
  its engine and was not changed.
