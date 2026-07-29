//! [`BarChart`] : un **graphique à barres** simple, piloté par les données et thémé.
//!
//! Premier widget du domaine « graphes » : une série de `(libellé, valeur)` rendue en barres
//! verticales mises à l'échelle de la valeur maximale, avec la valeur au-dessus de chaque barre,
//! le libellé en dessous, et une ligne de base. Purement **auto-peint** (aucun enfant), non
//! générique sur `Msg` (façon [`crate::Icon`]) : c'est une **vue** de données, pas un contrôle.

use frus_core::{Color, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Hauteur par défaut du graphique (px logiques).
const DEFAULT_HEIGHT: f32 = 200.0;
/// Bande réservée aux libellés de catégorie sous la ligne de base.
const X_LABEL_H: f32 = 22.0;
/// Taille de police des valeurs (au-dessus des barres) et des libellés (dessous).
const VALUE_SIZE: f32 = 12.0;
const LABEL_SIZE: f32 = 12.0;
/// Fraction de la « case » d'une barre réellement occupée par la barre (le reste = espacement).
const BAR_FILL: f32 = 0.6;

/// Un graphique à barres.
///
/// ```
/// use frus_widgets::BarChart;
/// let chart = BarChart::new([("Mon", 3.0), ("Tue", 5.0), ("Wed", 2.0)]).height(160.0);
/// ```
pub struct BarChart {
    values: Vec<(String, f32)>,
    /// Couleur des barres ; `None` = `primary` du thème.
    color: Option<Color>,
    height: f32,
}

impl BarChart {
    /// Crée un graphique depuis une série de `(libellé, valeur)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data.into_iter().map(|(l, v)| (l.into(), v.max(0.0))).collect(),
            color: None,
            height: DEFAULT_HEIGHT,
        }
    }

    /// Surcharge la couleur des barres (défaut : `primary` du thème).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Hauteur du graphique en pixels logiques (défaut 200).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(X_LABEL_H + VALUE_SIZE + 8.0);
        self
    }

    /// La valeur maximale de la série (au moins 1 pour une échelle stable).
    fn max_value(&self) -> f32 {
        self.values.iter().map(|(_, v)| *v).fold(0.0, f32::max).max(1.0)
    }
}

/// Formate une valeur : entière si elle l'est, sinon une décimale.
fn format_value(v: f32) -> String {
    if (v.fract()).abs() < 1e-6 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

impl<Msg> Widget<Msg> for BarChart {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let n = self.values.len();
        if n == 0 {
            return;
        }
        let o = status.opacity;
        let accent = self.color.unwrap_or(theme.primary);
        let max = self.max_value();

        // Zone de tracé : sous la bande des valeurs, au-dessus des libellés de catégorie.
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let slot = bounds.width / n as f32;
        let bar_w = slot * BAR_FILL;

        // Ligne de base (axe des abscisses).
        scene.fill_rect(
            Rect::new(bounds.x, baseline_y, bounds.width, 1.5),
            theme.border.fade(o),
        );

        for (i, (label, value)) in self.values.iter().enumerate() {
            let cx = bounds.x + slot * (i as f32 + 0.5);
            let h = (value / max) * plot_h;
            // Barre (coin supérieur arrondi via un petit rayon uniforme).
            scene.draw_rect(
                Rect::new(cx - bar_w * 0.5, baseline_y - h, bar_w, h),
                accent.fade(o),
                4.0,
                0.0,
                Color::TRANSPARENT,
            );
            // Valeur au-dessus de la barre.
            let vs = format_value(*value);
            let vw = frus_text::measure(&vs, VALUE_SIZE).width;
            scene.text(
                Point::new(cx - vw * 0.5, baseline_y - h - VALUE_SIZE - 2.0),
                vs,
                VALUE_SIZE,
                theme.on_surface.fade(o),
            );
            // Libellé de catégorie sous la ligne de base.
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(cx - lw * 0.5, baseline_y + 4.0),
                label.clone(),
                LABEL_SIZE,
                theme.muted.fade(o),
            );
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

    fn paint_chart(chart: &BarChart, w: f32, h: f32) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            chart,
            Rect::new(0.0, 0.0, w, h),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn value_formatting() {
        assert_eq!(format_value(3.0), "3");
        assert_eq!(format_value(2.5), "2.5");
    }

    #[test]
    fn empty_series_paints_nothing() {
        assert!(paint_chart(&BarChart::new(Vec::<(String, f32)>::new()), 300.0, 200.0).is_empty());
    }

    #[test]
    fn bars_scale_to_the_max_value() {
        // Trois barres : la plus grande valeur donne la barre la plus haute.
        let chart = BarChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        let prims = paint_chart(&chart, 300.0, 200.0);
        // Rectangles = ligne de base + 3 barres.
        let bar_heights: Vec<f32> = prims
            .iter()
            .filter_map(|p| match p {
                // Les barres ont une hauteur > 2 (la ligne de base fait 1.5).
                Primitive::Rect { rect, .. } if rect.height > 2.0 => Some(rect.height),
                _ => None,
            })
            .collect();
        assert_eq!(bar_heights.len(), 3, "une barre par valeur");
        // B (6) est la plus haute ; A (2) la plus basse ; proportionnel.
        let max_h = bar_heights.iter().cloned().fold(0.0_f32, f32::max);
        let min_h = bar_heights.iter().cloned().fold(f32::MAX, f32::min);
        assert!(max_h > min_h * 2.5, "6 vaut trois fois 2 : {max_h} vs {min_h}");
        // Valeurs et libellés dessinés.
        let has_text = |t: &str| {
            prims.iter().any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has_text("6") && has_text("2") && has_text("4"), "valeurs affichées");
        assert!(has_text("A") && has_text("B") && has_text("C"), "libellés affichés");
    }
}
