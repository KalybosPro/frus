//! Implémentation de [`winit::application::ApplicationHandler`] pour frus.
//!
//! Démontre la boucle interactive : un état applicatif, une fonction `view` qui
//! produit l'arbre de widgets, des événements souris routés par hit-test, un
//! état d'interaction (survol/pression) retenu au runtime, et `update` qui fait
//! évoluer l'état applicatif au relâchement du clic.

use std::sync::Arc;

use frus_gpu::{wgpu, Renderer};
use frus_widgets::{build_ui, Color, Container, Flex, InputState, Point, Size, Text, Ui};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

/// État de l'application.
#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    state: State,
    /// Dernière interface construite (pour le hit-test et le routage des clics).
    ui: Option<Ui<Msg>>,
    /// Dernière position connue du curseur, en pixels physiques.
    cursor: Point,
    /// État d'interaction retenu (survol/pression).
    input: InputState,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes().with_title("frus — Jalon 6");
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
                // Met à jour le survol ; ne redessine que s'il a changé.
                if let Some(ui) = &self.ui {
                    let hovered = ui.hit(self.cursor);
                    if hovered != self.input.hovered {
                        self.input.hovered = hovered;
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.input.pressed = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // Le clic n'est validé que si press et release sont sur le même widget.
                let released = self.ui.as_ref().and_then(|ui| ui.hit(self.cursor));
                let message = match (self.input.pressed, released) {
                    (Some(pressed), Some(released)) if pressed == released => {
                        self.ui.as_ref().and_then(|ui| ui.msg_for(pressed))
                    }
                    _ => None,
                };
                self.input.pressed = None;
                if let Some(message) = message {
                    update(&mut self.state, message);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                let size = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .unwrap_or_default();
                let (width, height) = (size.width as f32, size.height as f32);

                let tree = view(&self.state, width, height);
                let ui = build_ui(&tree, Size::new(width, height), &self.input);

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

                // Conserve l'interface pour le hit-test des prochains clics.
                self.ui = Some(ui);
            }

            _ => {}
        }
    }
}

// --- Application de démonstration (modèle à messages) ---

/// État : le nombre de carrés ajoutés.
#[derive(Default)]
struct State {
    squares: u32,
}

/// Messages émis par l'interface.
#[derive(Clone)]
enum Msg {
    AddSquare,
}

/// Fait évoluer l'état en réponse à un message.
fn update(state: &mut State, message: Msg) {
    match message {
        Msg::AddSquare => state.squares += 1,
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
    // Bouton avec libellé texte : couleurs distinctes au survol et à la pression.
    let button = Container::new()
        .padding(14.0)
        .color(Color::rgb8(80, 200, 120))
        .hover_color(Color::rgb8(110, 220, 150))
        .pressed_color(Color::rgb8(60, 170, 100))
        .on_click(Msg::AddSquare)
        .child(
            Text::new("+ Ajouter un carré")
                .size(20.0)
                .color(Color::rgb8(20, 40, 25)),
        );

    // Libellé du compteur, mis à jour à chaque clic.
    let label = Text::new(format!("Carrés : {}", state.squares))
        .size(26.0)
        .color(Color::rgb8(230, 230, 235));

    let mut squares = Flex::row().flex(1.0).gap(8.0);
    for i in 0..state.squares {
        squares = squares.child(
            Container::new()
                .width(40.0)
                .height(40.0)
                .color(PALETTE[(i as usize) % PALETTE.len()]),
        );
    }

    Flex::column()
        .width(width)
        .height(height)
        .padding(16.0)
        .gap(12.0)
        .child(button)
        .child(label)
        .child(squares)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_widgets::build_ui;

    #[test]
    fn clicking_the_button_adds_squares() {
        let mut state = State::default();
        // Simule trois clics sur le bouton.
        for _ in 0..3 {
            update(&mut state, Msg::AddSquare);
        }
        assert_eq!(state.squares, 3);

        let tree = view(&state, 800.0, 600.0);
        let ui = build_ui(&tree, Size::new(800.0, 600.0), &InputState::default());

        // Primitives peintes : fond du bouton (1) + libellé du bouton (1)
        // + libellé compteur (1) + 3 carrés = 6. Les Flex ne peignent rien.
        assert_eq!(ui.scene().primitives().len(), 6);
    }
}
