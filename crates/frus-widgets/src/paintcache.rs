//! The **repaint boundary** cache: holds, between frames, the painted output
//! (primitives + interaction maps) of a subtree marked
//! [`crate::Widget::repaint_boundary`], and **reuses it as is** for as long as
//! its geometry and its descendants' interaction state have not moved.
//!
//! It is the *paint* counterpart of the relayout cache (`relayout.rs`, milestone
//! 55): where the layout cache reuses the **rectangles** when the *style and
//! structure* are stable, this one reuses the **primitives** when the *state read
//! at paint time* (hover, focus, animated value, opacity, cursor) and the
//! **geometry** are stable. A widget animating elsewhere on screen no longer
//! forces a static subtree to repaint.
//!
//! ## Correctness
//! - Any rebuild of the `view` (a change of state, theme or size) **bumps the
//!   generation**; an entry from a stale generation is ignored. Since the widgets'
//!   configuration is then identical from one frame to the next (the tree is the
//!   **same** retained object for as long as `build` does not run), an entry from
//!   the current generation corresponds to an identical configuration.
//! - The **fingerprint** covers the rest: each descendant's interaction state
//!   (`Status`) and the subtree's absolute rectangles. Equal fingerprint and
//!   generation ⇒ the painting would be **bit-for-bit identical** → replay the cache.
//!
//! The cache does not know the application's `Msg` type (the `Runtime` is
//! generic-agnostic): the data is stored **erased** behind a `Box<dyn Any>`, and
//! `ui.rs` downcasts it back to its concrete `BoundaryData<Msg>` (a single `Msg`
//! per app → the `downcast` always succeeds).

use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::interaction::WidgetId;

/// One entry: the generation and fingerprint the output was captured under, the
/// number of rectangles the subtree consumes (to advance the walk index on a
/// *hit*), and the erased painted data.
struct Slot {
    generation: u64,
    fingerprint: u64,
    rect_count: usize,
    data: Box<dyn Any>,
}

/// The paint cache, retained in the [`crate::Runtime`] from one frame to the next.
#[derive(Default)]
pub struct PaintCache {
    entries: HashMap<WidgetId, Slot>,
    /// Boundaries touched during the current frame (to evict the vanished ones).
    touched: HashSet<WidgetId>,
    /// The current generation: bumped on every rebuild of the `view`.
    generation: u64,
    hits: u32,
    misses: u32,
    last_hits: u32,
    last_misses: u32,
}

impl PaintCache {
    /// Invalidates the whole cache **logically**: the `view` has been rebuilt, so
    /// the widgets' configuration may have changed. Entries from the old
    /// generation will never be *hits* again.
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// A boundary's erased data if its generation **and** its fingerprint match (a
    /// *hit*), along with the number of rectangles it covers. Marks the boundary
    /// as touched, so it survives the end of the frame.
    pub(crate) fn get(&mut self, key: WidgetId, fingerprint: u64) -> Option<(usize, &dyn Any)> {
        self.touched.insert(key);
        let slot = self.entries.get(&key)?;
        if slot.generation == self.generation && slot.fingerprint == fingerprint {
            return Some((slot.rect_count, slot.data.as_ref()));
        }
        None
    }

    /// Records (or replaces) a boundary's painted output under the current
    /// generation.
    pub(crate) fn put(
        &mut self,
        key: WidgetId,
        fingerprint: u64,
        rect_count: usize,
        data: Box<dyn Any>,
    ) {
        self.entries.insert(
            key,
            Slot {
                generation: self.generation,
                fingerprint,
                rect_count,
                data,
            },
        );
    }

    /// A diagnostic counter: one boundary reused this frame.
    pub(crate) fn note_hit(&mut self) {
        self.hits += 1;
    }

    /// A diagnostic counter: one boundary repainted this frame (a miss, or
    /// non-cacheable).
    pub(crate) fn note_miss(&mut self) {
        self.misses += 1;
    }

    /// To be called at the end of a frame: forgets untouched boundaries (vanished
    /// widgets) and freezes the frame's diagnostic counters.
    pub(crate) fn end_frame(&mut self) {
        let touched = std::mem::take(&mut self.touched);
        self.entries.retain(|id, _| touched.contains(id));
        self.last_hits = self.hits;
        self.last_misses = self.misses;
        self.hits = 0;
        self.misses = 0;
    }

    /// Reuses and repaints of the last completed frame (diagnostic).
    pub fn last_frame_stats(&self) -> (u32, u32) {
        (self.last_hits, self.last_misses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: WidgetId = WidgetId::ROOT;

    #[test]
    fn stored_entry_is_a_hit_until_generation_bumps() {
        let mut c = PaintCache::default();
        c.put(A, 42, 3, Box::new(7u32));
        // Current generation + equal fingerprint → a hit, with the rect_count.
        let (rc, any) = c.get(A, 42).expect("hit");
        assert_eq!(rc, 3);
        assert_eq!(*any.downcast_ref::<u32>().unwrap(), 7);
        // A different fingerprint → no hit.
        assert!(c.get(A, 99).is_none());
        // A stale generation → the entry is no longer a hit.
        c.bump_generation();
        assert!(c.get(A, 42).is_none());
    }

    #[test]
    fn end_frame_evicts_untouched_boundaries() {
        let mut c = PaintCache::default();
        c.put(A, 1, 1, Box::new(()));
        c.put(A.child(0), 1, 1, Box::new(()));
        // `put` alone does not mark "touched"; simulate a frame where only A is seen.
        c.get(A, 1);
        c.end_frame();
        assert!(c.entries.contains_key(&A));
        assert!(
            !c.entries.contains_key(&A.child(0)),
            "vanished boundary evicted"
        );
    }

    #[test]
    fn frame_stats_freeze_at_end_frame() {
        let mut c = PaintCache::default();
        c.note_hit();
        c.note_hit();
        c.note_miss();
        c.end_frame();
        assert_eq!(c.last_frame_stats(), (2, 1));
        // The counters start again from zero for the next frame.
        c.end_frame();
        assert_eq!(c.last_frame_stats(), (0, 0));
    }
}
