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
/// Taille de police de la part (`%`) écrite dans une strate en mode 100 % (jalon 227).
const STRATA_LABEL_SIZE: f32 = 11.0;
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
/// let chart: BarChart = BarChart::new([("Mon", 3.0), ("Tue", 5.0), ("Wed", 2.0)]).height(160.0);
/// ```
pub struct BarChart<Msg = ()> {
    values: Vec<(String, f32)>,
    /// Couleur des barres ; `None` = `primary` du thème.
    color: Option<Color>,
    height: f32,
    /// Nombre de divisions de l'axe des ordonnées (lignes de grille + graduations) ; `0` = aucun.
    grid: usize,
    /// Nom de la série principale (pour la légende) ; `None` = anonyme.
    name: Option<String>,
    /// Séries **additionnelles** `(nom, couleur, valeurs)` — barres **groupées** par catégorie.
    extra: Vec<(String, Color, Vec<f32>)>,
    /// Afficher une légende (pastille + nom par série) ?
    legend: bool,
    /// Index de séries **masquées** (non tracées, atténuées en légende) — jalon 215.
    hidden: Vec<usize>,
    /// Message émis au clic sur une entrée de légende (index de série) — jalon 215.
    on_legend: Option<Box<dyn Fn(usize) -> Msg>>,
    /// Empiler les séries (barres cumulées) plutôt que les grouper ? — jalon 216.
    stacked: bool,
    /// Message émis au clic sur une **barre** `(catégorie, série)` — jalon 222.
    on_point: Option<Box<dyn Fn(usize, usize) -> Msg>>,
    /// Barre/strate **épinglée** `(catégorie, série)`, mise en évidence par un anneau persistant —
    /// jalon 223.
    selected: Option<(usize, usize)>,
    /// Empilage **100 %** : chaque colonne est normalisée à son total (proportions) — jalon 224.
    normalized: bool,
}

impl<Msg> BarChart<Msg> {
    /// Crée un graphique depuis une série de `(libellé, valeur)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data
                .into_iter()
                .map(|(l, v)| (l.into(), v.max(0.0)))
                .collect(),
            color: None,
            height: DEFAULT_HEIGHT,
            grid: 0,
            name: None,
            extra: Vec::new(),
            legend: false,
            hidden: Vec::new(),
            on_legend: None,
            stacked: false,
            on_point: None,
            selected: None,
            normalized: false,
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

    /// Nomme la série **principale** (affiché dans la légende).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Ajoute une **série additionnelle** `(nom, couleur, valeurs)`, dessinée en barres **groupées**
    /// côte à côte dans chaque catégorie. Toutes les séries partagent l'échelle et l'axe.
    pub fn series(
        mut self,
        name: impl Into<String>,
        color: Color,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.extra.push((
            name.into(),
            color,
            values.into_iter().map(|v| v.max(0.0)).collect(),
        ));
        self
    }

    /// Affiche une **légende** (pastille de couleur + nom) pour chaque série. Défaut : off.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// **Masque** les séries d'index donnés (non tracées, atténuées en légende) — jalon 215.
    pub fn hidden(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.hidden = indices.into_iter().collect();
        self
    }

    /// Rend la **légende cliquable** : `on_legend(index)` au clic sur une entrée — jalon 215.
    pub fn on_legend(mut self, on_legend: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_legend = Some(Box::new(on_legend));
        self
    }

    /// **Empile** les séries : une barre par catégorie, segmentée par série (barres cumulées), au
    /// lieu de barres groupées côte à côte (jalon 212). Défaut : off.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Rend les **barres cliquables** : `on_point(catégorie, série)` au clic sur une barre (ou une
    /// strate empilée) visible. Défaut : aucun — jalon 222.
    pub fn on_point(mut self, on_point: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.on_point = Some(Box::new(on_point));
        self
    }

    /// **Épingle** une barre/strate `(catégorie, série)` : elle reçoit un anneau d'accent persistant
    /// (met en évidence la sélection courante, façon détail cliqué). `None` = rien — jalon 223.
    pub fn selected(mut self, selected: Option<(usize, usize)>) -> Self {
        self.selected = selected;
        self
    }

    /// Normalise l'empilage en **100 %** : chaque colonne remplit toute la hauteur, chaque strate
    /// occupant sa **proportion** du total de la catégorie (plutôt que sa valeur absolue). N'a d'effet
    /// qu'en mode empilé multi-séries. Défaut : off — jalon 224.
    pub fn normalized(mut self, normalized: bool) -> Self {
        self.normalized = normalized;
        self
    }

    /// La valeur maximale de **toutes** les séries (au moins 1 pour une échelle stable).
    fn max_value(&self) -> f32 {
        let primary = self.values.iter().map(|(_, v)| *v);
        let extra = self.extra.iter().flat_map(|(_, _, vs)| vs.iter().copied());
        primary.chain(extra).fold(0.0, f32::max).max(1.0)
    }

    /// Total (des séries **visibles**) de la catégorie `i` — dénominateur de l'empilage 100 %.
    fn category_total(&self, i: usize) -> f32 {
        let base = if self.hidden.contains(&0) {
            0.0
        } else {
            self.values.get(i).map(|(_, v)| *v).unwrap_or(0.0)
        };
        let rest: f32 = self
            .extra
            .iter()
            .enumerate()
            .filter(|(j, _)| !self.hidden.contains(&(j + 1)))
            .map(|(_, (_, _, vs))| vs.get(i).copied().unwrap_or(0.0))
            .sum();
        (base + rest).max(1e-6)
    }

    /// Le maximum de la **somme** des séries par catégorie (échelle en mode empilé).
    fn stacked_max(&self) -> f32 {
        let n = self.values.len();
        (0..n)
            .map(|i| {
                self.values[i].1
                    + self
                        .extra
                        .iter()
                        .map(|(_, _, vs)| vs.get(i).copied().unwrap_or(0.0))
                        .sum::<f32>()
            })
            .fold(0.0, f32::max)
            .max(1.0)
    }

    /// La légende doit-elle être dessinée (activée **et** au moins une série nommée) ?
    fn has_legend(&self) -> bool {
        self.legend && (self.name.is_some() || !self.extra.is_empty())
    }

    /// Noms de toutes les séries (principale puis additionnelles) — pour la légende / le routage.
    fn series_names(&self) -> Vec<&str> {
        let mut names = vec![self.name.as_deref().unwrap_or("Series 1")];
        names.extend(self.extra.iter().map(|(n, _, _)| n.as_str()));
        names
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

/// Formate une mesure pour l'infobulle : la valeur brute, suivie de sa **part** (`%`) du total quand
/// un dénominateur d'empilage 100 % est fourni (jalon 226). `None` = valeur seule.
fn format_measure(value: f32, percent_of: Option<f32>) -> String {
    match percent_of {
        Some(total) if total > 0.0 => {
            format!(
                "{} ({}%)",
                format_value(value),
                (value / total * 100.0).round() as i64
            )
        }
        _ => format_value(value),
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
    percent: bool,
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
            scene.fill_rect(
                Rect::new(plot_left, y, plot_w, 1.0),
                theme.border.fade(opacity * 0.6),
            );
        }
        // Graduation : valeur (ou pourcentage en mode 100 %) alignée à droite dans la marge.
        let label = if percent {
            format!("{}%", (t * 100.0).round() as i64)
        } else {
            format_value(max * t)
        };
        let lw = frus_text::measure(&label, AXIS_SIZE).width;
        scene.text(
            Point::new(plot_left - 6.0 - lw, y - AXIS_SIZE * 0.5),
            label,
            AXIS_SIZE,
            theme.muted.fade(opacity),
        );
    }
}

/// Dessine une **légende** (pastille de couleur + nom, de gauche à droite) dans la bande du haut.
/// Partagé BarChart / LineChart (jalon 209/212).
fn draw_legend(
    scene: &mut Scene,
    theme: &Theme,
    left: f32,
    top: f32,
    series: &[(Color, &str)],
    o: f32,
) {
    let mut x = left;
    let sy = top + (LEGEND_H - LEGEND_SWATCH) * 0.5;
    for (color, name) in series {
        scene.draw_rect(
            Rect::new(x, sy, LEGEND_SWATCH, LEGEND_SWATCH),
            color.fade(o),
            2.0,
            0.0,
            Color::TRANSPARENT,
        );
        x += LEGEND_SWATCH + 5.0;
        scene.text(
            Point::new(x, top + (LEGEND_H - LEGEND_SIZE) * 0.5),
            (*name).to_string(),
            LEGEND_SIZE,
            theme.muted.fade(o),
        );
        x += frus_text::measure(name, LEGEND_SIZE).width + 16.0;
    }
}

/// Quelle **entrée de légende** contient le point local `(x, y)` ? Reconstruit la disposition de
/// [`draw_legend`] pour router un clic vers l'index de série. Partagé (jalon 215).
fn legend_hit(local_x: f32, local_y: f32, plot_left: f32, names: &[&str]) -> Option<usize> {
    if local_y < 0.0 || local_y > LEGEND_H {
        return None;
    }
    let mut x = plot_left;
    for (i, name) in names.iter().enumerate() {
        let entry_w = LEGEND_SWATCH + 5.0 + frus_text::measure(name, LEGEND_SIZE).width;
        if local_x >= x && local_x <= x + entry_w {
            return Some(i);
        }
        x += entry_w + 16.0;
    }
    None
}

/// Dessine une **infobulle** de survol : un guide vertical à `gx`, puis une boîte listant `lines`
/// (chaque ligne : pastille optionnelle + texte). La boîte est dimensionnée au plus long libellé,
/// posée à droite du guide (repliée à gauche si elle déborde), ancrée en haut de la zone de tracé.
/// Partagé BarChart / LineChart (jalon 211/212).
#[allow(clippy::too_many_arguments)]
fn draw_tooltip(
    scene: &mut Scene,
    theme: &Theme,
    bounds: Rect,
    gx: f32,
    plot_top: f32,
    baseline_y: f32,
    lines: &[(Option<Color>, String)],
    o: f32,
) {
    scene.fill_rect(
        Rect::new(gx - 0.5, plot_top, 1.0, baseline_y - plot_top),
        theme.border.fade(o * 0.8),
    );
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
    scene.draw_rect(
        Rect::new(bx, by, bw, bh),
        theme.surface.fade(o),
        6.0,
        1.0,
        theme.border.fade(o),
    );
    let mut ty = by + pad;
    for (c, t) in lines {
        let mut tx = bx + pad;
        if let Some(col) = c {
            scene.fill_path(
                &Path::circle(
                    Point::new(tx + dot * 0.5, ty + TOOLTIP_SIZE * 0.5),
                    dot * 0.5,
                ),
                col.fade(o),
            );
            tx += dot + 5.0;
        }
        scene.text(
            Point::new(tx, ty),
            t.clone(),
            TOOLTIP_SIZE,
            theme.on_surface.fade(o),
        );
        ty += line_h;
    }
}

/// Le point local `(x, y)` est-il dans la **zone de tracé** ? Si oui, renvoie `Cursor::Default`
/// (active le suivi du pointeur pour l'infobulle sans changer la forme du curseur — un graphe n'est
/// pas cliquable), sinon `None`. Partagé BarChart / LineChart (jalon 211/212).
fn chart_plot_hit(
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    plot_left: f32,
    plot_top: f32,
) -> Option<crate::interaction::Cursor> {
    let baseline_y = height - X_LABEL_H;
    if local_x >= plot_left && local_x <= width && local_y >= plot_top && local_y <= baseline_y {
        Some(crate::interaction::Cursor::Default)
    } else {
        None
    }
}

impl<Msg> Widget<Msg> for BarChart<Msg> {
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
        let single = self.extra.is_empty();
        let stacked = self.stacked && !single;
        // Empilage **100 %** (jalon 224) : chaque colonne est normalisée à son total (proportions).
        let normalized = self.normalized && stacked;
        // En empilé, l'échelle doit contenir le **total** cumulé par catégorie.
        let max = if stacked {
            self.stacked_max()
        } else {
            self.max_value()
        };

        // Zone de tracé : sous la bande des valeurs, au-dessus des libellés de catégorie ; marge
        // gauche pour l'axe, et — le cas échéant — une bande de légende tout en haut.
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let axis_w = axis_width(self.grid);
        let plot_left = bounds.x + axis_w;
        let plot_w = bounds.width - axis_w;
        let slot = plot_w / n as f32;

        // Grille horizontale + graduations (derrière les barres) ; en 100 %, l'axe est en pourcentages.
        draw_grid(
            scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, normalized, o,
        );
        // Ligne de base (axe des abscisses).
        scene.fill_rect(
            Rect::new(plot_left, baseline_y, plot_w, 1.5),
            theme.border.fade(o),
        );

        // Toutes les séries : la principale puis les additionnelles (barres groupées).
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let mut series: Vec<(Color, &str, &[f32])> = vec![(
            accent,
            self.name.as_deref().unwrap_or("Series 1"),
            primary.as_slice(),
        )];
        for (name, color, vals) in &self.extra {
            series.push((*color, name.as_str(), vals.as_slice()));
        }
        let s = series.len();

        // Chaque catégorie : soit un groupe de `s` barres côte à côte (jalon 212), soit — en
        // empilé — une seule barre segmentée par série (barres cumulées, jalon 216).
        let group_w = slot * BAR_FILL;
        let bar_w = group_w / s as f32;
        let inner = if s == 1 { 1.0 } else { 0.86 };
        // Rectangle de la barre/strate épinglée, capturé pour tracer son anneau après coup (jalon 223).
        let mut sel_rect: Option<Rect> = None;
        for i in 0..n {
            let cx = plot_left + slot * (i as f32 + 0.5);
            if stacked {
                // Segments empilés du bas vers le haut (les séries masquées ne comptent pas). En
                // 100 %, le dénominateur est le total de la catégorie (colonne pleine), sinon l'échelle.
                let sbx = cx - group_w * 0.5;
                let denom = if normalized {
                    self.category_total(i)
                } else {
                    max
                };
                let mut lower = 0.0_f32;
                for (j, (color, _, vals)) in series.iter().enumerate() {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let value = vals.get(i).copied().unwrap_or(0.0);
                    let y_bottom = baseline_y - (lower / denom) * plot_h;
                    let y_top = baseline_y - ((lower + value) / denom) * plot_h;
                    let rect = Rect::new(sbx, y_top, group_w, y_bottom - y_top);
                    scene.draw_rect(rect, color.fade(o), 0.0, 0.0, Color::TRANSPARENT);
                    if self.selected == Some((i, j)) {
                        sel_rect = Some(rect);
                    }
                    // Libellé centré dans la strate si elle est assez haute pour l'accueillir : la
                    // part (%) en 100 % (jalon 227), la valeur brute en empilé absolu (jalon 229).
                    // Texte lisible sur fond saturé.
                    let seg_h = y_bottom - y_top;
                    if seg_h >= STRATA_LABEL_SIZE + 4.0 {
                        let label = if normalized {
                            format!("{}%", (value / denom * 100.0).round() as i64)
                        } else {
                            format_value(value)
                        };
                        let lw = frus_text::measure(&label, STRATA_LABEL_SIZE).width;
                        scene.text(
                            Point::new(
                                cx - lw * 0.5,
                                (y_top + y_bottom) * 0.5 - STRATA_LABEL_SIZE * 0.5,
                            ),
                            label,
                            STRATA_LABEL_SIZE,
                            theme.on_primary.fade(o * 0.95),
                        );
                    }
                    lower += value;
                }
                // Total de la colonne au-dessus de la strate supérieure (empilé **absolu** : parité
                // avec la valeur des barres simples ; en 100 % la colonne est pleine) — jalon 228.
                if !normalized && lower > 0.0 {
                    let vs = format_value(lower);
                    let vw = frus_text::measure(&vs, VALUE_SIZE).width;
                    let top_y = baseline_y - (lower / denom) * plot_h;
                    scene.text(
                        Point::new(cx - vw * 0.5, top_y - VALUE_SIZE - 2.0),
                        vs,
                        VALUE_SIZE,
                        theme.on_surface.fade(o),
                    );
                }
            } else {
                let group_left = cx - group_w * 0.5;
                for (j, (color, _, vals)) in series.iter().enumerate() {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let value = vals.get(i).copied().unwrap_or(0.0);
                    let h = (value / max) * plot_h;
                    let draw_w = bar_w * inner;
                    let bx = group_left + j as f32 * bar_w + (bar_w - draw_w) * 0.5;
                    let rect = Rect::new(bx, baseline_y - h, draw_w, h);
                    scene.draw_rect(rect, color.fade(o), 4.0, 0.0, Color::TRANSPARENT);
                    if self.selected == Some((i, j)) {
                        sel_rect = Some(rect);
                    }
                    // Valeur au-dessus (série unique seulement, pour éviter la surcharge).
                    if single {
                        let vs = format_value(value);
                        let vw = frus_text::measure(&vs, VALUE_SIZE).width;
                        scene.text(
                            Point::new(cx - vw * 0.5, baseline_y - h - VALUE_SIZE - 2.0),
                            vs,
                            VALUE_SIZE,
                            theme.on_surface.fade(o),
                        );
                    }
                }
            }
            // Libellé de catégorie sous la ligne de base.
            let label = &self.values[i].0;
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(cx - lw * 0.5, baseline_y + 4.0),
                label.clone(),
                LABEL_SIZE,
                theme.muted.fade(o),
            );
        }

        // Barre/strate épinglée (jalon 223) : anneau d'accent persistant, légèrement dilaté autour
        // du rectangle, dans une couleur contrastée (indépendant du survol).
        if let Some(r) = sel_rect {
            scene.draw_rect(
                Rect::new(r.x - 2.5, r.y - 2.5, r.width + 5.0, r.height + 5.0),
                Color::TRANSPARENT,
                5.0,
                2.0,
                theme.on_surface.fade(o),
            );
        }

        // Légende (bande du haut), partagée ; les séries masquées y sont atténuées.
        if self.has_legend() {
            let entries: Vec<(Color, &str)> = series
                .iter()
                .enumerate()
                .map(|(i, (c, name, _))| {
                    (
                        if self.hidden.contains(&i) {
                            c.fade(0.35)
                        } else {
                            *c
                        },
                        *name,
                    )
                })
                .collect();
            draw_legend(scene, theme, plot_left, bounds.y, &entries, o);
        }

        // Infobulle de survol (jalon 212) : catégorie la plus proche + valeur de chaque série visible.
        if let Some(hc) = status.hover_cursor {
            let lx = hc.x - bounds.x;
            let hi =
                (((lx - plot_left) / slot - 0.5).round() as i64).clamp(0, n as i64 - 1) as usize;
            let gx = plot_left + slot * (hi as f32 + 0.5);
            let mut lines: Vec<(Option<Color>, String)> = vec![(None, self.values[hi].0.clone())];
            // En 100 %, chaque mesure est suivie de sa part du total de la catégorie survolée (jalon 226).
            let percent_of = if normalized {
                Some(self.category_total(hi))
            } else {
                None
            };
            for (j, (color, name, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) {
                    continue;
                }
                let value = vals.get(hi).copied().unwrap_or(0.0);
                let vtxt = format_measure(value, percent_of);
                let txt = if single {
                    vtxt
                } else {
                    format!("{}  {}", name, vtxt)
                };
                lines.push((Some(*color), txt));
            }
            draw_tooltip(scene, theme, bounds, gx, plot_top, baseline_y, &lines, o);
        }
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        if self.values.is_empty() {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        chart_plot_hit(
            local_x,
            local_y,
            width,
            height,
            axis_width(self.grid),
            plot_top,
        )
    }

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        // 1) Clic sur une entrée de légende → on_legend(index) (jalon 215).
        if let Some(f) = &self.on_legend {
            if self.has_legend() {
                if let Some(idx) = legend_hit(
                    local_x,
                    local_y,
                    axis_width(self.grid),
                    &self.series_names(),
                ) {
                    return Some(f(idx));
                }
            }
        }
        // 2) Clic sur une **barre** (ou une strate empilée) → on_point(catégorie, série) (jalon 222).
        // Géométrie identique au paint : on reconstruit chaque rectangle et on teste l'inclusion.
        let f = self.on_point.as_ref()?;
        let n = self.values.len();
        if n == 0 {
            return None;
        }
        let single = self.extra.is_empty();
        let stacked = self.stacked && !single;
        let max = if stacked {
            self.stacked_max()
        } else {
            self.max_value()
        };
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = height - X_LABEL_H;
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let plot_left = axis_width(self.grid);
        let slot = (width - plot_left) / n as f32;
        let s = 1 + self.extra.len();
        let group_w = slot * BAR_FILL;
        let bar_w = group_w / s as f32;
        let inner = if s == 1 { 1.0 } else { 0.86 };
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let value_at = |j: usize, i: usize| -> f32 {
            if j == 0 {
                primary.get(i).copied().unwrap_or(0.0)
            } else {
                self.extra[j - 1].2.get(i).copied().unwrap_or(0.0)
            }
        };
        for i in 0..n {
            let cx = plot_left + slot * (i as f32 + 0.5);
            if stacked {
                let sbx = cx - group_w * 0.5;
                let mut lower = 0.0_f32;
                for j in 0..s {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let value = value_at(j, i);
                    let y_bottom = baseline_y - (lower / max) * plot_h;
                    let y_top = baseline_y - ((lower + value) / max) * plot_h;
                    if local_x >= sbx
                        && local_x <= sbx + group_w
                        && local_y >= y_top
                        && local_y <= y_bottom
                    {
                        return Some(f(i, j));
                    }
                    lower += value;
                }
            } else {
                let group_left = cx - group_w * 0.5;
                for j in 0..s {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let h = (value_at(j, i) / max) * plot_h;
                    let draw_w = bar_w * inner;
                    let bx = group_left + j as f32 * bar_w + (bar_w - draw_w) * 0.5;
                    if local_x >= bx
                        && local_x <= bx + draw_w
                        && local_y >= baseline_y - h
                        && local_y <= baseline_y
                    {
                        return Some(f(i, j));
                    }
                }
            }
        }
        None
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Rayon (px) des marqueurs ronds posés sur chaque point d'une [`LineChart`].
const MARKER_R: f32 = 3.5;
/// Rayon (px) de tolérance pour cliquer un point (jalon 221).
const POINT_HIT_R: f32 = 12.0;
/// Opacité (relative) de l'aire remplie sous la courbe.
const AREA_ALPHA: f32 = 0.16;
/// Opacité (relative) des bandes d'un graphique en **aires empilées** (plus soutenue, pour lire
/// chaque strate).
const STACK_ALPHA: f32 = 0.55;
/// Hauteur de la bande de légende (au-dessus de la zone de tracé) quand elle est affichée.
const LEGEND_H: f32 = 20.0;
/// Côté de la pastille de couleur d'une entrée de légende.
const LEGEND_SWATCH: f32 = 10.0;
/// Taille de police des entrées de légende.
const LEGEND_SIZE: f32 = 12.0;
/// Taille de police du contenu d'une infobulle.
const TOOLTIP_SIZE: f32 = 12.0;
/// Vitesse (cycles/seconde) du halo pulsant animé (jalon 217).
const PULSE_SPEED: f32 = 1.6;
/// Croissance (px) du rayon du halo pulsant sur un cycle.
const PULSE_GROW: f32 = 10.0;
/// Épaisseur (px) du trait de la polyligne.
const LINE_W: f32 = 2.0;

/// Un graphique en **lignes** : la même série `(libellé, valeur)` qu'une [`BarChart`], mais reliée
/// en polyligne (segments + marqueurs) pour donner à lire une **tendance**.
///
/// ```
/// use frus_widgets::LineChart;
/// let chart: LineChart = LineChart::new([("Mon", 3.0), ("Tue", 5.0), ("Wed", 2.0)]).height(160.0);
/// ```
pub struct LineChart<Msg = ()> {
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
    /// Empiler les séries (aires cumulées) plutôt que les superposer ?
    stacked: bool,
    /// Index de séries **masquées** (non tracées, atténuées en légende) — jalon 215.
    hidden: Vec<usize>,
    /// Message émis au clic sur une entrée de légende (index de série) — jalon 215.
    on_legend: Option<Box<dyn Fn(usize) -> Msg>>,
    /// Animer un **halo pulsant** sur le point survolé (repaint continu) — jalon 217.
    animated: bool,
    /// Message émis au clic sur un **point** `(catégorie, série)` — jalon 221.
    on_point: Option<Box<dyn Fn(usize, usize) -> Msg>>,
    /// Point **épinglé** `(catégorie, série)`, mis en évidence par un halo + anneau persistants —
    /// jalon 223.
    selected: Option<(usize, usize)>,
    /// Empilage **100 %** : chaque catégorie est normalisée à son total (proportions) — jalon 224.
    normalized: bool,
}

impl<Msg> LineChart<Msg> {
    /// Crée un graphique en lignes depuis une série de `(libellé, valeur)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data
                .into_iter()
                .map(|(l, v)| (l.into(), v.max(0.0)))
                .collect(),
            color: None,
            height: DEFAULT_HEIGHT,
            grid: 0,
            fill: false,
            name: None,
            extra: Vec::new(),
            legend: false,
            stacked: false,
            hidden: Vec::new(),
            on_legend: None,
            animated: false,
            on_point: None,
            selected: None,
            normalized: false,
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
        self.extra.push((
            name.into(),
            color,
            values.into_iter().map(|v| v.max(0.0)).collect(),
        ));
        self
    }

    /// Affiche une **légende** (pastille de couleur + nom) pour chaque série nommée. Défaut : off.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// **Empile** les séries : chaque aire est cumulée au-dessus des précédentes (aires cumulées),
    /// pour lire un total et sa composition. Implique le remplissage des bandes. Défaut : off.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// **Masque** les séries d'index donnés (non tracées, atténuées en légende) — jalon 215.
    pub fn hidden(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.hidden = indices.into_iter().collect();
        self
    }

    /// Rend la **légende cliquable** : `on_legend(index)` au clic sur une entrée — jalon 215.
    pub fn on_legend(mut self, on_legend: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_legend = Some(Box::new(on_legend));
        self
    }

    /// Anime un **halo pulsant** (grandit puis s'estompe) sur le point survolé. Demande un repaint
    /// continu tant que le graphique est affiché. Défaut : off — jalon 217.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Rend les **points cliquables** : `on_point(catégorie, série)` au clic près d'un marqueur
    /// (des séries visibles). Défaut : aucun — jalon 221.
    pub fn on_point(mut self, on_point: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.on_point = Some(Box::new(on_point));
        self
    }

    /// **Épingle** un point `(catégorie, série)` : il reçoit un halo + un anneau d'accent persistants
    /// (met en évidence la sélection courante, façon détail cliqué). `None` = rien — jalon 223.
    pub fn selected(mut self, selected: Option<(usize, usize)>) -> Self {
        self.selected = selected;
        self
    }

    /// Normalise l'empilage en **100 %** : chaque catégorie remplit toute la hauteur, chaque strate
    /// occupant sa **proportion** du total. N'a d'effet qu'en aires empilées multi-séries. Défaut :
    /// off — jalon 224.
    pub fn normalized(mut self, normalized: bool) -> Self {
        self.normalized = normalized;
        self
    }

    /// Total (des séries **visibles**) de la catégorie `i` — dénominateur de l'empilage 100 %.
    fn category_total(&self, i: usize) -> f32 {
        let base = if self.hidden.contains(&0) {
            0.0
        } else {
            self.values.get(i).map(|(_, v)| *v).unwrap_or(0.0)
        };
        let rest: f32 = self
            .extra
            .iter()
            .enumerate()
            .filter(|(j, _)| !self.hidden.contains(&(j + 1)))
            .map(|(_, (_, _, vs))| vs.get(i).copied().unwrap_or(0.0))
            .sum();
        (base + rest).max(1e-6)
    }

    /// Noms de toutes les séries (principale puis additionnelles).
    fn series_names(&self) -> Vec<&str> {
        let mut names = vec![self.name.as_deref().unwrap_or("Series 1")];
        names.extend(self.extra.iter().map(|(n, _, _)| n.as_str()));
        names
    }

    /// La valeur maximale de **toutes** les séries (au moins 1 pour une échelle stable).
    fn max_value(&self) -> f32 {
        let primary = self.values.iter().map(|(_, v)| *v);
        let extra = self.extra.iter().flat_map(|(_, _, vs)| vs.iter().copied());
        primary.chain(extra).fold(0.0, f32::max).max(1.0)
    }

    /// Le maximum de la **somme** des séries par catégorie (échelle en mode empilé).
    fn stacked_max(&self) -> f32 {
        let n = self.values.len();
        (0..n)
            .map(|i| {
                let base = self.values[i].1;
                let rest: f32 = self
                    .extra
                    .iter()
                    .map(|(_, _, vs)| vs.get(i).copied().unwrap_or(0.0))
                    .sum();
                base + rest
            })
            .fold(0.0, f32::max)
            .max(1.0)
    }

    /// La légende doit-elle être dessinée (activée **et** au moins une série nommée) ?
    fn has_legend(&self) -> bool {
        self.legend && (self.name.is_some() || !self.extra.is_empty())
    }
}

impl<Msg> Widget<Msg> for LineChart<Msg> {
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
        let single = self.extra.is_empty();
        let stacked = self.stacked && !single;
        // Empilage **100 %** (jalon 224) : chaque catégorie est normalisée à son total (proportions).
        let normalized = self.normalized && stacked;
        // En empilé, l'échelle doit contenir le **total** cumulé par catégorie.
        let max = if stacked {
            self.stacked_max()
        } else {
            self.max_value()
        };

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

        // Grille horizontale + graduations (derrière les courbes) ; en 100 %, l'axe est en pourcentages.
        draw_grid(
            scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, normalized, o,
        );

        // Ligne de base (axe des abscisses).
        scene.fill_rect(
            Rect::new(plot_left, baseline_y, plot_w, 1.5),
            theme.border.fade(o),
        );

        // Toutes les séries à tracer : la principale puis les additionnelles, alignées par index.
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let mut series: Vec<(Color, &str, &[f32])> = vec![(
            accent,
            self.name.as_deref().unwrap_or("Series 1"),
            primary.as_slice(),
        )];
        for (name, color, vals) in &self.extra {
            series.push((*color, name.as_str(), vals.as_slice()));
        }

        // Coordonnées d'une valeur (index, valeur) → point écran.
        let pt = |i: usize, v: f32| {
            Point::new(
                plot_left + slot * (i as f32 + 0.5),
                baseline_y - (v / max) * plot_h,
            )
        };
        // Point d'un **cumul** empilé : en 100 %, le dénominateur est le total de la catégorie
        // (bande pleine hauteur), sinon l'échelle globale (jalon 224).
        let spt = |i: usize, cum: f32| {
            let denom = if normalized {
                self.category_total(i)
            } else {
                max
            };
            Point::new(
                plot_left + slot * (i as f32 + 0.5),
                baseline_y - (cum / denom) * plot_h,
            )
        };

        if stacked {
            // Aires **cumulées** : chaque série est une bande entre son cumul bas et haut, du bas
            // vers le haut ; le trait suit le bord supérieur.
            let mut lower = vec![0.0_f32; n];
            for (j, (color, _, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) {
                    continue;
                }
                let upper: Vec<f32> = (0..n)
                    .map(|i| lower[i] + vals.get(i).copied().unwrap_or(0.0))
                    .collect();
                if n >= 2 {
                    // Bande : bord bas (gauche→droite) puis bord haut (droite→gauche).
                    let mut band = Path::new().move_to(spt(0, lower[0]));
                    for i in 1..n {
                        band = band.line_to(spt(i, lower[i]));
                    }
                    for i in (0..n).rev() {
                        band = band.line_to(spt(i, upper[i]));
                    }
                    scene.fill_path(&band, color.fade(o * STACK_ALPHA));
                    // Trait du bord supérieur.
                    let mut line = Path::new().move_to(spt(0, upper[0]));
                    for i in 1..n {
                        line = line.line_to(spt(i, upper[i]));
                    }
                    scene.stroke_path(&line, color.fade(o), LINE_W);
                }
                // Valeur (ou part %) au centre de la bande à chaque catégorie, si la bande y est
                // assez épaisse — parité avec les strates de barres (jalons 227/229) — jalon 230.
                for i in 0..n {
                    let value = vals.get(i).copied().unwrap_or(0.0);
                    if value <= 0.0 {
                        continue;
                    }
                    let y_lo = spt(i, lower[i]).y;
                    let y_hi = spt(i, upper[i]).y;
                    if y_lo - y_hi >= STRATA_LABEL_SIZE + 4.0 {
                        let label = if normalized {
                            format!(
                                "{}%",
                                (value / self.category_total(i) * 100.0).round() as i64
                            )
                        } else {
                            format_value(value)
                        };
                        let lw = frus_text::measure(&label, STRATA_LABEL_SIZE).width;
                        let px = plot_left + slot * (i as f32 + 0.5);
                        // Borne le libellé dans la zone de tracé (les catégories de bord sinon débordent).
                        let lx = (px - lw * 0.5)
                            .clamp(plot_left, (plot_left + plot_w - lw).max(plot_left));
                        scene.text(
                            Point::new(lx, (y_lo + y_hi) * 0.5 - STRATA_LABEL_SIZE * 0.5),
                            label,
                            STRATA_LABEL_SIZE,
                            theme.on_primary.fade(o * 0.95),
                        );
                    }
                }
                lower = upper;
            }
        } else {
            for (j, (color, _, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) {
                    continue;
                }
                let points: Vec<Point> = (0..n.min(vals.len())).map(|i| pt(i, vals[i])).collect();
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
        }

        // Point épinglé (jalon 223) : halo + anneau d'accent persistants sur le marqueur sélectionné
        // (hors empilé et hors série masquée), indépendant du survol.
        if let Some((sc, ss)) = self.selected {
            if !stacked && ss < series.len() && !self.hidden.contains(&ss) && sc < n {
                let (color, _, vals) = series[ss];
                if let Some(&v) = vals.get(sc) {
                    let p = pt(sc, v);
                    scene.fill_path(&Path::circle(p, MARKER_R + 6.0), color.fade(o * 0.22));
                    scene.stroke_path(&Path::circle(p, MARKER_R + 3.0), color.fade(o), 2.0);
                }
            }
        }

        // Libellés de catégorie (une fois, sous la ligne de base).
        for (i, (label, _)) in self.values.iter().enumerate() {
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(
                    plot_left + slot * (i as f32 + 0.5) - lw * 0.5,
                    baseline_y + 4.0,
                ),
                label.clone(),
                LABEL_SIZE,
                theme.muted.fade(o),
            );
        }

        // Légende (bande du haut), partagée ; les séries masquées y sont atténuées.
        if self.has_legend() {
            let entries: Vec<(Color, &str)> = series
                .iter()
                .enumerate()
                .map(|(i, (c, n, _))| {
                    (
                        if self.hidden.contains(&i) {
                            c.fade(0.35)
                        } else {
                            *c
                        },
                        *n,
                    )
                })
                .collect();
            draw_legend(scene, theme, plot_left, bounds.y, &entries, o);
        }

        // Infobulle de sous-région (jalon 211) : quand le pointeur survole la zone de tracé
        // (`hover_cursor`, pose par le shell via `cursor_icon`), on met en avant la catégorie la
        // plus proche, on accentue le marqueur de chaque série visible, et on liste leurs valeurs.
        if let Some(hc) = status.hover_cursor {
            let lx = hc.x - bounds.x;
            let hi =
                (((lx - plot_left) / slot - 0.5).round() as i64).clamp(0, n as i64 - 1) as usize;
            let gx = plot_left + slot * (hi as f32 + 0.5);
            let mut lines: Vec<(Option<Color>, String)> = vec![(None, self.values[hi].0.clone())];
            // En 100 %, chaque mesure est suivie de sa part du total de la catégorie survolée (jalon 226).
            let percent_of = if normalized {
                Some(self.category_total(hi))
            } else {
                None
            };
            for (j, (color, name, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) || hi >= vals.len() {
                    continue;
                }
                // Marqueur accentué à la valeur (hors empilé : la hauteur individuelle n'a pas
                // de sens sur une strate cumulée).
                if !stacked {
                    let py = baseline_y - (vals[hi] / max) * plot_h;
                    // Halo pulsant animé (jalon 217) : grandit puis s'estompe sous le marqueur.
                    if self.animated {
                        let phase = (status.time * PULSE_SPEED).fract();
                        let r = (MARKER_R + 2.0) + phase * PULSE_GROW;
                        scene.fill_path(
                            &Path::circle(Point::new(gx, py), r),
                            color.fade(o * (1.0 - phase) * 0.4),
                        );
                    }
                    scene.fill_path(
                        &Path::circle(Point::new(gx, py), MARKER_R + 2.0),
                        color.fade(o),
                    );
                }
                let vtxt = format_measure(vals[hi], percent_of);
                let txt = if single {
                    vtxt
                } else {
                    format!("{}  {}", name, vtxt)
                };
                lines.push((Some(*color), txt));
            }
            draw_tooltip(scene, theme, bounds, gx, plot_top, baseline_y, &lines, o);
        }
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        if self.values.is_empty() {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        chart_plot_hit(
            local_x,
            local_y,
            width,
            height,
            axis_width(self.grid),
            plot_top,
        )
    }

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        // 1) Clic sur une entrée de légende → on_legend(index) (jalon 215).
        if let Some(f) = &self.on_legend {
            if self.has_legend() {
                if let Some(idx) = legend_hit(
                    local_x,
                    local_y,
                    axis_width(self.grid),
                    &self.series_names(),
                ) {
                    return Some(f(idx));
                }
            }
        }
        // 2) Clic sur un **point** → on_point(catégorie, série) (jalon 221). Hors mode empilé, où
        // les marqueurs individuels n'existent pas. Géométrie identique au paint.
        let f = self.on_point.as_ref()?;
        let n = self.values.len();
        if n == 0 || (self.stacked && !self.extra.is_empty()) {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = height - X_LABEL_H;
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let plot_left = axis_width(self.grid);
        let slot = (width - plot_left) / n as f32;
        let max = self.max_value();
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        for j in 0..(1 + self.extra.len()) {
            if self.hidden.contains(&j) {
                continue;
            }
            let vals: &[f32] = if j == 0 {
                &primary
            } else {
                &self.extra[j - 1].2
            };
            for (i, &v) in vals.iter().enumerate().take(n) {
                let px = plot_left + slot * (i as f32 + 0.5);
                let py = baseline_y - (v / max) * plot_h;
                if (local_x - px).hypot(local_y - py) <= POINT_HIT_R {
                    return Some(f(i, j));
                }
            }
        }
        None
    }

    fn continuous(&self) -> bool {
        // Repaint continu quand le halo pulsant est actif (jalon 217).
        self.animated
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
        assert!(
            max_h > min_h * 2.5,
            "6 vaut trois fois 2 : {max_h} vs {min_h}"
        );
        // Valeurs et libellés dessinés.
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("6") && has_text("2") && has_text("4"),
            "valeurs affichées"
        );
        assert!(
            has_text("A") && has_text("B") && has_text("C"),
            "libellés affichés"
        );
    }

    #[test]
    fn grouped_series_draw_a_bar_per_series_and_a_legend() {
        let chart = BarChart::new([("A", 4.0), ("B", 8.0), ("C", 6.0)])
            .name("This year")
            .series("Last year", Color::rgb8(200, 120, 80), [3.0, 7.0, 5.0])
            .legend(true);
        let prims = paint_chart(&chart, 320.0, 220.0);
        // 3 catégories × 2 séries = 6 barres (hautes ; > la ligne de base et les pastilles).
        let bars = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.height > 15.0))
            .count();
        assert_eq!(bars, 6, "une barre par (catégorie, série)");
        // Deux pastilles de légende (~10×10).
        let swatches = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. }
                if (rect.width - LEGEND_SWATCH).abs() < 0.5 && (rect.height - LEGEND_SWATCH).abs() < 0.5))
            .count();
        assert_eq!(swatches, 2, "une pastille par série");
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("This year") && has_text("Last year"),
            "noms de séries en légende"
        );
    }

    #[test]
    fn stacked_bars_share_one_column_per_category() {
        let chart = BarChart::new([("A", 2.0), ("B", 4.0)])
            .series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
            .stacked(true);
        // Échelle empilée : max des totaux = max(2+3, 4+1) = 5.
        assert_eq!(chart.stacked_max(), 5.0);
        let prims = paint_chart(&chart, 300.0, 200.0);
        let seg_widths: Vec<f32> = prims
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } if rect.height > 2.0 => Some(rect.width),
                _ => None,
            })
            .collect();
        // 2 catégories × 2 séries = 4 segments, tous à la pleine largeur du groupe (une colonne).
        assert_eq!(seg_widths.len(), 4, "un segment par (catégorie, série)");
        let group_w = (300.0 / 2.0) * BAR_FILL;
        assert!(
            seg_widths.iter().all(|w| (w - group_w).abs() < 0.5),
            "segments empilés = pleine largeur, obtenu {seg_widths:?}"
        );
    }

    #[test]
    fn normalized_stacked_bars_fill_each_column() {
        // 100 % : chaque colonne remplit toute la hauteur, quelle que soit sa somme brute.
        let make = |norm: bool| {
            BarChart::new([("A", 2.0), ("B", 4.0)])
                .series("x", Color::rgb8(1, 2, 3), [3.0, 4.0])
                .stacked(true)
                .normalized(norm)
        };
        // Hauteur cumulée des segments de la colonne A (moitié gauche : x < 150).
        let col_a = |chart: &BarChart| {
            paint_chart(chart, 300.0, 200.0)
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, .. } if rect.height > 2.0 && rect.x < 150.0 => {
                        Some(rect.height)
                    }
                    _ => None,
                })
                .sum::<f32>()
        };
        let plot_h = (200.0 - X_LABEL_H) - (VALUE_SIZE + 6.0); // 160

        // Colonne A (total 5) : pleine en 100 %...
        assert!(
            (col_a(&make(true)) - plot_h).abs() < 1.0,
            "colonne A pleine en 100%, obtenu {}",
            col_a(&make(true))
        );
        // ...mais partielle en absolu (le max total, 8, est en B).
        assert!(
            col_a(&make(false)) < plot_h - 20.0,
            "colonne A partielle en absolu, obtenu {}",
            col_a(&make(false))
        );
        // L'axe en 100 % affiche un pourcentage.
        let prims = paint_chart(&make(true).grid(4), 300.0, 200.0);
        assert!(
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == "100%")),
            "axe en pourcentage"
        );
    }

    #[test]
    fn normalized_bar_tooltip_shows_percentages() {
        // Deux séries empilées ; en 100 %, l'infobulle de survol ajoute la part (%) de chaque série.
        let make = |norm: bool| {
            BarChart::new([("A", 2.0), ("B", 6.0)])
                .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
                .stacked(true)
                .normalized(norm)
        };
        let tooltip_texts = |chart: &BarChart| -> Vec<String> {
            let mut scene = Scene::new();
            // Survol de la catégorie A (x ~ centre de la 1re colonne).
            let status = Status {
                hover_cursor: Some(Point::new(75.0, 90.0)),
                ..Default::default()
            };
            Widget::<()>::paint(
                chart,
                Rect::new(0.0, 0.0, 300.0, 200.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        // Catégorie A : les deux séries valent 2 → 50 % chacune.
        let norm = tooltip_texts(&make(true));
        assert!(
            norm.iter().any(|t| t.contains("(50%)")),
            "part en % dans l'infobulle 100%, obtenu {norm:?}"
        );
        // En absolu : aucune mention de pourcentage.
        let abs = tooltip_texts(&make(false));
        assert!(
            !abs.iter().any(|t| t.contains('%')),
            "pas de % en absolu, obtenu {abs:?}"
        );
    }

    #[test]
    fn normalized_bars_label_each_strata_with_its_percentage() {
        // En 100 %, chaque strate assez haute porte sa part (%) en son centre.
        let make = |norm: bool| {
            BarChart::new([("A", 2.0), ("B", 6.0)])
                .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
                .stacked(true)
                .normalized(norm)
        };
        // Sans axe (pas de graduation %), tout texte finissant par « % » est un libellé de strate.
        let pct_labels = |chart: &BarChart| {
            paint_chart(chart, 300.0, 260.0)
                .iter()
                .filter(|p| matches!(p, Primitive::Text { text, .. } if text.ends_with('%')))
                .count()
        };
        // 2 catégories × 2 séries visibles = 4 strates étiquetées.
        assert_eq!(pct_labels(&make(true)), 4, "une part % par strate");
        assert_eq!(pct_labels(&make(false)), 0, "aucun % en absolu");
    }

    #[test]
    fn stacked_absolute_bars_show_the_column_total() {
        let make = || {
            BarChart::new([("A", 2.0), ("B", 4.0)]).series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
        };
        // Empilé absolu : le total de chaque colonne (A = 2+3 = 5, B = 4+1 = 5) est écrit au-dessus.
        let abs = paint_chart(&make().stacked(true), 300.0, 220.0);
        let count5 = abs
            .iter()
            .filter(|p| matches!(p, Primitive::Text { text, .. } if text == "5"))
            .count();
        assert_eq!(count5, 2, "un total par colonne (5 et 5)");
        // En 100 % : pas de total brut (colonnes pleines, parts en %).
        let norm = paint_chart(&make().stacked(true).normalized(true), 300.0, 220.0);
        assert!(
            !norm
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == "5")),
            "pas de total brut en 100%"
        );
    }

    #[test]
    fn stacked_absolute_bars_label_each_strata_with_its_value() {
        // Empilé absolu : chaque strate assez haute porte sa valeur brute (parité avec le % en 100 %).
        let chart = BarChart::new([("A", 3.0), ("B", 5.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 6.0])
            .stacked(true);
        let prims = paint_chart(&chart, 300.0, 260.0);
        let has = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Valeurs de strates : A = 3 et 4, B = 5 et 6.
        assert!(
            has("3") && has("4") && has("6"),
            "chaque strate porte sa valeur"
        );
        // Le total de colonne (A = 7, B = 11) reste au sommet (jalon 228).
        assert!(has("7") && has("11"), "total de colonne conservé");
    }

    #[test]
    fn hovering_bars_shows_a_tooltip_guide() {
        use crate::interaction::Cursor;
        let chart = BarChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        let guides = |status: Status| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &chart,
                Rect::new(0.0, 0.0, 300.0, 220.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.width <= 1.5 && rect.height > 50.0))
                .count()
        };
        let hovering = Status {
            hover_cursor: Some(Point::new(150.0, 100.0)),
            ..Default::default()
        };
        assert_eq!(guides(hovering), 1, "un guide au survol de la zone");
        assert_eq!(guides(Status::default()), 0, "pas d'infobulle sans survol");
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 100.0, 300.0, 220.0),
            Some(Cursor::Default)
        );
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 5.0, 300.0, 220.0),
            None
        );
    }

    #[test]
    fn clicking_a_bar_emits_category_and_series() {
        // Deux séries groupées : 2 barres par catégorie. Géométrie (width 300, height 200, sans
        // axe ni légende), identique au paint.
        let chart = BarChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s));
        let baseline_y = 200.0 - X_LABEL_H;
        let plot_h = baseline_y - (VALUE_SIZE + 6.0);
        let slot = 300.0 / 2.0;
        let group_w = slot * BAR_FILL;
        let bar_w = group_w / 2.0;
        // Catégorie A (i=0), série additionnelle (j=1), valeur 4 (max = 6).
        let cx = slot * 0.5;
        let group_left = cx - group_w * 0.5;
        let bx = group_left + bar_w + (bar_w - bar_w * 0.86) * 0.5;
        let bar_cx = bx + bar_w * 0.86 * 0.5;
        let mid_y = baseline_y - (4.0 / 6.0) * plot_h * 0.5;
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, bar_cx, mid_y, 300.0, 200.0),
            Some((0, 1)),
            "clic au milieu de la 2e barre de la catégorie A"
        );
        // Au-dessus de la barre (zone vide) : aucun message.
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, bar_cx, 2.0, 300.0, 200.0),
            None
        );
        // Strate empilée : le clic dans la colonne renvoie la strate touchée.
        let stacked = BarChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .stacked(true)
            .on_point(|c, s| (c, s));
        // Catégorie A : strate 0 (valeur 2) en bas, strate 1 (valeur 4) au-dessus ; max total = 6.
        let low_y = baseline_y - (1.0 / 6.0) * plot_h; // dans la strate du bas (0..2)
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&stacked, cx, low_y, 300.0, 200.0),
            Some((0, 0)),
            "clic bas de la colonne = strate 0"
        );
        // Série additionnelle masquée : sa barre n'est plus cliquable.
        let hidden = BarChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s))
            .hidden([1]);
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&hidden, bar_cx, mid_y, 300.0, 200.0),
            None,
            "barre d'une série masquée : pas de clic"
        );
    }

    #[test]
    fn selected_bar_draws_a_persistent_ring() {
        // La barre épinglée reçoit un rectangle à **bordure** (les barres normales ont une bordure 0).
        let rings = |chart: &BarChart| {
            paint_chart(chart, 300.0, 200.0)
                .iter()
                .filter(
                    |p| matches!(p, Primitive::Rect { border_width, .. } if *border_width > 0.5),
                )
                .count()
        };
        assert_eq!(
            rings(&BarChart::new([("A", 2.0), ("B", 6.0)])),
            0,
            "aucun anneau sans sélection"
        );
        assert_eq!(
            rings(&BarChart::new([("A", 2.0), ("B", 6.0)]).selected(Some((1, 0)))),
            1,
            "un anneau sur la barre épinglée"
        );
        // Série masquée épinglée : sa barre n'est pas tracée, donc pas d'anneau.
        let hidden = BarChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
            .hidden([1])
            .selected(Some((0, 1)));
        assert_eq!(rings(&hidden), 0, "pas d'anneau sur une série masquée");
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
            Primitive::Path {
                path,
                stroke: Some(_),
                fill: None,
                ..
            } => Some(path),
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
            .filter(|p| {
                matches!(
                    p,
                    Primitive::Path {
                        fill: Some(_),
                        stroke: None,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(markers, 3, "un marqueur par point");
        // Valeurs et libellés dessinés.
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("6") && has_text("2") && has_text("4"),
            "valeurs affichées"
        );
        assert!(
            has_text("A") && has_text("B") && has_text("C"),
            "libellés affichés"
        );
    }

    #[test]
    fn hovering_the_plot_shows_a_tooltip_guide() {
        use crate::interaction::Cursor;
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        // Rectangles verticaux fins et hauts = le guide de l'infobulle.
        let guides = |status: Status| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &chart,
                Rect::new(0.0, 0.0, 300.0, 220.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Rect { rect, .. }
                    if rect.width <= 1.5 && rect.height > 50.0)
                })
                .count()
        };
        let hovering = Status {
            hover_cursor: Some(Point::new(150.0, 100.0)),
            ..Default::default()
        };
        assert_eq!(
            guides(hovering),
            1,
            "un guide vertical au survol de la zone"
        );
        assert_eq!(guides(Status::default()), 0, "pas d'infobulle sans survol");
        // `cursor_icon` active le suivi sur la zone de tracé (Default), pas en dehors.
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 100.0, 300.0, 220.0),
            Some(Cursor::Default)
        );
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 5.0, 300.0, 220.0),
            None,
            "au-dessus de la zone"
        );
    }

    #[test]
    fn multi_series_draws_each_line_and_a_legend() {
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)])
            .name("Sales")
            .series(
                "Costs",
                frus_core::Color::rgb8(200, 80, 80),
                [1.0, 3.0, 2.0],
            )
            .legend(true);
        let prims = paint_line(&chart, 300.0, 220.0);
        // Deux polylignes (une par série).
        let polylines = prims
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    Primitive::Path {
                        stroke: Some(_),
                        fill: None,
                        ..
                    }
                )
            })
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
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("Sales") && has_text("Costs"),
            "noms de séries en légende"
        );
    }

    #[test]
    fn legend_click_emits_the_series_index() {
        let chart = LineChart::<usize>::new([("A", 1.0)])
            .name("Sales")
            .series("Costs", Color::rgb8(1, 2, 3), [2.0])
            .legend(true)
            .on_legend(|i| i);
        // Clic sur la 1re entrée (pastille près de x=0, y dans la bande de légende).
        assert_eq!(
            Widget::<usize>::positional_click(&chart, 5.0, 10.0, 300.0, 200.0),
            Some(0)
        );
        // 2e entrée : juste après la 1re (pastille + espace + « Sales » + écart).
        let after_first =
            LEGEND_SWATCH + 5.0 + frus_text::measure("Sales", LEGEND_SIZE).width + 16.0;
        assert_eq!(
            Widget::<usize>::positional_click(&chart, after_first + 4.0, 10.0, 300.0, 200.0),
            Some(1)
        );
        // Hors de la bande (y trop bas) : aucun clic de légende.
        assert_eq!(
            Widget::<usize>::positional_click(&chart, 5.0, 100.0, 300.0, 200.0),
            None
        );
        // Sans `on_legend`, la légende n'est pas cliquable.
        let plain = LineChart::<usize>::new([("A", 1.0)])
            .name("Sales")
            .series("Costs", Color::rgb8(1, 2, 3), [2.0])
            .legend(true);
        assert_eq!(
            Widget::<usize>::positional_click(&plain, 5.0, 10.0, 300.0, 200.0),
            None
        );
    }

    #[test]
    fn animated_pulse_adds_a_halo_and_requests_continuous_repaint() {
        let animated = LineChart::new([("A", 2.0), ("B", 6.0)]).animated(true);
        let plain = LineChart::new([("A", 2.0), ("B", 6.0)]);
        assert!(
            Widget::<()>::continuous(&animated),
            "animé => repaint continu"
        );
        assert!(
            !Widget::<()>::continuous(&plain),
            "fixe => pas de repaint continu"
        );
        // Cercles pleins (marqueurs/halo : chemins remplis sans segment droit) au survol.
        let circles = |chart: &LineChart, t: f32| {
            let mut scene = Scene::new();
            let status = Status {
                hover_cursor: Some(Point::new(150.0, 100.0)),
                time: t,
                ..Default::default()
            };
            Widget::<()>::paint(
                chart,
                Rect::new(0.0, 0.0, 300.0, 200.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Path { fill: Some(_), stroke: None, path, .. }
                    if !path.verbs().iter().any(|v| matches!(v, frus_core::PathVerb::LineTo(_))))
                })
                .count()
        };
        // Le halo animé ajoute un cercle que le graphique fixe n'a pas (au même survol).
        assert!(
            circles(&animated, 0.1) > circles(&plain, 0.1),
            "le halo animé ajoute un cercle"
        );
    }

    #[test]
    fn clicking_a_point_emits_category_and_series() {
        let chart = LineChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s));
        // Géométrie (width 300, height 200, sans axe ni légende) — identique au paint.
        let baseline_y = 200.0 - X_LABEL_H;
        let plot_h = baseline_y - (VALUE_SIZE + 6.0);
        let slot = 300.0 / 2.0;
        let px = slot * 0.5; // catégorie A (i = 0)
        let py_primary = baseline_y - (2.0 / 6.0) * plot_h; // série 0, valeur 2 (max = 6)
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, px, py_primary, 300.0, 200.0),
            Some((0, 0)),
            "clic sur le point A de la série principale"
        );
        // Loin de tout marqueur : aucun message.
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, slot, 5.0, 300.0, 200.0),
            None
        );
        // Série principale masquée : son point n'est plus cliquable.
        let hidden = LineChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s))
            .hidden([0]);
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&hidden, px, py_primary, 300.0, 200.0),
            None,
            "point d'une série masquée : pas de clic"
        );
    }

    #[test]
    fn selected_point_draws_a_persistent_ring() {
        // Un anneau (cercle **contour**, sans segment droit) apparaît sur le point épinglé, sans survol.
        let rings = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Path { stroke: Some(_), fill: None, path, .. }
                    if !path.verbs().iter().any(|v| matches!(v, frus_core::PathVerb::LineTo(_))))
                })
                .count()
        };
        assert_eq!(
            rings(&LineChart::new([("A", 2.0), ("B", 6.0)])),
            0,
            "aucun anneau sans sélection"
        );
        assert_eq!(
            rings(&LineChart::new([("A", 2.0), ("B", 6.0)]).selected(Some((0, 0)))),
            1,
            "un anneau sur le point épinglé"
        );
        // Série masquée épinglée : pas d'anneau.
        let hidden = LineChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
            .hidden([0])
            .selected(Some((0, 0)));
        assert_eq!(rings(&hidden), 0, "pas d'anneau sur une série masquée");
    }

    #[test]
    fn hidden_series_is_not_drawn() {
        let strokes = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| {
                    matches!(
                        p,
                        Primitive::Path {
                            stroke: Some(_),
                            fill: None,
                            ..
                        }
                    )
                })
                .count()
        };
        let both = LineChart::new([("A", 2.0), ("B", 6.0)]).series(
            "x",
            Color::rgb8(200, 80, 80),
            [3.0, 1.0],
        );
        assert_eq!(strokes(&both), 2, "deux séries visibles = deux lignes");
        let hidden = LineChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(200, 80, 80), [3.0, 1.0])
            .hidden([1]);
        assert_eq!(strokes(&hidden), 1, "la série masquée n'est pas tracée");
    }

    #[test]
    fn stacked_areas_fill_a_band_per_series() {
        let make = || {
            LineChart::new([("A", 2.0), ("B", 4.0)]).series(
                "x",
                Color::rgb8(200, 80, 80),
                [3.0, 1.0],
            )
        };
        // L'échelle empilée prend le max des totaux par catégorie : max(2+3, 4+1) = 5.
        assert_eq!(make().stacked(true).stacked_max(), 5.0);
        let bands = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Path { fill: Some(_), stroke: None, path, .. }
                    if path.verbs().iter().any(|v| matches!(v, frus_core::PathVerb::LineTo(_))))
                })
                .count()
        };
        assert_eq!(
            bands(&make().stacked(true)),
            2,
            "une bande cumulée par série"
        );
        assert_eq!(
            bands(&make()),
            0,
            "sans empilage : pas de bande (multi-séries sans aire)"
        );
    }

    #[test]
    fn normalized_stacked_areas_fill_to_the_top() {
        let make = |norm: bool| {
            LineChart::new([("A", 2.0), ("B", 4.0)])
                .series("x", Color::rgb8(1, 2, 3), [3.0, 4.0])
                .stacked(true)
                .normalized(norm)
        };
        // Ordonnées du trait du bord **supérieur** (dernière bande tracée = dernière série visible).
        let top_line_ys = |chart: &LineChart| -> Vec<f32> {
            let prims = paint_line(chart, 300.0, 200.0);
            let path = prims
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Path {
                        stroke: Some(_),
                        fill: None,
                        path,
                        ..
                    } if path
                        .verbs()
                        .iter()
                        .any(|v| matches!(v, frus_core::PathVerb::LineTo(_))) =>
                    {
                        Some(path)
                    }
                    _ => None,
                })
                .expect("un trait de bord supérieur");
            path.verbs()
                .iter()
                .filter_map(|v| match v {
                    frus_core::PathVerb::MoveTo(p) | frus_core::PathVerb::LineTo(p) => Some(p.y),
                    _ => None,
                })
                .collect()
        };
        let plot_top = VALUE_SIZE + 6.0; // 18 (sans légende ni axe)

        // 100 % : le bord supérieur est plat, à 100 % (plot_top) pour chaque catégorie.
        let ys = top_line_ys(&make(true));
        assert!(
            ys.iter().all(|y| (y - plot_top).abs() < 1.0),
            "bord haut a 100% partout, obtenu {ys:?}"
        );
        // Absolu : le bord supérieur suit les totaux (pas tous à plot_top).
        let ys_abs = top_line_ys(&make(false));
        assert!(
            ys_abs.iter().any(|y| (y - plot_top).abs() > 5.0),
            "bord haut suit les totaux, obtenu {ys_abs:?}"
        );
    }

    #[test]
    fn normalized_line_tooltip_shows_percentages() {
        // Aires empilées ; en 100 %, l'infobulle de survol ajoute la part (%) de chaque série.
        let make = |norm: bool| {
            LineChart::new([("A", 2.0), ("B", 6.0)])
                .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
                .stacked(true)
                .normalized(norm)
        };
        let tooltip_texts = |chart: &LineChart| -> Vec<String> {
            let mut scene = Scene::new();
            let status = Status {
                hover_cursor: Some(Point::new(75.0, 90.0)),
                ..Default::default()
            };
            Widget::<()>::paint(
                chart,
                Rect::new(0.0, 0.0, 300.0, 200.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let norm = tooltip_texts(&make(true));
        assert!(
            norm.iter().any(|t| t.contains("(50%)")),
            "part en % dans l'infobulle 100%, obtenu {norm:?}"
        );
        let abs = tooltip_texts(&make(false));
        assert!(
            !abs.iter().any(|t| t.contains('%')),
            "pas de % en absolu, obtenu {abs:?}"
        );
    }

    #[test]
    fn stacked_areas_label_each_band_with_value_or_percentage() {
        // Absolu : chaque bande assez épaisse porte sa valeur à chaque catégorie.
        let abs = LineChart::new([("A", 3.0), ("B", 5.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 6.0])
            .stacked(true);
        let prims = paint_line(&abs, 300.0, 260.0);
        let has = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Valeurs de bandes : série 0 = 3,5 ; série x = 4,6.
        assert!(
            has("3") && has("4") && has("5") && has("6"),
            "valeurs de bandes affichées"
        );
        // 100 % : parts en %.
        let norm = LineChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
            .stacked(true)
            .normalized(true);
        let np = paint_line(&norm, 300.0, 260.0);
        assert!(
            np.iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text.ends_with('%'))),
            "parts en % affichées dans les bandes"
        );
    }

    #[test]
    fn max_value_spans_all_series() {
        // L'échelle englobe la série additionnelle (max 9 > max principal 6).
        let chart = LineChart::<()>::new([("A", 2.0), ("B", 6.0)]).series(
            "x",
            Color::rgb8(1, 2, 3),
            [9.0, 1.0],
        );
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
                    Primitive::Path {
                        fill: Some(_),
                        stroke: None,
                        path,
                        ..
                    } => path
                        .verbs()
                        .iter()
                        .any(|v| matches!(v, frus_core::PathVerb::LineTo(_))),
                    _ => false,
                })
                .count()
        };
        assert_eq!(
            filled_polygons(&LineChart::new([("A", 2.0), ("B", 6.0)])),
            0,
            "aucune aire par défaut"
        );
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
        assert!(
            thin_lines >= 5,
            "4 lignes de grille + ligne de base, obtenu {thin_lines}"
        );
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Graduations de l'axe : 0 (base) et 8 (haut) au moins.
        assert!(
            has_text("0") && has_text("8"),
            "graduations de l'axe des ordonnées"
        );
    }

    #[test]
    fn no_grid_by_default_keeps_full_width() {
        // Sans grille, aucune graduation « 0 » n'est dessinée (comportement d'origine).
        let chart = LineChart::new([("A", 2.0), ("B", 8.0)]);
        let prims = paint_line(&chart, 300.0, 200.0);
        let has_zero = prims
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "0"));
        assert!(!has_zero, "pas d'axe par défaut");
    }
}
