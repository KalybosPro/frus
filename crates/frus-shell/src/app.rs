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
    build_ui, find_widget, Align, Axis, Color, Container, Edit, Flex, Justify, Key, Point, Runtime,
    Scroll, Size, Text, TextInput, Ui, Widget, WidgetId,
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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        self.clipboard = arboard::Clipboard::new().ok();
        let attributes = Window::default_attributes().with_title("frus — Jalon 12");
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
                let was_dragging = self.drag.take().is_some();
                if was_dragging {
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
                let animating = self.runtime.advance(dt);

                let tree = view(&self.state, width, height);
                let ui = build_ui(&tree, Size::new(width, height), &self.runtime);

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
        }
        self.drag = Some(drag);
        self.request_redraw();
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

/// État : le nombre de carrés et le nom saisi.
#[derive(Default)]
struct State {
    squares: u32,
    name: String,
    email: String,
}

/// Messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    AddSquare,
    NameChanged(String),
    EmailChanged(String),
}

/// Fait évoluer l'état en réponse à un message.
fn update(state: &mut State, message: Msg) {
    match message {
        Msg::AddSquare => state.squares += 1,
        Msg::NameChanged(name) => state.name = name,
        Msg::EmailChanged(email) => state.email = email,
    }
}

/// Palette cyclique pour les carrés.
const PALETTE: [Color; 4] = [
    Color::rgb(0.91, 0.30, 0.24),
    Color::rgb(0.95, 0.61, 0.07),
    Color::rgb(0.18, 0.80, 0.44),
    Color::rgb(0.20, 0.60, 0.86),
];

/// Construit l'arbre de widgets à partir de l'état : une barre-bouton verte
/// (cliquable) au-dessus d'une rangée de `state.squares` carrés colorés.
fn view(state: &State, width: f32, height: f32) -> Flex<Msg> {
    // En-tête centré : le compteur, mis à jour à chaque clic.
    let header = Flex::row().justify(Justify::Center).child(
        Text::new("Welcome To Frus")
            .size(28.0)
            .color(Color::rgb8(230, 230, 235)),
    );

    // Bouton arrondi, bordé, avec padding par côté et couleurs d'interaction.
    let button = Container::new()
        .radius(12.0)
        .border(2.0, Color::rgb8(40, 120, 80))
        .shadow(0.0, 4.0, 12.0, Color::rgba(0.0, 0.0, 0.0, 0.45))
        .gradient(Color::rgb8(56, 178, 104), [0.0, 1.0])
        .padding_each(12.0, 20.0, 12.0, 20.0)
        .color(Color::rgb8(96, 210, 132))
        .hover_color(Color::rgb8(120, 226, 158))
        .pressed_color(Color::rgb8(70, 180, 110))
        .on_click(Msg::AddSquare)
        .child(
            Text::new("+ Ajouter un carré")
                .size(20.0)
                .color(Color::rgb8(20, 40, 25)),
        );
    // Rangée qui centre le bouton horizontalement.
    let button_row = Flex::row().justify(Justify::Center).child(button);

    // Champ de saisie (contrôlé) + salutation.
    let greeting = if state.name.is_empty() {
        "Tapez votre nom ci-dessous".to_string()
    } else {
        format!("Nom: {}", state.name)
    };
    let name_row = Flex::row()
        .justify(Justify::Center)
        .align(Align::Center)
        .gap(12.0)
        .child(Text::new("Nom :").size(20.0).color(Color::rgb8(210, 210, 220)))
        .child(
            TextInput::new(state.name.as_str())
                .width(280.0)
                .size(18.0)
                .on_input(Msg::NameChanged),
        );

    let email_row = Flex::row()
        .justify(Justify::Center)
        .align(Align::Center)
        .gap(12.0)
        .child(Text::new("Email :").size(20.0).color(Color::rgb8(210, 210,220)))
        .child(
            TextInput::new(state.email.as_str())
                .width(280.0)
                .size(18.0)
                .on_input(Msg::EmailChanged)
        );

    let email_text = Flex::row().justify(Justify::Center).child(
        Container::new()
            .radius(10.0)
            .color(Color::rgb8(19,36,100))
            .padding(8.0)
            .child(
                Text::new(format!("Email: {}", state.email))
                    .size(18.0)
                    .color(Color::rgb8(170, 200, 176))
            )
    );

    let greeting_row = Flex::row().justify(Justify::Center).child(
        Text::new(greeting).size(18.0).color(Color::rgb8(170, 200, 175)),
    );

    // Bande défilante horizontale de vignettes (dégradé + coins arrondis).
    let mut strip = Flex::row().gap(10.0);
    for i in 0..12 {
        let c = PALETTE[i % PALETTE.len()];
        strip = strip.child(
            Container::new()
                .width(120.0)
                .height(72.0)
                .radius(10.0)
                .shadow(0.0, 3.0, 8.0, Color::rgba(0.0, 0.0, 0.0, 0.35))
                .gradient(c, [0.6, 1.0])
                .color(Color::rgb(c.r * 1.3, c.g * 1.3, c.b * 1.3)),
        );
    }
    let strip_scroll = Scroll::new()
        .axis(Axis::Horizontal)
        .height(84.0)
        .child(strip);

    // Liste défilante verticale : 8 éléments + ceux ajoutés au clic.
    let total = 8 + state.squares;
    let mut list = Flex::column().gap(8.0);
    for i in 0..total {
        list = list.child(
            Container::new()
                .height(48.0)
                .radius(8.0)
                .shadow(0.0, 2.0, 6.0, Color::rgba(0.0, 0.0, 0.0, 0.3))
                .color(PALETTE[(i as usize) % PALETTE.len()])
                .padding_each(14.0, 16.0, 14.0, 16.0)
                .child(
                    Text::new(format!("Élément {}", i + 1))
                        .size(18.0)
                        .color(Color::rgb8(20, 25, 30)),
                ),
        );
    }
    let scroll = Scroll::new().flex(1.0).height(240.0).child(list);

    Flex::column()
        .width(width)
        .height(height)
        .padding(20.0)
        .gap(16.0)
        .child(header)
        .child(button_row)
        .child(name_row)
        .child(email_row)
        .child(greeting_row)
        .child(email_text)
        .child(strip_scroll)
        .child(scroll)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::build_ui;

    fn primitive_count(state: &State) -> usize {
        let tree = view(state, 800.0, 600.0);
        build_ui(&tree, Size::new(800.0, 600.0), &frus_widgets::Runtime::default())
            .scene()
            .primitives()
            .len()
    }

    #[test]
    fn clicking_the_button_adds_squares() {
        let mut state = State::default();
        let base = primitive_count(&state);

        // Simule trois clics sur le bouton.
        for _ in 0..3 {
            update(&mut state, Msg::AddSquare);
        }
        assert_eq!(state.squares, 3);

        // Trois éléments de plus dans la liste => la scène a plus de primitives.
        assert!(primitive_count(&state) > base);
    }

    #[test]
    fn editing_updates_the_name() {
        let mut state = State::default();
        update(&mut state, Msg::NameChanged("Ada".to_string()));
        assert_eq!(state.name, "Ada");
    }
}
