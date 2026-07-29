//! [`Steps`] : un **indicateur d'étapes** (fil d'Ariane numéroté d'un formulaire
//! multi-étapes / assistant), façon `Stepper` de Material.
//!
//! Une rangée de marqueurs ronds numérotés reliés par des connecteurs, chacun dans
//! l'un de trois états : **terminé** (coche, accent), **courant** (numéro, accent),
//! **à venir** (numéro, surface bordée). Un libellé sous chaque marqueur.
//!
//! Le widget est **purement visuel** : la navigation (Suivant/Précédent) et la
//! validation par étape restent applicatives (un [`crate::form::Form`] par étape,
//! des boutons qui changent l'étape courante). Le nom `Stepper` étant déjà pris par
//! le sélecteur numérique −/valeur/+, cet indicateur s'appelle `Steps`.

use frus_core::{Color, Point, Rect, Role, Scene, Semantics};
use frus_layout::{Align, Dimension, FlexDirection, Justify, Style};

use crate::flex::Flex;
use crate::icons::IconName;
use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Diamètre d'un marqueur (rond).
const MARKER_D: f32 = 28.0;
/// Rayon d'un marqueur.
const R: f32 = MARKER_D / 2.0;
/// Écart marqueur → libellé.
const LABEL_GAP: f32 = 8.0;
/// Taille de police d'un libellé d'étape.
const LABEL_SIZE: f32 = 12.0;
/// Taille de police du numéro dans un marqueur.
const NUM_SIZE: f32 = 14.0;
/// Hauteur totale de l'indicateur (marqueur + libellé).
const HEIGHT: f32 = 56.0;

/// Indicateur d'étapes d'un formulaire multi-étapes.
///
/// ```
/// use frus_widgets::Steps;
/// // Trois étapes, la deuxième en cours (la première est donc « terminée »).
/// let steps: Steps<()> = Steps::new(["Account", "Profile", "Review"]).current(1);
/// ```
pub struct Steps<Msg> {
    labels: Vec<String>,
    current: usize,
    /// Couleur d'accent surchargée ; `None` = `primary` du thème.
    color: Option<Color>,
    /// Masque « terminé » **explicite** par étape (validité). Vide → règle par défaut
    /// (`i < current`, cf. [`completed`](Self::completed)).
    completed: Vec<bool>,
    /// Vide, ou **une** rangée de zones cliquables (hotspots) superposée aux marqueurs quand
    /// [`on_tap`](Self::on_tap) est fourni.
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> Steps<Msg> {
    /// Crée un indicateur depuis les libellés d'étapes ; l'étape courante est la première.
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            current: 0,
            color: None,
            completed: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Marque explicitement les étapes **terminées** (une coche) par un drapeau par étape —
    /// typiquement la **validité** de chaque étape, plutôt que la seule position. Sans cet appel,
    /// une étape est « terminée » si elle précède l'étape courante (`i < current`).
    pub fn completed(mut self, flags: impl IntoIterator<Item = bool>) -> Self {
        self.completed = flags.into_iter().collect();
        self
    }

    /// Fixe l'étape **courante** (les précédentes sont « terminées », les suivantes « à venir »).
    /// Bornée au dernier index.
    pub fn current(mut self, index: usize) -> Self {
        self.current = index.min(self.labels.len().saturating_sub(1));
        self
    }

    /// Surcharge la couleur d'accent (marqueurs terminés/courant + connecteurs franchis).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Rend les marqueurs **cliquables** : cliquer le marqueur de l'étape `i` émet `on_tap(i)`
    /// (pour y sauter — typiquement une étape déjà visitée). Superpose une rangée de zones
    /// cliquables transparentes alignées **exactement** sur les marqueurs (répartition
    /// `SpaceBetween` de boîtes de la taille d'un marqueur), sans changer le rendu.
    pub fn on_tap(mut self, on_tap: impl Fn(usize) -> Msg) -> Self {
        let mut row: Flex<Msg> = Flex::row().justify(Justify::SpaceBetween);
        for (i, label) in self.labels.iter().enumerate() {
            row = row.child(Hotspot { label: label.clone(), message: on_tap(i) });
        }
        self.children = vec![Box::new(row)];
        self
    }
}

impl<Msg> Steps<Msg> {
    /// L'étape `i` est-elle **terminée** ? Masque explicite s'il est fourni, sinon `i < current`.
    fn is_done(&self, i: usize) -> bool {
        if self.completed.is_empty() {
            i < self.current
        } else {
            self.completed.get(i).copied().unwrap_or(false)
        }
    }

    /// Abscisse du centre du marqueur `i` dans `bounds` (marqueurs répartis d'un bord à l'autre).
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
            // Une éventuelle rangée de hotspots occupe le haut (bande des marqueurs).
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

        // Connecteurs (sous les marqueurs) : franchis (accent) jusqu'à l'étape courante, sinon bord.
        for i in 0..n.saturating_sub(1) {
            let x0 = self.center_x(bounds, i) + R;
            let x1 = self.center_x(bounds, i + 1) - R;
            let col = if self.is_done(i) { accent } else { theme.border };
            let rect = Rect::new(x0, cy - 1.0, (x1 - x0).max(0.0), 2.0);
            scene.draw_rect(rect, col.fade(o), 0.0, 0.0, Color::TRANSPARENT);
        }

        // Marqueurs + numéros/coches + libellés.
        for i in 0..n {
            let cx = self.center_x(bounds, i);
            let rect = Rect::new(cx - R, cy - R, MARKER_D, MARKER_D);
            let current = i == self.current;
            // L'étape courante montre son numéro (même si valide) ; les autres, une coche si
            // terminées (validité), sinon leur numéro.
            let completed = !current && self.is_done(i);

            let (fill, bw, bc) = if completed || current {
                (accent, 0.0, Color::TRANSPARENT)
            } else {
                (theme.surface, 1.5, theme.border)
            };
            scene.draw_rect(rect, fill.fade(o), R, bw, bc.fade(o));

            if completed {
                // Coche (icône 16 px centrée) sur fond accent.
                let path = IconName::Check.path().scaled(16.0 / 24.0).translated(cx - 8.0, cy - 8.0);
                scene.fill_path(&path, theme.on_primary.fade(o));
            } else {
                let num = (i + 1).to_string();
                let m = frus_text::measure(&num, NUM_SIZE);
                let color = if current { theme.on_primary } else { theme.on_surface };
                let p = Point::new(cx - m.width * 0.5, cy - m.height * 0.5);
                scene.text(p, num, NUM_SIZE, color.fade(o));
            }

            // Libellé sous le marqueur, centré ; atténué hors étape courante.
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

/// Zone cliquable **transparente** de la taille d'un marqueur, superposée à celui-ci quand
/// [`Steps::on_tap`] est utilisé : elle ne dessine rien mais capte le clic (et le focus clavier)
/// pour sauter à l'étape correspondante.
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
        Some(Semantics::new(Role::Button).label(self.label.clone()).clickable())
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
        // 4 étapes, la 3e (index 2) courante : 0,1 terminées ; 2 courante ; 3 à venir.
        let prims = paint_steps(&Steps::<()>::new(["A", "B", "C", "D"]).current(2));
        let has_text = |t: &str| {
            prims.iter().any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Terminées → coches (pas de numéro) ; courante → « 3 » ; à venir → « 4 ».
        assert!(has_text("3") && has_text("4"), "numéros de l'étape courante et à venir");
        assert!(!has_text("1") && !has_text("2"), "les étapes terminées montrent une coche");
        // Une coche (chemin rempli) par étape terminée.
        let checks = prims.iter().filter(|p| matches!(p, Primitive::Path { fill: Some(_), .. })).count();
        assert_eq!(checks, 2, "deux coches pour les deux étapes terminées");
        // Tous les libellés sont dessinés.
        assert!(has_text("A") && has_text("B") && has_text("C") && has_text("D"));
    }

    #[test]
    fn completed_mask_overrides_position() {
        // Sans masque : « terminé » = position (i < current).
        let default = Steps::<()>::new(["A", "B", "C"]).current(2);
        assert!(default.is_done(0) && default.is_done(1));
        assert!(!default.is_done(2), "l'étape courante n'est pas terminée par défaut");
        // Avec masque (validité) : indépendant de la position.
        let masked = Steps::<()>::new(["A", "B", "C"]).current(1).completed([false, false, true]);
        assert!(!masked.is_done(0), "étape 0 invalide → non terminée malgré i < current");
        assert!(masked.is_done(2), "étape 2 valide → terminée bien que i > current");
        // Masque plus court que le nombre d'étapes : les manquants sont non terminés.
        let short = Steps::<()>::new(["A", "B", "C"]).completed([true]);
        assert!(short.is_done(0) && !short.is_done(1) && !short.is_done(2));
    }

    #[test]
    fn on_tap_overlays_clickable_hotspots() {
        #[derive(Clone, Debug, PartialEq)]
        enum Msg {
            Go(usize),
        }
        // Sans on_tap : aucun enfant (purement visuel).
        let plain = Steps::<Msg>::new(["A", "B", "C"]).current(1);
        assert!(Widget::<Msg>::children(&plain).is_empty());
        // Avec on_tap : une rangée d'enfants dont chaque marqueur émet son index.
        let tappable = Steps::new(["A", "B", "C"]).current(1).on_tap(Msg::Go);
        let row = Widget::<Msg>::children(&tappable);
        assert_eq!(row.len(), 1, "une seule rangée de hotspots");
        let spots = row[0].children();
        assert_eq!(spots.len(), 3, "un hotspot par étape");
        assert_eq!(spots[0].on_click(), Some(Msg::Go(0)));
        assert_eq!(spots[2].on_click(), Some(Msg::Go(2)));
        assert!(spots[0].focusable(), "un hotspot est focalisable");
    }
}
