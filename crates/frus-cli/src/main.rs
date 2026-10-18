//! `frus`, the command-line tool:
//!
//! ```text
//! frus create my_app        # a new application
//! frus devices              # what it can run on, now
//! frus run -d <device>      # build it, start it there, show its log
//! frus build apk|web|desktop
//! ```
//!
//! It drives the tools an application already uses — cargo, cargo-apk, adb, wasm-bindgen — and
//! adds no build system of its own: everything it does can be done by hand.

mod build;
mod cargo;
mod create;
mod daemon;
mod devices;
mod devlink;
mod hot;
mod json;
mod keys;
mod project;
mod report;
mod run;
mod serve;
mod setup;
mod template;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use create::{Created, Git, Options, DEFAULT_ORG};

const USAGE: &str = "\
frus — the frus command-line tool

Usage:
  frus create <directory> [options]   start a new application
  frus devices                        list what the application can run on
  frus run [options] [-- <args>]      build it, start it on a device, show its log
  frus build <apk|web|desktop>        build what you ship, in release
  frus daemon                         JSON lines for an editor: devices, as they come and go
  frus help [<command>]               print this, or a command's options
  frus --version                      print the version
";

const RUN_USAGE: &str = "\
frus run — build the application, start it on a device, and show its log here

Usage:
  frus run [options] [-- <args for the application>]

Run from the application's directory. Its log follows in this terminal, on every
platform: the desktop's output, Android's logcat, the browser's console.

Options:
  -d, --device <device>    where to run: a device's id or name from `frus devices`,
                           or `android`, `web`, `desktop` (default: this computer)
      --release            a release build (default: debug)
      --port <port>        the port the web build is served on (default: 8080)
      --hot                hot reload (desktop): each saved change is patched into the
                           running application, every state kept (installs its engine
                           the first time)
      --machine            JSON lines for an editor instead of text (see `frus daemon`)
  -h, --help               print this

Examples:
  frus run
  frus run --hot
  frus run -d android
  frus run -d edge --release
";

const BUILD_USAGE: &str = "\
frus build — build what you ship, for one platform

Usage:
  frus build <target> [--debug]

Targets:
  apk        the Android package (cargo-apk; a release needs a signing key)
  web        the site: web/, with the application compiled into web/pkg/
  desktop    the program for this computer (also: windows, linux, macos)

Options:
      --debug    a debug build instead of release
  -h, --help     print this
";

const DEVICES_USAGE: &str = "\
frus devices — list what the application can run on

This computer, the Android devices adb sees (USB debugging on), the browsers
installed here, and a web server any other browser can open. The id in the
second column is what `frus run -d` takes.
";

const CREATE_USAGE: &str = "\
frus create — start a new application for desktop, Android and the web

Usage:
  frus create <directory> [options]

The directory is made if it is not there. In one that has files already, only the
template's missing files are written and nothing there is changed: `frus create .`
in an application brings back what was deleted (web/, icons, the desktop binary),
keeps its manifest and code, and takes its name from its Cargo.toml.

A new application's name is the directory's own unless --name gives one:
lowercase letters, digits, `_` and `-`.

Options:
      --name <name>        the package's name (default: the directory's name)
      --org <org>          the reverse domain the Android package starts with
                           (default: com.example; the package is <org>.<name>)
      --frus-path <dir>    build against a checkout of frus, not the published crates
      --no-git             do not make the directory a git repository
  -h, --help               print this

Examples:
  frus create my_app --org com.acme
  frus create .                        # in an application: restore missing files
";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    CreateHelp,
    RunHelp,
    BuildHelp,
    DevicesHelp,
    Version,
    Create(Options),
    Devices,
    Daemon,
    Run(run::RunOptions),
    Build(build::Target, cargo::Mode),
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse(args) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::CreateHelp) => {
            print!("{CREATE_USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::RunHelp) => {
            print!("{RUN_USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::BuildHelp) => {
            print!("{BUILD_USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::DevicesHelp) => {
            print!("{DEVICES_USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("frus {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Devices) => {
            let found = devices::discover();
            println!("{} found:\n", plural(found.len(), "device"));
            print!("{}", devices::table(&found));
            ExitCode::SUCCESS
        }
        Ok(Command::Run(options)) => outcome(run::run(&options)),
        Ok(Command::Daemon) => outcome(daemon::daemon()),
        Ok(Command::Build(target, mode)) => outcome(build::build(target, mode)),
        Ok(Command::Create(options)) => match create::create(&options) {
            Ok(created) => {
                print!("{}", report(&created));
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("error: {error}\n\nRun `frus help` for the usage.");
            ExitCode::from(2)
        }
    }
}

fn parse(args: Vec<OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };
    match command.to_str() {
        Some("-h" | "--help") => Ok(Command::Help),
        Some("help") => match args.next().as_ref().and_then(|a| a.to_str()) {
            None => Ok(Command::Help),
            Some("create") => Ok(Command::CreateHelp),
            Some("run") => Ok(Command::RunHelp),
            Some("build") => Ok(Command::BuildHelp),
            Some("devices") => Ok(Command::DevicesHelp),
            Some(other) => Err(format!("no command `{other}`")),
        },
        Some("-V" | "--version" | "version") => Ok(Command::Version),
        Some("create") => parse_create(args),
        Some("devices") => match args.next().as_ref().and_then(|a| a.to_str()) {
            None => Ok(Command::Devices),
            Some("-h" | "--help") => Ok(Command::DevicesHelp),
            Some(other) => Err(format!("frus devices takes no `{other}`")),
        },
        Some("run") => parse_run(args),
        Some("daemon") => Ok(Command::Daemon),
        Some("build") => parse_build(args),
        _ => Err(format!("no command `{}`", command.to_string_lossy())),
    }
}

fn parse_create(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.peekable();
    let mut dir = None;
    let mut name = None;
    let mut org = None;
    let mut frus_path = None;
    let mut git = true;

    while let Some(arg) = args.next() {
        let Some(text) = arg.to_str() else {
            set_once(&mut dir, PathBuf::from(arg), "<directory>")?;
            continue;
        };
        // `--name=x` and `--name x` alike.
        let (flag, inline) = match text.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_string())),
            _ => (text, None),
        };
        let mut value = |flag: &str| match &inline {
            Some(value) => Ok(value.clone()),
            None => args
                .next()
                .and_then(|value| value.into_string().ok())
                .ok_or_else(|| format!("{flag} needs a value")),
        };
        match flag {
            "-h" | "--help" => return Ok(Command::CreateHelp),
            "--name" => set_once(&mut name, value(flag)?, flag)?,
            "--org" => set_once(&mut org, value(flag)?, flag)?,
            "--frus-path" => set_once(&mut frus_path, PathBuf::from(value(flag)?), flag)?,
            "--no-git" if inline.is_none() => git = false,
            _ if flag.starts_with('-') && flag != "-" => {
                return Err(format!("frus create has no option `{text}`"));
            }
            _ => set_once(&mut dir, PathBuf::from(text), "<directory>")?,
        }
    }

    let dir = dir.ok_or("frus create needs a directory: `frus create my_app`")?;
    Ok(Command::Create(Options {
        dir,
        name,
        org: org.unwrap_or_else(|| DEFAULT_ORG.to_string()),
        frus_path,
        git,
    }))
}

fn parse_run(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.peekable();
    let mut options = run::RunOptions {
        device: None,
        mode: cargo::Mode::Debug,
        port: run::DEFAULT_PORT,
        args: Vec::new(),
        machine: false,
        hot: false,
    };
    let mut port = None;
    while let Some(arg) = args.next() {
        let text = arg
            .into_string()
            .map_err(|arg| format!("`{}` is not valid text", arg.to_string_lossy()))?;
        if text == "--" {
            options.args.extend(
                args.by_ref()
                    .map(|rest| rest.to_string_lossy().into_owned()),
            );
            break;
        }
        let (flag, inline) = match text.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_string(), Some(value.to_string()))
            }
            _ => (text.clone(), None),
        };
        let mut value = |flag: &str| match &inline {
            Some(value) => Ok(value.clone()),
            None => args
                .next()
                .and_then(|value| value.into_string().ok())
                .ok_or_else(|| format!("{flag} needs a value")),
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::RunHelp),
            "-d" | "--device" => set_once(&mut options.device, value("--device")?, "--device")?,
            "--release" if inline.is_none() => options.mode = cargo::Mode::Release,
            "--machine" if inline.is_none() => options.machine = true,
            "--hot" if inline.is_none() => options.hot = true,
            "--port" => {
                let text = value(&flag)?;
                let number = text
                    .parse()
                    .map_err(|_| format!("--port takes a number, not `{text}`"))?;
                set_once(&mut port, number, "--port")?;
            }
            _ => return Err(format!("frus run has no option `{text}`")),
        }
    }
    if let Some(port) = port {
        options.port = port;
    }
    Ok(Command::Run(options))
}

fn parse_build(args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut target = None;
    let mut mode = cargo::Mode::Release;
    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => return Ok(Command::BuildHelp),
            "--debug" => mode = cargo::Mode::Debug,
            flag if flag.starts_with('-') => {
                return Err(format!("frus build has no option `{flag}`"));
            }
            word => set_once(&mut target, build::Target::parse(word)?, "the target")?,
        }
    }
    let target = target.ok_or("frus build needs a target: `frus build apk`, `web` or `desktop`")?;
    Ok(Command::Build(target, mode))
}

/// A command's result, as the program's exit.
fn outcome(result: Result<ExitCode, String>) -> ExitCode {
    result.unwrap_or_else(|error| {
        eprintln!("error: {error}");
        ExitCode::FAILURE
    })
}

fn plural(count: usize, word: &str) -> String {
    match count {
        1 => format!("1 {word}"),
        n => format!("{n} {word}s"),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, what: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("{what} is given twice"));
    }
    *slot = Some(value);
    Ok(())
}

/// What `frus create` prints once the application is written: where it is, and the commands
/// that run it.
fn report(created: &Created) -> String {
    let project = &created.project;
    let dir = created.dir.display();
    if created.existing {
        return report_existing(created);
    }
    let mut out = format!(
        "Created {} in {dir} ({} files).\n",
        project.project_name,
        created.written.len()
    );
    if created.kept > 0 {
        out.push_str(&format!(
            "{} of the template's files were there already and are left as they are.\n",
            created.kept
        ));
    }
    match &created.git {
        Git::Initialised => out.push_str("Initialised a git repository.\n"),
        Git::InsideRepository => {
            out.push_str("The directory is inside a git repository already: no new one.\n");
        }
        Git::Failed(why) => out.push_str(&format!("No git repository: {why}\n")),
        Git::Skipped => {}
    }
    let cd = if created.dir == std::path::Path::new(".") {
        String::new()
    } else {
        format!("  cd {}\n", quote(&dir.to_string()))
    };
    out.push_str(&format!(
        "\nTo run it:\n\n\
         {cd}  cargo run                  # desktop\n  \
         cargo apk run --lib        # Android, on the connected device\n\n\
         The web, the releases and the setup each platform needs are in README.md.\n\
         The Android package is {}: settle it before the first release.\n",
        project.android_package()
    ));
    out
}

/// What `frus create` prints in an application that was there: what came back.
fn report_existing(created: &Created) -> String {
    let name = &created.project.project_name;
    if created.written.is_empty() {
        return format!(
            "{name} has every file the template gives an application: nothing to recreate.\n"
        );
    }
    let mut out = format!(
        "Recreated {} missing file{} in {name}:\n",
        created.written.len(),
        if created.written.len() == 1 { "" } else { "s" }
    );
    for path in &created.written {
        out.push_str(&format!("  {path}\n"));
    }
    out.push_str("Nothing that was there has been changed.\n");
    out
}

/// A path as a shell needs it typed.
fn quote(path: &str) -> String {
    if path.contains(|c: char| c.is_whitespace() || "'\"$&;()|<>".contains(c)) {
        format!("\"{path}\"")
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::template::FILES;

    fn args(line: &str) -> Vec<OsString> {
        line.split_whitespace().map(OsString::from).collect()
    }

    fn create_options(line: &str) -> Options {
        match parse(args(line)) {
            Ok(Command::Create(options)) => options,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn commands() {
        assert_eq!(parse(args("")), Ok(Command::Help));
        assert_eq!(parse(args("help")), Ok(Command::Help));
        assert_eq!(parse(args("--help")), Ok(Command::Help));
        assert_eq!(parse(args("help create")), Ok(Command::CreateHelp));
        assert_eq!(parse(args("create --help")), Ok(Command::CreateHelp));
        assert_eq!(parse(args("create app -h")), Ok(Command::CreateHelp));
        assert_eq!(parse(args("--version")), Ok(Command::Version));
        assert_eq!(parse(args("-V")), Ok(Command::Version));
        assert!(parse(args("deploy"))
            .unwrap_err()
            .contains("no command `deploy`"));
        assert!(parse(args("help deploy"))
            .unwrap_err()
            .contains("no command `deploy`"));
        assert_eq!(parse(args("help run")), Ok(Command::RunHelp));
        assert_eq!(parse(args("build -h")), Ok(Command::BuildHelp));
        assert_eq!(parse(args("devices")), Ok(Command::Devices));
        assert_eq!(parse(args("devices --help")), Ok(Command::DevicesHelp));
    }

    fn run_options(line: &str) -> run::RunOptions {
        match parse(args(line)) {
            Ok(Command::Run(options)) => options,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn run_options_parse() {
        let plain = run_options("run");
        assert_eq!(plain.device, None);
        assert_eq!(plain.mode, cargo::Mode::Debug);
        assert_eq!(plain.port, run::DEFAULT_PORT);

        let full = run_options("run -d edge --release --port=9000 -- /widget/button --flag");
        assert_eq!(full.device.as_deref(), Some("edge"));
        assert_eq!(full.mode, cargo::Mode::Release);
        assert_eq!(full.port, 9000);
        assert_eq!(full.args, ["/widget/button", "--flag"]);
        assert!(!full.machine);
        assert!(run_options("run --machine -d windows").machine);
        assert_eq!(parse(args("daemon")), Ok(Command::Daemon));

        assert_eq!(
            run_options("run --device=android").device.as_deref(),
            Some("android")
        );
        let error = |line| parse(args(line)).unwrap_err();
        assert!(error("run -d").contains("needs a value"));
        assert!(error("run -d a -d b").contains("given twice"));
        assert!(error("run --port x").contains("takes a number"));
        assert!(error("run --verbose").contains("no option `--verbose`"));
    }

    #[test]
    fn build_options_parse() {
        assert_eq!(
            parse(args("build apk")),
            Ok(Command::Build(build::Target::Apk, cargo::Mode::Release))
        );
        assert_eq!(
            parse(args("build --debug web")),
            Ok(Command::Build(build::Target::Web, cargo::Mode::Debug))
        );
        let error = |line| parse(args(line)).unwrap_err();
        assert!(error("build").contains("needs a target"));
        assert!(error("build apk web").contains("given twice"));
        assert!(error("build ios").contains("not `ios`"));
    }

    #[test]
    fn create_defaults() {
        let options = create_options("create my_app");
        assert_eq!(
            options,
            Options {
                dir: PathBuf::from("my_app"),
                name: None,
                org: "com.example".into(),
                frus_path: None,
                git: true,
            }
        );
    }

    #[test]
    fn create_options_in_both_spellings_and_any_order() {
        let options = create_options(
            "create --org com.acme apps/x --name=my-app --frus-path=../frus --no-git",
        );
        assert_eq!(options.dir, PathBuf::from("apps/x"));
        assert_eq!(options.name.as_deref(), Some("my-app"));
        assert_eq!(options.org, "com.acme");
        assert_eq!(options.frus_path, Some(PathBuf::from("../frus")));
        assert!(!options.git);
    }

    #[test]
    fn create_mistakes() {
        let error = |line| parse(args(line)).unwrap_err();
        assert!(error("create").contains("needs a directory"));
        assert!(error("create a b").contains("<directory> is given twice"));
        assert!(error("create a --name").contains("--name needs a value"));
        assert!(error("create a --org x --org y").contains("--org is given twice"));
        assert!(error("create a --force").contains("no option `--force`"));
        assert!(error("create a --no-git=yes").contains("no option `--no-git=yes`"));
    }

    #[test]
    fn the_report_says_how_to_run_it() {
        let created = Created {
            project: create::Project::new("my_app", "com.acme").unwrap(),
            dir: PathBuf::from("my apps/my_app"),
            written: vec!["Cargo.toml".to_string(); 14],
            kept: 0,
            existing: false,
            git: Git::Initialised,
        };
        let out = report(&created);
        assert!(
            out.starts_with("Created my_app in my apps/my_app (14 files).\n"),
            "{out}"
        );
        assert!(out.contains("Initialised a git repository."));
        assert!(
            out.contains("  cd \"my apps/my_app\"\n  cargo run "),
            "{out}"
        );
        assert!(out.contains("com.acme.my_app"));

        let here = Created {
            dir: PathBuf::from("."),
            git: Git::Skipped,
            ..created
        };
        let out = report(&here);
        assert!(!out.contains("cd "), "{out}");
        assert!(!out.contains("git"), "{out}");

        let restored = Created {
            written: vec!["web/index.html".to_string()],
            kept: 13,
            existing: true,
            ..here.clone()
        };
        assert_eq!(
            report(&restored),
            "Recreated 1 missing file in my_app:\n  web/index.html\n\
             Nothing that was there has been changed.\n"
        );
        let whole = Created {
            written: Vec::new(),
            ..restored
        };
        assert!(report(&whole).contains("nothing to recreate"));
    }

    // --- The template and the crate stay in step ---

    fn template_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("template")
    }

    fn template_text(path: &str) -> String {
        let file = FILES.iter().find(|file| file.path == path).unwrap();
        std::str::from_utf8(file.bytes)
            .unwrap()
            .replace("\r\n", "\n")
    }

    /// Every file in `template/` is built into the program, and nothing else is.
    #[test]
    fn every_template_file_is_built_in() {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let relative = path.strip_prefix(root).unwrap();
                    let parts: Vec<_> = relative
                        .components()
                        .map(|c| c.as_os_str().to_str().unwrap())
                        .collect();
                    out.push(parts.join("/"));
                }
            }
        }
        let mut on_disk = Vec::new();
        walk(&template_dir(), &template_dir(), &mut on_disk);
        on_disk.sort();
        let built_in: Vec<String> = FILES.iter().map(|file| file.path.to_string()).collect();
        assert_eq!(built_in, on_disk, "template.rs's list and template/ differ");
    }

    /// The files `frus create` copies untouched are the ones `cargo generate` is told to.
    #[test]
    fn the_verbatim_files_are_the_cargo_generate_excludes() {
        let config = template_text("cargo-generate.toml");
        let exclude = config
            .lines()
            .find(|line| line.starts_with("exclude = "))
            .unwrap();
        assert_eq!(exclude, r#"exclude = ["*.png", ".github/workflows/*.yml"]"#);
        let verbatim: Vec<_> = FILES
            .iter()
            .map(|f| f.path)
            .filter(|path| create::is_verbatim(path))
            .collect();
        assert_eq!(verbatim.len(), 7, "{verbatim:?}");
    }

    /// A new application depends on the frus this tool was released with, and asks for the
    /// compiler frus asks for.
    #[test]
    fn the_template_tracks_this_release() {
        let manifest = template_text("Cargo.toml.liquid");
        let version = env!("CARGO_PKG_VERSION");
        let major_minor: Vec<_> = version.split('.').take(2).collect();
        let requirement = format!("frus = \"{}\"", major_minor.join("."));
        assert!(
            manifest.lines().any(|line| line == requirement),
            "the template should depend on {requirement}"
        );
        let rust_version = format!("rust-version = \"{}\"", env!("CARGO_PKG_RUST_VERSION"));
        assert!(
            manifest.lines().any(|line| line == rust_version),
            "the template should ask for {rust_version}"
        );
    }
}
