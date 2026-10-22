//! The **triangles of a path, kept from frame to frame** (milestone 692).
//!
//! Tessellating is the costly part of drawing a path, and what it produces depends on the
//! outline alone: the colour, clip and gradient go into every vertex afterwards. So the
//! triangles are kept, keyed by the outline **relative to its first point**, and a path that
//! comes back — the same icon in the next frame, in a list row scrolled by a few pixels, in an
//! animation's layer that only moved — is drawn without lyon.
//!
//! The key rounds the outline to 1/256 px. Two paths that round alike share triangles that
//! are at most half of that apart from either, far below the 0.1 px lyon flattens curves to.
//!
//! An entry not drawn for more than [`KEEP_FRAMES`] frames is dropped, and when the cache holds more
//! than [`MAX_VERTICES`] vertices it keeps only what the last frame drew — an animation that
//! changes shape on every frame must not grow it without end.

use frus_core::{FillRule, Path, PathVerb, Point, StrokeCap};
use lyon::math::point;
use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use std::collections::HashMap;
use std::sync::Arc;

/// How many frames an entry outlives its last use: two seconds at 60 Hz, so a list scrolled
/// away and back, or an animation's loop, finds its shapes again.
pub(crate) const KEEP_FRAMES: u64 = 120;

/// How many vertices the cache may hold across its entries, ~10 MB with their indices.
pub(crate) const MAX_VERTICES: usize = 1 << 18;

/// The fineness of the key: coordinates are rounded to `1 / KEY_SCALE` px.
const KEY_SCALE: f32 = 256.0;

/// A path's triangles, relative to the path's first point.
#[derive(Debug, Default)]
pub(crate) struct Mesh {
    pub(crate) positions: Vec<[f32; 2]>,
    pub(crate) indices: Vec<u32>,
}

/// What is done to an outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Op {
    Fill(FillRule),
    /// The width, rounded like the coordinates, and the cap.
    Stroke {
        width: i32,
        cap: StrokeCap,
    },
}

impl Op {
    pub(crate) fn stroke(width: f32, cap: StrokeCap) -> Self {
        Op::Stroke {
            width: quantize(width),
            cap,
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct Key {
    op: Op,
    /// Each verb's tag, then its points, rounded and relative to the first point.
    outline: Vec<i32>,
}

struct Entry {
    mesh: Arc<Mesh>,
    used: u64,
}

/// The cache, owned by the path painter.
pub(crate) struct MeshCache {
    entries: HashMap<Key, Entry>,
    frame: u64,
    vertices: usize,
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
    /// How many lookups found their triangles, and how many tessellated, since the start.
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl Default for MeshCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            frame: 0,
            vertices: 0,
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
            hits: 0,
            misses: 0,
        }
    }
}

impl MeshCache {
    /// `path`'s triangles under `op`, and the point they are relative to; `None` for a path
    /// with nothing to draw.
    pub(crate) fn get(&mut self, path: &Path, op: Op) -> Option<(Arc<Mesh>, Point)> {
        let origin = first_point(path)?;
        let key = Key {
            op,
            outline: outline(path, origin),
        };
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.frame;
            self.hits += 1;
            return Some((entry.mesh.clone(), origin));
        }
        self.misses += 1;
        let mesh = Arc::new(self.tessellate(path, origin, op));
        self.vertices += mesh.positions.len();
        self.entries.insert(
            key,
            Entry {
                mesh: mesh.clone(),
                used: self.frame,
            },
        );
        Some((mesh, origin))
    }

    /// Ends a frame: drops what has not been drawn for a while, or, over the budget,
    /// everything the frame did not draw.
    pub(crate) fn end_frame(&mut self) {
        let frame = self.frame;
        let keep = if self.vertices > MAX_VERTICES {
            0
        } else {
            KEEP_FRAMES
        };
        let vertices = &mut self.vertices;
        self.entries.retain(|_, entry| {
            let kept = frame - entry.used <= keep;
            if !kept {
                *vertices -= entry.mesh.positions.len();
            }
            kept
        });
        self.frame += 1;
    }

    /// How many outlines are kept.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    fn tessellate(&mut self, path: &Path, origin: Point, op: Op) -> Mesh {
        let lyon_path = to_lyon(path, origin);
        let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        match op {
            Op::Fill(rule) => {
                let rule = match rule {
                    FillRule::EvenOdd => lyon::tessellation::FillRule::EvenOdd,
                    FillRule::NonZero => lyon::tessellation::FillRule::NonZero,
                };
                let _ = self.fill_tess.tessellate_path(
                    &lyon_path,
                    &FillOptions::default().with_fill_rule(rule),
                    &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position().to_array()),
                );
            }
            Op::Stroke { width, cap } => {
                let cap = match cap {
                    StrokeCap::Butt => lyon::tessellation::LineCap::Butt,
                    StrokeCap::Square => lyon::tessellation::LineCap::Square,
                    StrokeCap::Round => lyon::tessellation::LineCap::Round,
                };
                let options = StrokeOptions::default()
                    .with_line_width(width as f32 / KEY_SCALE)
                    .with_line_cap(cap);
                let _ = self.stroke_tess.tessellate_path(
                    &lyon_path,
                    &options,
                    &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| {
                        v.position().to_array()
                    }),
                );
            }
        }
        Mesh {
            positions: buffers.vertices,
            indices: buffers.indices,
        }
    }
}

fn quantize(v: f32) -> i32 {
    (v * KEY_SCALE).round() as i32
}

fn first_point(path: &Path) -> Option<Point> {
    path.verbs().iter().find_map(|verb| match verb {
        PathVerb::MoveTo(p) => Some(*p),
        _ => None,
    })
}

fn outline(path: &Path, origin: Point) -> Vec<i32> {
    let mut out = Vec::with_capacity(path.verbs().len() * 4);
    let push = |out: &mut Vec<i32>, p: &Point| {
        out.push(quantize(p.x - origin.x));
        out.push(quantize(p.y - origin.y));
    };
    for verb in path.verbs() {
        match verb {
            PathVerb::MoveTo(p) => {
                out.push(0);
                push(&mut out, p);
            }
            PathVerb::LineTo(p) => {
                out.push(1);
                push(&mut out, p);
            }
            PathVerb::QuadTo { ctrl, to } => {
                out.push(2);
                push(&mut out, ctrl);
                push(&mut out, to);
            }
            PathVerb::CubicTo { c1, c2, to } => {
                out.push(3);
                push(&mut out, c1);
                push(&mut out, c2);
                push(&mut out, to);
            }
            PathVerb::Close => out.push(4),
        }
    }
    out
}

/// Converts a frus [`Path`] into a lyon path moved by `-origin`, handling sub-path opening
/// and closing — lyon requires `begin`/`end` around every contour.
fn to_lyon(path: &Path, origin: Point) -> LyonPath {
    let at = |p: &Point| point(p.x - origin.x, p.y - origin.y);
    let mut builder = LyonPath::builder();
    let mut open = false;
    for verb in path.verbs() {
        match verb {
            PathVerb::MoveTo(p) => {
                if open {
                    builder.end(false);
                }
                builder.begin(at(p));
                open = true;
            }
            PathVerb::LineTo(p) => {
                if open {
                    builder.line_to(at(p));
                }
            }
            PathVerb::QuadTo { ctrl, to } => {
                if open {
                    builder.quadratic_bezier_to(at(ctrl), at(to));
                }
            }
            PathVerb::CubicTo { c1, c2, to } => {
                if open {
                    builder.cubic_bezier_to(at(c1), at(c2), at(to));
                }
            }
            PathVerb::Close => {
                if open {
                    builder.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        builder.end(false);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Rect;

    fn circle(x: f32, y: f32) -> Path {
        Path::circle(Point::new(x, y), 20.0)
    }

    #[test]
    fn a_path_that_only_moved_is_not_tessellated_again() {
        let mut cache = MeshCache::default();
        let (first, at) = cache
            .get(&circle(50.0, 50.0), Op::Fill(FillRule::NonZero))
            .unwrap();
        assert!(!first.indices.is_empty());
        cache.end_frame();
        // Scrolled by a fraction of a pixel: the same triangles, from the new first point.
        let (again, moved) = cache
            .get(&circle(50.0, 87.3), Op::Fill(FillRule::NonZero))
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!((cache.hits, cache.misses), (1, 1));
        assert!((moved.y - at.y - 37.3).abs() < 1e-4);
    }

    #[test]
    fn what_is_done_to_the_outline_is_part_of_the_key() {
        let mut cache = MeshCache::default();
        let path = circle(0.0, 0.0);
        cache.get(&path, Op::Fill(FillRule::NonZero));
        cache.get(&path, Op::Fill(FillRule::EvenOdd));
        cache.get(&path, Op::stroke(2.0, StrokeCap::Butt));
        cache.get(&path, Op::stroke(3.0, StrokeCap::Butt));
        cache.get(&path, Op::stroke(3.0, StrokeCap::Round));
        cache.get(
            &Path::circle(Point::new(0.0, 0.0), 21.0),
            Op::Fill(FillRule::NonZero),
        );
        assert_eq!((cache.hits, cache.misses), (0, 6));
        cache.get(&path, Op::stroke(3.0, StrokeCap::Round));
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn the_triangles_are_the_ones_lyon_makes_where_the_path_is() {
        // A 10×10 square at (100, 200): relative to its corner, and covering 100 px².
        let mut cache = MeshCache::default();
        let square = Path::rect(Rect::new(100.0, 200.0, 10.0, 10.0));
        let (mesh, origin) = cache.get(&square, Op::Fill(FillRule::NonZero)).unwrap();
        assert_eq!((origin.x, origin.y), (100.0, 200.0));
        let area: f32 = mesh
            .indices
            .chunks(3)
            .map(|t| {
                let [a, b, c] = [0, 1, 2].map(|i| mesh.positions[t[i] as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum();
        assert!((area - 100.0).abs() < 1e-3, "{area}");
        assert!(mesh
            .positions
            .iter()
            .all(|p| (0.0..=10.0).contains(&p[0]) && (0.0..=10.0).contains(&p[1])));
    }

    #[test]
    fn an_empty_path_draws_nothing_and_keeps_nothing() {
        let mut cache = MeshCache::default();
        assert!(cache
            .get(&Path::new(), Op::Fill(FillRule::NonZero))
            .is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn what_is_not_drawn_for_a_while_is_dropped() {
        let mut cache = MeshCache::default();
        cache.get(&circle(0.0, 0.0), Op::Fill(FillRule::NonZero));
        // The frame it was drawn in, then KEEP_FRAMES frames without it.
        for _ in 0..=KEEP_FRAMES {
            cache.end_frame();
        }
        assert_eq!(cache.len(), 1, "still within its frames");
        cache.end_frame();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.vertices, 0);
    }

    #[test]
    fn over_the_budget_only_the_last_frame_is_kept() {
        let mut cache = MeshCache::default();
        cache.get(&circle(0.0, 0.0), Op::Fill(FillRule::NonZero));
        cache.end_frame();
        // Then a frame of shapes all different, as an animation's may be, past the budget.
        let mut drawn = 0;
        while cache.vertices <= MAX_VERTICES {
            let r = 50.0 + drawn as f32 / 64.0;
            cache.get(
                &Path::circle(Point::new(0.0, 0.0), r),
                Op::stroke(1.0, StrokeCap::Butt),
            );
            drawn += 1;
        }
        cache.end_frame();
        assert_eq!(cache.len(), drawn, "only what the last frame drew");
    }
}
