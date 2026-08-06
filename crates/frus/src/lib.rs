//! `frus` — la **façade** du framework : **une seule dépendance** (`frus`) suffit pour
//! écrire une application. Elle ré-exporte la couche framework (`frus-shell`) et les
//! widgets (`frus-widgets`), plus le **point d'entrée unique** [`main!`].
//!
//! ```ignore
//! use frus::{Application, Command, Widget, Theme, button, column};
//!
//! #[derive(Default)]
//! struct App { count: i32 }
//! # #[derive(Clone)] enum Msg { Inc }
//! impl Application for App {
//!     type Message = Msg;
//!     fn update(&mut self, _m: Msg) -> Command<Msg> { Command::none() }
//!     fn view(&self, _t: &Theme, _w: f32, _h: f32) -> Box<dyn Widget<Msg>> { todo!() }
//! }
//!
//! // Un seul point d'entrée — façon Flutter (`void main() => runApp(App())`).
//! frus::main!(App::default());
//! ```
//!
//! Le mince binaire bureau appelle la `run()` engendrée : `fn main() -> frus::anyhow::Result<()>
//! { my_app::run() }`. Pour le Web, l'app garde une dépendance `wasm-bindgen` (ciblée `wasm32`),
//! comme une app Flutter garde `flutter` dans son `pubspec`.

// Widgets, thème, layout, et le DSL (`row!` / `column!`) — tout `frus-widgets`.
pub use frus_widgets::*;

// Couche framework : le trait [`Application`] et ses compagnons (effets, souscriptions,
// cycle de vie).
pub use frus_shell::{Application, Command, Lifecycle, Subscription};

// Ré-exports utilitaires pour le binaire mince et pour la macro d'entrée (l'app n'a pas à
// déclarer `anyhow` / `log`).
pub use frus_shell::{anyhow, log};

// Macros : un glob (`frus_widgets::*`) ne ré-exporte **pas** les macros `#[macro_export]`, on
// les nomme donc explicitement — d'où `frus::main!`, `frus::column!`, `frus::row!`.
pub use frus_shell::main;
pub use frus_widgets::{column, row};

// HTTP cross-plateforme (feature `net`) : le raccourci `frus::fetch(url).await` et le
// constructeur `frus::Request` (méthode/en-têtes/corps/timeout) — voir [`frus::net`].
#[cfg(feature = "net")]
pub use frus_shell::{fetch, net, FetchError, Method, Request};
