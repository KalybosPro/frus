//! `frus-shell` — la **couche framework** de frus.
//!
//! Crée une fenêtre native (via `winit`), initialise le [`Renderer`] de
//! `frus-gpu` et pilote la boucle `événement → frame` pour n'importe quelle
//! [`Application`]. C'est la seule couche dépendante de la plateforme.

// Accessibilité (AccessKit) : bureau uniquement (ni Android ni Web).
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
mod a11y;
#[cfg(target_os = "android")]
mod android_ime;
mod app;
mod application;
mod command;
mod gesture;
/// Helper HTTP `fetch` cross-plateforme (derrière la feature `net`).
#[cfg(feature = "net")]
pub mod net;
mod reload;
mod subscription;

pub use app::App;
pub use application::{Application, Lifecycle};
pub use command::Command;
pub use subscription::Subscription;

/// GET HTTP cross-plateforme et son erreur (feature `net`) — voir [`net::fetch`].
#[cfg(feature = "net")]
pub use net::{fetch, FetchError};

/// Ré-export : paliers de taille et orientation, pour piloter la responsivité
/// côté app.
pub use frus_widgets::{Orientation, SizeClass};

/// Ré-export du type d'entrée Android (fourni par `winit`/`android-activity`),
/// pour typer le `android_main` côté application sans dépendre de winit.
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

/// Ré-exports **pour la macro [`main!`]** : l'application déclare son point d'entrée
/// unique sans dépendre directement de ces crates.
#[doc(hidden)]
pub use anyhow;
#[doc(hidden)]
pub use log;

/// Déclare le **point d'entrée unique** d'une application frus — l'équivalent du
/// `void main() => runApp(App())` de Flutter. Invoquée **une seule fois** (dans la
/// bibliothèque de l'app), elle engendre les points d'entrée de **chaque plateforme**,
/// tous délégant à la **même** application :
///
/// - **bureau** : une fonction `run()` (que le mince binaire de l'app appelle) ;
/// - **Android** : le symbole natif `android_main` attendu par l'activité ;
/// - **Web** : la fonction `#[wasm_bindgen(start)]`.
///
/// L'argument est une **expression** qui construit l'application (rappelée par
/// plateforme, jamais partagée) :
///
/// ```ignore
/// frus_shell::main!(App::default());
/// ```
///
/// La plateforme Web garde sa dépendance `wasm-bindgen` (ciblée `wasm32`), comme une
/// app Flutter garde `flutter` dans son `pubspec` : la macro y renvoie via `::wasm_bindgen`.
#[macro_export]
macro_rules! main {
    ($app:expr $(,)?) => {
        /// Point d'entrée **bureau** : ouvre la fenêtre et pilote la boucle (appelé par
        /// le binaire mince de l'application). Engendré par [`frus_shell::main!`].
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        pub fn run() -> $crate::anyhow::Result<()> {
            $crate::run($app)
        }

        /// Point d'entrée **Android** : appelé par l'activité native. Engendré par
        /// [`frus_shell::main!`].
        #[cfg(target_os = "android")]
        #[no_mangle]
        fn android_main(android_app: $crate::AndroidApp) {
            if let ::core::result::Result::Err(err) = $crate::run_android($app, android_app) {
                $crate::log::error!("frus (android) stopped: {:#}", err);
            }
        }

        /// Point d'entrée **Web** : appelé au chargement du module wasm. Engendré par
        /// [`frus_shell::main!`].
        #[cfg(target_arch = "wasm32")]
        #[::wasm_bindgen::prelude::wasm_bindgen(start)]
        pub fn start() {
            if let ::core::result::Result::Err(err) = $crate::run_web($app) {
                $crate::log::error!("frus (web) stopped: {:#}", err);
            }
        }
    };
}

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
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
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

/// Lance une application dans le **navigateur** (wasm + WebGPU) : winit crée et
/// **ajoute un `<canvas>`** au document, le renderer s'initialise de façon asynchrone
/// (pas de blocage possible sur le Web), et la boucle est confiée au navigateur via
/// `spawn_app` (qui ne rend jamais la main). Appelé depuis le point d'entrée
/// `#[wasm_bindgen(start)]` de l'application.
#[cfg(target_arch = "wasm32")]
pub fn run_web<A: Application + 'static>(app: A) -> anyhow::Result<()> {
    use winit::platform::web::EventLoopExtWebSys;

    // Panics → console du navigateur (au lieu d'un « unreachable » opaque).
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let event_loop = winit::event_loop::EventLoop::<A::Message>::with_user_event().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let app = App::new(app, proxy);
    // Sur le Web, la boucle est pilotée par le navigateur (requestAnimationFrame) et
    // `spawn_app` ne revient pas — d'où le retour immédiat `Ok`.
    event_loop.spawn_app(app);
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
    // Pont de saisie (InputConnection réelle) : composition/swipe/CJK. En cas
    // d'échec, le shell retombe sur le mode touches (TYPE_NULL).
    android_ime::install(&android_app);
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
