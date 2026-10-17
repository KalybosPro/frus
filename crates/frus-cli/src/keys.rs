//! Single key presses from the terminal, while `frus run` is attached: `R` restarts, `q` quits.
//!
//! A terminal hands a program whole lines by default, echoed as they are typed. Reading one key
//! at a time means switching that off for the length of the run — through the console's mode on
//! Windows, `stty` elsewhere — and putting it back afterwards, including when the run is ended
//! with Ctrl+C, or the developer's shell would be left without echo.
//!
//! When the input is not a terminal (an editor drives the run through a pipe), keys arrive as
//! lines instead, and the first character of each counts. With `--machine`, each line is a JSON
//! command instead — `{"id":1,"method":"app.restart","params":{"fullRestart":false}}` — answered
//! on standard output.

use std::io::{IsTerminal, Read};
use std::sync::mpsc::{self, Receiver};

/// Reads keys on a thread of its own; each arrives on the channel returned.
pub(crate) fn listen() -> Receiver<char> {
    if crate::report::is_machine() {
        return commands();
    }
    let (send, receive) = mpsc::channel();
    let raw = std::io::stdin().is_terminal() && terminal::raw();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        if raw {
            let mut byte = [0u8; 1];
            while stdin.read(&mut byte).is_ok_and(|n| n == 1) {
                if send.send(byte[0] as char).is_err() {
                    break;
                }
            }
        } else {
            let mut line = String::new();
            while std::io::BufRead::read_line(&mut stdin, &mut line).is_ok_and(|n| n > 0) {
                if let Some(key) = line.trim().chars().next() {
                    if send.send(key).is_err() {
                        break;
                    }
                }
                line.clear();
            }
        }
    });
    receive
}

/// An editor's commands, as the keys they stand for.
fn commands() -> Receiver<char> {
    use crate::json::{object, parse, Value};
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in std::io::BufRead::lines(stdin.lock()).map_while(Result::ok) {
            let Ok(command) = parse(&line) else {
                continue;
            };
            let id = command.get("id").cloned().unwrap_or(Value::Null);
            let key = command_key(&command);
            let answer = match key {
                Some(_) => object([("id", id), ("result", true.into())]),
                None => object([("id", id), ("error", "unknown method".into())]),
            };
            crate::report::line(&answer);
            if let Some(key) = key {
                if send.send(key).is_err() {
                    break;
                }
            }
        }
    });
    receive
}

/// The key an editor's command stands for.
pub(crate) fn command_key(command: &crate::json::Value) -> Option<char> {
    let full = command
        .get("params")
        .and_then(|p| p.get("fullRestart"))
        .and_then(crate::json::Value::as_bool)
        .unwrap_or(false);
    match command.get("method").and_then(crate::json::Value::as_str)? {
        "app.restart" if full => Some('R'),
        "app.restart" => Some('r'),
        "app.stop" => Some('q'),
        "app.detach" => Some('d'),
        // Any other key of the run's, by its letter: `{"method":"app.key","params":{"key":"p"}}`.
        "app.key" => {
            let key = command.get("params")?.get("key")?.as_str()?;
            let mut chars = key.chars();
            let key = chars.next().filter(|_| chars.next().is_none())?;
            crate::run::action(key).map(|_| key)
        }
        _ => None,
    }
}

/// Puts the terminal back as it was: called when the run ends.
pub(crate) fn restore() {
    terminal::restore();
}

#[cfg(windows)]
mod terminal {
    //! The Windows console, through kernel32 — which every Windows program links already.

    use std::sync::atomic::{AtomicU32, Ordering};

    type Handle = *mut core::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(which: u32) -> Handle;
        fn GetConsoleMode(console: Handle, mode: *mut u32) -> i32;
        fn SetConsoleMode(console: Handle, mode: u32) -> i32;
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const ENABLE_LINE_INPUT: u32 = 0x2;
    const ENABLE_ECHO_INPUT: u32 = 0x4;
    /// No mode saved yet.
    const NONE: u32 = u32::MAX;

    static ORIGINAL: AtomicU32 = AtomicU32::new(NONE);

    pub(super) fn raw() -> bool {
        // SAFETY: plain console calls on this process's own input handle.
        unsafe {
            let input = GetStdHandle(STD_INPUT_HANDLE);
            let mut mode = 0;
            if GetConsoleMode(input, &mut mode) == 0 {
                return false;
            }
            ORIGINAL.store(mode, Ordering::SeqCst);
            // Ctrl+C still ends the run (processed input stays on); the mode is put back first.
            SetConsoleCtrlHandler(Some(on_ctrl), 1);
            SetConsoleMode(input, mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT)) != 0
        }
    }

    pub(super) fn restore() {
        let mode = ORIGINAL.swap(NONE, Ordering::SeqCst);
        if mode != NONE {
            // SAFETY: as above.
            unsafe {
                SetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), mode);
            }
        }
    }

    unsafe extern "system" fn on_ctrl(_event: u32) -> i32 {
        restore();
        // Not handled: the default handler ends the process as before.
        0
    }
}

#[cfg(not(windows))]
mod terminal {
    //! A Unix terminal, through `stty` on `/dev/tty`.

    use std::process::{Command, Stdio};
    use std::sync::Mutex;

    static ORIGINAL: Mutex<Option<String>> = Mutex::new(None);

    fn stty(args: &[&str]) -> Option<String> {
        let tty = std::fs::File::open("/dev/tty").ok()?;
        let output = Command::new("stty")
            .args(args)
            .stdin(Stdio::from(tty))
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub(super) fn raw() -> bool {
        let Some(saved) = stty(&["-g"]) else {
            return false;
        };
        if let Ok(mut original) = ORIGINAL.lock() {
            *original = Some(saved);
        }
        // `isig` stays on: Ctrl+C still ends the run, and the shell resets the terminal's
        // mode when it shows its next prompt.
        stty(&["-icanon", "-echo", "min", "1"]).is_some()
    }

    pub(super) fn restore() {
        let saved = ORIGINAL.lock().ok().and_then(|mut o| o.take());
        if let Some(saved) = saved {
            stty(&[&saved]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_editor_presses_any_key_by_its_letter() {
        let key = |line: &str| command_key(&crate::json::parse(line).unwrap());
        assert_eq!(
            key(r#"{"id":1,"method":"app.key","params":{"key":"p"}}"#),
            Some('p')
        );
        assert_eq!(
            key(r#"{"id":1,"method":"app.key","params":{"key":"S"}}"#),
            Some('S')
        );
        assert_eq!(
            key(r#"{"id":1,"method":"app.key","params":{"key":"x"}}"#),
            None
        );
        assert_eq!(
            key(r#"{"id":1,"method":"app.key","params":{"key":"pp"}}"#),
            None
        );
        assert_eq!(key(r#"{"id":1,"method":"app.detach"}"#), Some('d'));
        assert_eq!(
            key(r#"{"id":1,"method":"app.restart","params":{"fullRestart":true}}"#),
            Some('R')
        );
    }
}
