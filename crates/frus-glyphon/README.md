# frus-glyphon

A copy of [glyphon](https://github.com/grovesNL/glyphon) 0.12.0, the 2D text renderer for wgpu
that frus draws its text with, changed in **one place**: its shader.

glyphon's fragment shader takes the vertex shader's output struct as its input, and that struct
marks the position `@invariant`. wgpu's shader translator writes that into the OpenGL ES fragment
shader as `invariant gl_FragCoord;`, which OpenGL ES refuses, so text — and with it every frus
application — could not be drawn where frus has to use OpenGL ES (the Android emulator, devices
without Vulkan). The copy gives the fragment shader an input struct of its own, without the
position (it never reads it); the vertex output keeps its `@invariant`.

The same change is proposed to glyphon (grovesNL/glyphon#183), and the translator's side is
gfx-rs/wgpu#10229. When either is released, frus goes back to the original crate and this one is
retired.

Imported as `glyphon` (`[lib] name`), so frus's code is the same with either. The licence is
glyphon's own: MIT, Apache-2.0 or Zlib, at your option (the three files are here).
