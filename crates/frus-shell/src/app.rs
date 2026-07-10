//! Le pilote générique : implémente [`winit::application::ApplicationHandler`]
//! pour n'importe quelle [`Application`].
//!
//! Le framework possède la fenêtre, le renderer, le [`Runtime`] (état
//! d'interaction retenu : survol/focus/scroll/édition/animations), le routage des
//! entrées par hit-test, le glissement (barres, sélection, poignées, geste retour)
//! et l'horloge d'animation. L'application ne fournit que `update`/`view`/… .

use std::sync::Arc;
use std::time::Instant;

use frus_gpu::{wgpu, Renderer};
use frus_widgets::{
    build_ui, collect_ids, find_widget, Edit, Key, Point, Runtime, Size, Ui, Widget, WidgetId,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

use crate::application::Application;

/// Vitesse de défilement (pixels par cran de molette).
const SCROLL_SPEED: f32 = 40.0;

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
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    /// Dernière interface construite (hit-test, focus, scroll).
    ui: Option<Ui<A::Message>>,
    /// Dernier arbre de widgets construit (routage clavier/édition).
    tree: Option<Box<dyn Widget<A::Message>>>,
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
    /// Glissement souris en cours.
    drag: Option<Drag>,
    /// Instant du dernier clic (détection du double-clic).
    last_click_time: Option<Instant>,
    /// Compteur pour clés d'événements de sortie (fondu de disparition).
    leaving_counter: u64,
}

impl<A: Application> App<A> {
    /// Crée le pilote autour d'une application.
    pub fn new(app: A) -> Self {
        Self {
            app,
            window: None,
            renderer: None,
            ui: None,
            tree: None,
            cursor: Point::new(0.0, 0.0),
            runtime: Runtime::default(),
            shift: false,
            ctrl: false,
            clipboard: None,
            last_frame: None,
            drag: None,
            last_click_time: None,
            leaving_counter: 0,
        }
    }
}

impl<A: Application> ApplicationHandler for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        self.clipboard = arboard::Clipboard::new().ok();
        let attributes = Window::default_attributes().with_title(self.app.title());
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

                // Glissement en cours : barre de défilement, sélection, geste…
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
                if let Some(Drag::Back { velocity, .. }) = ended {
                    // L'app décide (valider / annuler) à partir de la vélocité.
                    self.app.back_gesture_end(velocity);
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
                    self.app.update(message);
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

                // dt écoulé (clampé), pour toutes les animations.
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|prev| (now - prev).as_secs_f32().min(0.05))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);

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

                let animating = self.runtime.advance(dt)
                    | self.runtime.advance_leaving(dt)
                    | self.runtime.advance_values(tree.as_ref(), dt)
                    | app_animating;
                let ui = build_ui(tree.as_ref(), Size::new(width, height), &self.runtime, &theme);

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
                self.tree = Some(tree);

                // Tant qu'une animation tourne, on redemande une frame.
                if animating {
                    self.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl<A: Application> App<A> {
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
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
            Drag::Back {
                start_x,
                last_x,
                last_t,
                velocity,
            } => {
                let width = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().width as f32)
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
            self.app.update(message);
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
            self.app.update(message);
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
