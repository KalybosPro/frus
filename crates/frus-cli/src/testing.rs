//! `frus test`: the application's tests, run by cargo and said briefly (milestone 678).
//!
//! `cargo test` prints every test's line and, for a failure, its whole output. A person wants to
//! know how many passed, which failed, where, and why: the summary first, then each failure at
//! its place in the code (`src/counter.rs:40:9`), with its message. A build that does not
//! compile is said the same way, each error at its place. `-v` shows cargo's own output instead.
//!
//! The test harness's JSON output is not stable Rust, so its text is read: the lines it has
//! printed the same way for years.

use std::process::{Command, ExitCode, Stdio};

use crate::project::Project;

/// What a test run said.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Report {
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
    pub failures: Vec<Failure>,
    /// Compile errors, each `place: error[code]: message`.
    pub build_errors: Vec<String>,
}

/// One failed test.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Failure {
    pub name: String,
    /// Where it panicked: `src/counter.rs:40:9`.
    pub place: Option<String>,
    /// What it said.
    pub message: Vec<String>,
}

pub(crate) fn test(filter: Option<&str>, verbose: bool) -> Result<ExitCode, String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let project = Project::find(&here)?;
    let mut command = Command::new("cargo");
    command
        .args(["test", "--no-fail-fast"])
        .current_dir(&project.root);
    if let Some(filter) = filter {
        command.arg(filter);
    }
    if verbose {
        let status = command
            .status()
            .map_err(|e| format!("could not run cargo: {e}"))?;
        return Ok(if status.success() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }
    println!("Testing {}…", project.package);
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    let report = read(
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    );
    print!("{}", render(&report));
    Ok(if output.status.success() && report.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Reads what `cargo test` printed: the test harness on standard output, cargo and the compiler
/// on standard error.
pub(crate) fn read(stdout: &str, stderr: &str) -> Report {
    let mut report = Report::default();
    let lines: Vec<&str> = stdout.lines().collect();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        if let Some(rest) = line.strip_prefix("test result: ") {
            let (passed, failed, ignored) = counts(rest);
            report.passed += passed;
            report.failed += failed;
            report.ignored += ignored;
        } else if let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|l| l.strip_suffix(" stdout ----"))
        {
            // A failed test's output, up to the next one or the list of failures.
            let mut failure = Failure {
                name: name.trim().to_string(),
                ..Failure::default()
            };
            let mut body = Vec::new();
            at += 1;
            while at < lines.len()
                && !lines[at].starts_with("---- ")
                && lines[at] != "failures:"
                && !lines[at].starts_with("test result:")
            {
                body.push(lines[at]);
                at += 1;
            }
            read_panic(&body, &mut failure);
            report.failures.push(failure);
            continue;
        }
        at += 1;
    }
    report.build_errors = compile_errors(stderr);
    report
}

/// `ok. 85 passed; 1 failed; 2 ignored; 0 measured; …` → (85, 1, 2).
fn counts(text: &str) -> (u32, u32, u32) {
    let mut found = (0, 0, 0);
    for part in text.split([';', '.']) {
        let words: Vec<&str> = part.split_whitespace().collect();
        if let [number, what, ..] = words.as_slice() {
            let Ok(n) = number.parse::<u32>() else {
                continue;
            };
            match *what {
                "passed" => found.0 = n,
                "failed" => found.1 = n,
                "ignored" => found.2 = n,
                _ => {}
            }
        }
    }
    found
}

/// The place and the message of a failed test's panic:
/// `thread 'name' (1234) panicked at src/counter.rs:40:9:` then the message, up to the notes.
fn read_panic(body: &[&str], failure: &mut Failure) {
    let Some(start) = body.iter().position(|l| l.contains("panicked at ")) else {
        // No panic (a test that returned an error, say): its output is the message.
        failure.message = body
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect();
        return;
    };
    let line = body[start];
    let place = line
        .split("panicked at ")
        .nth(1)
        .map(|p| p.trim_end_matches(':').trim().to_string());
    failure.place = place;
    failure.message = body[start + 1..]
        .iter()
        .take_while(|l| !l.starts_with("note:") && !l.starts_with("stack backtrace:"))
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();
}

/// The compile errors in cargo's output, each at its place: `error[E0425]: …` and its
/// `--> src/x.rs:3:5` line become `src/x.rs:3:5: error[E0425]: …`.
pub(crate) fn compile_errors(stderr: &str) -> Vec<String> {
    let lines: Vec<&str> = stderr.lines().collect();
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let is_error = line.starts_with("error[") || line.starts_with("error:");
        if !is_error
            || line.starts_with("error: could not compile")
            || line.starts_with("error: test failed")
            || line.starts_with("error: aborting")
            || line.starts_with("error: process didn't exit")
            || line.starts_with("error: build failed")
            || line.starts_with("error: 1 target failed")
            || line.contains("targets failed")
        {
            continue;
        }
        let place = lines[i + 1..]
            .iter()
            .take(3)
            .find_map(|l| l.trim_start().strip_prefix("--> "))
            .map(str::trim);
        errors.push(match place {
            Some(place) => format!("{place}: {line}"),
            None => line.to_string(),
        });
    }
    errors
}

/// The report, for a person.
pub(crate) fn render(report: &Report) -> String {
    let mut out = String::new();
    if !report.build_errors.is_empty() {
        out.push_str("The tests do not compile:\n");
        for error in &report.build_errors {
            out.push_str(&format!("  {error}\n"));
        }
        return out;
    }
    let mut summary = format!("{} passed", report.passed);
    if report.failed > 0 {
        summary.push_str(&format!(", {} failed", report.failed));
    }
    if report.ignored > 0 {
        summary.push_str(&format!(", {} ignored", report.ignored));
    }
    out.push_str(&format!(
        "{} Tests: {summary}.\n",
        if report.failed == 0 { "✓" } else { "✗" }
    ));
    for failure in &report.failures {
        out.push_str(&format!("\n✗ {}\n", failure.name));
        if let Some(place) = &failure.place {
            out.push_str(&format!("    {place}\n"));
        }
        for line in &failure.message {
            out.push_str(&format!("    {line}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: &str = "\
running 3 tests
test counter::tests::starts_at_zero ... ok
test counter::tests::adds ... FAILED
test counter::tests::slow ... ignored

failures:

---- counter::tests::adds stdout ----

thread 'counter::tests::adds' (4512) panicked at src/counter.rs:40:9:
assertion `left == right` failed
  left: 1
 right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    counter::tests::adds

test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 2 tests
test a ... ok
test b ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";

    #[test]
    fn a_run_is_counted_and_its_failures_placed() {
        let report = read(RUN, "");
        assert_eq!((report.passed, report.failed, report.ignored), (3, 1, 1));
        assert_eq!(report.failures.len(), 1);
        let failure = &report.failures[0];
        assert_eq!(failure.name, "counter::tests::adds");
        assert_eq!(failure.place.as_deref(), Some("src/counter.rs:40:9"));
        assert_eq!(
            failure.message,
            ["assertion `left == right` failed", "  left: 1", " right: 2"]
        );
        let shown = render(&report);
        assert!(
            shown.starts_with("✗ Tests: 3 passed, 1 failed, 1 ignored.\n"),
            "{shown}"
        );
        assert!(
            shown.contains("\n✗ counter::tests::adds\n    src/counter.rs:40:9\n"),
            "{shown}"
        );
    }

    #[test]
    fn a_green_run_says_so() {
        let report = read(
            "test result: ok. 85 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n",
            "",
        );
        assert_eq!(render(&report), "✓ Tests: 85 passed.\n");
    }

    #[test]
    fn a_build_that_does_not_compile_is_said_at_its_places() {
        let stderr = "\
   Compiling counter v0.1.0
error[E0425]: cannot find value `x` in this scope
  --> src/counter.rs:12:5
   |
12 |     x
   |     ^ not found in this scope

error: could not compile `counter` (lib test) due to 1 previous error
";
        let report = read("", stderr);
        assert_eq!(
            report.build_errors,
            ["src/counter.rs:12:5: error[E0425]: cannot find value `x` in this scope"]
        );
        assert!(render(&report).starts_with("The tests do not compile:\n"));
    }
}
