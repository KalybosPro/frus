// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **The configuration changes an Android application follows in place** (milestone 763).
//!
//! Android recreates an activity for every configuration change its manifest does not declare:
//! the old activity is destroyed and a new one created in the same process. A frus application
//! cannot run a second activity in its process (its event loop is one per process), so it ends
//! with the first one and does not come back — the person sees it close. Every change an
//! activity *can* declare is therefore declared, and the shell follows it in place.
//!
//! The list lives in the application's `Cargo.toml`, where cargo-apk writes the manifest from:
//!
//! ```toml
//! [package.metadata.android.application.activity]
//! config_changes = "orientation|keyboardHidden|…"
//! ```
//!
//! Applications made before the line existed lack it, and those made between milestones 755
//! and 763 have a shorter list. Before an Android build, [`ensure`] adds the line, or replaces
//! a list frus itself wrote, and says so. A list the developer wrote is never changed: what
//! it leaves out is named in a warning.

use std::path::Path;

/// Every configuration change an activity can declare, with the API level that named it. A
/// name newer than the SDK the package is compiled against is refused by the packaging tool,
/// so the list follows the application's `target_sdk_version`.
const CHANGES: [(&str, u32); 18] = [
    ("orientation", 1),
    ("keyboardHidden", 1),
    ("keyboard", 1),
    ("screenSize", 13),
    ("smallestScreenSize", 13),
    ("locale", 1),
    ("layoutDirection", 17),
    ("fontScale", 1),
    ("screenLayout", 1),
    ("density", 17),
    ("uiMode", 8),
    ("mcc", 1),
    ("mnc", 1),
    ("touchscreen", 1),
    ("navigation", 1),
    ("colorMode", 26),
    ("fontWeightAdjustment", 31),
    ("grammaticalGender", 34),
];

/// The lists frus wrote before this one (milestone 755), replaced when found unchanged: they
/// are frus's choice, not the developer's.
const EARLIER: [&str; 1] = [
    "orientation|keyboardHidden|keyboard|screenSize|smallestScreenSize|locale|layoutDirection|fontScale|screenLayout|density|uiMode",
];

/// The table the activity's attributes are read from by cargo-apk.
const ACTIVITY: &str = "package.metadata.android.application.activity";

/// The configuration changes to declare for an application compiled against `target_sdk`.
pub(crate) fn wanted(target_sdk: u32) -> String {
    CHANGES
        .iter()
        .filter(|(_, api)| *api <= target_sdk)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join("|")
}

/// What to do about a manifest's configuration changes.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Fix {
    /// Everything is declared.
    Nothing,
    /// The manifest, with the changes declared, and what to tell the developer.
    Write { text: String, notice: String },
    /// Something is missing that frus does not write itself: what to tell the developer.
    Warn(String),
}

/// The value of the activity's `config_changes`, if the manifest gives one.
pub(crate) fn declared(manifest: &str) -> Option<String> {
    let mut section = String::new();
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(header) = header(line) {
            section = header;
            continue;
        }
        if section == ACTIVITY {
            if let Some(value) = key_value(line, "config_changes") {
                return Some(value);
            }
        }
    }
    None
}

/// What `manifest` needs so that the activity follows every configuration change it can
/// declare, compiled against `target_sdk`.
pub(crate) fn fix(manifest: &str, target_sdk: u32) -> Fix {
    let wanted = wanted(target_sdk);
    let newline = if manifest.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut section = String::new();
    let mut header_at = None;
    let mut value_at = None;
    // The activity's table made some other way — dotted keys or an inline table — which a
    // `[…activity]` header would define twice: the developer adds the line there.
    let mut elsewhere = false;
    let lines: Vec<&str> = manifest.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = header(trimmed) {
            if name == ACTIVITY && !trimmed.starts_with("[[") {
                header_at = Some(at);
            }
            section = name;
            continue;
        }
        let Some((key, _)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        if section == ACTIVITY && key == "config_changes" {
            value_at = Some(at);
        }
        // `activity.x = …` in the application's table, `application.activity… = …` in
        // Android's, and so on up: each is the activity's table, written as a key.
        let path = if section.is_empty() {
            key.to_string()
        } else {
            format!("{section}.{key}")
        };
        // Keys of the activity's own table, or of a table below it, do not count.
        if !section.starts_with(ACTIVITY)
            && (path == ACTIVITY
                || path.starts_with(&format!("{ACTIVITY}."))
                || ACTIVITY.starts_with(&format!("{path}.")))
        {
            elsewhere = true;
        }
    }
    let line = format!("config_changes = \"{wanted}\"");
    if let Some(at) = value_at {
        let value = declared(manifest).unwrap_or_default();
        let have: Vec<&str> = value.split('|').map(str::trim).collect();
        let missing: Vec<&str> = wanted.split('|').filter(|n| !have.contains(n)).collect();
        if missing.is_empty() {
            return Fix::Nothing;
        }
        if !EARLIER.iter().any(|earlier| same_set(earlier, &value)) {
            return Fix::Warn(format!(
                "Cargo.toml: `config_changes` of [{ACTIVITY}] leaves out {}. Android recreates \
                 the activity when one of them changes, and a frus application closes then. \
                 Add them to follow the change in place: {wanted}",
                missing.join(", ")
            ));
        }
        let indent = &lines[at][..lines[at].len() - lines[at].trim_start().len()];
        let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        out[at] = format!("{indent}{line}");
        return Fix::Write {
            text: join(&out, newline, manifest),
            notice: format!(
                "Cargo.toml: the activity now follows these configuration changes in place too: \
                 {} ([{ACTIVITY}] `config_changes`, written by an earlier frus).",
                missing.join(", ")
            ),
        };
    }
    if elsewhere {
        return Fix::Warn(format!(
            "Cargo.toml declares the activity's attributes without a [{ACTIVITY}] table and \
             without `config_changes`. Android recreates the activity at a configuration change \
             it does not declare (a rotation, a font size, the night setting…), and a frus \
             application closes then. Add: config_changes = \"{wanted}\""
        ));
    }
    let comment = "# The configuration changes the application follows in place (added by frus): \
                   Android recreates the activity for any other, and a frus application closes \
                   then.";
    let text = match header_at {
        Some(at) => {
            let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            out.insert(at + 1, line);
            join(&out, newline, manifest)
        }
        None => {
            let mut text = manifest.to_string();
            if !text.is_empty() && !text.ends_with('\n') {
                text.push_str(newline);
            }
            text.push_str(&format!(
                "{newline}{comment}{newline}[{ACTIVITY}]{newline}{line}{newline}"
            ));
            text
        }
    };
    Fix::Write {
        text,
        notice: format!(
            "Cargo.toml: added `config_changes` to [{ACTIVITY}], so that the activity follows a \
             rotation, a font size, the night setting, a language or a keyboard in place instead \
             of closing the application."
        ),
    }
}

/// Declares the configuration changes in the application's `Cargo.toml` when frus may (see
/// [`fix`]), saying what it did; warns when the developer has to.
pub(crate) fn ensure(root: &Path, target_sdk: u32) -> Result<(), String> {
    let path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    match fix(&manifest, target_sdk) {
        Fix::Nothing => {}
        Fix::Write { text, notice } => {
            std::fs::write(&path, text)
                .map_err(|e| format!("could not write {}: {e}", path.display()))?;
            crate::report::say(&notice);
        }
        Fix::Warn(message) => crate::report::warn(&message),
    }
    Ok(())
}

/// The name of a `[table]` or `[[array]]` header.
fn header(line: &str) -> Option<String> {
    let line = line.split(" #").next().unwrap_or(line).trim();
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let inner = inner
        .strip_prefix('[')
        .and_then(|i| i.strip_suffix(']'))
        .unwrap_or(inner);
    Some(
        inner
            .split('.')
            .map(|part| part.trim().trim_matches('"'))
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// The string value of `key = "…"` (or `'…'`) on `line`.
fn key_value(line: &str, key: &str) -> Option<String> {
    let (k, raw) = line.split_once('=')?;
    if k.trim().trim_matches('"') != key {
        return None;
    }
    let raw = raw.trim();
    let quote = raw.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let body = &raw[1..];
    Some(body[..body.find(quote)?].to_string())
}

/// Whether two `|` lists name the same changes, in any order.
fn same_set(a: &str, b: &str) -> bool {
    let mut a: Vec<&str> = a.split('|').map(str::trim).collect();
    let mut b: Vec<&str> = b.split('|').map(str::trim).collect();
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

/// `lines` joined back with `newline`, ending as `original` did.
fn join(lines: &[String], newline: &str, original: &str) -> String {
    let mut text = lines.join(newline);
    if original.ends_with('\n') {
        text.push_str(newline);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &str = "orientation|keyboardHidden|keyboard|screenSize|smallestScreenSize|locale|layoutDirection|fontScale|screenLayout|density|uiMode|mcc|mnc|touchscreen|navigation|colorMode|fontWeightAdjustment|grammaticalGender";

    const BEFORE_755: &str = "[package]\nname = \"tides\"\nversion = \"0.1.0\"\n\n\
        [package.metadata.android]\npackage = \"com.acme.tides\"\n\n\
        [package.metadata.android.application]\nlabel = \"Tides\"\n\n\
        [package.metadata.android.sdk]\ntarget_sdk_version = 34\n";

    /// The written manifest names the changes once, in the activity's table, and a second pass
    /// finds nothing to do.
    fn settles(text: &str, target_sdk: u32) {
        assert_eq!(declared(text).as_deref(), Some(wanted(target_sdk).as_str()));
        assert_eq!(text.matches("config_changes").count(), 1, "{text}");
        assert_eq!(fix(text, target_sdk), Fix::Nothing);
    }

    #[test]
    fn the_list_follows_the_sdk_the_package_is_compiled_against() {
        assert_eq!(wanted(34), ALL);
        assert!(!wanted(33).contains("grammaticalGender"));
        assert!(wanted(33).contains("fontWeightAdjustment"));
        assert!(!wanted(30).contains("fontWeightAdjustment"));
        assert!(wanted(30).contains("colorMode"));
    }

    /// An application made before milestone 755 gets the table, after everything it had.
    #[test]
    fn a_manifest_without_the_line_gets_the_table() {
        let Fix::Write { text, notice } = fix(BEFORE_755, 34) else {
            panic!("nothing written");
        };
        assert!(text.starts_with(BEFORE_755), "{text}");
        assert!(text.contains(&format!("[{ACTIVITY}]\nconfig_changes = \"{ALL}\"\n")));
        assert!(notice.contains("added"), "{notice}");
        settles(&text, 34);
        crate::project::Project::from_manifest(Path::new("."), &text).unwrap();
    }

    /// The table is there for something else (an attribute of the activity): the line goes
    /// into it, not into a second table of the same name.
    #[test]
    fn an_activity_table_without_the_line_gets_the_line() {
        let manifest = format!("{BEFORE_755}\n[{ACTIVITY}]\norientation = \"portrait\"\n");
        let Fix::Write { text, .. } = fix(&manifest, 34) else {
            panic!("nothing written");
        };
        assert_eq!(text.matches(&format!("[{ACTIVITY}]")).count(), 1);
        assert!(text.contains(&format!(
            "[{ACTIVITY}]\nconfig_changes = \"{ALL}\"\norientation = \"portrait\"\n"
        )));
        settles(&text, 34);
    }

    /// The list frus wrote at milestone 755 is frus's own: it is completed in place.
    #[test]
    fn the_list_of_milestone_755_is_completed() {
        let manifest = format!(
            "{BEFORE_755}\n[{ACTIVITY}]\nconfig_changes = \"{}\"\n",
            EARLIER[0]
        );
        let Fix::Write { text, notice } = fix(&manifest, 34) else {
            panic!("nothing written");
        };
        assert!(
            notice.contains("mcc") && notice.contains("fontWeightAdjustment"),
            "{notice}"
        );
        settles(&text, 34);
    }

    /// A list the developer chose is never rewritten: what it lacks is named.
    #[test]
    fn a_list_of_the_developer_s_own_is_kept_and_warned_about() {
        let manifest =
            format!("{BEFORE_755}\n[{ACTIVITY}]\nconfig_changes = \"orientation|screenSize\"\n");
        let Fix::Warn(message) = fix(&manifest, 34) else {
            panic!("not a warning");
        };
        assert!(message.contains("keyboardHidden") && message.contains("uiMode"));
        assert!(!message.contains("leaves out orientation"));
        // More than frus wants, or the same in another order: nothing to say.
        let reordered: Vec<&str> = ALL.split('|').rev().collect();
        let manifest = format!(
            "{BEFORE_755}\n[{ACTIVITY}]\nconfig_changes = '{}' # mine\n",
            reordered.join("|")
        );
        assert_eq!(fix(&manifest, 34), Fix::Nothing);
    }

    /// The activity's table written as keys elsewhere: a header would define it twice, which
    /// cargo refuses — the developer is told what to add.
    #[test]
    fn an_activity_written_as_keys_is_not_edited() {
        for manifest in [
            format!("{BEFORE_755}\n[package.metadata.android.application.activity.meta]\nx = 1\n"),
            "[package]\nname = \"a\"\n[package.metadata.android.application]\nactivity = { orientation = \"portrait\" }\n".to_string(),
            "[package]\nname = \"a\"\n[package.metadata.android.application]\nactivity.orientation = \"portrait\"\n".to_string(),
            "[package]\nname = \"a\"\n[package.metadata.android]\napplication.activity.config_changes = \"orientation\"\n".to_string(),
        ] {
            match fix(&manifest, 34) {
                Fix::Warn(message) => assert!(message.contains(ALL), "{message}"),
                other => {
                    // A sub-table header alone leaves the activity's table free to define.
                    assert!(manifest.contains(".activity.meta]"), "{manifest}: {other:?}");
                }
            }
        }
    }

    /// Windows line endings stay Windows line endings.
    #[test]
    fn line_endings_are_kept() {
        let manifest = BEFORE_755.replace('\n', "\r\n");
        let Fix::Write { text, .. } = fix(&manifest, 34) else {
            panic!("nothing written");
        };
        assert!(!text.replace("\r\n", "").contains('\n'), "{text:?}");
        let manifest = format!(
            "{BEFORE_755}\n[{ACTIVITY}]\nconfig_changes = \"{}\"\n",
            EARLIER[0]
        )
        .replace('\n', "\r\n");
        let Fix::Write { text, .. } = fix(&manifest, 34) else {
            panic!("nothing written");
        };
        assert!(!text.replace("\r\n", "").contains('\n'), "{text:?}");
    }

    /// Writes the file, says so once, and leaves it alone the second time.
    #[test]
    fn ensure_writes_once() {
        let root = std::env::temp_dir().join(format!("frus-config-changes-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Cargo.toml"), BEFORE_755).unwrap();
        ensure(&root, 34).unwrap();
        let once = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        ensure(&root, 34).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("Cargo.toml")).unwrap(),
            once
        );
        settles(&once, 34);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Every manifest the repository ships declares the whole list: the template, and the
    /// crates that build an APK.
    #[test]
    fn the_shipped_manifests_declare_every_change() {
        let template = include_str!("../template/Cargo.toml.liquid");
        assert_eq!(declared(template).as_deref(), Some(ALL));
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for name in [
            "frus-demo",
            "frus-hello",
            "frus-fetch-example",
            "frus-transforms",
        ] {
            let text = std::fs::read_to_string(crates.join(name).join("Cargo.toml")).unwrap();
            assert_eq!(declared(&text).as_deref(), Some(ALL), "{name}");
        }
    }
}
