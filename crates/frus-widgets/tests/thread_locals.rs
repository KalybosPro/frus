//! Every thread-local of frus is read and written only by functions that are neither generic nor
//! inlined (milestone 669).
//!
//! A generic function is compiled into the application that uses it, and so is an inlined one.
//! A hot patch of the application then carries its own copy of every thread-local such a
//! function touches directly — fresh, apart from the one the rest of frus holds — and the
//! patched code works on state nobody else sees. Kept to functions compiled once, in frus, the
//! accesses all reach the same thread-local, whatever has been patched.

use std::path::{Path, PathBuf};

/// The crates whose code ends up in an application's: the ones with generic widgets and the
/// shell's `App<A>`.
const CRATES: &[&str] = &["frus-core", "frus-widgets", "frus-shell"];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The names a file's `thread_local!` blocks declare.
fn thread_locals(lines: &[&str]) -> Vec<String> {
    let mut names = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("thread_local!") {
            inside = true;
        } else if inside && *line == "}" {
            inside = false;
        } else if inside {
            if let Some(rest) = trimmed.strip_prefix("static ") {
                if let Some(name) = rest.split(':').next() {
                    names.push(name.trim().to_string());
                }
            }
        }
    }
    names
}

fn is_fn_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let trimmed = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);
    trimmed.starts_with("fn ")
}

#[test]
fn thread_locals_are_reached_through_functions_compiled_once() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in CRATES {
        sources(&root.join(krate).join("src"), &mut files);
    }
    // Run from a package without its siblings (a published crate): nothing to look at.
    if files.is_empty() {
        return;
    }

    let mut wrong = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap().replace("\r\n", "\n");
        let lines: Vec<&str> = text.lines().collect();
        let names = thread_locals(&lines);
        if names.is_empty() {
            continue;
        }
        // The tests are not shipped in an application.
        let end = lines
            .iter()
            .position(|l| l.starts_with("#[cfg(test)]"))
            .unwrap_or(lines.len());
        for (at, line) in lines[..end].iter().enumerate() {
            let touches = names.iter().any(|name| {
                [".with(", ".set(", ".get(", ".take(", ".replace("]
                    .iter()
                    .any(|call| line.contains(&format!("{name}{call}")))
            });
            if !touches {
                continue;
            }
            let Some(fn_at) = (0..=at).rev().find(|i| is_fn_line(lines[*i])) else {
                continue;
            };
            let signature = lines[fn_at];
            let name_part = signature.split('(').next().unwrap_or("");
            let generic = name_part.contains('<') || signature.contains("impl ");
            let attributes = (0..fn_at)
                .rev()
                .take_while(|i| lines[*i].trim().starts_with("#["))
                .any(|i| lines[i].contains("#[inline(never)]"));
            if generic || !attributes {
                wrong.push(format!(
                    "{}:{}: {}{}",
                    file.strip_prefix(&root).unwrap_or(file).display(),
                    fn_at + 1,
                    signature.trim(),
                    if generic {
                        "  (generic)"
                    } else {
                        "  (no #[inline(never)])"
                    }
                ));
            }
        }
    }
    wrong.sort();
    wrong.dedup();
    assert!(
        wrong.is_empty(),
        "these functions touch a thread-local and would be copied into an application:\n{}",
        wrong.join("\n")
    );
}
