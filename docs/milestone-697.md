# Milestone 697 — The pipeline cache, hardened and on every desktop

## Objective

Milestone 685 (#217) kept the renderer's compiled pipelines between launches where the device
can (Vulkan): on the test phone the first frame of a second start came 3.5 times sooner. It
left the cache's file and folder untested, and four gaps:

- The file was named by wgpu's key alone — the adapter's vendor and device. A new driver or a
  new frus read the old file; wgpu then refused it by its header, or it grew with pipelines no
  one would ask for again. Either way the old file stayed for ever.
- wgpu checks the header of the data it is given, not the rest, which a Vulkan driver may
  trust. A file damaged on the disk past its header went to the driver as it was.
- On Android it lived in the application's `files` folder, which is for what must not be lost:
  "Clear cache" did not empty it.
- Only Linux had a folder for it: a desktop given `WGPU_BACKEND=vulkan` on Windows or macOS
  compiled everything at every start.

## What was done

- **`frus-gpu/src/pipeline_cache.rs`**: everything the renderer does with the cache, as plain
  functions of files, tested without a GPU. `Renderer::new` calls `open` before the painters
  and `keep` after the warm-up frame, as before.
- **The file is named for the adapter, the driver and frus**: `frus-pipelines-<frus version>-
  <wgpu's adapter key>-<digest of the driver's name and version>`. No name — so no cache —
  where wgpu keeps no pipelines (DirectX 12, Metal, OpenGL, the browser).
- **Sealed**: the file starts with a mark and the digest of the data (FNV-1a, 64 bits, no
  dependency). A file cut short, grown, with a flipped byte, from before this
  milestone, or not ours at all is refused, removed, and the pipelines are compiled again; the
  launch goes on as if there were none.
- **Written only when it changed**, aside first and then moved over; a write that fails leaves
  nothing aside. After a write, the other `frus-pipelines-*` files in the folder — other
  drivers, adapters, versions — are removed, and milestone 685's `frus-wgpu_pipeline_cache_*`
  file with them. Nothing else in the folder is touched.
- **`frus-shell/src/cache_dir.rs`**: where each platform keeps it, as functions of the
  environment, so each platform's answer is tested on any.
  - Android: the application's `cache` folder (beside `files`, what `Context.getCacheDir`
    names), `frus-pipelines/`. The folder milestone 685 used inside `files` is removed at
    start.
  - Linux and the other Unixes: `$XDG_CACHE_HOME/frus/<program>`, or `~/.cache/frus/<program>`;
    a relative `XDG_CACHE_HOME` is ignored, as the XDG rules say.
  - macOS: `~/Library/Caches/frus/<program>`; Windows: `%LOCALAPPDATA%\frus\<program>`. Used
    only where the renderer draws through Vulkan; nothing is written otherwise, since there is
    no cache to write.
  - No folder when the variable needed is missing or not a full path: never a folder relative
    to wherever the program was started.
- The web is unchanged: no folder is set, the device has no pipeline cache.

## Measured

Not measured on the phone this time: no device was attached. What the cache saves is
milestone 685's measurement (Huawei STK-L21, Mali-G51, counter release: pipelines 708 ms on the
first start, 46–49 ms from the cache; first frame 1153 ms, then 323–333 ms); this milestone
changes where the file lives, its name and its seal, not what is cached.

Through a real device, in WSL (the Vulkan driver there has a pipeline cache): the painters
built through a fresh cache, kept after the warm-up, and read back by a second device that built
through it too.

## Left

- **Falling back when the Vulkan driver is bad** — #217's second half: a device whose Vulkan
  driver fails or draws wrongly should be drawn through OpenGL ES without the user setting
  `WGPU_BACKEND`.
- glyphon's text pipelines are built inside glyphon and still not cached.
- The cache is written on the interface's thread after the warm-up (61 KB on the test phone
  in milestone 685), on the starts where it changed; not timed on its own.
- DirectX 12 and Metal keep no pipelines in wgpu; Windows' start is shortened by DXC instead
  (milestone 684).

## Tests

TESTS

## Mutation testing

Twelve hand mutants, each killed by a test:

| mutant | killed by |
|---|---|
| the name leaves out the driver | `the_file_is_named_for_the_adapter_the_driver_and_frus` |
| the name leaves out frus's version | the same |
| a file for a backend that keeps no pipelines | `no_file_where_the_backend_keeps_no_pipelines` |
| the digest not checked | `damaged_data_is_refused` |
| a damaged file kept on the disk | `a_damaged_file_is_ignored_and_removed` |
| an unchanged cache written again | `an_unchanged_cache_is_not_written_again` |
| a failed write leaves its file aside | `an_unreadable_file_is_no_cache` |
| pruning removes files that are not frus's | `storing_removes_the_files_of_other_drivers_and_versions` |
| pruning never runs | the same |
| milestone 685's file not pruned | the same |
| a relative cache folder accepted | `linux_uses_xdg_cache_home_then_home`, `nothing_is_kept_without_a_full_path` |
| Android keeps them in `files` | `android_keeps_them_in_the_cache_folder` |

A thirteenth, the data's length not checked, survived: the digest already refuses a file of
another length. The length was taken out of the seal rather than kept untested.

When the cache is written — after the warm-up frame, in `Renderer::new` — needs a window and is
not covered by a test; it is the line milestone 685 checked on the phone.
