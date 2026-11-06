# Milestone 763: what milestone 755 left on Android's activity lifecycle

## What

Milestone 755 ended the hang when Android destroys a frus application's activity: the loop ends
and the process with it. It left five things, which this milestone takes in turn.

1. **The hot-reload build declares the configuration changes.** `frus run` on Android builds
   through its engine's own Android project (milestone 680), whose manifest declared four
   configuration changes. frus now writes that manifest itself (`AndroidManifest.xml` in the
   shadow package, named by `android_manifest` in the `Dioxus.toml` it already writes), the
   engine's template unchanged but for the activity's `configChanges`, which are the
   application's own.
2. **Applications made before milestone 755 get the declaration.** Before an Android build
   (`frus build apk`, `frus run` with or without hot reload), frus reads the application's
   `Cargo.toml` (`crates/frus-cli/src/config_changes.rs`):
   - no `config_changes` in `[package.metadata.android.application.activity]`: the line is
     added — into that table if it exists, as a new table at the end otherwise — and frus says
     so;
   - the list milestone 755 wrote, unchanged: it is frus's own, and is completed in place,
     with a notice;
   - a list the developer wrote: it is **never changed**; what it leaves out is named in a
     warning, with the full list to copy;
   - the activity's table written some other way (`activity = { … }`, dotted keys), where a
     header would define it twice and cargo would refuse the manifest: a warning, nothing
     written.
   Line endings are kept, and a second build finds nothing to do.
3. **A second activity in the process no longer fails.** See *Design*.
4. **Back at the root on Android 12 and later** was run on an Android 16 emulator. See *How it
   was tested*.
5. **The configuration changes milestone 755 left undeclared are declared.** Checking whether
   they come back showed they cannot (see *Why*), so the cause is removed instead: every
   configuration change an activity can declare is declared. Milestone 755's list gains `mcc`,
   `mnc`, `touchscreen`, `navigation`, `colorMode`, `fontWeightAdjustment` (the "bold text"
   setting, Android 12) and `grammaticalGender` (Android 14), in the template, the demo, the
   sample crates and the eight examples. A name newer than the SDK the package is compiled
   against does not package, so the list frus writes follows the application's
   `target_sdk_version` (`fontWeightAdjustment` from 31, `grammaticalGender` from 34).

## Why

### An undeclared configuration change cannot come back

When a configuration change is not declared, Android *relaunches* the activity: on the
application's main thread it pauses, stops and destroys the old instance and creates a new one
with the state the old one saved — all inside the process, and the saved state is held by the
process, not reported to the system. frus cannot run the new instance (winit builds one event
loop per process), so since milestone 755 the process ends inside the old instance's
`onDestroy`. The system then finds a resumed activity whose process died with no saved state
and removes it (`app died, no saved state` in milestone 755's log). Restoration (milestone 753)
has nothing to restore: the state never left the process. Nothing short of a second event loop
in the process brings that activity back, which is why the changes are declared instead — the
activity then stays, and the shell follows the change in place (milestone 755's
`refresh_platform_settings`, which reads the bold-text setting too).

### Undeclarable changes

One configuration change cannot be declared: a change of the resources' asset paths, which
Android 12 and later make when the person picks new wallpaper colours (the system palette is a
resource overlay). On the Android 16 emulator, setting a system palette
(`theme_customization_overlay_packages`) and setting it back relaunched the demo's activity:
`the activity is being destroyed: the loop ends`, `the process ends`, then `Force removing
ActivityRecord… app died, no saved state` — the application closed cleanly, and did not come
back, as above. The launcher was relaunched by the same change.

## Design

### A second activity hands over (`frus-shell/src/android_lifecycle.rs`)

Android starts a second instance of the application's activity in its existing process when an
intent asks for one: a link opened from another application (`frusdemo://task/3` from a mail),
a notification or a shortcut whose intent lacks `FLAG_ACTIVITY_SINGLE_TOP`. The native activity
runs `android_main` again for it, on a thread of its own, while the first activity's loop still
runs. Until now `run_android` tried to build a second event loop, winit refused
(`EventLoop can't be recreated`), and the new activity was finished with nothing to show for
it: the link was lost.

Now `run_android` first asks `android_lifecycle::first_in_process`, an atomic flag set by the
first activity of the process (the decision is `app::first_loop_of_process`, unit-tested from
eight threads at once). A later activity does not touch anything the running one uses; it
`hand_over`s:

- the link it was opened with (`getIntent().getDataString()`, as at a launch) is queued for the
  running loop, whose window is asked for a frame; the loop opens it as it opens a link handed
  over by a second process on a desktop (`open_links_from_other_instances`, now on Android too);
- if the new activity is in another task than the running one — the link was opened from
  another application's task, or its intent asked for a task of its own — the running
  activity's task comes to the front. First it is looked for among
  `ActivityManager.getAppTasks()` by its id and `moveToFront()`ed, which needs no permission
  for an application's own tasks (Android 16 lists it). When the list does not hold it (Android
  10 listed only the new activity's task), the new activity **finishes first** and then starts
  the running activity's component with `FLAG_ACTIVITY_NEW_TASK | FLAG_ACTIVITY_SINGLE_TOP`:
  the system finds a task by affinity and passes over one whose activities are all finishing,
  so it finds the running task, brings it forward and hands the intent to the activity on top
  instead of making another;
- `android_main` returns, and the native activity finishes the new instance.

The person sees the application they had, at the link's location.

- *Considered: ending the process* so that the system starts the activity again in a new one.
  It kills the running activity too: it was only paused when the second one arrived, and from
  Android 9 an activity's state is saved when it *stops*, so whatever the person had since the
  application last went to the background would be lost — and the new activity, which had no
  saved state either, would be removed rather than restarted. Handing over keeps everything.
- *Tried first: the same intent while the new activity was still alive.* The system picks the
  task by affinity, and a new activity in a task of its own (`FLAG_ACTIVITY_MULTIPLE_TASK`) has
  the same affinity and is more recent: on the phone the intent went to that task, and once the
  new activity finished the person was on the home screen. Finishing it first is what makes the
  intent find the running task.
- *Considered: `ActivityManager.moveTaskToFront`*: it needs the `REORDER_TASKS` permission in
  every application's manifest.

### A failed loop ends the process

`run_android` returned early when building or running the loop failed, which left a process
behind with winit's one loop spent: the next activity in it could not run. The loop's result is
now logged and the process ends either way, as it does when the loop ends normally.

## How it was tested

Unit tests (`cargo test -p frus-cli -p frus-shell`):

- `config_changes::tests` (8): the list follows the target SDK; a manifest without the line gets
  the table, after everything it had, and still reads as the same application; an existing
  activity table gets the line, not a second table; milestone 755's list is completed in place;
  a developer's own list is kept and its gaps are named (another order, or more, is nothing to
  report); an activity written as an inline table or dotted keys is not edited; Windows line
  endings are kept; `ensure` writes once and the second build changes nothing; the template and
  the four sample crates that build an APK declare the whole list.
- `hot::tests::the_engine_s_android_project_declares_the_configuration_changes`: the engine's
  project gets `AndroidManifest.xml` with the application's `configChanges`, `Dioxus.toml` names
  it, and the engine's activity, library name, launcher filter and `INTERNET` are kept.
- `app::tests::only_the_first_activity_of_a_process_runs_the_loop`: eight threads ask at once,
  one is first, and every later call is not.

On devices, the demo as a release APK:

- **Huawei STK-L21, Android 10, before this milestone** (the installed 0.2.1 demo): the demo
  running, `am start -a VIEW -d frusdemo://task/2` → `Creating: Activity …` then `frus (android)
  stopped: EventLoop can't be recreated`; the second activity was finished and the demo stayed
  on its home page — the link was lost. The same with `-f 0x18000000` (a task of its own) from
  the home screen: the same error, and the person was left on the home screen.
- **Same phone, this branch**: the first link → `a second activity started in this process with
  frusdemo://task/2: it hands over to the running one and finishes`, the same process (one
  activity record), and the demo showed the Task page of that link. The link from a task of its
  own, three builds:
  - the intent with `NEW_TASK | SINGLE_TOP` sent while the new activity lived: it logged `the
    running activity's task comes to the front`, and the launcher stayed resumed;
  - `getAppTasks()` alone: `the running activity's task (229) was not found among the
    application's tasks ([230])` — the list held only the new activity's task — and the
    launcher stayed resumed;
  - the final code: `the running activity's task (238) is not among the application's tasks
    ([239]): the new activity finishes first, then asks for the running one`, `START u0
    {flg=0x30000000 cmp=com.frus.demo/android.app.NativeActivity} from uid 10198`, `the running
    activity's task comes to the front`; `dumpsys activity` showed the demo's task 238 resumed
    with one activity, the same process, and the screen showed the Task page of `task/3`.
- **Android 16 emulator (API 36, x86_64, GLES)**: the same two links. Both handed over in the same
  process; for the second, `the running activity's task comes to the front`, and
  `dumpsys activity` showed the demo's task resumed again, at the Task page of `task/3`.
- **Configuration changes in place on Android 16**: bold text on and off
  (`settings put secure font_weight_adjustment 300`, then deleted) and night mode on and off
  (`cmd uimode night`): the same process and the same activity record throughout. Before this
  milestone `fontWeightAdjustment` was not declared, so bold text recreated the activity, which
  closes a frus application (the relaunch path below; not run with the old list).
- **Back at the root on Android 16**: the demo started by `monkey` (the launch source is the
  shell, not the launcher), Back → the system *closed* the task (a `CLOSE` transition, not
  `TO_BACK`): `the loop ends`, `the process ends`; the launcher icon then started a new process
  that drew its first frame. No hang and no blank window. The move-to-background that Android 12
  and later apply to a root activity *started from the launcher* was not exercised (see *Not
  done*); it leaves the activity alive and stopped, the path HOME takes, which milestone 755
  covered.
- **An undeclared change** (the wallpaper palette, above): closed cleanly, not brought back.

Checks: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo clippy -p frus-shell --target aarch64-linux-android --no-deps -- -D warnings`.

## Not done

- **The finish-then-intent path on Android 16** was not run: the emulator's `getAppTasks()`
  listed the running task, so it took the first path. A link opened from inside another
  application's task (rather than `am start` with a task of its own) was not run either.
- **An undeclared configuration change still closes the application** — only the asset-paths
  change of the wallpaper palette (Android 12 and later) remains, since every declarable change
  is declared. Surviving it needs a second event loop in the process, or a shell that keeps
  one loop across activities.
- **Back at the root of a task started from the launcher**, on Android 12 and later, was not
  run: the emulator was too slow to drive its launcher (System UI not responding, first frames
  after 20 seconds), and the phone is Android 10.
- **The hot-reload manifest** was checked by its unit test, not by a `frus run` on a device (an
  engine build of the demo takes about fifteen minutes on the phone). It carries the activity
  and `INTERNET` permission the engine wrote, not the application's own permissions, as
  before.
- The test phone runs Android 10, where `fontWeightAdjustment` does not exist; bold text was seen
  on the emulator only.
