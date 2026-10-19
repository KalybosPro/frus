//! `frus emulators`: the emulators that can be started, starting one, making one (milestone 673).
//!
//! The same work as the daemon's `emulator.*` methods (see `daemon`), said for a person.

use std::process::ExitCode;

use crate::daemon;
use crate::json::Value;

/// What `frus emulators` was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    List,
    /// `--launch <id>`, `--cold` for a start without its saved state.
    Launch {
        id: String,
        cold: bool,
    },
    /// `--create`, `--name <name>`.
    Create {
        name: Option<String>,
    },
}

pub(crate) fn emulators(action: Action) -> Result<ExitCode, String> {
    match action {
        Action::List => {
            print!("{}", list(&daemon::emulators()));
            Ok(ExitCode::SUCCESS)
        }
        Action::Launch { id, cold } => {
            daemon::launch_emulator(Some(id), cold)?;
            println!("It shows in `frus devices` once it has started; `frus run -d <its id>` then runs on it.");
            Ok(ExitCode::SUCCESS)
        }
        Action::Create { name } => {
            let answer = daemon::create_emulator(name.as_deref());
            let created = answer.get("emulatorName").and_then(Value::as_str);
            match (answer.get("success").and_then(Value::as_bool), created) {
                (Some(true), Some(created)) => {
                    println!(
                        "Created the {} emulator. Start it: frus emulators --launch {created}",
                        created.replace('_', " ")
                    );
                    Ok(ExitCode::SUCCESS)
                }
                _ => Err(answer
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("the emulator was not created")
                    .to_string()),
            }
        }
    }
}

/// The emulators, as a table, with what to do next.
pub(crate) fn list(emulators: &[Value]) -> String {
    let text = |e: &Value, key: &str| e.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let mut out = String::new();
    if emulators.is_empty() {
        out.push_str(
            "No emulators available.\n\nTo make one: frus emulators --create [--name <name>]\n",
        );
        return out;
    }
    out.push_str(&format!(
        "{} available emulator{}:\n\n",
        emulators.len(),
        if emulators.len() == 1 { "" } else { "s" }
    ));
    let rows: Vec<[String; 3]> = emulators
        .iter()
        .map(|e| [text(e, "id"), text(e, "name"), text(e, "platformType")])
        .collect();
    let widths: Vec<usize> = (0..3)
        .map(|i| rows.iter().map(|r| r[i].chars().count()).max().unwrap_or(0))
        .collect();
    for row in &rows {
        out.push_str(&format!(
            "{:w0$} • {:w1$} • {}\n",
            row[0],
            row[1],
            row[2],
            w0 = widths[0],
            w1 = widths[1]
        ));
    }
    out.push_str(
        "\nTo start one: frus emulators --launch <id>   (--cold: without its saved state)\n\
         To make one:  frus emulators --create [--name <name>]\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::object;

    #[test]
    fn the_list_says_what_to_do_next() {
        let none = list(&[]);
        assert!(none.starts_with("No emulators available."), "{none}");
        assert!(none.contains("frus emulators --create"), "{none}");
        let some = list(&[
            object([
                ("id", "pixel_7".into()),
                ("name", "pixel 7".into()),
                ("platformType", "android".into()),
            ]),
            object([
                ("id", "frus_emulator".into()),
                ("name", "frus emulator".into()),
                ("platformType", "android".into()),
            ]),
        ]);
        assert!(some.starts_with("2 available emulators:\n\n"), "{some}");
        assert!(
            some.contains("pixel_7       • pixel 7       • android\n"),
            "{some}"
        );
        assert!(some.contains("--launch <id>"), "{some}");
    }
}
