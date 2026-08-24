//! `frus-text` — text measurement, and in time shaping, on top of
//! [`cosmic_text`](https://docs.rs/cosmic-text).
//!
//! At this milestone the API is limited to [`measure`]: the natural size of a line
//! of text, which the layout engine needs in order to size a `Text` widget.
//!
//! The `FontSystem`, which loads the fonts, is expensive: it is initialised
//! **lazily** and shared behind a `Mutex`. That is a pragmatic v1 choice;
//! unifying it with the renderer's own `FontSystem` will come later.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, Weight};
use frus_core::{FontWeight, Point, Rect, ResolvedTextStyle, Size, TextRun, TextStyle};

/// The default line-height to font-size ratio.
const LINE_HEIGHT_FACTOR: f32 = 1.2;

// --- The bundled fonts ---
//
// Bundling guarantees deterministic text on **every** platform — Android above all,
// where the system "sans-serif" alias, defined in `fonts.xml` and not read by fontdb,
// resolves to no font at all.
//
// It is also the single largest thing frus puts in an application: about 3.4 MB of
// faces, ~1.8 MB once the APK compresses them, which is roughly **40%** of a minimal
// frus app (milestone 292). So each group is a cargo feature, on by default, and an
// application that ships its own faces can turn off the ones it does not need:
//
// ```toml
// frus = { version = "0.1", default-features = false, features = ["bundled-sans"] }
// ```
//
// Turning one off never turns an API into a panic — see [`available_style`] and
// [`family_for`], which resolve to what is actually loaded.

/// The bundled sans-serif, regular and bold. cosmic-text demands an **exact** weight
/// match on the primary family, so both are needed for `.bold()` to shape rather than
/// fall through to platform lists that do not exist on Android.
#[cfg(feature = "bundled-sans")]
const DEJAVU_SANS: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
#[cfg(feature = "bundled-sans")]
const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../assets/DejaVuSans-Bold.ttf");
/// The **oblique** faces, for the same reason one step further: cosmic-text demands an
/// exact *style* match too, so without them a plain `.italic()` used to **panic** ("no
/// default font found") — caught on the Android device. They are 1.28 MB of the 3.4,
/// so they are separable; when they are absent italic text is rendered **upright**
/// rather than not at all.
#[cfg(feature = "bundled-italic")]
const DEJAVU_SANS_OBLIQUE: &[u8] = include_bytes!("../assets/DejaVuSans-Oblique.ttf");
#[cfg(feature = "bundled-italic")]
const DEJAVU_SANS_BOLD_OBLIQUE: &[u8] = include_bytes!("../assets/DejaVuSans-BoldOblique.ttf");
/// The monospace face — only ever reached by widgets that ask for it (`Kbd`, code).
#[cfg(feature = "bundled-mono")]
const DEJAVU_MONO: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
/// **Arabic** (Noto Naskh): DejaVu does not cover the Arabic script — it has no
/// contextual joining forms — so this face provides the fallback for Arabic runs.
#[cfg(feature = "bundled-arabic")]
const NOTO_ARABIC: &[u8] = include_bytes!("../assets/NotoNaskhArabic-Regular.ttf");
#[cfg(feature = "bundled-arabic")]
const NOTO_ARABIC_BOLD: &[u8] = include_bytes!("../assets/NotoNaskhArabic-Bold.ttf");

/// The bundled fonts' internal family names; they must match the TTFs.
#[cfg(feature = "bundled-sans")]
const SANS_FAMILY: &str = "DejaVu Sans";
#[cfg(feature = "bundled-mono")]
const MONO_FAMILY: &str = "DejaVu Sans Mono";
/// The bundled Arabic face's family (Noto Naskh).
#[cfg(feature = "bundled-arabic")]
const ARABIC_FAMILY: &str = "Noto Naskh Arabic";

// --- The families text actually resolves to ---
//
// `None` means "nothing is loaded for this role": text falls back to the generic
// family and lets the platform answer, which is the best that can be done and is what
// the desktop wants anyway. An application that ships its own faces names them here
// through [`set_default_family`] / [`set_monospace_family`].

#[cfg(feature = "bundled-sans")]
static SANS: RwLock<Option<&'static str>> = RwLock::new(Some(SANS_FAMILY));
#[cfg(not(feature = "bundled-sans"))]
static SANS: RwLock<Option<&'static str>> = RwLock::new(None);
#[cfg(feature = "bundled-mono")]
static MONO: RwLock<Option<&'static str>> = RwLock::new(Some(MONO_FAMILY));
#[cfg(not(feature = "bundled-mono"))]
static MONO: RwLock<Option<&'static str>> = RwLock::new(None);
#[cfg(feature = "bundled-arabic")]
static ARABIC: RwLock<Option<&'static str>> = RwLock::new(Some(ARABIC_FAMILY));
#[cfg(not(feature = "bundled-arabic"))]
static ARABIC: RwLock<Option<&'static str>> = RwLock::new(None);
/// Whether an italic face exists for the default family. Set when the font system is
/// built, by looking at what was actually loaded rather than at what was asked for.
static ITALIC: AtomicBool = AtomicBool::new(cfg!(feature = "bundled-italic"));

/// Faces the application supplies itself. Every `FontSystem` frus builds loads them,
/// so they must be registered **before the application runs** — the renderer builds
/// its own system at start-up, exactly like declaring fonts in a manifest.
static EXTRA_FONTS: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Registers a font face from its bytes (TTF/OTF), for the whole process.
///
/// Call it **before** starting the application: the renderer builds its font database
/// at start-up and a face added afterwards will measure but not paint.
///
/// ```ignore
/// frus_text::add_font(include_bytes!("../fonts/Inter-Regular.ttf").to_vec());
/// frus_text::set_default_family("Inter");
/// ```
pub fn add_font(data: Vec<u8>) {
    // Anything already measuring gets it too, so tests and tools do not have to care
    // about ordering.
    if let Some(system) = FONT_SYSTEM.get() {
        if let Ok(mut system) = system.lock() {
            system.db_mut().load_font_data(data.clone());
        }
    }
    EXTRA_FONTS.lock().expect("font registry").push(data);
    forget_measurements();
}

/// Names the family text uses by default — the one an application ships instead of,
/// or alongside, the bundled sans. The face itself must be registered with
/// [`add_font`], or be present on the system.
pub fn set_default_family(name: &'static str) {
    *SANS.write().expect("family registry") = Some(name);
    forget_measurements();
}

/// Names the family monospaced text uses. See [`set_default_family`].
pub fn set_monospace_family(name: &'static str) {
    *MONO.write().expect("family registry") = Some(name);
    forget_measurements();
}

/// The family a generic role resolves to, or the generic family when nothing is
/// loaded for it and the platform is the better judge.
fn family_or_generic(
    slot: &RwLock<Option<&'static str>>,
    generic: cosmic_text::Family<'static>,
) -> cosmic_text::Family<'static> {
    match *slot.read().expect("family registry") {
        Some(name) => cosmic_text::Family::Name(name),
        None => generic,
    }
}

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
        // No Arabic face loaded: the sans is a better guess than a family name that
        // resolves to nothing, and on the desktop the system usually has one.
        match *ARABIC.read().expect("family registry") {
            Some(name) => cosmic_text::Family::Name(name),
            None => family_or_generic(&SANS, cosmic_text::Family::SansSerif),
        }
    } else {
        family_or_generic(&SANS, cosmic_text::Family::SansSerif)
    }
}

/// The monospaced family, for the widgets that ask for one.
pub fn monospace_family() -> cosmic_text::Family<'static> {
    family_or_generic(&MONO, cosmic_text::Family::Monospace)
}

/// The style **actually available** for the default family. Italic when an oblique
/// face is loaded, upright when none is — because cosmic-text demands an exact style
/// match, and asking for one that does not exist sends it to platform fallback lists
/// that are empty on Android. Turning off `bundled-italic` should cost you slanted
/// text, not text.
pub fn available_style(italic: bool) -> Style {
    if italic && ITALIC.load(Ordering::Relaxed) {
        Style::Italic
    } else {
        Style::Normal
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
    #[cfg(feature = "bundled-sans")]
    {
        db.load_font_data(DEJAVU_SANS.to_vec());
        db.load_font_data(DEJAVU_SANS_BOLD.to_vec());
    }
    #[cfg(feature = "bundled-italic")]
    {
        db.load_font_data(DEJAVU_SANS_OBLIQUE.to_vec());
        db.load_font_data(DEJAVU_SANS_BOLD_OBLIQUE.to_vec());
    }
    #[cfg(feature = "bundled-mono")]
    db.load_font_data(DEJAVU_MONO.to_vec());
    #[cfg(feature = "bundled-arabic")]
    {
        db.load_font_data(NOTO_ARABIC.to_vec());
        db.load_font_data(NOTO_ARABIC_BOLD.to_vec());
    }
    // Whatever the application supplied itself, over the bundled faces.
    for face in EXTRA_FONTS.lock().expect("font registry").iter() {
        db.load_font_data(face.clone());
    }
    // Makes every generic family resolve to a font that is actually present — but
    // only when one is: pointing `sans-serif` at a family nobody loaded is worse than
    // leaving fontdb's own answer alone.
    if let Some(sans) = *SANS.read().expect("family registry") {
        db.set_sans_serif_family(sans);
        db.set_serif_family(sans);
        db.set_cursive_family(sans);
        db.set_fantasy_family(sans);
    }
    if let Some(mono) = *MONO.read().expect("family registry") {
        db.set_monospace_family(mono);
    }
    // What is *actually* there decides whether italic is asked for, since an
    // application may have supplied an oblique face of its own — or none.
    ITALIC.store(has_italic_face(db), Ordering::Relaxed);
    font_system
}

/// Does the database hold an oblique or italic face for the default family?
fn has_italic_face(db: &cosmic_text::fontdb::Database) -> bool {
    let sans = *SANS.read().expect("family registry");
    db.faces().any(|face| {
        face.style != cosmic_text::fontdb::Style::Normal
            && match sans {
                Some(name) => face.families.iter().any(|(family, _)| family == name),
                None => true,
            }
    })
}

static FONT_SYSTEM: OnceLock<Mutex<FontSystem>> = OnceLock::new();

fn font_system() -> &'static Mutex<FontSystem> {
    FONT_SYSTEM.get_or_init(|| Mutex::new(new_font_system()))
}

/// Everything about a measurement that can change its answer. The weight and the
/// style are the **resolved** ones — what the database can actually serve — so a
/// Medium asked for on a family that only has Regular hits the same entry as the
/// Regular, which is the same shaping either way. Floats are keyed by their bits:
/// two sizes that are bit-identical measure identically, and no other pair needs to
/// share an entry.
#[derive(PartialEq, Eq, Hash)]
struct MeasureKey {
    text: String,
    size: u32,
    weight: u16,
    italic: bool,
    max_width: Option<u32>,
}

/// How many entries a generation holds before it is retired. Two generations live at
/// once, so this bounds the cache at roughly `2 × CAP` measurements — a few hundred
/// kilobytes, against the ~290 µs a twelve-row screen was spending per frame
/// re-measuring strings that had not changed (milestone 299).
const CACHE_CAP: usize = 2048;

/// A two-generation cache. A string still being drawn is found in `current` or
/// promoted out of `previous`, so it survives a rotation; a string that has gone —
/// last second's clock, yesterday's search box — falls out with the generation it
/// was in. That is the whole eviction policy, and it needs no timestamps.
#[derive(Default)]
struct MeasureCache {
    current: std::collections::HashMap<MeasureKey, Size>,
    previous: std::collections::HashMap<MeasureKey, Size>,
}

static MEASURE_CACHE: Mutex<Option<MeasureCache>> = Mutex::new(None);

/// Forgets every measurement. Called whenever what a face measures to can have
/// changed — a font registered, a family renamed — because an answer from before
/// that is simply wrong, not merely stale.
fn forget_measurements() {
    if let Ok(mut cache) = MEASURE_CACHE.lock() {
        *cache = None;
    }
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

/// What a baseline depends on: the size and the **resolved** weight and style. The
/// text does not enter into it — a baseline is a property of the font at a size, not of
/// what is written in it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BaselineKey {
    size: u32,
    weight: u16,
    italic: bool,
}

static BASELINES: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<BaselineKey, f32>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// The line height for a given font size, in pixels.
pub fn line_height(size_px: f32) -> f32 {
    size_px * LINE_HEIGHT_FACTOR
}

/// The **alphabetic baseline** of a line of text at `size_px`: the distance from the
/// top of the line box down to the line the letters sit on.
///
/// It is the number two pieces of text need in order to agree on where "the same line"
/// is. Sizes and line heights differ, so aligning two runs by their tops or their
/// bottoms puts them at different heights; aligning them by this puts them where a
/// reader expects.
///
/// It is **shaped**, not derived from the size. The ascent belongs to the font the
/// fallback chain actually chose, and guessing it from the point size is wrong by a few
/// per cent per family — enough to see when two labels sit side by side, which is the
/// only situation this number is ever used in.
pub fn baseline(size_px: f32, weight: FontWeight, italic: bool) -> f32 {
    // The system is built before the key is resolved, for the reason `measure_wrapped`
    // gives: resolving reads state that building it sets.
    let _ = font_system();
    let key = BaselineKey {
        size: size_px.to_bits(),
        weight: available_weight(weight),
        italic: available_style(italic) == Style::Italic,
    };
    if let Some(cached) = BASELINES.lock().expect("baseline cache").get(&key) {
        return *cached;
    }

    let line_h = line_height(size_px);
    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(size_px, line_h);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    // Two letters with an ascender and a descender, so the line is shaped at its full
    // height rather than at the height of whatever happens to be in it.
    let probe = "Hg";
    let attrs = Attrs::new()
        .family(family_for(probe))
        .weight(Weight(available_weight(weight)))
        .style(available_style(italic));
    buffer.set_text(&mut font_system, probe, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);
    // `line_y` is where the renderer puts the baseline, so taking it from the same
    // place is what keeps layout and paint talking about the same line.
    let baseline = buffer
        .layout_runs()
        .next()
        .map(|run| run.line_y)
        .unwrap_or(size_px);
    drop(font_system);
    BASELINES
        .lock()
        .expect("baseline cache")
        .insert(key, baseline);
    baseline
}

/// The first baseline of a sequence of styled runs: the **largest** of their
/// baselines, since they share a line and the tallest ascender sets where it sits.
pub fn baseline_of_runs(runs: &[TextRun]) -> Option<f32> {
    runs.iter()
        .map(|run| baseline(run.size, run.weight, run.italic))
        .fold(None, |acc: Option<f32>, b| {
            Some(acc.map_or(b, |a| a.max(b)))
        })
}

/// Measures a text's natural size (multiple lines allowed), in pixels, at regular
/// weight. See [`measure_styled`] for weight and italics.
pub fn measure(text: &str, size_px: f32) -> Size {
    measure_styled(text, size_px, FontWeight::Regular, false)
}

/// Measures `text` under a [`TextStyle`], **resolving whatever it left open** to the
/// framework's own defaults.
///
/// Every widget that measures a label wants this and used to spell it out as three
/// arguments. Three arguments is three chances to pass a size from one style and a weight
/// from another, which draws text the layout never measured.
pub fn measure_style(text: &str, style: TextStyle) -> Size {
    measure_resolved(text, &style.resolved())
}

/// Measures `text` under a style that has **already** been resolved — the same thing one
/// step further down, for the code that has done the resolving once and is measuring
/// several times against it.
pub fn measure_resolved(text: &str, style: &ResolvedTextStyle) -> Size {
    measure_styled(text, style.size, style.weight, style.italic)
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

    // The key records the **resolved** weight and style, and resolving reads state
    // that building the font system sets (`ITALIC`, from the faces actually loaded).
    // So the system is built first: otherwise the very first measurement would be
    // filed under what was true before it existed.
    let _ = font_system();

    // Shaping the same string again every frame was three quarters of what building a
    // frame cost (milestone 299). A hit here also skips the `FontSystem` lock, which
    // the renderer wants at the same time.
    let key = MeasureKey {
        text: text.to_string(),
        size: size_px.to_bits(),
        weight: available_weight(weight),
        italic: available_style(italic) == Style::Italic,
        max_width: max_width.map(f32::to_bits),
    };
    if let Some(size) = cached_measurement(&key) {
        return size;
    }

    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(size_px, line_h);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    // A constrained width (wrapping) or a free one; the height is always free.
    buffer.set_size(&mut font_system, max_width, None);
    let attrs = Attrs::new()
        .family(family_for(text))
        .weight(Weight(available_weight(weight)))
        .style(available_style(italic));
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut width = 0.0_f32;
    let mut lines = 0.0_f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        lines += 1.0;
    }

    // **Rounded up**, and this matters more than it looks. The layout engine rounds
    // the boxes it hands out to whole pixels, so a natural width of 146.4 becomes a
    // box of 146 — narrower than the text that asked for it. The text is then shaped
    // again at 146 when painting, wraps onto a second line, and overlaps whatever the
    // layout put below it on the strength of a one-line height. A ceiling here is what
    // keeps the measurement and the painting talking about the same box.
    //
    // Under a constraint the ceiling is clamped back to it: the text did fit that
    // width, and a box a fraction wider than allowed is a different bug.
    let width = match max_width {
        Some(max) => width.ceil().min(max),
        None => width.ceil(),
    };
    let measured = Size::new(width, lines.max(1.0) * line_h);
    remember_measurement(key, measured);
    measured
}

/// The cached answer for `key`, promoting it out of the retiring generation if that
/// is where it was found.
fn cached_measurement(key: &MeasureKey) -> Option<Size> {
    let mut guard = MEASURE_CACHE.lock().ok()?;
    let cache = guard.as_mut()?;
    if let Some(size) = cache.current.get(key) {
        return Some(*size);
    }
    let size = *cache.previous.get(key)?;
    // Still in use, so it moves forward rather than falling out with its generation.
    cache.current.insert(
        MeasureKey {
            text: key.text.clone(),
            ..*key
        },
        size,
    );
    Some(size)
}

fn remember_measurement(key: MeasureKey, size: Size) {
    let Ok(mut guard) = MEASURE_CACHE.lock() else {
        return;
    };
    let cache = guard.get_or_insert_with(MeasureCache::default);
    if cache.current.len() >= CACHE_CAP {
        cache.previous = std::mem::take(&mut cache.current);
    }
    cache.current.insert(key, size);
}

/// The **visual lines** a piece of text breaks into inside a box `max_width` wide, as
/// byte ranges into `text`.
///
/// The reference's `maxLines` and `overflow` both need this and neither can be answered by
/// a measurement: a height tells you how many lines there are, not *where they broke*, and
/// a cut has to fall on a break the shaper chose or the words move.
///
/// Ranges rather than strings, because the caller wants a **prefix** of the original: a
/// text handed to the renderer as separate lines is a paragraph per line, and rules that
/// span a paragraph — a justified block leaving its last line ragged — stop working.
///
/// Not cached. Only text that asked for a line limit gets here, and the shaping it does is
/// the same shaping the renderer does again; everything else goes through the cached
/// measurement.
pub fn line_spans(
    text: &str,
    size_px: f32,
    weight: FontWeight,
    italic: bool,
    max_width: Option<f32>,
    soft_wrap: bool,
) -> Vec<std::ops::Range<usize>> {
    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(size_px, line_height(size_px));
    let mut buffer = Buffer::new(&mut font_system, metrics);
    if !soft_wrap {
        buffer.set_wrap(&mut font_system, cosmic_text::Wrap::None);
    }
    buffer.set_size(&mut font_system, max_width, None);
    let attrs = Attrs::new()
        .family(family_for(text))
        .weight(Weight(available_weight(weight)))
        .style(available_style(italic));
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    // A buffer line per explicit newline; a layout run's glyph offsets are into its own
    // buffer line, so the starts of those lines are what turns them into offsets into the
    // whole.
    let mut line_start = Vec::new();
    let mut at = 0usize;
    for line in &buffer.lines {
        line_start.push(at);
        at += line.text().len() + 1;
    }

    let mut spans = Vec::new();
    for run in buffer.layout_runs() {
        let base = line_start.get(run.line_i).copied().unwrap_or(0);
        // The glyphs are in **visual** order, so a right-to-left line has its last byte
        // first: the span is the extent of the run, not its ends in order.
        let start = run.glyphs.iter().map(|g| g.start).min();
        let end = run.glyphs.iter().map(|g| g.end).max();
        match (start, end) {
            (Some(start), Some(end)) => spans.push(base + start..base + end),
            // A blank line between two paragraphs has no glyphs and is still a line.
            _ => spans.push(base..base),
        }
    }
    // An empty paragraph still occupies a line, and a caller counting them would
    // otherwise see none.
    if spans.is_empty() {
        spans.push(0..0);
    }
    spans
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
                .style(available_style(run.italic))
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
    // Rounded up for the same reason as [`measure_wrapped`]: a box rounded down below
    // the width the text asked for makes the text wrap when it is painted, on a height
    // that says it did not.
    let width = match max_width {
        Some(max) => width.ceil().min(max),
        None => width.ceil(),
    };
    Size::new(width, height)
}

/// The byte offset at which a rich text runs past `max_lines` visual lines, and how many
/// lines it actually took — `None` when it fits.
///
/// The offset is into the **concatenation** of the runs' texts, which is what the shaper
/// was given and the only coordinate the runs and the lines share. Splitting the runs
/// there is the caller's job, and cheap once the number is known.
///
/// Rich text is cut here rather than in the widget because the cut has to land on a break
/// the shaper chose, and only the shaper knows where those are — the same reason
/// [`line_spans`] exists, one primitive along.
pub fn runs_cut_at(
    runs: &[TextRun],
    max_width: Option<f32>,
    soft_wrap: bool,
    max_lines: usize,
) -> Option<usize> {
    if runs.iter().all(|r| r.text.is_empty()) {
        return None;
    }
    let base = runs.iter().map(|r| r.size).fold(0.0_f32, f32::max);
    let mut font_system = font_system().lock().expect("FontSystem lock");
    let metrics = Metrics::new(base, line_height(base));
    let mut buffer = Buffer::new(&mut font_system, metrics);
    if !soft_wrap {
        buffer.set_wrap(&mut font_system, cosmic_text::Wrap::None);
    }
    buffer.set_size(&mut font_system, max_width, None);
    let spans = runs.iter().map(|run| {
        (
            run.text.as_str(),
            Attrs::new()
                .family(family_for(&run.text))
                .weight(Weight(available_weight(run.weight)))
                .style(available_style(run.italic))
                .metrics(Metrics::new(run.size, line_height(run.size))),
        )
    });
    buffer.set_rich_text(&mut font_system, spans, Attrs::new(), Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    // A buffer line per explicit newline; the offsets a layout run carries are into its
    // own buffer line, so the starts of those lines are what turns them back into offsets
    // into the whole.
    let mut line_start = Vec::new();
    let mut at = 0usize;
    for line in &buffer.lines {
        line_start.push(at);
        at += line.text().len() + 1;
    }
    for (index, run) in buffer.layout_runs().enumerate() {
        if index < max_lines {
            continue;
        }
        let start = run.glyphs.iter().map(|g| g.start).min().unwrap_or(0);
        return Some(line_start.get(run.line_i).copied().unwrap_or(0) + start);
    }
    None
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
                .style(available_style(italic));
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

    /// The spans come back **as the shaper broke them**: each one fits the box, and the
    /// words they cover are the words that were written, in order. That is what makes it
    /// safe to keep a prefix of the text and drop the rest.
    #[test]
    fn line_spans_break_where_the_shaper_breaks() {
        let text = "one two three four five six seven eight";
        let spans = line_spans(text, 14.0, FontWeight::Regular, false, Some(80.0), true);
        assert!(spans.len() > 1, "it wrapped: {spans:?}");
        let words: Vec<&str> = spans
            .iter()
            .flat_map(|s| text[s.clone()].split_whitespace())
            .collect();
        assert_eq!(words, text.split_whitespace().collect::<Vec<_>>());
        for span in &spans {
            let w = measure_styled(&text[span.clone()], 14.0, FontWeight::Regular, false).width;
            assert!(
                w <= 80.5,
                "each line fits: {:?} at {w}",
                &text[span.clone()]
            );
        }
    }

    /// The spans are **in order and do not overlap**, which is what lets a caller cut at
    /// the start of the first line it is dropping.
    #[test]
    fn the_spans_run_forwards() {
        let text = "one two three four five six seven eight";
        let spans = line_spans(text, 14.0, FontWeight::Regular, false, Some(80.0), true);
        for pair in spans.windows(2) {
            assert!(pair[0].end <= pair[1].start, "{pair:?}");
        }
    }

    /// Without wrapping there is one line however narrow the box: nothing may push a
    /// word onto the next one, so the line runs past the edge instead.
    #[test]
    fn without_wrapping_there_is_one_line() {
        let text = "one two three four five six seven eight";
        let spans = line_spans(text, 14.0, FontWeight::Regular, false, Some(40.0), false);
        assert_eq!(spans.len(), 1);
        assert_eq!(&text[spans[0].clone()], text);
    }
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
    #[cfg(all(
        feature = "bundled-sans",
        feature = "bundled-italic",
        feature = "bundled-mono"
    ))]
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
    #[cfg(all(feature = "bundled-sans", feature = "bundled-arabic"))]
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
    #[cfg(all(feature = "bundled-sans", feature = "bundled-arabic"))]
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
            buffer.set_text(&mut fs, text, attrs, Shaping::Advanced);
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
    #[cfg(all(feature = "bundled-sans", feature = "bundled-arabic"))]
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

    /// The natural width is a **whole** number, and never less than what the text
    /// needs. The layout engine rounds the boxes it hands out to whole pixels, so a
    /// measurement of 146.4 becomes a box of 146 — and the text, shaped again at 146
    /// when it is painted, wraps onto a line the layout never reserved.
    #[test]
    fn the_natural_width_is_never_rounded_down_below_the_text() {
        for text in ["Write code", "Bonjour le monde", "A", "iiiii"] {
            for size in [12.0_f32, 15.0, 20.0, 24.0] {
                let natural = measure_styled(text, size, FontWeight::Bold, false);
                assert_eq!(
                    natural.width,
                    natural.width.ceil(),
                    "a whole number for {text:?} at {size}: {}",
                    natural.width
                );
                // Given exactly that width, the text still fits on one line.
                let at_its_width =
                    measure_wrapped(text, size, FontWeight::Bold, false, Some(natural.width));
                assert_eq!(
                    at_its_width.height, natural.height,
                    "{text:?} at {size} wrapped inside its own width"
                );
            }
        }
    }

    /// Under a constraint the ceiling is clamped back to it: a box wider than allowed
    /// is a different bug from the one above.
    #[test]
    fn a_constrained_measurement_never_exceeds_its_constraint() {
        let measured = measure_wrapped(
            "A rather long sentence that has to wrap somewhere",
            18.0,
            FontWeight::Regular,
            false,
            Some(120.0),
        );
        assert!(measured.width <= 120.0, "{}", measured.width);
    }

    /// Italic text measures whatever faces are bundled. This is the guard on the
    /// promise that dropping `bundled-italic` costs slanted text and not text: asking
    /// cosmic-text for a style it has no face for used to reach the platform fallback
    /// lists, which are empty on Android, and panic there.
    #[test]
    fn italic_measures_whether_or_not_an_oblique_face_is_bundled() {
        let measured = measure_styled("Slanted", 16.0, FontWeight::Regular, true);
        assert!(measured.width > 0.0, "nothing shaped");
    }

    /// A style is only ever *asked for* when the database can answer it — the whole
    /// point of [`available_style`], and true in either configuration.
    #[test]
    fn italic_is_only_asked_for_when_a_face_can_answer() {
        // Measuring is what builds the font system, which is what reads the database.
        let _ = measure("x", 12.0);
        assert_eq!(
            available_style(false),
            Style::Normal,
            "upright asked for italic"
        );
        assert_eq!(
            available_style(true) == Style::Italic,
            ITALIC.load(Ordering::Relaxed),
            "italic asked for without a face to answer it"
        );
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

    /// The cache must be invisible: measuring twice gives the same answer as measuring
    /// once, whichever entry point asked, and under a constraint as well as free.
    #[test]
    fn a_cached_measurement_is_the_measurement() {
        for (text, size) in [("Open", 15.0_f32), ("Task number 42 — due Friday", 13.0)] {
            let first = measure(text, size);
            let second = measure(text, size);
            assert_eq!(
                first, second,
                "{text:?} measured differently the second time"
            );

            let styled = measure_styled(text, size, FontWeight::Bold, false);
            assert_eq!(
                styled,
                measure_styled(text, size, FontWeight::Bold, false),
                "{text:?} bold measured differently the second time"
            );

            let wrapped = measure_wrapped(text, size, FontWeight::Regular, false, Some(60.0));
            assert_eq!(
                wrapped,
                measure_wrapped(text, size, FontWeight::Regular, false, Some(60.0)),
                "{text:?} wrapped measured differently the second time"
            );
            // The constraint is part of the key: a different width is a different
            // answer, not a hit on the free one.
            assert!(
                wrapped.height >= first.height,
                "wrapping to 60 px should not make {text:?} shorter"
            );
        }
    }

    /// A retired generation must not lose an entry that is still being asked for:
    /// enough distinct strings to rotate the cache several times over, with one string
    /// re-measured throughout, which has to keep the same answer.
    #[test]
    fn a_string_still_in_use_survives_the_rotation() {
        let kept = measure("still here", 14.0);
        for i in 0..(CACHE_CAP * 2 + 64) {
            measure(&format!("filler {i}"), 14.0);
            if i % 512 == 0 {
                assert_eq!(kept, measure("still here", 14.0), "lost at {i}");
            }
        }
        assert_eq!(kept, measure("still here", 14.0));
    }

    /// Registering a face changes what text can measure to, so every remembered
    /// answer has to go. The measurement itself must still work afterwards.
    #[test]
    fn registering_a_font_forgets_what_was_measured() {
        let before = measure("forget me", 15.0);
        forget_measurements();
        let after = measure("forget me", 15.0);
        assert_eq!(before, after, "the same faces must measure the same");
    }
    /// A baseline sits inside its line box, below the top and above the bottom — and
    /// well above the middle, because most of a face is above the baseline.
    #[test]
    fn a_baseline_sits_where_a_line_of_text_does() {
        let size = 20.0;
        let b = baseline(size, FontWeight::Regular, false);
        let line = line_height(size);
        assert!(b > 0.0 && b < line, "inside the line box: {b} of {line}");
        assert!(b > line * 0.5, "and below its middle: {b} of {line}");
    }

    /// It scales with the size, which is the whole reason two sizes cannot be aligned
    /// by their tops.
    #[test]
    fn a_bigger_face_has_a_lower_baseline() {
        let small = baseline(12.0, FontWeight::Regular, false);
        let large = baseline(24.0, FontWeight::Regular, false);
        assert!(large > small * 1.5, "{small} then {large}");
    }

    /// Runs sharing a line share the **largest** baseline: the tallest ascender is
    /// what decides where the line sits.
    #[test]
    fn mixed_runs_take_the_tallest_baseline() {
        let run = |text: &str, size: f32| TextRun {
            text: text.into(),
            size,
            weight: FontWeight::Regular,
            italic: false,
            color: frus_core::Color::WHITE,
            decoration: frus_core::TextDecoration::default(),
            decoration_color: None,
        };
        let runs = vec![run("small", 12.0), run("large", 24.0)];
        let mixed = baseline_of_runs(&runs).expect("two runs");
        assert!((mixed - baseline(24.0, FontWeight::Regular, false)).abs() < 0.01);
    }
}
