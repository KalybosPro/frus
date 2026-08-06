//! `frus-shell` — la **couche framework** de frus.
//!
//! Crée une fenêtre native (via `winit`), initialise le [`Renderer`] de
//! `frus-gpu` et pilote la boucle `événement → frame` pour n'importe quelle
//! [`Application`]. C'est la seule couche dépendante de la plateforme.

// Les alias `desktop` / `android` / `ios` / `web` viennent de `build.rs`. Ils ne
// valent que pour **ce** crate : la macro `main!`, plus bas, s'expanse chez
// l'utilisateur et garde donc des prédicats `target_os` / `target_arch` explicites.

// Accessibilité (AccessKit) : bureau uniquement (ni Android, ni iOS, ni Web).
#[cfg(desktop)]
mod a11y;
#[cfg(android)]
mod android_ime;
mod app;
mod application;
mod command;
mod gesture;
/// Helper HTTP `fetch` cross-plateforme (derrière la feature `net`).
#[cfg(feature = "net")]
pub mod net;
mod reload;
mod remote;
mod subscription;

pub use app::App;
pub use application::{Application, Lifecycle};
pub use command::Command;
pub use remote::RemoteData;
pub use subscription::Subscription;

/// HTTP cross-plateforme (feature `net`) — le raccourci [`fetch`], le constructeur
/// [`Request`] (méthode/en-têtes/corps/timeout) et l'erreur [`FetchError`].
#[cfg(feature = "net")]
pub use net::{fetch, FetchError, Method, Request};

/// Ré-export : paliers de taille et orientation, pour piloter la responsivité
/// côté app.
pub use frus_widgets::{Orientation, SizeClass};

/// Ré-export du type d'entrée Android (fourni par `winit`/`android-activity`),
/// pour typer le `android_main` côté application sans dépendre de winit.
#[cfg(android)]
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
/// - **bureau et iOS** : une fonction `run()` (que le mince binaire de l'app appelle) ;
/// - **Android** : le symbole natif `android_main` attendu par l'activité ;
/// - **Web** : la fonction `#[wasm_bindgen(start)]`.
///
/// Les `cfg` ci-dessous sont écrits en `target_os` / `target_arch` **explicites**, et non
/// avec les alias `desktop`/`android`/`web` de `build.rs` : le corps d'une `macro_rules!`
/// s'expanse dans le crate de l'**application**, où ces alias ne sont pas définis. Un
/// `#[cfg(desktop)]` y serait toujours faux, et l'app n'aurait aucun point d'entrée.
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
        /// Point d'entrée **bureau et iOS** : ouvre la fenêtre et pilote la boucle
        /// (appelé par le binaire mince de l'application, ou par le `main` du bundle
        /// `.app` sur iOS). Engendré par [`frus_shell::main!`].
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
#[cfg(desktop)]
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
/// Lance une application sur **iOS**. Même forme que [`run`] — winit assure lui-même
/// l'`UIApplicationMain` et `wgpu` sort sur Metal —, mais sans les trois services de
/// bureau qui n'ont pas de backend UIKit : pas d'`env_logger` (stderr n'est pas lisible
/// sur appareil), pas de presse-papier `arboard`, pas d'AccessKit.
///
/// La macro [`main!`] engendre le même `run()` que sur bureau : c'est le binaire du
/// bundle `.app` qui l'appelle depuis son `main`.
///
/// **État : amorçage.** Le cycle de vie, les safe-area insets, l'IME et le clavier
/// logiciel ne sont pas encore câblés — voir la ROADMAP.
#[cfg(ios)]
pub fn run<A: Application>(app: A) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::<A::Message>::with_user_event().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(app, proxy);
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(web)]
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
#[cfg(android)]
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
