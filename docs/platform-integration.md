# Platform integration: what a developer writes, and where Rust stops

frus claims one language, top to bottom. This note states precisely what that claim
covers, where it stops, and what it would take to move the boundary. It is written
for two readers: someone deciding whether to build an application on frus, and
whoever implements the iOS shell or finishes Android accessibility.

The short version: **an application developer writes Rust, and no Java, Kotlin,
Swift or Objective-C.** That holds today on desktop and Android, and it will hold
on iOS. It is not the same statement as "the artifact contains only Rust", and the
difference is worth understanding before it is discovered in a comment thread.

---

## What a developer writes

| | Java / Kotlin | Swift / Obj-C | Rust alone |
| --- | :---: | :---: | :---: |
| UI, layout, state, animation, gestures | — | — | ✅ |
| Desktop, in its entirety | — | — | ✅ |
| Android — APIs frus wraps | — | — | ✅ |
| Android — APIs frus does not wrap | sometimes | — | via `jni` |
| iOS — once the shell exists | — | never required | ✅ |

Two things that table does not show, and that a developer does meet:

**Configuration is not Rust.** `AndroidManifest.xml` carries permissions, the
application name, the icon, deep links — and the activity theme, which is not
cosmetic: an application must ask for a *NoActionBar* theme, because an action bar
takes 56 dp out of `content_rect` and every subsequent layout inherits the error.
iOS will bring `Info.plist` and a launch configuration. This is XML and plist, not
a programming language, but it is not Rust either, and it must be understood rather
than copied.

**Coverage, not language, is the real limit.** A developer who needs notifications,
the camera, biometrics, share sheets, or in-app purchases today writes the JNI
themselves. That is possible and unpleasant. It is discussed under
[the real gap](#the-real-gap-coverage-not-language) below.

---

## Desktop

Nothing to qualify. Win32, X11, Wayland and AppKit are all reachable through
C-callable interfaces, and `winit` plus `wgpu` already stand in front of them.
`accesskit` and `accesskit_winit` expose the semantic tree to UIA, AT-SPI and the
macOS screen reader; the dependency block in
[`crates/frus-shell/Cargo.toml`](../crates/frus-shell/Cargo.toml) scopes them to
desktop deliberately, because neither has a UIKit backend and the Android and Web
providers are separate implementations.

---

## Android: JNI can call, but it cannot subclass

This is the only platform where a wall exists, and the shape of the wall is
specific.

JNI lets Rust **invoke** any Java API without a line of Java: look up the class,
look up the method, call it. Most of the Android SDK is reachable this way, and
everything frus needs from the platform beyond the NDK's C surface goes through it.

What JNI cannot do is **define** a class. Subclassing an abstract or concrete Java
class requires a class definition, which requires bytecode, which requires either a
compiler or a bytecode emitter. So wherever Android's design says *"extend this
class and override these methods"*, pure Rust cannot answer.

frus has met this exactly once so far.
[`FrusTextBridge.java`](../crates/frus-shell/java/dev/frus/input/FrusTextBridge.java)
is 209 lines and exists because `NativeActivity` provides no `InputConnection`.
Without it, every IME degrades to `TYPE_NULL` — Latin keys only, no composition, no
swipe typing, no CJK — which is not a rough edge but a broken text field for most of
the world's writing systems. The bridge is a focusable 1×1 `View` laid over the
native content; it supplies a real `InputConnection` and relays each IME operation
to natives registered from
[`android_ime.rs`](../crates/frus-shell/src/android_ime.rs) through
`RegisterNatives`. See [milestone 81](milestone-81.md) for the full reasoning.

The packaging decision matters as much as the shim: it is compiled to a dex by
[`scripts/build-input-dex.sh`](../scripts/build-input-dex.sh) and loaded at runtime
with `InMemoryDexClassLoader`. An application's build never learns that Java was
involved — no Gradle, no source set, no toolchain in the dependency graph. `cargo
apk run` remains the whole story.

### The escape hatch, and its limit

`java.lang.reflect.Proxy` is itself invocable over JNI, and it implements an
**interface** dynamically at runtime. Every listener-shaped API — callbacks,
observers, `OnClickListener`-style contracts — is therefore reachable with no `.java`
file at all. It does not extend classes, so it does not help with `View`,
`Service`, `BroadcastReceiver` or `AccessibilityNodeProvider`.

That is the rule to carry: **interfaces are free, classes cost a dex.**

### The next shim is already visible

Android accessibility is marked partial in [ROADMAP.md](../ROADMAP.md), and
finishing it means providing an `AccessibilityNodeProvider` — an abstract class, to
be subclassed. By the rule above, that is a second dex shim, a sibling of the IME
bridge. It should be planned as such rather than discovered halfway through, and
the two shims should probably share one dex and one loader.

---

## iOS: the asymmetry runs the other way

The common assumption is that Apple's platform will be the harder one to reach from
Rust. It is the easier one, and the reason is the Objective-C runtime.

That runtime is dynamic and C-callable. `objc_allocateClassPair` and
`class_addMethod` create classes **at execution time**, from Rust, with no compiler
and no bytecode step — the very thing the JVM refuses. The `objc2` family of crates
wraps this, and `winit` already constructs its `UIView` and `UIApplicationDelegate`
this way on iOS.

So the target is genuinely zero lines of Swift and zero lines of Objective-C, for
the framework as well as for the application. UIKit, Metal, CoreText,
`UIAccessibility` and `UITextInput` are all Objective-C or C, therefore all
reachable. Nothing in the iOS integration list from
[milestone 276](milestone-276.md) — lifecycle, safe-area insets, the soft keyboard,
`os_log`, UIKit accessibility, `.ipa` packaging — requires a Swift source file.

Two honest caveats. Building and signing needs a Mac with the Xcode toolchain: that
is tooling, not language, but it is a real prerequisite. And a handful of newer
Apple frameworks are Swift-only; none of them is on the path to a working shell,
but that could change if the platform surface grows.

---

## Web

Generated JavaScript glue is unavoidable, because wasm cannot touch the DOM
directly — every call crosses an import table that `wasm-bindgen` writes. No
developer writes that JavaScript, and no developer reads it, but it is present in
the artifact and `index.html` is a real file. When web accessibility is wired up it
will mean creating DOM nodes through `web-sys`, which is Rust calling generated
bindings: more glue, still no hand-written JavaScript.

---

## The real gap: coverage, not language

The honest summary of everything above is that the language question is nearly
settled and the *coverage* question is wide open.

An application needing notifications, the camera, biometrics, a share sheet, a file
picker or in-app purchases gets no help from frus today. Its author writes JNI by
hand — reachable, verbose, easy to get wrong, and repeated in every application
that needs the same thing. That is the actual obstacle for someone shipping a real
application, and it is not solved by adding another widget.

What it argues for is a platform-services layer: a small set of Rust APIs with
per-target implementations behind them, following the same rule as the rest of the
architecture — the portable crates state the intent, `frus-shell` knows the
platform. The shape is not settled and no milestone claims it. It is written down
here so the next person to feel the friction knows it was seen, not missed.

---

## What this means for the project's claims

- **"No embedded VM"** — structural. Rust compiles ahead of time to native code and
  to wasm, and [`reload.rs`](../crates/frus-shell/src/reload.rs) achieves
  state-preserving hot reload by serialising one struct and relaunching the binary,
  which is the usual reason a framework ends up embedding an interpreter. On
  Android the process hosts ART because `NativeActivity` is a Java class; that is
  the platform's runtime, not one frus ships.
- **"No second language for app logic"** — true, and the qualifier is load-bearing.
  The framework contains one Java file and will likely contain a second; an
  application contains none.
- **"No platform channel between your code and the pixels"** — true, and scoped on
  purpose. The render path is application → widget tree → scene → `wgpu` → native
  API, with no serialisation boundary. The JNI bridge sits on the *input* path.
  Restated as "no platform channel", the claim is false.
