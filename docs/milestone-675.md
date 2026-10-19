# Milestone 675 — A screenshot of the page

## Objective

`s` in `frus run` saved a screenshot on the desktop (the frame drawn again offscreen) and on
Android (the device's own), and on the web said to use the browser's tools. The page is the
application's: it can give its picture.

## What was done

- **The application reads its canvas.** On `screenshot page`, the next frame, right after it is
  drawn, reads the canvas as a PNG (`toDataURL`). Then and only then: a WebGPU canvas keeps what
  was drawn until the task that drew it ends, and is blank after.
- **The page sends it.** The script `frus run` adds to the page defines
  `window.__frusScreenshot`, which posts the picture to `/__frus/screenshot`; the application calls
  it. On a page `frus run` does not serve, there is nothing to call, and the application says so.
- **`frus run` writes it** where the key asked, `frus_01.png` (02, 03…) beside the application,
  decoding the base64 itself, and says so as on the other platforms.

## Verified

- In headless Edge on the counter example: `s` gave "Screenshot written to frus_01.png (24 kB)",
  and the file is the page as drawn — the counter, its buttons, the floating button.
- Unit test: the server writes the picture it is sent where the run asked, decoded.
- Clippy on the desktop, Android and the web.
