//! [`Alert`] : un encadré de message **persistant** (contextuel), à distinguer du
//! [`crate::Toast`] transitoire.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::Style;

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

const PAD: f32 = 12.0;
const ACCENT: f32 = 4.0;
const ICON_W: f32 = 26.0;
const TITLE_SIZE: f32 = 16.0;
const TEXT_SIZE: f32 = 15.0;

/// Nature d'un encadré.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Info,
    Success,
    Warning,
    Error,
}

/// Un encadré de message.
pub struct Alert {
    title: Option<String>,
    text: String,
    kind: AlertKind,
}

impl Alert {
    /// Crée un encadré d'information.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            title: None,
            text: text.into(),
            kind: AlertKind::Info,
        }
    }

    /// Ajoute un titre.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Variante succès.
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

impl<Msg> Widget<Msg> for Alert {
    fn style(&self) -> Style {
        // Paragraphe : dimensions libres, la taille vient de `measure()` —
        // le message se replie à la largeur offerte par le parent.
        Style::default()
    }

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        let text = self.text.clone();
        let title = self.title.clone();
        Some(Box::new(move |max_width, _| {
            use frus_core::FontWeight;
            let chrome = ACCENT + ICON_W + PAD; // barre + icône + marge droite
            let text_avail = max_width.map(|w| (w - chrome).max(40.0));
            let body = frus_text::measure_wrapped(
                &text,
                TEXT_SIZE,
                FontWeight::Regular,
                false,
                text_avail,
            );
            let title_h = title
                .as_ref()
                .map(|_| frus_text::line_height(TITLE_SIZE) + 4.0)
                .unwrap_or(0.0);
            let title_w = title
                .as_ref()
                .map(|t| frus_text::measure(t, TITLE_SIZE).width)
                .unwrap_or(0.0);
            let natural_w = chrome + body.width.max(title_w);
            frus_core::Size::new(
                max_width.map_or(natural_w, |w| w.min(natural_w)).ceil(),
                (PAD * 2.0 + title_h + body.height).ceil(),
            )
        }))
    }

    fn measure_key(&self) -> Option<u64> {
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
        // Fond teinté + bordure discrète.
        scene.draw_rect(
            bounds,
            accent.fade(0.12 * o),
            theme.radius,
            1.0,
            accent.fade(0.4 * o),
        );
        // Barre d'accent à gauche.
        scene.draw_rect(
            Rect::new(bounds.x, bounds.y, ACCENT, bounds.height),
            accent.fade(o),
            0.0,
            0.0,
            Color::TRANSPARENT,
        );
        // Glyphe.
        scene.text(
            Point::new(bounds.x + ACCENT + 7.0, bounds.y + PAD),
            self.icon().to_string(),
            TITLE_SIZE,
            accent.fade(o),
        );
        let text_x = bounds.x + ACCENT + ICON_W;
        // Le message se replie à la largeur réellement disponible (celle de la
        // mise en page), en cohérence avec `measure()`.
        let wrap_w = (bounds.width - (ACCENT + ICON_W + PAD)).max(40.0);
        let body_style = frus_core::TextStyle::new(TEXT_SIZE);
        match &self.title {
            Some(title) => {
                scene.text(
                    Point::new(text_x, bounds.y + PAD),
                    title.clone(),
                    TITLE_SIZE,
                    theme.on_surface.fade(o),
                );
                scene.text_wrapped(
                    Point::new(
                        text_x,
                        bounds.y + PAD + frus_text::line_height(TITLE_SIZE) + 4.0,
                    ),
                    self.text.clone(),
                    &body_style,
                    theme.muted.fade(o),
                    wrap_w,
                );
            }
            None => {
                scene.text_wrapped(
                    Point::new(text_x, bounds.y + PAD),
                    self.text.clone(),
                    &body_style,
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
        let alert = Alert::new("Attention !").title("Alerte").warning();
        let mut scene = Scene::new();
        Widget::<()>::paint(
            &alert,
            Rect::new(0.0, 0.0, 240.0, 60.0),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        let warn = Color::rgb8(230, 170, 40);
        // Barre d'accent (couleur pleine de la variante) présente.
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { color, .. } if *color == warn)));
        // Titre + texte peints.
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Alerte")));
        assert!(scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "Attention !")));
    }

    /// Le message se **replie** à la largeur offerte : plus étroit → plus haut
    /// (et jamais plus large que l'offre) — fini l'encadré qui déborde.
    #[test]
    fn message_wraps_to_the_offered_width() {
        let alert = Alert::new("Press Enter to add a task; swipe from the left edge to go back.")
            .title("Tip");
        let measure = Widget::<()>::measure(&alert).expect("closure de mesure");
        let free = measure(None, None);
        let narrow = measure(Some(280.0), None);
        assert!(narrow.width <= 280.0, "bornée à l'offre ({})", narrow.width);
        assert!(narrow.height > free.height, "repliée → plus haute");
        // La clé de mesure suit le contenu (cache de relayout).
        let other = Alert::new("court");
        assert_ne!(
            Widget::<()>::measure_key(&alert),
            Widget::<()>::measure_key(&other)
        );
    }
}
