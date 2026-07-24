//! Binaire bureau : délègue au point d'entrée de la bibliothèque.
//!
//! `cargo run -p frus-transforms` (ajouter `RUST_LOG=info` pour les logs).

#[cfg(not(target_os = "android"))]
fn main() -> anyhow::Result<()> {
    frus_transforms::run_desktop()
}

// Sur Android il n'y a pas de binaire : l'entrée est `android_main` dans la lib.
#[cfg(target_os = "android")]
fn main() {}
