//! Where `frus run` says things: to a person, or to an editor.
//!
//! By default everything is plain text in the terminal. With `--machine`, standard output carries
//! **one JSON object a line** and nothing else, for an editor to read: events
//! (`{"event":"app.log","params":{…}}`) and answers to its commands
//! (`{"id":1,"result":true}`). What is meant for a person then goes into events too —
//! `app.progress` for the tool's own messages, `app.log` for the application's — and the
//! programs the tool runs (cargo, cargo-apk, wasm-bindgen) write to standard error, where an
//! editor shows them as they are.

use std::io::Write;
use std::sync::{Mutex, OnceLock};

use crate::json::{object, Value};

/// The run's identity in its events, once `--machine` asked for them.
static MACHINE: OnceLock<String> = OnceLock::new();
/// One line at a time on standard output, whatever thread writes it.
static STDOUT: Mutex<()> = Mutex::new(());

/// Switches the run to JSON lines; `app_id` names it in every event.
pub(crate) fn machine(app_id: String) {
    let _ = MACHINE.set(app_id);
}

pub(crate) fn is_machine() -> bool {
    MACHINE.get().is_some()
}

/// An event, with the run's `appId` among its parameters (machine mode only).
pub(crate) fn event(name: &str, params: Value) {
    let Some(app_id) = MACHINE.get() else {
        return;
    };
    let mut members = vec![("appId".to_string(), Value::from(app_id.as_str()))];
    if let Value::Object(more) = params {
        members.extend(more);
    }
    line(&object([
        ("event", name.into()),
        ("params", Value::Object(members)),
    ]));
}

/// A JSON line on standard output, whole.
pub(crate) fn line(value: &Value) {
    let _guard = STDOUT.lock();
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", value.to_json());
    let _ = out.flush();
}

/// What the tool tells the developer: where it is, what it does.
pub(crate) fn say(message: &str) {
    if is_machine() {
        event("app.progress", object([("message", message.into())]));
    } else {
        println!("{message}");
    }
}

/// Something that went wrong without ending the run.
pub(crate) fn warn(message: &str) {
    if is_machine() {
        event(
            "app.progress",
            object([("message", message.into()), ("error", true.into())]),
        );
    } else {
        eprintln!("{message}");
    }
}

/// A line of the application's own log: its output, the phone's logcat, the browser's console.
pub(crate) fn app_log(text: &str, error: bool) {
    if is_machine() {
        event(
            "app.log",
            object([("log", text.into()), ("error", error.into())]),
        );
    } else if error {
        eprintln!("{text}");
    } else {
        println!("{text}");
    }
}
