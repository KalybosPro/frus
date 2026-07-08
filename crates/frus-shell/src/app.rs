//! Implémentation de [`winit::application::ApplicationHandler`] pour frus.

use std::sync::Arc;

use frus_gpu::{wgpu, Renderer};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

/// État de l'application : la fenêtre et son renderer.
///
/// Les deux sont `Option` car, avec le modèle `ApplicationHandler` de winit
/// 0.30, la fenêtre n'est créée qu'une fois l'application « reprise »
/// (`resumed`), et non à la construction.
#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `resumed` peut être appelé plusieurs fois (mobile) : on ne crée
        // la fenêtre qu'une seule fois.
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes().with_title("frus — Jalon 0");
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
                // Première frame.
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
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => match renderer.render() {
                Ok(()) => {}
                // La surface est perdue ou obsolète : on la reconfigure.
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    renderer.reconfigure();
                }
                // Plus de mémoire GPU : rien de mieux à faire que quitter.
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    log::error!("Mémoire GPU épuisée, fermeture.");
                    event_loop.exit();
                }
                Err(err) => log::warn!("Frame ignorée : {err:?}"),
            },

            _ => {}
        }
    }
}
