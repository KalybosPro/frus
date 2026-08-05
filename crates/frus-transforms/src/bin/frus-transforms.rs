//! Binaire bureau : appelle le point d'entrée `run()` engendré par `frus_shell::main!`
//! dans la bibliothèque. (Sur Android/Web l'entrée est `android_main` / `start`.)
//!
//! `cargo run -p frus-transforms` (ajouter `RUST_LOG=info` pour les logs).

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn main() -> frus_shell::anyhow::Result<()> {
    frus_transforms::run()
}

#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn main() {}
