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
    a.x < b.x + b.width
        && b.x < a.x + a.width
        && a.y < b.y + b.height
        && b.y < a.y + a.height
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

/// What a primitive covers, and which pipeline draws it — or `None` for the kinds this
/// planner does not order (text, and layers, which are composited separately).
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
        // A layer is rendered into its own texture and composited afterwards.
        Primitive::Layer { .. } => None,
    }
}

/// One level of the plan: primitives that provably do not cover one another, so they
/// may be drawn in any order among themselves.
struct Level {
    members: Vec<(usize, Kind, Rect)>,
    /// Their union, as a cheap rejection before testing them one by one.
    bounds: Rect,
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
    let mut levels: Vec<Level> = Vec::new();
    for (index, primitive) in scene.primitives().iter().enumerate() {
        let Some((kind, bounds)) = footprint(primitive) else {
            continue;
        };
        // From the top down: the highest level holding something we cover decides.
        // Nothing below it can ask for more, being lower already.
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
            levels.push(Level {
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

    // A level becomes one batch per kind it holds, in the order the kinds first
    // appear, and members keep their scene order inside each.
    let mut batches: Vec<Batch> = Vec::new();
    for level in &levels {
        let first = batches.len();
        for &(index, kind, bounds) in &level.members {
            match batches[first..].iter_mut().find(|b| b.kind == kind) {
                Some(batch) => {
                    batch.members.push(index);
                    batch.bounds = batch.bounds.union(bounds);
                }
                None => batches.push(Batch {
                    kind,
                    members: vec![index],
                    bounds,
                }),
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
        let order: Vec<usize> = batches.iter().flat_map(|b| b.members.iter().copied()).collect();
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
        scene.text(Point::new(20.0, 20.0), "Score", 14.0, RED);
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
            scene.text(Point::new(8.0, y + 6.0), "row", 14.0, RED);
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
        scene.text(Point::new(10.0, 10.0), "hello", 16.0, RED);
        scene.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), RED);
        let batches = plan(&scene);
        assert_eq!(batches.len(), 3, "{batches:#?}");
        assert_eq!(batches[0].members, vec![0]);
        assert_eq!(batches[1].kind, Kind::Text);
        assert_eq!(batches[2].members, vec![2]);
    }
}
