//! [`Rating`]: a **controlled** rating in **clickable stars**.

use frus_core::{Point, Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::disabled::{disabled_container, disabled_content};
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const STAR: f32 = 24.0;

/// One star, filled or empty, and clickable.
struct Star<Msg> {
    filled: bool,
    /// The rating's availability, handed down. A star that still answered under a
    /// disabled rating would be the whole control, a rating being only its stars.
    enabled: bool,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Star<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(STAR),
            height: Dimension::Length(STAR),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        // A disabled rating must still show its score. Flattening both to the same grey
        // was the first attempt and the golden refused it: five identical stars say
        // nothing, where the whole of what a rating carries is how many are lit. So the
        // two halves of the rule do the work - a lit star is the **mark** at 38 %, an
        // unlit one the **container** it sits in at 12 %.
        let base = if !self.enabled {
            if self.filled {
                disabled_content(theme)
            } else {
                disabled_container(theme)
            }
        } else if self.filled {
            theme.primary
        } else {
            theme.muted.fade(0.45)
        };
        // A slight lightening on hover, and none at all when there is nothing to press.
        let hover = if self.enabled {
            status.hover_progress
        } else {
            0.0
        };
        let color = base.lerp(theme.on_surface, 0.2 * hover);
        scene.text(
            Point::new(bounds.x, bounds.y - 2.0),
            "★".to_string(),
            STAR,
            color.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        if !self.enabled {
            return None;
        }
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        self.enabled
    }
}

/// A star rating: `value` out of `max`.
pub struct Rating<Msg> {
    value: u32,
    max: u32,
    enabled: bool,
    on_rate: Box<dyn Fn(u32) -> Msg>,
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Rating<Msg> {
    /// Creates a rating: `value` filled stars out of `max`. Clicking the i-th star
    /// emits `on_rate(i + 1)`.
    pub fn new(value: u32, max: u32, on_rate: impl Fn(u32) -> Msg + 'static) -> Self {
        let mut rating = Self {
            value,
            max,
            enabled: true,
            on_rate: Box::new(on_rate),
            children: Vec::new(),
        };
        rating.rebuild();
        rating
    }

    /// Whether the rating can be changed. Disabled it is **inert** - no star takes a
    /// press or the focus - and it still shows the score, because read-only is not
    /// invisible.
    ///
    /// See [`crate::disabled`] for the whole contract.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.rebuild();
        self
    }

    /// Rebuilds the stars, so that the order of the builders does not change what comes
    /// out - the trap `RadioGroup` fell into in milestone 322.
    fn rebuild(&mut self) {
        self.children = (0..self.max)
            .map(|i| {
                Box::new(Star {
                    filled: i < self.value,
                    enabled: self.enabled,
                    message: (self.on_rate)(i + 1),
                }) as Box<dyn Widget<Msg>>
            })
            .collect();
    }
}

impl<Msg: Clone> Widget<Msg> for Rating<Msg> {
    fn style(&self) -> Style {
        Style {
            flex_direction: FlexDirection::Row,
            gap: 4.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::Semantics> {
        // A rating said nothing to a reader before this: the score is the whole of what it
        // carries, and it is still owed to someone who cannot change it.
        let semantics = frus_core::Semantics::new(frus_core::Role::Slider)
            .value(format!("{} / {}", self.value, self.max))
            .range(0.0, self.value as f32, self.max as f32);
        Some(if self.enabled {
            semantics
        } else {
            semantics.disabled(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Rate(u32),
    }

    #[test]
    fn has_max_stars_and_click_emits_rank() {
        let rating = Rating::new(2, 5, Msg::Rate);
        let stars = Widget::<Msg>::children(&rating);
        assert_eq!(stars.len(), 5);
        // Clicking the fourth star, index 3, gives a rating of 4.
        assert_eq!(stars[3].on_click(), Some(Msg::Rate(4)));
    }

    #[test]
    fn filled_stars_match_value() {
        // Full and empty stars are told apart by the painted color.
        let rating = Rating::new(3, 5, Msg::Rate);
        let count_color = |filled_expected: bool| {
            let theme = Theme::default();
            let target = if filled_expected {
                theme.primary
            } else {
                theme.muted.fade(0.45)
            };
            (0..5)
                .filter(|&i| {
                    let mut scene = Scene::new();
                    Widget::<Msg>::children(&rating)[i].paint(
                        Rect::new(0.0, 0.0, STAR, STAR),
                        Status::default(),
                        &theme,
                        &mut scene,
                    );
                    matches!(scene.primitives()[0], frus_core::Primitive::Text { color, .. } if color == target)
                })
                .count()
        };
        assert_eq!(count_color(true), 3); // three filled
        assert_eq!(count_color(false), 2); // two empty
    }

    #[test]
    fn a_disabled_rating_is_inert_but_still_shows_the_score() {
        let dead = Rating::new(3, 5, Msg::Rate).enabled(false);
        for (i, star) in Widget::<Msg>::children(&dead).iter().enumerate() {
            assert_eq!(star.on_click(), None, "star {i} still answers");
            assert!(!star.focusable(), "star {i} is still in the tab order");
        }
        let semantics = Widget::<Msg>::semantics(&dead).expect("still announced");
        assert!(semantics.disabled, "announced as unavailable");
        assert_eq!(semantics.value.as_deref(), Some("3 / 5"), "score survives");
    }

    #[test]
    fn a_disabled_rating_flattens_rather_than_fading_the_accent() {
        // Filled and empty alike go to the content grey: a star is a mark, and a pale
        // accent would read as a quieter score rather than as an unavailable one.
        let theme = Theme::default();
        let dead = Rating::new(3, 5, Msg::Rate).enabled(false);
        let lit = |i: usize| {
            let mut scene = Scene::new();
            Widget::<Msg>::children(&dead)[i].paint(
                Rect::new(0.0, 0.0, STAR, STAR),
                Status::default(),
                &theme,
                &mut scene,
            );
            match scene.primitives()[0] {
                frus_core::Primitive::Text { color, .. } => color,
                _ => panic!("a star is text"),
            }
        };
        for i in 0..5 {
            assert_ne!(lit(i), theme.primary, "star {i} keeps the accent");
        }
        // And the score is still legible: three lit stars, two not.
        for i in 0..3 {
            assert_eq!(lit(i), disabled_content(&theme), "star {i} should be lit");
        }
        for i in 3..5 {
            assert_eq!(
                lit(i),
                disabled_container(&theme),
                "star {i} should be dark"
            );
        }
        assert!(
            lit(0).a > lit(4).a,
            "and a lit star reads louder than an unlit one"
        );
    }
}
