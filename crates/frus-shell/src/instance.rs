//! **One instance** of an application, and the link a second one carries to the first.
//!
//! A desktop registers a link scheme by starting the program with the link as an argument —
//! so opening `myapp://orders/42` while `myapp` is already running starts a *second* process.
//! Two windows of an application that means to be one is not what anyone wanted, so the second
//! asks the first to take the link, and leaves.
//!
//! It is opt-in ([`Application::instance_id`](crate::Application::instance_id)), because an
//! application that opens a window per document is right to have many instances.
//!
//! **The mechanism is a socket on the loopback interface and a lock file** in the temporary
//! directory that holds its port and a random token. The first instance listens; a second
//! reads the file and connects. It needs nothing but `std`, and behaves the same on the three
//! desktops, which a named pipe, a Unix socket and a mailbox would not.
//!
//! **What stops another process from sending links.** A local socket is open to anything on
//! the machine — including a web page that writes to `127.0.0.1:<port>` — so the listener
//! answers only a first line of exactly `frus-link <token> <link>`, where the token is in a
//! file that only the user can read on Unix. A request from a browser starts with
//! `POST / HTTP/1.1`, carries no token, and is closed without a word. Someone who can read the
//! user's temporary files can already do more than open a link.
//!
//! A second instance **leaves only if it was answered** with `ok`: a lock file left by a
//! crashed instance names a port that may be free or that some other program has, and either
//! way the new process becomes the first.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use winit::window::Window;

/// Links that arrived from a second instance and have not been read yet.
static LINKS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// The window to bring forward, and to ask for a frame in which the links are read.
static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);

/// How long a second instance waits for the first to answer.
const ANSWER: Duration = Duration::from_millis(600);

/// The longest first line taken: a link and a token fit in a small fraction of it.
const LINE: u64 = 4096;

/// What a starting instance is.
pub(crate) enum Claim {
    /// The first: it is to serve what the others send.
    First(Server),
    /// Not the first: the link went to the one that is, and this process should end.
    Handed,
}

/// A listening socket and the token a sender has to know.
pub(crate) struct Server {
    listener: TcpListener,
    token: String,
}

/// Claims the instance `id` for this process: hands `link` to the one that has it, if there
/// is one, and otherwise becomes it.
pub(crate) fn claim(id: &str, link: Option<&str>) -> Claim {
    claim_in(&std::env::temp_dir(), id, link)
}

fn claim_in(dir: &Path, id: &str, link: Option<&str>) -> Claim {
    let file = dir.join(format!("frus-instance-{}.lock", sanitised(id)));
    if let Some((port, token)) = read_lock(&file) {
        if hand(port, &token, link) {
            return Claim::Handed;
        }
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback socket");
    let token = new_token();
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    if let Err(err) = write_lock(&file, port, &token) {
        // No lock, no handoff: this instance runs alone, as it would without the feature.
        log::warn!("single instance: the lock file could not be written ({err})");
    }
    Claim::First(Server { listener, token })
}

/// `id` as something safe to put in a file name.
fn sanitised(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// A token nobody can guess from outside: two keys the operating system chose for this
/// process, hashed over the time and the process.
fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut token = String::new();
    for _ in 0..2 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u32(std::process::id());
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token
}

fn read_lock(file: &Path) -> Option<(u16, String)> {
    let content = std::fs::read_to_string(file).ok()?;
    let mut lines = content.lines();
    let port = lines.next()?.trim().parse().ok()?;
    let token = lines.next()?.trim().to_string();
    (!token.is_empty()).then_some((port, token))
}

fn write_lock(file: &Path, port: u16, token: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    // Readable by the user alone: the token is what lets a process send links.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut lock = options.open(file)?;
    write!(lock, "{port}\n{token}\n")
}

/// Sends `link` to the instance listening on `port`. Whether it answered `ok`.
fn hand(port: u16, token: &str, link: Option<&str>) -> bool {
    let address = std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, ANSWER) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(ANSWER));
    let _ = stream.set_write_timeout(Some(ANSWER));
    if writeln!(stream, "frus-link {token} {}", link.unwrap_or("-")).is_err() {
        return false;
    }
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).is_ok() && answer.trim() == "ok"
}

impl Server {
    /// Serves the instances that come after this one, on a thread of its own: each link they
    /// carry is given to `on_link`.
    pub(crate) fn serve(self, on_link: impl Fn(Option<String>) + Send + 'static) {
        std::thread::Builder::new()
            .name("frus-instance".into())
            .spawn(move || {
                for stream in self.listener.incoming().flatten() {
                    if let Some(link) = self.accept(stream) {
                        on_link(link);
                    }
                }
            })
            .ok();
    }

    /// Reads one request. `Some` of what it carried — a link, or nothing but a wish to bring the
    /// window forward — if it is one; `None`, with nothing said, if it is not.
    fn accept(&self, mut stream: TcpStream) -> Option<Option<String>> {
        let _ = stream.set_read_timeout(Some(ANSWER));
        let mut line = String::new();
        BufReader::new((&stream).take(LINE))
            .read_line(&mut line)
            .ok()?;
        let mut parts = line.trim_end().splitn(3, ' ');
        if parts.next()? != "frus-link" || parts.next()? != self.token {
            return None;
        }
        let link = parts.next()?;
        let _ = writeln!(stream, "ok");
        Some((link != "-").then(|| link.to_string()))
    }
}

/// Starts serving, with what arrives queued for [`take_links`] and the window brought to the
/// front.
pub(crate) fn serve_into_queue(server: Server) {
    server.serve(|link| {
        if let Some(link) = link {
            LINKS.lock().unwrap().push_back(link);
        }
        if let Some(window) = WINDOW.lock().unwrap().as_ref() {
            window.focus_window();
            window.request_redraw();
        }
    });
}

/// The window the queue's arrivals are announced to.
pub(crate) fn set_window(window: Arc<Window>) {
    *WINDOW.lock().unwrap() = Some(window);
}

/// The links that arrived since the last call, oldest first.
pub(crate) fn take_links() -> Vec<String> {
    LINKS.lock().unwrap().drain(..).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("frus-instance-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn first(dir: &Path, id: &str) -> mpsc::Receiver<Option<String>> {
        let Claim::First(server) = claim_in(dir, id, None) else {
            panic!("the first process to claim an instance is it");
        };
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        server.serve(move |link| {
            let _ = tx.lock().unwrap().send(link);
        });
        rx
    }

    #[test]
    fn the_second_instance_hands_its_link_to_the_first_and_leaves() {
        let dir = scratch("handoff");
        let arrived = first(&dir, "app");
        assert!(matches!(
            claim_in(&dir, "app", Some("myapp://orders/42")),
            Claim::Handed
        ));
        assert_eq!(
            arrived.recv_timeout(Duration::from_secs(2)).unwrap(),
            Some("myapp://orders/42".to_string())
        );
    }

    #[test]
    fn a_second_instance_with_no_link_only_asks_for_the_window() {
        let dir = scratch("nolink");
        let arrived = first(&dir, "app");
        assert!(matches!(claim_in(&dir, "app", None), Claim::Handed));
        assert_eq!(arrived.recv_timeout(Duration::from_secs(2)).unwrap(), None);
    }

    #[test]
    fn a_stale_lock_names_nobody_so_the_new_process_is_the_first() {
        let dir = scratch("stale");
        // A port nothing listens on, as a crashed instance leaves behind.
        let free = {
            let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            probe.local_addr().unwrap().port()
        };
        write_lock(&dir.join("frus-instance-app.lock"), free, "deadbeef").unwrap();
        assert!(matches!(claim_in(&dir, "app", None), Claim::First(_)));
    }

    #[test]
    fn instances_of_different_applications_do_not_meet() {
        let dir = scratch("apps");
        let _one = first(&dir, "one");
        assert!(matches!(claim_in(&dir, "two", None), Claim::First(_)));
    }

    #[test]
    fn a_request_without_the_token_is_closed_without_a_word() {
        let dir = scratch("token");
        let arrived = first(&dir, "app");
        let (port, _) = read_lock(&dir.join("frus-instance-app.lock")).unwrap();
        for request in [
            "frus-link wrong myapp://x\n".to_string(),
            "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\nfrus-link x myapp://x\n".to_string(),
            "\n".to_string(),
        ] {
            let mut stream =
                TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("the listener is there");
            stream
                .set_read_timeout(Some(Duration::from_millis(800)))
                .unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut answer = String::new();
            let _ = BufReader::new(stream).read_line(&mut answer);
            assert_eq!(answer, "", "nothing is said to {request:?}");
        }
        assert!(arrived.recv_timeout(Duration::from_millis(300)).is_err());
    }

    #[test]
    fn a_program_that_is_not_an_instance_does_not_make_the_second_one_leave() {
        let dir = scratch("foreign");
        // Something listens on the port in the lock, and says nothing that is `ok`.
        let foreign = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = foreign.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in foreign.incoming().flatten() {
                let mut stream = stream;
                let _ = writeln!(stream, "HTTP/1.1 400 Bad Request");
            }
        });
        write_lock(&dir.join("frus-instance-app.lock"), port, "token").unwrap();
        assert!(matches!(
            claim_in(&dir, "app", Some("myapp://x")),
            Claim::First(_)
        ));
    }

    #[test]
    fn a_file_name_is_made_of_safe_characters() {
        assert_eq!(sanitised("com.example/my app"), "com.example_my_app");
    }
}
