//! Binaire bureau : appelle le point d'entrée `run()` engendré par `frus::main!` dans la
//! bibliothèque. (Sur Android/Web l'entrée est `android_main` / `start`, elles aussi
//! engendrées par la macro.)
//!
//! `cargo run -p frus-fetch-example` (ajouter `RUST_LOG=info` pour les logs).

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn main() -> frus::anyhow::Result<()> {
    frus_fetch_example::run()
}

#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn main() {}
