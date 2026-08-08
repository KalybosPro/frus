# Jalon 87 — Arabic script (bidi): script rendering + off-screen RTL fix

## Goal

J84 laid down the RTL **layout mirroring** (rows/padding/overlays flipped). What
remained was the most visible part: **actually displaying Arabic**. This
milestone bundles an Arabic face, routes runs by script, and fixes a placement
bug that made all RTL text **invisible** on the device.

## What was done

### A bundled Arabic face
`NotoNaskhArabic-Regular.ttf` + `-Bold.ttf` (the "Noto Naskh Arabic" family)
loaded in `frus_text::new_font_system` alongside DejaVu. Bundled (rather than
resolved through the system) for **deterministic rendering everywhere**,
particularly on Android where no platform fallback list is populated.

### Routing by script (`family_for`)
DejaVu does not cover Arabic and **cosmic-text does no cross-family fallback on
Android**. So the family is chosen **at the source**: `family_for(text)` returns
the Naskh family if the text contains a character from the Arabic blocks
(0600–06FF, 0750–077F, 08A0–08FF, FB50–FDFF, FE70–FEFF), and the sans-serif
otherwise. Applied **identically** in measurement (`frus-text`) and in rendering
(`frus-gpu`), for plain **and** rich text.

### The fix: RTL text rendered off screen (the cause of the "blank")
The symptom: on the device the RTL layout mirrored correctly and the **Latin**
strings displayed, but **all the Arabic stayed blank** (title, filters, the
"العربية" menu label). Diagnosis on the device: shaping produced **real glyphs**
(7 glyphs, 0 `.notdef`) — so the font and the shaping were fine.

The real cause: for **non-paragraph** text (`max_width == None`), the renderer
bounded the buffer to the **surface's width** (`unwrap_or(width)`). But
cosmic-text **right-aligns an RTL run** within the buffer's width: the glyphs
landed at x ≈ surface_width, then were offset by `position.x` → **off screen to
the right**. Latin text (left-aligned, x ≈ 0) was unaffected — hence "Latin
visible, Arabic blank".

The fix (frus-gpu `text.rs`, the `Text` and `RichText` arms): pass `*max_width`
straight to `set_size` — **unconstrained** (`None`) for free text, never bounded
to the surface. A real paragraph keeps its layout width (RTL right-alignment
**within the box** is then correct, which is the expected behaviour).

### Demo: the Arabic locale
`i18n/ar.ftl` (title/filters + Arabic CLDR plurals zero/one/two/…), `LANGS` goes
to three entries (English / Français / العربية), and choosing Arabic
automatically enables the RTL theme (`lang_is_rtl` → `Theme::rtl`).

## Tests (frus-text + frus-gpu)

- `arabic_shapes_with_embedded_only_font_system`: reproduces the Android case (a
  bundled-only db, with **no** system fallback) → Arabic shapes real glyphs
  (`glyph_id != 0`) through `Family::Name`. It isolates font resolution from any
  platform fallback.
- `rtl_right_aligns_to_buffer_width`: **proves the cause** — a wide buffer ⇒ the
  first RTL glyph on the right (x > 500); unconstrained ⇒ on the left (x < 50).
- `renders_arabic_to_non_background_pixels` (frus-gpu, offscreen readback):
  Arabic does rasterise pixels.
- `arabic_falls_back_to_the_embedded_naskh_face` (measurement) kept.

## Validated on the device (Huawei STK-L21)

The العربية locale: the title **"مهامي"**, the filters
**"الكل / النشطة / المكتملة"**, the menu label **"العربية"** — all rendered, with
correct joining forms and RTL order, under a mirrored layout (hamburger on the
right, navigation reversed). ✔

## What's left

- Per-script font selection could be generalised (Hebrew, etc.) if needed.
- Per-locale date and number formatting (carried over from J86).
