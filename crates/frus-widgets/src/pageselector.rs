//! [`TabPageSelector`]: the row of dots that says which page of several you are on.
//!
//! A module of its own rather than a corner of the tab bar's, because it pairs with
//! [`PageView`](crate::PageView) as much as with a tab bar — an onboarding walkthrough, a
//! gallery, a carousel — and because a tab bar's own widgets are **disableable** and this
//! is not. A read-out has no disabled state; it says where you are or it is not there.

use frus_core::{BorderRadius, Color, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// A dot's diameter, and the room on either side of it (`tab_page_selector.dart:24`).
pub const PAGE_DOT_SIZE: f32 = 12.0;
/// The gap between two dots.
pub const PAGE_DOT_GAP: f32 = 8.0;

/// **The row of dots that says which page of several you are on.**
///
/// It pairs with [`PageView`](crate::PageView) as much as with a [`TabBar`](crate::TabBar): an onboarding
/// walkthrough, a gallery, a carousel — anything with a handful of pages and no room for
/// a bar. It is a **read-out**, not a control: it says where you are and does not take a
/// press, because a four-pixel target is not one.
///
/// The dot for the page you are on is filled and the rest are rings, and the fill
/// **crosses** rather than jumping — driven by the same fractional index the tab bar's
/// indicator slides along, so a selector under a page view and a bar over it agree on
/// where the crossing has got to.
///
/// ```
/// use frus_widgets::TabPageSelector;
///
/// let dots = TabPageSelector::<()>::new(4, 1);
/// ```
pub struct TabPageSelector<Msg> {
    count: usize,
    selected: usize,
    size: Option<f32>,
    gap: Option<f32>,
    color: Option<Color>,
    border_color: Option<Color>,
    marker: std::marker::PhantomData<fn() -> Msg>,
}

impl<Msg> TabPageSelector<Msg> {
    /// A row of `count` dots with the `selected`-th filled.
    pub fn new(count: usize, selected: usize) -> Self {
        Self {
            count,
            selected,
            size: None,
            gap: None,
            color: None,
            border_color: None,
            marker: std::marker::PhantomData,
        }
    }

    /// A dot's diameter, over the theme's and the reference's twelve.
    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// The gap between two dots.
    #[must_use]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// What the dot you are on is filled with. Unset, the accent.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The ring every dot is drawn with.
    #[must_use]
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    fn dot(&self, theme: Option<&Theme>) -> f32 {
        self.size
            .or(theme.and_then(|t| t.widgets.page_selector.size))
            .unwrap_or(PAGE_DOT_SIZE)
    }

    fn spacing(&self, theme: Option<&Theme>) -> f32 {
        self.gap
            .or(theme.and_then(|t| t.widgets.page_selector.gap))
            .unwrap_or(PAGE_DOT_GAP)
    }

    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let dot = self.dot(theme);
        let count = self.count as f32;
        Style {
            width: Dimension::Length(match self.count {
                0 => 0.0,
                _ => count * dot + (count - 1.0) * self.spacing(theme),
            }),
            height: Dimension::Length(dot),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for TabPageSelector<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    /// The page you are on, so that the runtime hands it back as a **fractional** index
    /// while the selection moves — the same value the bar's indicator slides along.
    fn anim_target(&self) -> Option<f32> {
        Some(self.selected as f32)
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        if self.count == 0 {
            return;
        }
        let o = status.opacity;
        let dot = self.dot(Some(theme));
        let gap = self.spacing(Some(theme));
        let fill = self
            .color
            .or(theme.widgets.page_selector.color)
            .unwrap_or(theme.scheme.primary);
        let ring = self
            .border_color
            .or(theme.widgets.page_selector.border_color)
            .unwrap_or(theme.border);
        // Where the crossing has got to, in whole and fractional pages.
        let at = status.value.clamp(0.0, (self.count - 1) as f32);
        let y = bounds.y + (bounds.height - dot) * 0.5;
        for index in 0..self.count {
            let x = bounds.x + index as f32 * (dot + gap);
            let box_ = Rect::new(x, y, dot, dot);
            let radius = BorderRadius::uniform(dot * 0.5);
            // **How much of this dot is filled**: all of it on the page you are on, none
            // two pages away, and the two either side of a crossing sharing it between
            // them. Fading one out while the other fades in is what the reference does,
            // and it is the only version where the row never reads as two filled dots or
            // as none.
            let share = (1.0 - (index as f32 - at).abs()).clamp(0.0, 1.0);
            scene.draw_rect(box_, Color::TRANSPARENT, radius, 1.0, ring.fade(o));
            if share > 0.0 {
                // The fill is **faded**, not lerped towards the ring's colour: a dot on a
                // dark surface and one on a light surface would need opposite lerps, and
                // an alpha is the same answer on both.
                scene.draw_rect(box_, fill.fade(o * share), radius, 0.0, Color::TRANSPARENT);
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }

    fn semantics(&self) -> Option<frus_core::SemanticsProperties> {
        // A row of dots says nothing at all to a reader who cannot see it, and *page two
        // of four* is exactly what it is drawing. The reference announces nothing here;
        // this costs one line and is the whole of the widget's meaning.
        Some(
            frus_core::SemanticsProperties::new(frus_core::Role::ProgressBar).value(format!(
                "{} / {}",
                (self.selected + 1).min(self.count),
                self.count
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;
    use crate::ui::build_ui;
    use frus_core::Size;

    /// The dots send nothing, so there is no message type to name.
    type Msg = ();

    /// **A row of dots says which page of several you are on**, and the fill **crosses**
    /// rather than jumping: two dots either side of a crossing share it between them,
    /// which is the only version where the row never reads as two filled dots or as none.
    #[test]
    fn the_dots_share_the_fill_across_a_crossing() {
        let filled = |value: f32| {
            let mut runtime = Runtime::default();
            runtime.set_value(crate::interaction::WidgetId::ROOT, value);
            let ui = build_ui(
                &TabPageSelector::<Msg>::new(4, value.round() as usize),
                Size::new(200.0, 40.0),
                &runtime,
                &Theme::default(),
            );
            let primary = Theme::default().scheme.primary;
            // Matched on the **colour**, not on the colour and its alpha together: the
            // alpha is the answer being measured, so folding it into the filter would
            // find only the dots that are completely filled.
            rects(ui.scene())
                .into_iter()
                .filter(|(_, c)| (c.r, c.g, c.b) == (primary.r, primary.g, primary.b) && c.a > 0.0)
                .map(|(_, color)| color.a)
                .collect::<Vec<_>>()
        };
        assert_eq!(filled(0.0).len(), 1, "at rest, one dot is filled");
        let crossing = filled(0.5);
        assert_eq!(crossing.len(), 2, "half way across, two are");
        assert!(
            (crossing[0] + crossing[1] - 1.0).abs() < 1e-3,
            "and they share exactly one dot's worth between them: {crossing:?}"
        );
    }

    /// **The dots are a read-out, not a control.** A twelve-pixel target is not one, and a
    /// row that took presses would be a row of controls a finger cannot tell apart.
    ///
    /// They are announced, which the reference's own does not do: *page two of four* is
    /// exactly what the widget is drawing, and it is the whole of its meaning to anyone
    /// who cannot see it.
    #[test]
    fn the_dots_are_a_read_out_and_are_announced() {
        let dots = TabPageSelector::<Msg>::new(4, 1);
        assert_eq!(Widget::<Msg>::on_click(&dots), None);
        assert!(!Widget::<Msg>::focusable(&dots));
        assert_eq!(
            Widget::<Msg>::semantics(&dots)
                .expect("announced")
                .value
                .as_deref(),
            Some("2 / 4")
        );
    }

    /// Every filled rectangle in the frame, innermost layers included.
    fn rects(scene: &frus_core::Scene) -> Vec<(Rect, Color)> {
        fn walk(primitives: &[frus_core::Primitive], out: &mut Vec<(Rect, Color)>) {
            for p in primitives {
                match p {
                    frus_core::Primitive::Rect { rect, color, .. } => out.push((*rect, *color)),
                    frus_core::Primitive::Layer { primitives, .. } => walk(primitives, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(scene.primitives(), &mut out);
        out
    }
}
