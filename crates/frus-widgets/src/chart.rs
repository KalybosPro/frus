//! Le domaine « graphes » : des **vues de données** auto-peintes, thémées.
//!
//! - [`BarChart`] : une série de `(libellé, valeur)` en barres verticales mises à l'échelle de la
//!   valeur maximale, valeur au-dessus, libellé en dessous, ligne de base.
//! - [`LineChart`] : la même série tracée en **polyligne** (segments reliant les points, marqueurs
//!   ronds), pour lire une tendance plutôt que comparer des grandeurs.
//!
//! Toutes deux sont purement **auto-peintes** (aucun enfant) et non génériques sur `Msg` (façon
//! [`crate::Icon`]) : ce sont des vues de données, pas des contrôles.

use frus_core::{Color, Path, Point, Rect, Scene};
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
/// Largeur de la marge gauche réservée aux graduations de l'axe des ordonnées (quand présent).
const Y_AXIS_W: f32 = 34.0;
/// Taille de police des graduations de l'axe des ordonnées.
const AXIS_SIZE: f32 = 11.0;

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
    /// Nombre de divisions de l'axe des ordonnées (lignes de grille + graduations) ; `0` = aucun.
    grid: usize,
}

impl BarChart {
    /// Crée un graphique depuis une série de `(libellé, valeur)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data.into_iter().map(|(l, v)| (l.into(), v.max(0.0))).collect(),
            color: None,
            height: DEFAULT_HEIGHT,
            grid: 0,
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

    /// Ajoute un **axe des ordonnées** : `divisions` lignes de grille horizontales avec leurs
    /// graduations (`0..max`) dans une marge à gauche. `0` (défaut) = aucun axe.
    pub fn grid(mut self, divisions: usize) -> Self {
        self.grid = divisions;
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

/// Largeur de la marge d'axe si `divisions > 0`, sinon `0` (partagé BarChart / LineChart).
fn axis_width(divisions: usize) -> f32 {
    if divisions > 0 {
        Y_AXIS_W
    } else {
        0.0
    }
}

/// Dessine l'**axe des ordonnées** : `divisions` lignes de grille horizontales de `plot_left` à
/// `plot_left + plot_w`, réparties entre la ligne de base et le haut de la zone de tracé, chacune
/// étiquetée de sa valeur (`0..max`) alignée à droite dans la marge de gauche. Partagé par les deux
/// graphiques (façon Flutter : la grille se lit derrière les barres ou la courbe).
#[allow(clippy::too_many_arguments)]
fn draw_grid(
    scene: &mut Scene,
    theme: &Theme,
    plot_left: f32,
    plot_w: f32,
    plot_top: f32,
    baseline_y: f32,
    max: f32,
    divisions: usize,
    opacity: f32,
) {
    if divisions == 0 {
        return;
    }
    let plot_h = baseline_y - plot_top;
    for i in 0..=divisions {
        let t = i as f32 / divisions as f32;
        let y = baseline_y - plot_h * t;
        // Ligne de grille (sauf i == 0 : c'est la ligne de base, déjà tracée par le graphique).
        if i > 0 {
            scene.fill_rect(Rect::new(plot_left, y, plot_w, 1.0), theme.border.fade(opacity * 0.6));
        }
        // Graduation : valeur alignée à droite dans la marge.
        let label = format_value(max * t);
        let lw = frus_text::measure(&label, AXIS_SIZE).width;
        scene.text(
            Point::new(plot_left - 6.0 - lw, y - AXIS_SIZE * 0.5),
            label,
            AXIS_SIZE,
            theme.muted.fade(opacity),
        );
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

        // Zone de tracé : sous la bande des valeurs, au-dessus des libellés de catégorie ; une
        // marge à gauche accueille les graduations de l'axe des ordonnées, s'il est demandé.
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let axis_w = axis_width(self.grid);
        let plot_left = bounds.x + axis_w;
        let plot_w = bounds.width - axis_w;
        let slot = plot_w / n as f32;
        let bar_w = slot * BAR_FILL;

        // Grille horizontale + graduations (derrière les barres).
        draw_grid(scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, o);

        // Ligne de base (axe des abscisses).
        scene.fill_rect(
            Rect::new(plot_left, baseline_y, plot_w, 1.5),
            theme.border.fade(o),
        );

        for (i, (label, value)) in self.values.iter().enumerate() {
            let cx = plot_left + slot * (i as f32 + 0.5);
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

/// Rayon (px) des marqueurs ronds posés sur chaque point d'une [`LineChart`].
const MARKER_R: f32 = 3.5;
/// Opacité (relative) de l'aire remplie sous la courbe.
const AREA_ALPHA: f32 = 0.16;
/// Épaisseur (px) du trait de la polyligne.
const LINE_W: f32 = 2.0;

/// Un graphique en **lignes** : la même série `(libellé, valeur)` qu'une [`BarChart`], mais reliée
/// en polyligne (segments + marqueurs) pour donner à lire une **tendance**.
///
/// ```
/// use frus_widgets::LineChart;
/// let chart = LineChart::new([("Mon", 3.0), ("Tue", 5.0), ("Wed", 2.0)]).height(160.0);
/// ```
pub struct LineChart {
    values: Vec<(String, f32)>,
    /// Couleur du trait et des marqueurs ; `None` = `primary` du thème.
    color: Option<Color>,
    height: f32,
    /// Nombre de divisions de l'axe des ordonnées (lignes de grille + graduations) ; `0` = aucun.
    grid: usize,
    /// Remplir l'aire sous la courbe (dégradé plat, couleur du trait atténuée) ?
    fill: bool,
}

impl LineChart {
    /// Crée un graphique en lignes depuis une série de `(libellé, valeur)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data.into_iter().map(|(l, v)| (l.into(), v.max(0.0))).collect(),
            color: None,
            height: DEFAULT_HEIGHT,
            grid: 0,
            fill: false,
        }
    }

    /// Surcharge la couleur du trait (défaut : `primary` du thème).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Hauteur du graphique en pixels logiques (défaut 200).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(X_LABEL_H + VALUE_SIZE + 8.0);
        self
    }

    /// Ajoute un **axe des ordonnées** : `divisions` lignes de grille horizontales avec leurs
    /// graduations (`0..max`) dans une marge à gauche. `0` (défaut) = aucun axe.
    pub fn grid(mut self, divisions: usize) -> Self {
        self.grid = divisions;
        self
    }

    /// Remplit l'**aire** sous la courbe (couleur du trait fortement atténuée), pour souligner le
    /// volume plutôt que la seule tendance. Défaut : désactivé.
    pub fn area(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// La valeur maximale de la série (au moins 1 pour une échelle stable).
    fn max_value(&self) -> f32 {
        self.values.iter().map(|(_, v)| *v).fold(0.0, f32::max).max(1.0)
    }
}

impl<Msg> Widget<Msg> for LineChart {
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

        // Même géométrie que la BarChart : bande des valeurs en haut, libellés en bas, marge
        // gauche pour l'axe des ordonnées s'il est demandé.
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let axis_w = axis_width(self.grid);
        let plot_left = bounds.x + axis_w;
        let plot_w = bounds.width - axis_w;
        let slot = plot_w / n as f32;

        // Grille horizontale + graduations (derrière la courbe).
        draw_grid(scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, o);

        // Ligne de base (axe des abscisses).
        scene.fill_rect(
            Rect::new(plot_left, baseline_y, plot_w, 1.5),
            theme.border.fade(o),
        );

        // Points : centre de chaque case, hauteur proportionnelle à la valeur.
        let points: Vec<Point> = self
            .values
            .iter()
            .enumerate()
            .map(|(i, (_, value))| {
                let cx = plot_left + slot * (i as f32 + 0.5);
                let py = baseline_y - (value / max) * plot_h;
                Point::new(cx, py)
            })
            .collect();

        // Aire sous la courbe : polygone points + retour par la ligne de base (façon non-zero,
        // refermé automatiquement au remplissage). Peint avant le trait pour rester dessous.
        if self.fill && points.len() >= 2 {
            let mut area = Path::new().move_to(Point::new(points[0].x, baseline_y));
            for p in &points {
                area = area.line_to(*p);
            }
            area = area.line_to(Point::new(points[points.len() - 1].x, baseline_y));
            scene.fill_path(&area, accent.fade(o * AREA_ALPHA));
        }

        // Polyligne reliant les points (au moins deux points pour un segment).
        if points.len() >= 2 {
            let mut line = Path::new().move_to(points[0]);
            for p in &points[1..] {
                line = line.line_to(*p);
            }
            scene.stroke_path(&line, accent.fade(o), LINE_W);
        }

        // Marqueurs + libellés.
        for (i, (label, value)) in self.values.iter().enumerate() {
            let p = points[i];
            scene.fill_path(&Path::circle(p, MARKER_R), accent.fade(o));
            // Valeur au-dessus du point.
            let vs = format_value(*value);
            let vw = frus_text::measure(&vs, VALUE_SIZE).width;
            scene.text(
                Point::new(p.x - vw * 0.5, p.y - MARKER_R - VALUE_SIZE - 2.0),
                vs,
                VALUE_SIZE,
                theme.on_surface.fade(o),
            );
            // Libellé de catégorie sous la ligne de base.
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(plot_left + slot * (i as f32 + 0.5) - lw * 0.5, baseline_y + 4.0),
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

    fn paint_line(chart: &LineChart, w: f32, h: f32) -> Vec<Primitive> {
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
    fn line_empty_series_paints_nothing() {
        assert!(paint_line(&LineChart::new(Vec::<(String, f32)>::new()), 300.0, 200.0).is_empty());
    }

    #[test]
    fn line_connects_all_points() {
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        let prims = paint_line(&chart, 300.0, 200.0);
        // Une polyligne tracée (chemin avec contour, sans remplissage).
        let polyline = prims.iter().find_map(|p| match p {
            Primitive::Path { path, stroke: Some(_), fill: None, .. } => Some(path),
            _ => None,
        });
        let polyline = polyline.expect("une polyligne tracée");
        // move_to + 2 line_to pour trois points.
        let segments = polyline
            .verbs()
            .iter()
            .filter(|v| matches!(v, frus_core::PathVerb::LineTo(_)))
            .count();
        assert_eq!(segments, 2, "deux segments relient trois points");
        // Un marqueur (chemin rempli) par point.
        let markers = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Path { fill: Some(_), stroke: None, .. }))
            .count();
        assert_eq!(markers, 3, "un marqueur par point");
        // Valeurs et libellés dessinés.
        let has_text = |t: &str| {
            prims.iter().any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(has_text("6") && has_text("2") && has_text("4"), "valeurs affichées");
        assert!(has_text("A") && has_text("B") && has_text("C"), "libellés affichés");
    }

    #[test]
    fn area_fills_a_polygon_under_the_curve() {
        // Un chemin **rempli** (sans contour) fait de segments droits = l'aire ; sans `.area`,
        // seuls les marqueurs (cercles, sans `LineTo`) sont remplis.
        let filled_polygons = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| match p {
                    Primitive::Path { fill: Some(_), stroke: None, path, .. } => path
                        .verbs()
                        .iter()
                        .any(|v| matches!(v, frus_core::PathVerb::LineTo(_))),
                    _ => false,
                })
                .count()
        };
        assert_eq!(filled_polygons(&LineChart::new([("A", 2.0), ("B", 6.0)])), 0, "aucune aire par défaut");
        assert_eq!(
            filled_polygons(&LineChart::new([("A", 2.0), ("B", 6.0)]).area(true)),
            1,
            "une aire remplie sous la courbe"
        );
    }

    #[test]
    fn grid_draws_horizontal_lines_and_axis_labels() {
        // Une série max 8, quatre divisions → graduations 0, 2, 4, 6, 8.
        let chart = LineChart::new([("A", 2.0), ("B", 8.0)]).grid(4);
        let prims = paint_line(&chart, 300.0, 200.0);
        // Lignes fines horizontales (hauteur ~1) : 4 lignes de grille + la ligne de base (1.5).
        let thin_lines = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.height <= 1.6))
            .count();
        assert!(thin_lines >= 5, "4 lignes de grille + ligne de base, obtenu {thin_lines}");
        let has_text = |t: &str| {
            prims.iter().any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Graduations de l'axe : 0 (base) et 8 (haut) au moins.
        assert!(has_text("0") && has_text("8"), "graduations de l'axe des ordonnées");
    }

    #[test]
    fn no_grid_by_default_keeps_full_width() {
        // Sans grille, aucune graduation « 0 » n'est dessinée (comportement d'origine).
        let chart = LineChart::new([("A", 2.0), ("B", 8.0)]);
        let prims = paint_line(&chart, 300.0, 200.0);
        let has_zero = prims.iter().any(|p| matches!(p, Primitive::Text { text, .. } if text == "0"));
        assert!(!has_zero, "pas d'axe par défaut");
    }
}
