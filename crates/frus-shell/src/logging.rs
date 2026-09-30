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

/// The crate an application's type is defined in: where its own `log` calls come from.
pub(crate) fn app_crate<A>() -> &'static str {
    let name = std::any::type_name::<A>();
    name.split("::").next().unwrap_or(name)
}

/// The filter used when nothing else says: `info` and up, the application's own crate at
/// `debug` in a debug build, and the noisy crates at `warn`. In `RUST_LOG`'s syntax.
pub(crate) fn default_filter(app_crate: &str, debug_build: bool) -> String {
    let mut filter = String::from("info");
    for quiet in QUIET {
        filter.push_str(&format!(",{quiet}=warn"));
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

    #[test]
    fn the_app_crate_is_where_its_type_lives() {
        assert_eq!(app_crate::<MyApp>(), "frus_shell");
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
