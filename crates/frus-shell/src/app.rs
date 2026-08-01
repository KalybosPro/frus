//! Le pilote générique : implémente [`winit::application::ApplicationHandler`]
//! pour n'importe quelle [`Application`].
//!
//! Le framework possède la fenêtre, le renderer, le [`Runtime`] (état
//! d'interaction retenu : survol/focus/scroll/édition/animations), le routage des
//! entrées par hit-test, le glissement (barres, sélection, poignées, geste retour)
//! et l'horloge d'animation. L'application ne fournit que `update`/`view`/… .

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::Arc;

use web_time::Instant;

use frus_gpu::{wgpu, Renderer};
use frus_widgets::{
    build_ui, collect_ids, find_by_key, find_path, find_widget, reflow_reorder_cards,
    reflow_reorder_columns, subtree_ids, Color, Cursor as UiCursor, Edit, FocusDirection, Insets,
    Key, KeyResponse, Point, Primitive, Rect, ReorderAxis, Runtime, Scene, Size, Theme, Ui, Widget,
    WidgetId, WindowInsets,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, StartCause, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{CursorIcon, Window, WindowId};

use crate::application::Application;
use crate::gesture::{PointerEvent, PointerKind, PressRecognizer};

/// Presse-papier : `arboard` sur les plateformes de bureau, no-op sur Android et Web
/// (pas de dépendance `arboard`, qui ne s'y compile pas). Une API uniforme pour
/// que le corps du pilote reste sans `cfg`.
mod clip {
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    pub struct Clipboard(Option<arboard::Clipboard>);

    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    impl Clipboard {
        pub fn new() -> Self {
            Self(arboard::Clipboard::new().ok())
        }
        pub fn get_text(&mut self) -> Option<String> {
            self.0.as_mut().and_then(|c| c.get_text().ok())
        }
        pub fn set_text(&mut self, text: String) {
            if let Some(c) = self.0.as_mut() {
                let _ = c.set_text(text);
            }
        }
    }

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    pub struct Clipboard;

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    impl Clipboard {
        pub fn new() -> Self {
            Self
        }
        pub fn get_text(&mut self) -> Option<String> {
            None
        }
        pub fn set_text(&mut self, _text: String) {}
    }
}

/// Timers navigateur (Web) : le pendant du thread `recv_timeout` des souscriptions
/// natives. Un `setInterval` **retenu** ; son **drop** appelle `clearInterval` (et
/// libère la closure). C'est ainsi qu'une souscription `every` ticke sur le Web, où
/// il n'y a pas de thread de fond.
#[cfg(target_arch = "wasm32")]
mod web_timer {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;

    /// Un `setInterval` actif : tant que cette poignée vit, la callback est rappelée
    /// toutes les `ms` ; son drop l'annule.
    pub(crate) struct Interval {
        id: i32,
        // La closure JS doit vivre aussi longtemps que l'intervalle.
        _closure: Closure<dyn FnMut()>,
    }

    impl Interval {
        /// Programme `f` toutes les `ms` millisecondes (minimum 1 ms). `None` s'il
        /// n'y a pas de fenêtre (contexte sans DOM).
        pub(crate) fn new(ms: i32, f: impl FnMut() + 'static) -> Option<Self> {
            let window = web_sys::window()?;
            let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
            let id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    ms.max(1),
                )
                .ok()?;
            Some(Self { id, _closure: closure })
        }
    }

    impl Drop for Interval {
        fn drop(&mut self) {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(self.id);
            }
        }
    }
}

/// Poignée d'annulation d'une souscription active : son **drop** l'arrête.
/// - Natif : un `Sender<()>` — le thread de la souscription sort à son prochain réveil.
/// - Web : un `setInterval` retenu (`None` si l'installation a échoué).
#[cfg(not(target_arch = "wasm32"))]
type SubHandle = Sender<()>;
#[cfg(target_arch = "wasm32")]
type SubHandle = Option<web_timer::Interval>;

/// Vitesse de défilement (pixels par cran de molette).
const SCROLL_SPEED: f32 = 40.0;

/// Seuil de mouvement (px logiques) au-delà duquel un appui tactile devient un
/// défilement plutôt qu'un tap.
const TOUCH_SLOP: f32 = 8.0;

/// Dépassement élastique autorisé au-delà des bornes de défilement (px) — rebond.
const SCROLL_OVER: f32 = 48.0;

/// Vitesse minimale (px/s) au relâchement d'un pan pour amorcer un fling.
const PAN_FLING_MIN: f32 = 80.0;

/// Largeur (px physiques) de la zone de bord activant le geste retour.
const BACK_EDGE: f32 = 24.0;

/// Aperçu de **glisser-déposer** (réordonnancement) — géométrie de peinture. La *couleur* d'ombre
/// vient du thème (`theme.scheme.shadow`, comme le fait `Button`) ; seule la géométrie est ici, en
/// constantes nommées plutôt qu'en nombres magiques épars.
mod drag_preview {
    /// Décalage vertical de l'ombre portée du fantôme (px).
    pub const SHADOW_OFFSET_Y: f32 = 4.0;
    /// Flou de l'ombre portée du fantôme (px).
    pub const SHADOW_BLUR: f32 = 12.0;
    /// Opacité de l'ombre du fantôme.
    pub const SHADOW_ALPHA: f32 = 0.28;
    /// Opacité du bord `primary` du fantôme.
    pub const BORDER_ALPHA: f32 = 0.9;
    /// Épaisseur du bord `primary` du fantôme (px).
    pub const BORDER_WIDTH: f32 = 1.5;
    /// Léger soulèvement vertical du fantôme lors d'un glisser **horizontal** (colonnes).
    pub const LIFT_Y: f32 = -2.0;
    /// Épaisseur de la **ligne d'insertion** (aperçu vertical) (px).
    pub const INSERT_THICKNESS: f32 = 3.0;
}

/// Glissement en cours à la souris.
enum Drag {
    /// Poignée de barre de défilement.
    Scrollbar {
        id: WidgetId,
        vertical: bool,
        grab: f32,
        track_start: f32,
        track_len: f32,
        thumb_len: f32,
        max: f32,
    },
    /// Sélection de texte dans un champ (avec ses bornes, pour le placement).
    TextSelect { id: WidgetId, rect: frus_widgets::Rect },
    /// Glissement d'un widget draggable (curseur/poignée) sur son axe horizontal.
    /// `last_x` = dernière abscisse du curseur, pour livrer le **delta** aux
    /// poignées qui accumulent (redimensionnement de colonne).
    Widget { id: WidgetId, rect: frus_widgets::Rect, last_x: f32 },
    /// Réordonnancement d'une **colonne** : on saisit un en-tête (`id`, colonne
    /// `from`) et on le dépose sur une autre. `moved` distingue le glissement d'un
    /// simple tap (qui reste un tri) — sous le seuil `TOUCH_SLOP` depuis `start`.
    Reorder { id: WidgetId, from: usize, start: Point, moved: bool },
    /// Déplacement (pan) d'une fenêtre interactive (`InteractiveViewer`) : le
    /// curseur pousse le contenu. `last` = dernière position (pour le delta) ;
    /// `moved` distingue un vrai pan d'un simple tap (sous le seuil `TOUCH_SLOP`),
    /// afin de laisser un clic sur un enfant passer. `velocity` (px/s, lissée) alimente
    /// le fling au relâchement ; `viewport` borne le pan au cadre.
    Pan {
        id: WidgetId,
        last: Point,
        moved: bool,
        velocity: (f32, f32),
        last_t: Instant,
        viewport: frus_widgets::Rect,
    },
    /// Défilement d'une zone scrollable au doigt (tactile). `moved` distingue un
    /// vrai défilement d'un simple tap (mouvement sous le seuil `TOUCH_SLOP`).
    /// La vitesse (en espace **défilement**, px/s, lissée) alimente le fling.
    Scroll {
        id: WidgetId,
        last: Point,
        moved: bool,
        velocity: (f32, f32),
        last_t: Instant,
    },
    /// Geste « retour » : le framework mesure la progression et la vélocité du
    /// doigt et les transmet à l'application (qui décide de la navigation).
    Back {
        start_x: f32,
        last_x: f32,
        last_t: Instant,
        velocity: f32,
    },
}

/// Le pilote : boucle `événement → frame` autour d'une [`Application`].
pub struct App<A: Application> {
    /// L'application pilotée (état + logique).
    app: A,
    /// Canal pour réinjecter les messages produits par les effets (threads).
    proxy: EventLoopProxy<A::Message>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    /// Renderer en cours d'initialisation **asynchrone** (Web uniquement) : rempli par
    /// la future `spawn_local`, récupéré à la première frame (le Web ne peut pas
    /// bloquer sur l'init GPU).
    #[cfg(target_arch = "wasm32")]
    pending_renderer: std::rc::Rc<std::cell::RefCell<Option<Renderer>>>,
    /// Pont accessibilité (AccessKit) de la fenêtre — bureau uniquement.
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    a11y: Option<crate::a11y::A11y>,
    /// Dernière interface construite (hit-test, focus, scroll).
    ui: Option<Ui<A::Message>>,
    /// Dernier arbre de widgets construit (routage clavier/édition).
    tree: Option<Box<dyn Widget<A::Message>>>,
    /// Dernière position connue du curseur, en pixels **logiques**.
    cursor: Point,
    /// Facteur d'échelle DPI de l'écran (physique = logique × scale × densité).
    scale: f32,
    /// Dernière taille **logique** transmise à l'app (pour détecter les paliers).
    last_size: Option<(f32, f32)>,
    /// État retenu entre frames (survol/focus, scroll, curseur/sélection).
    runtime: Runtime,
    /// Surveillance live-reload (dev, `FRUS_WATCH=1`) : relance sur recompilation.
    reload: Option<crate::reload::ReloadWatcher>,
    /// Inspecteur runtime actif ? (bascule F12, builds debug uniquement).
    inspector: bool,
    /// Dump de l'arbre à imprimer à la prochaine frame inspectée.
    inspector_dump: bool,
    /// Modificateurs clavier courants.
    shift: bool,
    ctrl: bool,
    /// Colonne visuelle « cible » mémorisée pour Haut/Bas/PgUp/PgDn : traverser
    /// des lignes plus courtes garde la colonne d'origine (comportement d'éditeur).
    /// Effacée dès qu'un autre déplacement du curseur survient.
    goal_x: Option<f32>,
    /// Accès presse-papier (no-op sur Android).
    clipboard: clip::Clipboard,
    /// Effet de démarrage (`init`) déjà exécuté ? Évite de le rejouer quand la
    /// surface est recréée (retour d'arrière-plan sur Android).
    started: bool,
    /// Instant de la dernière frame (pour le dt des animations).
    last_frame: Option<Instant>,
    /// Phase **build** sale : l'état de l'app (ou la taille) a changé, il faut
    /// reconstruire la `view`. La vue est une fonction pure de `(état, thème,
    /// taille)` — jamais du survol/scroll/focus (qui vivent dans le `Runtime`) —,
    /// donc une frame d'animation d'interaction se contente de **repeindre**
    /// l'arbre retenu (§1 : « un survol ne touche que paint »).
    build_dirty: bool,
    /// Glissement souris en cours.
    drag: Option<Drag>,
    /// Abscisse **lissée** du curseur pendant un réordonnancement : elle rejoint la
    /// position réelle par ressort, donnant au coulissement des colonnes une inertie
    /// douce (le fond « rattrape » le fantôme qui, lui, colle au curseur).
    reorder_x: f32,
    /// Dernier message d'**annonce** poussé à la région live AccessKit (lecteur
    /// d'écran). Persiste tel quel : il n'est re-énoncé qu'au changement, si bien
    /// qu'un même texte reconduit chaque frame ne se répète pas. Bureau uniquement.
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    announce: String,
    /// Reconnaisseur tap-ou-appui-long (palier 1 des gestes).
    press: PressRecognizer,
    /// Message d'appui long de la cible pressée, capturé à l'appui.
    long_press_msg: Option<A::Message>,
    /// Instant du dernier clic (détection du double-clic).
    last_click_time: Option<Instant>,
    /// Compteur pour clés d'événements de sortie (fondu de disparition).
    leaving_counter: u64,
    /// Souscriptions actives : id → poignée d'annulation (drop = arrêt).
    running_subs: HashMap<u64, SubHandle>,
    /// Demandes de focus en attente (clés issues de `Command::focus`), résolues
    /// contre l'arbre **fraîchement construit** à la prochaine frame.
    pending_focus: Vec<u64>,
    /// **Historique de focus** (déclencheurs) pour le retour du focus à la fermeture d'un
    /// overlay : à chaque changement de focus, l'ancien (encore présent) est empilé ; si le
    /// focus **disparaît** (menu/modale fermé), on revient au plus récent encore présent.
    focus_history: Vec<WidgetId>,
    /// Focus de la frame précédente (pour détecter les transitions à empiler).
    prev_focus: Option<WidgetId>,
    /// Fenêtre masquée (occultée) : on suspend le rendu.
    occluded: bool,
    /// Temps écoulé cumulé (secondes), pour les animations continues.
    elapsed: f32,
    /// Derniers insets fenêtre transmis à l'app (padding + clavier), en logique.
    last_insets: WindowInsets,
    /// Référence d'insets **sans clavier** (et la taille physique où elle a été
    /// prise — une rotation la réinitialise) : l'excédent bas au-delà d'elle est
    /// attribué au clavier logiciel.
    inset_baseline: Option<(Insets, (u32, u32))>,
    /// Poignée de l'activité Android (pour interroger les insets, le clavier…).
    #[cfg(target_os = "android")]
    android_app: Option<winit::platform::android::activity::AndroidApp>,
    /// Le clavier logiciel est-il demandé ? (suit le focus des champs texte).
    #[cfg(target_os = "android")]
    soft_input_shown: bool,
    /// Longueur (en caractères) de la **composition** IME en cours dans le
    /// champ focalisé — remplacée à chaque mise à jour de l'IME.
    #[cfg(target_os = "android")]
    ime_composing: usize,
}

impl<A: Application> App<A> {
    /// Crée le pilote autour d'une application et de son canal de messages.
    pub fn new(app: A, proxy: EventLoopProxy<A::Message>) -> Self {
        Self {
            app,
            proxy,
            window: None,
            renderer: None,
            #[cfg(target_arch = "wasm32")]
            pending_renderer: std::rc::Rc::new(std::cell::RefCell::new(None)),
            #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
            a11y: None,
            ui: None,
            tree: None,
            cursor: Point::new(0.0, 0.0),
            scale: 1.0,
            last_size: None,
            runtime: Runtime::default(),
            reload: crate::reload::ReloadWatcher::new(),
            inspector: false,
            inspector_dump: false,
            shift: false,
            ctrl: false,
            goal_x: None,
            clipboard: clip::Clipboard::new(),
            started: false,
            last_frame: None,
            build_dirty: true,
            drag: None,
            reorder_x: 0.0,
            #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
            announce: String::new(),
            press: PressRecognizer::new(),
            long_press_msg: None,
            last_click_time: None,
            leaving_counter: 0,
            running_subs: HashMap::new(),
            pending_focus: Vec::new(),
            focus_history: Vec::new(),
            prev_focus: None,
            occluded: false,
            elapsed: 0.0,
            last_insets: WindowInsets::ZERO,
            inset_baseline: None,
            #[cfg(target_os = "android")]
            android_app: None,
            #[cfg(target_os = "android")]
            soft_input_shown: false,
            #[cfg(target_os = "android")]
            ime_composing: 0,
        }
    }

    /// Rejoue les actions demandées par une technologie d'assistance : un clic
    /// AT active le widget (comme un clic pointeur), un focus AT le focalise.
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    fn drain_a11y_actions(&mut self) {
        use crate::a11y::A11yAction;
        let actions = match self.a11y.as_ref() {
            Some(a11y) => a11y.take_actions(),
            None => return,
        };
        for action in actions {
            match action {
                A11yAction::Click(id) => {
                    if let Some(msg) = self.ui.as_ref().and_then(|ui| ui.msg_for(id)) {
                        self.dispatch(msg);
                        self.request_redraw();
                    }
                }
                A11yAction::Focus(id) => {
                    self.runtime.input.focused = Some(id);
                    self.runtime.focus_visible = true;
                    self.request_redraw();
                }
            }
        }
    }

    /// Synchronise le **clavier logiciel** avec le focus : demandé quand le
    /// focus est dans un champ texte (`cursor_at` → `Some`), refermé sinon.
    /// Appelé en fin de frame (tout changement de focus redessine déjà).
    fn sync_soft_input(&mut self) {
        #[cfg(target_os = "android")]
        {
            let editing = self
                .runtime
                .input
                .focused
                .and_then(|id| {
                    self.tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                })
                .and_then(|widget| widget.cursor_at(0.0, 0.0, 1.0, 0))
                .is_some();
            if editing != self.soft_input_shown {
                self.soft_input_shown = editing;
                self.ime_composing = 0;
                if crate::android_ime::installed() {
                    // Pont InputConnection : la view Java capte l'IME.
                    if editing {
                        if let Some(id) = self.runtime.input.focused {
                            self.push_ime_context(id);
                        }
                        crate::android_ime::start_input();
                    } else {
                        crate::android_ime::clear_editor_state();
                        crate::android_ime::stop_input();
                    }
                } else if let Some(app) = &self.android_app {
                    // Repli TYPE_NULL : ouverture simple, touches latines.
                    if editing {
                        app.show_soft_input(true);
                    } else {
                        app.hide_soft_input(false);
                    }
                }
            }
        }
    }

    /// Applique les opérations IME en attente au champ focalisé (pont §6).
    /// La composition est matérialisée **dans le champ** : chaque mise à jour
    /// efface la précédente (le modèle contrôlé n'a pas encore de région de
    /// composition stylée — voir docs/jalon-81.md).
    #[cfg(target_os = "android")]
    fn drain_ime(&mut self) {
        use crate::android_ime::ImeEvent;
        let events = crate::android_ime::drain();
        if events.is_empty() {
            return;
        }
        let Some(focused) = self.runtime.input.focused else {
            self.ime_composing = 0;
            return;
        };
        for event in events {
            match event {
                // Un commit `\n` (certains IME) = soumission, pas insertion.
                ImeEvent::Commit(text) if text == "\n" || text == "\r" => {
                    self.clear_composing(focused);
                    self.apply_key(focused, Key::Enter);
                }
                ImeEvent::Commit(text) => {
                    self.clear_composing(focused);
                    self.apply_key(focused, Key::Text(text));
                }
                ImeEvent::Composing(text) => {
                    self.clear_composing(focused);
                    let n = text.chars().count();
                    self.ime_composing = n;
                    // Position AVANT insertion = début de la région composée.
                    let start = self.runtime.edits.get(&focused).map(|e| e.cursor).unwrap_or(0);
                    if !text.is_empty() {
                        self.apply_key(focused, Key::Text(text));
                    }
                    // Enregistre la plage soulignée (curseur désormais en fin).
                    if let Some(edit) = self.runtime.edits.get_mut(&focused) {
                        edit.composing = if n > 0 { Some((start, start + n)) } else { None };
                    }
                }
                ImeEvent::FinishComposing => {
                    self.ime_composing = 0;
                    if let Some(edit) = self.runtime.edits.get_mut(&focused) {
                        edit.composing = None;
                    }
                }
                ImeEvent::Delete { before, after } => {
                    for _ in 0..before {
                        self.apply_key(focused, Key::Backspace);
                    }
                    for _ in 0..after {
                        self.apply_key(focused, Key::Delete);
                    }
                }
                ImeEvent::Action => self.apply_key(focused, Key::Enter),
                ImeEvent::Key { code, unicode } => match code {
                    66 => self.apply_key(focused, Key::Enter),
                    67 => self.apply_key(focused, Key::Backspace),
                    _ => {
                        if let Some(c) = char::from_u32(unicode).filter(|c| !c.is_control()) {
                            self.apply_key(focused, Key::Text(c.to_string()));
                        }
                    }
                },
            }
        }
        // Rafraîchit le contexte de saisie (l'IME l'interroge pour ses suggestions).
        self.push_ime_context(focused);
        self.request_redraw();
    }

    /// Publie l'état d'édition du champ `id` vers le pont (texte + curseur +
    /// sélection) — le contexte que l'IME lit pour ses suggestions.
    #[cfg(target_os = "android")]
    fn push_ime_context(&self, id: WidgetId) {
        let value = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.text_value().map(|s| s.to_string()));
        if let Some(text) = value {
            let edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
            crate::android_ime::set_editor_state(&text, edit.cursor, edit.selection_range());
        }
    }

    /// Efface la composition courante du champ (curseur en fin de composition)
    /// et remet la plage soulignée à zéro.
    #[cfg(target_os = "android")]
    fn clear_composing(&mut self, focused: WidgetId) {
        for _ in 0..self.ime_composing {
            self.apply_key(focused, Key::Backspace);
        }
        self.ime_composing = 0;
        if let Some(edit) = self.runtime.edits.get_mut(&focused) {
            edit.composing = None;
        }
    }

    /// Mémorise la poignée de l'activité Android (source des insets système).
    #[cfg(target_os = "android")]
    pub(crate) fn set_android_app(
        &mut self,
        android_app: winit::platform::android::activity::AndroidApp,
    ) {
        self.android_app = Some(android_app);
    }

    /// Insets système (zone de sécurité) en px **logiques**. Sur Android, dérivés
    /// de la zone de contenu de l'activité (hors barres système) ; ailleurs, zéro.
    fn compute_insets(&self, phys_w: u32, phys_h: u32, scale: f32) -> Insets {
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            let r = app.content_rect();
            // Rectangle dégénéré (avant la première mise en page) → pas d'inset.
            if r.right > r.left && r.bottom > r.top {
                let left = r.left.max(0) as f32;
                let top = r.top.max(0) as f32;
                let right = (phys_w as i32 - r.right).max(0) as f32;
                let bottom = (phys_h as i32 - r.bottom).max(0) as f32;
                return Insets::new(top / scale, right / scale, bottom / scale, left / scale);
            }
        }
        let _ = (phys_w, phys_h, scale);
        Insets::ZERO
    }
}

impl<A: Application> ApplicationHandler<A::Message> for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attributes = Window::default_attributes()
            .with_title(self.app.title())
            // Taille minimale raisonnable (px logiques) : évite une UI absurde.
            .with_min_inner_size(winit::dpi::LogicalSize::new(360.0, 280.0));
        // L'adaptateur AccessKit doit être créé **avant** que la fenêtre soit
        // montrée : on la crée cachée puis on la révèle après (bureau).
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        {
            attributes = attributes.with_visible(false);
        }
        if let Some((w, h)) = self.app.window_size() {
            attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(w, h));
        }
        // Web : winit crée et **ajoute un `<canvas>`** au corps du document.
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            attributes = attributes.with_append(true);
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("Échec de création de la fenêtre : {err}");
                event_loop.exit();
                return;
            }
        };

        // Web : l'init GPU est **asynchrone** (impossible de bloquer). On lance la
        // future ; le renderer prêt est récupéré à la première frame (voir
        // `RedrawRequested`). `init` s'exécute tout de suite (il ne touche pas le GPU).
        #[cfg(target_arch = "wasm32")]
        {
            self.scale = window.scale_factor() as f32;
            self.window = Some(window.clone());
            self.build_dirty = true;
            if !self.started {
                self.started = true;
                let command = self.app.init();
                self.run_command(command);
                self.sync_subscriptions();
            }
            let slot = self.pending_renderer.clone();
            let win = window.clone();
            let size = window.inner_size();
            wasm_bindgen_futures::spawn_local(async move {
                match Renderer::new(win.clone(), size.width.max(1), size.height.max(1)).await {
                    Ok(r) => {
                        *slot.borrow_mut() = Some(r);
                        win.request_redraw();
                    }
                    Err(err) => log::error!("Échec d'initialisation du renderer (Web) : {err:#}"),
                }
            });
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let size = window.inner_size();
            let renderer = pollster::block_on(Renderer::new(
                window.clone(),
                size.width.max(1),
                size.height.max(1),
            ));

            match renderer {
                Ok(renderer) => {
                    self.scale = window.scale_factor() as f32;
                    // Pont accessibilité (AccessKit) — créé pendant que la fenêtre
                    // est encore cachée, puis on la révèle. Inerte sans lecteur d'écran.
                    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
                    {
                        self.a11y = Some(crate::a11y::A11y::new(event_loop, &window));
                        window.set_visible(true);
                    }
                    self.window = Some(window.clone());
                    self.renderer = Some(renderer);
                    // Surface (re)créée : forcer une reconstruction complète à la 1re frame.
                    self.build_dirty = true;
                    // Effet de démarrage (chargement initial, etc.) : une seule fois,
                    // pas à chaque recréation de surface (retour d'arrière-plan).
                    if !self.started {
                        self.started = true;
                        let command = self.app.init();
                        self.run_command(command);
                        self.sync_subscriptions();
                    }
                    window.request_redraw();
                }
                Err(err) => {
                    log::error!("Échec d'initialisation du renderer : {err:#}");
                    event_loop.exit();
                }
            }
        }
    }

    /// Mise en arrière-plan (Android) : la surface native est détruite. On
    /// relâche renderer + fenêtre ; `resumed` les recrée au retour (sans rejouer
    /// `init`, cf. `started`). Inoffensif sur bureau (l'événement n'y survient pas).
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer = None;
        self.window = None;
        self.last_frame = None;
    }

    /// Message produit par un effet (thread de fond) : on l'applique et on
    /// redemande une frame.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, message: A::Message) {
        self.dispatch(message);
        self.request_redraw();
    }

    /// Réveil de la boucle : si l'échéance d'appui long est atteinte, le
    /// reconnaisseur **accepte avidement** — le message est émis et le
    /// relâchement à venir sera avalé (l'appui long évince le tap).
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && self.press.poll(Instant::now())
        {
            if let Some(message) = self.long_press_msg.take() {
                self.dispatch(message);
            }
            // Un défilement tactile en attente n'a plus lieu d'être.
            self.drag = None;
            self.request_redraw();
        }

        // Opérations IME en attente (pont de saisie Android).
        #[cfg(target_os = "android")]
        self.drain_ime();

        // Actions demandées par une technologie d'assistance (AccessKit).
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        self.drain_a11y_actions();

        // Live-reload (dev) : binaire remplacé par une recompilation →
        // capture l'état et relance le nouveau binaire (ne revient pas).
        if let Some(watcher) = self.reload.as_mut() {
            if watcher.binary_changed() {
                watcher.handoff(self.app.save_state());
            }
        }
        event_loop.set_control_flow(self.idle_control_flow());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // L'adaptateur AccessKit observe les événements fenêtre (focus, taille…).
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        if let (Some(a11y), Some(window)) = (self.a11y.as_mut(), self.window.as_ref()) {
            a11y.process_event(window, &event);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor as f32;
                // Reconfigure la surface à la taille physique courante.
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer.resize(size.width, size.height);
                    }
                }
                self.request_redraw();
            }

            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if !occluded {
                    self.request_redraw();
                }
            }

            // Toutes les sources pointeur (souris, tactile) convergent vers
            // l'entrée **normalisée** `pointer()` — palier 0 des gestes, avec
            // `Cancel` explicite. winit fournit des px physiques ; on travaille
            // en logique (échelle totale = DPI × densité).
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.total_scale();
                let position = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                self.pointer(
                    event_loop,
                    PointerEvent { kind: PointerKind::Move, position, touch: false },
                );
            }

            WindowEvent::Touch(touch) => {
                let scale = self.total_scale();
                let position =
                    Point::new(touch.location.x as f32 / scale, touch.location.y as f32 / scale);
                let kind = match touch.phase {
                    TouchPhase::Started => PointerKind::Down,
                    TouchPhase::Moved => PointerKind::Move,
                    TouchPhase::Ended => PointerKind::Up,
                    TouchPhase::Cancelled => PointerKind::Cancel,
                };
                self.pointer(event_loop, PointerEvent { kind, position, touch: true });
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.shift = state.shift_key();
                self.ctrl = state.control_key();
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.pointer(
                event_loop,
                PointerEvent { kind: PointerKind::Down, position: self.cursor, touch: false },
            ),

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.pointer(
                event_loop,
                PointerEvent { kind: PointerKind::Up, position: self.cursor, touch: false },
            ),

            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed =>
            {
                // Interaction clavier : l'anneau de focus (re)devient visible.
                if !self.runtime.focus_visible {
                    self.runtime.focus_visible = true;
                    self.request_redraw();
                }

                // Retour **système** (bouton/geste Android, touche « précédent ») :
                // ferme l'overlay du dessus, sinon dépile un écran, sinon quitte.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::BrowserBack) | WinitKey::Named(NamedKey::GoBack)
                ) {
                    if !event.repeat {
                        self.system_back(event_loop);
                    }
                    return;
                }

                // F12 : bascule l'**inspecteur** (contours + fiche du widget
                // survolé + dump de l'arbre sur stderr) — debug uniquement.
                if matches!(event.logical_key, WinitKey::Named(NamedKey::F12)) {
                    if cfg!(debug_assertions) && !event.repeat {
                        self.inspector = !self.inspector;
                        self.inspector_dump = self.inspector;
                        self.request_redraw();
                    }
                    return;
                }

                // Tab / Shift+Tab : navigue entre les focusables (même sans focus).
                if matches!(event.logical_key, WinitKey::Named(NamedKey::Tab)) {
                    let forward = !self.shift;
                    let next = self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.focus_next(self.runtime.input.focused, forward));
                    if next.is_some() {
                        self.runtime.input.focused = next;
                        self.request_redraw();
                    }
                    return;
                }

                // Échap : montée **feuille→racine** depuis le focalisé (un
                // `Portal` la consomme pour se fermer) ; à défaut, fermeture de
                // l'overlay le plus au-dessus — Échap marche donc sans focus.
                if matches!(event.logical_key, WinitKey::Named(NamedKey::Escape)) {
                    // La répétition automatique ne re-déclenche pas de fermeture.
                    if !event.repeat {
                        self.escape();
                    }
                    return;
                }

                let Some(focused) = self.runtime.input.focused else {
                    return;
                };

                // Flèches : navigation **géométrique** du focus — sauf gauche/
                // droite dans un champ texte (elles y déplacent le curseur ;
                // haut/bas naviguent même depuis un champ mono-ligne).
                let arrow = match event.logical_key {
                    WinitKey::Named(NamedKey::ArrowUp) => Some(FocusDirection::Up),
                    WinitKey::Named(NamedKey::ArrowDown) => Some(FocusDirection::Down),
                    WinitKey::Named(NamedKey::ArrowLeft) => Some(FocusDirection::Left),
                    WinitKey::Named(NamedKey::ArrowRight) => Some(FocusDirection::Right),
                    _ => None,
                };
                // PgUp / PgDn : saut d'une **page** dans un champ multi-lignes (borné
                // au champ, on n'en sort pas). Sans effet ailleurs.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::PageUp) | WinitKey::Named(NamedKey::PageDown)
                ) {
                    let down = matches!(event.logical_key, WinitKey::Named(NamedKey::PageDown));
                    if self.move_caret_vertical(focused, down, true) {
                        return;
                    }
                }

                if let Some(direction) = arrow {
                    // Champ **multi-lignes** : Haut/Bas déplacent le caret entre lignes
                    // tant qu'il reste dans le champ ; à la première/dernière ligne, on
                    // retombe sur la navigation du focus (on quitte le champ).
                    if matches!(direction, FocusDirection::Up | FocusDirection::Down) {
                        let down = matches!(direction, FocusDirection::Down);
                        if self.move_caret_vertical(focused, down, false) {
                            return;
                        }
                    }

                    // Flèches gauche/droite : proposées d'abord au widget focalisé
                    // (curseur de plage…) via `on_key`. S'il les consomme, on n'en
                    // navigue pas le focus.
                    if matches!(direction, FocusDirection::Left | FocusDirection::Right) {
                        let key = if matches!(direction, FocusDirection::Left) {
                            Key::Left { shift: self.shift, word: self.ctrl }
                        } else {
                            Key::Right { shift: self.shift, word: self.ctrl }
                        };
                        let widget = self
                            .tree
                            .as_ref()
                            .and_then(|tree| find_widget(tree.as_ref(), focused));
                        // Un en-tête réordonnable déplace sa colonne d'un cran ; on
                        // capte sa position pour l'annoncer (le clavier est le chemin
                        // de réordonnancement des utilisateurs de lecteur d'écran).
                        let reorder_from = widget.and_then(|w| w.reorder_index());
                        let handled = widget.map(|w| w.on_key(&key));
                        if let Some(KeyResponse::Handled(message)) = handled {
                            if let Some(message) = message {
                                self.dispatch(message);
                            }
                            if let Some(from) = reorder_from {
                                let to = if matches!(direction, FocusDirection::Left) {
                                    from.wrapping_sub(1)
                                } else {
                                    from + 1
                                };
                                self.set_announcement(format!("Column moved to position {}", to + 1));
                            }
                            self.request_redraw();
                            return;
                        }
                    }

                    let is_text = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .and_then(|widget| widget.cursor_at(0.0, 0.0, 1.0, 0))
                        .is_some();
                    let navigates = !is_text
                        || matches!(direction, FocusDirection::Up | FocusDirection::Down);
                    if navigates {
                        if let Some(next) = self
                            .ui
                            .as_ref()
                            .and_then(|ui| ui.focus_directional(focused, direction))
                        {
                            self.runtime.input.focused = Some(next);
                            self.request_redraw();
                        }
                        return;
                    }
                }

                // Début/Fin proposées au widget focalisé (curseur de plage → min/max)
                // avant l'action par défaut. Un champ texte les ignore ici (`on_key`
                // Ignored) et retombe sur l'édition normale plus bas.
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::Home) | WinitKey::Named(NamedKey::End)
                ) {
                    let key = if matches!(event.logical_key, WinitKey::Named(NamedKey::Home)) {
                        Key::Home { shift: self.shift, doc: self.ctrl }
                    } else {
                        Key::End { shift: self.shift, doc: self.ctrl }
                    };
                    let handled = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .map(|widget| widget.on_key(&key));
                    if let Some(KeyResponse::Handled(message)) = handled {
                        if let Some(message) = message {
                            self.dispatch(message);
                        }
                        self.request_redraw();
                        return;
                    }
                }

                // Activation clavier (Entrée/Espace) d'un focusable cliquable
                // (bouton, case, interrupteur). Les champs texte (sans `on_click`)
                // retombent sur l'édition normale (Entrée = soumettre, Espace = espace).
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::Enter) | WinitKey::Named(NamedKey::Space)
                ) {
                    let widget = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .filter(|widget| widget.focusable());
                    let message = widget.and_then(|widget| widget.on_click());
                    if let Some(message) = message {
                        // La répétition automatique ne mitraille pas l'activation
                        // (maintenir Espace sur un bouton = un seul clic).
                        if !event.repeat {
                            // Annonce vocale de l'effet (tri, sélection…), capturée
                            // avant la reconstruction que déclenche `dispatch`.
                            let announce = widget.and_then(|widget| widget.announce());
                            self.dispatch(message);
                            if let Some(announce) = announce {
                                self.set_announcement(announce);
                            }
                            self.request_redraw();
                        }
                        return;
                    }
                }

                // Raccourcis presse-papier (Ctrl+C/X/V/A).
                if self.ctrl {
                    match &event.logical_key {
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("c") => {
                            self.copy_selection(focused);
                            return;
                        }
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("x") => {
                            self.copy_selection(focused);
                            self.apply_key(focused, Key::Backspace);
                            self.request_redraw();
                            return;
                        }
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("v") => {
                            if let Some(text) = self.clipboard.get_text() {
                                self.apply_key(focused, Key::Text(text));
                                self.request_redraw();
                            }
                            return;
                        }
                        WinitKey::Character(c) if c.eq_ignore_ascii_case("a") => {
                            self.runtime.edits.insert(
                                focused,
                                Edit {
                                    cursor: usize::MAX,
                                    anchor: Some(0),
                                    composing: None,
                                },
                            );
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                // Pont de saisie actif : l'édition passe EXCLUSIVEMENT par
                // l'InputConnection (file IME) — la view pont reçoit aussi les
                // touches matérielles. Sans cette garde, chaque frappe
                // arriverait en double (file native winit + pont).
                #[cfg(target_os = "android")]
                if crate::android_ime::installed() {
                    return;
                }

                let shift = self.shift;
                let key = match &event.logical_key {
                    WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
                    WinitKey::Named(NamedKey::Delete) => Some(Key::Delete),
                    // La répétition d'Entrée ne re-soumet pas (le texte et
                    // l'effacement, eux, répètent normalement).
                    WinitKey::Named(NamedKey::Enter) if !event.repeat => Some(Key::Enter),
                    WinitKey::Named(NamedKey::Enter) => None,
                    // Ctrl : Gauche/Droite sautent un mot, Début/Fin bornent le champ.
                    WinitKey::Named(NamedKey::ArrowLeft) => {
                        Some(Key::Left { shift, word: self.ctrl })
                    }
                    WinitKey::Named(NamedKey::ArrowRight) => {
                        Some(Key::Right { shift, word: self.ctrl })
                    }
                    WinitKey::Named(NamedKey::Home) => Some(Key::Home { shift, doc: self.ctrl }),
                    WinitKey::Named(NamedKey::End) => Some(Key::End { shift, doc: self.ctrl }),
                    WinitKey::Named(NamedKey::Space) => Some(Key::Text(" ".to_string())),
                    // Android livre Entrée en `Character("\n")` (KeyCharacterMap),
                    // pas en `Named(Enter)` : même soumission, sans insérer de
                    // retour à la ligne dans le champ.
                    WinitKey::Character(c) if c == "\n" || c == "\r" => {
                        if event.repeat {
                            None
                        } else {
                            Some(Key::Enter)
                        }
                    }
                    _ => event.text.as_ref().map(|text| Key::Text(text.to_string())),
                };

                if let Some(key) = key {
                    self.apply_key(focused, key);
                    self.request_redraw();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let (mut dx, mut dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * SCROLL_SPEED, y * SCROLL_SPEED),
                    // Delta physique → logique.
                    MouseScrollDelta::PixelDelta(pos) => {
                        let scale = self.total_scale();
                        (pos.x as f32 / scale, pos.y as f32 / scale)
                    }
                };
                // Shift : la molette défile horizontalement.
                if self.shift {
                    dx = dy;
                    dy = 0.0;
                }
                // Fenêtre interactive sous le curseur : la molette **zoome** (ancré au
                // curseur), bornée par les échelles min/max du widget.
                if let Some((id, viewport)) = self.ui.as_ref().and_then(|ui| ui.interactive_at(self.cursor)) {
                    let (min, max) = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                        .and_then(|w| w.interactive())
                        .unwrap_or((0.5, 4.0));
                    // Molette vers le haut (`dy > 0`) = zoom avant. Pas doux (~1.1×/cran).
                    let factor = (1.0 + dy * 0.1 / SCROLL_SPEED).clamp(0.2, 5.0);
                    // Le zoom coupe un fling en cours.
                    self.runtime.interactive_velocity.remove(&id);
                    let view = self.runtime.interactive.entry(id).or_default();
                    // Zoom ancré au curseur, puis bornage au cadre.
                    *view = view.zoom_at(factor, self.cursor, min, max).clamped(viewport);
                    self.request_redraw();
                    return;
                }
                if let Some((id, max_x, max_y)) =
                    self.ui.as_ref().and_then(|ui| ui.scroll_hit(self.cursor))
                {
                    // Défilement à inertie : la molette pousse la CIBLE (avec un
                    // léger dépassement élastique) ; le ressort la rejoint en douceur.
                    let current = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));
                    let target = self.runtime.scroll_target.entry(id).or_insert(current);
                    target.0 = (target.0 - dx).clamp(-SCROLL_OVER, max_x + SCROLL_OVER);
                    target.1 = (target.1 - dy).clamp(-SCROLL_OVER, max_y + SCROLL_OVER);
                    self.runtime.scroll_velocity.entry(id).or_insert((0.0, 0.0));
                    self.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                // Web : récupère le renderer initialisé de façon asynchrone dès qu'il
                // est prêt ; tant qu'il ne l'est pas, on ne peint pas.
                #[cfg(target_arch = "wasm32")]
                if self.renderer.is_none() {
                    match self.pending_renderer.borrow_mut().take() {
                        Some(renderer) => self.renderer = Some(renderer),
                        None => return,
                    }
                }
                // Fenêtre masquée : rendu suspendu (reprise sur Occluded(false)).
                if self.occluded {
                    return;
                }
                let size = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .unwrap_or_default();
                // Minimisée / taille nulle : rien à dessiner (évite les erreurs GPU).
                if size.width == 0 || size.height == 0 {
                    self.last_frame = None; // pas de saut de dt à la restauration
                    return;
                }
                // L'interface est décrite en pixels **logiques** ; la sortie GPU est
                // mise à l'échelle physique (DPI × densité) juste avant le rendu.
                let scale = self.total_scale();
                let (width, height) = (size.width as f32 / scale, size.height as f32 / scale);

                // Changement de taille logique (resize OU densité) → notifie l'app
                // avant la vue, pour qu'elle réagisse au palier dans sa logique.
                if self.last_size != Some((width, height)) {
                    self.last_size = Some((width, height));
                    self.build_dirty = true;
                    self.app.on_resize(width, height);
                }

                // Insets fenêtre : sépare le **padding** statique (barres/encoche)
                // du **clavier** (excédent bas au-delà de la référence sans
                // clavier). La référence est prise à la première mesure pour cette
                // taille physique (rotation → nouvelle référence), et se corrige
                // vers le bas si un état plus « nu » apparaît (clavier ouvert au
                // démarrage, barres masquées…).
                let raw = self.compute_insets(size.width, size.height, scale);
                let phys = (size.width, size.height);
                let mut baseline = match self.inset_baseline {
                    Some((b, s)) if s == phys => b,
                    _ => {
                        self.inset_baseline = Some((raw, phys));
                        raw
                    }
                };
                if raw.bottom < baseline.bottom {
                    baseline = raw;
                    self.inset_baseline = Some((raw, phys));
                }
                let insets = WindowInsets::from_baseline(baseline, raw);
                if self.last_insets != insets {
                    self.last_insets = insets;
                    self.build_dirty = true;
                    self.app.on_insets(insets);
                }

                // dt écoulé (clampé), pour toutes les animations.
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|prev| (now - prev).as_secs_f32().min(0.05))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);

                // Horloge continue (secondes) pour les animations pilotées par le temps.
                self.elapsed += dt;
                self.runtime.time = self.elapsed;

                // L'application avance ses propres animations (thème, nav, geste).
                let app_animating = self.app.tick(dt);
                let theme = self.app.theme();

                // === Phase BUILD (conditionnelle) ===
                // On ne reconstruit la `view` que si l'état de l'app ou la taille ont
                // changé (`build_dirty`), ou si l'app anime (thème/nav/geste modifient
                // l'état lu par la vue). Une frame d'animation d'interaction pure
                // (survol, scroll, focus, curseur) **réutilise l'arbre retenu** et ne
                // fait que repeindre : la `view` est une fonction pure de
                // `(état, thème, taille)`, jamais du `Runtime`.
                let need_build = self.build_dirty || app_animating || self.tree.is_none();
                if need_build {
                    let tree = self.app.view(&theme, width, height);
                    let ids = collect_ids(tree.as_ref());
                    let present: std::collections::HashSet<_> = ids.iter().copied().collect();

                    // Sortie : capture l'instantané des widgets présents à N-1 mais
                    // absents à N, pour les faire disparaître en fondu.
                    let leaving: std::collections::HashSet<u64> = self
                        .runtime
                        .mounted
                        .iter()
                        .filter(|id| !present.contains(id))
                        .map(|id| id.as_u64())
                        .collect();
                    if !leaving.is_empty() {
                        if let Some(ui) = &self.ui {
                            let captured: Vec<_> = ui
                                .scene()
                                .primitives()
                                .iter()
                                .filter(|p| leaving.contains(&p.owner()))
                                .cloned()
                                .collect();
                            if !captured.is_empty() {
                                self.runtime.leaving.insert(self.leaving_counter, (captured, 1.0));
                                self.leaving_counter = self.leaving_counter.wrapping_add(1);
                            }
                        }
                    }

                    // Montage : les nouveaux widgets démarrent en fondu.
                    for &id in &ids {
                        if self.runtime.mounted.insert(id) {
                            self.runtime.anims.entry(id).or_default().opacity = 0.0;
                        }
                    }
                    self.runtime.mounted.retain(|id| present.contains(id));

                    self.tree = Some(tree);

                    // La config a pu changer : invalide le cache de peinture
                    // (frontières de repaint). Ses entrées d'une génération
                    // périmée ne seront plus des *hits* → repaint complet cette
                    // frame, puis réutilisation aux frames d'interaction pure.
                    self.runtime.paint_cache.borrow_mut().bump_generation();
                }
                self.build_dirty = false;

                // Demandes de focus (`Command::focus`) : résolues contre l'arbre qui
                // vient d'être (re)construit — la clé désigne le champ enveloppé par
                // `keyed(k, …)`. La plus récente qui se résout l'emporte ; l'anneau de
                // focus (re)devient visible (on saute au champ, comme au clavier).
                if !self.pending_focus.is_empty() {
                    let keys = std::mem::take(&mut self.pending_focus);
                    let ids: Vec<WidgetId> = self
                        .tree
                        .as_deref()
                        .map(|tree| keys.iter().filter_map(|&k| find_by_key(tree, k)).collect())
                        .unwrap_or_default();
                    if let Some(&id) = ids.last() {
                        self.runtime.input.focused = Some(id);
                        self.runtime.focus_visible = true;
                    }
                }

                // === Phase PAINT ===
                // L'arbre retenu est (re)peint. La mise en page passe par le cache
                // de relayout (jalon 55 : taffy rappelé seulement si structure
                // changée) ; la peinture par le cache de repaint (jalon 88 : un
                // sous-arbre `RepaintBoundary` statique est rejoué sans repeindre
                // tant que sa géométrie et l'état d'interaction sont stables).
                let tree = self.tree.as_deref().expect("view construite au moins une fois");

                // Inertie de défilement et de pan (bornes/fenêtres issues de la frame
                // précédente).
                let scroll_maxes = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.scrollable_maxes())
                    .unwrap_or_default();
                let interactive_bounds = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.interactive_bounds())
                    .unwrap_or_default();

                // Ressort du réordonnancement **horizontal** : l'abscisse lissée rejoint le curseur
                // (constante de temps ~70 ms) — les colonnes coulissent avec inertie. Sans objet à la
                // verticale (cartes Kanban), dont le réagencement suit le curseur sans lissage x :
                // gardé à l'axe horizontal pour ne pas animer à vide.
                let horizontal_reorder = matches!(self.drag, Some(Drag::Reorder { moved: true, .. }))
                    && self.dragged_reorder_axis() == Some(ReorderAxis::Horizontal);
                let reorder_animating = if horizontal_reorder {
                    self.reorder_x = spring_toward(self.reorder_x, self.cursor.x, dt, 0.07);
                    (self.cursor.x - self.reorder_x).abs() > 0.5
                } else {
                    false
                };

                let animating = self.runtime.advance(dt)
                    | self.runtime.advance_leaving(dt)
                    | self.runtime.advance_values(tree, dt)
                    | self.runtime.advance_colors(tree, dt)
                    | self.runtime.advance_sizes(tree, dt)
                    | self.runtime.advance_radii(tree, dt)
                    | self.runtime.advance_paddings(tree, dt)
                    | self.runtime.advance_scroll(&scroll_maxes, dt)
                    | self.runtime.advance_interactive(&interactive_bounds, dt)
                    | reorder_animating
                    | app_animating;
                // Inspecteur actif : la même construction collecte les nœuds
                // observés, et le calque (contours + fiche du widget survolé)
                // se peint par-dessus une copie de la scène.
                let (ui, scene) = if self.inspector {
                    let (ui, nodes) = frus_widgets::build_ui_inspected(
                        tree,
                        Size::new(width, height),
                        &self.runtime,
                        &theme,
                    );
                    if std::mem::take(&mut self.inspector_dump) {
                        eprintln!("{}", frus_widgets::dump_tree(&nodes));
                    }
                    let mut scene = ui.scene().clone();
                    frus_widgets::paint_inspector_overlay(
                        &nodes,
                        Some(self.cursor),
                        Size::new(width, height),
                        &theme,
                        &mut scene,
                    );
                    // Scène logique → physique (DPI × densité).
                    (ui, scene.scaled(scale))
                } else {
                    let ui = build_ui(tree, Size::new(width, height), &self.runtime, &theme);
                    // Aperçu d'un réordonnancement de colonne en cours, par-dessus la scène.
                    let scene = if matches!(self.drag, Some(Drag::Reorder { moved: true, .. })) {
                        let mut scene = ui.scene().clone();
                        self.paint_reorder_preview(&ui, &theme, &mut scene);
                        scene.scaled(scale)
                    } else {
                        // Scène logique → physique (DPI × densité) pour un rendu net.
                        ui.scene().scaled(scale)
                    };
                    (ui, scene)
                };
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(&scene) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            renderer.reconfigure();
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("Mémoire GPU épuisée, fermeture.");
                            event_loop.exit();
                        }
                        Err(err) => log::warn!("Frame ignorée : {err:?}"),
                    }
                }

                // Un widget à animation continue (spinner…) force le redessin.
                let wants_animation = ui.wants_animation();

                // Conserve l'interface (hit-test). L'arbre est déjà retenu.
                self.ui = Some(ui);

                // Retour du focus : si le widget focalisé a **disparu** (overlay fermé),
                // revenir au déclencheur. Fait avant l'annonce AccessKit (focus à jour).
                self.reconcile_focus();

                // Publie l'arbre sémantique de la frame à AccessKit.
                #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
                if let Some(a11y) = self.a11y.as_mut() {
                    let focus = self.runtime.input.focused;
                    let title = self.app.title();
                    if let Some(ui) = self.ui.as_ref() {
                        a11y.update(ui.semantics(), focus, &title, &self.announce);
                    }
                }

                // Clavier logiciel Android : suit le focus des champs texte.
                self.sync_soft_input();

                // Tant qu'une animation tourne, on redemande une frame.
                if animating || wants_animation {
                    self.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl<A: Application> App<A> {
    /// Échelle totale : DPI système × densité applicative (physique = logique × ceci).
    fn total_scale(&self) -> f32 {
        (self.scale * self.app.density()).max(0.1)
    }

    /// Direction de mise en page courante (RTL retourne le layout et les gestes).
    fn is_rtl(&self) -> bool {
        self.app.theme().direction.is_rtl()
    }

    /// Largeur **logique** de la fenêtre (px), pour les seuils de bord.
    fn logical_width(&self) -> f32 {
        let scale = self.total_scale();
        self.window
            .as_ref()
            .map(|w| w.inner_size().width as f32 / scale)
            .unwrap_or(1.0)
            .max(1.0)
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Entrée pointeur **normalisée** (palier 0 des gestes) : souris et tactile
    /// convergent ici, avec `Cancel` explicite ; le reconnaisseur d'appui long
    /// est alimenté au passage et la boucle est réveillée à son échéance.
    fn pointer(&mut self, event_loop: &ActiveEventLoop, event: PointerEvent) {
        self.cursor = event.position;
        match event.kind {
            PointerKind::Down => {
                // Interaction pointeur : l'anneau de focus clavier s'efface.
                self.runtime.focus_visible = false;
                self.pointer_down(event.touch);
                // Candidat à l'appui long : seulement si l'appui n'a pas été
                // capturé par un glissement (barre, poignée, sélection) — un
                // défilement tactile pas encore en mouvement reste candidat.
                let free = matches!(self.drag, None | Some(Drag::Scroll { moved: false, .. }));
                self.long_press_msg = if free {
                    self.ui.as_ref().and_then(|ui| ui.long_press_at(self.cursor))
                } else {
                    None
                };
                self.press
                    .down(self.cursor, Instant::now(), self.long_press_msg.is_some());
            }
            PointerKind::Move => {
                self.press.moved(self.cursor);
                self.pointer_move();
                // L'inspecteur suit le curseur (surlignage) : redessine même
                // au-dessus de widgets inertes (aucune animation de survol).
                if self.inspector {
                    self.request_redraw();
                }
            }
            PointerKind::Up => {
                if self.press.up() {
                    // L'appui long a évincé le tap : relâchement avalé.
                    self.drag = None;
                    self.runtime.input.pressed = None;
                    self.request_redraw();
                } else {
                    self.pointer_up();
                }
            }
            PointerKind::Cancel => {
                self.press.cancel();
                self.drag = None;
                self.runtime.input.pressed = None;
                self.request_redraw();
            }
        }
        // Réveil de la boucle pile à la prochaine échéance, sinon repos.
        event_loop.set_control_flow(self.idle_control_flow());
    }

    /// Politique de repos de la boucle : réveil à la **plus proche** des
    /// échéances (appui long, sondage du live-reload), sinon attente pure.
    fn idle_control_flow(&self) -> ControlFlow {
        let press = self.press.deadline();
        let reload = self.reload.as_ref().map(|w| w.deadline());
        match (press, reload) {
            (Some(a), Some(b)) => ControlFlow::WaitUntil(a.min(b)),
            (Some(a), None) => ControlFlow::WaitUntil(a),
            (None, Some(b)) => ControlFlow::WaitUntil(b),
            (None, None) => ControlFlow::Wait,
        }
    }

    /// Déplacement du pointeur (souris ou doigt) : poursuit un glissement en
    /// cours, sinon met à jour le survol.
    fn pointer_move(&mut self) {
        if self.drag.is_some() {
            self.handle_drag();
            return;
        }
        let hovered = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        if hovered != self.runtime.input.hovered {
            self.runtime.input.hovered = hovered;
            self.request_redraw();
        }
        // Curseur système selon la sous-région survolée (jalon 205) : main sur une icône
        // cliquable, etc. Recalculé a chaque mouvement (la sous-region peut changer sans que le
        // widget survole change).
        self.update_cursor_icon(hovered);
    }

    /// Applique la forme de curseur demandee par le widget survole a la position locale du
    /// pointeur (jalon 205), sinon le curseur par defaut. Traduit `frus_widgets::Cursor` vers winit.
    fn update_cursor_icon(&mut self, hovered: Option<WidgetId>) {
        let requested = hovered.and_then(|id| {
            let rect = self.ui.as_ref()?.widget_rect(id)?;
            let widget = find_widget(self.tree.as_ref()?.as_ref(), id)?;
            widget.cursor_icon(
                self.cursor.x - rect.x,
                self.cursor.y - rect.y,
                rect.width,
                rect.height,
            )
        });
        let icon = match requested.unwrap_or(UiCursor::Default) {
            UiCursor::Default => CursorIcon::Default,
            UiCursor::Pointer => CursorIcon::Pointer,
            UiCursor::Text => CursorIcon::Text,
        };
        if let Some(window) = &self.window {
            window.set_cursor(icon);
        }
        // Surbrillance de sous-region (jalon 208) : on retient la position du pointeur tant qu'il
        // survole une sous-region interactive (cursor_icon a repondu). Un changement repeint (le
        // hash de statut inclut hover_cursor) pour suivre / retirer le halo.
        let hover_cursor = requested.map(|_| self.cursor);
        if hover_cursor != self.runtime.input.hover_cursor {
            self.runtime.input.hover_cursor = hover_cursor;
            self.request_redraw();
        }
    }

    /// Appui du pointeur (souris ou doigt) à la position `self.cursor`. `touch`
    /// active le défilement au doigt quand aucun autre geste ne capture l'appui.
    fn pointer_down(&mut self, touch: bool) {
        // 0) Geste retour : appui sur le **bord de départ** (gauche en LTR,
        // droite en RTL), si l'app l'autorise.
        let on_back_edge = if self.is_rtl() {
            self.cursor.x > self.logical_width() - BACK_EDGE
        } else {
            self.cursor.x < BACK_EDGE
        };
        if on_back_edge && self.app.can_go_back() {
            self.drag = Some(Drag::Back {
                start_x: self.cursor.x,
                last_x: self.cursor.x,
                last_t: Instant::now(),
                velocity: 0.0,
            });
            self.build_dirty = true;
            self.app.back_gesture(0.0);
            self.request_redraw();
            return;
        }

        // 1) Glissement d'une barre de défilement ?
        if let Some(bar) = self.ui.as_ref().and_then(|ui| ui.scrollbar_at(self.cursor)) {
            let (along, thumb_start) = if bar.vertical {
                (self.cursor.y, bar.thumb.y)
            } else {
                (self.cursor.x, bar.thumb.x)
            };
            self.drag = Some(Drag::Scrollbar {
                id: bar.id,
                vertical: bar.vertical,
                grab: along - thumb_start,
                track_start: bar.track_start,
                track_len: bar.track_len,
                thumb_len: bar.thumb_len,
                max: bar.max,
            });
            self.request_redraw();
            return;
        }

        // 1 bis) Glissement d'un widget draggable (ex. Slider) ?
        if let Some((id, rect)) = self.ui.as_ref().and_then(|ui| ui.draggable_at(self.cursor)) {
            self.drag = Some(Drag::Widget { id, rect, last_x: self.cursor.x });
            // Delta nul à l'appui : seul un curseur (fraction) saute au clic.
            self.apply_widget_drag(id, rect, 0.0);
            self.request_redraw();
            return;
        }

        // 1 ter) Réordonnancement de colonne : appui sur un en-tête réordonnable.
        // On ne `return` pas — le focus et `pressed` (tap = tri) se règlent ci-dessous ;
        // le glissement ne s'engage qu'au-delà du seuil (sinon le relâchement trie).
        if let Some((id, from)) = self.reorderable_at(self.cursor) {
            self.drag = Some(Drag::Reorder { id, from, start: self.cursor, moved: false });
            self.reorder_x = self.cursor.x; // départ collé au curseur (pas de ressaut)
        }

        self.runtime.input.pressed = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        // 2) Focus + placement du curseur, et début d'une sélection texte.
        let previously_focused = self.runtime.input.focused;
        let focus = self.ui.as_ref().and_then(|ui| ui.focus_hit(self.cursor));
        self.runtime.input.focused = focus.map(|(id, _)| id);
        if let Some((id, rect)) = focus {
            let local_x = self.cursor.x - rect.x;
            // Le défilement vertical retenu (champ multi-lignes) fait partie de la
            // coordonnée contenu : on l'ajoute pour que le clic tombe sur la bonne ligne.
            let local_y =
                self.cursor.y - rect.y + self.runtime.scroll.get(&id).map(|s| s.1).unwrap_or(0.0);
            // Défilement affiché juste avant ce clic : calculé depuis le
            // curseur courant si le champ était déjà focalisé, sinon 0.
            let scroll_cursor = if previously_focused == Some(id) {
                self.runtime.edits.get(&id).map(|e| e.cursor).unwrap_or(0)
            } else {
                0
            };
            // Seuls les **champs texte** (`cursor_at` → `Some`) démarrent une
            // sélection ; les autres focusables (boutons, cases…) gardent le
            // focus mais ne doivent PAS capturer le clic (sinon il est avalé
            // au relâchement comme une fin de glissement).
            let cursor = self
                .tree
                .as_ref()
                .and_then(|tree| find_widget(tree.as_ref(), id))
                .and_then(|widget| widget.cursor_at(local_x, local_y, rect.width, scroll_cursor));
            if let Some(cursor) = cursor {
                // Placer le caret à la souris oublie la colonne cible verticale.
                self.goal_x = None;
                self.runtime.edits.insert(id, Edit { cursor, anchor: None, composing: None });
                self.drag = Some(Drag::TextSelect { id, rect });

                // Double-clic : sélectionne le mot sous le curseur.
                let now = Instant::now();
                let double = self
                    .last_click_time
                    .map(|t| (now - t).as_secs_f32() < 0.4)
                    .unwrap_or(false);
                self.last_click_time = Some(now);
                if double {
                    if let Some((start, end)) = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                        .and_then(|widget| widget.word_at(cursor))
                    {
                        self.runtime.edits.insert(
                            id,
                            Edit {
                                cursor: end,
                                anchor: Some(start),
                                composing: None,
                            },
                        );
                        self.drag = None;
                    }
                }
            }
        }

        // 3) Tactile : si rien n'a capturé le geste (ni barre, ni widget, ni
        // sélection texte), préparer un défilement au doigt sur la zone sous le
        // doigt. Un relâchement sans mouvement (< TOUCH_SLOP) restera un tap.
        if touch && self.drag.is_none() {
            if let Some((id, _, _)) = self.ui.as_ref().and_then(|ui| ui.scroll_hit(self.cursor)) {
                self.drag = Some(Drag::Scroll {
                    id,
                    last: self.cursor,
                    moved: false,
                    velocity: (0.0, 0.0),
                    last_t: Instant::now(),
                });
            }
        }

        // 4) Fenêtre interactive (`InteractiveViewer`) sous le pointeur : préparer un
        // **pan** (souris ou doigt). Comme le défilement tactile, il ne s'engage
        // qu'au-delà du seuil — un tap passe alors à l'enfant (bouton, etc.).
        if self.drag.is_none() {
            if let Some((id, viewport)) = self.ui.as_ref().and_then(|ui| ui.interactive_at(self.cursor)) {
                // L'appui stoppe un fling en cours (on reprend la main sur le contenu).
                self.runtime.interactive_velocity.remove(&id);
                self.drag = Some(Drag::Pan {
                    id,
                    last: self.cursor,
                    moved: false,
                    velocity: (0.0, 0.0),
                    last_t: Instant::now(),
                    viewport,
                });
            }
        }
        self.request_redraw();
    }

    /// Relâchement du pointeur (souris ou doigt) : termine un glissement ou
    /// valide un clic/tap si le relâchement retombe sur le widget pressé.
    fn pointer_up(&mut self) {
        let ended = self.drag.take();
        if let Some(Drag::Back { velocity, .. }) = ended {
            // L'app décide (valider / annuler) à partir de la vélocité.
            self.build_dirty = true;
            self.app.back_gesture_end(velocity);
            self.request_redraw();
            return;
        }
        // Un défilement tactile ou un pan qui n'a pas bougé = un simple tap : on le
        // laisse suivre le chemin normal du clic ci-dessous.
        let was_tap = matches!(
            ended,
            Some(Drag::Scroll { moved: false, .. })
                | Some(Drag::Pan { moved: false, .. })
                | Some(Drag::Reorder { moved: false, .. })
        );
        // Réordonnancement : au dépôt, la colonne cible est l'en-tête réordonnable
        // sous le curseur ; on route `on_reorder(from, to)` de l'en-tête saisi.
        if let Some(Drag::Reorder { id, from, moved: true, .. }) = &ended {
            let target = self.ui.as_ref().and_then(|ui| ui.reorderable_at(self.cursor));
            let tree = self.tree.as_ref();
            let base = target
                .and_then(|tid| tree.and_then(|t| find_widget(t.as_ref(), tid)))
                .and_then(|widget| widget.reorder_index());
            // Moitié **basse** d'une cible verticale survolée → insertion **après** (index +1) :
            // l'emplacement effectif de dépôt suit la ligne d'insertion peinte. Sans effet à
            // l'horizontale (colonnes de `Table`) ni hors cible.
            let to = match (base, target) {
                (Some(base), Some(tid)) => {
                    let after = self
                        .ui
                        .as_ref()
                        .and_then(|ui| ui.widget_rect(tid))
                        .map(|rect| self.reorder_insert_after(tid, rect))
                        .unwrap_or(false);
                    Some(base + after as usize)
                }
                _ => None,
            };
            // Dépôt **sur soi-même** : aucun déplacement, même en moitié basse (où `to = from + 1`
            // passerait le garde `to != from`) — sinon on émettrait un mouvement nul + une annonce.
            let self_drop = target == Some(*id);
            let message = match to {
                Some(to) if to != *from && !self_drop => tree
                    .and_then(|t| find_widget(t.as_ref(), *id))
                    .and_then(|widget| widget.on_reorder(to)),
                _ => None,
            };
            if let Some(message) = message {
                let to = to.unwrap_or(*from);
                let axis = tree
                    .and_then(|t| find_widget(t.as_ref(), *id))
                    .map(|w| w.reorder_axis())
                    .unwrap_or(ReorderAxis::Horizontal);
                self.dispatch(message);
                // Déplacement énoncé au lecteur d'écran (pendant du fantôme, pour l'utilisateur non
                // voyant). L'index dépend de l'axe : à l'horizontale `to` est la **position de
                // colonne** (1-based) ; à la verticale c'est un index **plat** (col×STRIDE+pos) qui
                // n'a pas de sens à lire — on énonce alors le déplacement sans numéro.
                let announcement = match axis {
                    ReorderAxis::Horizontal => format!("Column moved to position {}", to + 1),
                    ReorderAxis::Vertical => "Card moved".to_string(),
                };
                self.set_announcement(announcement);
            }
        }
        // Fling : l'élan du doigt projette une destination balistique (friction),
        // le ressort de défilement existant y glisse (rebond aux bornes compris).
        if let Some(Drag::Scroll { id, moved: true, velocity, .. }) = &ended {
            self.fling(*id, *velocity);
        }
        // Fling de pan : l'élan lance le contenu, décéléré et borné par
        // `advance_interactive` (frame par frame).
        if let Some(Drag::Pan { id, moved: true, velocity, .. }) = &ended {
            if velocity.0.hypot(velocity.1) > PAN_FLING_MIN {
                self.runtime.interactive_velocity.insert(*id, *velocity);
            }
        }
        if ended.is_some() && !was_tap {
            self.request_redraw();
            return;
        }
        // Le clic n'est validé que si press et release sont sur le même widget.
        let released = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        let (message, announce) = match (self.runtime.input.pressed, released) {
            (Some(pressed), Some(released)) if pressed == released => {
                // Clic **positionnel** (sous-région, p. ex. suffixe cliquable d'un champ) :
                // prioritaire sur `on_click`. Coordonnées locales = curseur − coin du widget.
                let positional = self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.widget_rect(released))
                    .and_then(|rect| {
                        self.tree
                            .as_ref()
                            .and_then(|tree| find_widget(tree.as_ref(), released))
                            .and_then(|widget| {
                                widget.positional_click(
                                    self.cursor.x - rect.x,
                                    self.cursor.y - rect.y,
                                    rect.width,
                                    rect.height,
                                )
                            })
                    });
                let message =
                    positional.or_else(|| self.ui.as_ref().and_then(|ui| ui.msg_for(pressed)));
                // Annonce vocale de l'effet (tri, sélection…), lue sur le widget cliqué
                // avant que `dispatch` ne reconstruise l'arbre.
                let announce = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), pressed))
                    .and_then(|widget| widget.announce());
                (message, announce)
            }
            _ => (None, None),
        };
        self.runtime.input.pressed = None;
        if let Some(message) = message {
            self.dispatch(message);
            if let Some(announce) = announce {
                self.set_announcement(announce);
            }
        }
        self.request_redraw();
    }

    /// Énonce un message à voix haute via la **région live** du lecteur d'écran
    /// (réordonnancement de colonne…). Sans lecteur d'écran actif, sans coût. Le
    /// texte n'est re-énoncé qu'au changement. Bureau uniquement (no-op ailleurs).
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    fn set_announcement(&mut self, message: String) {
        self.announce = message;
    }
    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    fn set_announcement(&mut self, _message: String) {}

    /// Retour du focus à la fermeture d'un overlay : si le widget focalisé a **disparu**
    /// de la frame (menu/modale refermé), revient au **déclencheur** — le focusable présent
    /// le plus récent de l'historique. Enregistre au passage chaque transition (l'ancien
    /// focus, encore présent, devient un déclencheur candidat). Anneau de focus rendu à la
    /// frame suivante (`request_redraw` si le focus a bougé).
    fn reconcile_focus(&mut self) {
        let present: std::collections::HashSet<WidgetId> = match self.ui.as_ref() {
            Some(ui) => ui.focusable_ids().collect(),
            None => return,
        };
        let before = self.runtime.input.focused;
        let after = resolve_focus(before, &present, &mut self.focus_history, &mut self.prev_focus);
        if after != before {
            self.runtime.input.focused = after;
            self.request_redraw();
        }
    }

    /// Applique un message à l'application, exécute ses effets, puis réévalue les
    /// souscriptions (l'état a pu changer celles qui doivent tourner).
    fn dispatch(&mut self, message: A::Message) {
        // L'app a (potentiellement) changé d'état → la `view` doit être reconstruite.
        self.build_dirty = true;
        let command = self.app.update(message);
        self.run_command(command);
        self.sync_subscriptions();
    }

    /// Exécute une commande : chaque tâche part en arrière-plan (thread natif, ou
    /// `spawn_local` microtask sur le Web) et son message éventuel revient dans la
    /// boucle via le proxy ; les demandes de focus sont **mises en attente** puis
    /// résolues contre l'arbre fraîchement construit à la prochaine frame.
    ///
    /// Sur le Web, le travail des tâches reste **synchrone** (le type `Task` ne peut
    /// pas `await`) — un effet réellement asynchrone (fetch réseau) demandera une
    /// variante `Command` dédiée.
    fn run_command(&mut self, command: crate::command::Command<A::Message>) {
        let (tasks, focus) = command.into_parts();
        self.pending_focus.extend(focus);
        for task in tasks {
            let proxy = self.proxy.clone();
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                if let Some(message) = task() {
                    let _ = proxy.send_event(message);
                }
            });
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(message) = task() {
                    let _ = proxy.send_event(message);
                }
            });
        }
    }

    /// Diffe les souscriptions déclarées par l'app contre celles en cours :
    /// démarre les nouvelles, arrête (drop du `Sender`) celles disparues.
    fn sync_subscriptions(&mut self) {
        let entries = self.app.subscription().into_entries();
        let declared: std::collections::HashSet<u64> = entries.iter().map(|e| e.id).collect();

        // Arrête les souscriptions qui ne sont plus déclarées.
        self.running_subs.retain(|id, _| declared.contains(id));

        // Démarre les nouvelles.
        for entry in entries {
            if self.running_subs.contains_key(&entry.id) {
                continue;
            }
            let handle = self.start_subscription(entry.kind);
            self.running_subs.insert(entry.id, handle);
        }
    }

    /// Démarre une souscription sur un thread de fond ; renvoie sa poignée
    /// d'annulation (drop du `Sender` → le thread sort au prochain réveil).
    #[cfg(not(target_arch = "wasm32"))]
    fn start_subscription(&self, kind: crate::subscription::Kind<A::Message>) -> SubHandle {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let proxy = self.proxy.clone();
        match kind {
            crate::subscription::Kind::Every { interval, make } => {
                std::thread::spawn(move || loop {
                    match rx.recv_timeout(interval) {
                        // Intervalle écoulé : émet le message.
                        Err(RecvTimeoutError::Timeout) => {
                            if proxy.send_event(make(Instant::now())).is_err() {
                                break;
                            }
                        }
                        // Annulation (Sender droppé) ou boucle fermée : on sort.
                        _ => break,
                    }
                });
            }
        }
        tx
    }

    /// Démarre une souscription sur le **Web** : un `setInterval` navigateur (pas de
    /// thread). Renvoie sa poignée d'annulation (`clearInterval` au drop). Le proxy
    /// réinjecte le message dans la boucle à chaque tick.
    #[cfg(target_arch = "wasm32")]
    fn start_subscription(&self, kind: crate::subscription::Kind<A::Message>) -> SubHandle {
        let proxy = self.proxy.clone();
        match kind {
            crate::subscription::Kind::Every { interval, make } => {
                let ms = interval.as_millis() as i32;
                web_timer::Interval::new(ms, move || {
                    let _ = proxy.send_event(make(Instant::now()));
                })
            }
        }
    }

    /// Applique le glissement souris en cours.
    fn handle_drag(&mut self) {
        let Some(mut drag) = self.drag.take() else {
            return;
        };
        match &mut drag {
            Drag::Scrollbar {
                id,
                vertical,
                grab,
                track_start,
                track_len,
                thumb_len,
                max,
            } => {
                let along = if *vertical { self.cursor.y } else { self.cursor.x };
                let travel = (*track_len - *thumb_len).max(1.0);
                let thumb_start = (along - *grab).clamp(*track_start, *track_start + travel);
                let offset = ((thumb_start - *track_start) / travel * *max).clamp(0.0, *max);
                let entry = self.runtime.scroll.entry(*id).or_insert((0.0, 0.0));
                if *vertical {
                    entry.1 = offset;
                } else {
                    entry.0 = offset;
                }
                // Glissement précis : la cible suit l'offset, l'inertie est coupée.
                let synced = *entry;
                self.runtime.scroll_target.insert(*id, synced);
                self.runtime.scroll_velocity.remove(&*id);
            }
            Drag::TextSelect { id, rect } => {
                let local_x = self.cursor.x - rect.x;
                let local_y =
                    self.cursor.y - rect.y + self.runtime.scroll.get(id).map(|s| s.1).unwrap_or(0.0);
                // Le champ est focalisé pendant le drag : défilement depuis le curseur courant.
                let scroll_cursor = self.runtime.edits.get(id).map(|e| e.cursor).unwrap_or(0);
                let cursor = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), *id))
                    .and_then(|widget| widget.cursor_at(local_x, local_y, rect.width, scroll_cursor));
                if let Some(cursor) = cursor {
                    let edit = self.runtime.edits.entry(*id).or_default();
                    if edit.anchor.is_none() {
                        edit.anchor = Some(edit.cursor);
                    }
                    edit.cursor = cursor;
                }
            }
            Drag::Widget { id, rect, last_x } => {
                let dx = self.cursor.x - *last_x;
                *last_x = self.cursor.x;
                self.apply_widget_drag(*id, *rect, dx);
            }
            Drag::Reorder { start, moved, .. } => {
                // Au-delà du seuil, c'est un vrai glissement (plus un tri).
                let dx = self.cursor.x - start.x;
                let dy = self.cursor.y - start.y;
                if !*moved && (dx * dx + dy * dy) > TOUCH_SLOP * TOUCH_SLOP {
                    *moved = true;
                }
                // La carte fantôme suit le curseur : redessiner à chaque déplacement.
                if *moved {
                    self.request_redraw();
                }
            }
            Drag::Pan { id, last, moved, velocity, last_t, viewport } => {
                let dx = self.cursor.x - last.x;
                let dy = self.cursor.y - last.y;
                if !*moved && (dx * dx + dy * dy) > TOUCH_SLOP * TOUCH_SLOP {
                    *moved = true;
                }
                if *moved {
                    let view = self.runtime.interactive.entry(*id).or_default();
                    // Le doigt pousse le contenu, borné au cadre.
                    *view = view.pan(dx, dy).clamped(*viewport);
                    // Vitesse lissée (moyenne exponentielle) → l'élan du fling.
                    let now = Instant::now();
                    let dt = (now - *last_t).as_secs_f32();
                    if dt > 1e-4 {
                        velocity.0 = velocity.0 * 0.5 + (dx / dt) * 0.5;
                        velocity.1 = velocity.1 * 0.5 + (dy / dt) * 0.5;
                    }
                    *last_t = now;
                    *last = self.cursor;
                }
            }
            Drag::Scroll { id, last, moved, velocity, last_t } => {
                let dx = self.cursor.x - last.x;
                let dy = self.cursor.y - last.y;
                // Sous le seuil, on ne défile pas encore (le geste peut être un tap).
                if !*moved && (dx * dx + dy * dy) > TOUCH_SLOP * TOUCH_SLOP {
                    *moved = true;
                }
                if *moved {
                    let maxes = self
                        .ui
                        .as_ref()
                        .map(|u| u.scrollable_maxes())
                        .unwrap_or_default();
                    let (mx, my) = maxes
                        .iter()
                        .find(|(i, _, _)| *i == *id)
                        .map(|(_, x, y)| (*x, *y))
                        .unwrap_or((0.0, 0.0));
                    let cur = self.runtime.scroll.get(id).copied().unwrap_or((0.0, 0.0));
                    // Le doigt « pousse » le contenu : on suit le delta immédiatement.
                    let nx = (cur.0 - dx).clamp(0.0, mx);
                    let ny = (cur.1 - dy).clamp(0.0, my);
                    self.runtime.scroll.insert(*id, (nx, ny));
                    self.runtime.scroll_target.insert(*id, (nx, ny));
                    self.runtime.scroll_velocity.remove(id);
                    // Vitesse en espace défilement (le contenu va à l'opposé du
                    // doigt), lissée par moyenne exponentielle — l'élan du fling.
                    let now = Instant::now();
                    let dt = (now - *last_t).as_secs_f32();
                    if dt > 1e-4 {
                        let inst = (-dx / dt, -dy / dt);
                        velocity.0 = velocity.0 * 0.5 + inst.0 * 0.5;
                        velocity.1 = velocity.1 * 0.5 + inst.1 * 0.5;
                    }
                    *last_t = now;
                    *last = self.cursor;
                }
            }
            Drag::Back {
                start_x,
                last_x,
                last_t,
                velocity,
            } => {
                let scale = self.total_scale();
                let width = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().width as f32 / scale)
                    .unwrap_or(1.0)
                    .max(1.0);
                let now = Instant::now();
                let x = self.cursor.x;
                // En RTL, le doigt glisse vers la **gauche** depuis le bord droit :
                // la progression (et la vélocité) suivent le sens inverse.
                let sign = if self.is_rtl() { -1.0 } else { 1.0 };
                let progress = (sign * (x - *start_x) / width).clamp(0.0, 1.0);
                let dt = (now - *last_t).as_secs_f32();
                if dt > 1e-4 {
                    // Vitesse instantanée (fraction/s), lissée par moyenne exponentielle.
                    let inst = sign * (x - *last_x) / width / dt;
                    *velocity = *velocity * 0.5 + inst * 0.5;
                    *last_x = x;
                    *last_t = now;
                }
                self.build_dirty = true;
                self.app.back_gesture(progress);
            }
        }
        self.drag = Some(drag);
        self.request_redraw();
    }

    /// Fling de défilement : projette la destination balistique de chaque axe
    /// (friction en forme close), bornée avec le dépassement élastique, et amorce
    /// le ressort de défilement avec l'élan du doigt.
    fn fling(&mut self, id: WidgetId, velocity: (f32, f32)) {
        let (max_x, max_y) = self
            .ui
            .as_ref()
            .map(|ui| ui.scrollable_maxes())
            .unwrap_or_default()
            .iter()
            .find(|(i, _, _)| *i == id)
            .map(|(_, x, y)| (*x, *y))
            .unwrap_or((0.0, 0.0));
        let current = self.runtime.scroll.get(&id).copied().unwrap_or((0.0, 0.0));

        let dest_x = crate::gesture::fling_destination(current.0, velocity.0)
            .map(|d| d.clamp(-SCROLL_OVER, max_x + SCROLL_OVER));
        let dest_y = crate::gesture::fling_destination(current.1, velocity.1)
            .map(|d| d.clamp(-SCROLL_OVER, max_y + SCROLL_OVER));
        if dest_x.is_none() && dest_y.is_none() {
            return; // relâchement lent : pas d'entraînement.
        }
        self.runtime
            .scroll_target
            .insert(id, (dest_x.unwrap_or(current.0), dest_y.unwrap_or(current.1)));
        self.runtime.scroll_velocity.insert(id, velocity);
    }

    /// Applique un glissement de widget : calcule la fraction horizontale et
    /// route le message produit par `on_drag`.
    /// Widget **réordonnable** le plus au-dessus sous `point` : `(id, index plat)`. Utilise le registre
    /// des réordonnables (indépendant du clic) — donc aussi les cartes/zones Kanban, non cliquables.
    fn reorderable_at(&self, point: Point) -> Option<(WidgetId, usize)> {
        let id = self.ui.as_ref()?.reorderable_at(point)?;
        let widget = self.tree.as_ref().and_then(|tree| find_widget(tree.as_ref(), id))?;
        // Cible **seule** (zone de dépôt) : réordonnable mais non saisissable — on ne démarre pas de
        // glisser dessus (le dépôt, lui, continue de la viser via `ui.reorderable_at`).
        if !widget.reorder_draggable() {
            return None;
        }
        let from = widget.reorder_index()?;
        Some((id, from))
    }

    /// Axe du réordonnable actuellement **saisi** (glisser en cours), s'il y en a un. Sert à
    /// n'appliquer le **ressort horizontal** (`reorder_x`) qu'aux colonnes de `Table` — les cartes
    /// Kanban (vertical) réagencent sans lissage x.
    fn dragged_reorder_axis(&self) -> Option<ReorderAxis> {
        let Some(Drag::Reorder { id, .. }) = self.drag else {
            return None;
        };
        self.tree.as_ref().and_then(|t| find_widget(t.as_ref(), id)).map(|w| w.reorder_axis())
    }

    /// Peint l'**aperçu de réordonnancement** par-dessus la scène (non découpée) :
    /// colonne source estompée, **indicateur de dépôt** au bord d'insertion de la
    /// colonne cible, et **carte soulevée** (ombre + bord `primary`) suivant le
    /// curseur. Sans effet hors d'un glissement d'en-tête engagé.
    fn paint_reorder_preview(&self, ui: &Ui<A::Message>, theme: &Theme, scene: &mut Scene) {
        let Some(Drag::Reorder { id, from: _, start, moved: true }) = self.drag else {
            return;
        };
        let Some(src) = ui.widget_rect(id) else {
            return;
        };
        let axis = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(|w| w.reorder_axis())
            .unwrap_or(ReorderAxis::Horizontal);
        let dx = self.cursor.x - start.x;
        let dy = self.cursor.y - start.y;

        // Décalage du fantôme selon l'axe : **horizontal** (colonnes de Table) suit `dx` (léger
        // soulèvement `-2`) ; **vertical** (cartes Kanban) suit le curseur en 2D.
        let (gx, gy) = match axis {
            ReorderAxis::Horizontal => (dx, drag_preview::LIFT_Y),
            ReorderAxis::Vertical => (dx, dy),
        };

        // Propriétaires du **sous-arbre** de l'élément saisi : sert au fantôme (capter le contenu
        // d'une carte riche, peint par des enfants, jalon 251) **et** au réagencement vertical
        // (retirer la carte soulevée de l'aperçu). Repli : la carte seule.
        let owners: std::collections::HashSet<u64> = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), id))
            .map(|w| subtree_ids(w, id).iter().map(|i| i.as_u64()).collect())
            .unwrap_or_else(|| std::iter::once(id.as_u64()).collect());

        match axis {
            ReorderAxis::Horizontal => {
                // Réagence les colonnes voisines : le trou de la source se referme et la place de
                // dépôt s'ouvre, selon l'abscisse **lissée** (ressort) du curseur — coulissement à
                // inertie douce, tandis que le fantôme colle au curseur réel.
                let reflowed = reflow_reorder_columns(scene.primitives(), src, self.reorder_x, id.as_u64());
                scene.clear();
                for primitive in reflowed {
                    scene.push_primitive(primitive);
                }
            }
            ReorderAxis::Vertical => {
                // Réagence les **cartes** : le trou de la carte soulevée se referme (colonne source)
                // et la place s'ouvre sous la **ligne d'insertion** (colonne cible). Puis on pose la
                // ligne par-dessus, au bord retenu (moitié survolée, jalon 252).
                let line = self.reorder_drop_line(drag_preview::INSERT_THICKNESS);
                let reflowed = reflow_reorder_cards(scene.primitives(), src, line, &owners);
                scene.clear();
                for primitive in reflowed {
                    scene.push_primitive(primitive);
                }
                if let Some(line) = line {
                    scene.set_clip(Rect::UNBOUNDED);
                    scene.draw_rect(line, theme.primary, theme.radius.min(line.height * 0.5), 0.0, Color::TRANSPARENT);
                }
            }
        }

        // Fantôme fidèle : les primitives de l'élément saisi — depuis la scène d'origine —,
        // translatées et **dé-découpées** (sinon rognées à la source). Pour une **carte riche**,
        // le fond est peint par la carte mais son contenu (libellé, étiquettes, bouton ×) par des
        // enfants, sous d'autres propriétaires : on capte donc les primitives de **tout le
        // sous-arbre** de l'élément (voir `owners` ci-dessus).
        let ghost: Vec<Primitive> = ui
            .scene()
            .primitives()
            .iter()
            .filter(|p| owners.contains(&p.owner()))
            .map(|p| p.translated(gx, gy).with_clip(Rect::UNBOUNDED))
            .collect();
        draw_ghost_card(scene, theme, src.translate(gx, gy), &ghost);
    }

    /// Ligne d'**insertion** (aperçu vertical) : un fin bandeau au bord de l'emplacement réordonnable
    /// survolé (carte ou zone de dépôt) — **supérieur** si le curseur est dans sa moitié haute
    /// (insertion **avant**), **inférieur** dans sa moitié basse (insertion **après**). `None` si le
    /// curseur n'est pas sur une cible.
    fn reorder_drop_line(&self, thickness: f32) -> Option<Rect> {
        // Emplacement réordonnable (carte/zone de dépôt) sous le curseur, via le registre dédié.
        let target = self.ui.as_ref()?.reorderable_at(self.cursor)?;
        let rect = self.ui.as_ref()?.widget_rect(target)?;
        Some(drop_insertion_line(rect, thickness, self.reorder_insert_after(target, rect)))
    }

    /// Pour un emplacement à réordonnancement **vertical**, indique si le curseur est dans sa moitié
    /// **basse** — donc si l'insertion se fait **après** lui (index +1 : entre cette carte et la
    /// suivante). Toujours `false` sur l'axe **horizontal** (les colonnes de `Table` gardent leur
    /// logique de dépôt inchangée). C'est le pendant, côté **routage**, de la ligne d'insertion.
    fn reorder_insert_after(&self, target: WidgetId, rect: Rect) -> bool {
        let vertical = self
            .tree
            .as_ref()
            .and_then(|t| find_widget(t.as_ref(), target))
            .map(|w| matches!(w.reorder_axis(), ReorderAxis::Vertical))
            .unwrap_or(false);
        vertical && rect.height > 0.0 && self.cursor.y > rect.y + rect.height * 0.5
    }

    fn apply_widget_drag(&mut self, id: WidgetId, rect: frus_widgets::Rect, dx: f32) {
        let fraction = if rect.width > 0.0 {
            ((self.cursor.x - rect.x) / rect.width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // Poignée à accumulation (delta) d'abord, sinon curseur (fraction absolue).
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.on_drag_delta(dx).or_else(|| widget.on_drag(fraction)));
        if let Some(message) = message {
            self.dispatch(message);
        }
    }

    /// Retour **système** (Android `KEYCODE_BACK`, touche « précédent ») — la
    /// chaîne native : ① l'overlay du dessus se ferme (feuille, tiroir, modale,
    /// menu) ; ② sinon un écran se dépile, en rejouant le **geste retour validé
    /// d'emblée** (mêmes hooks que le swipe → détente animée) ; ③ sinon, à la
    /// racine, le retour **quitte l'application** (comportement Android).
    fn system_back(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(message) = self.ui.as_ref().and_then(|ui| ui.top_dismiss()) {
            self.dispatch(message);
            self.request_redraw();
            return;
        }
        if self.app.can_go_back() {
            self.build_dirty = true;
            self.app.back_gesture(0.0);
            // Élan « validé » : la projection dépasse le seuil de commit, la
            // détente anime le dépilement.
            self.app.back_gesture_end(5.0);
            self.request_redraw();
            return;
        }
        event_loop.exit();
    }

    /// Route **Échap** : montée feuille→racine depuis le widget focalisé
    /// (résultat à 3 états — `Ignored` continue de remonter, `Handled` consomme,
    /// `Skip` arrête sans repli), puis repli sur la fermeture de l'overlay le
    /// plus au-dessus si personne n'a répondu (ou sans focus).
    fn escape(&mut self) {
        // 1) Montée le long du chemin de focus. `Some(None)` = consommé sans
        // message ; `None` extérieur = tout le chemin a ignoré → repli.
        let outcome: Option<Option<A::Message>> =
            self.runtime.input.focused.and_then(|focused| {
                let tree = self.tree.as_ref()?;
                let path = find_path(tree.as_ref(), focused);
                for widget in path.iter().rev() {
                    match widget.on_key(&Key::Escape) {
                        KeyResponse::Handled(message) => return Some(message),
                        KeyResponse::Skip => return Some(None),
                        KeyResponse::Ignored => {}
                    }
                }
                None
            });

        match outcome {
            Some(message) => {
                if let Some(message) = message {
                    self.dispatch(message);
                    self.request_redraw();
                }
            }
            // 2) Personne sur le chemin (ou pas de focus) : overlay du dessus.
            None => {
                if let Some(message) = self.ui.as_ref().and_then(|ui| ui.top_dismiss()) {
                    self.dispatch(message);
                    self.request_redraw();
                }
            }
        }
    }

    /// Déplace le caret verticalement dans le champ multi-lignes focalisé (Haut/Bas
    /// si `page=false`, PgUp/PgDn sinon), en préservant la **colonne cible mémorisée**.
    /// Applique la sélection au Shift et révèle le caret. Rend `true` si le caret a
    /// bougé dans le champ, `false` sinon (le shell navigue alors le focus).
    fn move_caret_vertical(&mut self, id: WidgetId, down: bool, page: bool) -> bool {
        let width = self
            .ui
            .as_ref()
            .and_then(|ui| ui.widget_rect(id))
            .map(|r| r.width)
            .unwrap_or(0.0);
        let cursor = self.runtime.edits.get(&id).map(|e| e.cursor).unwrap_or(0);
        let moved = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.caret_vertical(width, cursor, down, page, self.goal_x));
        let Some((new_cursor, goal)) = moved else {
            return false;
        };
        let shift = self.shift;
        let edit = self.runtime.edits.entry(id).or_default();
        if shift {
            if edit.anchor.is_none() {
                edit.anchor = Some(edit.cursor);
            }
        } else {
            edit.anchor = None;
        }
        edit.cursor = new_cursor;
        // Mémorise la colonne visée pour le prochain saut vertical.
        self.goal_x = Some(goal);
        self.reveal_caret(id, new_cursor);
        self.request_redraw();
        true
    }

    /// Route une touche vers le champ focalisé : met à jour l'état d'édition et
    /// applique le message éventuel (changement de valeur ou soumission).
    fn apply_key(&mut self, id: WidgetId, key: Key) {
        // Tout déplacement horizontal / frappe oublie la colonne cible verticale.
        self.goal_x = None;
        let mut edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.on_edit(&mut edit, &key));
        self.runtime.edits.insert(id, edit);
        // Champ multi-lignes : fait suivre le défilement retenu au caret (le révèle).
        self.reveal_caret(id, edit.cursor);
        if let Some(message) = message {
            self.dispatch(message);
            // Les touches peuvent arriver en **rafale** (plus vite qu'une
            // frame — clavier logiciel, `adb input text`, répétition) : la
            // suivante doit voir la valeur À JOUR, pas celle de l'arbre
            // retenu (sinon elle écrase la frappe précédente). On rafraîchit
            // l'arbre tout de suite ; `build_dirty` reste levé, la prochaine
            // frame refera la passe complète (montages, fondus de sortie).
            if let Some((width, height)) = self.last_size {
                let theme = self.app.theme();
                self.tree = Some(self.app.view(&theme, width, height));
            }
        }
    }

    /// Fait **suivre le caret** au défilement retenu d'un champ multi-lignes : ajuste
    /// `runtime.scroll[id]` juste assez pour que le caret reste visible (comme un
    /// éditeur qui recentre à la frappe). No-op pour un champ non défilable.
    fn reveal_caret(&mut self, id: WidgetId, cursor: usize) {
        let Some(vp) = self.ui.as_ref().and_then(|ui| ui.scrollable_viewport(id)) else {
            return;
        };
        let metrics = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.text_metrics(vp.width, cursor));
        let Some((content_h, visible_h, caret_top, caret_h)) = metrics else {
            return;
        };
        let max_y = (content_h - visible_h).max(0.0);
        let cur = self.runtime.scroll.get(&id).map(|s| s.1).unwrap_or(0.0);
        // Fenêtre de défilement où le caret reste visible : du « bas du caret visible »
        // au « haut du caret visible ». On y ramène le défilement courant.
        let lo = (caret_top + caret_h - visible_h).max(0.0);
        let hi = caret_top;
        let target = cur.clamp(lo.min(hi), lo.max(hi)).clamp(0.0, max_y);
        self.runtime.scroll.insert(id, (0.0, target));
        self.runtime.scroll_target.insert(id, (0.0, target));
        self.runtime.scroll_velocity.remove(&id);
    }

    /// Copie le texte sélectionné du champ `id` dans le presse-papier.
    fn copy_selection(&mut self, id: WidgetId) {
        let edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let text = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.selected_text(&edit));
        if let Some(text) = text {
            self.clipboard.set_text(text);
        }
    }
}

/// Rapproche `current` de `target` d'un pas de ressort **exponentiel** (constante de
/// temps `tau`, en secondes) sur un intervalle `dt` : cadence-indépendant et sans
/// dépassement. Sert au lissage de l'abscisse pendant un réordonnancement.
fn spring_toward(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    let k = 1.0 - (-dt / tau.max(1e-4)).exp();
    current + (target - current) * k
}

/// Profondeur max de l'historique de focus (déclencheurs d'overlays imbriqués).
const FOCUS_HISTORY_MAX: usize = 8;

/// Résout le focus après (re)construction, à partir des focusables **présents** cette
/// frame. Si `current` a **disparu** (overlay fermé), revient au **déclencheur** présent le
/// plus récent de `history` (dépilé jusqu'à en trouver un). Enregistre la transition :
/// l'ancien focus (`prev`), s'il est encore présent et différent du nouveau, devient un
/// déclencheur candidat empilé (borné à [`FOCUS_HISTORY_MAX`]). Renvoie le focus résolu.
/// Fonction **pure** (hors `history`/`prev` mutés) — testable sans fenêtre.
fn resolve_focus(
    current: Option<WidgetId>,
    present: &std::collections::HashSet<WidgetId>,
    history: &mut Vec<WidgetId>,
    prev: &mut Option<WidgetId>,
) -> Option<WidgetId> {
    let mut cur = current;
    if let Some(c) = cur {
        if !present.contains(&c) {
            cur = None;
            while let Some(cand) = history.pop() {
                if present.contains(&cand) {
                    cur = Some(cand);
                    break;
                }
            }
        }
    }
    if *prev != cur {
        if let Some(old) = *prev {
            if present.contains(&old) && Some(old) != cur {
                history.push(old);
                if history.len() > FOCUS_HISTORY_MAX {
                    history.remove(0);
                }
            }
        }
        *prev = cur;
    }
    cur
}

/// Peint la **carte soulevée** (fantôme) de l'en-tête glissé dans `scene` (non découpé)
/// à `card` : ombre portée, **face fidèle** (primitives de l'en-tête déjà translatées)
/// ou pleine en repli si `ghost` est vide, puis bord `primary`. Fonction pure — testable
/// sans GPU. Le coulissement des voisines est fait en amont (`reflow_reorder_columns`).
fn draw_ghost_card(scene: &mut Scene, theme: &Theme, card: Rect, ghost: &[Primitive]) {
    use drag_preview::{BORDER_ALPHA, BORDER_WIDTH, SHADOW_ALPHA, SHADOW_BLUR, SHADOW_OFFSET_Y};
    scene.set_clip(Rect::UNBOUNDED);
    // Couleur d'ombre **du thème** (surchargeable) — même rôle que l'ombre de `Button` ; seule la
    // géométrie (décalage, flou, opacité) est en constantes locales.
    let shadow = theme.scheme.shadow.with_alpha(SHADOW_ALPHA);
    scene.shadow(card.translate(0.0, SHADOW_OFFSET_Y), shadow, theme.radius, SHADOW_BLUR);
    let border = theme.primary.fade(BORDER_ALPHA);
    if ghost.is_empty() {
        scene.draw_rect(card, theme.surface, theme.radius, BORDER_WIDTH, border);
    } else {
        scene.draw_rect(card, theme.surface, theme.radius, 0.0, Color::TRANSPARENT);
        for primitive in ghost {
            scene.push_primitive(primitive.clone());
        }
        scene.draw_rect(card, Color::TRANSPARENT, theme.radius, BORDER_WIDTH, border);
    }
}

/// Géométrie de la **ligne d'insertion** d'un aperçu de réordonnancement **vertical** : un fin
/// bandeau (épaisseur `thickness`) centré sur le bord de la cible **où se fera l'insertion** — bord
/// **supérieur** (insertion avant, moitié haute survolée) ou **inférieur** (insertion après,
/// `after = true`, moitié basse survolée), sur toute la largeur. Fonction pure — testable sans GPU.
fn drop_insertion_line(target: Rect, thickness: f32, after: bool) -> Rect {
    let edge = if after { target.y + target.height } else { target.y };
    Rect::new(target.x, edge - thickness * 0.5, target.width, thickness)
}

#[cfg(test)]
mod tests {
    use super::{draw_ghost_card, drop_insertion_line, resolve_focus, spring_toward, Rect, Scene, Theme};
    use frus_widgets::WidgetId;
    use std::collections::HashSet;

    #[test]
    fn insertion_line_sits_on_the_target_top_edge() {
        // Moitié haute survolée → insertion **avant** : bandeau d'épaisseur 4 centré sur le bord
        // supérieur (y=100) d'une cible large de 200.
        let line = drop_insertion_line(Rect::new(20.0, 100.0, 200.0, 44.0), 4.0, false);
        assert_eq!(line.x, 20.0, "aligné à gauche de la cible");
        assert_eq!(line.width, 200.0, "toute la largeur de la cible");
        assert_eq!(line.y, 98.0, "centré sur le bord supérieur (100 - 4/2)");
        assert_eq!(line.height, 4.0);
    }

    #[test]
    fn insertion_line_sits_on_the_target_bottom_edge_when_inserting_after() {
        // Moitié basse survolée → insertion **après** : le bandeau glisse sur le bord **inférieur**
        // (y = 100 + 44 = 144, centré → 142). Même largeur, même épaisseur.
        let line = drop_insertion_line(Rect::new(20.0, 100.0, 200.0, 44.0), 4.0, true);
        assert_eq!(line.x, 20.0, "toujours aligné à gauche");
        assert_eq!(line.width, 200.0, "toute la largeur de la cible");
        assert_eq!(line.y, 142.0, "centré sur le bord inférieur (144 - 4/2)");
        assert_eq!(line.height, 4.0);
    }

    #[test]
    fn focus_returns_to_trigger_when_overlay_closes() {
        let anchor = WidgetId::from_u64(1);
        let item = WidgetId::from_u64(2);
        let (mut history, mut prev) = (Vec::new(), None);

        // Frame 1 : focus sur l'ancre (présente).
        let f = resolve_focus(Some(anchor), &HashSet::from([anchor]), &mut history, &mut prev);
        assert_eq!(f, Some(anchor));
        assert!(history.is_empty());

        // Frame 2 : menu ouvert, focus sur l'item ; l'ancre est empilée (déclencheur).
        let present = HashSet::from([anchor, item]);
        let f = resolve_focus(Some(item), &present, &mut history, &mut prev);
        assert_eq!(f, Some(item));
        assert_eq!(history, vec![anchor]);

        // Frame 3 : menu fermé, l'item a disparu → retour au déclencheur, historique consommé.
        let f = resolve_focus(Some(item), &HashSet::from([anchor]), &mut history, &mut prev);
        assert_eq!(f, Some(anchor), "le focus revient au déclencheur");
        assert!(history.is_empty());
    }

    #[test]
    fn focus_falls_to_none_when_no_trigger_remains() {
        let a = WidgetId::from_u64(1);
        let (mut history, mut prev) = (Vec::new(), Some(a));
        // `a` disparaît et l'historique est vide → aucun focus.
        let f = resolve_focus(Some(a), &HashSet::new(), &mut history, &mut prev);
        assert_eq!(f, None);
        assert_eq!(prev, None);
    }

    #[test]
    fn spring_approaches_target_monotonically_and_settles() {
        // Part de 0, cible 100 : approche monotone sans dépassement, quasi atteinte
        // après plusieurs pas de 16 ms.
        let mut x = 0.0;
        let mut prev = -1.0;
        for _ in 0..30 {
            x = spring_toward(x, 100.0, 0.016, 0.07);
            assert!(x > prev && x <= 100.0, "monotone borné : {x}");
            prev = x;
        }
        assert!(x > 99.0, "quasi atteint après ~0.5 s : {x}");
    }

    #[test]
    fn ghost_card_shape() {
        let theme = Theme::default();
        let card = Rect::new(140.0, 0.0, 80.0, 34.0);
        // Fantôme vide → repli : ombre + carte pleine bordée = 2 primitives.
        let mut scene = Scene::new();
        draw_ghost_card(&mut scene, &theme, card, &[]);
        assert_eq!(scene.primitives().len(), 2, "ombre + carte pleine");
    }
}
