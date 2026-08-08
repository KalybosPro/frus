# Jalon 50 — First run on physical Android

frus now runs on a **real Android phone** (Huawei STK-L21, arm64, Android 10,
Mali-G51 GPU): Vulkan rendering, touch (tap + finger scrolling), navigation, and
the whole widget library. The same code as the desktop; only the **shell layer**
gains an Android-specific entry point and lifecycle handling.

## What had to be added

### 1. The `android_main` entry point
Android does not call `main()`. The demo becomes a **`cdylib` library** (the `.so`
loaded by the native activity) exposing `android_main(app: AndroidApp)`, on top of
a desktop binary (`src/bin/frus-demo.rs`). Both call the same code:
- desktop → `frus_demo::run_desktop()` → `frus_shell::run`;
- Android → `android_main` → `frus_shell::run_android(app, android_app)`.

`frus-shell` gains `run_android` (a winit loop built with
`.with_android_app(...)`, logs to logcat through `android_logger`). `winit` gets
the `android-native-activity` feature on the Android target only. `arboard`
(clipboard) and `env_logger` become **desktop-only** (they do not compile for
Android); the clipboard is neutralised by a small `clip` wrapper.

### 2. Touch input
The driver only handled the mouse. The mouse arms are factored into
`pointer_down / pointer_move / pointer_up` helpers, which `WindowEvent::Touch`
reuses (one finger = one pointer). On top of that, a `Drag::Scroll` drag variant
implements **finger scrolling**: below `TOUCH_SLOP` (8 px) the gesture stays a
tap; beyond it, it scrolls the scrollable area under the finger.

### 3. Surface lifecycle
Android **destroys the surface** in the background. Added `suspended` (releases
the renderer + the window); `resumed` recreates them. The `init` start-up effect
is played only once (a `started` flag), not on every return to the foreground.

### 4. Real GPU limits (a cross-cutting fix)
`downlevel_defaults()` caps the maximum texture at **2048**, whereas the screen is
1080×2340 → `surface.configure` was panicking. The renderer now asks for
`downlevel_defaults().using_resolution(adapter.limits())`: downlevel
compatibility but the adapter's real resolution. (Beneficial on high-resolution
desktops too.)

### 5. Bundled font (a cross-cutting fix)
`FontSystem::new()` relies on the system fonts; on Android, fontdb does not read
`fonts.xml`, so the "sans-serif" alias resolves to **no** default font → a
cosmic-text "no default font found" panic. `frus-text` now bundles **DejaVu Sans
/ Sans Mono** (`include_bytes!`) and exposes `new_font_system()`: the system
fonts (emoji/script fallback) **plus** the bundled font set as the default
family. `frus-gpu` (glyphon rendering) reuses that same `FontSystem` →
deterministic text rendering on every platform.

## Tooling (WSL) & device workflow
- Build **inside WSL** (cross-compilation, Linux build scripts — native Windows
  is still blocked by Smart App Control). SDK/NDK (r26d) + `cargo-apk` installed
  in WSL; target `aarch64-linux-android`.
- `cargo apk build -p frus-demo --lib` → a signed APK (automatic debug key).
- **Installation through Windows' `adb.exe`** (the phone is plugged into the
  Windows side, no usbipd): copy the APK to a `/mnt/...` path, then
  `adb.exe install -r` + `adb.exe shell am start -n com.frus.demo/android.app.NativeActivity`.
- cargo-apk metadata in `frus-demo/Cargo.toml` (`package = com.frus.demo`,
  `min_sdk 24`, `target_sdk 34`, `build_targets = ["aarch64-linux-android"]`).

## Validation on the device
- Rendering: Tasks, Log (a 5000-row virtual list), Settings (Switch, Slider,
  RadioGroup, Dropdown, Rating, Stepper, DatePicker, Tabs, Breadcrumb, Card).
- Tap → navigation; swipe → scrolling (Row 1 → Row 11); the stopwatch running; no
  crash. Vulkan backend (Mali-G51).
- Desktop non-regression: the workspace built, **162 tests** green, the demo ran.

## Limits (v1)
- No IME / soft keyboard: text fields receive focus but soft-keyboard input is
  not wired up yet.
- The demo's header overlaps at very small widths (a layout refinement on the
  demo side, not a framework defect).
- A single ABI packaged (`arm64-v8a`); no `armeabi-v7a`/`x86_64`.
- No system inset handling yet (status bar / gestures): the UI extends under the
  status bar.
