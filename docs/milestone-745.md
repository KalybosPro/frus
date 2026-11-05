# Milestone 745 — WebGL 2 where there is no WebGPU

## Objective

The widget gallery opened on an Android phone (Chrome 156, Android 10, Mali-G51) as a uniform dark
page. The page's "Loading…" was gone, since the wasm module had started, and nothing was drawn.
Chrome on Android offers WebGPU from Android 12 only: on that phone `'gpu' in navigator` is false,
while `canvas.getContext('webgl2')` works. frus built wgpu for the web with WebGPU alone, so the
renderer could not be made, and the shell said so in one line of the developer console, which
nobody on a phone sees. The pages had their own check for `navigator.gpu`, but a browser that
blocks a GPU keeps `navigator.gpu` and gives no adapter, and that went through the same silence.

## What was done

**WebGL 2 behind WebGPU.** `frus-gpu` turns on wgpu's `webgl` feature for `wasm32` only, through a
target-specific dependency: native builds keep their backends. On the web the renderer chooses
before the canvas is given any context, because a canvas that has handed out a `webgpu` context
never hands out a `webgl2` one:

1. no `navigator.gpu` → WebGL 2, WebGPU is not touched;
2. `navigator.gpu`, but `requestAdapter()` gives none (a GPU or driver the browser blocks) →
   WebGL 2;
3. an adapter, but no device with frus's limits → WebGL 2;
4. an adapter and a device → WebGPU, and only then is the surface made, from that device.

On WebGL 2 the adapter comes from the canvas's context, so the surface is made first and the
adapter asked for with it, as on native. The console says which: `GPU interface: WebGPU`, or
`GPU interface: WebGL 2 (this browser has no WebGPU (no navigator.gpu))`. The choice itself is a
plain function (`web_backend::choose`) with its tests.

**What WebGL 2 refused.** The first run on WebGL 2 stopped at the first pipeline: four shaders
declared their viewport uniform as a lone `vec2<f32>`, eight bytes, and WebGL 2 binds no uniform
whose size is not a multiple of sixteen. The buffers were already sixteen bytes; the shaders now
say so too. A test parses every shader frus draws with and checks what WebGL 2 needs: no compute
stage, no storage buffer, every uniform a multiple of sixteen bytes. The rest already ran on
OpenGL ES 3.0 (the Android emulator, milestone 686), whose limits — the WebGL 2 ones — are asked
of any device without compute shaders.

**Never a silent page.** When no renderer can be made, or a new one after a lost device, the
shell dispatches a cancelable `frus-error` event on `window`, with a readable message in its
`detail`. A page that shows it calls `preventDefault()`; when none did, the shell lays its own
box over the canvas. A panic, which ends a web application for good, is shown the same way. Every
`index.html` — the `frus create` template, `frus-demo`, `frus-hello` and the examples — no longer
refuses to start without `navigator.gpu`, and shows the message in its status box.

## Size

`frus-hello` (`web-release`, after wasm-bindgen), the CI web size budget's measure:

| | wasm | gzip | brotli |
|---|---|---|---|
| before | 6,829,974 | 2,937,991 | 1,887,488 |
| after | 8,787,754 | 3,621,725 | 2,417,036 |
| delta | +1,957,780 (+28.7%) | +683,734 | +529,548 |

WebGL 2 is drawn through wgpu's own core and its OpenGL ES backend, with naga's GLSL writer: a
WebGPU-only build carried none of it, since the browser does that work. That is 287,754 bytes
over the budget (8,500,000) in `ci/web-size-budget.toml`. The budget is **not** raised in this
change: whether to raise it, or to make WebGL 2 a default feature that a size-first application
can leave out (and get the dark page back on every browser without WebGPU), is left to decide.

## Checks

The gallery, built as the Pages workflow builds it, served locally and loaded in headless Edge at
390 by 800 as a phone:

- WebGPU: `GPU interface: WebGPU`, the gallery drawn;
- `navigator.gpu` removed before the page's scripts, as on the phone: `GPU interface: WebGL 2`,
  the gallery drawn;
- a `navigator.gpu` whose `requestAdapter()` gives nothing: WebGL 2, the gallery drawn;
- neither (WebGL contexts refused too): the page's status box says the application cannot draw
  here, and why; a bare page that does not listen for `frus-error` gets the shell's own box with
  the same message.

On the phone that showed the dark page (Huawei STK-L21, Android 10, Chrome, Mali-G51), the gallery
served from this machine over `adb reverse`: Chrome there has `navigator.gpu` on `localhost` but
gives no adapter, the console says `GPU interface: WebGL 2 (this browser's WebGPU gives no
adapter ...)`, adapter `ANGLE (ARM, Mali-G51, OpenGL ES 3.2)`, and the gallery is drawn: the list,
the backdrop filter demo (blur) and the elevation demo (shadows). The first start compiled the
pipelines in 1.6 s (0.26 s on a reload). In desktop Edge, the elevation demo is the same to the
pixel through WebGPU and through WebGL 2.

## Not done

A WebGPU device that is made and then fails at the surface cannot fall back: the canvas already
has its WebGPU context. That needs a new canvas, which winit owns; no browser has been seen to do
it. WebGL 2 draws without the GPU's clock (frame times are the CPU's) and without a pipeline
cache, as OpenGL ES does.
