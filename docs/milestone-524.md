# Milestone 524 — More than one ABI

Issue #10. Every example, and the template an application starts from, builds
`aarch64-linux-android` and nothing else, and nothing in the documentation said that an
application shipping to people wants more than one ABI — or what the packager does and does
not do about it. The first person to find out would have found out from a store's console.

## What `cargo-apk` does, read rather than assumed

The guide could only be as right as its description of the tool, so the tool was read first —
`cargo-apk` 0.10.0 and the `ndk-build` 0.10.0 it drives, the versions on the build machine. CI
installs `cargo-apk` without a version, so it gets whatever is newest:

- **`build_targets` makes one APK.** The library is compiled once per target and every result
  is added under `lib/<abi>/`; the package is written, aligned and signed once, at
  `target/<profile>/apk/<name>.apk`.
- **`--target` replaces the list.** Given on the command line, it is the only target built,
  whatever the manifest says.
- **The version code is the crate's version**, one byte per part, and a `version_code` in the
  manifest is refused. Every APK built from the same version, one per ABI or not, has the same
  code.
- **There is no split and no bundle.** The tool runs `aapt`, `zipalign` and `apksigner`, and
  writes an APK. Nothing in it produces an `.aab` or per-ABI APKs meant to be published together.
- **Release signing** reads `CARGO_APK_RELEASE_KEYSTORE` and
  `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` before the manifest's table.

## What was decided

**A section of the getting-started guide, not a page of its own.** It sits in *Shipping*,
between *Signing* and the fonts, because it is the same moment in an application's life, and
its size numbers belong beside the ones already there.

**Which ABIs, said plainly.** `arm64-v8a` always; `armeabi-v7a` for 32-bit devices, as an
addition, since some recent devices run 64-bit code only; `x86_64` for the emulator and
ChromeOS; `x86` not at all. No market-share percentages: none could be checked from here, and
a number that cannot be checked is the kind of claim the issue asked not to make.

**What the tool does not do is a list, not a footnote**: no split, no bundle, no Rust targets
installed, no version past 255. The per-ABI route is documented because it works and is useful
on a device, and documented as *not for publishing*, because identical version codes are not
something a store accepts.

**The app bundle is named and not described.** Google Play requires one for new applications;
producing one for a frus application needs tooling this repository has never run, and a
documented command that has not been run is what the issue asked for least.

**The signing section gained the environment variables**, since a CI job that builds a release
wants the password out of `Cargo.toml`, and the size measurement below was signed that way.

## Verification

**The sizes are measured.** `frus-hello` was built on the build machine — `cargo-apk` 0.10.0,
rustc 1.96.1, NDK 26.3 — in release, with the workspace profile and every bundled face, three
times into one target directory: with both ABIs in `build_targets`, then once with each
`--target`. Each APK was signed through `CARGO_APK_RELEASE_KEYSTORE` and its password variable,
and `apksigner verify` accepted the two-ABI one. Sizes in bytes, read from the files:

| APK                | APK       | `.so`      | `.so` compressed in the APK |
| ------------------ | --------- | ---------- | --------------------------- |
| `arm64-v8a` only   | 4,829,606 | 10,488,624 | 4,818,740                   |
| `armeabi-v7a` only | 4,567,464 | 8,444,544  | 4,558,262                   |
| both               | 9,388,532 | both       | both                        |

A second ABI costs what the first does. The library is nearly the whole APK, and two
architectures' machine code have nothing to share: the two-ABI APK is 8,538 bytes smaller than
the two single ones together, which is one manifest, one resource table and one signature.
The `arm64-v8a` library is the one the size budget counts; milestone 508 read 10,427,568 bytes
for it, and it has grown by 61,056 since.

**The tool's behaviour is checked on its output, not only in its source.** `aapt dump badging`
gives all three APKs `versionCode='16777472'` — `0x01000100`: the `1` the tool puts in the top
byte, and 0.1.0 below it — and `native-code: 'arm64-v8a' 'armeabi-v7a'` for the two-ABI APK,
one ABI for each of the others. `unzip -lv` shows `lib/arm64-v8a/` and `lib/armeabi-v7a/` side
by side in the one package. The `--target` builds wrote to the same `target/release/apk/`
path, each over the last, which is why the script copied each away.

**The page alignment** is `llvm-readelf -lW` on both release libraries: every `LOAD` segment,
`arm64-v8a` and `armeabi-v7a` alike, is aligned to `0x1000`.

**The signing variables** are the ones in the tool's source — `CARGO_APK_<PROFILE>_KEYSTORE` and
`_PASSWORD`, read before the manifest — and a release profile given the first without the second
returns `MissingReleaseKey`, which is what *set both or neither* says.

`cargo fmt --all -- --check` and `cargo metadata --no-deps` pass. No code changed.

**Not run**: none of the three APKs was installed on a device — what runs is the counter as it
was, packaged differently — and `rustup target add` was not run, since both targets were already
installed.

## What is left

- **An app bundle.** The route to Google Play. It needs `aapt2` and `bundletool`, or a different
  packager, and it needs to be done once, for real, before it is written down.
- **`x86_64`**, built and measured. Its Rust target was not installed on the build machine.
- **16 KB pages.** Found while measuring, not looked for: the release `libfrus_hello.so` for
  `aarch64` has every `LOAD` segment aligned to `0x1000`, 4 KB. Google Play asks applications
  targeting Android 15 or later to support 16 KB pages; the template targets SDK 34, so nothing
  fails today, and the first application to raise its target will meet it. The guide says so
  and stops there — the linker flag that fixes it has not been built with, let alone run on a
  16 KB device.
- **The template and the examples still list one ABI.** That is the right default for a
  project's first build — every ABI added is another full compile of the library — and the
  guide says how to add more.
