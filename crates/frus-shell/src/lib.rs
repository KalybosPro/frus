//! `frus-shell` — la **couche framework** de frus.
//!
//! Crée une fenêtre native (via `winit`), initialise le [`Renderer`] de
//! `frus-gpu` et pilote la boucle `événement → frame` pour n'importe quelle
//! [`Application`]. C'est la seule couche dépendante de la plateforme.

mod app;
mod application;

pub use app::App;
pub use application::Application;

/// Lance une application : ouvre la fenêtre et pilote la boucle d'événements.
///
/// ```no_run
/// # struct MyApp;
/// # impl frus_shell::Application for MyApp {
/// #     type Message = ();
/// #     fn update(&mut self, _m: ()) {}
/// #     fn view(&self, _t: &frus_widgets::Theme, _w: f32, _h: f32)
/// #         -> Box<dyn frus_widgets::Widget<()>> { unimplemented!() }
/// # }
/// frus_shell::run(MyApp).unwrap();
/// ```
pub fn run<A: Application>(app: A) -> anyhow::Result<()> {
    // `RUST_LOG=info` pour voir les logs (adaptateur GPU, etc.).
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new()?;
    // On redemande une frame tant qu'une animation tourne ; sinon on attend.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = App::new(app);
    event_loop.run_app(&mut app)?;
    Ok(())
}
