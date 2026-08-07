//! `frus-text` — text measurement, and in time shaping, on top of
//! [`cosmic_text`](https://docs.rs/cosmic-text).
//!
//! At this milestone the API is limited to [`measure`]: the natural size of a line
//! of text, which the layout engine needs in order to size a `Text` widget.
//!
//! The `FontSystem`, which loads the fonts, is expensive: it is initialised
//! **lazily** and shared behind a `Mutex`. That is a pragmatic v1 choice;
//! unifying it with the renderer's own `FontSystem` will come later.

use std::sync::{Mutex, OnceLock};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, Weight};
use frus_core::{FontWeight, Point, Rect, Size, TextRun};

/// The default line-height to font-size ratio.
const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// The **bundled** fallback font (sans-serif) and its monospace variant. Bundling
/// them guarantees deterministic text rendering on **every** platform — Android
/// above all, where the system "sans-serif" alias, defined in `fonts.xml` and not
/// read by fontdb, resolves to no font at all.
const DEJAVU_SANS: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
/// The **bold, oblique and bold-oblique** faces are bundled too: cosmic-text demands
/// an **exact** style match on the primary family, so without an oblique face a
/// plain `.italic()` **panics** ("no default font found") anywhere only the bundled
/// fonts exist — caught on the Android device.
/// The full {400, 700} × {upright, italic} matrix makes every style the API can
/// reach safe and deterministic.
const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf");
const DEJAVU_SANS_OBLIQUE: &[u8] = include_bytes!("../assets/DejaVuSans-Oblique.ttf");
const DEJAVU_SANS_BOLD_OBLIQUE: &[u8] = include_bytes!("../assets/DejaVuSans-BoldOblique.ttf");
const DEJAVU_MONO: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
/// **Arabic** (Noto Naskh): DejaVu does not cover the Arabic script — it has no
/// contextual joining forms — so this face provides the fallback for Arabic runs.
/// It is bundled for deterministic rendering everywhere, Android included, where no
/// system font is loaded at all.
const NOTO_ARABIC: &[u8] = include_bytes!("../assets/NotoNaskhArabic-Regular.ttf");
const NOTO_ARABIC_BOLD: &[u8] = include_bytes!("../assets/NotoNaskhArabic-Bold.ttf");

/// The bundled fonts' internal family name; it must match the TTFs.
const SANS_FAMILY: &str = "DejaVu Sans";
const MONO_FAMILY: &str = "DejaVu Sans Mono";
/// The bundled Arabic face's family (Noto Naskh).
const ARABIC_FAMILY: &str = "Noto Naskh Arabic";

/// `true` when `text` holds at least one character of the **Arabic** script
/// (the Arabic, Supplement, Extended-A and Presentation Forms A/B blocks).
fn contains_arabic(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
    })
}

/// The **font family** to use for `text`: the bundled Arabic face when the text
/// contains Arabic, otherwise the default sans-serif.
///
/// This is essential because cosmic-text does **not** fall back across families on
/// Android, where the platform fallback lists are empty: without an explicit
/// assignment an Arabic run would render nothing. So the family is chosen by script
/// at the source, and measurement **and** rendering share the rule.
pub fn family_for(text: &str) -> cosmic_text::Family<'static> {
    if contains_arabic(text) {
        cosmic_text::Family::Name(ARABIC_FAMILY)
    } else {
        cosmic_text::Family::Name(SANS_FAMILY)
    }
}

/// Builds a ready-to-use `FontSystem`: the system fonts, which provide the emoji
/// and script fallbacks, **plus** the bundled font, set as the default family. Use
/// it everywhere a `FontSystem` is created — measurement here, rendering in
/// `frus-gpu` — for consistent text rendering that does not depend on system fonts,
/// which may have no resolvable default at all, as on Android.
pub fn new_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    let db = font_system.db_mut();
    db.load_font_data(DEJAVU_SANS.to_vec());
    db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
    db.load_font_data(DEJAVU_SANS_OBLIQUE.to_vec());
    db.load_font_data(DEJAVU_SANS_BOLD_OBLIQUE.to_vec());
    db.load_font_data(DEJAVU_MONO.to_vec());
    db.load_font_data(NOTO_ARABIC.to_vec());
    db.load_font_data(NOTO_ARABIC_BOLD.to_vec());
    // Makes every generic family resolve to a font that is actually present.
    db.set_sans_serif_family(SANS_FAMILY);
    db.set_serif_family(SANS_FAMILY);
    db.set_cursive_family(SANS_FAMILY);
    db.set_fantasy_family(SANS_FAMILY);
    db.set_monospace_family(MONO_FAMILY);
    font_system
}

fn font_system() -> &'static Mutex<FontSystem> {
    static FONT_SYSTEM: OnceLock<Mutex<FontSystem>> = OnceLock::new();
    FONT_SYSTEM.get_or_init(|| Mutex::new(new_font_system()))
}

/// The weight **actually available** among the bundled faces (400 or 700) closest to
/// the one requested. This is essential: cosmic-text demands an **exact** weight
/// match on the primary family, and a missing weight (Medium 500 on DejaVu) sends it
/// off to the platform fallback lists — which do not exist on Android ("no default
/// font found", a panic caught on the device). Routing every `Attrs` through here
/// is what makes rendering deterministic.
pub fn available_weight(weight: FontWeight) -> u16 {
    if weight.to_u16() < 550 {
        400
    } else {
        700
    }
}

/// The line height for a given font size, in pixels.
pub fn line_height(size_px: f32) -> f32 {
    size_px * LINE_HEIGHT_FACTOR
}

/// Measures a text's natural size (multiple lines allowed), in pixels, at regular
/// weight. See [`measure_styled`] for weight and italics.
pub fn measure(text: &str, size_px: f32) -> Size {
    measure_styled(text, size_px, FontWeight::Regular, false)
}

/// Measures a **styled** text's natural size; weight and italics count, since bold
/// is wider than regular and the layout has to know.
pub fn measure_styled(text: &str, size_px: f32, weight: FontWeight, italic: bool) -> Size {
    measure_wrapped(text, size_px, weight, italic, None)
}

/// Measures a styled text **under a width constraint**: beyond `max_width` the text
/// wraps and the height grows. `None` means unconstrained, giving the natural size.
/// This is the measurement wired into taffy's measure closure for paragraphs.
pub fn measure_wrapped(
    text: &str,
    size_px: f32,
    weight: FontWeight,
    italic: bool,
    max_width: Option<f32>,
) -> Size {
    let line_h = line_height(size_px);
    if text.is_empty() {
        return Size::new(0.0, line_h);
    }

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(size_px, line_h);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    // A constrained width (wrapping) or a free one; the height is always free.
    buffer.set_size(&mut font_system, max_width, None);
    let attrs = Attrs::new()
        .family(family_for(text))
        .weight(Weight(available_weight(weight)))
        .style(if italic { Style::Italic } else { Style::Normal });
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut width = 0.0_f32;
    let mut lines = 0.0_f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        lines += 1.0;
    }

    Size::new(width, lines.max(1.0) * line_h)
}

/// Measures a **rich text**'s natural size — resolved runs with mixed styles and
/// sizes: the longest line's width, and the shaped lines' real height.
pub fn measure_runs(runs: &[TextRun]) -> Size {
    measure_runs_wrapped(runs, None)
}

/// Measures a rich text **under a width constraint**: beyond `max_width` the runs
/// wrap. `None` means unconstrained.
pub fn measure_runs_wrapped(runs: &[TextRun], max_width: Option<f32>) -> Size {
    if runs.iter().all(|r| r.text.is_empty()) {
        return Size::new(0.0, 0.0);
    }
    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(base, line_height(base));
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, max_width, None);
    let spans = runs.iter().map(|run| {
        (
            run.text.as_str(),
            Attrs::new()
                .family(family_for(&run.text))
                .weight(Weight(available_weight(run.weight)))
                .style(if run.italic {
                    Style::Italic
                } else {
                    Style::Normal
                })
                .metrics(Metrics::new(run.size, line_height(run.size))),
        )
    });
    buffer.set_rich_text(&mut font_system, spans, Attrs::new(), Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        height = height.max(run.line_top + run.line_height);
    }
    Size::new(width, height)
}

/// One shaped line of a [`TextLayout`]: the `x` offsets of every **character
/// boundary**, taken from the real glyphs, kerning and ligatures included.
struct LayoutLine {
    /// The line's first boundary, as a character index global to the text.
    start_char: usize,
    /// The `x` of every character boundary on the line (`chars + 1` entries).
    offsets: Vec<f32>,
    /// The line's top edge.
    top: f32,
    /// The line's height.
    height: f32,
}

/// A text's **shaped** layout — a single cosmic-text pass — exposing the geometry an
/// editing widget needs: caret position by character index, reverse hit-testing, and
/// selection rectangles. Coordinates are **local** to the text, with the origin at
/// its top-left corner. Indices are in **characters** — frus's editing convention —
/// with boundaries `0..=len`.
///
/// Unlike a prefix measurement re-shaped substring by substring, the offsets come
/// from the **whole** shaped line: consistent with one another (kerning), and
/// computed in one pass instead of `n`.
pub struct TextLayout {
    lines: Vec<LayoutLine>,
    size: Size,
    /// The total character count — the last valid boundary.
    chars: usize,
}

impl TextLayout {
    /// Shapes `text` (multiple lines allowed, width unconstrained) in the given style
    /// and extracts its geometry.
    pub fn new(text: &str, size_px: f32, weight: FontWeight, italic: bool) -> Self {
        Self::wrapped(text, size_px, weight, italic, None)
    }

    /// Like [`TextLayout::new`], but **wraps** the text at `max_width` — a soft wrap,
    /// as in a multi-line field. `None` is unconstrained, where only `\n` breaks.
    ///
    /// Every **visual line** (one cosmic-text `LayoutRun`) is delimited by **its
    /// glyphs' bytes**, not by `run.text` — which is the whole *hard* line, repeated
    /// for each wrap. A soft wrap therefore fabricates no phantom character, and each
    /// line's `start_char` comes from its hard line's byte offset — exact indexing
    /// straight through the wraps.
    pub fn wrapped(
        text: &str,
        size_px: f32,
        weight: FontWeight,
        italic: bool,
        max_width: Option<f32>,
    ) -> Self {
        let fallback_h = line_height(size_px);
        let mut lines: Vec<LayoutLine> = Vec::new();
        let mut width = 0.0_f32;
        let mut height = 0.0_f32;

        if !text.is_empty() {
            // The byte offset of every **hard** line's start (those split by `\n`).
            let hard: Vec<&str> = text.split('\n').collect();
            let mut line_byte_start = Vec::with_capacity(hard.len());
            {
                let mut b = 0usize;
                for l in &hard {
                    line_byte_start.push(b);
                    b += l.len() + 1; // +1 for the `\n`
                }
            }

            let mut font_system = font_system().lock().expect("FontSystem lock");
            let metrics = Metrics::new(size_px, fallback_h);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, max_width, None);
            let attrs = Attrs::new()
                .family(family_for(text))
                .weight(Weight(available_weight(weight)))
                .style(if italic { Style::Italic } else { Style::Normal });
            buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
            buffer.shape_until_scroll(&mut font_system, false);

            // Collect the runs so we can look at the next one, which ends this segment.
            let runs: Vec<_> = buffer.layout_runs().collect();
            for (idx, run) in runs.iter().enumerate() {
                let line_text = hard[run.line_i];
                let first_of_line = idx == 0 || runs[idx - 1].line_i != run.line_i;
                // The segment [lo, hi) of the hard line, in bytes, that this visual line
                // carries: from its first glyph (0 if it is the first visual line) to the
                // first glyph of the next visual line of the same hard line, or the end
                // otherwise — which takes in the break space, absent from the glyphs.
                let lo = if first_of_line {
                    0
                } else {
                    run.glyphs.iter().map(|g| g.start).min().unwrap_or(0)
                };
                let hi = runs
                    .get(idx + 1)
                    .filter(|r| r.line_i == run.line_i)
                    .and_then(|r| r.glyphs.iter().map(|g| g.start).min())
                    .unwrap_or(line_text.len());

                // The segment's character boundaries, in bytes relative to the hard line.
                let span = &line_text[lo..hi];
                let char_bytes: Vec<usize> = span.char_indices().map(|(b, _)| lo + b).collect();
                let n = char_bytes.len();
                let mut offsets = vec![f32::NAN; n + 1];
                offsets[0] = 0.0;

                // Each glyph covers a byte cluster [start, end) over [x, x+w), where x is
                // **local to the visual line**; interior boundaries are interpolated.
                for glyph in run.glyphs.iter() {
                    let covered: Vec<usize> = (0..n)
                        .filter(|&i| char_bytes[i] >= glyph.start && char_bytes[i] < glyph.end)
                        .collect();
                    let k = covered.len().max(1) as f32;
                    for (j, &i) in covered.iter().enumerate() {
                        offsets[i] = glyph.x + glyph.w * (j as f32 / k);
                    }
                    if let Some(next) = (0..=n).find(|&i| {
                        let b = if i == n {
                            lo + span.len()
                        } else {
                            char_bytes[i]
                        };
                        b == glyph.end
                    }) {
                        offsets[next] = glyph.x + glyph.w;
                    }
                }
                // Boundaries left without a glyph (the break space) keep continuity.
                for i in 1..=n {
                    if offsets[i].is_nan() {
                        offsets[i] = offsets[i - 1];
                    }
                }

                let start_char = text[..line_byte_start[run.line_i] + lo].chars().count();
                width = width.max(run.line_w);
                height = height.max(run.line_top + run.line_height);
                lines.push(LayoutLine {
                    start_char,
                    offsets,
                    top: run.line_top,
                    height: run.line_height,
                });
            }
        }

        // Empty text, or no shaped line: a synthetic empty line keeps caret and hit
        // defined, with the caret at x = 0.
        if lines.is_empty() {
            lines.push(LayoutLine {
                start_char: 0,
                offsets: vec![0.0],
                top: 0.0,
                height: fallback_h,
            });
            height = fallback_h;
        }

        let chars = text.chars().count();
        Self {
            lines,
            size: Size::new(width, height),
            chars,
        }
    }

    /// The shaped text's natural size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The line containing the character boundary `index`.
    fn line_of(&self, index: usize) -> &LayoutLine {
        self.lines
            .iter()
            .rev()
            .find(|line| index >= line.start_char)
            .unwrap_or(&self.lines[0])
    }

    /// The caret's rectangle at boundary `index`, of zero width — it is up to the
    /// widget to choose the stroke thickness. `index` is clamped to the text.
    pub fn caret_rect(&self, index: usize) -> Rect {
        let index = index.min(self.chars);
        let line = self.line_of(index);
        let local = (index - line.start_char).min(line.offsets.len() - 1);
        Rect::new(line.offsets[local], line.top, 0.0, line.height)
    }

    /// The character boundary **closest** to `point`, in text-local coordinates. The
    /// `y` picks the line (clamped), the `x` picks the boundary.
    pub fn hit_test(&self, point: Point) -> usize {
        let line = self
            .lines
            .iter()
            .find(|line| point.y < line.top + line.height)
            .unwrap_or(self.lines.last().expect("at least one line"));

        let mut best = 0;
        let mut best_dist = f32::MAX;
        for (i, x) in line.offsets.iter().enumerate() {
            let dist = (x - point.x).abs();
            if dist < best_dist {
                best_dist = dist;
                best = i;
            }
        }
        (line.start_char + best).min(self.chars)
    }

    /// The rectangles covering the character range `[start, end)`, one per line
    /// crossed; empty ones are omitted.
    pub fn selection_rects(&self, start: usize, end: usize) -> Vec<Rect> {
        let (start, end) = (start.min(self.chars), end.min(self.chars));
        if start >= end {
            return Vec::new();
        }
        let mut rects = Vec::new();
        for line in &self.lines {
            let line_len = line.offsets.len() - 1;
            let lo = start.max(line.start_char);
            let hi = end.min(line.start_char + line_len);
            if lo >= hi {
                continue;
            }
            let x0 = line.offsets[lo - line.start_char];
            let x1 = line.offsets[hi - line.start_char];
            if x1 > x0 {
                rects.push(Rect::new(x0, line.top, x1 - x0, line.height));
            }
        }
        rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_zero_width() {
        let size = measure("", 16.0);
        assert_eq!(size.width, 0.0);
        assert!(size.height > 0.0);
    }

    #[test]
    fn non_empty_text_has_positive_size() {
        let size = measure("Bonjour", 24.0);
        assert!(size.width > 0.0, "width = {}", size.width);
        assert!(size.height > 0.0, "height = {}", size.height);
    }

    #[test]
    fn rich_runs_measure_mixed_styles() {
        use frus_core::Color;
        let run = |text: &str, size: f32, weight: FontWeight| TextRun {
            text: text.to_string(),
            size,
            weight,
            italic: false,
            color: Color::WHITE,
            decoration: frus_core::TextDecoration::NONE,
            decoration_color: None,
        };
        // "normal BOLD" is wider than "normal" alone; the height comes from the
        // largest run.
        let plain = measure_runs(&[run("normal", 16.0, FontWeight::Regular)]);
        let mixed = measure_runs(&[
            run("normal", 16.0, FontWeight::Regular),
            run(" BOLD", 24.0, FontWeight::Bold),
        ]);
        assert!(mixed.width > plain.width);
        assert!(
            mixed.height >= line_height(24.0) - 1.0,
            "height driven by the 24px run: {}",
            mixed.height
        );
        // Empty gives a zero size.
        assert_eq!(measure_runs(&[]), Size::new(0.0, 0.0));
    }

    #[test]
    fn wrapped_text_grows_taller_within_the_width() {
        let text = "a text that is long enough to wrap onto several lines";
        let free = measure_wrapped(text, 16.0, FontWeight::Regular, false, None);
        let narrow = measure_wrapped(text, 16.0, FontWeight::Regular, false, Some(120.0));
        assert!(
            narrow.width <= 120.0,
            "wrapped within the width: {}",
            narrow.width
        );
        assert!(narrow.height > free.height, "wrapping grows the height");
    }

    #[test]
    fn layout_offsets_are_monotonic_and_match_size() {
        let layout = TextLayout::new("Bonjour le monde", 18.0, FontWeight::Regular, false);
        let mut prev = -1.0;
        for i in 0..=16 {
            let x = layout.caret_rect(i).x;
            assert!(x >= prev, "offset decreasing at boundary {i}");
            prev = x;
        }
        // The last boundary reaches the natural width.
        assert!((layout.caret_rect(16).x - layout.size().width).abs() < 0.5);
        // A boundary past the end of the text is clamped.
        assert_eq!(layout.caret_rect(99).x, layout.caret_rect(16).x);
    }

    #[test]
    fn hit_test_roundtrips_caret_positions() {
        let layout = TextLayout::new("Bonjour", 18.0, FontWeight::Regular, false);
        for i in 0..=7 {
            let caret = layout.caret_rect(i);
            let hit = layout.hit_test(Point::new(caret.x, caret.y + 1.0));
            assert_eq!(hit, i, "caret->hit round trip at boundary {i}");
        }
        // Far to the left or right: the bounds.
        assert_eq!(layout.hit_test(Point::new(-100.0, 0.0)), 0);
        assert_eq!(layout.hit_test(Point::new(10_000.0, 0.0)), 7);
    }

    #[test]
    fn selection_rects_cover_the_range() {
        let layout = TextLayout::new("Bonjour", 18.0, FontWeight::Regular, false);
        let rects = layout.selection_rects(2, 5);
        assert_eq!(rects.len(), 1);
        let r = rects[0];
        assert!((r.x - layout.caret_rect(2).x).abs() < 0.01);
        assert!((r.x + r.width - layout.caret_rect(5).x).abs() < 0.01);
        assert!(r.height > 0.0);
        // An empty or inverted range gives no rectangle.
        assert!(layout.selection_rects(3, 3).is_empty());
        assert!(layout.selection_rects(5, 2).is_empty());
    }

    #[test]
    fn multiline_layout_maps_lines_and_indices() {
        // "ab\ncd": boundaries 0..=2 on line 1, 3..=5 on line 2.
        let layout = TextLayout::new("ab\ncd", 18.0, FontWeight::Regular, false);
        let first = layout.caret_rect(0);
        let second = layout.caret_rect(3);
        assert!(second.y > first.y, "the 2nd line sits lower");
        assert_eq!(second.x, 0.0, "second line starts at x = 0");
        // A hit on the second line gives second-line indices.
        let hit = layout.hit_test(Point::new(0.0, second.y + 1.0));
        assert_eq!(hit, 3);
    }

    #[test]
    fn soft_wrap_indexes_chars_correctly_across_lines() {
        // A soft wrap with no `\n`: each word on its own visual line. Indexing must
        // stay exact — an index's caret lands on the right line, and the line starts
        // are contiguous, with no phantom character at the break.
        let text = "aaaa bbbb cccc dddd"; // 19 characters, one word per line at 60px
        let layout = TextLayout::wrapped(text, 18.0, FontWeight::Regular, false, Some(60.0));
        assert!(
            layout.size().height > line_height(18.0) * 2.0,
            "several visual lines"
        );

        // Each word's start: "aaaa"@0, "bbbb"@5, "cccc"@10, "dddd"@15 — each at x
        // about 0, the start of its line, on lines of increasing y.
        let mut prev_y = -1.0;
        for &start in &[0usize, 5, 10, 15] {
            let c = layout.caret_rect(start);
            assert!(c.x < 1.0, "word start {start} at x about 0 (x={})", c.x);
            assert!(c.y > prev_y, "lines of increasing y at {start}");
            prev_y = c.y;
        }
        // A point in the **middle** of a wrapped line round-trips (index 11 is the
        // second 'c' of "cccc", clearly not a break boundary).
        let c = layout.caret_rect(11);
        assert_eq!(layout.hit_test(Point::new(c.x, c.y + 1.0)), 11);
        // The last boundary is 19: the three wraps inject no stray +1.
        assert_eq!(layout.caret_rect(19).x, layout.caret_rect(99).x);
        assert_eq!(layout.caret_rect(19).y, layout.caret_rect(99).y);
    }

    #[test]
    fn empty_layout_keeps_caret_at_origin() {
        let layout = TextLayout::new("", 18.0, FontWeight::Regular, false);
        let caret = layout.caret_rect(0);
        assert_eq!(caret.x, 0.0);
        assert!(caret.height > 0.0, "wrapped line height");
        assert_eq!(layout.hit_test(Point::new(50.0, 0.0)), 0);
    }

    /// Reproduces the Android worst case, caught on the device: **no** usable system
    /// font at all, only the bundled faces. Shaping must never panic ("no default
    /// font found") for **any** weight × italic combination the API can reach.
    /// cosmic-text demands an *exact* style and weight match on the primary family:
    /// without a bundled oblique face, `.italic()` panicked on the device.
    #[test]
    fn embedded_only_font_system_shapes_every_style() {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
        db.load_font_data(DEJAVU_SANS_OBLIQUE.to_vec());
        db.load_font_data(DEJAVU_SANS_BOLD_OBLIQUE.to_vec());
        db.load_font_data(DEJAVU_MONO.to_vec());
        db.set_sans_serif_family(SANS_FAMILY);
        db.set_serif_family(SANS_FAMILY);
        db.set_cursive_family(SANS_FAMILY);
        db.set_fantasy_family(SANS_FAMILY);
        db.set_monospace_family(MONO_FAMILY);
        let mut fs = FontSystem::new_with_locale_and_db("en-TG".to_string(), db);

        for weight in [
            FontWeight::Regular,
            FontWeight::Medium,
            FontWeight::SemiBold,
            FontWeight::Bold,
        ] {
            for italic in [false, true] {
                let attrs = Attrs::new()
                    .weight(Weight(available_weight(weight)))
                    .style(if italic { Style::Italic } else { Style::Normal });
                let mut buffer = Buffer::new(&mut fs, Metrics::new(20.0, 24.0));
                buffer.set_size(&mut fs, None, None);
                buffer.set_text(&mut fs, "Nothing to show", attrs, Shaping::Advanced);
                buffer.shape_until_scroll(&mut fs, false); // panics here if broken
                let w: f32 = buffer.layout_runs().map(|r| r.line_w).fold(0.0, f32::max);
                assert!(w > 0.0, "weight {weight:?} italic {italic}: nothing shaped");
            }
        }
    }

    /// Reproduces the Android Arabic case **exactly**: the bundled db alone, with no
    /// system font and therefore no platform fallback list, with the Noto Naskh face
    /// loaded. `family_for` must route the Arabic run to the Naskh family, and shaping
    /// must produce **real** glyphs (non-zero ids), not `.notdef`. If `Family::Name`
    /// fails to resolve here, the glyphs come out empty — the blank seen on the
    /// device.
    #[test]
    fn arabic_shapes_with_embedded_only_font_system() {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(NOTO_ARABIC.to_vec());
        db.load_font_data(NOTO_ARABIC_BOLD.to_vec());
        db.set_sans_serif_family(SANS_FAMILY);
        let mut fs = FontSystem::new_with_locale_and_db("en-TG".to_string(), db);

        let text = "مهامي";
        let attrs = Attrs::new().family(family_for(text));
        let mut buffer = Buffer::new(&mut fs, Metrics::new(40.0, 48.0));
        buffer.set_size(&mut fs, None, None);
        buffer.set_text(&mut fs, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut fs, false);

        let mut glyphs = 0usize;
        let mut real = 0usize; // glyphs whose glyph_id != 0, so not .notdef
        for run in buffer.layout_runs() {
            for g in run.glyphs.iter() {
                glyphs += 1;
                if g.glyph_id != 0 {
                    real += 1;
                }
            }
        }
        assert!(glyphs > 0, "no glyph shaped for Arabic");
        assert!(
            real > 0,
            "only .notdef ({glyphs} glyphs, 0 real): Family::Name(\"{ARABIC_FAMILY}\") did not resolve the Naskh face"
        );
    }

    /// A positioning diagnosis: an RTL run in a **wide** buffer (width = the surface)
    /// gets **right-aligned** by cosmic-text, so the glyphs land near the right edge
    /// (x about the width) and therefore off-screen once shifted by `position.x`.
    /// With no width constraint (`None`) they start at x about 0. This is the cause
    /// of the blank Arabic text on the device.
    #[test]
    fn rtl_right_aligns_to_buffer_width() {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(NOTO_ARABIC.to_vec());
        db.set_sans_serif_family(SANS_FAMILY);
        let mut fs = FontSystem::new_with_locale_and_db("en-TG".to_string(), db);
        let text = "العربية";
        let attrs = Attrs::new().family(family_for(text));

        let mut first_glyph_x = |width: Option<f32>| {
            let mut buffer = Buffer::new(&mut fs, Metrics::new(40.0, 48.0));
            buffer.set_size(&mut fs, width, Some(200.0));
            buffer.set_text(&mut fs, text, attrs.clone(), Shaping::Advanced);
            buffer.shape_until_scroll(&mut fs, false);
            buffer
                .layout_runs()
                .flat_map(|r| r.glyphs.iter().map(|g| g.x))
                .fold(f32::MAX, f32::min)
        };

        let wide = first_glyph_x(Some(1080.0));
        let free = first_glyph_x(None);
        assert!(wide > 500.0, "a wide RTL run should push right (x={wide})");
        assert!(
            free < 50.0,
            "an unconstrained RTL run should start left (x={free})"
        );
    }

    #[test]
    fn arabic_falls_back_to_the_embedded_naskh_face() {
        // DejaVu does not cover Arabic: the bundled fallback (Noto Naskh) has to take
        // over and **shape** the glyphs, giving a non-zero, sensible width. A missing
        // fallback would give 0, or .notdef glyphs.
        let hello = "مرحبا بالعالم"; // "hello world"
        let m = measure(hello, 24.0);
        assert!(m.width > 60.0, "Arabic must shape (width {})", m.width);
        assert!(m.height > 0.0);

        // The fallback does not trample Latin: "Hello" keeps a coherent width.
        let latin = measure("Hello", 24.0);
        assert!(latin.width > 0.0);

        // Mixed bidi (Arabic plus Latin digits) is measured as one piece, without
        // panicking; cosmic-text reorders internally.
        let mixed = measure("قيمة 42 نقطة", 24.0);
        assert!(mixed.width > 0.0, "mixed bidi text was not shaped");
    }

    #[test]
    fn weights_snap_to_embedded_faces() {
        // A weight with no exact face (Medium 500) MUST fall back to a bundled face —
        // otherwise cosmic-text switches to platform fallback lists, which do not
        // exist on Android (a panic on the device).
        assert_eq!(available_weight(FontWeight::Regular), 400);
        assert_eq!(available_weight(FontWeight::Medium), 400);
        assert_eq!(available_weight(FontWeight::SemiBold), 700);
        assert_eq!(available_weight(FontWeight::Bold), 700);
        // And shaping is deterministic: Medium measures like Regular (the same face),
        // and SemiBold like Bold.
        let text = "Titre de section";
        let regular = measure_styled(text, 20.0, FontWeight::Regular, false);
        let medium = measure_styled(text, 20.0, FontWeight::Medium, false);
        assert_eq!(medium.width, regular.width);
        let semibold = measure_styled(text, 20.0, FontWeight::SemiBold, false);
        let bold = measure_styled(text, 20.0, FontWeight::Bold, false);
        assert_eq!(semibold.width, bold.width);
        assert!(bold.width > regular.width);
    }

    #[test]
    fn bold_measures_wider_than_regular() {
        // The bundled bold face must actually be chosen: bold is wider than regular
        // at the same size.
        let regular = measure_styled("Bonjour le monde", 24.0, FontWeight::Regular, false);
        let bold = measure_styled("Bonjour le monde", 24.0, FontWeight::Bold, false);
        assert!(
            bold.width > regular.width,
            "gras {} <= normal {}",
            bold.width,
            regular.width
        );
    }
}
