# Milestone 694 — Bitmaps inside SVG pictures and Lottie animations

## Objective

#303 and #304 both left embedded bitmaps out. An SVG exported from a design tool often carries
a photo or a texture as a `data:` image, and a Lottie animation's image layers are its photos,
avatars and painted backgrounds: until now each was skipped and named by `unsupported`, and the
picture showed with a hole. lottie-android's `Heart.json`, whose heart is a PNG over a blue
solid, painted only the blue.

## What was done

- **`Scene::draw_image_mapped(image, to_screen, tint)`** draws a whole bitmap through any affine
  map from its pixels to the screen — the one piece both formats needed.
  - A map that only moves and scales, the usual case, is one image primitive.
  - Any other — turned, slanted, mirrored — draws the image upright **at its size on screen**,
    at the origin, in a transformed layer that turns it into place. Upright at its screen size,
    not at its pixel size, so it is sampled once at the resolution it is shown at, as an image
    drawn straight is.
  - Inside that layer the clip is the image's own rectangle: the clip around it is the screen's,
    which would cut a flat image at the origin to nothing.
- **SVG** (`frus-svg`):
  - a PNG or JPEG is decoded **when the picture is parsed**, once, through `frus-image`; every
    frame then draws the same pixels, under the same GPU cache key;
  - an SVG embedded in an SVG is drawn **as vectors**, its items moved into the outer picture —
    sharp at any size, like the rest, and compared against the reference renderer, which draws
    it too;
  - a GIF, a WebP or a broken bitmap is named `"image"` by `unsupported`;
  - **a picture never opens a file it names.** usvg's default reads an `href` that is not a
    `data:` address from the disk; a picture fetched from the network could otherwise show any
    PNG the application can read. Only `data:` addresses are read now.
  - The tint recolours shapes only: a bitmap in a recoloured icon keeps its own colours.
- **Lottie** (`frus-lottie`):
  - an asset embedded as a `data:` address is decoded when the file is parsed;
  - an asset the file names beside it (`"u": "images/", "p": "img_0.png"`) is the
    application's to give: `Animation::missing_images()` lists them by id and location, and
    `Animation::with_image(id, image)` gives each. The widget shows such an animation through
    `Lottie::new(handle)`;
  - pixels of another size than the asset's are fitted into it whole and centred, as the
    reference player does, and the layer's opacity fades the image;
  - an asset without pixels, or one that does not decode, is named `"image"`.

## Considered

- **Decoding GIF and WebP**: `frus-image` reads PNG and JPEG only, by choice (size); a picture
  with a GIF says so rather than growing every application.
- **Loading named files relative to the picture**: an application knows where its assets are
  and the parser does not — on the web and on Android there is no such path. Hence
  `with_image` for Lottie, and nothing for SVG, whose exporters embed.

## Not done yet

- A turned bitmap larger than the window once on screen loses what lies past the window's size,
  as any transformed layer does.
- `image-rendering: pixelated` is read and not honoured: every image is filtered.

## Verified

- `frus-core`: a moved and scaled image is one primitive at the right place; a turned one is
  drawn upright at its screen size in a layer whose map takes its corner where the image's goes;
  a map that flattens it draws nothing.
- `frus-svg`: a PNG decoded once (the same pixels painted twice), placed where the picture says;
  a turned one in a layer; an embedded SVG drawn as paths in its place; a GIF named; and a
  picture naming a PNG on the disk draws nothing — with usvg's default resolver put back, that
  test fails.
- `frus-lottie`: an embedded image where its layer puts it, faded by the layer; a named one
  listed, drawn once given, fitted into its asset; a broken one named.
- On the GPU (`frus-test`): an embedded SVG straight and turned matches the reference renderer;
  a bitmap straight and turned a quarter shows red and blue where they belong and nothing beside
  it; a Lottie image layer turned a quarter likewise.
- The 134 lottie-android samples all still paint; `Heart.json` now draws its heart (checked by
  eye on a GPU frame), and only the two files whose images sit in a folder beside them name
  `"image"`.
