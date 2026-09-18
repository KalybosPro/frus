# Milestone 550 — Re-blessing the goldens for the new text stack

Follows this branch's wgpu 22→30 upgrade, which brought glyphon 0.6→0.12 and
cosmic-text 0.12→0.19 along with it (seven minor versions on cosmic-text). CI's
now-blocking golden check (milestone 539, #15) caught the result: 91 of 102
goldens differed, entirely in anti-aliasing and hinting on the text — small pixel
counts (2 to 148) with no shift in layout, colour or content, consistent with a
font-shaping library seven minor versions newer rather than a rendering
regression.

Re-blessed under WSL (`FRUS_UPDATE_GOLDENS=1`), across `goldens`, `widgets` and
`motion` — 102 + 47 + 16, all green afterward. 158 PNGs changed in total (more
than the 91 that failed under the old tolerance, since re-blessing rewrites every
golden the suite touches, not only the ones that had drifted past it).
