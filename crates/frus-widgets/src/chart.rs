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
/// Hauteur de la bande de légende (au-dessus de la zone de tracé) quand elle est affichée.
const LEGEND_H: f32 = 20.0;
/// Côté de la pastille de couleur d'une entrée de légende.
const LEGEND_SWATCH: f32 = 10.0;
/// Taille de police des entrées de légende.
const LEGEND_SIZE: f32 = 12.0;
/// Taille de police du contenu d'une infobulle.
const TOOLTIP_SIZE: f32 = 12.0;
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
    /// Nom de la série principale (pour la légende) ; `None` = anonyme.
    name: Option<String>,
    /// Séries **additionnelles** `(nom, couleur, valeurs)`, alignées par index sur les catégories
    /// de la série principale.
    extra: Vec<(String, Color, Vec<f32>)>,
    /// Afficher une légende (pastille + nom par série) ?
    legend: bool,
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
            name: None,
            extra: Vec::new(),
            legend: false,
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

    /// Nomme la série **principale** (affiché dans la légende).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Ajoute une **série additionnelle** `(nom, couleur, valeurs)`, alignée par index sur les
    /// catégories de la série principale. Toutes les séries partagent l'échelle et l'axe.
    pub fn series(
        mut self,
        name: impl Into<String>,
        color: Color,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.extra.push((name.into(), color, values.into_iter().map(|v| v.max(0.0)).collect()));
        self
    }

    /// Affiche une **légende** (pastille de couleur + nom) pour chaque série nommée. Défaut : off.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// La valeur maximale de **toutes** les séries (au moins 1 pour une échelle stable).
    fn max_value(&self) -> f32 {
        let primary = self.values.iter().map(|(_, v)| *v);
        let extra = self.extra.iter().flat_map(|(_, _, vs)| vs.iter().copied());
        primary.chain(extra).fold(0.0, f32::max).max(1.0)
    }

    /// La légende doit-elle être dessinée (activée **et** au moins une série nommée) ?
    fn has_legend(&self) -> bool {
        self.legend && (self.name.is_some() || !self.extra.is_empty())
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
        let single = self.extra.is_empty();

        // Géométrie partagée avec la BarChart : bande des valeurs en haut (libellés de valeur,
        // série unique seulement), libellés de catégorie en bas, marge gauche pour l'axe, et — le
        // cas échéant — une bande de légende tout en haut.
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let axis_w = axis_width(self.grid);
        let plot_left = bounds.x + axis_w;
        let plot_w = bounds.width - axis_w;
        let slot = plot_w / n as f32;

        // Grille horizontale + graduations (derrière les courbes).
        draw_grid(scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, o);

        // Ligne de base (axe des abscisses).
        scene.fill_rect(Rect::new(plot_left, baseline_y, plot_w, 1.5), theme.border.fade(o));

        // Toutes les séries à tracer : la principale puis les additionnelles, alignées par index.
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let mut series: Vec<(Color, &str, &[f32])> =
            vec![(accent, self.name.as_deref().unwrap_or("Series 1"), primary.as_slice())];
        for (name, color, vals) in &self.extra {
            series.push((*color, name.as_str(), vals.as_slice()));
        }

        // Points d'une série (index → centre de case, hauteur ∝ valeur).
        let points_of = |vals: &[f32]| -> Vec<Point> {
            (0..n.min(vals.len()))
                .map(|i| {
                    Point::new(plot_left + slot * (i as f32 + 0.5), baseline_y - (vals[i] / max) * plot_h)
                })
                .collect()
        };

        for (color, _, vals) in &series {
            let points = points_of(vals);
            // Aire sous la courbe (série unique seulement, façon non-zero refermée).
            if single && self.fill && points.len() >= 2 {
                let mut area = Path::new().move_to(Point::new(points[0].x, baseline_y));
                for p in &points {
                    area = area.line_to(*p);
                }
                area = area.line_to(Point::new(points[points.len() - 1].x, baseline_y));
                scene.fill_path(&area, color.fade(o * AREA_ALPHA));
            }
            // Polyligne.
            if points.len() >= 2 {
                let mut line = Path::new().move_to(points[0]);
                for p in &points[1..] {
                    line = line.line_to(*p);
                }
                scene.stroke_path(&line, color.fade(o), LINE_W);
            }
            // Marqueurs, et — série unique — la valeur au-dessus de chaque point.
            for (i, p) in points.iter().enumerate() {
                scene.fill_path(&Path::circle(*p, MARKER_R), color.fade(o));
                if single {
                    let vs = format_value(vals[i]);
                    let vw = frus_text::measure(&vs, VALUE_SIZE).width;
                    scene.text(
                        Point::new(p.x - vw * 0.5, p.y - MARKER_R - VALUE_SIZE - 2.0),
                        vs,
                        VALUE_SIZE,
                        theme.on_surface.fade(o),
                    );
                }
            }
        }

        // Libellés de catégorie (une fois, sous la ligne de base).
        for (i, (label, _)) in self.values.iter().enumerate() {
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(plot_left + slot * (i as f32 + 0.5) - lw * 0.5, baseline_y + 4.0),
                label.clone(),
                LABEL_SIZE,
                theme.muted.fade(o),
            );
        }

        // Légende : pastille de couleur + nom par série, en haut, de gauche à droite.
        if self.has_legend() {
            let mut x = plot_left;
            let sy = bounds.y + (LEGEND_H - LEGEND_SWATCH) * 0.5;
            for (color, name, _) in &series {
                scene.draw_rect(
                    Rect::new(x, sy, LEGEND_SWATCH, LEGEND_SWATCH),
                    color.fade(o),
                    2.0,
                    0.0,
                    Color::TRANSPARENT,
                );
                x += LEGEND_SWATCH + 5.0;
                scene.text(
                    Point::new(x, bounds.y + (LEGEND_H - LEGEND_SIZE) * 0.5),
                    (*name).to_string(),
                    LEGEND_SIZE,
                    theme.muted.fade(o),
                );
                x += frus_text::measure(name, LEGEND_SIZE).width + 16.0;
            }
        }

        // Infobulle de sous-région (jalon 211) : quand le pointeur survole la zone de tracé
        // (`hover_cursor`, pose par le shell via `cursor_icon`), on met en avant la catégorie la
        // plus proche et on liste la valeur de chaque série.
        if let Some(hc) = status.hover_cursor {
            let lx = hc.x - bounds.x;
            let hi = (((lx - plot_left) / slot - 0.5).round() as i64).clamp(0, n as i64 - 1) as usize;
            let gx = plot_left + slot * (hi as f32 + 0.5);
            // Guide vertical à la catégorie visée.
            scene.fill_rect(
                Rect::new(gx - 0.5, plot_top, 1.0, baseline_y - plot_top),
                theme.border.fade(o * 0.8),
            );
            // Marqueurs accentués + lignes de l'infobulle (catégorie puis une par série).
            let mut lines: Vec<(Option<Color>, String)> = vec![(None, self.values[hi].0.clone())];
            for (color, name, vals) in &series {
                if hi < vals.len() {
                    let py = baseline_y - (vals[hi] / max) * plot_h;
                    scene.fill_path(&Path::circle(Point::new(gx, py), MARKER_R + 2.0), color.fade(o));
                    let txt = if single {
                        format_value(vals[hi])
                    } else {
                        format!("{}  {}", name, format_value(vals[hi]))
                    };
                    lines.push((Some(*color), txt));
                }
            }
            // Boîte : dimensionnée au plus long libellé, placée à droite du guide (repliée à gauche
            // si elle déborderait), ancrée en haut de la zone de tracé.
            let pad = 8.0;
            let line_h = TOOLTIP_SIZE + 5.0;
            let dot = 7.0;
            let text_w = lines
                .iter()
                .map(|(c, t)| {
                    let lead = if c.is_some() { dot + 5.0 } else { 0.0 };
                    lead + frus_text::measure(t, TOOLTIP_SIZE).width
                })
                .fold(0.0, f32::max);
            let bw = text_w + pad * 2.0;
            let bh = lines.len() as f32 * line_h + pad * 2.0 - 3.0;
            let mut bx = gx + 12.0;
            if bx + bw > bounds.x + bounds.width {
                bx = gx - 12.0 - bw;
            }
            bx = bx.max(bounds.x);
            let by = plot_top.max(bounds.y);
            scene.draw_rect(Rect::new(bx, by, bw, bh), theme.surface.fade(o), 6.0, 1.0, theme.border.fade(o));
            let mut ty = by + pad;
            for (c, t) in &lines {
                let mut tx = bx + pad;
                if let Some(col) = c {
                    scene.fill_path(
                        &Path::circle(Point::new(tx + dot * 0.5, ty + TOOLTIP_SIZE * 0.5), dot * 0.5),
                        col.fade(o),
                    );
                    tx += dot + 5.0;
                }
                scene.text(Point::new(tx, ty), t.clone(), TOOLTIP_SIZE, theme.on_surface.fade(o));
                ty += line_h;
            }
        }
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        // Active le suivi du pointeur (jalon 208) sur la zone de tracé — sans changer la forme du
        // curseur (Default) — pour alimenter l'infobulle. Hors zone : pas d'avis.
        if self.values.is_empty() {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = height - X_LABEL_H;
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        let plot_left = axis_width(self.grid);
        let plot_right = width;
        if local_x >= plot_left && local_x <= plot_right && local_y >= plot_top && local_y <= baseline_y {
            Some(crate::interaction::Cursor::Default)
        } else {
            None
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
    fn hovering_the_plot_shows_a_tooltip_guide() {
        use crate::interaction::Cursor;
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        // Rectangles verticaux fins et hauts = le guide de l'infobulle.
        let guides = |status: Status| {
            let mut scene = Scene::new();
            Widget::<()>::paint(&chart, Rect::new(0.0, 0.0, 300.0, 220.0), status, &Theme::default(), &mut scene);
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { rect, .. }
                    if rect.width <= 1.5 && rect.height > 50.0))
                .count()
        };
        let hovering = Status { hover_cursor: Some(Point::new(150.0, 100.0)), ..Default::default() };
        assert_eq!(guides(hovering), 1, "un guide vertical au survol de la zone");
        assert_eq!(guides(Status::default()), 0, "pas d'infobulle sans survol");
        // `cursor_icon` active le suivi sur la zone de tracé (Default), pas en dehors.
        assert_eq!(Widget::<()>::cursor_icon(&chart, 150.0, 100.0, 300.0, 220.0), Some(Cursor::Default));
        assert_eq!(Widget::<()>::cursor_icon(&chart, 150.0, 5.0, 300.0, 220.0), None, "au-dessus de la zone");
    }

    #[test]
    fn multi_series_draws_each_line_and_a_legend() {
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)])
            .name("Sales")
            .series("Costs", frus_core::Color::rgb8(200, 80, 80), [1.0, 3.0, 2.0])
            .legend(true);
        let prims = paint_line(&chart, 300.0, 220.0);
        // Deux polylignes (une par série).
        let polylines = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Path { stroke: Some(_), fill: None, .. }))
            .count();
        assert_eq!(polylines, 2, "une polyligne par série");
        // Deux pastilles de légende (~10x10).
        let swatches = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. }
                if (rect.width - LEGEND_SWATCH).abs() < 0.5 && (rect.height - LEGEND_SWATCH).abs() < 0.5))
            .count();
        assert_eq!(swatches, 2, "une pastille par série");
        // Les noms de séries figurent dans la légende.
        let has_text = |t: &str| prims.iter().any(|p| matches!(p, Primitive::Text { text, .. } if text == t));
        assert!(has_text("Sales") && has_text("Costs"), "noms de séries en légende");
    }

    #[test]
    fn max_value_spans_all_series() {
        // L'échelle englobe la série additionnelle (max 9 > max principal 6).
        let chart = LineChart::new([("A", 2.0), ("B", 6.0)]).series("x", Color::rgb8(1, 2, 3), [9.0, 1.0]);
        assert_eq!(chart.max_value(), 9.0);
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
