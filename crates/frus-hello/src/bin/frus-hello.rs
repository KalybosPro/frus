//! Binaire bureau : délègue au point d'entrée de la bibliothèque.
//!
//! `cargo run -p frus-hello` (ajouter `RUST_LOG=info` pour les logs).

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn main() -> anyhow::Result<()> {
    frus_hello::run_desktop()
}

// Ni sur Android ni sur le Web il n'y a de binaire : l'entrée est `android_main` /
// `start` dans la lib.
#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn main() {}
