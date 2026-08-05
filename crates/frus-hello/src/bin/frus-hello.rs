//! Binaire bureau : appelle le point d'entrée `run()` engendré par `frus_shell::main!`
//! dans la bibliothèque. (Sur Android/Web il n'y a pas de binaire : l'entrée est
//! `android_main` / `start`, elles aussi engendrées par la macro.)
//!
//! `cargo run -p frus-hello` (ajouter `RUST_LOG=info` pour les logs).

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn main() -> frus_shell::anyhow::Result<()> {
    frus_hello::run()
}

#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn main() {}
