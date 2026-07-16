//! `frus-shell` — la **couche framework** de frus.
//!
//! Crée une fenêtre native (via `winit`), initialise le [`Renderer`] de
//! `frus-gpu` et pilote la boucle `événement → frame` pour n'importe quelle
//! [`Application`]. C'est la seule couche dépendante de la plateforme.

mod app;
mod application;
mod command;
mod gesture;
mod reload;
mod subscription;

pub use app::App;
pub use application::Application;
pub use command::Command;
pub use subscription::Subscription;

/// Ré-export : paliers de taille et orientation, pour piloter la responsivité
/// côté app.
pub use frus_widgets::{Orientation, SizeClass};

/// Ré-export du type d'entrée Android (fourni par `winit`/`android-activity`),
/// pour typer le `android_main` côté application sans dépendre de winit.
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

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
#[cfg(not(target_os = "android"))]
pub fn run<A: Application>(mut app: A) -> anyhow::Result<()> {
    // `RUST_LOG=info` pour voir les logs (adaptateur GPU, etc.).
    env_logger::init();

    // Live-reload (dev) : réhydrate l'état laissé par le binaire précédent,
    // avant `init` — voir [`Application::restore_state`].
    reload::restore_from_env(&mut app);

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

/// Lance une application sur **Android** : l'activité native fournit l'[`AndroidApp`],
/// à transmettre à la boucle winit. Point d'entrée appelé depuis `android_main`.
#[cfg(target_os = "android")]
pub fn run_android<A: Application>(app: A, android_app: AndroidApp) -> anyhow::Result<()> {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    // Les logs partent dans logcat (`adb logcat`).
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let event_loop = winit::event_loop::EventLoop::<A::Message>::with_user_event()
        .with_android_app(android_app.clone())
        .build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(app, proxy);
    // Conserve la poignée d'activité pour interroger les insets système.
    app.set_android_app(android_app);
    event_loop.run_app(&mut app)?;

    // Android garde le **processus** en cache après la fin de l'activité, mais
    // winit n'autorise qu'une seule `EventLoop` par processus : un relancement
    // dans le même processus échouerait aussitôt (l'icône ne « répond » plus
    // jusqu'à ce qu'Android tue le cache). On termine donc le processus pour
    // que le prochain lancement reparte propre.
    std::process::exit(0);
}
