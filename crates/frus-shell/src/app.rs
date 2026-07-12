//! Le pilote générique : implémente [`winit::application::ApplicationHandler`]
//! pour n'importe quelle [`Application`].
//!
//! Le framework possède la fenêtre, le renderer, le [`Runtime`] (état
//! d'interaction retenu : survol/focus/scroll/édition/animations), le routage des
//! entrées par hit-test, le glissement (barres, sélection, poignées, geste retour)
//! et l'horloge d'animation. L'application ne fournit que `update`/`view`/… .

use std::collections::HashMap;
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Instant;

use frus_gpu::{wgpu, Renderer};
use frus_widgets::{
    build_ui, collect_ids, find_widget, Edit, Insets, Key, Point, Runtime, Size, Ui, Widget,
    WidgetId,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

use crate::application::Application;

/// Presse-papier : `arboard` sur les plateformes de bureau, no-op sur Android
/// (pas de dépendance `arboard`, qui ne s'y compile pas). Une API uniforme pour
/// que le corps du pilote reste sans `cfg`.
mod clip {
    #[cfg(not(target_os = "android"))]
    pub struct Clipboard(Option<arboard::Clipboard>);

    #[cfg(not(target_os = "android"))]
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

    #[cfg(target_os = "android")]
    pub struct Clipboard;

    #[cfg(target_os = "android")]
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

/// Vitesse de défilement (pixels par cran de molette).
const SCROLL_SPEED: f32 = 40.0;

/// Seuil de mouvement (px logiques) au-delà duquel un appui tactile devient un
/// défilement plutôt qu'un tap.
const TOUCH_SLOP: f32 = 8.0;

/// Dépassement élastique autorisé au-delà des bornes de défilement (px) — rebond.
const SCROLL_OVER: f32 = 48.0;

/// Largeur (px physiques) de la zone de bord activant le geste retour.
const BACK_EDGE: f32 = 24.0;

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
    Widget { id: WidgetId, rect: frus_widgets::Rect },
    /// Défilement d'une zone scrollable au doigt (tactile). `moved` distingue un
    /// vrai défilement d'un simple tap (mouvement sous le seuil `TOUCH_SLOP`).
    Scroll { id: WidgetId, last: Point, moved: bool },
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
    /// Modificateurs clavier courants.
    shift: bool,
    ctrl: bool,
    /// Accès presse-papier (no-op sur Android).
    clipboard: clip::Clipboard,
    /// Effet de démarrage (`init`) déjà exécuté ? Évite de le rejouer quand la
    /// surface est recréée (retour d'arrière-plan sur Android).
    started: bool,
    /// Instant de la dernière frame (pour le dt des animations).
    last_frame: Option<Instant>,
    /// Glissement souris en cours.
    drag: Option<Drag>,
    /// Instant du dernier clic (détection du double-clic).
    last_click_time: Option<Instant>,
    /// Compteur pour clés d'événements de sortie (fondu de disparition).
    leaving_counter: u64,
    /// Souscriptions actives : id → poignée d'annulation (drop = arrêt).
    running_subs: HashMap<u64, Sender<()>>,
    /// Fenêtre masquée (occultée) : on suspend le rendu.
    occluded: bool,
    /// Temps écoulé cumulé (secondes), pour les animations continues.
    elapsed: f32,
    /// Derniers insets système transmis à l'app (zone de sécurité), en logique.
    last_insets: Insets,
    /// Poignée de l'activité Android (pour interroger les insets, le clavier…).
    #[cfg(target_os = "android")]
    android_app: Option<winit::platform::android::activity::AndroidApp>,
}

impl<A: Application> App<A> {
    /// Crée le pilote autour d'une application et de son canal de messages.
    pub fn new(app: A, proxy: EventLoopProxy<A::Message>) -> Self {
        Self {
            app,
            proxy,
            window: None,
            renderer: None,
            ui: None,
            tree: None,
            cursor: Point::new(0.0, 0.0),
            scale: 1.0,
            last_size: None,
            runtime: Runtime::default(),
            shift: false,
            ctrl: false,
            clipboard: clip::Clipboard::new(),
            started: false,
            last_frame: None,
            drag: None,
            last_click_time: None,
            leaving_counter: 0,
            running_subs: HashMap::new(),
            occluded: false,
            elapsed: 0.0,
            last_insets: Insets::ZERO,
            #[cfg(target_os = "android")]
            android_app: None,
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
        if let Some((w, h)) = self.app.window_size() {
            attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(w, h));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("Échec de création de la fenêtre : {err}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let renderer = pollster::block_on(Renderer::new(
            window.clone(),
            size.width.max(1),
            size.height.max(1),
        ));

        match renderer {
            Ok(renderer) => {
                self.scale = window.scale_factor() as f32;
                self.window = Some(window.clone());
                self.renderer = Some(renderer);
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

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
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

            WindowEvent::CursorMoved { position, .. } => {
                // winit fournit des px physiques ; on travaille en logique
                // (échelle totale = DPI × densité).
                let scale = self.total_scale();
                self.cursor = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                self.pointer_move();
            }

            // Écran tactile : on ramène chaque phase au même chemin que la souris
            // (un doigt = un pointeur), avec en plus le défilement au doigt.
            WindowEvent::Touch(touch) => {
                let scale = self.total_scale();
                self.cursor =
                    Point::new(touch.location.x as f32 / scale, touch.location.y as f32 / scale);
                match touch.phase {
                    TouchPhase::Started => self.pointer_down(true),
                    TouchPhase::Moved => self.pointer_move(),
                    TouchPhase::Ended => self.pointer_up(),
                    TouchPhase::Cancelled => {
                        self.drag = None;
                        self.runtime.input.pressed = None;
                        self.request_redraw();
                    }
                }
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
            } => self.pointer_down(false),

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.pointer_up(),

            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed =>
            {
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

                let Some(focused) = self.runtime.input.focused else {
                    return;
                };

                // Activation clavier (Entrée/Espace) d'un focusable cliquable
                // (bouton, case, interrupteur). Les champs texte (sans `on_click`)
                // retombent sur l'édition normale (Entrée = soumettre, Espace = espace).
                if matches!(
                    event.logical_key,
                    WinitKey::Named(NamedKey::Enter) | WinitKey::Named(NamedKey::Space)
                ) {
                    let message = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), focused))
                        .filter(|widget| widget.focusable())
                        .and_then(|widget| widget.on_click());
                    if let Some(message) = message {
                        self.dispatch(message);
                        self.request_redraw();
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
                                },
                            );
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                let shift = self.shift;
                let key = match &event.logical_key {
                    WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
                    WinitKey::Named(NamedKey::Delete) => Some(Key::Delete),
                    WinitKey::Named(NamedKey::Enter) => Some(Key::Enter),
                    WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left { shift }),
                    WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right { shift }),
                    WinitKey::Named(NamedKey::Home) => Some(Key::Home { shift }),
                    WinitKey::Named(NamedKey::End) => Some(Key::End { shift }),
                    WinitKey::Named(NamedKey::Space) => Some(Key::Text(" ".to_string())),
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
                    self.app.on_resize(width, height);
                }

                // Insets système (zone de sécurité) : notifie l'app quand ils changent.
                let insets = self.compute_insets(size.width, size.height, scale);
                if self.last_insets != insets {
                    self.last_insets = insets;
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

                // Inertie de défilement (bornes issues de la frame précédente).
                let scroll_maxes = self
                    .ui
                    .as_ref()
                    .map(|ui| ui.scrollable_maxes())
                    .unwrap_or_default();

                let animating = self.runtime.advance(dt)
                    | self.runtime.advance_leaving(dt)
                    | self.runtime.advance_values(tree.as_ref(), dt)
                    | self.runtime.advance_scroll(&scroll_maxes, dt)
                    | app_animating;
                let ui = build_ui(tree.as_ref(), Size::new(width, height), &self.runtime, &theme);

                // Scène logique → physique (DPI × densité) pour un rendu net.
                let scene = ui.scene().scaled(scale);
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

                // Conserve l'interface (hit-test) et l'arbre (routage clavier).
                self.ui = Some(ui);
                self.tree = Some(tree);

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

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Déplacement du pointeur (souris ou doigt) : poursuit un glissement en
    /// cours, sinon met à jour le survol.
    fn pointer_move(&mut self) {
        if self.drag.is_some() {
            self.handle_drag();
            return;
        }
        if let Some(ui) = &self.ui {
            let hovered = ui.hit(self.cursor);
            if hovered != self.runtime.input.hovered {
                self.runtime.input.hovered = hovered;
                self.request_redraw();
            }
        }
    }

    /// Appui du pointeur (souris ou doigt) à la position `self.cursor`. `touch`
    /// active le défilement au doigt quand aucun autre geste ne capture l'appui.
    fn pointer_down(&mut self, touch: bool) {
        // 0) Geste retour : appui sur le bord gauche, si l'app l'autorise.
        if self.cursor.x < BACK_EDGE && self.app.can_go_back() {
            self.drag = Some(Drag::Back {
                start_x: self.cursor.x,
                last_x: self.cursor.x,
                last_t: Instant::now(),
                velocity: 0.0,
            });
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
            self.drag = Some(Drag::Widget { id, rect });
            self.apply_widget_drag(id, rect);
            self.request_redraw();
            return;
        }

        self.runtime.input.pressed = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        // 2) Focus + placement du curseur, et début d'une sélection texte.
        let previously_focused = self.runtime.input.focused;
        let focus = self.ui.as_ref().and_then(|ui| ui.focus_hit(self.cursor));
        self.runtime.input.focused = focus.map(|(id, _)| id);
        if let Some((id, rect)) = focus {
            let local_x = self.cursor.x - rect.x;
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
                .and_then(|widget| widget.cursor_at(local_x, rect.width, scroll_cursor));
            if let Some(cursor) = cursor {
                self.runtime.edits.insert(id, Edit { cursor, anchor: None });
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
                self.drag = Some(Drag::Scroll { id, last: self.cursor, moved: false });
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
            self.app.back_gesture_end(velocity);
            self.request_redraw();
            return;
        }
        // Un défilement tactile qui n'a pas bougé = un simple tap : on le laisse
        // suivre le chemin normal du clic ci-dessous.
        let was_tap = matches!(ended, Some(Drag::Scroll { moved: false, .. }));
        if ended.is_some() && !was_tap {
            self.request_redraw();
            return;
        }
        // Le clic n'est validé que si press et release sont sur le même widget.
        let released = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
        let message = match (self.runtime.input.pressed, released) {
            (Some(pressed), Some(released)) if pressed == released => {
                self.ui.as_ref().and_then(|ui| ui.msg_for(pressed))
            }
            _ => None,
        };
        self.runtime.input.pressed = None;
        if let Some(message) = message {
            self.dispatch(message);
        }
        self.request_redraw();
    }

    /// Applique un message à l'application, exécute ses effets, puis réévalue les
    /// souscriptions (l'état a pu changer celles qui doivent tourner).
    fn dispatch(&mut self, message: A::Message) {
        let command = self.app.update(message);
        self.run_command(command);
        self.sync_subscriptions();
    }

    /// Exécute une commande : chaque tâche tourne sur un thread de fond ; son
    /// message éventuel est renvoyé dans la boucle via le proxy.
    fn run_command(&self, command: crate::command::Command<A::Message>) {
        for task in command.into_tasks() {
            let proxy = self.proxy.clone();
            std::thread::spawn(move || {
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
            let sender = self.start_subscription(entry.kind);
            self.running_subs.insert(entry.id, sender);
        }
    }

    /// Démarre une souscription sur un thread de fond ; renvoie sa poignée
    /// d'annulation (drop du `Sender` → le thread sort au prochain réveil).
    fn start_subscription(&self, kind: crate::subscription::Kind<A::Message>) -> Sender<()> {
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
                // Le champ est focalisé pendant le drag : défilement depuis le curseur courant.
                let scroll_cursor = self.runtime.edits.get(id).map(|e| e.cursor).unwrap_or(0);
                let cursor = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), *id))
                    .and_then(|widget| widget.cursor_at(local_x, rect.width, scroll_cursor));
                if let Some(cursor) = cursor {
                    let edit = self.runtime.edits.entry(*id).or_default();
                    if edit.anchor.is_none() {
                        edit.anchor = Some(edit.cursor);
                    }
                    edit.cursor = cursor;
                }
            }
            Drag::Widget { id, rect } => self.apply_widget_drag(*id, *rect),
            Drag::Scroll { id, last, moved } => {
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
                let progress = ((x - *start_x) / width).clamp(0.0, 1.0);
                let dt = (now - *last_t).as_secs_f32();
                if dt > 1e-4 {
                    // Vitesse instantanée (fraction/s), lissée par moyenne exponentielle.
                    let inst = (x - *last_x) / width / dt;
                    *velocity = *velocity * 0.5 + inst * 0.5;
                    *last_x = x;
                    *last_t = now;
                }
                self.app.back_gesture(progress);
            }
        }
        self.drag = Some(drag);
        self.request_redraw();
    }

    /// Applique un glissement de widget : calcule la fraction horizontale et
    /// route le message produit par `on_drag`.
    fn apply_widget_drag(&mut self, id: WidgetId, rect: frus_widgets::Rect) {
        let fraction = if rect.width > 0.0 {
            ((self.cursor.x - rect.x) / rect.width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.on_drag(fraction));
        if let Some(message) = message {
            self.dispatch(message);
        }
    }

    /// Route une touche vers le champ focalisé : met à jour l'état d'édition et
    /// applique le message éventuel (changement de valeur ou soumission).
    fn apply_key(&mut self, id: WidgetId, key: Key) {
        let mut edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.on_edit(&mut edit, &key));
        self.runtime.edits.insert(id, edit);
        if let Some(message) = message {
            self.dispatch(message);
        }
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
