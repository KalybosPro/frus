//! The web build's server, while `frus run` lasts: it serves `web/`, and it carries the page's
//! console back to the terminal.
//!
//! The page it serves is the application's own `index.html` with one script added at the top
//! of `<head>`: it copies every console message, and every uncaught error, to `/__frus/log`,
//! which this server prints. So the application's `log::info!` shows in the terminal on the web
//! as it does on the desktop and on Android. The same script asks `/__frus/build` once a
//! second which build is being served, and reloads the page when a restart has made a new one:
//! the address — the application's location — survives the reload. The file on disk is not
//! touched.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Where the page posts its console.
const LOG_PATH: &str = "/__frus/log";
/// Where the page asks which build is served.
const BUILD_PATH: &str = "/__frus/build";
/// Where the page asks what the keys of `frus run` asked of it since it last asked.
const COMMANDS_PATH: &str = "/__frus/commands";
/// Where the page sends its picture, as a `data:image/png;base64,…` URL (`s`, milestone 675).
const SCREENSHOT_PATH: &str = "/__frus/screenshot";

/// The script added to the page. Each message goes out as soon as it is written, in order.
const CONSOLE_BRIDGE: &str = r#"<script>(() => {
  const text = (value) => {
    if (value instanceof Error) return value.stack || String(value);
    if (typeof value === 'object' && value !== null) {
      try { return JSON.stringify(value); } catch (_) { return String(value); }
    }
    return String(value);
  };
  let queue = Promise.resolve();
  const send = (level, values) => {
    const body = values.map(text).join(' ');
    queue = queue.then(() => fetch('/__frus/log?level=' + level, { method: 'POST', body, keepalive: true }))
      .catch(() => {});
  };
  for (const level of ['log', 'info', 'warn', 'error', 'debug']) {
    const original = console[level].bind(console);
    console[level] = (...values) => { original(...values); send(level, values); };
  }
  addEventListener('error', (event) => send('error', [event.error || event.message]));
  addEventListener('unhandledrejection', (event) => send('error', ['Unhandled rejection:', event.reason]));
  window.__frusRun = true;
  let build = null;
  setInterval(() => fetch('/__frus/build', { cache: 'no-store' })
    .then((answer) => answer.text())
    .then((served) => {
      if (build === null) build = served;
      else if (served !== build) {
        // A restart afresh: the application keeps none of its states across this reload.
        if (served.endsWith(' fresh')) sessionStorage.setItem('frus-run-fresh', '1');
        location.reload();
      }
    })
    .catch(() => {}), 1000);
  // The page's picture, which the application reads from its canvas on `s`.
  window.__frusScreenshot = (data) =>
    fetch('/__frus/screenshot', { method: 'POST', body: data }).catch(() => {});
  // What the keys of `frus run` ask: the inspector, debug paint, a tree printed… The shell
  // defines `__frusCommand` once the application has started.
  setInterval(() => fetch('/__frus/commands', { cache: 'no-store' })
    .then((answer) => answer.text())
    .then((lines) => lines.split('\n').filter(Boolean)
      .forEach((command) => window.__frusCommand && window.__frusCommand(command)))
    .catch(() => {}), 300);
})();</script>"#;

/// A server running on a thread of its own until the program ends.
pub(crate) struct Server {
    pub url: String,
    build: Arc<AtomicU64>,
    fresh: Arc<AtomicBool>,
    commands: Arc<Mutex<Vec<String>>>,
    /// Where the next picture the page sends is written.
    screenshot_to: Arc<Mutex<Option<PathBuf>>>,
}

impl Server {
    /// A new build is in `web/`: the open pages reload within a second — keeping the states of
    /// the components that keep one, unless `fresh`.
    pub(crate) fn rebuilt(&self, fresh: bool) {
        self.fresh.store(fresh, Ordering::SeqCst);
        self.build.fetch_add(1, Ordering::SeqCst);
    }

    /// The next picture the page sends is written to `path`.
    pub(crate) fn expect_screenshot(&self, path: PathBuf) {
        if let Ok(mut slot) = self.screenshot_to.lock() {
            *slot = Some(path);
        }
    }

    /// A command for the page, which it picks up the next time it asks (within a moment).
    pub(crate) fn command(&self, command: &str) {
        if let Ok(mut queue) = self.commands.lock() {
            queue.push(command.to_string());
        }
    }
}

/// Serves `dir` on `localhost`, on `port` if it is free, on another otherwise.
///
/// `localhost` and not `127.0.0.1`: WebGPU is only offered to secure contexts, and a browser
/// counts `localhost` as one.
pub(crate) fn start(dir: PathBuf, port: u16) -> Result<Server, String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .or_else(|_| TcpListener::bind(("127.0.0.1", 0)))
        .map_err(|e| format!("could not open a port to serve the application on: {e}"))?;
    let address: SocketAddr = listener.local_addr().map_err(|e| e.to_string())?;
    let build = Arc::new(AtomicU64::new(0));
    let fresh = Arc::new(AtomicBool::new(false));
    let commands = Arc::new(Mutex::new(Vec::new()));
    let screenshot_to = Arc::new(Mutex::new(None));
    let (served, served_fresh, queued) = (build.clone(), fresh.clone(), commands.clone());
    let shots = screenshot_to.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().map_while(Result::ok) {
            let dir = dir.clone();
            let build = match served.load(Ordering::SeqCst) {
                n if served_fresh.load(Ordering::SeqCst) => format!("{n} fresh"),
                n => n.to_string(),
            };
            let queued = queued.clone();
            let shots = shots.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, &dir, &build, &queued, &shots);
            });
        }
    });
    Ok(Server {
        url: format!("http://localhost:{}/", address.port()),
        build,
        fresh,
        commands,
        screenshot_to,
    })
}

/// One request: a file, or a console message.
fn handle(
    mut stream: TcpStream,
    dir: &Path,
    build: &str,
    commands: &Mutex<Vec<String>>,
    screenshot_to: &Mutex<Option<PathBuf>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/"));

    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if method == "POST" && path == LOG_PATH {
        let mut body = vec![0; length.min(1 << 20)];
        reader.read_exact(&mut body)?;
        print_console(query_value(query, "level"), &String::from_utf8_lossy(&body));
        return respond(&mut stream, "204 No Content", "text/plain", b"");
    }
    if method == "POST" && path == SCREENSHOT_PATH {
        // A picture of a large window is a few megabytes as base64.
        let mut body = vec![0; length.min(64 << 20)];
        reader.read_exact(&mut body)?;
        let png = String::from_utf8_lossy(&body)
            .split_once("base64,")
            .and_then(|(_, data)| decode_base64(data.trim()));
        let target = screenshot_to.lock().ok().and_then(|mut slot| slot.take());
        if let (Some(png), Some(target)) = (png, target) {
            std::fs::write(&target, png)?;
        }
        return respond(&mut stream, "204 No Content", "text/plain", b"");
    }
    if path == BUILD_PATH {
        return respond(&mut stream, "200 OK", "text/plain", build.as_bytes());
    }
    if path == COMMANDS_PATH {
        let taken = commands
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default();
        return respond(
            &mut stream,
            "200 OK",
            "text/plain",
            taken.join("\n").as_bytes(),
        );
    }
    if method != "GET" && method != "HEAD" {
        return respond(&mut stream, "405 Method Not Allowed", "text/plain", b"");
    }
    match file_for(dir, path) {
        Some(file) => match std::fs::read(&file) {
            Ok(mut bytes) => {
                if file.file_name().is_some_and(|name| name == "index.html") {
                    bytes = with_bridge(&String::from_utf8_lossy(&bytes)).into_bytes();
                }
                let body: &[u8] = if method == "HEAD" { b"" } else { &bytes };
                respond(&mut stream, "200 OK", content_type(&file), body)
            }
            Err(_) => respond(&mut stream, "404 Not Found", "text/plain", b"not found"),
        },
        None => respond(&mut stream, "404 Not Found", "text/plain", b"not found"),
    }
}

/// Standard base64, padding allowed; `None` on anything else.
pub(crate) fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let bytes: Vec<u8> = text.bytes().filter(|b| *b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, b) in chunk.iter().enumerate() {
            n |= value(*b)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    // `no-store`: after a rebuild the browser must fetch the new code, never its cached copy.
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

/// The file a request path names under `dir`; `/` is `index.html`. Nothing outside `dir`.
pub(crate) fn file_for(dir: &Path, path: &str) -> Option<PathBuf> {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let relative = Path::new(path);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let file = dir.join(relative);
    file.is_file().then_some(file)
}

/// `index.html` with the console bridge at the top of its `<head>`, so it runs before the
/// application's module.
pub(crate) fn with_bridge(html: &str) -> String {
    match html.find("<head>") {
        Some(at) => {
            let at = at + "<head>".len();
            format!("{}\n    {CONSOLE_BRIDGE}{}", &html[..at], &html[at..])
        }
        None => format!("{CONSOLE_BRIDGE}\n{html}"),
    }
}

fn query_value<'a>(query: &'a str, key: &str) -> &'a str {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .unwrap_or("log")
}

/// A message from the page, printed the way the desktop's logger prints: errors and warnings
/// on stderr, the rest on stdout.
fn print_console(level: &str, text: &str) {
    for line in text.lines() {
        crate::report::app_log(line, matches!(level, "error" | "warn"));
    }
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "wasm" => "application/wasm",
        "css" => "text/css",
        "json" | "map" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bridge_runs_before_the_application() {
        let page = "<!doctype html>\n<html>\n  <head>\n    <title>x</title>\n  </head>\n  <body><script type=\"module\">import('./pkg/x.js')</script></body>\n</html>";
        let served = with_bridge(page);
        let bridge = served.find("/__frus/log").unwrap();
        assert!(bridge > served.find("<head>").unwrap());
        assert!(bridge < served.find("<title>").unwrap());
        assert!(bridge < served.find("./pkg/x.js").unwrap());
        assert!(with_bridge("<p>no head</p>").starts_with("<script>"));
    }

    #[test]
    fn nothing_outside_the_site_is_served() {
        let dir = std::env::temp_dir().join(format!("frus-serve-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("pkg")).unwrap();
        std::fs::write(dir.join("index.html"), "x").unwrap();
        std::fs::write(dir.join("pkg").join("app.js"), "x").unwrap();
        assert_eq!(file_for(&dir, "/"), Some(dir.join("index.html")));
        assert_eq!(
            file_for(&dir, "/pkg/app.js"),
            Some(dir.join("pkg").join("app.js"))
        );
        assert_eq!(file_for(&dir, "/missing.js"), None);
        assert_eq!(file_for(&dir, "/../secret"), None);
        assert_eq!(file_for(&dir, "/pkg/../../secret"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_level_comes_from_the_query() {
        assert_eq!(query_value("level=warn", "level"), "warn");
        assert_eq!(query_value("x=1&level=error", "level"), "error");
        assert_eq!(query_value("", "level"), "log");
    }

    /// A real round trip: the page is served with the bridge, and a console message is taken.
    #[test]
    fn it_serves_and_takes_messages() {
        let dir = std::env::temp_dir().join(format!("frus-serve-live-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.html"),
            "<html><head></head><body></body></html>",
        )
        .unwrap();
        let server = start(dir.clone(), 0).unwrap();
        let port = server.url.trim_end_matches('/').rsplit(':').next().unwrap();
        let ask = |request: &str| {
            let mut stream =
                TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut answer = String::new();
            stream.read_to_string(&mut answer).unwrap();
            answer
        };
        let page = ask("GET / HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
        assert!(page.contains("Cache-Control: no-store"), "{page}");
        assert!(page.contains("/__frus/log"), "{page}");
        let posted = ask("POST /__frus/log?level=info HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello");
        assert!(posted.starts_with("HTTP/1.1 204"), "{posted}");
        assert!(ask("GET /nope HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        // The build the page polls moves on when a restart has made a new one.
        assert!(ask("GET /__frus/build HTTP/1.1\r\n\r\n").ends_with("\r\n\r\n0"));
        server.rebuilt(false);
        assert!(ask("GET /__frus/build HTTP/1.1\r\n\r\n").ends_with("\r\n\r\n1"));
        // A restart afresh says so: the page drops its states.
        server.rebuilt(true);
        assert!(ask("GET /__frus/build HTTP/1.1\r\n\r\n").ends_with("\r\n\r\n2 fresh"));
        assert!(page.contains("__frusRun"), "{page}");
        // What the keys ask, handed to the page once, in order.
        assert!(ask("GET /__frus/commands HTTP/1.1\r\n\r\n").ends_with("\r\n\r\n"));
        server.command("outlines");
        server.command("platform");
        assert!(
            ask("GET /__frus/commands HTTP/1.1\r\n\r\n").ends_with("\r\n\r\noutlines\nplatform")
        );
        assert!(ask("GET /__frus/commands HTTP/1.1\r\n\r\n").ends_with("\r\n\r\n"));
        assert!(page.contains("__frusCommand"), "{page}");
        // The page's picture, written where the run asked.
        let shot = dir.join("shot.png");
        server.expect_screenshot(shot.clone());
        let body = "data:image/png;base64,iVBORw0KGgo=";
        let posted = ask(&format!(
            "POST /__frus/screenshot HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ));
        assert!(posted.starts_with("HTTP/1.1 204"), "{posted}");
        assert_eq!(std::fs::read(&shot).unwrap(), b"\x89PNG\r\n\x1a\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
