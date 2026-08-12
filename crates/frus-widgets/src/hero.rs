//! [`Hero`]: a **shared element** between two screens — the thing that appears to fly
//! from where it was to where it is going, instead of one copy fading out while
//! another fades in.
//!
//! The whole difficulty is identity. The two screens are separate trees, built
//! independently, and nothing in a widget tree says "this picture is the same picture
//! as that one". A `Hero` says exactly that, and nothing else: it carries a **tag**,
//! and two heroes with the same tag on the two sides of a route transition are
//! understood to be one thing in two places.
//!
//! ```ignore
//! // In the list:
//! Hero::new(task.id, Avatar::new(task.text.clone()).size(30.0))
//! // On the detail screen:
//! Hero::new(task.id, Avatar::new(task.text.clone()).size(96.0))
//! ```
//!
//! What flies is the **destination**'s own painting, lifted out of the frame and
//! mapped onto the box it is travelling through — never a third widget built for the
//! occasion, which would be a second definition of the same thing. Both originals are
//! taken out of the frame for as long as the flight lasts, since a thing cannot be in
//! three places at once.
//!
//! A hero with no counterpart on the other side simply renders: there is nothing to
//! fly to, and pretending otherwise would be worse than a plain transition.

use std::hash::{Hash, Hasher};

use frus_core::{Rect, Scene};
use frus_layout::Style;

use crate::interaction::{Status, WidgetId};
use crate::theme::Theme;
use crate::widget::{sizing_of, Widget};

/// A hero seen by the render driver: which tag, which screen of the transition it
/// belongs to, and where it sits.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HeroSpot {
    /// The shared tag.
    pub tag: u64,
    /// `0` = the screen being left, `1` = the one being entered. Outside a transition
    /// there is no flight and this is meaningless.
    pub screen: u8,
    pub id: WidgetId,
    /// Where it sits, in absolute coordinates.
    pub rect: Rect,
}

/// Interpolates a rectangle towards another, corner by corner.
///
/// A straight line between the two boxes. Platforms that curve the path do it to make
/// a diagonal flight read as one motion rather than two; that is a refinement on top
/// of this, not a different mechanism.
pub fn lerp_rect(from: Rect, to: Rect, t: f32) -> Rect {
    let mix = |a: f32, b: f32| a + (b - a) * t;
    Rect::new(
        mix(from.x, to.x),
        mix(from.y, to.y),
        mix(from.width, to.width),
        mix(from.height, to.height),
    )
}

/// Marks its child as **the same element** as the hero carrying the same tag on the
/// other side of a route transition.
pub struct Hero<Msg> {
    tag: u64,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Hero<Msg> {
    /// Tags `child` as a shared element. The tag may be any hashable value — a domain
    /// id is the usual one, since it is already the thing that says "the same".
    pub fn new(tag: impl Hash, child: impl Widget<Msg> + 'static) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tag.hash(&mut hasher);
        Self {
            tag: hasher.finish(),
            children: vec![Box::new(child)],
        }
    }

    /// The resolved tag.
    pub fn tag(&self) -> u64 {
        self.tag
    }
}

impl<Msg: Clone> Widget<Msg> for Hero<Msg> {
    fn style(&self) -> Style {
        sizing_of(
            self.children
                .first()
                .map(|child| child.style())
                .unwrap_or_default(),
        )
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    // Forwarded, not answered: see `sizing_of` and milestone 285. A wrapper that said
    // "not a stack" over a layout leaf would collapse it to nothing.
    fn stack(&self) -> bool {
        self.children.first().is_some_and(|child| child.stack())
    }

    fn continuous(&self) -> bool {
        self.children.first().is_some_and(|child| child.continuous())
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {
        // Nothing of its own: the child is the element.
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn hero_tag(&self) -> Option<u64> {
        Some(self.tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Navigator, Runtime};
    use frus_core::{Color, Primitive, Size};

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// A transition at `progress`, with a small hero on the outgoing screen and a big
    /// one on the incoming screen.
    fn flight(progress: f32, tags: (u64, u64)) -> Vec<Rect> {
        let leaving = Container::<()>::new().width(400.0).height(300.0).child(
            Hero::new(tags.0, Container::new().width(40.0).height(40.0).color(RED)),
        );
        let entering = Container::<()>::new().width(400.0).height(300.0).child(
            Hero::new(tags.1, Container::new().width(200.0).height(200.0).color(RED)),
        );
        let navigator = Navigator::new(entering, 400.0, 300.0).from(leaving, progress, true);
        let runtime = Runtime::default();
        let ui = build_ui(&navigator, Size::new(400.0, 300.0), &runtime, &Theme::dark());
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, color, .. } if color.r > 0.5 && color.g < 0.5 => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_matched_pair_flies_as_one_thing() {
        // Mid-flight there is **one** red box, not two: both originals are out of the
        // frame and a single copy is travelling.
        let mid = flight(0.5, (7, 7));
        assert_eq!(mid.len(), 1, "one thing in one place: {mid:?}");
        let size = mid[0].width;
        assert!(
            size > 45.0 && size < 195.0,
            "between the two sizes, not at either: {size}"
        );
    }

    #[test]
    fn the_flight_starts_where_it_was_and_ends_where_it_is_going() {
        let start = flight(0.0, (7, 7));
        assert_eq!(start.len(), 1);
        assert!(
            (start[0].width - 40.0).abs() < 1.0,
            "starts at the source's size: {:?}",
            start[0]
        );
        let end = flight(1.0, (7, 7));
        assert_eq!(end.len(), 1);
        assert!(
            (end[0].width - 200.0).abs() < 1.0,
            "ends at the destination's size: {:?}",
            end[0]
        );
    }

    #[test]
    fn heroes_that_do_not_match_are_left_alone() {
        // Different tags: no shared element, so both screens keep their own — the
        // ordinary transition, which is the right answer and not a silent nothing.
        let mid = flight(0.5, (7, 8));
        assert_eq!(mid.len(), 2, "both drawn as usual: {mid:?}");
    }

    #[test]
    fn a_tag_is_whatever_says_the_same() {
        let by_number = Hero::<()>::new(42u64, Container::new());
        let same = Hero::<()>::new(42u64, Container::new());
        let other = Hero::<()>::new(43u64, Container::new());
        assert_eq!(by_number.tag(), same.tag());
        assert_ne!(by_number.tag(), other.tag());
        // Strings work as well, and are their own kind of identity.
        let by_name = Hero::<()>::new("avatar", Container::new());
        assert_eq!(by_name.tag(), Hero::<()>::new("avatar", Container::new()).tag());
    }
}
