//! Démonstration du Jalon 0 : ouvre une fenêtre affichant un quad coloré.
//!
//! Lancer avec : `cargo run -p frus-demo`
//! (ajouter `RUST_LOG=info` pour les logs.)

fn main() -> anyhow::Result<()> {
    frus_shell::run()
}
