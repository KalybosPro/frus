//! `frus analyze`: what clippy and rustfmt find in the application, listed together (milestone
//! 678).
//!
//! Each lint at its place (`src/x.rs:12:5: warning[clippy::needless_return]: …`), then the files
//! rustfmt would change; the exit code is not zero while anything is left, which is what a CI step
//! or a pre-commit hook wants. `--fix` formats the files and applies clippy's own suggestions
//! first. The components are installed if the toolchain lacks them.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use crate::json::{parse, Value};
use crate::project::Project;

pub(crate) fn analyze(fix: bool) -> Result<ExitCode, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = Project::find(&here)?;
    for (component, probe) in [("clippy", "clippy"), ("rustfmt", "fmt")] {
        let present = Command::new("cargo")
            .args([probe, "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !present {
            crate::report::say(&format!("Installing {component} (rustup component add)…"));
            crate::cargo::run(Command::new("rustup").args(["component", "add", component]))?;
        }
    }
    if fix {
        println!("Formatting, and applying clippy's suggestions…");
        crate::cargo::run(Command::new("cargo").arg("fmt").current_dir(&project.root))?;
        let _ = Command::new("cargo")
            .args([
                "clippy",
                "--fix",
                "--allow-dirty",
                "--allow-staged",
                "--allow-no-vcs",
                "--all-targets",
                "--quiet",
            ])
            .current_dir(&project.root)
            .stdin(Stdio::null())
            .status();
    }
    println!("Analyzing {} (clippy, rustfmt)…", project.package);
    let clippy = Command::new("cargo")
        .args(["clippy", "--all-targets", "--message-format=json"])
        .current_dir(&project.root)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("could not run cargo clippy: {e}"))?;
    let lints = lints(&String::from_utf8_lossy(&clippy.stdout));
    let fmt = Command::new("cargo")
        .args(["fmt", "--check"])
        .current_dir(&project.root)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run cargo fmt: {e}"))?;
    let unformatted = unformatted(&String::from_utf8_lossy(&fmt.stdout), &project.root);
    print!("{}", render(&lints, &unformatted));
    Ok(if lints.is_empty() && unformatted.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The warnings and errors in clippy's JSON messages, each at its place, without repeats (a
/// file in two targets is linted twice).
pub(crate) fn lints(json_lines: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    for line in json_lines.lines() {
        let Ok(value) = parse(line) else {
            continue;
        };
        if value.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let level = message.get("level").and_then(Value::as_str).unwrap_or("");
        if level != "warning" && level != "error" {
            continue;
        }
        let Some(span) = message
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
            })
        else {
            // "3 warnings emitted": a summary, not a lint.
            continue;
        };
        let file = span.get("file_name").and_then(Value::as_str).unwrap_or("");
        let row = span
            .get("line_start")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let column = span
            .get("column_start")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let code = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(Value::as_str);
        let text = message.get("message").and_then(Value::as_str).unwrap_or("");
        let entry = match code {
            Some(code) => format!("{file}:{row}:{column}: {level}[{code}]: {text}"),
            None => format!("{file}:{row}:{column}: {level}: {text}"),
        };
        if seen.insert(entry.clone()) {
            found.push(entry);
        }
    }
    found
}

/// The files `cargo fmt --check` would change, relative to the application: its output names
/// each as `Diff in <path>:<line>:` (or, older, `Diff in <path> at line <n>:`).
pub(crate) fn unformatted(output: &str, root: &Path) -> Vec<String> {
    let mut files = BTreeSet::new();
    for line in output.lines() {
        let Some(rest) = line.strip_prefix("Diff in ") else {
            continue;
        };
        let path = match rest.find(" at line ") {
            Some(at) => &rest[..at],
            None => rest
                .trim_end_matches(':')
                .rsplit_once(':')
                .map_or(rest, |(path, line)| {
                    if line.parse::<u32>().is_ok() {
                        path
                    } else {
                        rest
                    }
                }),
        };
        let path = path.trim_start_matches(r"\\?\");
        let shown = Path::new(path)
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.to_string());
        files.insert(shown);
    }
    files.into_iter().collect()
}

/// What was found, for a person.
pub(crate) fn render(lints: &[String], unformatted: &[String]) -> String {
    let mut out = String::new();
    for lint in lints {
        out.push_str(&format!("{lint}\n"));
    }
    if !unformatted.is_empty() {
        if !lints.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!(
            "{} not formatted: {}\n",
            if unformatted.len() == 1 {
                "1 file is".to_string()
            } else {
                format!("{} files are", unformatted.len())
            },
            unformatted.join(", ")
        ));
    }
    let issues = lints.len() + unformatted.len();
    if issues == 0 {
        out.push_str("✓ No issues found.\n");
    } else {
        out.push_str(&format!(
            "✗ {issues} issue{} found (`frus analyze --fix` formats the files and applies clippy's \
             own suggestions).\n",
            if issues == 1 { "" } else { "s" }
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clippy_lints_are_placed_and_said_once() {
        let message = r#"{"reason":"compiler-message","message":{"message":"unneeded `return` statement","code":{"code":"clippy::needless_return","explanation":null},"level":"warning","spans":[{"file_name":"src/counter.rs","line_start":12,"column_start":5,"is_primary":true}]}}"#;
        let summary = r#"{"reason":"compiler-message","message":{"message":"1 warning emitted","code":null,"level":"warning","spans":[]}}"#;
        let artifact = r#"{"reason":"compiler-artifact","package_id":"x"}"#;
        let note = r#"{"reason":"compiler-message","message":{"message":"x","code":null,"level":"note","spans":[{"file_name":"a.rs","line_start":1,"column_start":1,"is_primary":true}]}}"#;
        let input = [message, summary, artifact, message, note, "not json"].join("\n");
        assert_eq!(
            lints(&input),
            ["src/counter.rs:12:5: warning[clippy::needless_return]: unneeded `return` statement"]
        );
    }

    #[test]
    fn the_files_to_format_are_named_from_the_application() {
        let root = Path::new("/apps/shop");
        let output = "Diff in /apps/shop/src/lib.rs:3:\n-fn a(){}\n+fn a() {}\nDiff in /apps/shop/src/lib.rs:9:\nDiff in /apps/shop/src/b.rs at line 1:\n";
        assert_eq!(unformatted(output, root), ["src/b.rs", "src/lib.rs"]);
    }

    #[test]
    fn the_report_says_how_to_fix_it() {
        assert_eq!(render(&[], &[]), "✓ No issues found.\n");
        let shown = render(&["src/a.rs:1:1: warning: x".into()], &["src/b.rs".into()]);
        assert!(
            shown.starts_with("src/a.rs:1:1: warning: x\n\n1 file is not formatted: src/b.rs\n"),
            "{shown}"
        );
        assert!(shown.contains("✗ 2 issues found"), "{shown}");
    }
}
