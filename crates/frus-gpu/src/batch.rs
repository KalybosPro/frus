//! Deciding **what may be drawn together**, and in what order.
//!
//! The renderer has one pipeline per kind of primitive — rectangles, images, paths —
//! and for a long time it drew one pass of each, in a fixed order. That is the
//! cheapest thing to do and it is wrong: a scene that puts a rectangle *over* a path
//! has the path drawn last, and the rectangle disappears. Milestone 291 found it the
//! only way anyone was ever going to: a filled button on a notched bottom bar, on a
//! phone.
//!
//! Drawing one call per primitive instead would be correct and would cost two draw
//! calls per widget. So this module does what a 2D renderer normally does: it batches
//! greedily, and only breaks a batch when something would actually be covered.
//!
//! A primitive may join an earlier batch of its own kind provided **nothing between
//! them overlaps it**. Painting a button's background with the previous button's
//! background is free — they are nowhere near each other. Painting a button's
//! background with a bar that the button sits on is not, and that is exactly the case
//! that breaks the batch.
//!
//! ## Text
//!
//! Text is ordered here too, since milestone 295: a `Primitive::Text` records the box
//! it was laid out in — the emitting widget's — alongside where it starts. That box is
//! an over-estimate of what the glyphs cover, which is the safe direction.
//!
//! A scene built by hand rather than by the widget walk leaves that box `UNBOUNDED`,
//! which overlaps everything: such text is ordered strictly by where it sits in the
//! scene, and batches with nothing. Conservative, and the only safe reading of "no
//! idea what this covers".

use frus_core::{Path, PathVerb, Primitive, Rect, Scene};

/// The pipeline a primitive is drawn by. Two primitives can share a draw call only if
/// they share a kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Rect,
    Image,
    Path,
    Text,
    /// A composited group. It is not drawn by a content pipeline at all — the
    /// compositor draws it from the texture it was rendered into — but it has to be
    /// **ordered** with everything else, which is what it is doing here. A layer batch
    /// always holds exactly one member: two groups never share a draw call.
    Layer,
}

/// A run of primitives drawn by one pipeline, in one call. `members` are indices into
/// the scene, in scene order; `bounds` is their union, which is what later primitives
/// test against.
#[derive(Clone, Debug)]
pub(crate) struct Batch {
    pub(crate) kind: Kind,
    pub(crate) members: Vec<usize>,
    bounds: Rect,
}

/// `true` when two rectangles share any area. Touching edges do not count: a box that
/// ends where the next begins covers nothing of it.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// The smallest rectangle containing every node of `path`. Control points are included
/// rather than solved for: a Bézier never leaves its control hull, so this is an
/// over-estimate and never an under-estimate, which is the direction that keeps the
/// planner correct.
fn path_bounds(path: &Path) -> Rect {
    let (mut x0, mut y0) = (f32::MAX, f32::MAX);
    let (mut x1, mut y1) = (f32::MIN, f32::MIN);
    let mut seen = false;
    let mut point = |p: &frus_core::Point| {
        seen = true;
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    };
    for verb in path.verbs() {
        match verb {
            PathVerb::MoveTo(p) | PathVerb::LineTo(p) => point(p),
            PathVerb::QuadTo { ctrl, to } => {
                point(ctrl);
                point(to);
            }
            PathVerb::CubicTo { c1, c2, to } => {
                point(c1);
                point(c2);
                point(to);
            }
            PathVerb::Close => {}
        }
    }
    if !seen {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    }
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// What a primitive covers, and which pipeline draws it.
fn footprint(primitive: &Primitive) -> Option<(Kind, Rect)> {
    match primitive {
        // Exactly its rectangle. A shadow's `blur` softens the edge *inside* the quad
        // — the shader ramps the alpha across it and nothing is rasterised beyond —
        // and a widget casting one passes an already-widened rect. Growing by `blur`
        // here would double-count it, and the cost is not theoretical: it made every
        // button's shadow reach into the row above and pushed a twelve-row list from
        // 3 draw calls to 27.
        Primitive::Rect { rect, clip, .. } => Some((Kind::Rect, rect.intersect(*clip))),
        Primitive::Image { rect, clip, .. } => Some((Kind::Image, rect.intersect(*clip))),
        Primitive::Path {
            path, stroke, clip, ..
        } => {
            // Half the line width falls outside the outline, on both sides.
            let half = stroke.as_ref().map_or(0.0, |s| s.width / 2.0);
            let b = path_bounds(path);
            let grown = Rect::new(
                b.x - half,
                b.y - half,
                b.width + half * 2.0,
                b.height + half * 2.0,
            );
            Some((Kind::Path, grown.intersect(*clip)))
        }
        // Text covers the box it was laid out in, bounded by its clip. That box is
        // the widget's, so it is wider than the glyphs — a level too many at worst.
        Primitive::Text { bounds, clip, .. } | Primitive::RichText { bounds, clip, .. } => {
            Some((Kind::Text, bounds.intersect(*clip)))
        }
        // A layer is rendered into its own texture and composited afterwards — which
        // for a long time meant *after everything*, so a group that had been covered
        // came back on top. It covers what its own contents cover, bounded by its clip
        // and moved by its transform, and it is ordered against the rest like anything
        // else.
        Primitive::Layer {
            primitives,
            clip,
            transform,
            ..
        } => {
            let mut inner: Option<Rect> = None;
            for p in primitives {
                if let Some((_, bounds)) = footprint(p) {
                    inner = Some(match inner {
                        Some(u) => u.union(bounds),
                        None => bounds,
                    });
                }
            }
            // An empty group covers nothing and still has to be ordered somewhere; its
            // clip is the honest answer, and an empty one is cheap either way.
            let bounds = inner.unwrap_or(*clip);
            let bounds = match transform {
                // The four corners through the matrix, then their box. A rotation's
                // box is bigger than the rectangle it came from, which is the safe
                // direction: over-estimating a footprint costs a draw call, and
                // under-estimating it loses a pixel.
                Some(t) => {
                    let (x0, y0) = (bounds.x, bounds.y);
                    let (x1, y1) = (bounds.x + bounds.width, bounds.y + bounds.height);
                    let corners = [
                        t.affine.apply(frus_core::Point::new(x0, y0)),
                        t.affine.apply(frus_core::Point::new(x1, y0)),
                        t.affine.apply(frus_core::Point::new(x0, y1)),
                        t.affine.apply(frus_core::Point::new(x1, y1)),
                    ];
                    let min_x = corners.iter().fold(f32::MAX, |m, p| m.min(p.x));
                    let min_y = corners.iter().fold(f32::MAX, |m, p| m.min(p.y));
                    let max_x = corners.iter().fold(f32::MIN, |m, p| m.max(p.x));
                    let max_y = corners.iter().fold(f32::MIN, |m, p| m.max(p.y));
                    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
                }
                None => bounds,
            };
            Some((Kind::Layer, bounds.intersect(*clip)))
        }
    }
}

/// Height of a band of the index, in logical pixels.
///
/// A power of two, so that dividing by it is exact and the band a coordinate falls in is
/// never a rounding decision. Sixty-four is about a row of an interface: a band holds a
/// row or two and whatever sits on them.
const BAND: f32 = 64.0;

/// A level is scanned member by member until it holds this many.
///
/// Below it, a scan of a hundred comparisons costs less than building an index would,
/// and the small scenes are most of the scenes — a dialog, a settings page — so they
/// keep the plain scan, verbatim. A first version indexed from twenty-four members, in
/// two dimensions, and planned a twelve-row list about three times slower than the scan
/// it replaced.
const INDEX_FROM: usize = 128;

/// A member crossing more bands than this is kept on a short list of its own and tested
/// by every query, rather than written into every band it crosses.
///
/// That list is the screen's backdrops and the odd full-height panel: a few, even on a
/// dense screen. A backdrop written into two hundred bands would cost more to insert
/// than every query it will ever answer.
const MAX_BANDS: i64 = 16;

/// The most bands a level's table may ever span. A member outside them goes on the short
/// list: a coordinate a million pixels off is not a place to allocate the way to.
const MAX_TABLE: i64 = 1 << 14;

/// The bands `r` crosses, inclusive — or `None` when its vertical extent is not a finite
/// number and no band would mean anything.
///
/// **This is what makes the index exact rather than approximate.** Two rectangles that
/// `overlaps` says share area have vertical extents that share a point, and that point
/// lies in one band — inside both ranges, because the band function is monotone. So a
/// member the index does not offer to a query is one that cannot overlap it: the index
/// only ever saves comparisons, and never skips one that would have answered.
///
/// The extent is normalised because `union` can hand back a negative height, and the
/// band is clamped rather than cast because a finite coordinate can still be far outside
/// any screen. Both are monotone, which is the only property the argument needs.
///
/// **Only the vertical axis**, and that is a measurement too. The first version indexed
/// both axes in a grid; it wrote every row of a list into seven to fourteen cells, each a
/// hashed entry with an allocation of its own, and lost to the plain scan below four
/// hundred primitives. An interface stacks vertically: the rows of a list, the sections
/// of a page, the cards of a feed. Height alone rejects almost everything a query could
/// meet, at one or two insertions a row and no hashing at all.
fn band_range(r: Rect) -> Option<(i32, i32)> {
    let (a, b) = (r.y, r.y + r.height);
    if !(a.is_finite() && b.is_finite()) {
        return None;
    }
    let (y0, y1) = if a <= b { (a, b) } else { (b, a) };
    let band = |v: f32| (v / BAND).floor().clamp(-1.0e9, 1.0e9) as i32;
    Some((band(y0), band(y1)))
}

/// Where a busy level's members are, by height, so that a query meets only the ones
/// that could be level with it.
struct Bands {
    /// The band that `table[0]` holds.
    first: i32,
    /// Members by the bands they cross, as positions in the level's `members`. A band
    /// nothing crosses is an empty vector, which does not allocate.
    table: Vec<Vec<u32>>,
    /// Members too tall to be worth writing into every band, or with no finite extent,
    /// or outside `window`: tested by every query.
    wide: Vec<u32>,
    /// The bands the table may ever cover, inclusive: `MAX_TABLE` of them, centred on
    /// where the level's members were when the index was built.
    window: (i64, i64),
}

impl Bands {
    /// An index over `members`, **anchored on the middle of them** rather than on
    /// whichever came first.
    ///
    /// Anchored on the first, one member a hundred million pixels off became the
    /// table's origin, and every ordinary member after it was too far from that origin
    /// to be written in — so it went on the list every query reads, and the index
    /// quietly became the scan it was meant to replace. Still exact; no longer fast; and
    /// nothing would have said so.
    fn build(members: &[(usize, Kind, Rect)]) -> Self {
        let mut tops: Vec<i32> = members
            .iter()
            .filter_map(|&(_, _, bounds)| band_range(bounds))
            .filter(|&(b0, b1)| i64::from(b1) - i64::from(b0) < MAX_BANDS)
            .map(|(b0, _)| b0)
            .collect();
        let centre = if tops.is_empty() {
            0
        } else {
            let middle = tops.len() / 2;
            *tops.select_nth_unstable(middle).1
        };
        let half = MAX_TABLE / 2;
        let mut bands = Bands {
            first: 0,
            table: Vec::new(),
            wide: Vec::new(),
            window: (i64::from(centre) - half, i64::from(centre) + half - 1),
        };
        for (at, &(_, _, bounds)) in members.iter().enumerate() {
            bands.insert(at as u32, bounds);
        }
        bands
    }

    fn insert(&mut self, at: u32, bounds: Rect) {
        let Some((b0, b1)) = band_range(bounds) else {
            self.wide.push(at);
            return;
        };
        let (lo, hi) = (i64::from(b0), i64::from(b1));
        if hi - lo >= MAX_BANDS || lo < self.window.0 || hi > self.window.1 {
            self.wide.push(at);
            return;
        }
        // Inside the window from here on, so every difference below fits comfortably.
        if self.table.is_empty() {
            self.first = b0;
        }
        // Scene order runs mostly down the page, so this is nearly always a push at the
        // end; a member above everything so far shifts the table once.
        if b0 < self.first {
            let grow = (self.first - b0) as usize;
            self.table
                .splice(0..0, std::iter::repeat_with(Vec::new).take(grow));
            self.first = b0;
        }
        let need = (b1 - self.first + 1) as usize;
        if need > self.table.len() {
            self.table.resize_with(need, Vec::new);
        }
        for band in b0..=b1 {
            self.table[(band - self.first) as usize].push(at);
        }
    }

    /// The bands of the table a query crossing `b0..=b1` meets.
    fn level_with(&self, b0: i32, b1: i32) -> &[Vec<u32>] {
        let lo = (i64::from(b0) - i64::from(self.first)).max(0);
        let hi = (i64::from(b1) - i64::from(self.first)).min(self.table.len() as i64 - 1);
        if lo > hi {
            &[]
        } else {
            &self.table[lo as usize..=hi as usize]
        }
    }
}

/// One level of the plan: primitives that provably do not cover one another, so they
/// may be drawn in any order among themselves.
struct Level {
    members: Vec<(usize, Kind, Rect)>,
    /// Their union, as a cheap rejection before testing them one by one.
    bounds: Rect,
    /// Built once the level is busy enough to be worth it (see `INDEX_FROM`).
    bands: Option<Box<Bands>>,
}

impl Level {
    fn new(bounds: Rect) -> Self {
        Self {
            members: Vec::new(),
            bounds,
            bands: None,
        }
    }

    fn push(&mut self, index: usize, kind: Kind, bounds: Rect) {
        self.bounds = if self.members.is_empty() {
            bounds
        } else {
            self.bounds.union(bounds)
        };
        let at = self.members.len() as u32;
        self.members.push((index, kind, bounds));
        match &mut self.bands {
            Some(bands) => bands.insert(at, bounds),
            None if self.members.len() >= INDEX_FROM => {
                self.bands = Some(Box::new(Bands::build(&self.members)));
            }
            None => {}
        }
    }

    /// Whether anything in this level overlaps `bounds` — and whether anything that does
    /// is of a kind other than `kind`.
    ///
    /// **Two existence questions, and that is what makes an index possible at all.**
    /// Neither depends on which member answers or in what order they are asked, so a
    /// structure that asks fewer of them gets the same two booleans the full scan does —
    /// provided it never leaves out one that would have said yes, which is what
    /// `band_range` is for. And once some foreign member has answered, both are settled
    /// and nothing further can change them.
    fn probe(&self, bounds: Rect, kind: Kind) -> (bool, bool) {
        // Nothing about the index is worked out for a level that has none: this runs for
        // every level every primitive meets, and a small level is the common one.
        if let Some(bands) = &self.bands {
            if let Some((b0, b1)) = band_range(bounds) {
                // A query as tall as half the table meets most of it, and meets a member
                // once per band it crosses: a plain scan is cheaper, and asks each once.
                if (i64::from(b1) - i64::from(b0) + 1) * 2 < bands.table.len() as i64 {
                    let mut hit = false;
                    let mut test = |at: u32| -> bool {
                        let (_, member_kind, member_bounds) = self.members[at as usize];
                        if overlaps(member_bounds, bounds) {
                            hit = true;
                            member_kind != kind
                        } else {
                            false
                        }
                    };
                    for &at in &bands.wide {
                        if test(at) {
                            return (true, true);
                        }
                    }
                    // A member crossing two of these bands is met twice, which the two
                    // booleans do not mind.
                    for band in bands.level_with(b0, b1) {
                        for &at in band {
                            if test(at) {
                                return (true, true);
                            }
                        }
                    }
                    return (hit, false);
                }
            }
        }
        // The plain scan, exactly as it was before there was an index: every member, no
        // way out half-way. So a level too small to be indexed is planned by the very
        // code the index is checked against, rather than by a third variant of it.
        let mut hit = false;
        let mut foreign = false;
        for &(_, member_kind, member_bounds) in &self.members {
            if overlaps(member_bounds, bounds) {
                hit = true;
                foreign |= member_kind != kind;
            }
        }
        (hit, foreign)
    }
}

/// Plans `scene` into batches, in the order they must be drawn.
///
/// Every primitive is given a **level**, from what it covers among the primitives
/// before it:
///
/// - it covers something of **another** kind: it goes one level above that, since
///   another kind means another draw call and the calls run in level order;
/// - it covers something of its **own** kind: the same level is enough, because a
///   batch draws its members in scene order and so already puts it on top;
/// - it covers nothing earlier: level zero.
///
/// A level therefore holds nothing that has to be drawn in a particular order across
/// kinds, so one draw call per kind present covers it.
///
/// This is what keeps the cost down. Ordering primitive against primitive would put a
/// checkbox's tick behind every later row's background and cost two calls a row; a
/// level puts every tick in the scene on one level, above every checkbox, and charges
/// one call for the lot. A dense list comes out at two calls rather than twenty-five.
pub(crate) fn plan(scene: &Scene) -> Vec<Batch> {
    plan_footprints(
        scene
            .primitives()
            .iter()
            .enumerate()
            .filter_map(|(index, p)| footprint(p).map(|(kind, bounds)| (index, kind, bounds))),
    )
}

/// `plan`, over footprints already worked out: `(scene index, kind, bounds)` in scene
/// order.
///
/// Apart so that it can be fed rectangles no scene would produce — a negative width, a
/// coordinate that is not a number — and shown to agree with the plain scan on those too.
fn plan_footprints(items: impl IntoIterator<Item = (usize, Kind, Rect)>) -> Vec<Batch> {
    let mut levels: Vec<Level> = Vec::new();
    for (index, kind, bounds) in items {
        // From the top down: the highest level holding something we cover decides.
        // Nothing below it can ask for more, being lower already.
        let mut level = 0;
        for (l, existing) in levels.iter().enumerate().rev() {
            if !overlaps(existing.bounds, bounds) {
                continue;
            }
            let (hit, foreign) = existing.probe(bounds, kind);
            if hit {
                level = if foreign { l + 1 } else { l };
                break;
            }
        }
        if level == levels.len() {
            levels.push(Level::new(bounds));
        }
        levels[level].push(index, kind, bounds);
    }

    // A level becomes one batch per kind it holds, in the order the kinds first
    // appear, and members keep their scene order inside each.
    let mut batches: Vec<Batch> = Vec::new();
    for level in &levels {
        // Where this level's batch of each drawable kind is, once it has one. There is
        // at most one per kind and it is the first, so this answers what a search from
        // the level's first batch would — without walking past every layer to find it.
        let mut lanes: [Option<usize>; 4] = [None; 4];
        for &(index, kind, bounds) in &level.members {
            // A layer is one draw of its own: it is composited from its own texture,
            // and two of them on the same level still have to keep their scene order.
            let lane = match kind {
                Kind::Rect => Some(0),
                Kind::Image => Some(1),
                Kind::Path => Some(2),
                Kind::Text => Some(3),
                Kind::Layer => None,
            };
            match lane.and_then(|l| lanes[l]) {
                Some(at) => {
                    let batch = &mut batches[at];
                    batch.members.push(index);
                    batch.bounds = batch.bounds.union(bounds);
                }
                None => {
                    if let Some(l) = lane {
                        lanes[l] = Some(batches.len());
                    }
                    batches.push(Batch {
                        kind,
                        members: vec![index],
                        bounds,
                    });
                }
            }
        }
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{Color, Point};

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    fn scene_of(f: impl FnOnce(&mut Scene)) -> Scene {
        let mut scene = Scene::new();
        f(&mut scene);
        scene
    }

    fn bar(x: f32, y: f32, w: f32, h: f32) -> Path {
        Path::rect(Rect::new(x, y, w, h))
    }

    /// The ordinary case, and the one that has to stay cheap: a column of buttons,
    /// each a background and an icon that are nowhere near the next button's. Every
    /// rectangle shares one call, every path shares another.
    #[test]
    fn things_that_do_not_touch_share_a_draw_call() {
        let scene = scene_of(|s| {
            for i in 0..4 {
                let y = i as f32 * 100.0;
                s.fill_rect(Rect::new(0.0, y, 80.0, 40.0), RED);
                s.fill_path(&bar(200.0, y, 20.0, 20.0), RED);
            }
        });
        let batches = plan(&scene);
        assert_eq!(batches.len(), 2, "{batches:#?}");
        assert_eq!(batches[0].kind, Kind::Rect);
        assert_eq!(batches[0].members.len(), 4);
        assert_eq!(batches[1].kind, Kind::Path);
        assert_eq!(batches[1].members.len(), 4);
    }

    /// The bug this exists for: a bar drawn as a path, with a filled button on it.
    /// The button comes after the bar in the scene, so it must come after it in the
    /// drawing — which means it cannot join the rectangles that went down first.
    #[test]
    fn a_rectangle_over_a_path_is_drawn_after_it() {
        let scene = scene_of(|s| {
            s.fill_rect(Rect::new(0.0, 0.0, 400.0, 600.0), RED); // the background
            s.fill_path(&bar(0.0, 540.0, 400.0, 60.0), RED); // the bar
            s.fill_rect(Rect::new(20.0, 550.0, 90.0, 40.0), RED); // a button on it
        });
        let batches = plan(&scene);
        assert_eq!(batches.len(), 3, "{batches:#?}");
        assert_eq!(batches[0].members, vec![0]);
        assert_eq!(batches[1].members, vec![1]);
        assert_eq!(batches[2].members, vec![2], "the button lost its own call");
        assert_eq!(batches[2].kind, Kind::Rect);
    }

    /// The same shapes, moved apart: the button is no longer on the bar, so there is
    /// nothing to stay above and the two rectangles batch again.
    #[test]
    fn the_same_rectangle_clear_of_the_path_rejoins_the_batch() {
        let scene = scene_of(|s| {
            s.fill_rect(Rect::new(0.0, 0.0, 400.0, 100.0), RED);
            s.fill_path(&bar(0.0, 540.0, 400.0, 60.0), RED);
            s.fill_rect(Rect::new(20.0, 200.0, 90.0, 40.0), RED);
        });
        let batches = plan(&scene);
        assert_eq!(batches.len(), 2, "{batches:#?}");
        assert_eq!(batches[0].members, vec![0, 2]);
        assert_eq!(batches[1].members, vec![1]);
    }

    /// Every primitive the planner orders keeps its place: a batch's members are in
    /// scene order, and reading the batches in order reproduces a sequence in which
    /// no primitive precedes one that overlaps it and came earlier.
    #[test]
    fn the_plan_never_reorders_two_overlapping_primitives() {
        let scene = scene_of(|s| {
            s.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), RED);
            s.fill_path(&bar(50.0, 50.0, 100.0, 100.0), RED);
            s.fill_rect(Rect::new(90.0, 90.0, 100.0, 100.0), RED);
            s.fill_path(&bar(0.0, 0.0, 10.0, 10.0), RED);
        });
        let batches = plan(&scene);
        // The order the GPU will see.
        let order: Vec<usize> = batches
            .iter()
            .flat_map(|b| b.members.iter().copied())
            .collect();
        let boxes: Vec<Rect> = scene
            .primitives()
            .iter()
            .map(|p| footprint(p).expect("planned").1)
            .collect();
        for (i, &a) in order.iter().enumerate() {
            for &b in &order[i + 1..] {
                if overlaps(boxes[a], boxes[b]) {
                    assert!(a < b, "{a} was drawn before {b} but comes after it");
                }
            }
        }
    }

    /// A **stroke** puts half its width outside the outline it follows, and that
    /// counts, or a batch would be broken a pixel too late. A shadow's blur does not:
    /// it softens the edge inside the quad, and the widget casting one has already
    /// widened the rectangle.
    #[test]
    fn a_stroke_claims_the_room_it_spills_into() {
        let mut stroked = Scene::new();
        stroked.stroke_path(
            &Path::new()
                .move_to(Point::new(100.0, 100.0))
                .line_to(Point::new(200.0, 100.0)),
            RED,
            20.0,
        );
        stroked.fill_rect(Rect::new(120.0, 95.0, 10.0, 4.0), RED);
        assert_eq!(plan(&stroked).len(), 2, "the stroke's width was ignored");
    }

    /// The defect this exists for, in miniature: a label, then a menu panel dropped
    /// over it. The panel comes later in the scene, so it has to be drawn later — and
    /// before this, text was a pass above the whole frame and the label read straight
    /// through the panel. A committed golden had it that way.
    #[test]
    fn a_panel_dropped_over_a_label_covers_it() {
        let mut scene = Scene::new();
        scene.set_bounds(Rect::new(20.0, 20.0, 120.0, 20.0));
        scene.text(
            Point::new(20.0, 20.0),
            "Score",
            &frus_core::ResolvedTextStyle::exact(14.0),
            RED,
        );
        scene.set_bounds(Rect::new(10.0, 10.0, 200.0, 100.0));
        scene.fill_rect(Rect::new(10.0, 10.0, 200.0, 100.0), RED);
        let batches = plan(&scene);
        assert_eq!(batches.len(), 2, "{batches:#?}");
        assert_eq!(batches[0].kind, Kind::Text, "the label goes down first");
        assert_eq!(batches[1].kind, Kind::Rect, "the panel goes over it");
    }

    /// The other half: a label *on* the panel is drawn after it, and labels that are
    /// nowhere near one another still share a single call.
    #[test]
    fn labels_batch_together_above_what_they_sit_on() {
        let mut scene = Scene::new();
        for i in 0..4 {
            let y = i as f32 * 40.0;
            scene.set_bounds(Rect::new(0.0, y, 200.0, 30.0));
            scene.fill_rect(Rect::new(0.0, y, 200.0, 30.0), RED);
            scene.text(
                Point::new(8.0, y + 6.0),
                "row",
                &frus_core::ResolvedTextStyle::exact(14.0),
                RED,
            );
        }
        let batches = plan(&scene);
        assert_eq!(batches.len(), 2, "{batches:#?}");
        assert_eq!(batches[0].kind, Kind::Rect);
        assert_eq!(batches[0].members.len(), 4);
        assert_eq!(batches[1].kind, Kind::Text);
        assert_eq!(batches[1].members.len(), 4, "four labels, one call");
    }

    /// A scene built by hand rather than by the widget walk leaves text's box
    /// `UNBOUNDED`, which overlaps everything: such text is ordered strictly by its
    /// place in the scene, since nothing says what it really covers. Conservative, and
    /// the only safe reading.
    #[test]
    fn text_with_no_box_is_ordered_by_its_place_in_the_scene() {
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), RED);
        scene.text(
            Point::new(10.0, 10.0),
            "hello",
            &frus_core::ResolvedTextStyle::exact(16.0),
            RED,
        );
        scene.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), RED);
        let batches = plan(&scene);
        assert_eq!(batches.len(), 3, "{batches:#?}");
        assert_eq!(batches[0].members, vec![0]);
        assert_eq!(batches[1].kind, Kind::Text);
        assert_eq!(batches[2].members, vec![2]);
    }

    // --- The index (milestone 500) ---

    /// **The planner as it was before the index**: every member of a level tested in
    /// turn. Kept verbatim, as the definition the index has to agree with — "the plan
    /// must not change" is a claim about this function, so it has to stay somewhere a
    /// test can run it.
    fn plan_linear(items: &[(usize, Kind, Rect)]) -> Vec<(Kind, Vec<usize>)> {
        struct Plain {
            members: Vec<(usize, Kind, Rect)>,
            bounds: Rect,
        }
        let mut levels: Vec<Plain> = Vec::new();
        for &(index, kind, bounds) in items {
            let mut level = 0;
            for (l, existing) in levels.iter().enumerate().rev() {
                if !overlaps(existing.bounds, bounds) {
                    continue;
                }
                let mut hit = false;
                let mut foreign = false;
                for &(_, member_kind, member_bounds) in &existing.members {
                    if overlaps(member_bounds, bounds) {
                        hit = true;
                        foreign |= member_kind != kind;
                    }
                }
                if hit {
                    level = if foreign { l + 1 } else { l };
                    break;
                }
            }
            if level == levels.len() {
                levels.push(Plain {
                    members: Vec::new(),
                    bounds,
                });
            }
            let target = &mut levels[level];
            target.bounds = if target.members.is_empty() {
                bounds
            } else {
                target.bounds.union(bounds)
            };
            target.members.push((index, kind, bounds));
        }
        let mut batches: Vec<(Kind, Vec<usize>)> = Vec::new();
        for level in &levels {
            let first = batches.len();
            for &(index, kind, _) in &level.members {
                let existing = if kind == Kind::Layer {
                    None
                } else {
                    batches[first..].iter_mut().find(|b| b.0 == kind)
                };
                match existing {
                    Some(batch) => batch.1.push(index),
                    None => batches.push((kind, vec![index])),
                }
            }
        }
        batches
    }

    fn shape(batches: &[Batch]) -> Vec<(Kind, Vec<usize>)> {
        batches
            .iter()
            .map(|b| (b.kind, b.members.clone()))
            .collect()
    }

    fn footprints(scene: &Scene) -> Vec<(usize, Kind, Rect)> {
        scene
            .primitives()
            .iter()
            .enumerate()
            .filter_map(|(i, p)| footprint(p).map(|(k, b)| (i, k, b)))
            .collect()
    }

    /// A small deterministic generator: a failure has to be reproducible from the seed
    /// in its message, and the crate takes no dependency for a test.
    struct Dice(u64);

    impl Dice {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        fn unit(&mut self) -> f32 {
            (self.next() >> 40) as f32 / (1u64 << 24) as f32
        }
    }

    /// A footprint of every sort a plan can meet, and several sorts it should never
    /// meet but must survive: the exactness argument is about *any* rectangle, so the
    /// test is too.
    ///
    /// **Not enough on its own**, and that was found by breaking the index on purpose.
    /// One footprint in eight here covers everything — `UNBOUNDED`, or a backdrop — and
    /// such members answer every query themselves, so a scene built from this alone
    /// passes with half the index missing. See `sparse_rect`.
    fn any_rect(d: &mut Dice) -> Rect {
        let (x, y) = (d.unit() * 400.0, d.unit() * 4000.0);
        match d.below(24) {
            0 => Rect::UNBOUNDED,
            // No area at all, which `overlaps` still counts when strictly inside.
            1 => Rect::new(x, y, 0.0, 0.0),
            // What `union` can hand back.
            2 => Rect::new(x, y, -d.unit() * 80.0, d.unit() * 80.0),
            3 => Rect::new(f32::NAN, y, 20.0, 20.0),
            4 => Rect::new(x, y, f32::INFINITY, 20.0),
            // Finite, and nowhere near any screen.
            5 => Rect::new(1.0e8 + x, -1.0e8 + y, 40.0, 40.0),
            6 | 7 => {
                let (cx, cy) = (d.below(8) as f32, d.below(60) as f32);
                Rect::new(cx * 64.0, cy * 64.0, 64.0, 64.0)
            }
            // Backdrops: tall enough to go on the list every query reads.
            8..=10 => Rect::new(0.0, 0.0, 400.0 + x, 800.0 + y),
            // Rows that abut, edge on edge — touching is not overlapping.
            11..=15 => Rect::new(0.0, d.below(70) as f32 * 56.0, 400.0, 56.0),
            _ => Rect::new(x, y, d.unit() * 48.0, d.unit() * 48.0),
        }
    }

    /// A footprint from a **sparse** page: small things, rows that cross band
    /// boundaries, tiles lying exactly on them, things with no height or a negative one —
    /// and nothing that covers everything.
    ///
    /// Here a query usually meets one member or none, so the member the index fails to
    /// offer is the member that decides the answer. This is the generator that makes the
    /// tests below able to fail.
    fn sparse_rect(d: &mut Dice) -> Rect {
        let (x, y) = (d.unit() * 400.0, d.unit() * 6000.0);
        match d.below(10) {
            0 => Rect::new(x, y, d.unit() * 300.0, 0.0),
            1 => Rect::new(x, y, 30.0, -d.unit() * 60.0),
            // On the boundaries: a tile's top edge is where one band starts and its
            // bottom edge where the next does.
            2 | 3 => {
                let (cx, cy) = (d.below(8) as f32, d.below(90) as f32);
                Rect::new(cx * 64.0, cy * 64.0, 64.0, 64.0)
            }
            // Rows at any height, so that most of them straddle two bands.
            4 | 5 => Rect::new(d.unit() * 200.0, y, 120.0 + d.unit() * 200.0, 56.0),
            _ => Rect::new(x, y, 4.0 + d.unit() * 40.0, 4.0 + d.unit() * 40.0),
        }
    }

    fn any_kind(d: &mut Dice) -> Kind {
        [Kind::Rect, Kind::Image, Kind::Path, Kind::Text, Kind::Layer][d.below(5) as usize]
    }

    /// **The index changes no plan**, over three hundred scenes of up to nine hundred
    /// footprints — half of them sparse pages, where the index decides answers, and half
    /// every sort of footprint there is, where it has to survive them.
    #[test]
    fn the_index_changes_no_plan() {
        for seed in 1..=300u64 {
            let mut d = Dice(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let n = d.below(900) as usize;
            let sparse = seed % 2 == 0;
            let items: Vec<(usize, Kind, Rect)> = (0..n)
                .map(|i| {
                    let kind = any_kind(&mut d);
                    let rect = if sparse {
                        sparse_rect(&mut d)
                    } else {
                        any_rect(&mut d)
                    };
                    (i, kind, rect)
                })
                .collect();
            assert_eq!(
                shape(&plan_footprints(items.iter().copied())),
                plan_linear(&items),
                "seed {seed}, {n} footprints, sparse: {sparse}"
            );
        }
    }

    /// The same, through `plan` itself and a scene built the way a frame builds one: a
    /// long list of rows, each a background, an icon, a label and a picture, under
    /// clips, with composited groups among them and an unbounded label to finish.
    #[test]
    fn the_index_changes_no_plan_on_a_real_scene() {
        let picture = frus_core::ImageData::from_rgba(1, 1, vec![255; 4]).into_handle();
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 400.0, 800.0), RED);
        for i in 0..400 {
            let y = i as f32 * 48.0;
            scene.set_clip(Rect::new(0.0, 0.0, 400.0, 800.0 + (i % 7) as f32 * 300.0));
            scene.set_bounds(Rect::new(0.0, y, 400.0, 48.0));
            scene.fill_rect(Rect::new(0.0, y, 400.0, 48.0), RED);
            scene.fill_path(&bar(12.0, y + 12.0, 24.0, 24.0), RED);
            scene.text(
                Point::new(48.0, y + 14.0),
                "row",
                &frus_core::ResolvedTextStyle::exact(14.0),
                RED,
            );
            scene.draw_image(
                &picture,
                Rect::new(360.0, y + 8.0, 32.0, 32.0),
                Rect::new(0.0, 0.0, 1.0, 1.0),
                RED,
            );
            if i % 50 == 0 {
                scene.layer(0.5, |s| s.fill_rect(Rect::new(20.0, y, 200.0, 96.0), RED));
            }
        }
        scene.text(
            Point::new(10.0, 10.0),
            "no box",
            &frus_core::ResolvedTextStyle::exact(16.0),
            RED,
        );
        let planned = plan(&scene);
        assert_eq!(shape(&planned), plan_linear(&footprints(&scene)));
        assert!(
            planned.len() > 2,
            "a plan with some structure to it: {}",
            planned.len()
        );
    }

    /// **And it is actually used, and actually tested.** A test that the plan did not
    /// change would pass just as well if the index never switched on — or if it were
    /// half broken and something else always answered first, which is what the first
    /// version of this test allowed. So this one opens a busy level up, checks it is
    /// indexed, and **counts the queries its answer hung on a single member**: those are
    /// the ones a missing band would get wrong, and a test with too few of them is a test
    /// that cannot fail.
    #[test]
    fn a_busy_level_is_indexed_and_answers_as_a_scan_would() {
        let mut d = Dice(0x5EED);
        let mut level = Level::new(Rect::new(0.0, 0.0, 0.0, 0.0));
        for i in 0..600 {
            level.push(i, any_kind(&mut d), sparse_rect(&mut d));
        }
        // A few that go on the wide list without answering for everyone.
        level.push(600, Kind::Rect, Rect::new(10.0, 1.0e8, 20.0, 20.0));
        // A height that is not a number, not a left edge: the index reads only the
        // vertical axis, so a stray horizontal coordinate is not its business.
        level.push(601, Kind::Path, Rect::new(10.0, f32::NAN, 20.0, 20.0));
        level.push(602, Kind::Text, Rect::new(0.0, 0.0, 400.0, 64.0 * 40.0));
        let bands = level.bands.as_ref().expect("a level this busy is indexed");
        assert_eq!(bands.wide.len(), 3, "exactly the three that belong there");
        let used = bands.table.iter().filter(|b| !b.is_empty()).count();
        assert!(used > 50, "and the rest into bands: {used}");
        let mut decisive = 0;
        for _ in 0..4000 {
            let (query, kind) = (sparse_rect(&mut d), any_kind(&mut d));
            let mut hits = 0;
            let mut foreign = false;
            for &(_, k, b) in &level.members {
                if overlaps(b, query) {
                    hits += 1;
                    foreign |= k != kind;
                }
            }
            decisive += usize::from(hits == 1);
            assert_eq!(
                level.probe(query, kind),
                (hits > 0, foreign),
                "{query:?} as {kind:?}"
            );
        }
        assert!(
            decisive > 400,
            "a test whose answers hang on one member this rarely cannot fail: {decisive}"
        );
    }

    /// **One member far away does not become the index's origin.** Anchored on the
    /// first member, a stray footprint a hundred million pixels down sent every ordinary
    /// member after it to the list every query reads: exact still, and quietly as slow as
    /// the scan the index replaced.
    #[test]
    fn a_stray_member_does_not_become_the_origin() {
        let mut level = Level::new(Rect::new(0.0, 0.0, 0.0, 0.0));
        level.push(0, Kind::Rect, Rect::new(0.0, 1.0e8, 10.0, 10.0));
        for i in 1..500 {
            level.push(i, Kind::Rect, Rect::new(0.0, i as f32 * 56.0, 400.0, 56.0));
        }
        let bands = level.bands.as_ref().expect("indexed");
        assert_eq!(
            bands.wide.len(),
            1,
            "only the stray goes on the wide list, not the four hundred and ninety-nine rows"
        );
    }
}
