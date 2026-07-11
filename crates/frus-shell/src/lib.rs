//! `frus-shell` — la **couche framework** de frus.
//!
//! Crée une fenêtre native (via `winit`), initialise le [`Renderer`] de
//! `frus-gpu` et pilote la boucle `événement → frame` pour n'importe quelle
//! [`Application`]. C'est la seule couche dépendante de la plateforme.

mod app;
mod application;
mod command;
mod subscription;

pub use app::App;
pub use application::Application;
pub use command::Command;
pub use subscription::Subscription;

/// Ré-export : paliers de taille et orientation, pour piloter la responsivité
/// côté app.
pub use frus_widgets::{Orientation, SizeClass};

/// Lance une application : ouvre la fenêtre et pilote la boucle d'événements.
///
/// ```no_run
/// # struct MyApp;
/// # impl frus_shell::Application for MyApp {
/// #     type Message = ();
/// #     fn update(&mut self, _m: ()) -> frus_shell::Command<()> { frus_shell::Command::none() }
/// #     fn view(&self, _t: &frus_widgets::Theme, _w: f32, _h: f32)
/// #         -> Box<dyn frus_widgets::Widget<()>> { unimplemented!() }
/// # }
/// frus_shell::run(MyApp).unwrap();
/// ```
pub fn run<A: Application>(app: A) -> anyhow::Result<()> {
    // `RUST_LOG=info` pour voir les logs (adaptateur GPU, etc.).
    env_logger::init();

    // Boucle avec **événements utilisateur** = messages : les effets asynchrones
    // renvoient leur résultat via un `EventLoopProxy<Message>`.
    let event_loop = winit::event_loop::EventLoop::<A::Message>::with_user_event().build()?;
    // On redemande une frame tant qu'une animation tourne ; sinon on attend.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(app, proxy);
    event_loop.run_app(&mut app)?;
    Ok(())
}
