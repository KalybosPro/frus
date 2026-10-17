//! What the running application reports while `frus run` is attached: where it is, and where its
//! window sits. A restart hands both to the next build, so it opens where the last one was.
//!
//! On a desktop the application connects to the socket this opens, whose address it is given in
//! `FRUS_DEV`, and writes a line each time one of them changes (see
//! `crates/frus-shell/src/dev.rs`). On Android it writes the same lines into its log, marked
//! `frus-dev`, and the logcat reader hands them here — a socket would need the application to
//! declare the `INTERNET` permission. The web needs none of this: a page reloaded keeps its
//! address, which is its location.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What a log line carrying a report holds after its tag, before the report itself.
pub(crate) const LOG_MARK: &str = "frus-dev ";

/// The last report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Report {
    /// `/products/3?tab=reviews`.
    pub location: Option<String>,
    /// `x,y,width,height`: the window's outer position and inner size, physical pixels.
    pub window: Option<String>,
}

impl Report {
    /// One line of the application's, taken in.
    pub(crate) fn take(&mut self, line: &str) {
        if let Some(location) = line.strip_prefix("location ") {
            self.location = Some(location.trim().to_string());
        } else if let Some(window) = line.strip_prefix("window ") {
            let numbers: Vec<&str> = window.split_whitespace().collect();
            if numbers.len() == 4 && numbers.iter().all(|n| n.parse::<i32>().is_ok()) {
                self.window = Some(numbers.join(","));
            }
        }
    }
}

/// The socket the application reports to, and what it last said.
pub(crate) struct DevLink {
    pub port: u16,
    report: Arc<Mutex<Report>>,
    /// The application connected last, to ask it things.
    latest: Arc<Mutex<Option<TcpStream>>>,
    /// It answered `saved`.
    saved: Arc<AtomicBool>,
}

impl DevLink {
    /// Opens the socket on `localhost`; each connection's lines update the report, so a
    /// restarted application simply connects again.
    pub(crate) fn open() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("could not open the restart channel: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let report = Arc::new(Mutex::new(Report::default()));
        let latest = Arc::new(Mutex::new(None));
        let saved = Arc::new(AtomicBool::new(false));
        let (shared, newest, answered) = (report.clone(), latest.clone(), saved.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if let (Ok(mut slot), Ok(writer)) = (newest.lock(), stream.try_clone()) {
                    *slot = Some(writer);
                }
                let (shared, answered) = (shared.clone(), answered.clone());
                std::thread::spawn(move || {
                    for line in BufReader::new(stream).lines().map_while(Result::ok) {
                        if line == "saved" {
                            answered.store(true, Ordering::SeqCst);
                        } else if let Ok(mut report) = shared.lock() {
                            report.take(&line);
                        }
                    }
                });
            }
        });
        Ok(Self {
            port,
            report,
            latest,
            saved,
        })
    }

    /// `127.0.0.1:<port>`, what the application is told.
    pub(crate) fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    pub(crate) fn report(&self) -> Report {
        self.report.lock().map(|r| r.clone()).unwrap_or_default()
    }

    /// Asks the running application to write its component states to `file`, and waits — a few
    /// seconds at most — for it to say it did. `false`: it did not answer, and the restart
    /// goes on without them.
    pub(crate) fn save_states(&self, file: &Path) -> bool {
        self.saved.store(false, Ordering::SeqCst);
        let asked = self.latest.lock().ok().and_then(|mut slot| {
            let stream = slot.as_mut()?;
            stream
                .write_all(format!("save {}\n", file.display()).as_bytes())
                .ok()
        });
        if asked.is_none() {
            return false;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if self.saved.load(Ordering::SeqCst) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// A log line, taken in if it carries a report: `true` when it did, and is not the
    /// developer's to read.
    pub(crate) fn from_log(report: &Mutex<Report>, line: &str) -> bool {
        let Some(at) = line.find(LOG_MARK) else {
            return false;
        };
        if let Ok(mut report) = report.lock() {
            report.take(&line[at + LOG_MARK.len()..]);
        }
        true
    }

    /// The report, shared with the thread that reads a device's log.
    pub(crate) fn shared(&self) -> Arc<Mutex<Report>> {
        self.report.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpStream;

    use super::*;

    #[test]
    fn reports_are_read() {
        let mut report = Report::default();
        report.take("location /products/3?tab=reviews");
        report.take("window 120 -8 1280 800");
        report.take("window 1 2 three 4");
        report.take("something else");
        assert_eq!(report.location.as_deref(), Some("/products/3?tab=reviews"));
        assert_eq!(report.window.as_deref(), Some("120,-8,1280,800"));
    }

    #[test]
    fn reports_come_through_the_log_too() {
        let report = Mutex::new(Report::default());
        assert!(DevLink::from_log(
            &report,
            "I/navigation: frus::dev: frus-dev location /search?q=shoes"
        ));
        assert!(!DevLink::from_log(&report, "I/navigation: signed in"));
        assert_eq!(
            report.lock().unwrap().location.as_deref(),
            Some("/search?q=shoes")
        );
    }

    /// The request goes to the application, and its answer is waited for.
    #[test]
    fn states_are_asked_for_and_answered() {
        let link = DevLink::open().unwrap();
        let app = TcpStream::connect(link.address()).unwrap();
        let mut answer = app.try_clone().unwrap();
        std::thread::spawn(move || {
            let mut asked = String::new();
            BufReader::new(app).read_line(&mut asked).unwrap();
            assert_eq!(asked, "save states.txt\n");
            answer.write_all(b"saved\n").unwrap();
        });
        // The listener takes the connection on its own thread.
        let deadline = Instant::now() + Duration::from_secs(5);
        while link.latest.lock().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(link.save_states(Path::new("states.txt")));
    }

    #[test]
    fn the_application_reports_over_the_socket() {
        let link = DevLink::open().unwrap();
        let mut app = TcpStream::connect(link.address()).unwrap();
        app.write_all(b"location /search?q=shoes\nwindow 0 0 800 600\n")
            .unwrap();
        drop(app);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while link.report().window.is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(
            link.report(),
            Report {
                location: Some("/search?q=shoes".into()),
                window: Some("0,0,800,600".into()),
            }
        );
    }
}
