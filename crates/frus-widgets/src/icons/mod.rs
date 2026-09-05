//! The **bundled icon set**: the whole material icon vocabulary, reachable by name as
//! [`Icons`] constants — `Icons::ADD`, `Icons::STAR`, `Icons::ARROW_BACK` — and an
//! escape hatch, [`IconData::custom`], for a mark the set does not have.
//!
//! # What an icon is here
//!
//! An [`IconData`] is a **value**, not a widget: it says *which* icon, and nothing about
//! size or colour. [`crate::Icon`] is what draws one, scaling its path onto the box the
//! layout gave it and colouring it from the theme. The same `IconData` therefore serves
//! an app bar action, a list tile's leading slot and a button's glyph without any of
//! them agreeing on anything but the name.
//!
//! Every icon is a **filled** silhouette on a `24 × 24` grid, y downwards, drawn by the
//! non-zero rule — a ring is an outer contour plus an inner one wound the other way, not
//! a stroke.
//!
//! # The four styles
//!
//! `Icons::ADD` is the **filled** icon, and it is always there. The set also draws each
//! icon outlined, rounded and sharp, and those sit behind a cargo feature apiece —
//! `icons-outlined`, `icons-rounded`, `icons-sharp` — because four times the artwork is
//! 1.3 MB and most applications want one of them at most:
//!
//! ```toml
//! frus = { version = "0.1", features = ["icons-outlined"] }
//! ```
//!
//! With the feature on, `Icons::ADD_OUTLINED` exists and [`Icons::by_name`] answers
//! `"add_outlined"`. Without it neither does — `by_name` returns `None` rather than
//! falling back to a different drawing, because an application that asked for the
//! outlined set and silently got the filled one would ship looking wrong.
//!
//! The pairing is what the styles are for: an outlined icon at rest and its filled twin
//! when selected is how a navigation bar says which destination you are on.
//!
//! # Where the paths live
//!
//! The outlines are not Rust source. They sit in `assets/material-icons.bin` and one
//! file per variant style — a compact blob of about 300 KiB for the 2 233 filled icons,
//! roughly 140 bytes each — decoded to a [`Path`] on demand. Two reasons for a blob
//! rather than generated code: 2 233 constant path expressions is a megabyte of source
//! that slows every build down, and the coordinates are exact integers in font units, so
//! nothing is lost by storing them as integers.
//!
//! Decoding is cheap (a linear walk over a few hundred bytes) but it is not free. A
//! caller that paints the same icon thousands of times per frame should hold the [`Path`]
//! rather than call [`IconData::path`] in a loop.
//!
//! # Your own icons
//!
//! ```
//! use frus_widgets::{IconData, Icon};
//! use frus_core::{Path, Point, Rect};
//!
//! // A `24 × 24` path, y downwards, filled by the non-zero rule — the same contract the
//! // bundled ones honour.
//! fn lozenge() -> Path {
//!     Path::rect(Rect::new(4.0, 8.0, 16.0, 8.0))
//! }
//!
//! const LOZENGE: IconData = IconData::custom(lozenge);
//! let _ = Icon::new(LOZENGE).size(32.0);
//! ```
//!
//! The artwork is the material icon set; see `assets/README.md` for its licence.

use frus_core::{Path, Point, TextDirection};

mod names;

const MAGIC: &[u8; 8] = b"FRUSICO1";

/// One style's outline blob: a signature, a header, an offset table, and one byte-code
/// stream per icon. See the module docs for the format, and `scripts/gen_icons.py` for
/// the generator that writes it.
#[derive(Clone, Copy)]
struct Blob(&'static [u8]);

impl Blob {
    /// Reads a little-endian `u32`.
    const fn le_u32(self, at: usize) -> u32 {
        let b = self.0;
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    /// How many icons this style holds.
    const fn count(self) -> usize {
        self.le_u32(8) as usize
    }

    /// The font em the coordinates are expressed in.
    const fn upem(self) -> f32 {
        self.le_u32(12) as f32
    }

    /// The side of the square an icon is drawn on, before the widget scales it.
    const fn grid(self) -> f32 {
        self.le_u32(16) as f32
    }

    /// Where the offset table ends and the streams begin.
    const fn data(self) -> usize {
        8 + 12 + 4 * (self.count() + 1)
    }

    /// The offsets of icon `index`'s stream.
    const fn stream(self, index: usize) -> (usize, usize) {
        let table = 8 + 12 + 4 * index;
        (
            self.data() + self.le_u32(table) as usize,
            self.data() + self.le_u32(table + 4) as usize,
        )
    }

    /// Every blob is checked **at compile time**: its signature, the grid and em it was
    /// generated on, and that its offset table ends exactly where the file does. A build
    /// that picked up a stale or foreign file would otherwise decode it into nonsense,
    /// silently, in every icon on screen.
    const fn check(self) -> bool {
        assert!(self.0.len() >= 20, "the icon blob is truncated");
        let mut i = 0;
        while i < 8 {
            assert!(
                self.0[i] == MAGIC[i],
                "the icon blob is not a frus icon blob"
            );
            i += 1;
        }
        assert!(
            self.0.len() == self.data() + self.le_u32(20 + 4 * self.count()) as usize,
            "the icon blob's offset table and its data disagree"
        );
        assert!(
            self.upem() == FILLED.upem() && self.grid() == FILLED.grid(),
            "the styles were generated on different grids"
        );
        true
    }
}

const FILLED: Blob = Blob(include_bytes!("../../assets/material-icons.bin"));
#[cfg(feature = "icons-outlined")]
const OUTLINED: Blob = Blob(include_bytes!("../../assets/material-icons-outlined.bin"));
#[cfg(feature = "icons-rounded")]
const ROUNDED: Blob = Blob(include_bytes!("../../assets/material-icons-rounded.bin"));
#[cfg(feature = "icons-sharp")]
const SHARP: Blob = Blob(include_bytes!("../../assets/material-icons-sharp.bin"));

const _: () = {
    assert!(FILLED.check());
    #[cfg(feature = "icons-outlined")]
    assert!(OUTLINED.check());
    #[cfg(feature = "icons-rounded")]
    assert!(ROUNDED.check());
    #[cfg(feature = "icons-sharp")]
    assert!(SHARP.check());
};

/// The side of the square an icon is drawn on, before the widget scales it. Read from
/// the blob rather than written here, so the grid the paths were generated on and the
/// grid the widgets scale from cannot drift apart.
pub(crate) const GRID: f32 = FILLED.grid();

/// **How an icon is drawn.** The filled set is always bundled; the other three sit
/// behind a cargo feature apiece, and asking for one that is not compiled in yields
/// nothing rather than a different drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconStyle {
    /// The solid silhouette — the default, and the only one always bundled.
    Filled,
    /// The line drawing. Needs the `icons-outlined` feature.
    Outlined,
    /// Softened corners. Needs the `icons-rounded` feature.
    Rounded,
    /// Squared-off corners. Needs the `icons-sharp` feature.
    Sharp,
}

impl IconStyle {
    /// This style's blob, or `None` when the feature that carries it is off. Nothing can
    /// reach that case through the generated names — a style's constants are compiled
    /// out with its blob — but [`Icons::by_name`] can be handed the name of one.
    fn blob(self) -> Option<Blob> {
        match self {
            IconStyle::Filled => Some(FILLED),
            #[cfg(feature = "icons-outlined")]
            IconStyle::Outlined => Some(OUTLINED),
            #[cfg(feature = "icons-rounded")]
            IconStyle::Rounded => Some(ROUNDED),
            #[cfg(feature = "icons-sharp")]
            IconStyle::Sharp => Some(SHARP),
            #[cfg(not(all(
                feature = "icons-outlined",
                feature = "icons-rounded",
                feature = "icons-sharp"
            )))]
            _ => None,
        }
    }

    /// The suffix a caller writes after an icon's name to ask for this style — what
    /// [`Icons::by_name`] recognises, and what the constants are named with:
    /// `Icons::ADD_OUTLINED` is `"add"` plus `IconStyle::Outlined.suffix()`.
    pub const fn suffix(self) -> &'static str {
        match self {
            IconStyle::Filled => "",
            IconStyle::Outlined => "_outlined",
            IconStyle::Rounded => "_rounded",
            IconStyle::Sharp => "_sharp",
        }
    }
}

// The opcodes. A `_D` op carries signed byte deltas from the current point; an `_A` op
// carries absolute `i16` coordinates, for the rare jump a byte cannot express.
const OP_CLOSE: u8 = 0;
const OP_MOVE_D: u8 = 1;
const OP_MOVE_A: u8 = 2;
const OP_LINE_D: u8 = 3;
const OP_LINE_A: u8 = 4;
const OP_CUBIC_D: u8 = 5;
const OP_CUBIC_A: u8 = 6;

/// **Which icon** — one of the bundled set, or a path of the caller's own.
///
/// It is `Copy` and cheap to pass around; the path behind it is built only when
/// [`IconData::path`] or [`IconData::placed`] is called.
///
/// An icon also says whether it **carries a direction**. An arrow, an indent, a reply
/// and a chevron all point somewhere, and where they point is relative to the reading
/// order: in a right-to-left order, *back* is to the right. A tick, a star and a
/// magnifying glass point nowhere and must not be touched. Which is which is not a
/// judgement a widget can make, so it is a property of the icon —
/// [`IconData::matches_text_direction`] — and 76 of the bundled 2 233 carry it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IconData {
    source: Source,
    /// Whether the icon is mirrored in a right-to-left reading order.
    directional: bool,
}

/// Where an icon's outline comes from. Private: a caller reaches the bundled ones
/// through [`Icons`] and supplies its own through [`IconData::custom`], so there is no
/// third case to name.
#[derive(Clone, Copy, Debug)]
enum Source {
    /// A style, and an index into that style's blob.
    Bundled(IconStyle, u16),
    /// A function that draws the outline on the `24 × 24` grid.
    Custom(fn() -> Path),
}

/// Equality is hand-written because one of the two cases is a function pointer, and the
/// derived comparison of those is not meaningful: the compiler may give two identical
/// functions the same address, or the same function two addresses across codegen units.
/// [`std::ptr::fn_addr_eq`] is the sanctioned comparison — it can answer `true` for two
/// functions that were merged, and never `false` for one function compared with itself,
/// which is the direction that matters for keying a cache on an icon.
impl PartialEq for Source {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Source::Bundled(sa, a), Source::Bundled(sb, b)) => sa == sb && a == b,
            (Source::Custom(a), Source::Custom(b)) => std::ptr::fn_addr_eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for Source {}

impl std::hash::Hash for Source {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Source::Bundled(style, index) => (0u8, *style, *index as usize).hash(state),
            // Equal pointers hash alike, which is what `Hash` and `Eq` have to agree on.
            Source::Custom(draw) => (1u8, *draw as usize).hash(state),
        }
    }
}

impl IconData {
    /// The `n`th icon of one style's blob. Only the generated names call this, which is
    /// why it is not public: an index is meaningless outside the file that wrote it.
    pub(crate) const fn bundled(style: IconStyle, index: u16) -> Self {
        Self {
            source: Source::Bundled(style, index),
            directional: false,
        }
    }

    /// An icon of the caller's own, drawn by `draw` on the same `24 × 24` grid, y
    /// downwards, and filled by the non-zero rule.
    ///
    /// A function pointer and not a `Path`, so that an icon stays a `const` — a caller's
    /// icon is declared exactly where a bundled one would be, and costs nothing until it
    /// is painted.
    pub const fn custom(draw: fn() -> Path) -> Self {
        Self {
            source: Source::Custom(draw),
            directional: false,
        }
    }

    /// The same icon, declared to **carry a direction**: [`IconData::placed`] mirrors it
    /// when the reading order is right-to-left.
    ///
    /// `const`, so a caller's directional icon is still a constant:
    ///
    /// ```
    /// # use frus_widgets::IconData;
    /// # use frus_core::{Path, Rect};
    /// # fn arrow() -> Path { Path::rect(Rect::new(4.0, 10.0, 16.0, 4.0)) }
    /// const BACK: IconData = IconData::custom(arrow).mirrored();
    /// assert!(BACK.matches_text_direction());
    /// ```
    pub const fn mirrored(self) -> Self {
        Self {
            source: self.source,
            directional: true,
        }
    }

    /// Whether this icon is turned round for a right-to-left reading order.
    pub const fn matches_text_direction(self) -> bool {
        self.directional
    }

    /// The icon's outline on the `24 × 24` grid, ready to be scaled and filled.
    ///
    /// The raw artwork, in the direction it was drawn. [`IconData::placed`] is what a
    /// widget wants: it is this, sized, positioned, and turned round where it should be.
    pub fn path(self) -> Path {
        match self.source {
            Source::Bundled(style, index) => decode(style, index as usize),
            Source::Custom(draw) => draw(),
        }
    }

    /// How this icon is drawn, or `None` for a caller's own — which is drawn however its
    /// function draws it.
    pub fn style(self) -> Option<IconStyle> {
        match self.source {
            Source::Bundled(style, _) => Some(style),
            Source::Custom(_) => None,
        }
    }

    /// The icon's outline **scaled to `side` and placed at `(x, y)`** — the top-left of
    /// the square it occupies — and mirrored within that square when it carries a
    /// direction and `direction` is right-to-left.
    ///
    /// Every widget that paints an icon goes through here. That is the point: the rule
    /// about which icons turn round is stated once, in the icon, and applied once, here,
    /// rather than remembered at each of the sixteen places that fill an icon's path.
    pub fn placed(self, side: f32, x: f32, y: f32, direction: TextDirection) -> Path {
        let path = self.path().scaled(side / GRID).translated(x, y);
        if self.directional && direction == TextDirection::Rtl {
            // Reflected in the vertical centre line of the square it was just placed in,
            // so a mirrored icon occupies exactly the box an unmirrored one would.
            path.mirrored_x(x + side * 0.5)
        } else {
            path
        }
    }

    /// Whether this icon comes from the bundled set rather than from a caller.
    pub fn is_bundled(self) -> bool {
        matches!(self.source, Source::Bundled(..))
    }
}

/// The **bundled icon set**, as one constant per icon: `Icons::ADD`, `Icons::STAR`,
/// `Icons::ARROW_BACK`, and 2 230 more — four times over, where the variant styles
/// are compiled in.
///
/// The names are the icon set's own, in Rust's constant case — `arrow_back` is
/// `Icons::ARROW_BACK`. Names that begin with a digit are spelled out, because an
/// identifier cannot: `Icons::TEN_K`, `Icons::FOUR_K_PLUS`.
///
/// ```
/// use frus_widgets::Icons;
///
/// let icon = Icons::FAVORITE;
/// assert!(!icon.path().is_empty());
/// assert_eq!(Icons::by_name("favorite"), Some(icon));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Icons;

impl Icons {
    /// How many **filled** icons the bundled set holds — the styles behind a feature
    /// have a few less each, since the set does not draw quite every icon four times.
    pub const COUNT: usize = FILLED.count();

    /// The icon called `name`, or `None` if the set has no such icon **compiled in**.
    ///
    /// The name is the set's own spelling — lower case with underscores, as in
    /// `"arrow_back"` — which is what a configuration file or a design tool exports. A
    /// style is asked for by its suffix, `"arrow_back_outlined"`, and answers `None`
    /// unless that style's feature is on: an application that asked for the outlined set
    /// and silently got the filled one would ship looking wrong, and a `None` it can
    /// report is better than a drawing nobody chose.
    pub fn by_name(name: &str) -> Option<IconData> {
        fn find(table: &'static [(&'static str, IconData)], name: &str) -> Option<IconData> {
            table
                .binary_search_by(|(known, _)| (*known).cmp(name))
                .ok()
                .map(|i| table[i].1)
        }
        // The suffixes cannot collide with a filled name: `insert_chart_outlined` is a
        // real icon, and it is looked for in the filled table first, where it is.
        if let Some(icon) = find(&names::FILLED, name) {
            return Some(icon);
        }
        #[cfg(feature = "icons-outlined")]
        if let Some(icon) = find(&names::OUTLINED, name) {
            return Some(icon);
        }
        #[cfg(feature = "icons-rounded")]
        if let Some(icon) = find(&names::ROUNDED, name) {
            return Some(icon);
        }
        #[cfg(feature = "icons-sharp")]
        if let Some(icon) = find(&names::SHARP, name) {
            return Some(icon);
        }
        None
    }

    /// Every bundled icon as `(name, icon)` — for a picker, a gallery, or anything else
    /// that has to show the whole set. Sorted within each style, filled first, and the
    /// styles that are not compiled in are simply not there.
    pub fn all() -> impl Iterator<Item = (&'static str, IconData)> {
        let styles: [&'static [(&'static str, IconData)]; 4] = [
            &names::FILLED,
            #[cfg(feature = "icons-outlined")]
            &names::OUTLINED,
            #[cfg(not(feature = "icons-outlined"))]
            &[],
            #[cfg(feature = "icons-rounded")]
            &names::ROUNDED,
            #[cfg(not(feature = "icons-rounded"))]
            &[],
            #[cfg(feature = "icons-sharp")]
            &names::SHARP,
            #[cfg(not(feature = "icons-sharp"))]
            &[],
        ];
        styles.into_iter().flatten().copied()
    }

    /// The icons of one style, or an empty run when that style is not compiled in.
    pub fn of_style(style: IconStyle) -> impl Iterator<Item = (&'static str, IconData)> {
        Self::all().filter(move |(_, icon)| icon.style() == Some(style))
    }
}

/// Walks one icon's byte stream into a [`Path`] on the `24 × 24` grid.
///
/// The font's y axis points up and the grid's points down, so every y is mirrored on the
/// way out. Doing it here, once, is what lets the rest of the framework treat an icon
/// path like any other path it drew itself.
fn decode(style: IconStyle, index: usize) -> Path {
    // A style whose feature is off, or an index past the end: an empty path, not a
    // panic and not somebody else's bytes read as coordinates.
    let Some(blob) = style.blob() else {
        return Path::new();
    };
    if index >= blob.count() {
        return Path::new();
    }
    let (mut p, end) = blob.stream(index);
    let bytes = blob.0;
    let scale = blob.grid() / blob.upem();
    let grid = blob.grid();
    let (mut cx, mut cy) = (0i32, 0i32);
    let mut path = Path::new();
    // Font units to grid, y mirrored.
    let point = |x: i32, y: i32| Point::new(x as f32 * scale, grid - y as f32 * scale);
    let i8_at = |at: usize| bytes[at] as i8 as i32;
    let i16_at = |at: usize| i16::from_le_bytes([bytes[at], bytes[at + 1]]) as i32;

    while p < end {
        let op = bytes[p];
        p += 1;
        match op {
            OP_CLOSE => path = path.close(),
            OP_MOVE_D | OP_LINE_D => {
                cx += i8_at(p);
                cy += i8_at(p + 1);
                p += 2;
                let to = point(cx, cy);
                path = if op == OP_MOVE_D {
                    path.move_to(to)
                } else {
                    path.line_to(to)
                };
            }
            OP_MOVE_A | OP_LINE_A => {
                cx = i16_at(p);
                cy = i16_at(p + 2);
                p += 4;
                let to = point(cx, cy);
                path = if op == OP_MOVE_A {
                    path.move_to(to)
                } else {
                    path.line_to(to)
                };
            }
            OP_CUBIC_D => {
                let d = [
                    i8_at(p),
                    i8_at(p + 1),
                    i8_at(p + 2),
                    i8_at(p + 3),
                    i8_at(p + 4),
                    i8_at(p + 5),
                ];
                p += 6;
                let (x1, y1) = (cx + d[0], cy + d[1]);
                let (x2, y2) = (x1 + d[2], y1 + d[3]);
                cx = x2 + d[4];
                cy = y2 + d[5];
                path = path.cubic_to(point(x1, y1), point(x2, y2), point(cx, cy));
            }
            OP_CUBIC_A => {
                let (x1, y1) = (i16_at(p), i16_at(p + 2));
                let (x2, y2) = (i16_at(p + 4), i16_at(p + 6));
                cx = i16_at(p + 8);
                cy = i16_at(p + 10);
                p += 12;
                path = path.cubic_to(point(x1, y1), point(x2, y2), point(cx, cy));
            }
            // Unreachable for a blob this build shipped with; a stream that goes wrong
            // stops here rather than reading the next icon's bytes as coordinates.
            _ => break,
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::PathVerb;

    /// The blobs a build was compiled with, and the styles it names, are two files that
    /// have to agree. The signature, the grid and the offset table are checked at compile
    /// time (see the `const` block above); what a compiler cannot see is the generated
    /// names agreeing with the blobs they were written from.
    #[test]
    fn every_compiled_style_agrees_with_its_blob() {
        assert_eq!(FILLED.upem(), 512.0);
        assert_eq!(GRID, 24.0);
        for (style, table) in compiled_styles() {
            let blob = style.blob().expect("a compiled style has a blob");
            assert_eq!(
                blob.count(),
                table.len(),
                "{style:?}: blob and names disagree on the count"
            );
            // The last offset lands exactly on the end of the file, or the table and the
            // data have drifted apart.
            let (_, end) = blob.stream(blob.count() - 1);
            assert_eq!(end, blob.0.len(), "{style:?}");
        }
        assert_eq!(Icons::COUNT, names::FILLED.len());
    }

    /// A style's names all carry its suffix, and only its own: an entry filed under the
    /// wrong table would be reachable by a name that draws something else.
    #[test]
    fn each_table_carries_its_own_suffix() {
        for (style, table) in compiled_styles() {
            for (name, icon) in table {
                assert_eq!(icon.style(), Some(style), "{name} is filed under {style:?}");
                if !style.suffix().is_empty() {
                    assert!(
                        name.ends_with(style.suffix()),
                        "{name} should end with {}",
                        style.suffix()
                    );
                }
            }
        }
    }

    /// Every style compiled into this build, with its name table. A style whose feature
    /// is off has no blob, and drops out here.
    fn compiled_styles() -> Vec<(IconStyle, &'static [(&'static str, IconData)])> {
        let styles: [(IconStyle, &'static [(&'static str, IconData)]); 4] = [
            (IconStyle::Filled, &names::FILLED),
            #[cfg(feature = "icons-outlined")]
            (IconStyle::Outlined, &names::OUTLINED),
            #[cfg(not(feature = "icons-outlined"))]
            (IconStyle::Outlined, &[]),
            #[cfg(feature = "icons-rounded")]
            (IconStyle::Rounded, &names::ROUNDED),
            #[cfg(not(feature = "icons-rounded"))]
            (IconStyle::Rounded, &[]),
            #[cfg(feature = "icons-sharp")]
            (IconStyle::Sharp, &names::SHARP),
            #[cfg(not(feature = "icons-sharp"))]
            (IconStyle::Sharp, &[]),
        ];
        styles
            .into_iter()
            .filter(|(style, _)| style.blob().is_some())
            .collect()
    }

    /// Every single icon, walked. This is the test that a corrupt or truncated blob
    /// cannot survive, and it is why the blob is checked in with its generator.
    #[test]
    fn every_icon_decodes_to_a_closed_filled_outline() {
        for (name, icon) in Icons::all() {
            let path = icon.path();
            assert!(!path.is_empty(), "{name} decoded to nothing");
            assert!(
                matches!(path.verbs().first(), Some(PathVerb::MoveTo(_))),
                "{name} should start with a MoveTo"
            );
            assert!(
                matches!(path.verbs().last(), Some(PathVerb::Close)),
                "{name} should end closed — an unclosed contour fills wrong"
            );
        }
    }

    /// Every icon stays on its grid. An icon that spilled would not be clipped, it would
    /// be *drawn over its neighbours*, which is the kind of thing that only shows up in a
    /// screenshot months later.
    ///
    /// The two kinds of point are held to different bounds on purpose. **On-curve points
    /// are within the box to a single font unit** — the whole filled, outlined and
    /// rounded sets sit exactly inside it, and a handful of sharp ones overhang by one
    /// unit of the 512 the artwork was drawn on, which is the artwork's own doing and a
    /// thirtieth of a pixel at the size an icon is used. Anything more is a mirrored axis
    /// or a lost scale. A **control** point may sit further out: that is how a curve is
    /// given its bend, and it says nothing about where the ink lands. It is still
    /// bounded, because a control point a long way out is a decoding error, not a design.
    #[test]
    fn every_icon_stays_within_its_grid() {
        let unit = GRID / FILLED.upem();
        let control_slack = 2.0;
        for (name, icon) in Icons::all() {
            for verb in icon.path().verbs() {
                let (controls, ends): (Vec<Point>, Vec<Point>) = match verb {
                    PathVerb::MoveTo(p) | PathVerb::LineTo(p) => (vec![], vec![*p]),
                    PathVerb::QuadTo { ctrl, to } => (vec![*ctrl], vec![*to]),
                    PathVerb::CubicTo { c1, c2, to } => (vec![*c1, *c2], vec![*to]),
                    PathVerb::Close => (vec![], vec![]),
                };
                for p in ends {
                    assert!(
                        p.x >= -unit && p.x <= GRID + unit && p.y >= -unit && p.y <= GRID + unit,
                        "{name} has an on-curve point off the grid at {p:?}"
                    );
                }
                for p in controls {
                    assert!(
                        p.x >= -control_slack && p.x <= GRID + control_slack,
                        "{name} has a control point far off the grid at {p:?}"
                    );
                    assert!(
                        p.y >= -control_slack && p.y <= GRID + control_slack,
                        "{name} has a control point far off the grid at {p:?}"
                    );
                }
            }
        }
    }

    /// `by_name` is a binary search, so each table has to be sorted — and a duplicate
    /// name would make one of the two unreachable.
    #[test]
    fn each_name_table_is_sorted_and_unique() {
        for (style, table) in compiled_styles() {
            let names: Vec<&str> = table.iter().map(|(n, _)| *n).collect();
            assert!(
                names.windows(2).all(|w| w[0] < w[1]),
                "{style:?}: the name table must ascend with no duplicates"
            );
        }
        assert_eq!(names::FILLED.len(), Icons::COUNT);
    }

    /// The two names that read like a style suffix but are not one. `insert_chart_outlined`
    /// is a filled icon in its own right, and its outlined variant is
    /// `insert_chart_outlined_outlined` — which is exactly the pair a suffix-stripping
    /// lookup gets wrong, so the lookup does not strip suffixes.
    #[test]
    fn a_name_that_ends_like_a_suffix_is_still_its_own_icon() {
        let chart = Icons::by_name("insert_chart_outlined").expect("a filled icon");
        assert_eq!(chart, Icons::INSERT_CHART_OUTLINED);
        assert_eq!(chart.style(), Some(IconStyle::Filled));
        let wifi = Icons::by_name("wifi_tethering_error_rounded").expect("a filled icon");
        assert_eq!(wifi.style(), Some(IconStyle::Filled));
        // And the keyword-shaped one keeps the set's spelling, not a language's escape.
        assert_eq!(Icons::by_name("class"), Some(Icons::CLASS));
    }

    #[test]
    fn a_name_resolves_to_the_same_icon_as_its_constant() {
        assert_eq!(Icons::by_name("add"), Some(Icons::ADD));
        assert_eq!(Icons::by_name("arrow_back"), Some(Icons::ARROW_BACK));
        assert_eq!(
            Icons::by_name("visibility_off"),
            Some(Icons::VISIBILITY_OFF)
        );
        assert_eq!(Icons::by_name("no_such_icon"), None);
        // The constants are the set's own names, not ours: the digit-leading ones are
        // spelled out because an identifier cannot start with a digit.
        assert_eq!(Icons::by_name("ten_k"), Some(Icons::TEN_K));
    }

    /// A spot-check with a shape simple enough to state exactly: `add` is a plus, four
    /// units thick, centred, spanning 5 to 19 on the grid. If the decoder mirrored the
    /// wrong axis or lost the scale, this is where it shows.
    #[test]
    fn add_is_a_centred_plus() {
        let path = Icons::ADD.path();
        let points: Vec<Point> = path
            .verbs()
            .iter()
            .filter_map(|v| match v {
                PathVerb::MoveTo(p) | PathVerb::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        let (min_x, max_x) = bounds(points.iter().map(|p| p.x));
        let (min_y, max_y) = bounds(points.iter().map(|p| p.y));
        assert!((min_x - 5.0).abs() < 0.1, "left edge at {min_x}");
        assert!((max_x - 19.0).abs() < 0.1, "right edge at {max_x}");
        assert!((min_y - 5.0).abs() < 0.1, "top edge at {min_y}");
        assert!((max_y - 19.0).abs() < 0.1, "bottom edge at {max_y}");
        // Twelve corners, and the closing repeat of the first: a plus and nothing else.
        assert_eq!(points.len(), 13);
    }

    fn bounds(values: impl Iterator<Item = f32>) -> (f32, f32) {
        values.fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
    }

    /// An icon of the caller's own goes through the same door as a bundled one.
    #[test]
    fn a_custom_icon_draws_its_own_path() {
        fn bar() -> Path {
            Path::rect(frus_core::Rect::new(4.0, 10.0, 16.0, 4.0))
        }
        const BAR: IconData = IconData::custom(bar);
        assert!(!BAR.is_bundled());
        assert!(Icons::ADD.is_bundled());
        assert_eq!(BAR.path().verbs().len(), bar().verbs().len());
        // Two icons drawn by the same function are the same icon; by a different one,
        // not — which is what lets a caller key a cache on an `IconData`.
        assert_eq!(BAR, IconData::custom(bar));
        assert_ne!(BAR, Icons::ADD);
    }

    /// An index past the end, or a style this build does not carry, yields an empty path
    /// rather than reading whatever bytes follow the table.
    #[test]
    fn an_index_past_the_end_is_empty() {
        assert!(IconData::bundled(IconStyle::Filled, u16::MAX)
            .path()
            .is_empty());
        for style in [IconStyle::Outlined, IconStyle::Rounded, IconStyle::Sharp] {
            if style.blob().is_none() {
                assert!(IconData::bundled(style, 0).path().is_empty());
                assert_eq!(Icons::by_name(&format!("add{}", style.suffix())), None);
            }
        }
    }

    /// The filled set is always there, whatever features are on — the framework's own
    /// widgets reach for it, so it cannot be optional.
    #[test]
    fn the_filled_style_is_never_optional() {
        assert!(IconStyle::Filled.blob().is_some());
        assert!(!Icons::ADD.path().is_empty());
        assert_eq!(Icons::ADD.style(), Some(IconStyle::Filled));
    }

    /// Which icons carry a direction is the set's own answer, not ours. The count is
    /// asserted because a generator that lost the flag would otherwise leave every arrow
    /// pointing the wrong way in Arabic, and nothing would fail.
    #[test]
    fn the_directional_icons_are_the_ones_that_point_somewhere() {
        for icon in [
            Icons::ARROW_BACK,
            Icons::ARROW_FORWARD,
            Icons::CHEVRON_LEFT,
            Icons::CHEVRON_RIGHT,
            Icons::REPLY,
            Icons::UNDO,
            Icons::FORMAT_INDENT_INCREASE,
            Icons::TRENDING_UP,
        ] {
            assert!(icon.matches_text_direction(), "{icon:?} points somewhere");
        }
        // A tick, a star and a magnifying glass point nowhere: mirroring them is a bug.
        for icon in [Icons::CHECK, Icons::STAR, Icons::SEARCH, Icons::FAVORITE] {
            assert!(!icon.matches_text_direction(), "{icon:?} points nowhere");
        }
        // 76 of the filled set. A variant style carries the same flags on the same
        // names, so its count only differs where the set has no icon in that style —
        // which is what this asserts rather than four hand-written numbers.
        let filled: Vec<&str> = names::FILLED
            .iter()
            .filter(|(_, icon)| icon.matches_text_direction())
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(filled.len(), 76);
        for (style, table) in compiled_styles() {
            if style == IconStyle::Filled {
                continue;
            }
            for base in &filled {
                let spelled = format!("{base}{}", style.suffix());
                if let Some(icon) = Icons::by_name(&spelled) {
                    assert!(
                        icon.matches_text_direction(),
                        "{spelled} should carry the direction its filled twin does"
                    );
                }
            }
            let turning = table
                .iter()
                .filter(|(_, icon)| icon.matches_text_direction())
                .count();
            assert!(
                turning <= filled.len() && turning + 2 >= filled.len(),
                "{style:?} turns {turning} icons round, the filled set {}",
                filled.len()
            );
        }
    }

    /// In a left-to-right reading order `placed` is exactly the scale-and-translate every
    /// call site used to write by hand — which is what makes the refactor a no-op there.
    #[test]
    fn placed_is_the_old_scale_and_translate_in_ltr() {
        for icon in [Icons::CHECK, Icons::ARROW_BACK] {
            let placed = icon.placed(16.0, 3.0, 7.0, TextDirection::Ltr);
            let by_hand = icon.path().scaled(16.0 / GRID).translated(3.0, 7.0);
            assert_eq!(placed.verbs(), by_hand.verbs(), "{icon:?}");
        }
    }

    /// Right-to-left turns a directional icon round **inside its own box**: every point
    /// is reflected, and the box it occupies does not move. An icon that shifted while
    /// mirroring would break every layout that reserved a square for it.
    #[test]
    fn rtl_mirrors_a_directional_icon_within_its_box() {
        let (side, x, y) = (16.0, 3.0, 7.0);
        let ltr = Icons::ARROW_BACK.placed(side, x, y, TextDirection::Ltr);
        let rtl = Icons::ARROW_BACK.placed(side, x, y, TextDirection::Rtl);
        assert_ne!(ltr.verbs(), rtl.verbs(), "a back arrow should turn round");

        let axis = x + side * 0.5;
        for (before, after) in points(&ltr).into_iter().zip(points(&rtl)) {
            assert!((after.x - (2.0 * axis - before.x)).abs() < 1e-4);
            assert!((after.y - before.y).abs() < 1e-4, "only x is reflected");
        }
        // The same square, before and after.
        let (lo_ltr, hi_ltr) = bounds(points(&ltr).into_iter().map(|p| p.x));
        let (lo_rtl, hi_rtl) = bounds(points(&rtl).into_iter().map(|p| p.x));
        assert!((lo_ltr - (2.0 * axis - hi_rtl)).abs() < 1e-4);
        assert!((hi_ltr - (2.0 * axis - lo_rtl)).abs() < 1e-4);
    }

    #[test]
    fn rtl_leaves_an_icon_that_points_nowhere_alone() {
        let ltr = Icons::CHECK.placed(16.0, 3.0, 7.0, TextDirection::Ltr);
        let rtl = Icons::CHECK.placed(16.0, 3.0, 7.0, TextDirection::Rtl);
        assert_eq!(ltr.verbs(), rtl.verbs());
    }

    /// A caller's own icon can say it points somewhere, and is then treated exactly as a
    /// bundled one — the flag is on the icon, not in a table only the generator writes.
    #[test]
    fn a_custom_icon_can_carry_a_direction() {
        fn wedge() -> Path {
            Path::new()
                .move_to(Point::new(4.0, 12.0))
                .line_to(Point::new(20.0, 4.0))
                .line_to(Point::new(20.0, 20.0))
                .close()
        }
        const BACK: IconData = IconData::custom(wedge).mirrored();
        const MARK: IconData = IconData::custom(wedge);
        assert!(BACK.matches_text_direction());
        assert!(!MARK.matches_text_direction());
        let rtl = BACK.placed(24.0, 0.0, 0.0, TextDirection::Rtl);
        // The tip was at x = 4; reflected about x = 12 it lands at x = 20.
        assert!(
            matches!(rtl.verbs().first(), Some(PathVerb::MoveTo(p)) if (p.x - 20.0).abs() < 1e-4)
        );
        assert_eq!(
            MARK.placed(24.0, 0.0, 0.0, TextDirection::Rtl).verbs(),
            MARK.placed(24.0, 0.0, 0.0, TextDirection::Ltr).verbs()
        );
    }

    fn points(path: &Path) -> Vec<Point> {
        path.verbs()
            .iter()
            .flat_map(|v| match v {
                PathVerb::MoveTo(p) | PathVerb::LineTo(p) => vec![*p],
                PathVerb::QuadTo { ctrl, to } => vec![*ctrl, *to],
                PathVerb::CubicTo { c1, c2, to } => vec![*c1, *c2, *to],
                PathVerb::Close => vec![],
            })
            .collect()
    }
}
