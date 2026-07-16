//! [`RichText`] : un paragraphe de **texte riche** — un arbre [`TextSpan`]
//! (styles mêlés, héritage en cascade) aplati en runs résolus et mis en forme
//! d'un seul tenant.
//!
//! ```ignore
//! RichText::new(
//!     TextSpan::new("frus is ")
//!         .child(TextSpan::new("fast").bold())
//!         .child(TextSpan::new(" and portable.").italic()),
//! )
//! ```

use frus_core::{Color, Point, Rect, Scene, TextRun, TextSpan, TextStyle};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Un paragraphe de texte riche. Taille naturelle par défaut (les `\n`
/// explicites font les lignes) ; avec [`RichText::wrap`], il revient à la ligne
/// à la largeur offerte par le parent.
pub struct RichText {
    span: TextSpan,
    /// Style de **base** du paragraphe : la racine de la cascade (ce dont les
    /// spans héritent quand ils ne précisent rien).
    base: TextStyle,
    /// Paragraphe replié à la largeur offerte.
    wrap: bool,
}

impl RichText {
    /// Crée un paragraphe (base : 16 px, graisse normale, couleur du thème).
    pub fn new(span: TextSpan) -> Self {
        Self {
            span,
            base: TextStyle::new(16.0),
            wrap: false,
        }
    }

    /// Surcharge le style de base (racine de la cascade) — typiquement un cran de
    /// l'échelle du thème (`.base_style(theme.text.body_large)`).
    pub fn base_style(mut self, style: TextStyle) -> Self {
        self.base = style;
        self
    }

    /// Fait du paragraphe un texte **replié** : il revient à la ligne à la
    /// largeur offerte par le parent (mesure sous contraintes via taffy).
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Runs résolus : la couleur héritée est tranchée contre `fallback` et
    /// modulée par `opacity`. (Pour la mesure, la couleur est indifférente.)
    fn runs(&self, fallback: Color, opacity: f32) -> Vec<TextRun> {
        self.span
            .flatten(self.base)
            .into_iter()
            .map(|(text, style)| TextRun {
                text,
                size: style.size,
                weight: style.weight,
                italic: style.italic,
                color: style.color.unwrap_or(fallback).fade(opacity),
                decoration: style.decoration,
                // Héritée = couleur du run : résolue ici pour que le fondu de
                // sortie s'applique aussi aux décorations.
                decoration_color: style
                    .decoration_color
                    .map(|c| c.fade(opacity)),
            })
            .collect()
    }
}

impl<Msg> Widget<Msg> for RichText {
    fn style(&self) -> Style {
        // Paragraphe replié : dimensions libres, la taille vient de `measure()`.
        if self.wrap {
            return Style::default();
        }
        let measured = frus_text::measure_runs(&self.runs(Color::WHITE, 1.0));
        Style {
            width: Dimension::Length(measured.width.ceil()),
            height: Dimension::Length(measured.height.ceil()),
            ..Default::default()
        }
    }

    fn measure(&self) -> Option<frus_layout::MeasureFn> {
        if !self.wrap {
            return None;
        }
        let runs = self.runs(Color::WHITE, 1.0);
        Some(Box::new(move |max_width, _| {
            frus_text::measure_runs_wrapped(&runs, max_width)
        }))
    }

    fn measure_key(&self) -> Option<u64> {
        if !self.wrap {
            return None;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.span.measure_hash(&mut hasher);
        self.base.size.to_bits().hash(&mut hasher);
        self.base.weight.to_u16().hash(&mut hasher);
        self.base.italic.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let runs = self.runs(theme.on_surface, status.opacity);
        if self.wrap {
            scene.rich_text_wrapped(Point::new(bounds.x, bounds.y), runs, bounds.width);
        } else {
            scene.rich_text(Point::new(bounds.x, bounds.y), runs);
        }
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::{FontWeight, Primitive};

    #[test]
    fn paints_resolved_runs_with_theme_colour() {
        let theme = Theme::default();
        let rich = RichText::new(
            TextSpan::new("plain ")
                .child(TextSpan::new("bold").bold())
                .child(TextSpan::new("red").color(Color::rgb(1.0, 0.0, 0.0))),
        )
        .base_style(theme.text.body_large);

        let mut scene = Scene::new();
        Widget::<()>::paint(
            &rich,
            Rect::new(2.0, 3.0, 300.0, 24.0),
            Status::default(),
            &theme,
            &mut scene,
        );

        assert_eq!(scene.primitives().len(), 1);
        match &scene.primitives()[0] {
            Primitive::RichText { position, runs, .. } => {
                assert_eq!(*position, Point::new(2.0, 3.0));
                assert_eq!(runs.len(), 3);
                // Run 0 : hérite tout de la base (16 px, couleur du thème).
                assert_eq!(runs[0].size, 16.0);
                assert_eq!(runs[0].color, theme.on_surface);
                // Run 1 : gras, taille/couleur héritées.
                assert_eq!(runs[1].weight, FontWeight::Bold);
                assert_eq!(runs[1].size, 16.0);
                assert_eq!(runs[1].color, theme.on_surface);
                // Run 2 : couleur explicite.
                assert_eq!(runs[2].color, Color::rgb(1.0, 0.0, 0.0));
            }
            _ => panic!("attendu du texte riche"),
        }
    }

    #[test]
    fn wrapped_rich_text_measures_and_keys_by_content() {
        let para = |text: &str| {
            RichText::new(TextSpan::new(text).child(TextSpan::new(" gras").bold())).wrap()
        };
        let rich = para("un paragraphe riche assez long pour se replier sur plusieurs lignes");
        // La mesure sous contraintes se replie : plus haut à 120 px qu'en libre.
        let measure = Widget::<()>::measure(&rich).expect("closure de mesure");
        let free = measure(None, None);
        let narrow = measure(Some(120.0), None);
        assert!(narrow.width <= 120.0);
        assert!(narrow.height > free.height, "replié → plus haut");
        // La clé de mesure suit le contenu (correction du cache de relayout)…
        assert_ne!(
            Widget::<()>::measure_key(&rich),
            Widget::<()>::measure_key(&para("court"))
        );
        // … mais pas la couleur (sans effet sur la géométrie).
        let recolored = RichText::new(
            TextSpan::new("un paragraphe riche assez long pour se replier sur plusieurs lignes")
                .color(Color::rgb(1.0, 0.0, 0.0))
                .child(TextSpan::new(" gras").bold()),
        )
        .wrap();
        assert_eq!(
            Widget::<()>::measure_key(&rich),
            Widget::<()>::measure_key(&recolored),
            "la couleur ne doit pas invalider la mise en page"
        );
        // Sans `.wrap()` : ni mesure ni clé.
        let plain = RichText::new(TextSpan::new("x"));
        assert!(Widget::<()>::measure(&plain).is_none());
        assert!(Widget::<()>::measure_key(&plain).is_none());
    }

    #[test]
    fn layout_accounts_for_the_largest_run() {
        // Un run 28 px au milieu : la hauteur mesurée dépasse celle d'un 16 px.
        let small: Style = Widget::<()>::style(&RichText::new(TextSpan::new("plain")));
        let tall: Style = Widget::<()>::style(&RichText::new(
            TextSpan::new("plain").child(TextSpan::new("BIG").size(28.0)),
        ));
        let h = |s: &Style| match s.height {
            Dimension::Length(v) => v,
            _ => panic!("hauteur mesurée attendue"),
        };
        assert!(h(&tall) > h(&small), "le grand run doit grandir la ligne");
    }
}
