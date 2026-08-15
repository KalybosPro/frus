//! Renders the README's pictures. See `frus_demo::shots`.
//!
//! ```sh
//! cargo run -p frus-demo --features shots --bin shots -- docs/media
//! ```

fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/media".to_string());
    frus_demo::shots::write_previews(std::path::Path::new(&dir))
}
