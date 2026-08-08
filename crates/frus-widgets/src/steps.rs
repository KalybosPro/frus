//! [`Steps`]: a **step indicator** — the numbered breadcrumb of a multi-step form
//! or wizard, in the Material `Stepper` style.
//!
//! A row of round numbered markers joined by connectors, each in one of three
//! states: **done** (a tick, accent), **current** (the number, accent), **upcoming**
//! (the number, a bordered surface). A label sits under each marker.
//!
//! The widget is **purely visual**: navigation (Next/Previous) and per-step
//! validation stay in the application (one [`crate::form::Form`] per step, buttons
//! that change the current step). Since the name `Stepper` is already taken by the
//! −/value/+ numeric picker, this indicator is called `Steps`.

use frus_core::{Color, Point, Rect, Role, Scene, Semantics};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::flex::Flex;
use crate::icons::IconName;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// The diameter of a (round) marker.
const MARKER_D: f32 = 28.0;
/// The radius of a marker.
const R: f32 = MARKER_D / 2.0;
/// The marker → label gap.
const LABEL_GAP: f32 = 8.0;
/// Font size of a step label.
const LABEL_SIZE: f32 = 12.0;
/// Font size of the number inside a marker.
const NUM_SIZE: f32 = 14.0;
/// Total height of the indicator (marker + label).
const HEIGHT: f32 = 56.0;

/// The step indicator of a multi-step form.
///
/// ```
/// use frus_widgets::Steps;
/// // Three steps, the second in progress (so the first counts as "done").
/// let steps: Steps<()> = Steps::new(["Account", "Profile", "Review"]).current(1);
/// ```
pub struct Steps<Msg> {
    labels: Vec<String>,
    current: usize,
    /// Overridden accent color; `None` = the theme's `primary`.
    color: Option<Color>,
    /// An **explicit** per-step "done" mask (validity). Empty → the default rule
    /// (`i < current`, see [`completed`](Self::completed)).
    completed: Vec<bool>,
    /// Empty, or **one** row of clickable hotspots laid over the markers when
    /// [`on_tap`](Self::on_tap) is supplied.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Steps<Msg> {
    /// Creates an indicator from the step labels; the current step is the first one.
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            current: 0,
            color: None,
            completed: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Explicitly marks the **done** steps (a tick) with one flag per step — typically each
    /// step's **validity**, rather than position alone. Without this call, a step counts as
    /// "done" if it precedes the current one (`i < current`).
    pub fn completed(mut self, flags: impl IntoIterator<Item = bool>) -> Self {
        self.completed = flags.into_iter().collect();
        self
    }

    /// Sets the **current** step (the earlier ones are "done", the later ones "upcoming").
    /// Clamped to the last index.
    pub fn current(mut self, index: usize) -> Self {
        self.current = index.min(self.labels.len().saturating_sub(1));
        self
    }

    /// Overrides the accent color (done and current markers + the connectors crossed).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Makes the markers **clickable**: clicking step `i`'s marker emits `on_tap(i)` (to jump
    /// there — typically a step already visited). Lays a row of transparent clickable zones
    /// **exactly** over the markers (a `SpaceBetween` distribution of marker-sized boxes),
    /// without changing the rendering.
    pub fn on_tap(mut self, on_tap: impl Fn(usize) -> Msg) -> Self {
        let mut row: Flex<Msg> = Flex::row().justify(Justify::SpaceBetween);
        for (i, label) in self.labels.iter().enumerate() {
            row = row.child(Hotspot {
                label: label.clone(),
                message: on_tap(i),
            });
        }
        self.children = vec![Box::new(row)];
        self
    }
}

impl<Msg> Steps<Msg> {
    /// Is step `i` **done**? The explicit mask if one is supplied, otherwise `i < current`.
    fn is_done(&self, i: usize) -> bool {
        if self.completed.is_empty() {
            i < self.current
        } else {
            self.completed.get(i).copied().unwrap_or(false)
        }
    }

    /// The x of marker `i`'s centre within `bounds` (markers spread from edge to edge).
    fn center_x(&self, bounds: Rect, i: usize) -> f32 {
        let n = self.labels.len();
        if n <= 1 {
            bounds.x + bounds.width * 0.5
        } else {
            bounds.x + R + i as f32 * (bounds.width - 2.0 * R) / (n as f32 - 1.0)
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Steps<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Length(HEIGHT),
            // Any row of hotspots occupies the top, the markers' band.
            flex_direction: FlexDirection::Column,
            align: Align::Stretch,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let n = self.labels.len();
        if n == 0 {
            return;
        }
        let o = status.opacity;
        let accent = self.color.unwrap_or(theme.primary);
        let cy = bounds.y + R;

        // Connectors (under the markers): crossed (accent) up to the current step, else border.
        for i in 0..n.saturating_sub(1) {
            let x0 = self.center_x(bounds, i) + R;
            let x1 = self.center_x(bounds, i + 1) - R;
            let col = if self.is_done(i) {
                accent
            } else {
                theme.border
            };
            let rect = Rect::new(x0, cy - 1.0, (x1 - x0).max(0.0), 2.0);
            scene.draw_rect(rect, col.fade(o), 0.0, 0.0, Color::TRANSPARENT);
        }

        // Markers + numbers/ticks + labels.
        for i in 0..n {
            let cx = self.center_x(bounds, i);
            let rect = Rect::new(cx - R, cy - R, MARKER_D, MARKER_D);
            let current = i == self.current;
            // The current step shows its number (even when valid); the others show a tick
            // if done (validity), otherwise their number.
            let completed = !current && self.is_done(i);

            let (fill, bw, bc) = if completed || current {
                (accent, 0.0, Color::TRANSPARENT)
            } else {
                (theme.surface, 1.5, theme.border)
            };
            scene.draw_rect(rect, fill.fade(o), R, bw, bc.fade(o));

            if completed {
                // A tick (a centred 16 px icon) on an accent background.
                let path = IconName::Check
                    .path()
                    .scaled(16.0 / 24.0)
                    .translated(cx - 8.0, cy - 8.0);
                scene.fill_path(&path, theme.on_primary.fade(o));
            } else {
                let num = (i + 1).to_string();
                let m = frus_text::measure(&num, NUM_SIZE);
                let color = if current {
                    theme.on_primary
                } else {
                    theme.on_surface
                };
                let p = Point::new(cx - m.width * 0.5, cy - m.height * 0.5);
                scene.text(p, num, NUM_SIZE, color.fade(o));
            }

            // The label under the marker, centred; dimmed away from the current step.
            let label = &self.labels[i];
            let lm = frus_text::measure(label, LABEL_SIZE);
            let alpha = if current { o } else { 0.6 * o };
            let p = Point::new(cx - lm.width * 0.5, bounds.y + MARKER_D + LABEL_GAP);
            scene.text(p, label.clone(), LABEL_SIZE, theme.on_surface.fade(alpha));
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// A **transparent** clickable zone the size of a marker, laid over it when
/// [`Steps::on_tap`] is used: it draws nothing but captures the click (and the keyboard
/// focus) to jump to the matching step.
struct Hotspot<Msg> {
    label: String,
    message: Msg,
}

impl<Msg: Clone> Widget<Msg> for Hotspot<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Length(MARKER_D),
            height: Dimension::Length(MARKER_D),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        Some(self.message.clone())
    }

    fn focusable(&self) -> bool {
        true
    }

    fn semantics(&self) -> Option<Semantics> {
        Some(
            Semantics::new(Role::Button)
                .label(self.label.clone())
                .clickable(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn paint_steps(steps: &Steps<()>) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            steps,
            Rect::new(0.0, 0.0, 400.0, HEIGHT),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn current_is_clamped_to_last() {
        let steps = Steps::<()>::new(["A", "B", "C"]).current(9);
        assert_eq!(steps.current, 2);
        assert_eq!(Steps::<()>::new(Vec::<String>::new()).current(3).current, 0);
    }

    #[test]
    fn markers_reflect_progress() {
        // 4 steps, the 3rd (index 2) current: 0 and 1 done; 2 current; 3 upcoming.
        let prims = paint_steps(&Steps::<()>::new(["A", "B", "C", "D"]).current(2));
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Done → ticks (no number); current → "3"; upcoming → "4".
        assert!(
            has_text("3") && has_text("4"),
            "the numbers of the current and upcoming steps"
        );
        assert!(
            !has_text("1") && !has_text("2"),
            "the done steps show a tick"
        );
        // One tick (a filled path) per done step.
        let checks = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Path { fill: Some(_), .. }))
            .count();
        assert_eq!(checks, 2, "two ticks for the two done steps");
        // All the labels are drawn.
        assert!(has_text("A") && has_text("B") && has_text("C") && has_text("D"));
    }

    #[test]
    fn completed_mask_overrides_position() {
        // Without a mask: "done" = position (i < current).
        let default = Steps::<()>::new(["A", "B", "C"]).current(2);
        assert!(default.is_done(0) && default.is_done(1));
        assert!(
            !default.is_done(2),
            "the current step is not done by default"
        );
        // With a mask (validity): independent of position.
        let masked = Steps::<()>::new(["A", "B", "C"])
            .current(1)
            .completed([false, false, true]);
        assert!(
            !masked.is_done(0),
            "step 0 invalid → not done despite i < current"
        );
        assert!(
            masked.is_done(2),
            "step 2 valid → done even though i > current"
        );
        // A mask shorter than the number of steps: the missing ones are not done.
        let short = Steps::<()>::new(["A", "B", "C"]).completed([true]);
        assert!(short.is_done(0) && !short.is_done(1) && !short.is_done(2));
    }

    #[test]
    fn on_tap_overlays_clickable_hotspots() {
        #[derive(Clone, Debug, PartialEq)]
        enum Msg {
            Go(usize),
        }
        // Without on_tap: no children (purely visual).
        let plain = Steps::<Msg>::new(["A", "B", "C"]).current(1);
        assert!(Widget::<Msg>::children(&plain).is_empty());
        // With on_tap: a row of children whose every marker emits its index.
        let tappable = Steps::new(["A", "B", "C"]).current(1).on_tap(Msg::Go);
        let row = Widget::<Msg>::children(&tappable);
        assert_eq!(row.len(), 1, "a single row of hotspots");
        let spots = row[0].children();
        assert_eq!(spots.len(), 3, "one hotspot per step");
        assert_eq!(spots[0].on_click(), Some(Msg::Go(0)));
        assert_eq!(spots[2].on_click(), Some(Msg::Go(2)));
        assert!(spots[0].focusable(), "a hotspot is focusable");
    }
}
