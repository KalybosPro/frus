//! `frus clean`: the application's builds taken away, so the next one starts from nothing
//! (milestone 673).
//!
//! When the build folder is the application's own (`target/` beside it), all of it goes. When it
//! is shared — `CARGO_TARGET_DIR`, or a workspace's — only this application's part does, for each
//! platform it builds for: another application's build in the same folder is left alone. The
//! web build's generated glue (`web/pkg`) and `frus run --hot`'s shadow package go too.

use std::path::Path;
use std::process::{Command, ExitCode};

use crate::project::Project;

/// The platforms whose builds a shared folder holds for an application, besides this computer's.
const TARGETS: &[&str] = &["wasm32-unknown-unknown", "aarch64-linux-android"];

pub(crate) fn clean() -> Result<ExitCode, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = Project::find(&here)?;
    let target = project.target_dir()?;
    if owns(&project.root, &target) {
        println!("Deleting {}…", shown(&target, &project.root));
        crate::cargo::run(
            Command::new("cargo")
                .arg("clean")
                .current_dir(&project.root),
        )?;
    } else {
        println!(
            "The build folder {} is shared: deleting {}'s builds only…",
            target.display(),
            project.package
        );
        let clean = |extra: &[&str]| {
            crate::cargo::run(
                Command::new("cargo")
                    .args(["clean", "-p", &project.package])
                    .args(extra)
                    .current_dir(&project.root),
            )
        };
        clean(&[])?;
        clean(&["--release"])?;
        for platform in TARGETS {
            // A platform never built for has nothing to clean, and cargo may say so: not an error.
            let _ = clean(&["--target", platform]);
            let _ = clean(&["--target", platform, "--release"]);
        }
        remove(
            &target.join("frus-hot").join(&project.package),
            &project.root,
        );
    }
    remove(&project.root.join("web").join("pkg"), &project.root);
    println!("Done: the next build starts from nothing.");
    Ok(ExitCode::SUCCESS)
}

/// Whether `target` belongs to the application at `root` alone.
pub(crate) fn owns(root: &Path, target: &Path) -> bool {
    let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canonical(target).starts_with(canonical(root))
}

fn remove(dir: &Path, root: &Path) {
    if dir.exists() {
        println!("Deleting {}…", shown(dir, root));
        if let Err(error) = std::fs::remove_dir_all(dir) {
            crate::report::warn(&format!("could not delete {}: {error}", dir.display()));
        }
    }
}

/// `dir` as the developer would type it from the application: relative when inside it.
fn shown(dir: &Path, root: &Path) -> String {
    dir.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| dir.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_folder_is_not_the_applications() {
        let base = std::env::temp_dir().join(format!("frus-clean-{}", std::process::id()));
        let app = base.join("app");
        std::fs::create_dir_all(app.join("target")).unwrap();
        std::fs::create_dir_all(base.join("shared")).unwrap();
        assert!(owns(&app, &app.join("target")));
        assert!(!owns(&app, &base.join("shared")));
        assert_eq!(shown(&app.join("target"), &app), "target");
        std::fs::remove_dir_all(&base).unwrap();
    }
}
