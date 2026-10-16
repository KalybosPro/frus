//! `frus create`: a new application, written from the template.
//!
//! The work is split in three so that each part can be tested without the others: the names
//! are checked ([`Project::new`]), the files are rendered in memory ([`render`]), and only then
//! is anything written ([`create`]). A name that would not build, or a directory that is not
//! empty, stops the command before the disk is touched.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::template::{TemplateFile, FILES};

/// The organisation an application gets when nobody names one, as the template's own default.
pub const DEFAULT_ORG: &str = "com.example";

/// What `frus create` was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Where the application goes: a directory that does not exist yet, or an empty one.
    pub dir: PathBuf,
    /// The package's name; the directory's own name when `None`.
    pub name: Option<String>,
    /// The reverse domain the Android package starts with.
    pub org: String,
    /// A checkout of frus to build against, instead of the crates on crates.io.
    pub frus_path: Option<PathBuf>,
    /// Whether to make the directory a git repository.
    pub git: bool,
}

/// The three values the template is written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// The package's name, as given: `my_app` or `my-app`. It names the desktop program.
    pub project_name: String,
    /// The same name as Rust spells a crate: `my_app`.
    pub crate_name: String,
    /// The reverse domain of the Android package, `<org>.<crate_name>`.
    pub org: String,
}

impl Project {
    /// Checks the name and the organisation, and derives the crate's name.
    pub fn new(name: &str, org: &str) -> Result<Project, String> {
        check_name(name)?;
        let crate_name = name.replace('-', "_");
        check_org(org, &crate_name)?;
        Ok(Project {
            project_name: name.to_string(),
            crate_name,
            org: org.to_string(),
        })
    }

    /// The Android package: what a store knows the application by.
    pub fn android_package(&self) -> String {
        format!("{}.{}", self.org, self.crate_name)
    }

    fn substitute(&self, text: &str) -> String {
        text.replace("{{project-name}}", &self.project_name)
            .replace("{{crate_name}}", &self.crate_name)
            .replace("{{org}}", &self.org)
    }
}

/// Rust's keywords, the strict and the reserved ones: the crate's name is written as a path in
/// the desktop binary (`my_app::run()`), where a keyword does not parse.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Names a package cannot take: the standard library's crates, the directories cargo puts
/// beside a binary, the frus crates and the template's other dependency, which a package of
/// the same name would shadow.
const TAKEN_NAMES: &[&str] = &[
    "alloc",
    "core",
    "proc_macro",
    "proc-macro",
    "std",
    "test",
    "build",
    "deps",
    "examples",
    "incremental",
    "frus",
    "frus-cli",
    "frus-core",
    "frus-gpu",
    "frus-image",
    "frus-l10n",
    "frus-layout",
    "frus-shell",
    "frus-test",
    "frus-text",
    "frus-widgets",
    "wasm-bindgen",
    "wasm_bindgen",
];

/// Names Windows keeps for devices, which no file may have.
const WINDOWS_DEVICES: &[&str] = &[
    "aux", "con", "nul", "prn", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Java's keywords and literals, which no part of an Android package may be.
const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "void",
    "volatile",
    "while",
];

/// A name every platform accepts: a package, a crate, a program, a `.wasm` file and the last
/// part of an Android package. That is lowercase ASCII letters, digits, `_` and `-`, starting
/// with a letter.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("the name is empty".into());
    }
    if name.len() > 64 {
        return Err(format!(
            "`{name}` is longer than 64 characters, crates.io's limit"
        ));
    }
    let lower = name.to_ascii_lowercase();
    if name != lower && is_name_shaped(&lower) {
        return Err(format!(
            "`{name}` has capital letters: a package's name is lowercase, `{lower}`"
        ));
    }
    if !is_name_shaped(name) {
        return Err(format!(
            "`{name}` is not a valid name: use lowercase letters, digits, `_` and `-`, \
             starting with a letter (`my_app`, `my-app`)"
        ));
    }
    let crate_name = name.replace('-', "_");
    if RUST_KEYWORDS.contains(&crate_name.as_str()) {
        return Err(format!(
            "`{name}` is a Rust keyword: the crate could not be named"
        ));
    }
    // Cargo reads `-` and `_` as the same character in a crate's name.
    if TAKEN_NAMES
        .iter()
        .any(|taken| taken.replace('-', "_") == crate_name)
    {
        return Err(format!(
            "`{name}` is taken: it is the name of a crate the application builds with"
        ));
    }
    if WINDOWS_DEVICES.contains(&name) {
        return Err(format!(
            "`{name}` is a device name on Windows, where no file may have it"
        ));
    }
    Ok(())
}

fn is_name_shaped(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// An organisation is one or more lowercase parts separated by dots, `com.example`; with the
/// crate's name after it, it is the Android package.
pub fn check_org(org: &str, crate_name: &str) -> Result<(), String> {
    for part in org.split('.') {
        let shaped = part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !shaped {
            return Err(format!(
                "`{org}` is not a valid organisation: use lowercase parts separated by dots, \
                 each starting with a letter (`com.example`)"
            ));
        }
        if JAVA_KEYWORDS.contains(&part) {
            return Err(format!(
                "`{org}` contains `{part}`, a Java keyword, which Android refuses in a package"
            ));
        }
    }
    if JAVA_KEYWORDS.contains(&crate_name) {
        return Err(format!(
            "`{crate_name}` is a Java keyword, which Android refuses as the last part of the \
             package `{org}.{crate_name}`: choose another name"
        ));
    }
    Ok(())
}

/// Template files copied byte for byte, the same ones `cargo-generate.toml` excludes: the
/// icons are binary, and the workflows' `${{ … }}` is GitHub's syntax, not the template's.
pub fn is_verbatim(path: &str) -> bool {
    path.ends_with(".png") || (path.starts_with(".github/workflows/") && path.ends_with(".yml"))
}

/// The template's own description, which `cargo generate` reads and an application does not
/// need.
const TEMPLATE_CONFIG: &str = "cargo-generate.toml";

/// One file of the new application: its path inside it, `/`-separated, and its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// The application's files, in memory. With `frus_path`, a checkout of frus, the manifest
/// depends on that checkout's `frus` crate rather than on the published one.
pub fn render(project: &Project, frus_path: Option<&Path>) -> Result<Vec<RenderedFile>, String> {
    FILES
        .iter()
        .filter(|file| file.path != TEMPLATE_CONFIG)
        .map(|file| render_file(project, file, frus_path))
        .collect()
}

fn render_file(
    project: &Project,
    file: &TemplateFile,
    frus_path: Option<&Path>,
) -> Result<RenderedFile, String> {
    let path = project.substitute(file.path.strip_suffix(".liquid").unwrap_or(file.path));
    let bytes = if file.path.ends_with(".png") {
        file.bytes.to_vec()
    } else {
        // A checkout on Windows has CRLF files; the application gets LF, wherever `frus` was
        // built.
        let text = std::str::from_utf8(file.bytes)
            .map_err(|_| format!("template file {} is not UTF-8", file.path))?
            .replace("\r\n", "\n");
        let mut text = if is_verbatim(file.path) {
            text
        } else {
            project.substitute(&text)
        };
        if !is_verbatim(file.path) && text.contains("{{") {
            return Err(format!(
                "template file {} uses template syntax `frus create` does not know",
                file.path
            ));
        }
        if path == "Cargo.toml" {
            if let Some(checkout) = frus_path {
                text = depend_on_checkout(&text, checkout)?;
            }
        }
        text.into_bytes()
    };
    Ok(RenderedFile { path, bytes })
}

/// Replaces the manifest's `frus = "x.y"` with a path to the checkout's `frus` crate. The
/// facade reaches every other frus crate by path, so this one line moves all of them.
fn depend_on_checkout(manifest: &str, checkout: &Path) -> Result<String, String> {
    let facade = checkout.join("crates").join("frus");
    let line = manifest
        .lines()
        .find(|line| line.starts_with("frus = \""))
        .ok_or("the template's manifest has no `frus = \"…\"` dependency to replace")?;
    let path = toml_string(&display_path(&facade));
    let replacement = format!(
        "# Built against a checkout of frus (`frus create --frus-path`). To build against the\n\
         # published crates instead: {line}\n\
         frus = {{ path = {path} }}"
    );
    Ok(manifest.replacen(line, &replacement, 1))
}

/// A path as a person would type it: Windows' `\\?\` prefix, which `canonicalize` adds, removed.
fn display_path(path: &Path) -> String {
    let text = path.display().to_string();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => rest.to_string(),
        _ => text,
    }
}

/// A TOML basic string.
fn toml_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What happened to version control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Git {
    /// `--no-git`.
    Skipped,
    /// The directory is a new repository.
    Initialised,
    /// The directory is inside a repository already, which keeps it.
    InsideRepository,
    /// git could not be run, or failed: the application is there all the same.
    Failed(String),
}

/// What `frus create` made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub project: Project,
    pub dir: PathBuf,
    pub files: usize,
    pub git: Git,
}

/// Creates the application: checks the names, renders the files, then writes them.
pub fn create(options: &Options) -> Result<Created, String> {
    let name = match &options.name {
        Some(name) => name.clone(),
        None => name_of(&options.dir)?,
    };
    let project = Project::new(&name, &options.org)?;

    let frus_path = match &options.frus_path {
        Some(path) => Some(check_checkout(path)?),
        None => None,
    };
    let files = render(&project, frus_path.as_deref())?;

    let made_dir = prepare_dir(&options.dir)?;
    if let Err(error) = write_files(&options.dir, &files) {
        // Only a directory this command made is taken away again: an empty one that was there
        // before stays, with whatever was written.
        if made_dir {
            let _ = fs::remove_dir_all(&options.dir);
        }
        return Err(error);
    }

    let git = if options.git {
        git_init(&options.dir)
    } else {
        Git::Skipped
    };
    Ok(Created {
        project,
        dir: options.dir.clone(),
        files: files.len(),
        git,
    })
}

/// The name a directory gives an application: its last component, `.` included.
fn name_of(dir: &Path) -> Result<String, String> {
    // A trailing `..` is refused as written, before the path is made absolute: Windows
    // resolves it there and Unix keeps it, so the answer would depend on the platform.
    if matches!(
        dir.components().next_back(),
        Some(std::path::Component::ParentDir)
    ) {
        return Err(format!(
            "cannot tell a name from {}: pass --name",
            dir.display()
        ));
    }
    let absolute = std::path::absolute(dir)
        .map_err(|error| format!("cannot resolve {}: {error}", dir.display()))?;
    // `components` drops a trailing `.`.
    let last = absolute.components().next_back();
    match last {
        Some(std::path::Component::Normal(name)) => name
            .to_str()
            .map(str::to_string)
            .ok_or_else(|| format!("{} is not a UTF-8 name: pass --name", dir.display())),
        _ => Err(format!(
            "cannot tell a name from {}: pass --name",
            dir.display()
        )),
    }
}

/// The checkout's root, made absolute, once it is known to hold the `frus` facade.
fn check_checkout(path: &Path) -> Result<PathBuf, String> {
    let root = fs::canonicalize(path)
        .map_err(|error| format!("--frus-path {}: {error}", path.display()))?;
    let manifest = root.join("crates").join("frus").join("Cargo.toml");
    let is_facade = fs::read_to_string(&manifest)
        .is_ok_and(|text| text.lines().any(|line| line.trim() == "name = \"frus\""));
    if !is_facade {
        return Err(format!(
            "--frus-path {}: not a checkout of frus (no crates/frus/Cargo.toml)",
            path.display()
        ));
    }
    Ok(root)
}

/// Makes the directory, or checks that the one there is empty. True when it was made here.
fn prepare_dir(dir: &Path) -> Result<bool, String> {
    if dir.exists() {
        if !dir.is_dir() {
            return Err(format!("{} exists and is not a directory", dir.display()));
        }
        let mut entries =
            fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "{} is not empty: frus create writes only into a new or empty directory",
                dir.display()
            ));
        }
        Ok(false)
    } else {
        fs::create_dir_all(dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
        Ok(true)
    }
}

fn write_files(dir: &Path, files: &[RenderedFile]) -> Result<(), String> {
    for file in files {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        fs::write(&path, &file.bytes)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

/// `git init`, unless the directory already sits in a repository, as `cargo new` does.
fn git_init(dir: &Path) -> Git {
    let inside = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output();
    match inside {
        Err(error) => return Git::Failed(format!("git could not be run: {error}")),
        Ok(output) if output.status.success() && output.stdout.starts_with(b"true") => {
            return Git::InsideRepository;
        }
        Ok(_) => {}
    }
    match Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir)
        .output()
    {
        Ok(output) if output.status.success() => Git::Initialised,
        Ok(output) => Git::Failed(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Git::Failed(format!("git could not be run: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        Project::new("my-app", "com.acme").unwrap()
    }

    fn file<'a>(files: &'a [RenderedFile], path: &str) -> &'a str {
        let file = files
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| {
                panic!(
                    "no {path} in {:?}",
                    files.iter().map(|f| &f.path).collect::<Vec<_>>()
                )
            });
        std::str::from_utf8(&file.bytes).unwrap()
    }

    /// A directory of its own under the system's temporary one, gone when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!("frus-cli-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn options(dir: PathBuf) -> Options {
        Options {
            dir,
            name: None,
            org: DEFAULT_ORG.to_string(),
            frus_path: None,
            git: false,
        }
    }

    #[test]
    fn names_that_build_everywhere() {
        for name in ["app", "my_app", "my-app", "app2", "a_b-c"] {
            assert_eq!(check_name(name), Ok(()), "{name}");
        }
        let refused = [
            ("", "empty"),
            ("2app", "not a valid name"),
            ("_app", "not a valid name"),
            ("-app", "not a valid name"),
            ("my app", "not a valid name"),
            ("my.app", "not a valid name"),
            ("café", "not a valid name"),
            ("MyApp", "lowercase, `myapp`"),
            ("fn", "Rust keyword"),
            ("self", "Rust keyword"),
            ("frus", "taken"),
            ("frus-widgets", "taken"),
            ("frus_widgets", "taken"),
            ("test", "taken"),
            ("proc-macro", "taken"),
            ("wasm-bindgen", "taken"),
            ("con", "device name"),
            ("lpt1", "device name"),
        ];
        for (name, why) in refused {
            let error = check_name(name).expect_err(name);
            assert!(error.contains(why), "{name}: {error}");
        }
        assert!(check_name(&"a".repeat(64)).is_ok());
        assert!(check_name(&"a".repeat(65)).unwrap_err().contains("64"));
    }

    #[test]
    fn organisations_android_accepts() {
        for org in ["com.example", "io.github.someone", "acme", "com.a_b2"] {
            assert_eq!(check_org(org, "app"), Ok(()), "{org}");
        }
        let refused = [
            ("", "not a valid organisation"),
            ("com.", "not a valid organisation"),
            (".com", "not a valid organisation"),
            ("com..acme", "not a valid organisation"),
            ("Com.Acme", "not a valid organisation"),
            ("com.2acme", "not a valid organisation"),
            ("com.my-co", "not a valid organisation"),
            ("com.new", "Java keyword"),
        ];
        for (org, why) in refused {
            let error = check_org(org, "app").expect_err(org);
            assert!(error.contains(why), "{org}: {error}");
        }
        // The crate's name ends the package, so it is held to the same rule.
        assert!(check_org("com.acme", "class")
            .unwrap_err()
            .contains("Java keyword"));
        assert!(Project::new("int", "com.acme").is_err());
    }

    #[test]
    fn the_crate_name_is_the_name_with_underscores() {
        let project = project();
        assert_eq!(project.project_name, "my-app");
        assert_eq!(project.crate_name, "my_app");
        assert_eq!(project.android_package(), "com.acme.my_app");
    }

    #[test]
    fn rendering_fills_in_every_value() {
        let files = render(&project(), None).unwrap();
        let manifest = file(&files, "Cargo.toml");
        assert!(manifest.contains("name = \"my-app\""));
        assert!(manifest.contains("package = \"com.acme.my_app\""));
        assert!(manifest.contains("label = \"my-app\""));
        assert!(file(&files, "src/bin/my-app.rs").contains("my_app::run()"));
        assert!(file(&files, "web/index.html").contains("./pkg/my_app.js"));
        assert!(file(&files, "src/lib.rs").contains(".title(\"my-app\")"));
        // The tests live in `tests/`, as a Flutter project's do in `test/`, and reach the
        // application as the library it is (milestone 658).
        assert!(file(&files, "tests/counter_test.rs").contains("use my_app::CounterState;"));

        for rendered in &files {
            assert!(!rendered.path.contains("{{"), "{}", rendered.path);
            assert!(!rendered.path.ends_with(".liquid"), "{}", rendered.path);
            if rendered.path.ends_with(".png") {
                continue;
            }
            assert!(
                !rendered.bytes.windows(2).any(|w| w == b"\r\n"),
                "{}",
                rendered.path
            );
            if !is_verbatim(&rendered.path) {
                let text = std::str::from_utf8(&rendered.bytes).unwrap();
                assert!(
                    !text.contains("{{"),
                    "{} keeps template syntax",
                    rendered.path
                );
            }
        }
    }

    #[test]
    fn rendering_leaves_out_the_template_config() {
        let files = render(&project(), None).unwrap();
        assert!(files.iter().all(|file| file.path != TEMPLATE_CONFIG));
        assert_eq!(files.len(), FILES.len() - 1);
    }

    #[test]
    fn the_icons_and_the_workflows_are_copied_as_they_are() {
        let files = render(&project(), None).unwrap();
        let icon = files
            .iter()
            .find(|f| f.path == "res/mipmap-hdpi/ic_launcher.png")
            .unwrap();
        let original = FILES
            .iter()
            .find(|f| f.path == "res/mipmap-hdpi/ic_launcher.png")
            .unwrap();
        assert_eq!(icon.bytes, original.bytes);
        assert!(icon.bytes.starts_with(b"\x89PNG"));
        // GitHub's own `${{ … }}` survives.
        assert!(file(&files, ".github/workflows/release.yml")
            .contains("${{ needs.meta.outputs.name }}"));
    }

    #[test]
    fn a_checkout_replaces_the_published_frus() {
        let checkout = Path::new("/src/frus");
        let files = render(&project(), Some(checkout)).unwrap();
        let manifest = file(&files, "Cargo.toml");
        let facade = toml_string(&checkout.join("crates").join("frus").display().to_string());
        assert!(
            manifest.contains(&format!("\nfrus = {{ path = {facade} }}\n")),
            "{manifest}"
        );
        // The published requirement is kept in a comment, to go back to.
        assert!(
            manifest.contains("# published crates instead: frus = \"0."),
            "{manifest}"
        );
        assert_eq!(
            manifest
                .lines()
                .filter(|l| l.starts_with("frus = "))
                .count(),
            1
        );
    }

    #[test]
    fn toml_strings_escape_what_they_must() {
        assert_eq!(toml_string(r"C:\src\frus"), r#""C:\\src\\frus""#);
        assert_eq!(toml_string("a\"b"), r#""a\"b""#);
        assert_eq!(toml_string("a\tb"), r#""a\u0009b""#);
    }

    #[test]
    fn windows_verbatim_prefix_is_dropped() {
        assert_eq!(display_path(Path::new(r"\\?\D:\src\frus")), r"D:\src\frus");
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\host\share")),
            r"\\?\UNC\host\share"
        );
        assert_eq!(display_path(Path::new("/src/frus")), "/src/frus");
    }

    #[test]
    fn a_name_comes_from_the_directory() {
        assert_eq!(name_of(Path::new("some/where/my_app")).unwrap(), "my_app");
        assert_eq!(name_of(Path::new("my_app/.")).unwrap(), "my_app");
        let current = std::env::current_dir().unwrap();
        let expected = current.file_name().unwrap().to_str().unwrap();
        assert_eq!(name_of(Path::new(".")).unwrap(), expected);
        assert!(name_of(Path::new("my_app/.."))
            .unwrap_err()
            .contains("--name"));
    }

    #[test]
    fn create_writes_the_application() {
        let temp = TempDir::new("writes");
        let dir = temp.0.join("hello_app");
        let created = create(&options(dir.clone())).unwrap();
        assert_eq!(created.project.crate_name, "hello_app");
        assert_eq!(created.git, Git::Skipped);
        assert_eq!(created.files, FILES.len() - 1);
        let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("package = \"com.example.hello_app\""));
        assert!(dir.join("src/bin/hello_app.rs").is_file());
        assert!(dir.join("res/mipmap-xxxhdpi/ic_launcher.png").is_file());
        assert!(dir.join(".github/workflows/ci.yml").is_file());
        assert!(!dir.join(TEMPLATE_CONFIG).exists());
    }

    #[test]
    fn create_fills_an_empty_directory() {
        let temp = TempDir::new("empty");
        let mut opts = options(temp.0.clone());
        opts.name = Some("given".into());
        let created = create(&opts).unwrap();
        assert_eq!(created.project.project_name, "given");
        assert!(temp.0.join("src/bin/given.rs").is_file());
    }

    #[test]
    fn create_refuses_a_directory_with_something_in_it() {
        let temp = TempDir::new("full");
        fs::write(temp.0.join("notes.txt"), "mine").unwrap();
        let mut opts = options(temp.0.clone());
        opts.name = Some("app".into());
        assert!(create(&opts).unwrap_err().contains("not empty"));
        // Nothing was touched.
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 1);
        assert_eq!(
            fs::read_to_string(temp.0.join("notes.txt")).unwrap(),
            "mine"
        );
    }

    #[test]
    fn create_checks_everything_before_writing() {
        let temp = TempDir::new("checks");
        let dir = temp.0.join("Bad Name");
        assert!(create(&options(dir.clone()))
            .unwrap_err()
            .contains("not a valid name"));
        assert!(!dir.exists(), "a refused name leaves no directory behind");

        let dir = temp.0.join("app");
        let mut opts = options(dir.clone());
        opts.frus_path = Some(temp.0.join("nowhere"));
        assert!(create(&opts).unwrap_err().contains("--frus-path"));
        assert!(!dir.exists());

        // A directory that exists and is not a checkout.
        opts.frus_path = Some(temp.0.clone());
        assert!(create(&opts)
            .unwrap_err()
            .contains("not a checkout of frus"));
        assert!(!dir.exists());
    }

    #[test]
    fn create_refuses_a_file_in_the_way() {
        let temp = TempDir::new("file");
        let path = temp.0.join("app");
        fs::write(&path, "").unwrap();
        assert!(create(&options(path))
            .unwrap_err()
            .contains("not a directory"));
    }

    #[test]
    fn create_against_this_checkout() {
        let temp = TempDir::new("checkout");
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let mut opts = options(temp.0.join("app"));
        opts.frus_path = Some(checkout.clone());
        create(&opts).unwrap();
        let manifest = fs::read_to_string(temp.0.join("app").join("Cargo.toml")).unwrap();
        let facade = fs::canonicalize(&checkout)
            .unwrap()
            .join("crates")
            .join("frus");
        let expected = toml_string(&display_path(&facade));
        assert!(
            manifest.contains(&format!("frus = {{ path = {expected} }}")),
            "{manifest}"
        );
    }
}
