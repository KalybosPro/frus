//! **An application's licences**: the registry, and the three widgets that show them —
//! [`LicensePage`], [`AboutDialog`] and [`AboutListTile`].
//!
//! Every application distributed anywhere, and every application depending on anything
//! under Apache-2.0 or MIT, has to show the licences of what it links. Until now this
//! framework gave it nothing to show them with, so an application wrote its own screen or
//! — more likely — shipped without one. The obligation is silent: nothing warns anybody
//! that it is missing.
//!
//! ## Where the text comes from, which is the whole question
//!
//! The reference keeps a **registry** that packages add themselves to at startup. That
//! shape does not survive the crossing: a Rust crate cannot run code before `main`, so
//! nothing a dependency ships would ever register itself, and the list would be
//! hand-maintained — which is to say wrong the first time somebody adds a dependency.
//!
//! What Rust has instead is better: **cargo knows the answer**. `scripts/gen_licenses.py`
//! reads the actual dependency graph of an application (`cargo tree -e no-dev --target
//! all`), finds the licence files each package ships, groups the ones that are
//! byte-identical, and writes a plain-text file the application embeds:
//!
//! ```ignore
//! frus::licenses::add_all(include_str!("../assets/licenses.txt"));
//! ```
//!
//! One call, at startup, and the list cannot drift from what is linked without the file
//! changing — which is the one failure mode that matters here.
//!
//! **Nothing is registered by default**, and that is deliberate. A framework can only know
//! its own subtree, and an application's list has to cover the application; a token entry
//! for frus would be a list that looks complete and is not. An empty registry says so on
//! the page instead, in words, which turns a silent obligation into a visible one.

use std::rc::Rc;
use std::sync::Mutex;

use frus_core::TextStyle;
use frus_layout::{Align, Dimension};

use crate::theme::Theme;
use crate::widget::Widget;

/// A package, as cargo names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    /// Its crate name.
    pub name: String,
    /// Its version.
    pub version: String,
}

impl Package {
    /// A package from its name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    /// `name version`, the way cargo prints it.
    pub fn label(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}

/// One licence text, and every package that ships exactly it.
///
/// The grouping is the generator's: a hundred packages under the same Apache-2.0 file are
/// one notice covering a hundred packages, which is what keeps a page of four hundred
/// dependencies readable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LicenseNotice {
    /// The packages this text covers.
    pub packages: Vec<Package>,
    /// The licence, as the packages ship it.
    pub text: String,
}

/// The notices an application has registered.
///
/// A `Mutex` and not a thread-local: registering happens once, at startup, from wherever
/// the application's `main` runs, and the page is built on the UI thread. The same shape
/// as [`frus_core::set_image_fetcher`], for the same reason.
static REGISTRY: Mutex<Vec<LicenseNotice>> = Mutex::new(Vec::new());

/// Registers one notice.
pub fn add(notice: LicenseNotice) {
    if let Ok(mut guard) = REGISTRY.lock() {
        guard.push(notice);
    }
}

/// Registers everything in a generated file, and answers how many notices that was.
///
/// ```ignore
/// frus::licenses::add_all(include_str!("../assets/licenses.txt"));
/// ```
pub fn add_all(generated: &str) -> usize {
    let notices = parse(generated);
    let count = notices.len();
    if let Ok(mut guard) = REGISTRY.lock() {
        guard.extend(notices);
    }
    count
}

/// Everything registered, in the order it was registered.
pub fn all() -> Vec<LicenseNotice> {
    REGISTRY
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Forgets everything registered. For a test, and for an application that rebuilds its
/// list — registering twice would otherwise show every licence twice.
pub fn clear() {
    if let Ok(mut guard) = REGISTRY.lock() {
        guard.clear();
    }
}

/// Reads the generator's format: a header, then one entry per notice.
///
/// ```text
/// frus-licenses 1
/// @ <byte length of the text>
/// - <package> <version>
/// .
/// <exactly that many bytes>
/// ```
///
/// The length prefix is what makes it safe: a licence text can contain any line at all,
/// including one that looks like a separator, and several of them do.
pub fn parse(generated: &str) -> Vec<LicenseNotice> {
    /// The next line, and what follows it.
    fn line<'a>(rest: &mut &'a str) -> Option<&'a str> {
        if rest.is_empty() {
            return None;
        }
        let (head, tail) = match rest.find('\n') {
            Some(at) => (&rest[..at], &rest[at + 1..]),
            None => (*rest, ""),
        };
        *rest = tail;
        Some(head)
    }

    let mut rest = generated;
    let mut notices = Vec::new();
    while let Some(head) = line(&mut rest) {
        // The header, comments and blank lines are skipped: an entry starts at `@`.
        let Some(length) = head
            .strip_prefix("@ ")
            .and_then(|n| n.trim().parse::<usize>().ok())
        else {
            continue;
        };
        let mut packages = Vec::new();
        while let Some(head) = line(&mut rest) {
            if head == "." {
                break;
            }
            if let Some(entry) = head.strip_prefix("- ") {
                let mut parts = entry.split_whitespace();
                if let (Some(name), Some(version)) = (parts.next(), parts.next()) {
                    packages.push(Package::new(name, version));
                }
            }
        }
        // A truncated file stops the walk rather than panicking: what was read is still
        // worth showing, and a page missing its last entry is better than no page.
        if rest.len() < length || !rest.is_char_boundary(length) {
            break;
        }
        let (text, tail) = rest.split_at(length);
        rest = tail.strip_prefix('\n').unwrap_or(tail);
        notices.push(LicenseNotice {
            packages,
            text: text.to_string(),
        });
    }
    notices
}

/// The packages of a set of notices, each with the notices covering it: the list the page
/// shows, which is the transpose of what the generator writes.
///
/// The generator groups by text because that is what deduplicates; a reader looks for a
/// package. Sorted by name, case-insensitively, because that is the order somebody
/// looking for `wgpu` expects to find it in.
pub fn by_package(notices: &[LicenseNotice]) -> Vec<(Package, Vec<usize>)> {
    let mut out: Vec<(Package, Vec<usize>)> = Vec::new();
    for (index, notice) in notices.iter().enumerate() {
        for package in &notice.packages {
            match out.iter_mut().find(|(p, _)| p == package) {
                Some((_, list)) => list.push(index),
                None => out.push((package.clone(), vec![index])),
            }
        }
    }
    out.sort_by(|(a, _), (b, _)| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.version.cmp(&b.version))
    });
    out
}

/// The gap between two paragraphs of a licence.
const PARAGRAPH_GAP: f32 = 12.0;
/// The size licence text is set at: smaller than body text, because there is a great deal
/// of it and nobody reads it as prose.
const LICENCE_SIZE: f32 = 13.0;

/// A licence's paragraphs, re-flowed.
///
/// Licence files are hard-wrapped at seventy-odd columns for a terminal, which on a phone
/// is a paragraph with a ragged edge or one that runs off the side. The lines of a
/// paragraph are joined and left to wrap at the width they are given — the same thing the
/// reference does with a `LicenseParagraph`, and for the same reason.
fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|block| {
            block
                .split('\n')
                .map(str::trim_end)
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
        })
        .filter(|block| !block.is_empty())
        .collect()
}

/// **The licences of everything an application links**, as a page.
///
/// A list of packages, each with the licences covering it; selecting one shows the text.
/// The selection is the **application's**, like every other piece of state here, so the
/// page fits a route as easily as a panel: `on_select(Some(i))` on a row, and
/// `on_select(None)` from the way back.
///
/// It does not scroll itself, and it does not guess how tall it is: put it in a
/// [`crate::SingleChildScrollView`], or in a scaffold's scrolling body, which is where a page
/// belongs anyway.
///
/// ```ignore
/// LicensePage::new(app.licence_open, Msg::OpenLicence)
///     .application("Tasks")
///     .version("1.4.0")
///     .legalese("© 2026 Someone")
///     .build()
/// ```
pub struct LicensePage<Msg> {
    notices: Vec<LicenseNotice>,
    selected: Option<usize>,
    on_select: Option<Rc<dyn Fn(Option<usize>) -> Msg>>,
    application: Option<String>,
    version: Option<String>,
    legalese: Option<String>,
    empty: Option<String>,
    back_label: Option<String>,
    text_style: Option<TextStyle>,
    width: Dimension,
    flex_grow: f32,
}

impl<Msg: Clone + 'static> LicensePage<Msg> {
    /// A page over everything [`add_all`] has registered. `selected` is which package is
    /// open — an index into the page's own list — and `on_select` carries the row that was
    /// touched, or `None` for the way back.
    pub fn new(
        selected: Option<usize>,
        on_select: impl Fn(Option<usize>) -> Msg + 'static,
    ) -> Self {
        Self {
            notices: all(),
            selected,
            on_select: Some(Rc::new(on_select)),
            application: None,
            version: None,
            legalese: None,
            empty: None,
            back_label: None,
            text_style: None,
            width: Dimension::Auto,
            flex_grow: 0.0,
        }
    }

    /// A page over notices of the caller's own, instead of the registry's.
    #[must_use]
    pub fn notices(mut self, notices: Vec<LicenseNotice>) -> Self {
        self.notices = notices;
        self
    }

    /// The application's name, at the head of the page.
    #[must_use]
    pub fn application(mut self, name: impl Into<String>) -> Self {
        self.application = Some(name.into());
        self
    }

    /// Its version, under the name.
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// The application's own legal line — a copyright, usually.
    #[must_use]
    pub fn legalese(mut self, legalese: impl Into<String>) -> Self {
        self.legalese = Some(legalese.into());
        self
    }

    /// What the page says when **nothing** is registered. There is a default, and it says
    /// what to do about it.
    #[must_use]
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty = Some(message.into());
        self
    }

    /// The label on the way back from a licence, for an application in another language.
    #[must_use]
    pub fn back_label(mut self, label: impl Into<String>) -> Self {
        self.back_label = Some(label.into());
        self
    }

    /// The type the licence text is set in.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// An explicit width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Dimension::Length(width);
        self
    }

    /// The share of a parent flex's spare room the page takes.
    #[must_use]
    pub fn flex(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Composes the page. Deferred until the theme is known, because everything on it is
    /// text and every piece of that text takes its size from the theme.
    #[must_use]
    pub fn build(self) -> Box<dyn Widget<Msg>> {
        let LicensePage {
            notices,
            selected,
            on_select,
            application,
            version,
            legalese,
            empty,
            back_label,
            text_style,
            width,
            flex_grow,
        } = self;
        Box::new(crate::ThemeBuilder::boxed(move |theme: &Theme| {
            let scale = crate::theme::type_scale(Some(theme));
            let packages = by_package(&notices);
            let mut column = crate::Flex::column()
                .align(Align::Stretch)
                .gap(8.0)
                .flex(flex_grow);
            if let Dimension::Length(w) = width {
                column = column.width(w);
            }

            // The head of the page: who this is, which the licences are *of*.
            if let Some(name) = &application {
                column = column.child(crate::Text::styled(name.clone(), scale.title_large));
            }
            if let Some(version) = &version {
                column = column.child(
                    crate::Text::styled(version.clone(), scale.body_medium).color(theme.muted),
                );
            }
            if let Some(legalese) = &legalese {
                column = column.child(
                    crate::Text::styled(legalese.clone(), scale.body_small)
                        .color(theme.muted)
                        .wrap(),
                );
            }
            if application.is_some() || version.is_some() || legalese.is_some() {
                column = column.child(crate::Divider::new());
            }

            // Nothing registered: the page says so, and says what to do. An application
            // that has forgotten this obligation is exactly the one that will not notice
            // an empty list.
            if packages.is_empty() {
                let message = empty.clone().unwrap_or_else(|| {
                    "No licences are registered. Generate them from the dependency graph \
                     and register them at startup: see scripts/gen_licenses.py and \
                     licenses::add_all."
                        .to_string()
                });
                return Box::new(
                    crate::Flex::column()
                        .align(Align::Stretch)
                        .gap(8.0)
                        .child(column)
                        .child(
                            crate::Text::styled(message, scale.body_medium)
                                .color(theme.error)
                                .wrap(),
                        ),
                );
            }

            match selected.and_then(|index| packages.get(index)) {
                // One package: its name, and the whole of every licence covering it.
                Some((package, covering)) => {
                    let back = back_label.clone().unwrap_or_else(|| "Back".to_string());
                    if let Some(on_select) = &on_select {
                        let on_select = Rc::clone(on_select);
                        column = column.child(
                            crate::Flex::row().child(
                                crate::Button::new(back)
                                    .variant(crate::Variant::Text)
                                    .on_press(on_select(None)),
                            ),
                        );
                    }
                    column = column.child(crate::Text::styled(package.label(), scale.title_medium));
                    let body = text_style.unwrap_or(TextStyle {
                        size: Some(LICENCE_SIZE),
                        ..scale.body_small
                    });
                    for index in covering {
                        let Some(notice) = notices.get(*index) else {
                            continue;
                        };
                        let mut block = crate::Flex::column()
                            .align(Align::Stretch)
                            .gap(PARAGRAPH_GAP);
                        for paragraph in paragraphs(&notice.text) {
                            block = block.child(crate::Text::styled(paragraph, body).wrap());
                        }
                        column = column.child(crate::Divider::new()).child(block);
                    }
                }
                // The list: every package, and how many licences cover it.
                None => {
                    for (index, (package, covering)) in packages.iter().enumerate() {
                        let mut tile = crate::ListTile::new()
                            .title(package.label())
                            .subtitle(if covering.len() == 1 {
                                "1 licence".to_string()
                            } else {
                                format!("{} licences", covering.len())
                            })
                            .dense();
                        if let Some(on_select) = &on_select {
                            tile = tile.on_tap(on_select(Some(index)));
                        }
                        column = column.child(tile);
                    }
                }
            }
            Box::new(column)
        }))
    }
}

/// **The box that says what this application is**, with a way through to the licences.
///
/// A name, a version, an icon, a legal line, and two buttons: one to the licences, one to
/// close. It is an [`crate::AlertDialog`] underneath, so everything an alert dialog can be
/// told, this can be told too.
pub struct AboutDialog<Msg> {
    open: bool,
    application: Option<String>,
    version: Option<String>,
    legalese: Option<String>,
    icon: Option<Box<dyn Widget<Msg>>>,
    licences_label: Option<String>,
    close_label: Option<String>,
    on_licences: Option<Msg>,
    on_close: Option<Msg>,
}

impl<Msg: Clone + 'static> AboutDialog<Msg> {
    /// A dialog, shown when `open`.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            application: None,
            version: None,
            legalese: None,
            icon: None,
            licences_label: None,
            close_label: None,
            on_licences: None,
            on_close: None,
        }
    }

    /// The application's name.
    #[must_use]
    pub fn application(mut self, name: impl Into<String>) -> Self {
        self.application = Some(name.into());
        self
    }

    /// Its version.
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// The legal line — a copyright, a notice of the application's own.
    #[must_use]
    pub fn legalese(mut self, legalese: impl Into<String>) -> Self {
        self.legalese = Some(legalese.into());
        self
    }

    /// Its mark, beside the name.
    #[must_use]
    pub fn icon(mut self, icon: impl Widget<Msg> + 'static) -> Self {
        self.icon = Some(Box::new(icon));
        self
    }

    /// The button through to the licences, and what it says.
    #[must_use]
    pub fn on_licences(mut self, message: Msg) -> Self {
        self.on_licences = Some(message);
        self
    }

    /// Its label, for an application in another language.
    #[must_use]
    pub fn licences_label(mut self, label: impl Into<String>) -> Self {
        self.licences_label = Some(label.into());
        self
    }

    /// The button that closes it — which is also what a tap outside sends.
    #[must_use]
    pub fn on_close(mut self, message: Msg) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Its label.
    #[must_use]
    pub fn close_label(mut self, label: impl Into<String>) -> Self {
        self.close_label = Some(label.into());
        self
    }

    /// Sets the screen behind it and finalises the dialog.
    #[must_use]
    pub fn body(self, body: impl Widget<Msg> + 'static) -> crate::Dialog<Msg> {
        let AboutDialog {
            open,
            application,
            version,
            legalese,
            icon,
            licences_label,
            close_label,
            on_licences,
            on_close,
        } = self;
        let mut dialog = crate::AlertDialog::new(open);
        if let Some(icon) = icon {
            dialog = dialog.icon(icon);
        }
        if let Some(name) = application {
            dialog = dialog.title(name);
        }
        // The version and the legal line are one block of content: two texts in a column,
        // rather than a title's subtitle, because the dialog has one title and it is the
        // application's name.
        let content: Vec<String> = [version, legalese].into_iter().flatten().collect();
        if !content.is_empty() {
            dialog = dialog.content(content.join("\n"));
        }
        if let Some(message) = on_licences {
            dialog = dialog.action(
                crate::Button::new(licences_label.unwrap_or_else(|| "Licences".to_string()))
                    .variant(crate::Variant::Text)
                    .on_press(message),
            );
        }
        if let Some(message) = on_close {
            dialog = dialog.on_dismiss(message.clone()).action(
                crate::Button::new(close_label.unwrap_or_else(|| "Close".to_string()))
                    .variant(crate::Variant::Text)
                    .on_press(message),
            );
        }
        dialog.body(body)
    }
}

/// **The row that opens the about box**, for a settings screen or a drawer.
///
/// One line over [`crate::ListTile`], which is the point: the reference has this widget
/// because every application writes the same row, and every application gets it slightly
/// differently.
pub struct AboutListTile<Msg> {
    application: Option<String>,
    icon: Option<crate::IconData>,
    title: Option<String>,
    subtitle: Option<String>,
    on_tap: Option<Msg>,
}

impl<Msg: Clone + 'static> AboutListTile<Msg> {
    /// A row that emits `on_tap` — which an application turns into an open about box.
    pub fn new(on_tap: Msg) -> Self {
        Self {
            application: None,
            icon: Some(crate::Icons::INFO),
            title: None,
            subtitle: None,
            on_tap: Some(on_tap),
        }
    }

    /// The application's name, which the row's default title is *about*.
    #[must_use]
    pub fn application(mut self, name: impl Into<String>) -> Self {
        self.application = Some(name.into());
        self
    }

    /// A title of the caller's own, over the default `About <name>`.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// A second line under it — a version, usually.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The glyph at its leading edge, or `None` for a row with no icon at all.
    #[must_use]
    pub fn icon(mut self, icon: Option<crate::IconData>) -> Self {
        self.icon = icon;
        self
    }

    /// Composes the row.
    #[must_use]
    pub fn build(self) -> crate::ListTile<Msg> {
        let title = self.title.unwrap_or_else(|| match &self.application {
            Some(name) => format!("About {name}"),
            None => "About".to_string(),
        });
        let mut tile = crate::ListTile::new().title(title);
        if let Some(icon) = self.icon {
            tile = tile.leading(crate::Icon::new(icon));
        }
        if let Some(subtitle) = self.subtitle {
            tile = tile.subtitle(subtitle);
        }
        if let Some(message) = self.on_tap {
            tile = tile.on_tap(message);
        }
        tile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Open(Option<usize>),
    }

    /// A file in the generator's format, with **a licence text that contains the format's
    /// own punctuation** — a line starting with `@`, a lone full stop, a line that looks
    /// like a package. Several real licences have lines like these, which is why the
    /// entries are length-prefixed rather than delimited.
    const AWKWARD: &str = "frus-licenses 1\n# a comment\n\n@ 52\n- one 1.0.0\n- two 0.2.1\n.\n@ 9\nnot a header\n.\n- not a package\nstill the licence\n@ 4\n- three 0.1.0\n.\nlast\n";

    #[test]
    fn the_format_survives_a_licence_that_looks_like_the_format() {
        let notices = parse(AWKWARD);
        assert_eq!(notices.len(), 2, "two entries: {notices:#?}");
        assert_eq!(
            notices[0].packages,
            vec![Package::new("one", "1.0.0"), Package::new("two", "0.2.1")]
        );
        assert!(
            notices[0].text.starts_with("@ 9\nnot a header"),
            "the text is taken by length, whatever it contains: {:?}",
            notices[0].text
        );
        assert_eq!(notices[1].text, "last");
    }

    /// A file cut off mid-entry gives back what was read rather than panicking: a page
    /// missing its last notice is better than no page, and better than a crash on a
    /// screen somebody opened to comply with a licence.
    #[test]
    fn a_truncated_file_stops_where_it_stops() {
        let cut = "frus-licenses 1\n@ 3\n- one 1.0.0\n.\nMIT\n@ 4000\n- two 1.0.0\n.\nshort";
        let notices = parse(cut);
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].text, "MIT");
    }

    /// The generator groups by text; a reader looks for a package. The page transposes,
    /// and sorts the way somebody looking for `wgpu` expects.
    #[test]
    fn the_page_lists_packages_and_not_texts() {
        let notices = vec![
            LicenseNotice {
                packages: vec![
                    Package::new("Zebra", "1.0.0"),
                    Package::new("apple", "2.0.0"),
                ],
                text: "MIT".to_string(),
            },
            LicenseNotice {
                packages: vec![Package::new("apple", "2.0.0")],
                text: "Apache".to_string(),
            },
        ];
        let listed = by_package(&notices);
        let names: Vec<&str> = listed.iter().map(|(p, _)| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["apple", "Zebra"],
            "sorted, and case is not an order"
        );
        // A package under two licences is one row that says two.
        assert_eq!(listed[0].1.len(), 2);
        assert_eq!(listed[1].1.len(), 1);
    }

    /// Licence files are hard-wrapped for a terminal. A paragraph is re-flowed so that it
    /// wraps to the width it is given instead of to somebody else's eighty columns.
    #[test]
    fn paragraphs_are_reflowed_and_blank_blocks_dropped() {
        let text =
            "Permission is hereby\ngranted, free of charge\n\n\nTHE SOFTWARE IS\nPROVIDED AS IS";
        assert_eq!(
            paragraphs(text),
            [
                "Permission is hereby granted, free of charge",
                "THE SOFTWARE IS PROVIDED AS IS"
            ]
        );
    }

    /// The registry: what an application registers is what the page shows. One test
    /// touches it, because it is process-wide and tests share a process.
    #[test]
    fn the_registry_holds_what_was_registered() {
        clear();
        assert!(all().is_empty());
        let added = add_all("frus-licenses 1\n@ 3\n- one 1.0.0\n.\nMIT\n");
        assert_eq!(added, 1);
        add(LicenseNotice {
            packages: vec![Package::new("two", "1.0.0")],
            text: "BSD".to_string(),
        });
        assert_eq!(all().len(), 2);
        clear();
        assert!(all().is_empty(), "and an application can start again");
    }

    /// **The empty page says so.** The obligation this widget exists for is a silent one,
    /// and a page that says nothing at all where the licences should be is exactly how it
    /// stays silent.
    #[test]
    fn a_page_with_nothing_registered_says_what_to_do() {
        let page: Box<dyn Widget<Msg>> = LicensePage::new(None, Msg::Open)
            .notices(Vec::new())
            .application("Something")
            .build();
        let words = text_of(page.as_ref());
        assert!(
            words.iter().any(|line| line.contains("No licences")),
            "the page says the list is empty: {words:#?}"
        );
        assert!(
            words.iter().any(|line| line.contains("gen_licenses")),
            "and what to do about it: {words:#?}"
        );
    }

    /// The list shows a row per package, with how many licences cover it; opening one
    /// shows the text itself.
    #[test]
    fn the_list_opens_onto_the_licence_itself() {
        let notices = vec![LicenseNotice {
            packages: vec![Package::new("wgpu", "22.1.0")],
            text: "Apache License\n\nSome terms.".to_string(),
        }];
        let list = LicensePage::new(None, Msg::Open)
            .notices(notices.clone())
            .build();
        let words = text_of(list.as_ref());
        assert!(words.iter().any(|w| w == "wgpu 22.1.0"), "{words:#?}");
        assert!(words.iter().any(|w| w == "1 licence"), "{words:#?}");
        assert!(
            !words.iter().any(|w| w.contains("Some terms")),
            "the list is a list, not four hundred licences at once: {words:#?}"
        );

        let open = LicensePage::new(Some(0), Msg::Open)
            .notices(notices)
            .build();
        let words = text_of(open.as_ref());
        assert!(words.iter().any(|w| w == "Some terms."), "{words:#?}");
        assert!(
            words.iter().any(|w| w == "Back"),
            "and a way back: {words:#?}"
        );
    }

    /// The row that opens the box, and the box itself, say what they are about.
    #[test]
    fn the_about_row_and_box_name_the_application() {
        let tile = AboutListTile::new(Msg::Open(None))
            .application("Tasks")
            .subtitle("Version 1.0")
            .build();
        let words = text_of(&tile);
        assert!(words.iter().any(|w| w == "About Tasks"), "{words:#?}");
        assert!(words.iter().any(|w| w == "Version 1.0"), "{words:#?}");

        let dialog = AboutDialog::new(true)
            .application("Tasks")
            .version("Version 1.0")
            .legalese("© somebody")
            .on_licences(Msg::Open(Some(0)))
            .on_close(Msg::Open(None))
            .body(crate::Container::new().width(400.0).height(400.0));
        let words = text_of(&dialog);
        for expected in ["Tasks", "Licences", "Close"] {
            assert!(
                words.iter().any(|w| w.contains(expected)),
                "the box says {expected}: {words:#?}"
            );
        }
    }

    /// Every string a widget paints, in paint order.
    fn text_of(root: &dyn Widget<Msg>) -> Vec<String> {
        let size = Size::new(420.0, 900.0);
        let ui = build_ui(root, size, &Runtime::default(), &Theme::dark());
        ui.scene()
            .primitives()
            .iter()
            .filter_map(|p| match p {
                frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}
