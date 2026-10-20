# Milestone 684 — DXC: a Windows start half a second shorter

## Objective

Milestone 683 measured a frus application's first frame (#211): on Windows, about one second
for the counter, three quarters of it building the renderer's pipelines — at every launch,
since nothing keeps them (#217). Most of that is DirectX 12 compiling the shaders with FXC,
Windows' own compiler: old and slow. DXC, its successor, is several times faster, but it is a
pair of DLLs the program has to have beside it.

## Measured

The counter, release build, Windows 11 on an Intel Iris Xe laptop, launches alternated so that
both compilers ran under the same load:

| launch | compiler | pipelines | first frame |
|---|---|---|---|
| 1 | FXC | 1365 ms | 1998 ms |
| 1 | DXC | 190 ms | 629 ms |
| 2 | FXC | 937 ms | 1279 ms |
| 2 | DXC | 240 ms | 696 ms |
| 3 | FXC | 1234 ms | 1766 ms |
| 3 | DXC | 258 ms | 761 ms |

Under a heavier load the same ratio held (DXC 680–1130 ms of pipelines, FXC 4100–6100 ms).

## What was done

- **frus fetches DXC** — the pinned release 1.9.2609 from Microsoft's DirectXShaderCompiler
  repository, about 32 MB, once — into `~/.frus/tools/dxc-1.9.2609`.
- **`frus run` puts it beside the program it starts** (its copy in `target/frus-run`, where
  the DLLs are kept across restarts), and **`frus build desktop` beside the program it
  builds**, saying they are to be shipped with it: `dxcompiler.dll` (22 MB) and `dxil.dll`
  (2 MB). A program without them still runs, with FXC.
- **The renderer uses the DXC beside the program** explicitly, and says which compiler it
  uses (`DirectX 12 shaders: DXC (…)`, or Windows' own, with the reason).

Fetching DXC never stops a run: if it fails, `frus` says so and the program starts with FXC.

## Left for #217

The pipelines are still built at every launch: a cache of them (wgpu's `PipelineCache`,
Vulkan only) would remove the rest on Android and Linux. DirectX 12 has no such cache in wgpu.

## Verified

`frus run` on the counter fetched DXC, placed it beside the running copy, and the program
logged `DirectX 12 shaders: DXC`; `frus build desktop` placed the two DLLs beside
`target/release/counter.exe`. `cargo test -p frus-cli`: 99 tests; clippy for `frus-cli` and
`frus-gpu`.
