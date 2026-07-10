//! Implémentation de [`winit::application::ApplicationHandler`] pour frus.
//!
//! Démontre la boucle interactive : un état applicatif, une fonction `view` qui
//! produit l'arbre de widgets, des événements souris routés par hit-test, un
//! état d'interaction (survol/pression) retenu au runtime, et `update` qui fait
//! évoluer l'état applicatif au relâchement du clic.

use std::sync::Arc;
use std::time::Instant;

use frus_gpu::{wgpu, Renderer};
use frus_widgets::{
    build_ui, find_widget, Align, Button, Card, Checkbox, Container, Dropdown, Edit, Flex, Justify,
    Key, Navigator, Placement, Point, Portal, RadioGroup, Runtime, Scroll, Size, Slider, Switch,
    Text, TextInput, Theme, Ui, Variant, Widget, WidgetId,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

/// Vitesse de défilement (pixels par cran de molette).
const SCROLL_SPEED: f32 = 40.0;

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
    /// Geste « retour » : glissement depuis le bord gauche pour dépiler un écran.
    Back,
}

/// Largeur (px physiques) de la zone de bord activant le geste retour.
const BACK_EDGE: f32 = 24.0;
/// Horizon de projection de la vélocité (s) pour décider retour / annulation.
const BACK_PROJECT: f32 = 0.12;
/// Position projetée (fraction) au-delà de laquelle on valide le retour.
const BACK_COMMIT_POS: f32 = 0.5;
/// Raideur du ressort de transition (fraction·s⁻²). Partagée geste **et** bouton.
const NAV_SPRING_K: f32 = 220.0;
/// Amortissement du ressort, ~critique (2·√K ≈ 29,7) → arrivée douce sans dépassement.
const NAV_SPRING_C: f32 = 30.0;

/// Un pas de ressort amorti (Euler semi-implicite) faisant tendre `progress` vers
/// `target`, amorcé par `velocity` (l'élan du doigt, ou 0 pour un bouton). Renvoie
/// `(progress, velocity, terminé)`. Amortissement quasi critique → pas de rebond.
fn spring_step(progress: f32, velocity: f32, target: f32, dt: f32) -> (f32, f32, bool) {
    let accel = NAV_SPRING_K * (target - progress) - NAV_SPRING_C * velocity;
    let velocity = velocity + accel * dt;
    let progress = progress + velocity * dt;
    let done = (progress - target).abs() < 0.004 && velocity.abs() < 0.06;
    (progress, velocity, done)
}

/// Modèle physique du geste retour : suivi 1:1 du doigt, puis détente à ressort
/// (validation ou annulation) avec l'élan du doigt en vitesse initiale.
struct BackGesture {
    /// Abscisse (px) du début du geste, pour la position relative.
    start_x: f32,
    /// Avancement `0 → 1` (1 = écran dépilé).
    progress: f32,
    /// Vitesse en fraction/s (lissée pendant le glissement, puis intégrée).
    velocity: f32,
    /// Dernière abscisse échantillonnée (px), pour la vitesse.
    last_x: f32,
    /// Instant du dernier échantillon.
    last_t: Instant,
    /// `Some(cible)` une fois relâché : détente vers `0.0` (annule) ou `1.0` (valide).
    settling: Option<f32>,
}

/// État de l'application.
#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    state: State,
    /// Dernière interface construite (hit-test, focus, scroll).
    ui: Option<Ui<Msg>>,
    /// Dernier arbre de widgets construit (routage clavier/édition).
    tree: Option<Box<dyn Widget<Msg>>>,
    /// Dernière position connue du curseur, en pixels physiques.
    cursor: Point,
    /// État retenu entre frames (survol/focus, scroll, curseur/sélection).
    runtime: Runtime,
    /// Modificateurs clavier courants.
    shift: bool,
    ctrl: bool,
    /// Accès presse-papier (initialisé au démarrage).
    clipboard: Option<arboard::Clipboard>,
    /// Instant de la dernière frame (pour le dt des animations).
    last_frame: Option<Instant>,
    /// Glissement souris en cours (barre de défilement ou sélection texte).
    drag: Option<Drag>,
    /// Instant du dernier clic (détection du double-clic).
    last_click_time: Option<Instant>,
    /// Compteur pour clés d'événements de sortie (fondu de disparition).
    leaving_counter: u64,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        self.clipboard = arboard::Clipboard::new().ok();
        let attributes = Window::default_attributes().with_title("frus — Jalon 20 · Todo");
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
                self.window = Some(window.clone());
                self.renderer = Some(renderer);
                window.request_redraw();
            }
            Err(err) => {
                log::error!("Échec d'initialisation du renderer : {err:#}");
                event_loop.exit();
            }
        }
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

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point::new(position.x as f32, position.y as f32);

                // Glissement en cours : barre de défilement ou sélection texte.
                if self.drag.is_some() {
                    self.handle_drag();
                    return;
                }

                // Met à jour le survol ; ne redessine que s'il a changé.
                if let Some(ui) = &self.ui {
                    let hovered = ui.hit(self.cursor);
                    if hovered != self.runtime.input.hovered {
                        self.runtime.input.hovered = hovered;
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
            } => {
                // 0) Geste retour : appui sur le bord gauche, s'il y a un écran
                //    à dépiler et aucun overlay ouvert.
                if self.cursor.x < BACK_EDGE
                    && !self.state.routes.is_empty()
                    && !self.state.confirm_clear
                    && !self.state.menu_open
                {
                    self.drag = Some(Drag::Back);
                    self.state.back = Some(BackGesture {
                        start_x: self.cursor.x,
                        progress: 0.0,
                        velocity: 0.0,
                        last_x: self.cursor.x,
                        last_t: Instant::now(),
                        settling: None,
                    });
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
                let focus = self.ui.as_ref().and_then(|ui| ui.focus_hit(self.cursor));
                self.runtime.input.focused = focus.map(|(id, _)| id);
                if let Some((id, rect)) = focus {
                    let local_x = self.cursor.x - rect.x;
                    let cursor = self
                        .tree
                        .as_ref()
                        .and_then(|tree| find_widget(tree.as_ref(), id))
                        .and_then(|widget| widget.cursor_at(local_x))
                        .unwrap_or(0);
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
                self.request_redraw();
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // Fin d'un éventuel glissement.
                let ended = self.drag.take();
                if let Some(Drag::Back) = ended {
                    if let Some(gesture) = self.state.back.as_mut() {
                        // Projection à la iOS : la position + un peu d'élan décident.
                        // Un flick rapide valide même à mi-course ; un arrêt lent
                        // sous la moitié annule. La vitesse sert d'élan à la détente.
                        let projected = gesture.progress + gesture.velocity * BACK_PROJECT;
                        let commit = projected > BACK_COMMIT_POS && !self.state.routes.is_empty();
                        gesture.settling = Some(if commit { 1.0 } else { 0.0 });
                    }
                    self.request_redraw();
                    return;
                }
                if ended.is_some() {
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
                    update(&mut self.state, message);
                }
                self.request_redraw();
            }

            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed =>
            {
                let Some(focused) = self.runtime.input.focused else {
                    return;
                };

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
                            if let Some(text) =
                                self.clipboard.as_mut().and_then(|cb| cb.get_text().ok())
                            {
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
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                // Shift : la molette défile horizontalement.
                if self.shift {
                    dx = dy;
                    dy = 0.0;
                }
                if let Some((id, max_x, max_y)) =
                    self.ui.as_ref().and_then(|ui| ui.scroll_hit(self.cursor))
                {
                    let offset = self.runtime.scroll.entry(id).or_insert((0.0, 0.0));
                    offset.0 = (offset.0 - dx).clamp(0.0, max_x);
                    offset.1 = (offset.1 - dy).clamp(0.0, max_y);
                    self.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                let size = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .unwrap_or_default();
                let (width, height) = (size.width as f32, size.height as f32);

                // Avance les animations selon le temps écoulé (dt clampé).
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|prev| (now - prev).as_secs_f32().min(0.05))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);

                // Avance la transition d'écran (bouton) : même ressort que le geste,
                // amorcé à vitesse nulle → ease-out cohérent avec le swipe.
                if self.state.nav_from.is_some() {
                    let (p, v, done) =
                        spring_step(self.state.nav_progress, self.state.nav_velocity, 1.0, dt);
                    self.state.nav_progress = p;
                    self.state.nav_velocity = v;
                    if done {
                        self.state.nav_progress = 1.0;
                        self.state.nav_velocity = 0.0;
                        self.state.nav_from = None;
                    }
                }
                let nav_animating = self.state.nav_from.is_some();

                // Détente à ressort du geste retour relâché (vers 0 = annule, vers
                // 1 = valide), amorcée par la vélocité du doigt → élan naturel.
                let mut commit_back = false;
                if let Some(gesture) = self.state.back.as_mut() {
                    if let Some(target) = gesture.settling {
                        // Même ressort que la nav bouton, mais amorcé par l'élan du doigt.
                        let (p, v, done) =
                            spring_step(gesture.progress, gesture.velocity, target, dt);
                        gesture.progress = p;
                        gesture.velocity = v;
                        if done {
                            gesture.progress = target;
                            commit_back = target >= 1.0;
                            self.state.back = None;
                        }
                    }
                }
                if commit_back {
                    // Retour validé : dépile (l'écran arrière est déjà en place).
                    self.state.routes.pop();
                }
                let gesture_animating =
                    self.state.back.as_ref().is_some_and(|g| g.settling.is_some());

                // Avance le fondu de thème.
                if self.state.theme_from.is_some() {
                    self.state.theme_progress += dt / 0.25;
                    if self.state.theme_progress >= 1.0 {
                        self.state.theme_progress = 1.0;
                        self.state.theme_from = None;
                    }
                }
                let theme_animating = self.state.theme_from.is_some();

                // Thème affiché : mélange sortant → cible pendant le fondu.
                let target = theme_of(&self.state);
                let theme = match self.state.theme_from {
                    Some(from) => from.lerp(&target, self.state.theme_progress),
                    None => target,
                };
                let tree = view(&self.state, &theme, width, height);
                let ids = frus_widgets::collect_ids(&tree);
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

                let animating = self.runtime.advance(dt)
                    | self.runtime.advance_leaving(dt)
                    | self.runtime.advance_values(&tree, dt)
                    | nav_animating
                    | theme_animating
                    | gesture_animating;
                let ui = build_ui(&tree, Size::new(width, height), &self.runtime, &theme);

                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(ui.scene()) {
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

                // Conserve l'interface (hit-test) et l'arbre (routage clavier).
                self.ui = Some(ui);
                self.tree = Some(Box::new(tree));

                // Tant qu'une animation tourne, on redemande une frame.
                if animating {
                    self.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl App {
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Applique le glissement souris en cours (barre de défilement ou sélection).
    fn handle_drag(&mut self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        match &drag {
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
                let travel = (track_len - thumb_len).max(1.0);
                let thumb_start = (along - grab).clamp(*track_start, track_start + travel);
                let offset = ((thumb_start - track_start) / travel * max).clamp(0.0, *max);
                let entry = self.runtime.scroll.entry(*id).or_insert((0.0, 0.0));
                if *vertical {
                    entry.1 = offset;
                } else {
                    entry.0 = offset;
                }
            }
            Drag::TextSelect { id, rect } => {
                let local_x = self.cursor.x - rect.x;
                let cursor = self
                    .tree
                    .as_ref()
                    .and_then(|tree| find_widget(tree.as_ref(), *id))
                    .and_then(|widget| widget.cursor_at(local_x));
                if let Some(cursor) = cursor {
                    let edit = self.runtime.edits.entry(*id).or_default();
                    if edit.anchor.is_none() {
                        edit.anchor = Some(edit.cursor);
                    }
                    edit.cursor = cursor;
                }
            }
            Drag::Widget { id, rect } => self.apply_widget_drag(*id, *rect),
            Drag::Back => {
                let width = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().width as f32)
                    .unwrap_or(1.0)
                    .max(1.0);
                let now = Instant::now();
                let x = self.cursor.x;
                if let Some(gesture) = self.state.back.as_mut() {
                    gesture.progress = ((x - gesture.start_x) / width).clamp(0.0, 1.0);
                    let dt = (now - gesture.last_t).as_secs_f32();
                    if dt > 1e-4 {
                        // Vitesse instantanée (fraction/s), lissée par moyenne exponentielle.
                        let inst = (x - gesture.last_x) / width / dt;
                        gesture.velocity = gesture.velocity * 0.5 + inst * 0.5;
                        gesture.last_x = x;
                        gesture.last_t = now;
                    }
                }
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
            update(&mut self.state, message);
        }
    }

    /// Route une touche vers le champ focalisé : met à jour l'état d'édition et
    /// applique le message éventuel (changement de valeur).
    fn apply_key(&mut self, id: WidgetId, key: Key) {
        let mut edit = self.runtime.edits.get(&id).copied().unwrap_or_default();
        let message = self
            .tree
            .as_ref()
            .and_then(|tree| find_widget(tree.as_ref(), id))
            .and_then(|widget| widget.on_edit(&mut edit, &key));
        self.runtime.edits.insert(id, edit);
        if let Some(message) = message {
            update(&mut self.state, message);
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
        if let (Some(text), Some(clipboard)) = (text, self.clipboard.as_mut()) {
            let _ = clipboard.set_text(text);
        }
    }
}

// --- Application de démonstration (modèle à messages) ---

/// Une tâche de la liste.
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

/// Filtre d'affichage de la liste des tâches.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Active,
    Done,
}

/// État de l'app todo (+ écran Réglages conservé pour la nav et le geste retour).
#[derive(Default)]
struct State {
    /// Les tâches, dans l'ordre d'ajout.
    todos: Vec<Todo>,
    /// Texte en cours de saisie.
    draft: String,
    /// Filtre courant.
    filter: Filter,
    /// Prochain identifiant de tâche.
    next_id: u64,
    /// Modale de confirmation d'effacement des terminées ouverte ?
    confirm_clear: bool,
    /// Thème clair (sinon sombre).
    light: bool,
    /// Thème sortant pendant un fondu de bascule (`None` = pas de transition).
    theme_from: Option<Theme>,
    /// Avancement du fondu de thème (`0 → 1`).
    theme_progress: f32,
    /// Pile d'écrans (vide = accueil).
    routes: Vec<Route>,
    /// Écran sortant pendant une transition.
    nav_from: Option<Route>,
    nav_progress: f32,
    /// Vitesse de la transition (ressort partagé avec le geste).
    nav_velocity: f32,
    nav_forward: bool,
    /// Geste retour en cours (glissement ou détente à ressort).
    back: Option<BackGesture>,
    // --- Contrôles de l'écran Réglages ---
    notifs: bool,
    volume: f32,
    radio: usize,
    menu_open: bool,
    menu_choice: usize,
}

/// Les écrans de l'application.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Route {
    Home,
    Settings,
}

fn current_route(state: &State) -> Route {
    state.routes.last().copied().unwrap_or(Route::Home)
}

/// Messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    /// Le champ de saisie a changé.
    DraftChanged(String),
    /// Ajoute la tâche saisie (bouton ou touche Entrée).
    AddTodo,
    /// Bascule l'état d'une tâche (par identifiant).
    ToggleTodo(u64),
    /// Supprime une tâche (par identifiant).
    DeleteTodo(u64),
    /// Change le filtre d'affichage.
    SetFilter(Filter),
    /// Ouvre la confirmation d'effacement des terminées.
    AskClearDone,
    /// Efface les tâches terminées.
    ConfirmClearDone,
    /// Ferme la confirmation.
    CancelClear,
    ToggleTheme,
    SetNotifs(bool),
    SetVolume(f32),
    SetRadio(usize),
    ToggleMenu,
    SetMenu(usize),
    Push(Route),
    Pop,
}

/// Libellés du menu déroulant.
const MENU: [&str; 3] = ["Option A", "Option B", "Option C"];

/// Fait évoluer l'état en réponse à un message.
fn update(state: &mut State, message: Msg) {
    match message {
        Msg::DraftChanged(text) => state.draft = text,
        Msg::AddTodo => {
            let text = state.draft.trim();
            if !text.is_empty() {
                state.todos.push(Todo {
                    id: state.next_id,
                    text: text.to_string(),
                    done: false,
                });
                state.next_id += 1;
                state.draft.clear();
            }
        }
        Msg::ToggleTodo(id) => {
            if let Some(todo) = state.todos.iter_mut().find(|t| t.id == id) {
                todo.done = !todo.done;
            }
        }
        Msg::DeleteTodo(id) => state.todos.retain(|t| t.id != id),
        Msg::SetFilter(filter) => state.filter = filter,
        Msg::AskClearDone => state.confirm_clear = true,
        Msg::ConfirmClearDone => {
            state.todos.retain(|t| !t.done);
            state.confirm_clear = false;
        }
        Msg::CancelClear => state.confirm_clear = false,
        Msg::ToggleTheme => {
            // Capture le thème courant (avant bascule) comme point de départ du fondu.
            state.theme_from = Some(theme_of(state));
            state.light = !state.light;
            state.theme_progress = 0.0;
        }
        Msg::SetNotifs(v) => state.notifs = v,
        Msg::SetVolume(v) => state.volume = v,
        Msg::SetRadio(i) => state.radio = i,
        Msg::ToggleMenu => state.menu_open = !state.menu_open,
        Msg::SetMenu(i) => {
            state.menu_choice = i;
            state.menu_open = false;
        }
        Msg::Push(route) => {
            state.nav_from = Some(current_route(state));
            state.routes.push(route);
            state.nav_progress = 0.0;
            state.nav_velocity = 0.0;
            state.nav_forward = true;
        }
        Msg::Pop => {
            if !state.routes.is_empty() {
                state.nav_from = Some(current_route(state));
                state.routes.pop();
                state.nav_progress = 0.0;
                state.nav_velocity = 0.0;
                state.nav_forward = false;
            }
        }
    }
}

/// Nombre de tâches non terminées.
fn active_count(state: &State) -> usize {
    state.todos.iter().filter(|t| !t.done).count()
}

/// Nombre de tâches terminées.
fn done_count(state: &State) -> usize {
    state.todos.iter().filter(|t| t.done).count()
}

/// Contenu de la modale de confirmation d'effacement des terminées.
fn confirm_content(done: usize) -> Card<Msg> {
    Card::new().padding(24.0).child(
        Flex::column()
            .gap(16.0)
            .child(Text::new("Effacer les tâches terminées ?").size(22.0))
            .child(
                Text::new(format!("{done} tâche(s) seront supprimées.")).size(16.0),
            )
            .child(
                Flex::row()
                    .justify(Justify::Center)
                    .gap(12.0)
                    .child(
                        Button::new("Annuler")
                            .variant(Variant::Secondary)
                            .on_press(Msg::CancelClear),
                    )
                    .child(
                        Button::new("Supprimer")
                            .variant(Variant::Danger)
                            .on_press(Msg::ConfirmClearDone),
                    ),
            ),
    )
}

/// Thème courant selon l'état.
fn theme_of(state: &State) -> Theme {
    if state.light {
        Theme::light()
    } else {
        Theme::dark()
    }
}

/// Point d'entrée : un `Navigator` autour de l'écran courant, avec transition.
fn view(state: &State, theme: &Theme, width: f32, height: f32) -> Navigator<Msg> {
    // Geste retour en cours : prévisualise le dépilement, piloté par le doigt.
    if let Some(gesture) = &state.back {
        let progress = gesture.progress;
        let top = screen(current_route(state), state, theme, width, height);
        let below_route = state
            .routes
            .split_last()
            .and_then(|(_, rest)| rest.last().copied())
            .unwrap_or(Route::Home);
        let below = screen(below_route, state, theme, width, height);
        return Navigator::new(below, width, height).from(top, progress, false);
    }

    let current = screen(current_route(state), state, theme, width, height);
    match state.nav_from {
        Some(from) => Navigator::new(current, width, height).from(
            screen(from, state, theme, width, height),
            state.nav_progress,
            state.nav_forward,
        ),
        None => Navigator::new(current, width, height),
    }
}

/// Construit l'écran correspondant à une route.
fn screen(route: Route, state: &State, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    match route {
        Route::Home => todo_screen(state, theme, width, height),
        Route::Settings => settings_screen(state, theme, width, height),
    }
}

/// En-tête d'écran : titre + bouton retour.
fn screen_header(title: &str) -> Flex<Msg> {
    Flex::row()
        .align(Align::Center)
        .gap(12.0)
        .child(
            Button::new("← Retour")
                .variant(Variant::Secondary)
                .on_press(Msg::Pop),
        )
        .child(Text::new(title.to_string()).size(26.0))
}

/// Écran « Réglages » : la carte de contrôles.
fn settings_screen(state: &State, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let volume_pct = (state.volume * 100.0).round() as u32;
    let controls = Card::new().child(
        Flex::column()
            .gap(14.0)
            .child(
                Flex::row()
                    .align(Align::Center)
                    .gap(12.0)
                    .child(Text::new("Notifications").size(18.0))
                    .child(Flex::row().flex(1.0))
                    .child(Switch::new(state.notifs).on_toggle(Msg::SetNotifs)),
            )
            .child(
                Flex::row()
                    .align(Align::Center)
                    .gap(12.0)
                    .child(Text::new(format!("Volume : {volume_pct}%")).size(18.0))
                    .child(Slider::new(state.volume).width(220.0).on_change(Msg::SetVolume)),
            )
            .child(
                RadioGroup::new(state.radio, Msg::SetRadio)
                    .option("Petit")
                    .option("Moyen")
                    .option("Grand"),
            )
            .child(Dropdown::new(MENU[state.menu_choice], Msg::ToggleMenu).options(
                state.menu_open,
                &MENU,
                Msg::SetMenu,
            )),
    );
    let column = Flex::column()
        .width(width)
        .height(height)
        .padding(20.0)
        .gap(16.0)
        .child(screen_header("Réglages"))
        .child(Flex::row().justify(Justify::Center).child(controls));
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(column)
}

/// Une ligne de tâche : case à cocher, libellé (grisé si terminée) et suppression.
fn todo_row(todo: &Todo, theme: &Theme) -> Container<Msg> {
    let id = todo.id;
    let label_color = if todo.done { theme.muted } else { theme.on_surface };
    let row = Flex::row()
        .align(Align::Center)
        .gap(12.0)
        .child(Checkbox::new(todo.done).on_toggle(move |_| Msg::ToggleTodo(id)))
        .child(Text::new(todo.text.clone()).size(18.0).color(label_color))
        .child(Flex::row().flex(1.0))
        .child(
            Button::new("×")
                .variant(Variant::Danger)
                .size(15.0)
                .on_press(Msg::DeleteTodo(id)),
        );
    Container::new()
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(8.0, 12.0, 8.0, 12.0)
        .child(row)
}

/// Écran principal : la liste de tâches (l'app exemple).
fn todo_screen(state: &State, theme: &Theme, width: f32, height: f32) -> Container<Msg> {
    let active = active_count(state);
    let done = done_count(state);

    // En-tête : titre + bascule de thème + accès Réglages.
    let theme_label = if state.light { "Sombre" } else { "Clair" };
    let header = Flex::row()
        .align(Align::Center)
        .gap(10.0)
        .child(Text::new("Mes tâches").size(30.0))
        .child(Flex::row().flex(1.0))
        .child(
            Button::new(theme_label)
                .variant(Variant::Secondary)
                .size(15.0)
                .on_press(Msg::ToggleTheme),
        )
        .child(
            Button::new("Réglages →")
                .variant(Variant::Secondary)
                .size(15.0)
                .on_press(Msg::Push(Route::Settings)),
        );

    // Saisie : champ (Entrée valide) + bouton d'ajout.
    let input_row = Flex::row()
        .align(Align::Center)
        .gap(10.0)
        .child(
            TextInput::new(state.draft.as_str())
                .width(400.0)
                .size(18.0)
                .on_input(Msg::DraftChanged)
                .on_submit(Msg::AddTodo),
        )
        .child(Button::new("Ajouter").on_press(Msg::AddTodo));

    // Filtres : le filtre actif est mis en avant.
    let filter_button = |label: &str, f: Filter| {
        let variant = if state.filter == f {
            Variant::Primary
        } else {
            Variant::Secondary
        };
        Button::new(label)
            .variant(variant)
            .size(15.0)
            .on_press(Msg::SetFilter(f))
    };
    let filters = Flex::row()
        .gap(8.0)
        .child(filter_button("Toutes", Filter::All))
        .child(filter_button("Actives", Filter::Active))
        .child(filter_button("Terminées", Filter::Done));

    // Liste filtrée (ou état vide).
    let mut list = Flex::column().gap(8.0);
    let mut shown = 0;
    for todo in state.todos.iter().filter(|t| match state.filter {
        Filter::All => true,
        Filter::Active => !t.done,
        Filter::Done => t.done,
    }) {
        list = list.child(todo_row(todo, theme));
        shown += 1;
    }
    if shown == 0 {
        list = Flex::column().child(
            Text::new("Rien à afficher pour ce filtre.")
                .size(18.0)
                .color(theme.muted),
        );
    }
    let scroll = Scroll::new().flex(1.0).height(320.0).child(list);

    // Pied : compteurs + effacer les terminées (avec confirmation modale).
    let clear_button = Button::new("Effacer les terminées")
        .variant(Variant::Danger)
        .size(15.0)
        .on_press(Msg::AskClearDone);
    let clear = if state.confirm_clear {
        Portal::new(clear_button)
            .overlay(confirm_content(done), Placement::Center)
            .dismiss(Msg::CancelClear)
    } else {
        Portal::new(clear_button)
    };
    let footer = Flex::row()
        .align(Align::Center)
        .gap(12.0)
        .child(
            Text::new(format!("{active} active(s) · {done} terminée(s)"))
                .size(16.0)
                .color(theme.muted),
        )
        .child(Flex::row().flex(1.0))
        .child(clear);

    // Carte de l'app, largeur fixe, centrée en haut de l'écran.
    let card = Card::new().padding(20.0).child(
        Flex::column()
            .width(560.0)
            .gap(16.0)
            .child(header)
            .child(input_row)
            .child(filters)
            .child(scroll)
            .child(footer),
    );
    let column = Flex::column()
        .width(width)
        .height(height)
        .padding(24.0)
        .child(Flex::row().justify(Justify::Center).child(card));

    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::build_ui;

    fn primitive_count(state: &State) -> usize {
        let theme = Theme::default();
        let tree = view(state, &theme, 800.0, 600.0);
        build_ui(
            &tree,
            Size::new(800.0, 600.0),
            &frus_widgets::Runtime::default(),
            &theme,
        )
        .scene()
        .primitives()
        .len()
    }

    /// Ajoute une tâche depuis un libellé.
    fn add(state: &mut State, text: &str) {
        update(state, Msg::DraftChanged(text.to_string()));
        update(state, Msg::AddTodo);
    }

    #[test]
    fn add_todo_from_draft_and_trims_blanks() {
        let mut state = State::default();
        add(&mut state, "Acheter du pain");
        assert_eq!(state.todos.len(), 1);
        assert_eq!(state.todos[0].text, "Acheter du pain");
        assert!(state.draft.is_empty(), "le champ est vidé après l'ajout");

        // Une saisie vide / d'espaces n'ajoute rien.
        add(&mut state, "   ");
        assert_eq!(state.todos.len(), 1);
    }

    #[test]
    fn toggle_delete_and_clear_done() {
        let mut state = State::default();
        for t in ["a", "b", "c"] {
            add(&mut state, t);
        }
        let id_b = state.todos[1].id;
        update(&mut state, Msg::ToggleTodo(id_b));
        assert!(state.todos[1].done);
        assert_eq!(done_count(&state), 1);
        assert_eq!(active_count(&state), 2);

        // Supprime "a" par identité.
        let id_a = state.todos[0].id;
        update(&mut state, Msg::DeleteTodo(id_a));
        assert_eq!(state.todos.len(), 2);

        // Efface les terminées : "b" disparaît, "c" reste.
        update(&mut state, Msg::ConfirmClearDone);
        assert_eq!(state.todos.len(), 1);
        assert_eq!(state.todos[0].text, "c");
    }

    #[test]
    fn view_builds_a_non_empty_scene() {
        let mut state = State::default();
        add(&mut state, "tâche");
        assert!(primitive_count(&state) > 0);
    }
}
