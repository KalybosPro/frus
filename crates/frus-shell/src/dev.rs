//! **The channel `frus run` listens on**, and what a restarted application is handed back.
//!
//! `frus run` restarts an application by building it again, stopping the one that runs and
//! starting the new one. For the new one to open **where the old one was** — the same page, the
//! same window in the same place — the tool has to know those, and only the application does. So
//! when `frus run` started it — `FRUS_DEV` naming a listening socket on a desktop, the launch
//! intent's `frus_dev` extra on Android — the application reports to it, a line each time one
//! of them changes:
//!
//! ```text
//! location /products/3?tab=reviews
//! window 120 80 1280 800
//! ```
//!
//! (the window's outer position and inner size, in physical pixels). Nothing is asked of the
//! application and nothing is waited for: a report that cannot be written closes the channel,
//! and the application carries on.
//!
//! **On Android the report goes through the log**, as `frus-dev location /products/3`, which
//! `frus run` reads already and hides from the developer. A socket would need the application
//! to declare the `INTERNET` permission, and running it under `frus run` must not ask for one.
//!
//! Started again, it is handed those back: `FRUS_RESTORE_LOCATION` and `FRUS_RESTORE_WINDOW` on
//! a desktop, and on Android a link the launch intent carries, which
//! [`crate::link::location_of_link`] reads like any other. The web needs none of this: a reload
//! keeps the address, and the address is the location.
//!
//! **Component states** that opt in (`State::save`) cross the restart too, as a snapshot of the
//! state store. On a desktop `frus run` asks for it just before stopping the application —
//! `save <file>` on the socket, answered `saved` once written — and the new build reads it from
//! `FRUS_RESTORE_STATES`. On Android, where nothing reaches the application, `frus run` sends
//! it to the background first: the shell writes the snapshot to the application's own storage
//! as it goes, and the next launch under `frus run` reads it back.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use winit::window::Window;

/// The environment variable a desktop application finds the channel's address in.
#[cfg(desktop)]
const DEV_ENV: &str = "FRUS_DEV";
/// The launch intent's extra an Android application finds it in.
#[cfg(android)]
pub(crate) const DEV_EXTRA: &str = "frus_dev";
/// Where a restarted desktop application opens.
#[cfg(desktop)]
const LOCATION_ENV: &str = "FRUS_RESTORE_LOCATION";
/// Where its window goes, and how big it is: `x,y,width,height`, physical pixels.
#[cfg(desktop)]
const WINDOW_ENV: &str = "FRUS_RESTORE_WINDOW";
/// The file a restarted desktop application's component states are read from.
#[cfg(desktop)]
const STATES_ENV: &str = "FRUS_RESTORE_STATES";
/// The file, in an Android application's own storage, its states are kept in across a restart.
#[cfg(android)]
const STATES_FILE: &str = "frus-run-states";

/// Where `frus run` asked the states to be written, not yet done.
static SAVE_TO: Mutex<Option<PathBuf>> = Mutex::new(None);
/// The window to wake when it asks: the request arrives on another thread.
static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);

/// A window's place and size: outer position, inner size, in physical pixels.
pub(crate) type Placement = [i32; 4];

/// What a log line carrying a report starts with.
pub(crate) const LOG_MARK: &str = "frus-dev ";

/// The open channel, and what it was last told.
pub(crate) struct DevChannel {
    sink: Sink,
    location: Option<String>,
    window: Option<Placement>,
}

/// Where the reports go.
enum Sink {
    /// A socket `frus run` listens on (desktop).
    #[cfg_attr(not(desktop), allow(dead_code))]
    Socket(TcpStream),
    /// The application's log, which `frus run` reads (Android).
    #[cfg_attr(not(android), allow(dead_code))]
    Log,
}

impl DevChannel {
    /// Connects to `address` — or does nothing, within a second, if nobody listens there.
    #[cfg_attr(not(desktop), allow(dead_code))]
    pub(crate) fn connect(address: &str) -> Option<Self> {
        let address: SocketAddr = address.trim().parse().ok()?;
        let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_secs(1)) else {
            log::warn!(
                "frus run's channel at {address} could not be reached: a restart will start over"
            );
            return None;
        };
        let _ = stream.set_nodelay(true);
        // What `frus run` asks: `save <file>`, before it stops the application.
        if let Ok(reading) = stream.try_clone() {
            std::thread::spawn(move || {
                for line in BufReader::new(reading).lines().map_while(Result::ok) {
                    if let Some(path) = line.strip_prefix("save ") {
                        if let Ok(mut slot) = SAVE_TO.lock() {
                            *slot = Some(PathBuf::from(path.trim()));
                        }
                        if let Ok(window) = WINDOW.lock() {
                            if let Some(window) = window.as_ref() {
                                window.request_redraw();
                            }
                        }
                    }
                }
            });
        }
        Some(Self::to(Sink::Socket(stream)))
    }

    /// Writes `snapshot` where `frus run` asked, if it asked, and says it is done.
    pub(crate) fn save_if_asked(&mut self, snapshot: impl FnOnce() -> String) {
        let Some(path) = SAVE_TO.lock().ok().and_then(|mut slot| slot.take()) else {
            return;
        };
        let written = std::fs::write(&path, snapshot()).is_ok();
        if let Sink::Socket(stream) = &mut self.sink {
            let _ = stream.write_all(if written { b"saved\n" } else { b"unsaved\n" });
        }
    }

    /// Reports through the log, which `frus run` reads.
    #[cfg_attr(not(android), allow(dead_code))]
    pub(crate) fn through_log() -> Self {
        Self::to(Sink::Log)
    }

    fn to(sink: Sink) -> Self {
        Self {
            sink,
            location: None,
            window: None,
        }
    }

    /// Tells the channel what changed since the last report. `false` once it can no longer be
    /// written to — `frus run` ended — and the channel should be dropped.
    pub(crate) fn report(&mut self, location: Option<String>, window: Option<Placement>) -> bool {
        let mut lines = String::new();
        if location.is_some() && location != self.location {
            lines.push_str(&format!(
                "location {}\n",
                location.as_deref().unwrap_or("/")
            ));
            self.location = location;
        }
        if window.is_some() && window != self.window {
            if let Some([x, y, w, h]) = window {
                lines.push_str(&format!("window {x} {y} {w} {h}\n"));
            }
            self.window = window;
        }
        if lines.is_empty() {
            return true;
        }
        match &mut self.sink {
            Sink::Socket(stream) => stream.write_all(lines.as_bytes()).is_ok(),
            Sink::Log => {
                for line in lines.lines() {
                    log::info!(target: "frus::dev", "{LOG_MARK}{line}");
                }
                true
            }
        }
    }
}

/// The window a request from `frus run` wakes.
#[cfg(desktop)]
pub(crate) fn set_window(window: Option<Arc<Window>>) {
    if let Ok(mut slot) = WINDOW.lock() {
        *slot = window;
    }
}

/// The component states a restart handed over, if it did; the file is consumed.
#[cfg(desktop)]
pub(crate) fn restored_states() -> Option<String> {
    let path = std::env::var_os(STATES_ENV)?;
    let states = std::fs::read_to_string(&path).ok();
    let _ = std::fs::remove_file(&path);
    states
}

/// The same on Android, from the application's own storage.
#[cfg(android)]
pub(crate) fn restored_states(storage: &std::path::Path) -> Option<String> {
    let path = storage.join(STATES_FILE);
    let states = std::fs::read_to_string(&path).ok();
    let _ = std::fs::remove_file(&path);
    states
}

/// Keeps the states in the application's own storage, as it goes to the background under
/// `frus run`: a reinstall keeps that storage, and the next launch reads them back.
#[cfg(android)]
pub(crate) fn keep_states(storage: &std::path::Path, snapshot: &str) {
    let _ = std::fs::write(storage.join(STATES_FILE), snapshot);
}

/// The channel's address, if `frus run` gave one.
#[cfg(desktop)]
pub(crate) fn address() -> Option<String> {
    std::env::var(DEV_ENV).ok().filter(|a| !a.is_empty())
}

/// The location a restarted desktop application opens at, as a link.
#[cfg(desktop)]
pub(crate) fn restore_link() -> Option<String> {
    let location = std::env::var(LOCATION_ENV).ok()?;
    Some(as_link(&location))
}

/// `location` written as a link [`crate::link::location_of_link`] reads back unchanged.
#[cfg_attr(not(desktop), allow(dead_code))]
pub(crate) fn as_link(location: &str) -> String {
    format!("frus:///{}", location.trim_start_matches('/'))
}

/// Where a restarted desktop application's window goes.
#[cfg(desktop)]
pub(crate) fn restore_window() -> Option<Placement> {
    parse_placement(&std::env::var(WINDOW_ENV).ok()?)
}

/// `x,y,width,height` → a placement, refusing a window with no area.
#[cfg_attr(not(desktop), allow(dead_code))]
pub(crate) fn parse_placement(text: &str) -> Option<Placement> {
    let numbers: Vec<i32> = text
        .split(',')
        .map(|n| n.trim().parse().ok())
        .collect::<Option<_>>()?;
    match numbers.as_slice() {
        &[x, y, w, h] if w > 0 && h > 0 => Some([x, y, w, h]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    use super::*;
    use crate::link::location_of_link;

    #[test]
    fn a_location_survives_the_trip_through_a_link() {
        for location in ["/", "/products/3", "/search?q=shoes&sort=price", "/a/b/c"] {
            assert_eq!(
                location_of_link(&as_link(location)).as_deref(),
                Some(location)
            );
        }
    }

    #[test]
    fn placements_are_read_strictly() {
        assert_eq!(
            parse_placement("120,80,1280,800"),
            Some([120, 80, 1280, 800])
        );
        assert_eq!(
            parse_placement("-8, -8, 800, 600"),
            Some([-8, -8, 800, 600])
        );
        assert_eq!(parse_placement("1,2,0,600"), None);
        assert_eq!(parse_placement("1,2,3"), None);
        assert_eq!(parse_placement("a,b,c,d"), None);
    }

    /// Only what changed is written, and nothing at all when nothing did.
    #[test]
    fn the_channel_reports_changes_only() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let mut channel = DevChannel::connect(&address).unwrap();
        let (peer, _) = listener.accept().unwrap();

        assert!(channel.report(Some("/".into()), Some([0, 0, 800, 600])));
        assert!(channel.report(Some("/".into()), Some([0, 0, 800, 600])));
        assert!(channel.report(Some("/products/3".into()), Some([0, 0, 800, 600])));
        assert!(channel.report(None, Some([10, 20, 800, 600])));
        drop(channel);

        let lines: Vec<String> = BufReader::new(peer).lines().map_while(Result::ok).collect();
        assert_eq!(
            lines,
            [
                "location /",
                "window 0 0 800 600",
                "location /products/3",
                "window 10 20 800 600"
            ]
        );
    }

    #[test]
    fn nobody_listening_is_no_channel() {
        let unused = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = unused.local_addr().unwrap().to_string();
        drop(unused);
        assert!(DevChannel::connect(&address).is_none());
        assert!(DevChannel::connect("not an address").is_none());
    }
}
