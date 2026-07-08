//! Implémentation de [`winit::application::ApplicationHandler`] pour frus.

use std::sync::Arc;

use frus_gpu::{wgpu, Color, Renderer, Scene};
use frus_layout::{Dimension, FlexDirection, Layout, Size, Style};
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

            WindowEvent::RedrawRequested => {
                let size = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .unwrap_or_default();
                let scene = demo_scene(size.width as f32, size.height as f32);

                match renderer.render(&scene) {
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
                }
            }

            _ => {}
        }
    }
}

/// Construit une scène de démonstration via le moteur de layout : une colonne
/// avec padding contient une barre supérieure, puis une rangée (sidebar + zone
/// principale). Les positions/tailles sont calculées par flexbox et s'adaptent
/// à la taille de la fenêtre.
fn demo_scene(width: f32, height: f32) -> Scene {
    let mut layout: Layout<Color> = Layout::new();

    // Barre supérieure : hauteur fixe, s'étire en largeur.
    let top_bar = layout.leaf(
        Style {
            height: Dimension::Length(56.0),
            ..Default::default()
        },
        Color::rgb8(80, 200, 120),
    );

    // Contenu : une rangée avec une sidebar fixe et une zone principale extensible.
    let sidebar = layout.leaf(
        Style {
            width: Dimension::Length(200.0),
            ..Default::default()
        },
        Color::rgb8(233, 69, 96),
    );
    let main_area = layout.leaf(
        Style {
            flex_grow: 1.0,
            ..Default::default()
        },
        Color::rgba(0.25, 0.55, 0.95, 0.9),
    );
    let content_row = layout.container(
        Style {
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            gap: 12.0,
            ..Default::default()
        },
        &[sidebar, main_area],
    );

    let root = layout.container(
        Style {
            width: Dimension::Length(width),
            height: Dimension::Length(height),
            flex_direction: FlexDirection::Column,
            padding: 16.0,
            gap: 12.0,
            ..Default::default()
        },
        &[top_bar, content_row],
    );

    layout.compute(root, Size::new(width, height));

    // Les conteneurs n'ont pas de couleur (None) : seules les feuilles sont dessinées.
    let mut scene = Scene::new();
    for (rect, color) in layout.absolute_rects(root) {
        if let Some(color) = color {
            scene.fill_rect(rect, *color);
        }
    }
    scene
}
