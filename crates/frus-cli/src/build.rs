//! `frus build <target>`: what you ship, for one platform, in release.

use std::path::Path;
use std::process::ExitCode;

use crate::cargo::{self, Mode};
use crate::project::Project;

/// What `frus build` makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// The Android package.
    Apk,
    /// The site: `web/`, with the release build in `web/pkg/`.
    Web,
    /// The desktop program, for the system this runs on.
    Desktop,
}

impl Target {
    pub(crate) fn parse(word: &str) -> Result<Target, String> {
        match word {
            "apk" | "android" => Ok(Target::Apk),
            "web" => Ok(Target::Web),
            "desktop" => Ok(Target::Desktop),
            "windows" | "linux" | "macos" if word == std::env::consts::OS => Ok(Target::Desktop),
            "windows" | "linux" | "macos" => Err(format!(
                "a {word} program is built on {word}: this is {}",
                std::env::consts::OS
            )),
            other => Err(format!(
                "frus build makes `apk`, `web` or `{}`, not `{other}`",
                std::env::consts::OS
            )),
        }
    }
}

pub(crate) fn build(target: Target, mode: Mode) -> Result<ExitCode, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = Project::find(&here)?;
    match target {
        Target::Apk => {
            let apk = cargo::build_apk(&project, mode, None)?;
            println!("\nBuilt {} ({}).", apk.display(), size(&apk));
            if mode == Mode::Release {
                println!("Install it with `adb install -r <apk>`; a store wants it signed (see README.md, \"Signing\").");
            }
        }
        Target::Web => {
            let site = cargo::build_web(&project, mode)?;
            let wasm = site.join("pkg").join(format!("{}_bg.wasm", project.lib));
            println!(
                "\nBuilt the site: {} (the application is {}).\n\
                 Put that directory on any static host, and serve the .wasm compressed.\n\
                 Try it here first: `frus run -d web{}`.",
                site.display(),
                size(&wasm),
                if mode == Mode::Release {
                    " --release"
                } else {
                    ""
                }
            );
        }
        Target::Desktop => {
            let exe = cargo::build_desktop(&project, mode)?;
            println!("\nBuilt {} ({}).", exe.display(), size(&exe));
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn size(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(meta) => format!("{:.1} MB", meta.len() as f64 / 1_048_576.0),
        Err(_) => "size unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets() {
        assert_eq!(Target::parse("apk"), Ok(Target::Apk));
        assert_eq!(Target::parse("android"), Ok(Target::Apk));
        assert_eq!(Target::parse("web"), Ok(Target::Web));
        assert_eq!(Target::parse("desktop"), Ok(Target::Desktop));
        assert_eq!(Target::parse(std::env::consts::OS), Ok(Target::Desktop));
        assert!(Target::parse("ios").unwrap_err().contains("not `ios`"));
        let elsewhere = if cfg!(windows) { "linux" } else { "windows" };
        assert!(Target::parse(elsewhere)
            .unwrap_err()
            .contains("is built on"));
    }
}
