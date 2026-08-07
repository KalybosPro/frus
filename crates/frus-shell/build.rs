//! Platform `cfg` aliases for frus.
//!
//! `frus-shell` is the only platform-dependent layer, and until now "desktop" was
//! written here **by negation**: `not(any(target_os = "android", target_arch =
//! "wasm32"))`. That spelling has a fatal flaw the moment a fourth platform
//! arrives: **iOS falls into it silently**, inheriting the `arboard` clipboard,
//! `env_logger` and AccessKit — three things with no UIKit backend. The code might
//! even compile, and it would be wrong.
//!
//! So we name the platforms explicitly, once. Adding a target now touches only
//! this file.
//!
//! Two limits worth knowing:
//!
//! 1. These aliases are `--cfg` flags passed to **this crate only**. They are not
//!    visible in the application's crate — which is why the body of the
//!    `frus_shell::main!` macro (which expands at the user's side) keeps explicit
//!    `target_os` / `target_arch` predicates.
//! 2. Cargo does not evaluate these aliases in `[target.'cfg(…)'.dependencies]`
//!    tables in `Cargo.toml`: dependency selection there stays written in
//!    `target_os` / `target_arch`.

fn main() {
    cfg_aliases::cfg_aliases! {
        // The three platforms in service.
        web: { target_arch = "wasm32" },
        android: { target_os = "android" },
        // The target currently being bootstrapped (milestone 276 onwards).
        ios: { target_os = "ios" },
        // "Desktop" = Windows / macOS / Linux: winit with a window, a system
        // clipboard, AccessKit and logs on stderr. Still defined by excluding the
        // others, but **iOS is now excluded too** — that is the whole point.
        //
        // Written by reusing the aliases above: `cfg_aliases` blows its recursion
        // limit if handed the full `target_os` / `target_arch` list.
        desktop: { not(any(web, android, ios)) },
    }
}
