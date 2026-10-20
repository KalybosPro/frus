# Milestone 682 — The web's colours

## Objective

Issue #234: in a browser every colour came out darker than on the desktop and on Android — the
template's background `#E9EBEF` instead of `#F5F6F8`, its greens far darker. Each channel was
the desktop value converted to linear and never encoded back.

## Cause

The renderer takes the first sRGB format the surface offers. A WebGPU canvas offers none — only
`bgra8unorm` or `rgba8unorm` — so it drew into a plain format. The shaders write linear values,
which an sRGB target encodes as it stores them; a plain one stores them as they are, and the
browser shows them as if they were encoded.

## What was done

The surface keeps the format it offers, and declares its sRGB twin in `view_formats`
(`add_srgb_suffix`), which a WebGPU canvas allows. The frames are drawn through a view in that
twin: the painters' pipelines, the MSAA choice and the warm-up are all made for it, and the
hardware encodes, as on the other platforms. Where the surface already offers an sRGB format —
the desktop, Android — the twin is the format itself and nothing changes.

## Verified

The counter in headless Edge (WebGPU), its rendered pixels sampled from a screenshot, against
the same application on the Windows desktop: background `#F5F6F8` on both (it was `#E9EBEF`),
the count's green `#16A34A` on both. Translucent colours now blend on the web as they do
elsewhere, in linear. The goldens are rendered off screen and are not affected.
