// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **The permissions an application declares** (milestone 744): one table in its `Cargo.toml`,
//! and every platform's own declaration written from it.
//!
//! ```toml
//! [package.metadata.frus.permissions]
//! camera = "Scans the code on your ticket."
//! microphone = "Records your voice notes."
//! location = "Shows the shops near you."
//! notifications = true
//! ```
//!
//! The text is the reason the person reads in the system's question on macOS and iOS, which end
//! an application that asks without one: the camera, the microphone and the location must have
//! it. Notifications need none, so `true` declares them.
//!
//! What is written from it:
//!
//! - **macOS**: the usage strings, in the `Info.plist` of the `.app` that `frus build` and
//!   `frus run` make (see `bundle`).
//! - **Android**: the `[[package.metadata.android.uses_permission]]` tables cargo-apk turns into
//!   the manifest's `<uses-permission>`, added to `Cargo.toml` when one is missing.
//! - **iOS** takes the same usage strings, from [`usage_strings`], once frus builds for it.

use std::fmt;
use std::path::Path;

/// The table the permissions are declared in.
pub(crate) const SECTION: &str = "package.metadata.frus.permissions";

/// The table cargo-apk reads a permission from, once per permission.
const ANDROID: &str = "[package.metadata.android.uses_permission]";

/// What an application may ask the person for: the shell's `Permission`, by the name the table
/// gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Camera,
    Microphone,
    Location,
    Notifications,
}

impl Kind {
    const ALL: [Kind; 4] = [
        Kind::Camera,
        Kind::Microphone,
        Kind::Location,
        Kind::Notifications,
    ];

    fn key(self) -> &'static str {
        match self {
            Kind::Camera => "camera",
            Kind::Microphone => "microphone",
            Kind::Location => "location",
            Kind::Notifications => "notifications",
        }
    }

    /// Whether the system's question shows a reason, which must then be declared.
    fn needs_reason(self) -> bool {
        self != Kind::Notifications
    }

    /// The `Info.plist` key holding the reason, on macOS and iOS alike.
    fn usage_key(self) -> Option<&'static str> {
        match self {
            Kind::Camera => Some("NSCameraUsageDescription"),
            Kind::Microphone => Some("NSMicrophoneUsageDescription"),
            Kind::Location => Some("NSLocationWhenInUseUsageDescription"),
            Kind::Notifications => None,
        }
    }

    /// What Android's manifest must name — the shell asks for the same ones.
    fn android_names(self) -> &'static [&'static str] {
        match self {
            Kind::Camera => &["android.permission.CAMERA"],
            Kind::Microphone => &["android.permission.RECORD_AUDIO"],
            Kind::Location => &[
                "android.permission.ACCESS_FINE_LOCATION",
                "android.permission.ACCESS_COARSE_LOCATION",
            ],
            Kind::Notifications => &["android.permission.POST_NOTIFICATIONS"],
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

/// One permission the application declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declared {
    pub kind: Kind,
    /// Why the application wants it, as the person reads it.
    pub reason: Option<String>,
}

/// The declared permissions, from the table's lines as the manifest holds them: `(key, value)`,
/// the value as written after the `=`.
pub(crate) fn parse(lines: &[(String, String)]) -> Result<Vec<Declared>, String> {
    let mut declared: Vec<Declared> = Vec::new();
    for (key, raw) in lines {
        let Some(kind) = Kind::ALL.into_iter().find(|kind| kind.key() == key) else {
            return Err(format!(
                "[{SECTION}] has `{key}`, which is not a permission frus knows: it knows {}",
                Kind::ALL.map(Kind::key).join(", ")
            ));
        };
        if declared.iter().any(|d| d.kind == kind) {
            return Err(format!("[{SECTION}] declares `{key}` twice"));
        }
        let missing_reason = || {
            format!(
                "[{SECTION}] `{key}` needs the reason the person is shown, as text: \
                 {key} = \"Why the application wants {}.\" macOS and iOS end an application \
                 that asks without one",
                match kind {
                    Kind::Camera => "the camera",
                    Kind::Microphone => "the microphone",
                    _ => "the location",
                }
            )
        };
        let reason = match value(raw).map_err(|why| format!("[{SECTION}] `{key}`: {why}"))? {
            Value::Bool(false) => continue,
            Value::Bool(true) if kind.needs_reason() => return Err(missing_reason()),
            Value::Bool(true) => None,
            Value::Text(text) if text.trim().is_empty() => {
                if kind.needs_reason() {
                    return Err(missing_reason());
                }
                None
            }
            Value::Text(text) => {
                if let Some(c) = text
                    .chars()
                    .find(|c| c.is_control() && !matches!(c, '\n' | '\t'))
                {
                    return Err(format!(
                        "[{SECTION}] `{key}`: the reason holds the control character U+{:04X}, \
                         which an Info.plist cannot carry",
                        c as u32
                    ));
                }
                Some(text.trim().to_string())
            }
        };
        declared.push(Declared { kind, reason });
    }
    Ok(declared)
}

/// The `Info.plist` keys and their text, in the order declared: what macOS and iOS read before
/// they ask.
pub(crate) fn usage_strings(declared: &[Declared]) -> Vec<(&'static str, &str)> {
    declared
        .iter()
        .filter_map(|d| Some((d.kind.usage_key()?, d.reason.as_deref()?)))
        .collect()
}

/// The Android permissions the declarations need, each once.
pub(crate) fn android_permissions(declared: &[Declared]) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for name in declared.iter().flat_map(|d| d.kind.android_names()) {
        if !names.contains(name) {
            names.push(name);
        }
    }
    names
}

/// The text to add to `manifest` so that cargo-apk declares every permission in `wanted`, or
/// `None` when it already does. An inline `uses_permission = [...]` cannot be added to by
/// tables: what it lacks is an error, naming what to add.
pub(crate) fn android_additions(manifest: &str, wanted: &[&str]) -> Result<Option<String>, String> {
    let mut section = String::new();
    let mut named: Vec<String> = Vec::new();
    let mut inline = false;
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = header.trim().to_string();
            continue;
        }
        let Some((key, raw)) = line.split_once('=') else {
            continue;
        };
        match (section.as_str(), key.trim()) {
            (ANDROID, "name") => {
                if let Ok(Value::Text(name)) = value(raw) {
                    named.push(name);
                }
            }
            ("package.metadata.android", "uses_permission") => {
                inline = true;
                named.extend(
                    raw.split('"')
                        .filter(|part| part.starts_with("android.permission."))
                        .map(str::to_string),
                );
            }
            _ => {}
        }
    }
    let missing: Vec<&str> = wanted
        .iter()
        .copied()
        .filter(|name| !named.iter().any(|n| n == name))
        .collect();
    if missing.is_empty() {
        return Ok(None);
    }
    if inline {
        return Err(format!(
            "[{SECTION}] needs {} in Android's manifest, and the `uses_permission` list of \
             [package.metadata.android] does not name it: add it there",
            missing.join(", ")
        ));
    }
    let newline = if manifest.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut text = String::new();
    if !manifest.is_empty() && !manifest.ends_with('\n') {
        text.push_str(newline);
    }
    text.push_str(newline);
    text.push_str(&format!(
        "# Written by frus from [{SECTION}]: Android refuses, without asking, a permission its \
         manifest does not name.{newline}"
    ));
    for (i, name) in missing.iter().enumerate() {
        if i > 0 {
            text.push_str(newline);
        }
        text.push_str(&format!("[{ANDROID}]{newline}name = \"{name}\"{newline}"));
    }
    Ok(Some(text))
}

/// Adds to the application's `Cargo.toml` the Android permissions its declarations need, saying
/// which. Nothing is written when none is missing.
pub(crate) fn write_android(root: &Path, declared: &[Declared]) -> Result<(), String> {
    let wanted = android_permissions(declared);
    if wanted.is_empty() {
        return Ok(());
    }
    let path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let Some(addition) = android_additions(&manifest, &wanted)? else {
        return Ok(());
    };
    std::fs::write(&path, format!("{manifest}{addition}"))
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    crate::report::say(&format!(
        "Declared for Android in Cargo.toml, from [{SECTION}]: {}",
        wanted
            .iter()
            .filter(|name| addition.contains(*name))
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    ));
    Ok(())
}

/// `text` as XML character data: what a plist `<string>` may hold.
pub(crate) fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A value of the table, as far as permissions go.
#[derive(Debug, PartialEq, Eq)]
enum Value {
    Bool(bool),
    Text(String),
}

/// The TOML value written after a key's `=`: a boolean, a basic string with its escapes, or a
/// literal string; a comment may follow.
fn value(raw: &str) -> Result<Value, String> {
    let raw = raw.trim();
    let (value, rest) = if raw.starts_with("\"\"\"") || raw.starts_with("'''") {
        return Err("write the reason on one line".into());
    } else if let Some(body) = raw.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = body.char_indices();
        let mut end = None;
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    end = Some(i + 1);
                    break;
                }
                '\\' => {
                    let (_, escape) = chars.next().ok_or("a string is not closed")?;
                    match escape {
                        'b' => out.push('\u{8}'),
                        't' => out.push('\t'),
                        'n' => out.push('\n'),
                        'f' => out.push('\u{c}'),
                        'r' => out.push('\r'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        'u' | 'U' => {
                            let digits = if escape == 'u' { 4 } else { 8 };
                            let hex: String = chars.by_ref().take(digits).map(|(_, c)| c).collect();
                            let code = u32::from_str_radix(&hex, 16)
                                .ok()
                                .filter(|_| hex.len() == digits)
                                .and_then(char::from_u32)
                                .ok_or_else(|| format!("`\\{escape}{hex}` is not a character"))?;
                            out.push(code);
                        }
                        other => return Err(format!("`\\{other}` is not an escape TOML knows")),
                    }
                }
                c => out.push(c),
            }
        }
        let end = end.ok_or("a string is not closed")?;
        (Value::Text(out), &body[end..])
    } else if let Some(body) = raw.strip_prefix('\'') {
        let end = body.find('\'').ok_or("a string is not closed")?;
        (Value::Text(body[..end].to_string()), &body[end + 1..])
    } else {
        let word = raw.split('#').next().unwrap_or("").trim();
        let rest = &raw[word.len()..];
        match word {
            "true" => (Value::Bool(true), rest),
            "false" => (Value::Bool(false), rest),
            other => {
                return Err(format!(
                    "`{other}` is neither the reason, as text, nor true or false"
                ))
            }
        }
    };
    let rest = rest.trim();
    if !rest.is_empty() && !rest.starts_with('#') {
        return Err(format!("`{rest}` follows the value"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn the_table_declares_each_permission_with_its_reason() {
        let declared = parse(&lines(&[
            (
                "camera",
                r#" "Scans the code on your ticket."  # shown by macOS"#,
            ),
            ("microphone", r#" 'Records C:\notes "as said".'"#),
            ("location", r#" "Shops \"near\" you \u00e9\n2""#),
            ("notifications", " true"),
        ]))
        .unwrap();
        assert_eq!(
            declared,
            [
                Declared {
                    kind: Kind::Camera,
                    reason: Some("Scans the code on your ticket.".into())
                },
                Declared {
                    kind: Kind::Microphone,
                    reason: Some(r#"Records C:\notes "as said"."#.into())
                },
                Declared {
                    kind: Kind::Location,
                    reason: Some("Shops \"near\" you \u{e9}\n2".into())
                },
                Declared {
                    kind: Kind::Notifications,
                    reason: None
                },
            ]
        );
        assert_eq!(
            usage_strings(&declared),
            [
                ("NSCameraUsageDescription", "Scans the code on your ticket."),
                (
                    "NSMicrophoneUsageDescription",
                    r#"Records C:\notes "as said"."#
                ),
                (
                    "NSLocationWhenInUseUsageDescription",
                    "Shops \"near\" you \u{e9}\n2"
                ),
            ]
        );
        assert_eq!(
            android_permissions(&declared),
            [
                "android.permission.CAMERA",
                "android.permission.RECORD_AUDIO",
                "android.permission.ACCESS_FINE_LOCATION",
                "android.permission.ACCESS_COARSE_LOCATION",
                "android.permission.POST_NOTIFICATIONS",
            ]
        );
    }

    /// **A missing reason is an error before anything is built**, naming the key and what to
    /// write: macOS would end the application at the question, with nothing to catch.
    #[test]
    fn a_permission_the_person_is_asked_for_needs_its_reason() {
        for raw in [" true", r#" """#, r#" "   ""#] {
            let error = parse(&lines(&[("camera", raw)])).unwrap_err();
            assert!(error.contains("`camera` needs the reason"), "{error}");
            assert!(error.contains("camera = \"Why the application wants the camera.\""));
        }
        let error = parse(&lines(&[("location", " true")])).unwrap_err();
        assert!(error.contains("wants the location"), "{error}");
        // Notifications show no reason: `true`, or text, declares them.
        assert_eq!(
            parse(&lines(&[("notifications", r#" "Reminders""#)])).unwrap()[0].kind,
            Kind::Notifications
        );
        // `false` declares nothing.
        assert_eq!(parse(&lines(&[("camera", " false # later")])).unwrap(), []);
    }

    #[test]
    fn what_the_table_cannot_hold_is_said() {
        let error = parse(&lines(&[("contacts", r#" "x""#)])).unwrap_err();
        assert!(error.contains("`contacts`"), "{error}");
        assert!(error.contains("camera, microphone, location, notifications"));
        let error = parse(&lines(&[("camera", r#" "a""#), ("camera", r#" "b""#)])).unwrap_err();
        assert!(error.contains("twice"), "{error}");
        for (raw, says) in [
            (r#" "open"#, "not closed"),
            (r#" """long""""#, "one line"),
            (" 3", "neither the reason"),
            (r#" "a" "b""#, "follows the value"),
            (r#" "\q""#, "not an escape"),
            (r#" "\u00""#, "not a character"),
            (r#" "bell\u0007""#, "U+0007"),
        ] {
            let error = parse(&lines(&[("camera", raw)])).unwrap_err();
            assert!(error.contains(says), "{raw}: {error}");
        }
    }

    #[test]
    fn a_reason_is_escaped_for_xml() {
        assert_eq!(
            xml_escape(r#"Tom & Jerry's <"cam"> é"#),
            "Tom &amp; Jerry&apos;s &lt;&quot;cam&quot;&gt; é"
        );
    }

    const ANDROID_MANIFEST: &str = "[package]\nname = \"app\"\n\n\
        [package.metadata.android]\npackage = \"com.acme.app\"\n\n\
        [[package.metadata.android.uses_permission]]\nname = \"android.permission.CAMERA\"\n";

    /// **Android's manifest is completed, not rewritten**: what it names stays, what is missing
    /// is added as cargo-apk's tables, once.
    #[test]
    fn android_gets_the_permissions_it_does_not_name() {
        let wanted = [
            "android.permission.CAMERA",
            "android.permission.RECORD_AUDIO",
            "android.permission.POST_NOTIFICATIONS",
        ];
        let addition = android_additions(ANDROID_MANIFEST, &wanted)
            .unwrap()
            .unwrap();
        assert!(addition.starts_with("\n# Written by frus"), "{addition}");
        assert!(!addition.contains("CAMERA"), "{addition}");
        assert!(addition.contains(
            "[[package.metadata.android.uses_permission]]\nname = \"android.permission.RECORD_AUDIO\"\n\n\
             [[package.metadata.android.uses_permission]]\nname = \"android.permission.POST_NOTIFICATIONS\"\n"
        ));
        let completed = format!("{ANDROID_MANIFEST}{addition}");
        assert_eq!(android_additions(&completed, &wanted), Ok(None));
        assert_eq!(android_additions(ANDROID_MANIFEST, &wanted[..1]), Ok(None));

        // A manifest written with CRLF and no last newline is continued as written.
        let windows = "[package]\r\nname = \"app\"";
        let addition = android_additions(windows, &wanted[..1]).unwrap().unwrap();
        assert!(
            addition.starts_with("\r\n\r\n# Written by frus"),
            "{addition:?}"
        );
        assert!(!addition.replace("\r\n", "").contains('\n'), "{addition:?}");

        // An inline list is the developer's to complete.
        let inline = "[package.metadata.android]\nuses_permission = [{ name = \"android.permission.CAMERA\" }]\n";
        assert_eq!(android_additions(inline, &wanted[..1]), Ok(None));
        let error = android_additions(inline, &wanted).unwrap_err();
        assert!(error.contains("RECORD_AUDIO"), "{error}");
    }

    #[test]
    fn write_android_adds_to_the_manifest_once() {
        let root = std::env::temp_dir().join(format!("frus-perm-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Cargo.toml"), ANDROID_MANIFEST).unwrap();
        let declared = parse(&lines(&[("camera", r#" "x""#), ("location", r#" "y""#)])).unwrap();
        write_android(&root, &declared).unwrap();
        let once = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(once.contains("ACCESS_FINE_LOCATION") && once.contains("ACCESS_COARSE_LOCATION"));
        assert_eq!(once.matches("android.permission.CAMERA").count(), 1);
        write_android(&root, &declared).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("Cargo.toml")).unwrap(),
            once
        );
        // And it still reads as the same application.
        crate::project::Project::from_manifest(&root, &once).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }
}
