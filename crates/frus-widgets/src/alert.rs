//! [`AlertDialog`]: a **persistent** (contextual) message box, as opposed to the
//! transient [`crate::SnackBar`].

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD: f32 = 12.0;
const ACCENT: f32 = 4.0;
const ICON_W: f32 = 26.0;
const TITLE_SIZE: f32 = 16.0;
const TEXT_SIZE: f32 = 15.0;

/// The title's style, **resolved once**: the same number measures the box and draws the
/// glyphs. Resolving is where the reader's font setting is applied, so measuring at the
/// bare constant and painting through a style is a layout that disagrees with itself.
fn title_style() -> ResolvedTextStyle {
    TextStyle::new(TITLE_SIZE).resolved()
}

/// The message's style. See [`title_style`].
fn body_style() -> ResolvedTextStyle {
    TextStyle::new(TEXT_SIZE).resolved()
}

/// The nature of an alert box.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Info,
    Success,
    Warning,
    Error,
}

/// A message box.
pub struct AlertDialog {
    title: Option<String>,
    text: String,
    kind: AlertKind,
}

impl AlertDialog {
    /// Creates an informational box.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            title: None,
            text: text.into(),
            kind: AlertKind::Info,
        }
    }

    /// Adds a title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The success variant.
    pub fn success(mut self) -> Self {
        self.kind = AlertKind::Success;
        self
    }

    /// Variante avertissement.
    pub fn warning(mut self) -> Self {
        self.kind = AlertKind::Warning;
        self
    }

    /// Variante erreur.
    pub fn error(mut self) -> Self {
        self.kind = AlertKind::Error;
        self
    }

    fn accent(&self, theme: &Theme) -> Color {
        match self.kind {
            AlertKind::Info => theme.primary,
            AlertKind::Success => Color::rgb8(70, 190, 120),
            AlertKind::Warning => Color::rgb8(230, 170, 40),
            AlertKind::Error => Color::rgb8(210, 96, 96),
        }
    }

    fn icon(&self) -> &'static str {
        match self.kind {
            AlertKind::Info => "i",
            AlertKind::Success => "✓",
            AlertKind::Warning => "!",
            AlertKind::Error => "×",
        }
    }
}

impl<Msg> Widget<Msg> for AlertDialog {
    fn style(&self) -> Style {
        // A paragraph: free dimensions, the size comes from `measure()` — the
        // message wraps to the width the parent offers.
        Style::default()
    }

    fn measure(&self, _theme: &Theme) -> Option<frus_layout::MeasureFn<'_>> {
        let text = self.text.clone();
        let title = self.title.clone();
        Some(Box::new(move |max_width, _| {
            let (title_s, body_s) = (title_style(), body_style());
            let chrome = ACCENT + ICON_W + PAD; // bar + icon + right margin
            let text_avail = max_width.map(|w| (w - chrome).max(40.0));
            let body = frus_text::measure_wrapped_resolved(&text, &body_s, text_avail);
            let title_h = title
                .as_ref()
                .map(|_| title_s.line_height() + 4.0)
                .unwrap_or(0.0);
            let title_w = title
                .as_ref()
                .map(|t| frus_text::measure_resolved(t, &title_s).width)
                .unwrap_or(0.0);
            let natural_w = chrome + body.width.max(title_w);
            frus_core::Size::new(
                max_width.map_or(natural_w, |w| w.min(natural_w)).ceil(),
                (PAD * 2.0 + title_h + body.height).ceil(),
            )
        }))
    }

    fn measure_key(&self, _theme: &Theme) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.text.hash(&mut hasher);
        self.title.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let o = status.opacity;
        let accent = self.accent(theme);
        // Tinted background + a discreet border.
        scene.draw_rect(
            bounds,
            accent.fade(0.12 * o),
            theme.radius,
            1.0,
            accent.fade(0.4 * o),
        );
        // Accent bar on the left.
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y, ACCENT, bounds.height),
            accent.fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        // The glyph standing in for an icon. `exact`, not resolved: it sits in a column
        // of a fixed `ICON_W`, and a reader who doubles the type would push it out.
        scene.text(
            Point::new(bounds.x + ACCENT + 7.0, bounds.y + PAD),
            self.icon().to_string(),
            &ResolvedTextStyle::exact(TITLE_SIZE),
            accent.fade(o),
        );
        let text_x = bounds.x + ACCENT + ICON_W;
        // The message wraps to the width actually available (the laid-out one),
        // to stay consistent with `measure()`.
        let wrap_w = (bounds.width - (ACCENT + ICON_W + PAD)).max(40.0);
        let (title_s, body_s) = (title_style(), body_style());
        match &self.title {
            Some(title) => {
                scene.text(
                    Point::new(text_x, bounds.y + PAD),
                    title.clone(),
                    &title_s,
                    theme.on_surface.fade(o),
                );
                scene.text_wrapped(
                    Point::new(text_x, bounds.y + PAD + title_s.line_height() + 4.0),
                    self.text.clone(),
                    &body_s,
                    theme.muted.fade(o),
                    wrap_w,
                );
            }
            None => {
                scene.text_wrapped(
                    Point::new(text_x, bounds.y + PAD),
                    self.text.clone(),
                    &body_s,
                    theme.on_surface.fade(o),
                    wrap_w,
                );
            }
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    #[test]
    fn paints_accent_and_text() {
        let alert = AlertDialog::new("Attention !").title("Alerte").warning();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &alert,
            Rect::new(0.0, 0.0, 240.0, 60.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let warn = Color::rgb8(230, 170, 40);
        // The accent bar (the variant's solid color) is present.
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == warn)));
        // Title + text are painted.
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Alerte")));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Attention !")));
    }

    /// The message **wraps** to the offered width: narrower → taller (and never
    /// wider than the offer) — no more box overflowing its parent.
    #[test]
    fn message_wraps_to_the_offered_width() {
        let alert =
            AlertDialog::new("Press Enter to add a task; swipe from the left edge to go back.")
                .title("Tip");
        let theme = Theme::default();
        let measure = Widget::<()>::measure(&alert, &theme).expect("a measure closure");
        let free = measure(None, None);
        let narrow = measure(Some(280.0), None);
        assert!(
            narrow.width <= 280.0,
            "clamped to the offer ({})",
            narrow.width
        );
        assert!(narrow.height > free.height, "wrapped → taller");
        // The measure key follows the content (the relayout cache).
        let other = AlertDialog::new("short");
        assert_ne!(
            Widget::<()>::measure_key(&alert, &theme),
            Widget::<()>::measure_key(&other, &theme)
        );
    }
}
