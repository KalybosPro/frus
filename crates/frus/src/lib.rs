//! `frus` — the framework's **facade**: a **single dependency** (`frus`) is all an
//! application needs. It re-exports the framework layer (`frus-shell`) and the widgets
//! (`frus-widgets`), plus the **single entry point** [`main!`].
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
//! // One entry point, for every platform.
//! frus::main!(App::default());
//! ```
//!
//! The thin desktop binary calls the generated `run()`: `fn main() -> frus::anyhow::Result<()>
//! { my_app::run() }`. On the web, the application keeps a `wasm-bindgen` dependency (targeted
//! at `wasm32`), which the generated `#[wasm_bindgen(start)]` entry point needs.

// Widgets, theming, layout, and the DSL (`row!` / `column!`) — all of `frus-widgets`.
pub use frus_widgets::*;

// Framework layer: the [`Application`] trait and its companions (effects, subscriptions,
// lifecycle).
pub use frus_shell::{Application, Command, Lifecycle, RemoteData, Subscription};

// Utility re-exports for the thin binary and for the entry-point macro (so the application
// does not have to declare `anyhow` / `log` itself).
pub use frus_shell::{anyhow, log};

// Macros: a glob (`frus_widgets::*`) does **not** re-export `#[macro_export]` macros, so we
// name them explicitly — hence `frus::main!`, `frus::column!`, `frus::row!`.
pub use frus_shell::main;
pub use frus_widgets::{asset, column, row};

// Fonts: registering the application's own faces, and naming the families text
// resolves to. An application that ships its own can drop the bundled ones through
// the `bundled-*` features — see the "Shipping" section of the getting-started guide.
pub mod fonts {
    pub use frus_text::{add_font, set_default_family, set_monospace_family};
}

// Cross-platform HTTP (feature `net`): the `frus::fetch(url).await` shorthand and the
// `frus::Request` builder (method / headers / body / timeout) — see [`frus::net`].
#[cfg(feature = "net")]
pub use frus_shell::{fetch, fetch_bytes, net, FetchError, Method, Request, MAX_RESPONSE_BYTES};
