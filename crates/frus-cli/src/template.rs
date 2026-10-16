//! The application template, built into the program.
//!
//! The files live in `template/`, beside this crate's manifest, so that `cargo publish` ships
//! them inside `frus-cli` and `cargo install` builds them in: a `frus` installed from crates.io
//! needs no checkout and no network to create an application. A test compares this list with
//! the directory, so a file added to one and not the other fails the build's tests.

/// One file of the template: its path inside the template, with `/` separators and the
/// template syntax still in it, and its bytes.
pub struct TemplateFile {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

macro_rules! files {
    ($($path:literal),* $(,)?) => {
        &[$(TemplateFile {
            path: $path,
            bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/template/", $path)),
        }),*]
    };
}

/// Every file of the template, in byte order of their paths.
pub static FILES: &[TemplateFile] = files![
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    ".gitignore",
    "Cargo.toml.liquid",
    "README.md",
    "cargo-generate.toml",
    "res/mipmap-hdpi/ic_launcher.png",
    "res/mipmap-mdpi/ic_launcher.png",
    "res/mipmap-xhdpi/ic_launcher.png",
    "res/mipmap-xxhdpi/ic_launcher.png",
    "res/mipmap-xxxhdpi/ic_launcher.png",
    "res/values/styles.xml",
    "src/bin/{{project-name}}.rs",
    "src/counter.rs",
    "src/lib.rs",
    "tests/counter_test.rs",
    "web/index.html",
];
