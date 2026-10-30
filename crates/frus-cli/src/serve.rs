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
/// Where the page asks for the latest hot-reload patch (`frus run --hot`, milestone 679).
const PATCH_PATH: &str = "/__frus/patch";

/// The script added to the page under `frus run --hot`: it asks for the latest patch until the
/// shell can take one (`window.__frusPatch`), then for each newer one. A page that loads asks
/// from the start, and gets the latest: patches are cumulative.
const PATCH_BRIDGE: &str = r#"<script>(() => {
  let after = 0, asking = false;
  setInterval(() => {
    if (asking || !window.__frusPatch) return;
    asking = true;
    fetch('/__frus/patch?after=' + after, { cache: 'no-store' })
      .then((answer) => answer.text())
      .then((text) => {
        if (!text) return;
        const at = text.indexOf('\n');
        after = Number(text.slice(0, at));
        window.__frusPatch(text.slice(at + 1));
      })
      .catch(() => {})
      .finally(() => { asking = false; });
  }, 300);
})();</script>"#;

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
    /// The latest hot-reload patch (`frus run --hot`).
    pub patches: PatchSlot,
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

/// What is served: `dir`, and under `frus run --hot` (milestone 679) the hot-reload engine's
/// build mounted in it, with the address of its patch socket given to the page.
#[derive(Clone, Debug, Default)]
pub(crate) struct Site {
    pub dir: PathBuf,
    /// `(prefix, folder)`: a request under `/<prefix>/` is a file of `folder`.
    pub mounts: Vec<(String, PathBuf)>,
    /// Under `frus run --hot`: the page is given each patch (see [`PatchSlot`]).
    pub hot: bool,
    /// The simulated device the page stands in for, under `frus run -d <device> --web`
    /// (milestone 724): told to the shell before the application loads.
    pub device: Option<String>,
}

/// The latest hot-reload patch, as the engine sent it, and how many there were: what a page
/// asks `/__frus/patch` for.
#[derive(Clone, Default)]
pub(crate) struct PatchSlot(Arc<Mutex<(u64, String)>>);

impl PatchSlot {
    /// A new patch, unless it is the latest again: the engine sends it to each client that
    /// connects, in a message of its own. A patch is told by its module (`"lib"`).
    pub(crate) fn push(&self, message: String) {
        if let Ok(mut slot) = self.0.lock() {
            if patch_module(&slot.1) != patch_module(&message) {
                slot.0 += 1;
                slot.1 = message;
            }
        }
    }

    /// `<number>\n<patch>` when there is one newer than `after`; nothing otherwise.
    pub(crate) fn after(&self, after: u64) -> String {
        match self.0.lock() {
            Ok(slot) if slot.0 > after => format!("{}\n{}", slot.0, slot.1),
            _ => String::new(),
        }
    }
}

/// Serves `dir` on `localhost`, on `port` if it is free, on another otherwise. The runs serve
/// a [`Site`], which also says whether the page is hot and what device it stands in for.
#[cfg(test)]
pub(crate) fn start(dir: PathBuf, port: u16) -> Result<Server, String> {
    start_site(
        Site {
            dir,
            ..Site::default()
        },
        port,
    )
}

/// Serves `site` on `localhost`, on `port` if it is free, on another otherwise.
///
/// `localhost` and not `127.0.0.1`: WebGPU is only offered to secure contexts, and a browser
/// counts `localhost` as one.
pub(crate) fn start_site(site: Site, port: u16) -> Result<Server, String> {
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
    let patches = PatchSlot::default();
    let patch_slot = patches.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().map_while(Result::ok) {
            let site = site.clone();
            let build = match served.load(Ordering::SeqCst) {
                n if served_fresh.load(Ordering::SeqCst) => format!("{n} fresh"),
                n => n.to_string(),
            };
            let queued = queued.clone();
            let shots = shots.clone();
            let patch_slot = patch_slot.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, &site, &build, &queued, &shots, &patch_slot);
            });
        }
    });
    Ok(Server {
        url: format!("http://localhost:{}/", address.port()),
        build,
        fresh,
        commands,
        screenshot_to,
        patches,
    })
}

/// One request: a file, or a console message.
fn handle(
    mut stream: TcpStream,
    site: &Site,
    build: &str,
    commands: &Mutex<Vec<String>>,
    screenshot_to: &Mutex<Option<PathBuf>>,
    patches: &PatchSlot,
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
    if path == PATCH_PATH {
        let after = query_value(query, "after").parse().unwrap_or(0);
        return respond(
            &mut stream,
            "200 OK",
            "text/plain",
            patches.after(after).as_bytes(),
        );
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
    match site_file(site, path) {
        Some(file) => match std::fs::read(&file) {
            Ok(mut bytes) => {
                if file.file_name().is_some_and(|name| name == "index.html") {
                    let mut page = with_bridge(&String::from_utf8_lossy(&bytes));
                    if let Some(device) = &site.device {
                        page = with_device(&page, device);
                    }
                    if site.hot {
                        page = with_patches(&page);
                    }
                    bytes = page.into_bytes();
                } else if site.hot && file.extension().is_some_and(|e| e == "js") {
                    if let Some(glue) = loaded_once(&String::from_utf8_lossy(&bytes)) {
                        bytes = glue.into_bytes();
                    }
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

/// The file a request path names in `site`: under a mount's folder when it starts with the
/// mount's prefix, under its `dir` otherwise.
pub(crate) fn site_file(site: &Site, path: &str) -> Option<PathBuf> {
    let trimmed = path.trim_start_matches('/');
    for (prefix, folder) in &site.mounts {
        if let Some(rest) = trimmed
            .strip_prefix(prefix.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
        {
            return file_for(folder, rest).filter(|_| !rest.is_empty());
        }
    }
    file_for(&site.dir, path)
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

/// The page told which simulated device it stands in for, at the top of its `<head>`, before
/// the application's module asks. A property of `window`, not the address: on the web the
/// address is the router's.
///
/// `device` is a built-in device's id or a project's own device's whole description (milestone
/// 727), written by its developer, names and all: it goes into the script as a JavaScript
/// string, escaped, and nothing in it can end the string or the script.
pub(crate) fn with_device(page: &str, device: &str) -> String {
    let mut quoted = String::with_capacity(device.len());
    for c in device.chars() {
        match c {
            '\\' => quoted.push_str("\\\\"),
            '\'' => quoted.push_str("\\'"),
            '<' => quoted.push_str("\\x3c"),
            '\n' | '\r' | '\u{2028}' | '\u{2029}' => quoted.push(' '),
            c => quoted.push(c),
        }
    }
    let script = format!("<script>window.__frusDevice = '{quoted}';</script>");
    match page.find("<head>") {
        Some(at) => {
            let at = at + "<head>".len();
            format!("{}\n    {script}{}", &page[..at], &page[at..])
        }
        None => format!("{script}\n{page}"),
    }
}

/// The hot-reload engine's module glue loads the module itself, as it is imported; the page
/// then calls its `default()` to load it, which would start a second instance of the
/// application beside the first (milestone 679). Rewritten so that `default()` waits for the
/// load already started. `None` when the glue does not load itself.
pub(crate) fn loaded_once(glue: &str) -> Option<String> {
    const SELF_LOAD: &str = "__wbg_init({module_or_path:";
    const EXPORT: &str = "export { initSync, __wbg_init as default };";
    let at = glue.rfind(SELF_LOAD)?;
    if !glue.contains(EXPORT) {
        return None;
    }
    let glue = format!("{}const __frusLoaded = {}", &glue[..at], &glue[at..]);
    Some(glue.replacen(
        EXPORT,
        "export { initSync };\nexport default () => __frusLoaded;",
        1,
    ))
}

/// The module a patch message names, without its leading `/` (`wasm\\lib-patch-1.wasm`).
fn patch_module(message: &str) -> Option<&str> {
    let at = message.find("\"lib\":\"")? + "\"lib\":\"".len();
    let rest = &message[at..];
    Some(rest[..rest.find('"')?].trim_start_matches('/'))
}

/// The page made to ask for each hot-reload patch, at the top of its `<head>`.
pub(crate) fn with_patches(page: &str) -> String {
    match page.find("<head>") {
        Some(at) => {
            let at = at + "<head>".len();
            format!("{}\n    {PATCH_BRIDGE}{}", &page[..at], &page[at..])
        }
        None => format!("{PATCH_BRIDGE}\n{page}"),
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
    fn a_hot_build_is_served_from_its_own_folder() {
        let root = std::env::temp_dir().join(format!("frus-serve-hot-{}", std::process::id()));
        let (web, built) = (root.join("web"), root.join("dx").join("wasm"));
        std::fs::create_dir_all(&web).unwrap();
        std::fs::create_dir_all(&built).unwrap();
        std::fs::write(web.join("index.html"), "page").unwrap();
        std::fs::write(built.join("app.js"), "glue").unwrap();
        let site = Site {
            dir: web.clone(),
            mounts: vec![
                ("pkg".into(), built.clone()),
                ("wasm".into(), built.clone()),
            ],
            hot: true,
            device: None,
        };
        assert_eq!(site_file(&site, "/"), Some(web.join("index.html")));
        assert_eq!(site_file(&site, "/pkg/app.js"), Some(built.join("app.js")));
        assert_eq!(site_file(&site, "/wasm/app.js"), Some(built.join("app.js")));
        assert_eq!(site_file(&site, "/pkg/../index.html"), None);
        assert_eq!(site_file(&site, "/pkg/"), None);
        let page = with_patches("<html><head><title>x</title>");
        assert!(page.starts_with("<html><head>\n    <script>"), "{page}");
        assert!(page.contains("/__frus/patch?after="), "{page}");
        assert!(page.ends_with("</script><title>x</title>"), "{page}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **The page is told its simulated device before the application loads**, as a property
    /// of `window` and not the address, which is the router's; a name that is not a device id
    /// is not written into a script (milestone 724).
    #[test]
    fn a_page_is_told_its_simulated_device() {
        let page = with_device("<html><head><title>x</title>", "iphone-15");
        assert_eq!(
            page,
            "<html><head>\n    <script>window.__frusDevice = 'iphone-15';</script><title>x</title>"
        );
        assert!(with_device("<p>no head</p>", "ipad").starts_with("<script>"));
        // A project's own device, described in full: its text stays inside the string.
        let own = with_device("<head>", "id=x;name=Bob's </script> \\ phone");
        assert!(
            own.contains(r"window.__frusDevice = 'id=x;name=Bob\'s \x3c/script> \\ phone';"),
            "{own}"
        );
        assert_eq!(own.matches("</script>").count(), 1, "{own}");
    }

    #[test]
    fn a_page_gets_the_latest_patch_once() {
        let slot = PatchSlot::default();
        assert_eq!(slot.after(0), "");
        let one = r#"{"HotReload":{"ms_elapsed":4000,"jump_table":{"lib":"/wasm\\a.wasm"}}}"#;
        // The engine sends the latest again to each client that connects: not a new patch.
        let again = r#"{"HotReload":{"ms_elapsed":0,"jump_table":{"lib":"/wasm\\a.wasm"}}}"#;
        let two = r#"{"HotReload":{"ms_elapsed":3000,"jump_table":{"lib":"/wasm\\b.wasm"}}}"#;
        slot.push(one.into());
        slot.push(again.into());
        assert_eq!(slot.after(0), format!("1\n{one}"));
        assert_eq!(slot.after(1), "");
        slot.push(two.into());
        // A page that has the first gets the second; a page that loads now, the latest.
        assert_eq!(slot.after(1), format!("2\n{two}"));
        assert_eq!(slot.after(0), format!("2\n{two}"));
    }

    #[test]
    fn the_engine_glue_loads_the_module_once() {
        let glue = "async function __wbg_init(module_or_path) {}\n\
                    export { initSync, __wbg_init as default };\n\
                    __wbg_init({module_or_path: \"/./wasm/app_bg.wasm\"}).then((wasm) => {});\n";
        let served = loaded_once(glue).unwrap();
        assert!(
            served.contains("const __frusLoaded = __wbg_init({module_or_path:"),
            "{served}"
        );
        assert!(
            served.contains("export default () => __frusLoaded;"),
            "{served}"
        );
        assert!(!served.contains("__wbg_init as default"), "{served}");
        // wasm-bindgen's own glue, which loads nothing by itself, is served as it is.
        assert_eq!(
            loaded_once("export { initSync, __wbg_init as default };"),
            None
        );
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
