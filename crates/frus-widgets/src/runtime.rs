//! État retenu au runtime entre les frames, **clé par identité de widget**.
//!
//! La *valeur* d'un champ reste contrôlée (état applicatif) ; ce qui vit ici est
//! l'état d'**interaction/édition** propre aux widgets : survol/focus, offsets de
//! défilement, et position curseur/sélection des champs. C'est la fondation
//! d'une reconciliation par identité (posée au Jalon 6).

use std::cell::RefCell;
use std::collections::HashMap;

use frus_core::{BorderRadius, Color, Curve, Insets, Primitive, Size};

use crate::interaction::{InputState, WidgetId};
use crate::relayout::LayoutCache;

/// Offsets de défilement `(x, y)`, par zone défilable.
pub type ScrollState = HashMap<WidgetId, (f32, f32)>;

/// État d'édition d'un champ de saisie : curseur + ancre de sélection.
///
/// Les indices sont en **caractères**. Ils peuvent dépasser la longueur de la
/// valeur (p. ex. `usize::MAX` pour « fin ») : les widgets les bornent à l'usage.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    /// Position du curseur.
    pub cursor: usize,
    /// Ancre de sélection (`None` = pas de sélection).
    pub anchor: Option<usize>,
    /// Plage `(début, fin)` en **cours de composition** IME (texte provisoire,
    /// souligné à l'écran) ; `None` hors composition. En indices de caractères.
    pub composing: Option<(usize, usize)>,
}

impl Edit {
    /// Plage sélectionnée `(début, fin)`, non vide, sinon `None`.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.anchor
            .map(|anchor| (anchor.min(self.cursor), anchor.max(self.cursor)))
            .filter(|(start, end)| start < end)
    }
}

/// Durée **par défaut** des transitions, en secondes. Un widget peut la régler
/// via [`crate::widget::Widget::anim_duration`].
pub(crate) const ANIM_DURATION: f32 = 0.12;

/// Raideur du ressort de défilement (px·s⁻²).
const SCROLL_K: f32 = 200.0;
/// Amortissement du ressort de défilement.
const SCROLL_C: f32 = 28.0;
/// Rappel élastique de la cible vers les bornes valides (par seconde) — rebond.
const SCROLL_RETRACT: f32 = 14.0;

/// Un axe de défilement : rappel élastique de la cible dans `[0, max]`, puis
/// ressort de l'offset courant vers cette cible. Renvoie
/// `(offset, vitesse, cible, en_mouvement)`.
fn scroll_axis(current: f32, vel: f32, target: f32, max: f32, dt: f32) -> (f32, f32, f32, bool) {
    let clamp_t = target.clamp(0.0, max);
    // La cible est ramenée vers la borne valide (dépassement → rebond).
    let target = target + (clamp_t - target) * (1.0 - (-SCROLL_RETRACT * dt).exp());
    let (offset, vel, _) = spring_step(current, vel, target, dt, SCROLL_K, SCROLL_C);
    // Seuils en pixels (spring_step est calibré en fractions).
    let moving = (offset - target).abs() > 0.5 || vel.abs() > 2.0 || (target - clamp_t).abs() > 0.5;
    if moving {
        (offset, vel, target, true)
    } else {
        (clamp_t, 0.0, clamp_t, false)
    }
}

/// Progressions d'animation d'un widget (`0.0..=1.0`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Anim {
    pub hover: f32,
    pub focus: f32,
    /// Opacité (1 au repos ; démarrée à 0 au montage pour le fondu d'apparition).
    pub opacity: f32,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            hover: 0.0,
            focus: 0.0,
            opacity: 1.0,
        }
    }
}

/// **Timeline** d'une valeur animée implicitement (`Widget::anim_target`) :
/// interpole `from → to` selon la courbe et la durée du widget. `current` est la
/// valeur restituée au paint. Un changement de cible **rebase** la timeline
/// depuis la valeur courante (départ franc et continu, façon Flutter implicit).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValueAnim {
    /// Valeur interpolée courante (ce que lit le paint).
    pub current: f32,
    /// Valeur de départ de la transition en cours.
    from: f32,
    /// Cible de la transition en cours.
    to: f32,
    /// Temps écoulé (s) depuis le début de la transition.
    elapsed: f32,
}

impl ValueAnim {
    /// Une valeur **au repos** à `v` (aucune transition en cours).
    fn settled(v: f32) -> Self {
        Self { current: v, from: v, to: v, elapsed: 0.0 }
    }
}

/// Timeline d'une **couleur** animée (`Container::animated_color`) : interpole
/// `from → to` par canal, selon la courbe et la durée du widget. Même modèle de
/// rebase que [`ValueAnim`], appliqué à une couleur.
#[derive(Copy, Clone, Debug, PartialEq)]
struct ColorAnim {
    current: Color,
    from: Color,
    to: Color,
    elapsed: f32,
}

impl ColorAnim {
    fn settled(c: Color) -> Self {
        Self { current: c, from: c, to: c, elapsed: 0.0 }
    }
}

/// Timeline d'une **taille** animée (`Container::animated_size`) : interpole
/// `from → to` (largeur/hauteur) selon la courbe et la durée du widget. La taille
/// interpolée est injectée **au layout** (pas au paint) via `effective_style`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct SizeAnim {
    current: Size,
    from: Size,
    to: Size,
    elapsed: f32,
}

impl SizeAnim {
    fn settled(s: Size) -> Self {
        Self { current: s, from: s, to: s, elapsed: 0.0 }
    }
}

/// Interpolation linéaire de deux tailles (par composante).
fn lerp_size(a: Size, b: Size, t: f32) -> Size {
    Size::new(a.width + (b.width - a.width) * t, a.height + (b.height - a.height) * t)
}

/// Timeline d'un **rayon de coin** animé (`Container::animated_radius`) :
/// interpole `from → to` (les quatre coins) selon la courbe et la durée du
/// widget. Propriété **picturale** : livrée au paint via `Status::anim_radius`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct RadiusAnim {
    current: BorderRadius,
    from: BorderRadius,
    to: BorderRadius,
    elapsed: f32,
}

impl RadiusAnim {
    fn settled(r: BorderRadius) -> Self {
        Self { current: r, from: r, to: r, elapsed: 0.0 }
    }
}

/// Timeline d'un **padding** animé (`Container::animated_padding`) : interpole
/// `from → to` (les quatre côtés) selon la courbe et la durée du widget. La
/// marge interpolée est injectée **au layout** (`effective_style`), comme la taille.
#[derive(Copy, Clone, Debug, PartialEq)]
struct PaddingAnim {
    current: Insets,
    from: Insets,
    to: Insets,
    elapsed: f32,
}

impl PaddingAnim {
    fn settled(p: Insets) -> Self {
        Self { current: p, from: p, to: p, elapsed: 0.0 }
    }
}

/// Interpolation linéaire de deux marges (par côté).
fn lerp_insets(a: Insets, b: Insets, t: f32) -> Insets {
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Insets::new(
        mix(a.top, b.top),
        mix(a.right, b.right),
        mix(a.bottom, b.bottom),
        mix(a.left, b.left),
    )
}

/// Interpolation linéaire de deux rayons (par coin).
fn lerp_radius(a: BorderRadius, b: BorderRadius, t: f32) -> BorderRadius {
    let mix = |x: f32, y: f32| x + (y - x) * t;
    BorderRadius {
        top_left: mix(a.top_left, b.top_left),
        top_right: mix(a.top_right, b.top_right),
        bottom_right: mix(a.bottom_right, b.bottom_right),
        bottom_left: mix(a.bottom_left, b.bottom_left),
    }
}

/// Un pas de ressort amorti (Euler semi-implicite) faisant tendre `progress` vers
/// `target`, amorcé par `velocity`. `stiffness`/`damping` règlent la raideur et
/// l'amortissement (≈ `2·√stiffness` = amortissement critique, sans dépassement).
/// Renvoie `(progress, velocity, au_repos)`.
///
/// Util générique de mouvement : sert aux transitions d'écran et aux gestes
/// (détente amorcée par la vélocité du doigt).
pub fn spring_step(
    progress: f32,
    velocity: f32,
    target: f32,
    dt: f32,
    stiffness: f32,
    damping: f32,
) -> (f32, f32, bool) {
    let accel = stiffness * (target - progress) - damping * velocity;
    let velocity = velocity + accel * dt;
    let progress = progress + velocity * dt;
    let at_rest = (progress - target).abs() < 0.004 && velocity.abs() < 0.06;
    (progress, velocity, at_rest)
}

/// Courbe en **ressort** (réponse indicielle d'un ressort en amortissement
/// **critique**) remappant une progression linéaire `t ∈ [0,1]` : départ au
/// repos (pente nulle), montée franche, arrivée douce **sans dépassement** —
/// même sensation que les transitions d'écran, mais sous forme fermée (pas
/// d'état de vélocité). `f(0) = 0`, `f(1) = 1`, monotone croissante.
pub fn spring_ease(t: f32) -> f32 {
    // Réponse critique (`omega = 8`), désormais fournie par la couche d'animation
    // partagée de `frus-core` : une seule source de vérité pour cette courbe.
    frus_core::Curve::critical_spring().transform(t)
}

/// Fait tendre `value` vers `target` par pas de `step` ; note si ça bouge encore.
fn approach(value: &mut f32, target: f32, step: f32, animating: &mut bool) {
    if *value < target {
        *value = (*value + step).min(target);
    } else if *value > target {
        *value = (*value - step).max(target);
    }
    if (*value - target).abs() > 1e-3 {
        *animating = true;
    }
}

/// Contexte runtime transmis à `build_ui` : tout l'état retenu entre frames.
#[derive(Default)]
pub struct Runtime {
    /// Survol / pression / focus.
    pub input: InputState,
    /// Offsets de défilement **courants** (rendus), par zone.
    pub scroll: ScrollState,
    /// Offsets de défilement **visés** (le ressort y tend), par zone.
    pub scroll_target: ScrollState,
    /// Vitesse de défilement (pour le ressort), par zone.
    pub scroll_velocity: ScrollState,
    /// État d'édition, par champ de saisie.
    pub edits: HashMap<WidgetId, Edit>,
    /// Progressions d'animation (survol/focus/opacité), par widget.
    pub anims: HashMap<WidgetId, Anim>,
    /// Valeurs animées propres aux widgets (`Widget::anim_target`), par widget —
    /// chacune une **timeline** courbée (voir [`ValueAnim`]).
    pub values: HashMap<WidgetId, ValueAnim>,
    /// Couleurs de fond animées (`Container::animated_color`), par widget.
    colors: HashMap<WidgetId, ColorAnim>,
    /// Tailles animées (`Container::animated_size`), par widget — injectées au layout.
    sizes: HashMap<WidgetId, SizeAnim>,
    /// Rayons de coin animés (`Container::animated_radius`), par widget.
    radii: HashMap<WidgetId, RadiusAnim>,
    /// Marges animées (`Container::animated_padding`), par widget — injectées au layout.
    paddings: HashMap<WidgetId, PaddingAnim>,
    /// Widgets présents à la frame précédente (pour détecter les montages).
    pub mounted: std::collections::HashSet<WidgetId>,
    /// Instantanés des sous-arbres sortants, en cours de fondu de sortie :
    /// clé d'événement → (primitives capturées, opacité restante `1 → 0`).
    pub leaving: HashMap<u64, (Vec<Primitive>, f32)>,
    /// Temps écoulé (secondes) depuis le démarrage, pour les animations continues.
    pub time: f32,
    /// La dernière interaction était-elle **clavier** ? L'anneau de focus
    /// générique n'est peint que dans ce cas (`FocusHighlightMode` : un clic ne
    /// doit pas faire flasher d'anneau). Le focus lui-même reste actif.
    pub focus_visible: bool,
    /// Cache de frontière de relayout (rectangles retenus par racine de layout,
    /// d'une frame à l'autre). Mutabilité intérieure : `build_ui` le met à jour
    /// tout en ne tenant qu'une référence partagée au `Runtime`.
    pub layout_cache: RefCell<LayoutCache>,
    /// Cache de frontière de **repaint** (primitives + interactions retenues par
    /// frontière, d'une frame à l'autre). Même mutabilité intérieure.
    pub paint_cache: RefCell<crate::paintcache::PaintCache>,
}

impl Runtime {
    /// Progression de survol animée d'un widget.
    pub fn hover_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.hover).unwrap_or(0.0)
    }

    /// Progression de focus animée d'un widget.
    pub fn focus_progress(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.focus).unwrap_or(0.0)
    }

    /// Opacité animée d'un widget (1 par défaut).
    pub fn opacity(&self, id: WidgetId) -> f32 {
        self.anims.get(&id).map(|a| a.opacity).unwrap_or(1.0)
    }

    /// Valeur animée d'un widget (0 par défaut).
    pub fn value(&self, id: WidgetId) -> f32 {
        self.values.get(&id).map(|v| v.current).unwrap_or(0.0)
    }

    /// Valeur animée d'un widget, ou `default` si **aucune** valeur n'est encore
    /// enregistrée (widget jamais animé — p. ex. rendu isolé sans boucle). Permet
    /// d'adopter la cible immédiatement, comme au montage.
    pub fn value_or(&self, id: WidgetId, default: f32) -> f32 {
        self.values.get(&id).map(|v| v.current).unwrap_or(default)
    }

    /// Fixe la valeur animée d'un widget à `v` (au repos, aucune transition en
    /// cours) — pour les rendus/tests isolés qui veulent une progression précise
    /// sans dérouler l'animation.
    pub fn set_value(&mut self, id: WidgetId, v: f32) {
        self.values.insert(id, ValueAnim::settled(v));
    }

    /// Fait tendre chaque valeur animée vers la cible déclarée par son widget
    /// (`Widget::anim_target`). Un widget vu pour la **première** fois adopte sa
    /// cible sans transition (pas d'animation au montage). Renvoie `true` s'il
    /// reste une valeur en mouvement.
    pub fn advance_values<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            out: &mut Vec<(WidgetId, f32, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_target() {
                out.push((id, target, widget.anim_duration().max(0.0), widget.anim_curve()));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(child.as_ref(), crate::ui::child_id(id, index, child.as_ref()), out);
            }
        }
        let mut targets: Vec<(WidgetId, f32, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

        // Oublie les valeurs des widgets disparus.
        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.values.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.values.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let v = e.get_mut();
                    // Nouvelle cible : rebase la timeline depuis la valeur courante.
                    if v.to != target {
                        v.from = v.current;
                        v.to = target;
                        v.elapsed = 0.0;
                    }
                    if v.from == v.to {
                        v.current = v.to;
                    } else {
                        v.elapsed += dt;
                        let t = if duration > 0.0 {
                            (v.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        v.current = v.from + (v.to - v.from) * curve.transform(t);
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    // Montage : adopte la cible sans transition.
                    e.insert(ValueAnim::settled(target));
                }
            }
        }
        animating
    }

    /// Couleur de fond animée d'un widget, si en transition (`None` sinon).
    pub fn anim_color(&self, id: WidgetId) -> Option<Color> {
        self.colors.get(&id).map(|c| c.current)
    }

    /// Fait tendre chaque couleur de fond animée vers la cible déclarée par son
    /// widget (`Widget::anim_color`), suivant sa durée/courbe (`anim_duration`/
    /// `anim_curve`). Montage : adopte la cible sans transition. Renvoie `true`
    /// s'il reste une couleur en mouvement. Même modèle que [`Self::advance_values`].
    pub fn advance_colors<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            out: &mut Vec<(WidgetId, Color, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_color() {
                out.push((id, target, widget.anim_duration().max(0.0), widget.anim_curve()));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(child.as_ref(), crate::ui::child_id(id, index, child.as_ref()), out);
            }
        }
        let mut targets: Vec<(WidgetId, Color, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.colors.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.colors.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let c = e.get_mut();
                    if c.to != target {
                        c.from = c.current;
                        c.to = target;
                        c.elapsed = 0.0;
                    }
                    if c.from == c.to {
                        c.current = c.to;
                    } else {
                        c.elapsed += dt;
                        let t = if duration > 0.0 {
                            (c.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        c.current = c.from.lerp(c.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(ColorAnim::settled(target));
                }
            }
        }
        animating
    }

    /// Taille animée d'un widget, si en transition (`None` sinon).
    pub fn anim_size(&self, id: WidgetId) -> Option<Size> {
        self.sizes.get(&id).map(|s| s.current)
    }

    /// Fait tendre chaque taille animée vers la cible déclarée par son widget
    /// (`Widget::anim_size`), suivant sa durée/courbe. Montage : adopte la cible
    /// sans transition. Renvoie `true` s'il reste une taille en mouvement. Même
    /// modèle que [`Self::advance_values`], mais la sortie est **consommée au
    /// layout** (`effective_style`), pas au paint.
    pub fn advance_sizes<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            out: &mut Vec<(WidgetId, Size, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_size() {
                out.push((id, target, widget.anim_duration().max(0.0), widget.anim_curve()));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(child.as_ref(), crate::ui::child_id(id, index, child.as_ref()), out);
            }
        }
        let mut targets: Vec<(WidgetId, Size, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.sizes.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.sizes.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let s = e.get_mut();
                    if s.to != target {
                        s.from = s.current;
                        s.to = target;
                        s.elapsed = 0.0;
                    }
                    if s.from == s.to {
                        s.current = s.to;
                    } else {
                        s.elapsed += dt;
                        let t = if duration > 0.0 {
                            (s.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        s.current = lerp_size(s.from, s.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(SizeAnim::settled(target));
                }
            }
        }
        animating
    }

    /// Rayon de coin animé d'un widget, si en transition (`None` sinon).
    pub fn anim_radius(&self, id: WidgetId) -> Option<BorderRadius> {
        self.radii.get(&id).map(|r| r.current)
    }

    /// Fait tendre chaque rayon de coin animé vers la cible déclarée par son
    /// widget (`Widget::anim_radius`), suivant sa durée/courbe. Montage : adopte
    /// la cible sans transition. Renvoie `true` s'il reste un rayon en mouvement.
    /// Même modèle que [`Self::advance_colors`], appliqué à un [`BorderRadius`].
    pub fn advance_radii<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            out: &mut Vec<(WidgetId, BorderRadius, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_radius() {
                out.push((id, target, widget.anim_duration().max(0.0), widget.anim_curve()));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(child.as_ref(), crate::ui::child_id(id, index, child.as_ref()), out);
            }
        }
        let mut targets: Vec<(WidgetId, BorderRadius, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.radii.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.radii.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let r = e.get_mut();
                    if r.to != target {
                        r.from = r.current;
                        r.to = target;
                        r.elapsed = 0.0;
                    }
                    if r.from == r.to {
                        r.current = r.to;
                    } else {
                        r.elapsed += dt;
                        let t = if duration > 0.0 {
                            (r.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        r.current = lerp_radius(r.from, r.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(RadiusAnim::settled(target));
                }
            }
        }
        animating
    }

    /// Marge (padding) animée d'un widget, si en transition (`None` sinon).
    pub fn anim_padding(&self, id: WidgetId) -> Option<Insets> {
        self.paddings.get(&id).map(|p| p.current)
    }

    /// Fait tendre chaque marge animée vers la cible déclarée par son widget
    /// (`Widget::anim_padding`), suivant sa durée/courbe. Montage : adopte la
    /// cible sans transition. Renvoie `true` s'il reste une marge en mouvement.
    /// Comme la taille, la sortie est **consommée au layout** (`effective_style`).
    pub fn advance_paddings<Msg>(&mut self, root: &dyn crate::widget::Widget<Msg>, dt: f32) -> bool {
        fn collect<Msg>(
            widget: &dyn crate::widget::Widget<Msg>,
            id: WidgetId,
            out: &mut Vec<(WidgetId, Insets, f32, Curve)>,
        ) {
            if let Some(target) = widget.anim_padding() {
                out.push((id, target, widget.anim_duration().max(0.0), widget.anim_curve()));
            }
            for (index, child) in widget.children().iter().enumerate() {
                collect(child.as_ref(), crate::ui::child_id(id, index, child.as_ref()), out);
            }
        }
        let mut targets: Vec<(WidgetId, Insets, f32, Curve)> = Vec::new();
        collect(root, WidgetId::ROOT, &mut targets);

        let present: std::collections::HashSet<WidgetId> =
            targets.iter().map(|(id, ..)| *id).collect();
        self.paddings.retain(|id, _| present.contains(id));

        let mut animating = false;
        for (id, target, duration, curve) in targets {
            match self.paddings.entry(id) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let p = e.get_mut();
                    if p.to != target {
                        p.from = p.current;
                        p.to = target;
                        p.elapsed = 0.0;
                    }
                    if p.from == p.to {
                        p.current = p.to;
                    } else {
                        p.elapsed += dt;
                        let t = if duration > 0.0 {
                            (p.elapsed / duration).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        p.current = lerp_insets(p.from, p.to, curve.transform(t));
                        if t < 1.0 {
                            animating = true;
                        }
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(PaddingAnim::settled(target));
                }
            }
        }
        animating
    }

    /// Fait avancer les transitions (survol/focus) de `dt` secondes vers leurs
    /// cibles. Renvoie `true` si au moins une animation est encore en cours.
    pub fn advance(&mut self, dt: f32) -> bool {
        let hovered = self.input.hovered;
        let focused = self.input.focused;
        if let Some(id) = hovered {
            self.anims.entry(id).or_default();
        }
        if let Some(id) = focused {
            self.anims.entry(id).or_default();
        }

        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        let mut animating = false;

        self.anims.retain(|id, anim| {
            let hover_target = if Some(*id) == hovered { 1.0 } else { 0.0 };
            let focus_target = if Some(*id) == focused { 1.0 } else { 0.0 };
            approach(&mut anim.hover, hover_target, step, &mut animating);
            approach(&mut anim.focus, focus_target, step, &mut animating);
            // L'opacité tend toujours vers 1 (fondu d'apparition).
            approach(&mut anim.opacity, 1.0, step, &mut animating);
            // On oublie les entrées entièrement au repos (rien à animer).
            !(hover_target == 0.0
                && focus_target == 0.0
                && anim.hover <= 0.0
                && anim.focus <= 0.0
                && anim.opacity >= 1.0)
        });

        animating
    }

    /// Fait tendre chaque offset de défilement **courant** vers sa **cible** par
    /// un ressort (défilement lissé), avec rappel élastique aux bords (rebond).
    /// `maxes` fournit `(max_x, max_y)` par zone (issus de la dernière frame).
    /// Renvoie `true` s'il reste un défilement en mouvement.
    pub fn advance_scroll(&mut self, maxes: &[(WidgetId, f32, f32)], dt: f32) -> bool {
        let ids: Vec<WidgetId> = self.scroll_target.keys().copied().collect();
        let mut animating = false;
        for id in ids {
            let (max_x, max_y) = maxes
                .iter()
                .find(|(i, _, _)| *i == id)
                .map(|(_, x, y)| (*x, *y))
                .unwrap_or((0.0, 0.0));
            let current = self.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
            let target = self.scroll_target.get(&id).copied().unwrap_or(current);
            let vel = self.scroll_velocity.get(&id).copied().unwrap_or((0.0, 0.0));

            let (cx, vx, tx, ax) = scroll_axis(current.0, vel.0, target.0, max_x, dt);
            let (cy, vy, ty, ay) = scroll_axis(current.1, vel.1, target.1, max_y, dt);

            self.scroll.insert(id, (cx, cy));
            if ax || ay {
                self.scroll_target.insert(id, (tx, ty));
                self.scroll_velocity.insert(id, (vx, vy));
                animating = true;
            } else {
                // Au repos : on nettoie l'état d'animation (l'offset courant reste).
                self.scroll_target.remove(&id);
                self.scroll_velocity.remove(&id);
            }
        }
        animating
    }

    /// Fait décroître l'opacité des sous-arbres sortants ; oublie ceux arrivés à
    /// 0. Renvoie `true` s'il reste une sortie en cours.
    pub fn advance_leaving(&mut self, dt: f32) -> bool {
        let step = if ANIM_DURATION > 0.0 {
            dt / ANIM_DURATION
        } else {
            1.0
        };
        let mut animating = false;
        self.leaving.retain(|_, (_, opacity)| {
            *opacity -= step;
            if *opacity > 0.0 {
                animating = true;
                true
            } else {
                false
            }
        });
        animating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_rises_then_falls_and_clears() {
        let id = WidgetId::ROOT.child(0);
        let mut rt = Runtime::default();
        rt.input.hovered = Some(id);

        // Survolé : petites étapes → la progression monte sans atteindre 1.
        assert!(rt.advance(0.03)); // ~0.25, encore en cours
        assert!(rt.advance(0.03)); // ~0.5, encore en cours
        let p = rt.hover_progress(id);
        assert!(p > 0.4 && p < 0.6, "progression = {p}");

        // Grand pas : atteint 1.0 puis y reste (plus d'animation).
        rt.advance(1.0);
        assert_eq!(rt.hover_progress(id), 1.0);
        assert!(!rt.advance(0.03));

        // Fin du survol : redescend (en cours), puis arrive à 0 et l'entrée disparaît.
        rt.input.hovered = None;
        assert!(rt.advance(0.03));
        rt.advance(1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
        assert!(rt.anims.is_empty());
    }

    #[test]
    fn focus_animates_independently() {
        let id = WidgetId::ROOT.child(1);
        let mut rt = Runtime::default();
        rt.input.focused = Some(id);
        rt.advance(1.0);
        assert_eq!(rt.focus_progress(id), 1.0);
        assert_eq!(rt.hover_progress(id), 0.0);
    }

    #[test]
    fn opacity_rises_to_one() {
        let id = WidgetId::ROOT.child(2);
        let mut rt = Runtime::default();
        // Montage : démarre transparent.
        rt.anims.insert(id, Anim { opacity: 0.0, ..Default::default() });
        assert!(rt.advance(0.03));
        let o = rt.opacity(id);
        assert!(o > 0.0 && o < 1.0, "opacité = {o}");
        rt.advance(1.0);
        assert_eq!(rt.opacity(id), 1.0);
        // Défaut sans entrée : opaque.
        assert_eq!(rt.opacity(WidgetId::ROOT), 1.0);
    }

    #[test]
    fn value_snaps_on_mount_then_animates() {
        let mut rt = Runtime::default();
        // Montage d'un interrupteur off : adopte la cible (0) sans animation.
        let off: crate::Switch<()> = crate::Switch::new(false);
        assert!(!rt.advance_values(&off, 1.0));
        assert_eq!(rt.value(WidgetId::ROOT), 0.0);

        // Bascule on : la valeur monte vers 1 par petits pas.
        let on: crate::Switch<()> = crate::Switch::new(true);
        assert!(rt.advance_values(&on, 0.03));
        let v = rt.value(WidgetId::ROOT);
        assert!(v > 0.0 && v < 1.0, "valeur = {v}");
        rt.advance_values(&on, 1.0);
        assert_eq!(rt.value(WidgetId::ROOT), 1.0);

        // Widget disparu : la valeur est oubliée.
        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_values(&empty, 1.0);
        assert!(rt.values.is_empty());
    }

    /// Widget minimal exposant une valeur animée réglable (cible, durée, courbe)
    /// — pour tester la timeline sans dépendre d'un widget concret.
    struct Mock {
        target: f32,
        duration: f32,
        curve: Curve,
    }

    impl crate::widget::Widget<()> for Mock {
        fn style(&self) -> frus_layout::Style {
            frus_layout::Style::default()
        }
        fn children(&self) -> &[Box<dyn crate::widget::Widget<()>>] {
            &[]
        }
        fn paint(
            &self,
            _bounds: frus_core::Rect,
            _status: crate::interaction::Status,
            _theme: &crate::theme::Theme,
            _scene: &mut frus_core::Scene,
        ) {
        }
        fn on_click(&self) -> Option<()> {
            None
        }
        fn anim_target(&self) -> Option<f32> {
            Some(self.target)
        }
        fn anim_duration(&self) -> f32 {
            self.duration
        }
        fn anim_curve(&self) -> Curve {
            self.curve.clone()
        }
    }

    /// La **courbe** façonne la trajectoire : à t=0.25, un *ease-in* est en
    /// retard sur la progression linéaire, un *ease-out* en avance ; toutes
    /// convergent vers la cible.
    #[test]
    fn curve_shapes_the_value_timeline() {
        let id = WidgetId::ROOT;
        let dt = 0.03; // t = 0.25 sur une durée de 0.12
        let dur = 0.12;
        let sample = |curve: Curve| {
            let mut rt = Runtime::default();
            rt.set_value(id, 0.0);
            rt.advance_values(&Mock { target: 1.0, duration: dur, curve }, dt);
            (rt.value(id), rt)
        };
        let (ein, mut rt_in) = sample(Curve::ease_in());
        let (eout, mut rt_out) = sample(Curve::ease_out());
        let (lin, mut rt_lin) = sample(Curve::Linear);

        assert!((lin - 0.25).abs() < 1e-3, "linéaire = t : {lin}");
        assert!(ein < 0.25, "ease-in en retard : {ein}");
        assert!(eout > 0.25, "ease-out en avance : {eout}");

        // Grand pas : toutes atteignent la cible (les courbes finissent à 1).
        for rt in [&mut rt_in, &mut rt_out, &mut rt_lin] {
            rt.advance_values(&Mock { target: 1.0, duration: dur, curve: Curve::Linear }, 1.0);
        }
        assert_eq!(rt_in.value(id), 1.0);
        assert_eq!(rt_out.value(id), 1.0);
        assert_eq!(rt_lin.value(id), 1.0);
    }

    /// La couleur animée **snap** au montage puis **tween** au changement de
    /// cible, canal par canal ; le widget disparu est oublié.
    #[test]
    fn animated_color_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let red = Color::rgb(1.0, 0.0, 0.0);
        let blue = Color::rgb(0.0, 0.0, 1.0);
        let mut rt = Runtime::default();

        // Montage au rouge : adopte la cible sans transition.
        let start: crate::Container<()> = crate::Container::new().animated_color(red, 0.10, Curve::Linear);
        assert!(!rt.advance_colors(&start, 1.0));
        assert_eq!(rt.anim_color(id), Some(red));

        // Cible bleue : tween linéaire, à mi-parcours ≈ (0.5, 0, 0.5).
        let to_blue: crate::Container<()> = crate::Container::new().animated_color(blue, 0.10, Curve::Linear);
        assert!(rt.advance_colors(&to_blue, 0.05));
        let mid = rt.anim_color(id).unwrap();
        assert!((mid.r - 0.5).abs() < 0.05 && (mid.b - 0.5).abs() < 0.05, "mi-parcours = {mid:?}");

        // Fin : atteint le bleu.
        rt.advance_colors(&to_blue, 1.0);
        assert_eq!(rt.anim_color(id), Some(blue));

        // Widget disparu : la couleur est oubliée.
        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_colors(&empty, 1.0);
        assert_eq!(rt.anim_color(id), None);
    }

    /// La taille animée **snap** au montage puis **tween** au changement de
    /// cible (largeur/hauteur) ; le widget disparu est oublié.
    #[test]
    fn animated_size_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();

        let small: crate::Container<()> =
            crate::Container::new().animated_size(20.0, 20.0, 0.10, Curve::Linear);
        assert!(!rt.advance_sizes(&small, 1.0));
        assert_eq!(rt.anim_size(id), Some(Size::new(20.0, 20.0)));

        // Cible 40×40 : à mi-parcours linéaire ≈ 30×30.
        let big: crate::Container<()> =
            crate::Container::new().animated_size(40.0, 40.0, 0.10, Curve::Linear);
        assert!(rt.advance_sizes(&big, 0.05));
        let mid = rt.anim_size(id).unwrap();
        assert!(
            (mid.width - 30.0).abs() < 0.5 && (mid.height - 30.0).abs() < 0.5,
            "mi-parcours = {mid:?}"
        );

        rt.advance_sizes(&big, 1.0);
        assert_eq!(rt.anim_size(id), Some(Size::new(40.0, 40.0)));

        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_sizes(&empty, 1.0);
        assert_eq!(rt.anim_size(id), None);
    }

    /// Le rayon de coin animé **snap** au montage puis **tween** au changement
    /// de cible (par coin) ; le widget disparu est oublié.
    #[test]
    fn animated_radius_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();

        let sharp: crate::Container<()> =
            crate::Container::new().animated_radius(0.0, 0.10, Curve::Linear);
        assert!(!rt.advance_radii(&sharp, 1.0));
        assert_eq!(rt.anim_radius(id), Some(BorderRadius::from(0.0)));

        // Cible 20 : à mi-parcours linéaire ≈ 10.
        let round: crate::Container<()> =
            crate::Container::new().animated_radius(20.0, 0.10, Curve::Linear);
        assert!(rt.advance_radii(&round, 0.05));
        let mid = rt.anim_radius(id).unwrap();
        assert!((mid.top_left - 10.0).abs() < 0.5, "mi-parcours = {}", mid.top_left);

        rt.advance_radii(&round, 1.0);
        assert_eq!(rt.anim_radius(id), Some(BorderRadius::from(20.0)));

        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_radii(&empty, 1.0);
        assert_eq!(rt.anim_radius(id), None);
    }

    /// La marge animée **snap** au montage puis **tween** au changement de cible
    /// (par côté) ; le widget disparu est oublié.
    #[test]
    fn animated_padding_tweens_between_frames() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();

        let p0: crate::Container<()> = crate::Container::new().animated_padding(0.0, 0.10, Curve::Linear);
        assert!(!rt.advance_paddings(&p0, 1.0));
        assert_eq!(rt.anim_padding(id), Some(Insets::uniform(0.0)));

        let p20: crate::Container<()> = crate::Container::new().animated_padding(20.0, 0.10, Curve::Linear);
        assert!(rt.advance_paddings(&p20, 0.05)); // t = 0.5 → 10
        let mid = rt.anim_padding(id).unwrap();
        assert!((mid.left - 10.0).abs() < 0.5, "mi-parcours = {}", mid.left);

        rt.advance_paddings(&p20, 1.0);
        assert_eq!(rt.anim_padding(id), Some(Insets::uniform(20.0)));

        let empty: crate::Container<()> = crate::Container::new();
        rt.advance_paddings(&empty, 1.0);
        assert_eq!(rt.anim_padding(id), None);
    }

    /// La **durée** règle la vitesse : à `dt` égal, une transition plus courte
    /// est plus avancée.
    #[test]
    fn shorter_duration_animates_faster() {
        let id = WidgetId::ROOT;
        let advance = |duration: f32| {
            let mut rt = Runtime::default();
            rt.set_value(id, 0.0);
            rt.advance_values(&Mock { target: 1.0, duration, curve: Curve::Linear }, 0.025);
            rt.value(id)
        };
        let fast = advance(0.05); // t = 0.5
        let slow = advance(0.20); // t = 0.125
        assert!(fast > slow, "courte durée plus avancée : {fast} vs {slow}");
        assert!((fast - 0.5).abs() < 1e-3, "fast = {fast}");
        assert!((slow - 0.125).abs() < 1e-3, "slow = {slow}");
    }

    #[test]
    fn spring_ease_is_monotonic_no_overshoot() {
        assert!((spring_ease(0.0) - 0.0).abs() < 1e-6);
        assert!((spring_ease(1.0) - 1.0).abs() < 1e-6);
        // Croissante et bornée à [0,1] (aucun dépassement au-delà de 1).
        let mut prev = 0.0;
        for i in 0..=100 {
            let v = spring_ease(i as f32 / 100.0);
            assert!(v >= prev - 1e-6, "décroît en {i}");
            assert!(v <= 1.0 + 1e-6, "dépasse 1 en {i}");
            prev = v;
        }
        // Déjà bien avancée à mi-parcours (arrivée douce en fin).
        assert!(spring_ease(0.5) > 0.7);
        // Bornée hors domaine.
        assert_eq!(spring_ease(-1.0), 0.0);
        assert_eq!(spring_ease(2.0), 1.0);
    }

    #[test]
    fn scroll_springs_to_target_and_settles() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        rt.scroll_target.insert(id, (0.0, 100.0));
        rt.scroll_velocity.insert(id, (0.0, 0.0));
        let maxes = [(id, 0.0, 200.0)];
        for _ in 0..600 {
            if !rt.advance_scroll(&maxes, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 100.0).abs() < 1.0, "arrivé à la cible : {y}");
        assert!(!rt.scroll_target.contains_key(&id), "état d'animation nettoyé au repos");
    }

    #[test]
    fn scroll_overshoot_rubber_bands_back_to_max() {
        let id = WidgetId::ROOT;
        let mut rt = Runtime::default();
        // Cible au-delà de la borne (dépassement) → doit revenir à max.
        rt.scroll_target.insert(id, (0.0, 240.0));
        rt.scroll_velocity.insert(id, (0.0, 0.0));
        let maxes = [(id, 0.0, 200.0)];
        for _ in 0..1000 {
            if !rt.advance_scroll(&maxes, 0.016) {
                break;
            }
        }
        let (_, y) = rt.scroll.get(&id).copied().unwrap();
        assert!((y - 200.0).abs() < 1.0, "revenu à la borne max : {y}");
    }

    #[test]
    fn leaving_fades_out_and_clears() {
        let mut rt = Runtime::default();
        rt.leaving.insert(0, (Vec::new(), 1.0));
        assert!(rt.advance_leaving(0.06)); // ~0.5, encore en cours
        assert!(!rt.advance_leaving(0.06)); // atteint 0 → nettoyé
        assert!(rt.leaving.is_empty());
    }
}
