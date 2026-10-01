//! Where an application's [`log`] output goes: **the console, on every platform, with nothing
//! to set up** (milestone 604).
//!
//! - **Desktop**: the terminal the application was started from (standard error).
//! - **Android**: logcat, under the application's crate name as the tag
//!   (`adb logcat -s my_app`).
//! - **Web**: the browser's console, each level with its own method, so a warning is yellow
//!   and an error red in the developer tools.
//!
//! By default, everything from `info` up is shown, and the application's **own** crate from
//! `debug` up in a debug build. The graphics and windowing crates, which talk a great deal at
//! `info`, are held to `warn`. `RUST_LOG`, where a platform has one (desktop), still decides
//! over all of this: `RUST_LOG=trace` shows everything.
//!
//! The application's crate is the one [`main!`](crate::main) is invoked in, which it names
//! before running the application.
//!
//! A **panic** is logged as an error before the usual report, so it reaches the same console:
//! on Android, standard error goes nowhere, and a crash used to leave nothing to read.

/// The crates that fill a console at `info` with what an application developer did not ask
/// about: the GPU stack and the window layer.
const QUIET: &[&str] = &[
    "wgpu",
    "wgpu_core",
    "wgpu_hal",
    "naga",
    "winit",
    "calloop",
    "sctk",
    "smithay_client_toolkit",
    "cosmic_text",
    "fontdb",
    "accesskit_windows",
    "accesskit_unix",
];

/// What says nothing a developer can act on, even at `warn`: held to `error`. The GPU layer
/// reports at every launch, in ten lines, that a phone's GPU is not a full desktop one.
const SILENT: &[&str] = &["wgpu_core::instance"];

/// The crates that are the framework's, or the language's own: never the application's.
const NOT_THE_APP: &[&str] = &[
    "frus",
    "frus_shell",
    "frus_widgets",
    "frus_core",
    "frus_layout",
    "frus_text",
    "frus_gpu",
    "frus_test",
    "core",
    "alloc",
    "std",
];

/// The application's crate, as [`main!`](crate::main) names it from inside the application.
static NAMED: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// Names the application's crate from a `module_path!()` taken inside it: what
/// [`main!`](crate::main) does before it runs the application.
pub fn name_app(module_path: &'static str) {
    let _ = NAMED.set(module_path.split("::").next().unwrap_or(module_path));
}

/// The application's crate: where its own `log` calls come from, and logcat's tag.
///
/// What [`main!`](crate::main) named, when it ran the application. Otherwise read off the
/// application's type: the first crate in its name that is not the framework's. The type the
/// shell is handed is often one of the framework's own, wrapping the application's, so the
/// first crate of the name alone said `frus_shell` for every application (milestone 604,
/// seen in logcat on a phone).
pub(crate) fn app_crate<A>() -> &'static str {
    NAMED
        .get()
        .copied()
        .unwrap_or_else(|| crate_of(std::any::type_name::<A>()))
}

/// The first crate named in `type_name` that is not the framework's or the standard
/// library's; the first crate named when every one is.
fn crate_of(type_name: &'static str) -> &'static str {
    let crates = type_name
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .filter(|path| path.contains("::"))
        .filter_map(|path| path.split("::").next())
        .filter(|name| !name.is_empty());
    let mut first = None;
    for name in crates {
        first.get_or_insert(name);
        if !NOT_THE_APP.contains(&name) {
            return name;
        }
    }
    first.unwrap_or(type_name)
}

/// The filter used when nothing else says: `info` and up, the application's own crate at
/// `debug` in a debug build, and the noisy crates at `warn`. In `RUST_LOG`'s syntax.
pub(crate) fn default_filter(app_crate: &str, debug_build: bool) -> String {
    let mut filter = String::from("info");
    for quiet in QUIET {
        filter.push_str(&format!(",{quiet}=warn"));
    }
    for silent in SILENT {
        filter.push_str(&format!(",{silent}=error"));
    }
    if debug_build && !app_crate.is_empty() {
        filter.push_str(&format!(",{app_crate}=debug"));
    }
    filter
}

/// A filter in `RUST_LOG`'s syntax, `level,crate=level,…`, for the platforms whose logger
/// has none of its own (the web). The most specific prefix of a record's target wins.
#[cfg_attr(not(any(web, test)), allow(dead_code))]
pub(crate) struct Filter {
    default: log::LevelFilter,
    rules: Vec<(String, log::LevelFilter)>,
}

#[cfg_attr(not(any(web, test)), allow(dead_code))]
impl Filter {
    /// Reads `spec`. A part it does not understand is skipped rather than refused: a filter
    /// that stopped the application from starting would be a worse failure than a missing
    /// line.
    pub(crate) fn parse(spec: &str) -> Self {
        let mut filter = Filter {
            default: log::LevelFilter::Error,
            rules: Vec::new(),
        };
        for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match part.split_once('=') {
                Some((target, level)) => {
                    if let Ok(level) = level.trim().parse() {
                        filter.rules.push((target.trim().to_string(), level));
                    }
                }
                None => {
                    if let Ok(level) = part.parse() {
                        filter.default = level;
                    }
                }
            }
        }
        filter
    }

    /// The most detailed level any rule lets through: what the `log` crate is told to stop
    /// at before a record is even built.
    pub(crate) fn max(&self) -> log::LevelFilter {
        self.rules
            .iter()
            .map(|(_, level)| *level)
            .fold(self.default, std::cmp::max)
    }

    /// Whether a record from `target` at `level` is shown.
    pub(crate) fn enabled(&self, target: &str, level: log::Level) -> bool {
        let rule = self
            .rules
            .iter()
            .filter(|(prefix, _)| {
                target == prefix
                    || target
                        .strip_prefix(prefix.as_str())
                        .is_some_and(|rest| rest.starts_with("::"))
            })
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(_, level)| *level)
            .unwrap_or(self.default);
        level <= rule
    }
}

/// Logs a panic as an error, then lets the report that was going to happen happen: the same
/// crash reaches the console the rest of the application's messages go to.
///
/// The web has its own: `console_error_panic_hook`, set up beside the logger.
#[cfg(android)]
fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info
            .location()
            .map(|at| format!(" at {}:{}", at.file(), at.line()))
            .unwrap_or_default();
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic".to_string());
        log::error!(target: "panic", "{what}{place}");
        previous(info);
    }));
}

/// Desktop: the terminal, through `env_logger`, with `RUST_LOG` over the default.
#[cfg(desktop)]
pub(crate) fn init<A>() {
    let mut builder = env_logger::Builder::new();
    builder.parse_filters(&default_filter(app_crate::<A>(), cfg!(debug_assertions)));
    if let Ok(env) = std::env::var("RUST_LOG") {
        builder.parse_filters(&env);
    }
    // A second call — a test harness that set one up first — is not an error.
    let _ = builder.try_init();
}

/// Android: logcat, tagged with the application's crate.
#[cfg(android)]
pub(crate) fn init<A>() {
    let app = app_crate::<A>();
    let filter = default_filter(app, cfg!(debug_assertions));
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag(app)
            .with_filter(android_logger::FilterBuilder::new().parse(&filter).build()),
    );
    log_panics();
}

/// The web: the browser's console, a method per level.
#[cfg(web)]
pub(crate) fn init<A>() {
    struct Console(Filter);
    impl log::Log for Console {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            self.0.enabled(metadata.target(), metadata.level())
        }
        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let line = wasm_bindgen::JsValue::from_str(&format!(
                "[{} {}] {}",
                record.level(),
                record.target(),
                record.args()
            ));
            match record.level() {
                log::Level::Error => web_sys::console::error_1(&line),
                log::Level::Warn => web_sys::console::warn_1(&line),
                log::Level::Info => web_sys::console::info_1(&line),
                log::Level::Debug | log::Level::Trace => web_sys::console::debug_1(&line),
            }
        }
        fn flush(&self) {}
    }
    let filter = Filter::parse(&default_filter(app_crate::<A>(), cfg!(debug_assertions)));
    let max = filter.max();
    // Once per page, for as long as the page lives: a leaked box is the logger `log` asks
    // for, without its `std` feature.
    if log::set_logger(Box::leak(Box::new(Console(filter)))).is_ok() {
        log::set_max_level(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MyApp;

    /// **The application's crate, through the framework's wrappers**: the type the shell is
    /// handed is often the framework's, with the application's inside it.
    #[test]
    fn the_app_crate_is_the_first_that_is_not_the_frameworks() {
        assert_eq!(
            crate_of("frus_shell::component::Host<my_app::screens::Root>"),
            "my_app"
        );
        assert_eq!(crate_of("my_app::Model"), "my_app");
        assert_eq!(
            crate_of("alloc::boxed::Box<dyn frus_widgets::Widget<my_app::Msg>>"),
            "my_app"
        );
        // A type of the framework's alone: the framework's, rather than nothing.
        assert_eq!(crate_of("frus_shell::logging::tests::MyApp"), "frus_shell");
        assert_eq!(app_crate::<MyApp>(), "frus_shell");
    }

    /// **The GPU layer's launch report is not shown**, but its errors are.
    #[test]
    fn the_gpu_layers_launch_report_is_held_back() {
        let filter = Filter::parse(&default_filter("my_app", false));
        assert!(!filter.enabled("wgpu_core::instance", log::Level::Warn));
        assert!(filter.enabled("wgpu_core::instance", log::Level::Error));
        assert!(filter.enabled("wgpu_core::device", log::Level::Warn));
    }

    /// **By default**: info and up everywhere, the application at debug in a debug build
    /// only, the noisy crates at warn.
    #[test]
    fn the_default_shows_info_and_the_apps_debug() {
        let debug = Filter::parse(&default_filter("my_app", true));
        assert!(debug.enabled("my_app", log::Level::Debug));
        assert!(debug.enabled("my_app::screens", log::Level::Debug));
        assert!(!debug.enabled("my_app", log::Level::Trace));
        assert!(debug.enabled("frus_widgets", log::Level::Info));
        assert!(!debug.enabled("frus_widgets", log::Level::Debug));
        assert!(!debug.enabled("wgpu_core::device", log::Level::Info));
        assert!(debug.enabled("wgpu_core::device", log::Level::Warn));
        // A crate whose name starts like the application's is not the application.
        assert!(!debug.enabled("my_application", log::Level::Debug));

        let release = Filter::parse(&default_filter("my_app", false));
        assert!(!release.enabled("my_app", log::Level::Debug));
        assert!(release.enabled("my_app", log::Level::Info));
        assert_eq!(debug.max(), log::LevelFilter::Debug);
    }

    /// **A filter it cannot read in part** keeps the parts it can.
    #[test]
    fn a_broken_part_is_skipped() {
        let filter = Filter::parse("warn,,my_app=loud,other=trace");
        assert!(filter.enabled("anything", log::Level::Warn));
        assert!(!filter.enabled("anything", log::Level::Info));
        assert!(!filter.enabled("my_app", log::Level::Info));
        assert!(filter.enabled("other", log::Level::Trace));
    }
}
