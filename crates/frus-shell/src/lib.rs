//! `frus-shell` — couche plateforme du Jalon 0.
//!
//! Crée une fenêtre native (via `winit`), initialise le [`Renderer`] de
//! `frus-gpu` et pilote la boucle `événement → frame`. C'est la seule couche
//! dépendante de la plateforme à ce stade.

mod app;

pub use app::App;

/// Point d'entrée : ouvre la fenêtre et lance la boucle d'événements.
pub fn run() -> anyhow::Result<()> {
    // `RUST_LOG=info` pour voir les logs (adaptateur GPU, etc.).
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new()?;
    // La scène est statique : on n'a pas besoin de redessiner en continu.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
