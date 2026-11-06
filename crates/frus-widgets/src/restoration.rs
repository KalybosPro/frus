// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **State restoration**: what an application was showing comes back after the system ended
//! its process (milestone 753).
//!
//! A phone ends an application it has put in the background whenever it wants the memory
//! back. The person does not see that happen: they come back from the recent apps and expect
//! the page they left, scrolled where they left it, with what they had typed still there.
//! The system keeps a small parcel of data for exactly that — Android's saved instance state,
//! a browser tab's session storage — and hands it back to the new process.
//!
//! This module is what goes in that parcel:
//!
//! - a [`RestorationBucket`]: a tree of named values, each value a short text, each bucket
//!   named by a **restoration id**. It is written to bytes with
//!   [`RestorationBucket::encode`] and read back with [`RestorationBucket::decode`];
//! - **restorable values**, registered under an id by what owns them: a component's hook
//!   ([`BuildContext::use_restorable_state`],
//!   [`BuildContext::use_restorable_text_controller`]), a [`State`](crate::State) that names
//!   a [`restoration_id`](crate::State::restoration_id), a scroll area or a text field given
//!   a `restoration_id`, and the router, which keeps its stack of pages;
//! - **scopes**: [`restoration_scope`] puts what is built inside it in a bucket of its own,
//!   so that two parts of an application may both use the id `"query"`. The router gives
//!   every page a scope, so ids only have to be unique within a page.
//!
//! Nothing is kept until the application asks for it, with a **restoration scope id**
//! (`FrusApp::restoration_scope_id`): the parcel is the application's to opt into, as a
//! page that restores itself must be written for it — a value read back from yesterday's
//! process has to make sense in today's.
//!
//! ```
//! use frus_widgets::{column, restoration_scope, BuildContext, ListView, Text, TextField, Widget};
//!
//! fn compose(cx: &BuildContext) -> Box<dyn Widget> {
//!     // The draft, and the count of sends, come back after the process was ended.
//!     let draft = cx.use_restorable_text_controller("draft", "");
//!     let sent = cx.use_restorable_state("sent", || 0u32);
//!     Box::new(column![
//!         TextField::new("").label("Message").controller(&draft).restoration_id("field"),
//!         Text::new(format!("{} sent", sent.get())),
//!         // And how far down the list was scrolled.
//!         ListView::new(100, 48.0, |i| Text::new(format!("Row {i}"))).restoration_id("rows"),
//!     ])
//! }
//!
//! // A part of the page with ids of its own.
//! let settings = restoration_scope("settings", |cx| {
//!     let dark = cx.use_restorable_state("dark", || false);
//!     Box::new(Text::new(format!("dark: {}", dark.get())))
//! });
//! # let _ = settings;
//! ```
//!
//! What is restored is **state**, not data: a draft, a scroll position, which tab is open. A
//! document the person saved belongs in a file or a database, which survive far more than a
//! process does. And the parcel is small — the system refuses one that grows past a few
//! hundred kilobytes — so a list's items are fetched again, not kept here.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use crate::component::{BuildContext, Component, UseState};
use crate::editing::TextEditingController;
use crate::interaction::WidgetId;
use crate::runtime::{Edit, Runtime};
use crate::widget::Widget;

// ---------------------------------------------------------------------------------------
// The bucket
// ---------------------------------------------------------------------------------------

/// A tree of restorable values: each value a text under an id, each child bucket under an
/// id of its own. What is written into the system's parcel, and read back from it.
///
/// ```
/// use frus_widgets::RestorationBucket;
///
/// let mut root = RestorationBucket::new();
/// root.set_value("tab", "2");
/// root.child_mut("settings").set_value("dark", "true");
///
/// let bytes = root.encode();
/// let back = RestorationBucket::decode(&bytes).expect("what was written reads back");
/// assert_eq!(back.value("tab"), Some("2"));
/// assert_eq!(back.child("settings").and_then(|s| s.value("dark")), Some("true"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RestorationBucket {
    values: BTreeMap<String, String>,
    children: BTreeMap<String, RestorationBucket>,
}

/// The first line of the bytes: what they are, and in which version of the format.
const HEADER: &str = "frus-restoration 1";

impl RestorationBucket {
    /// An empty bucket.
    pub fn new() -> Self {
        Self::default()
    }

    /// The value kept under `id`, if there is one.
    pub fn value(&self, id: &str) -> Option<&str> {
        self.values.get(id).map(String::as_str)
    }

    /// Keeps `value` under `id`, replacing what was there.
    pub fn set_value(&mut self, id: impl Into<String>, value: impl Into<String>) {
        self.values.insert(id.into(), value.into());
    }

    /// Takes the value kept under `id` out of the bucket.
    pub fn remove_value(&mut self, id: &str) -> Option<String> {
        self.values.remove(id)
    }

    /// The child bucket named `id`, if there is one.
    pub fn child(&self, id: &str) -> Option<&RestorationBucket> {
        self.children.get(id)
    }

    /// The child bucket named `id`, made empty if there was none.
    pub fn child_mut(&mut self, id: impl Into<String>) -> &mut RestorationBucket {
        self.children.entry(id.into()).or_default()
    }

    /// Takes the child bucket named `id` out of this one.
    pub fn remove_child(&mut self, id: &str) -> Option<RestorationBucket> {
        self.children.remove(id)
    }

    /// The values of this bucket, by id. Not its children's.
    pub fn values(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// The child buckets, by id.
    pub fn children(&self) -> impl Iterator<Item = (&str, &RestorationBucket)> {
        self.children.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Whether the bucket holds nothing, in itself or below.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.children.values().all(RestorationBucket::is_empty)
    }

    /// The value at `path`: the buckets named by all but its last id, then the value named
    /// by the last.
    pub fn get(&self, path: &[String]) -> Option<&str> {
        let (id, scope) = path.split_last()?;
        let mut bucket = self;
        for name in scope {
            bucket = bucket.children.get(name)?;
        }
        bucket.value(id)
    }

    /// Keeps `value` at `path`, making the buckets on the way.
    pub fn insert(&mut self, path: &[String], value: impl Into<String>) {
        let Some((id, scope)) = path.split_last() else {
            return;
        };
        let mut bucket = self;
        for name in scope {
            bucket = bucket.child_mut(name.clone());
        }
        bucket.set_value(id.clone(), value);
    }

    /// Takes the value at `path` out of the tree; buckets it leaves empty go with it.
    pub fn take(&mut self, path: &[String]) -> Option<String> {
        let (id, scope) = path.split_last()?;
        match scope.split_first() {
            None => self.values.remove(id),
            Some((first, rest)) => {
                let child = self.children.get_mut(first)?;
                let mut inner: Vec<String> = rest.to_vec();
                inner.push(id.clone());
                let taken = child.take(&inner);
                if child.is_empty() {
                    self.children.remove(first);
                }
                taken
            }
        }
    }

    /// Removes the bucket at `scope`, and all it holds. An empty `scope` empties this one.
    pub fn remove_scope(&mut self, scope: &[String]) {
        match scope.split_first() {
            None => *self = Self::default(),
            Some((first, [])) => {
                self.children.remove(first);
            }
            Some((first, rest)) => {
                if let Some(child) = self.children.get_mut(first) {
                    child.remove_scope(rest);
                    if child.is_empty() {
                        self.children.remove(first);
                    }
                }
            }
        }
    }

    /// The bucket as bytes: a line naming the format, then a line per value — its path and
    /// its text, tabs and line breaks escaped. Text, so that a browser's storage can hold it
    /// as it is.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = String::from(HEADER);
        self.encode_into(&mut Vec::new(), &mut out);
        out.into_bytes()
    }

    fn encode_into(&self, path: &mut Vec<String>, out: &mut String) {
        for (id, value) in &self.values {
            out.push('\n');
            for name in path.iter() {
                out.push_str(&escape(name));
                out.push('/');
            }
            out.push_str(&escape(id));
            out.push('\t');
            out.push_str(&escape(value));
        }
        for (name, child) in &self.children {
            path.push(name.clone());
            child.encode_into(path, out);
            path.pop();
        }
    }

    /// Reads back what [`encode`](Self::encode) wrote. `None` for bytes that are not a
    /// bucket — another format, another version, or damaged: a page starts afresh rather
    /// than from a guess.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut lines = text.split('\n');
        if lines.next()? != HEADER {
            return None;
        }
        let mut bucket = Self::default();
        for line in lines {
            let (path, value) = line.split_once('\t')?;
            let path = split_path(path)?;
            if path.is_empty() {
                return None;
            }
            bucket.insert(&path, unescape(value)?);
        }
        Some(bucket)
    }
}

/// An id or a value on one line of the bytes: `\`, `/`, tabs and line breaks escaped.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            '\\' => '\\',
            '/' => '/',
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            _ => return None,
        });
    }
    Some(out)
}

/// A path's ids, split at the slashes that are not escaped.
fn split_path(text: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut part = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                part.push('\\');
                part.push(chars.next()?);
            }
            '/' => parts.push(unescape(&std::mem::take(&mut part))?),
            c => part.push(c),
        }
    }
    parts.push(unescape(&part)?);
    Some(parts)
}

// ---------------------------------------------------------------------------------------
// Restorable values
// ---------------------------------------------------------------------------------------

/// A value that can be kept in a [`RestorationBucket`]: written as a short text, and read
/// back from it.
///
/// Numbers, `bool`, `char`, `String` and `Option`s of them are; a type of the application's
/// own implements the two methods — or, with the `serde` feature, any serde type goes
/// through `use_restorable_serde` on [`BuildContext`] (a method that only exists with the
/// feature, so it is named here rather than linked: the link would not resolve without it).
///
/// ```
/// use frus_widgets::RestorableValue;
///
/// #[derive(Clone, Copy, PartialEq, Debug)]
/// enum Tab { Inbox, Sent }
///
/// impl RestorableValue for Tab {
///     fn to_restoration(&self) -> String {
///         match self { Tab::Inbox => "inbox", Tab::Sent => "sent" }.to_string()
///     }
///     fn from_restoration(saved: &str) -> Option<Self> {
///         match saved { "inbox" => Some(Tab::Inbox), "sent" => Some(Tab::Sent), _ => None }
///     }
/// }
///
/// assert_eq!(Tab::from_restoration(&Tab::Sent.to_restoration()), Some(Tab::Sent));
/// ```
pub trait RestorableValue: Sized {
    /// The value as text.
    fn to_restoration(&self) -> String;

    /// The value back from its text. `None` when the text does not make one — written by
    /// another version of the application, say — and the value then starts afresh.
    fn from_restoration(saved: &str) -> Option<Self>;
}

macro_rules! restorable_by_display {
    ($($ty:ty),* $(,)?) => {$(
        impl RestorableValue for $ty {
            fn to_restoration(&self) -> String {
                self.to_string()
            }
            fn from_restoration(saved: &str) -> Option<Self> {
                saved.parse().ok()
            }
        }
    )*};
}

restorable_by_display!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char, String,
);

impl<T: RestorableValue> RestorableValue for Option<T> {
    fn to_restoration(&self) -> String {
        match self {
            None => "-".to_string(),
            Some(value) => format!("+{}", value.to_restoration()),
        }
    }

    fn from_restoration(saved: &str) -> Option<Self> {
        if saved == "-" {
            return Some(None);
        }
        saved
            .strip_prefix('+')
            .and_then(T::from_restoration)
            .map(Some)
    }
}

// ---------------------------------------------------------------------------------------
// Scopes
// ---------------------------------------------------------------------------------------

/// The ids of the buckets a value is kept in, from the root down.
pub(crate) type Scope = Rc<[String]>;

thread_local! {
    /// The scope what is being built belongs to.
    static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}

// Non-generic and out of line, like the component layer's other thread-locals: a hot patch
// must reach the one copy frus holds (milestone 669).

/// The scope of what is being built right now: the root outside any.
#[inline(never)]
pub(crate) fn current_scope() -> Scope {
    SCOPE.with(|scope| {
        scope
            .borrow()
            .clone()
            .unwrap_or_else(|| Rc::from(Vec::new()))
    })
}

/// Builds what follows in `scope`, until the guard is dropped.
#[inline(never)]
pub(crate) fn enter_scope(scope: Scope) -> ScopeGuard {
    ScopeGuard(SCOPE.with(|current| current.borrow_mut().replace(scope)))
}

/// Puts the scope that was current back when dropped.
pub(crate) struct ScopeGuard(Option<Scope>);

impl Drop for ScopeGuard {
    #[inline(never)]
    fn drop(&mut self) {
        let previous = self.0.take();
        SCOPE.with(|current| *current.borrow_mut() = previous);
    }
}

/// `scope` with the bucket `id` below it.
pub(crate) fn scope_below(scope: &[String], id: &str) -> Scope {
    let mut path = scope.to_vec();
    path.push(id.to_string());
    Rc::from(path)
}

/// The path of the value `id` in the scope current now.
pub(crate) fn path_here(id: &str) -> Vec<String> {
    scope_below(&current_scope(), id).to_vec()
}

/// A part of the page with restoration ids of its own: what `build` builds — its hooks, its
/// fields, its scroll areas, the components it makes — keeps its values in the bucket `id`,
/// below the bucket of what is around it.
///
/// Two scopes may then both hold a value called `"query"`. The router gives each page a
/// scope of its own already; this is for the parts of one page — the tabs of a tab bar, the
/// panes of a split view.
///
/// ```
/// use frus_widgets::{column, restoration_scope, BuildContext, TextField, Widget};
///
/// fn search(cx: &BuildContext) -> Box<dyn Widget> {
///     let query = cx.use_restorable_text_controller("query", "");
///     Box::new(TextField::new("").controller(&query))
/// }
///
/// let panes = column![
///     restoration_scope("people", search),
///     restoration_scope("places", search),
/// ];
/// # let _ = panes;
/// ```
pub fn restoration_scope(
    id: impl Into<String>,
    build: impl Fn(&BuildContext) -> Box<dyn Widget> + 'static,
) -> Component {
    let scope = scope_below(&current_scope(), &id.into());
    let _inside = enter_scope(scope);
    Component::stateless(build)
}

// ---------------------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------------------

/// What a restorable value reads as, now.
pub(crate) enum Reading {
    /// Its owner has left the tree: the value goes with it.
    Gone,
    /// Nothing to keep at the moment.
    Nothing,
    /// The value, as text.
    Value(String),
}

/// Where a registered value is read from.
enum Source {
    /// A value something holds: a hook, a state, the router.
    Value(Box<dyn Fn() -> Reading>),
    /// A scroll area's offset, from the runtime.
    Scroll(WidgetId),
    /// A field's caret and selection, from the runtime.
    Edit(WidgetId),
    /// The page a paged view shows, from the runtime (milestone 756).
    Page(WidgetId),
}

struct Live {
    source: Source,
    /// The last value read: a widget's own state goes when the widget leaves the frame, and
    /// what it was is kept until its scope is.
    last: Option<String>,
}

/// What was restored for a widget, waiting for the runtime to take it.
pub(crate) enum Pending {
    Scroll(WidgetId, (f32, f32)),
    Edit(WidgetId, Edit),
    /// A paged view's page, and the page the application asks for in the build that
    /// claimed it: that request then stands as already honoured, so the restored page is
    /// the one on show (milestone 756).
    Page {
        id: WidgetId,
        page: usize,
        requested: usize,
    },
}

/// **The restoration of one application**: whether it has asked for it, what the platform
/// handed back, and every restorable value registered since. Kept in the [`Runtime`], read
/// by the shell after each frame and written into the platform's parcel.
#[derive(Default)]
pub struct Restoration {
    /// The application's restoration scope id; `None` keeps nothing.
    scope_id: RefCell<Option<String>>,
    /// What the platform handed back and nothing has claimed yet.
    restored: RefCell<RestorationBucket>,
    /// Every value registered, by path.
    live: RefCell<BTreeMap<Vec<String>, Live>>,
    /// What was restored for widgets, for the runtime to take.
    pending: RefCell<Vec<Pending>>,
    /// Whether the router's stack is restored. Not in a browser, where the address and the
    /// history are what say which page is on show.
    keep_router: Cell<Option<bool>>,
}

impl Restoration {
    /// Turns restoration on, the application's values kept under `scope_id`.
    pub fn enable(&self, scope_id: impl Into<String>) {
        *self.scope_id.borrow_mut() = Some(scope_id.into());
    }

    /// Whether the application asked for restoration.
    pub fn is_enabled(&self) -> bool {
        self.scope_id.borrow().is_some()
    }

    /// Whether the router keeps its stack of pages here. On by default; a platform whose own
    /// history says which page is on show — a browser — turns it off.
    pub fn set_restores_router(&self, restores: bool) {
        self.keep_router.set(Some(restores));
    }

    pub(crate) fn restores_router(&self) -> bool {
        self.keep_router.get().unwrap_or(true)
    }

    /// Takes the bytes the platform handed back ([`Restoration::data`] of the process that
    /// was ended). Whether they held anything for this application: bytes of another format,
    /// or kept under another scope id, are ignored.
    pub fn restore(&self, bytes: &[u8]) -> bool {
        let Some(id) = self.scope_id.borrow().clone() else {
            return false;
        };
        let Some(mut root) = RestorationBucket::decode(bytes) else {
            log::warn!("the restoration data handed back was not readable; starting afresh");
            return false;
        };
        match root.remove_child(&id) {
            Some(bucket) => {
                *self.restored.borrow_mut() = bucket;
                true
            }
            None => false,
        }
    }

    /// What there is to keep, now: every registered value, and what was restored and not
    /// claimed yet — a page under another, whose fields were never built. In the
    /// application's own bucket, below the root.
    pub fn bucket(&self, runtime: &Runtime) -> RestorationBucket {
        let mut bucket = self.restored.borrow().clone();
        let mut live = self.live.borrow_mut();
        live.retain(|path, entry| {
            let now = match &entry.source {
                Source::Value(read) => match read() {
                    Reading::Gone => return false,
                    Reading::Nothing => None,
                    Reading::Value(value) => Some(value),
                },
                Source::Scroll(id) => runtime
                    .scroll
                    .get(id)
                    .map(|(x, y)| format!("{x},{y}"))
                    .or_else(|| entry.last.clone()),
                Source::Edit(id) => runtime
                    .edits
                    .get(id)
                    .map(write_edit)
                    .or_else(|| entry.last.clone()),
                Source::Page(id) => runtime
                    .page_shown
                    .get(id)
                    .map(usize::to_string)
                    .or_else(|| entry.last.clone()),
            };
            if let Some(value) = now {
                bucket.insert(path, value.clone());
                entry.last = Some(value);
            }
            true
        });
        bucket
    }

    /// The bytes to keep for the platform: the application's bucket under its scope id.
    /// `None` when restoration is off.
    pub fn data(&self, runtime: &Runtime) -> Option<Vec<u8>> {
        let id = self.scope_id.borrow().clone()?;
        let mut root = RestorationBucket::new();
        *root.child_mut(id) = self.bucket(runtime);
        Some(root.encode())
    }

    /// Takes what was restored at `path`, which is then the claimer's alone.
    pub(crate) fn claim(&self, path: &[String]) -> Option<String> {
        if !self.is_enabled() {
            return None;
        }
        self.restored.borrow_mut().take(path)
    }

    /// Keeps the value `read` reads at `path`, from now on.
    pub(crate) fn register(&self, path: Vec<String>, read: impl Fn() -> Reading + 'static) {
        if !self.is_enabled() {
            return;
        }
        self.live.borrow_mut().insert(
            path,
            Live {
                source: Source::Value(Box::new(read)),
                last: None,
            },
        );
    }

    /// A scroll area at `path` is the one called `id` this frame: its offset is kept, and
    /// the one restored for it, if any, is handed to the runtime.
    pub(crate) fn register_scroll(&self, path: Vec<String>, id: WidgetId) {
        if !self.is_enabled() {
            return;
        }
        if let Some(saved) = self.claim(&path) {
            if let Some(offset) = read_offset(&saved) {
                self.pending.borrow_mut().push(Pending::Scroll(id, offset));
            }
        }
        self.register_widget(path, Source::Scroll(id));
    }

    /// A field at `path` is the one called `id` this frame: its caret and selection are
    /// kept, and the ones restored for it handed to the runtime.
    pub(crate) fn register_edit(&self, path: Vec<String>, id: WidgetId) {
        if !self.is_enabled() {
            return;
        }
        if let Some(saved) = self.claim(&path) {
            if let Some(edit) = read_edit(&saved) {
                self.pending.borrow_mut().push(Pending::Edit(id, edit));
            }
        }
        self.register_widget(path, Source::Edit(id));
    }

    /// A paged view at `path` — a [`PageView`](crate::PageView), a
    /// [`ListWheel`](crate::ListWheel) — is the one called `id` this frame, asking for the
    /// page `requested`: the page it shows is kept, and the one restored for it, if any, is
    /// handed to the runtime (milestone 756).
    ///
    /// A **page**, not an offset: a phone turned on its side between the two processes
    /// has pages of another width, and the page is what the reader was on.
    pub(crate) fn register_page(&self, path: Vec<String>, id: WidgetId, requested: usize) {
        if !self.is_enabled() {
            return;
        }
        if let Some(saved) = self.claim(&path) {
            if let Ok(page) = saved.parse::<usize>() {
                self.pending.borrow_mut().push(Pending::Page {
                    id,
                    page,
                    requested,
                });
            }
        }
        self.register_widget(path, Source::Page(id));
    }

    fn register_widget(&self, path: Vec<String>, source: Source) {
        let mut live = self.live.borrow_mut();
        match live.get_mut(&path) {
            Some(entry) => entry.source = source,
            None => {
                live.insert(path, Live { source, last: None });
            }
        }
    }

    /// Forgets the bucket at `scope` and everything in it, registered or restored: a page
    /// that left the router's stack takes its values with it.
    pub(crate) fn forget_scope(&self, scope: &[String]) {
        self.live
            .borrow_mut()
            .retain(|path, _| !path.starts_with(scope));
        self.restored.borrow_mut().remove_scope(scope);
    }

    /// What was restored for widgets and not handed to the runtime yet, taken: the
    /// runtime's [`apply_restored`](Runtime::apply_restored) puts it in place.
    pub(crate) fn take_pending(&self) -> Vec<Pending> {
        std::mem::take(&mut *self.pending.borrow_mut())
    }
}

fn read_offset(saved: &str) -> Option<(f32, f32)> {
    let (x, y) = saved.split_once(',')?;
    let (x, y) = (x.parse::<f32>().ok()?, y.parse::<f32>().ok()?);
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// A caret and its selection's anchor: `cursor` or `cursor,anchor`, in characters.
fn write_edit(edit: &Edit) -> String {
    match edit.anchor {
        Some(anchor) => format!("{},{anchor}", edit.cursor),
        None => edit.cursor.to_string(),
    }
}

fn read_edit(saved: &str) -> Option<Edit> {
    let (cursor, anchor) = match saved.split_once(',') {
        Some((cursor, anchor)) => (cursor.parse().ok()?, Some(anchor.parse().ok()?)),
        None => (saved.parse().ok()?, None),
    };
    Some(Edit {
        cursor,
        anchor,
        ..Edit::default()
    })
}

// ---------------------------------------------------------------------------------------
// The hooks
// ---------------------------------------------------------------------------------------

impl BuildContext<'_> {
    /// Like [`use_state`](Self::use_state), and **restored**: the value is kept in the
    /// platform's parcel under `id`, and a process the system started again begins with it
    /// rather than with `init()`.
    ///
    /// `id` names the value within its scope — the page, or a [`restoration_scope`] — and
    /// must be unique there. Without restoration turned on for the application this is
    /// `use_state`.
    ///
    /// ```
    /// # use frus_widgets::{text, BuildContext, Widget};
    /// fn tabs(cx: &BuildContext) -> Box<dyn Widget> {
    ///     let tab = cx.use_restorable_state("tab", || 0usize);
    ///     Box::new(text(format!("tab {}", tab.get())))
    /// }
    /// ```
    pub fn use_restorable_state<T: RestorableValue + 'static>(
        &self,
        id: &str,
        init: impl FnOnce() -> T,
    ) -> UseState<T> {
        self.use_restorable_with(
            id,
            init,
            |value: &T| Some(value.to_restoration()),
            T::from_restoration,
        )
    }

    /// Like [`use_restorable_state`](Self::use_restorable_state), for any value serde can
    /// write: kept as JSON. Needs the `serde` feature.
    #[cfg(feature = "serde")]
    pub fn use_restorable_serde<T>(&self, id: &str, init: impl FnOnce() -> T) -> UseState<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + 'static,
    {
        self.use_restorable_with(
            id,
            init,
            |value: &T| serde_json::to_string(value).ok(),
            |saved| serde_json::from_str(saved).ok(),
        )
    }

    fn use_restorable_with<T: 'static>(
        &self,
        id: &str,
        init: impl FnOnce() -> T,
        write: fn(&T) -> Option<String>,
        read: fn(&str) -> Option<T>,
    ) -> UseState<T> {
        let restoration = &self.runtime().restoration;
        let path = self.restoration_path(id);
        let made = Cell::new(false);
        let state = self.use_state(|| {
            made.set(true);
            restoration
                .claim(&path)
                .and_then(|saved| read(&saved))
                .unwrap_or_else(init)
        });
        if made.get() {
            let cell = Rc::downgrade(&state.cell);
            restoration.register(path, move || read_cell(&cell, write));
        }
        state
    }

    /// Like [`use_text_controller`](Self::use_text_controller), and **restored**: the text
    /// is kept under `id`, and a process the system started again begins with it.
    ///
    /// The field's caret and selection are the field's: give it a
    /// [`restoration_id`](crate::TextField::restoration_id) too.
    pub fn use_restorable_text_controller(&self, id: &str, initial: &str) -> TextEditingController {
        let restoration = &self.runtime().restoration;
        let path = self.restoration_path(id);
        let made = Cell::new(false);
        let controller = self.use_ref(|| {
            made.set(true);
            let text = restoration
                .claim(&path)
                .unwrap_or_else(|| initial.to_string());
            TextEditingController::with_text(text)
        });
        if made.get() {
            let cell = Rc::downgrade(&controller.cell);
            restoration.register(path, move || {
                read_cell(&cell, |controller: &TextEditingController| {
                    Some(controller.text())
                })
            });
        }
        controller.get()
    }

    /// The path of the value `id`, in this component's scope.
    pub(crate) fn restoration_path(&self, id: &str) -> Vec<String> {
        scope_below(self.scope(), id).to_vec()
    }
}

/// What a hook's value reads as: gone with its component, or written.
fn read_cell<T>(cell: &Weak<RefCell<T>>, write: impl Fn(&T) -> Option<String>) -> Reading {
    let Some(cell) = cell.upgrade() else {
        return Reading::Gone;
    };
    let Ok(value) = cell.try_borrow() else {
        return Reading::Nothing;
    };
    match write(&value) {
        Some(text) => Reading::Value(text),
        None => Reading::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_bucket_keeps_values_and_children() {
        let mut root = RestorationBucket::new();
        root.insert(&path(&["page", "list", "offset"]), "0,120");
        root.insert(&path(&["tab"]), "2");
        assert_eq!(root.get(&path(&["page", "list", "offset"])), Some("0,120"));
        assert_eq!(root.value("tab"), Some("2"));
        assert_eq!(
            root.child("page")
                .and_then(|p| p.child("list"))
                .and_then(|l| l.value("offset")),
            Some("0,120")
        );
        assert_eq!(root.get(&path(&["page", "missing"])), None);
        assert_eq!(root.get(&[]), None);
    }

    #[test]
    fn taking_a_value_prunes_the_buckets_it_empties() {
        let mut root = RestorationBucket::new();
        root.insert(&path(&["a", "b", "c"]), "1");
        root.insert(&path(&["a", "d"]), "2");
        assert_eq!(root.take(&path(&["a", "b", "c"])), Some("1".to_string()));
        assert!(
            root.child("a").unwrap().child("b").is_none(),
            "b held only c"
        );
        assert_eq!(root.take(&path(&["a", "b", "c"])), None, "claimed once");
        assert_eq!(root.take(&path(&["a", "d"])), Some("2".to_string()));
        assert!(root.is_empty());
        assert!(root.child("a").is_none());
    }

    #[test]
    fn removing_a_scope_removes_everything_below() {
        let mut root = RestorationBucket::new();
        root.insert(&path(&["route 1", "list"]), "0,40");
        root.insert(&path(&["route 1", "form", "name"]), "Ada");
        root.insert(&path(&["route 0", "list"]), "0,10");
        root.remove_scope(&path(&["route 1"]));
        assert!(root.child("route 1").is_none());
        assert_eq!(root.get(&path(&["route 0", "list"])), Some("0,10"));
        root.insert(&path(&["x", "y", "z"]), "1");
        root.remove_scope(&path(&["x", "y"]));
        assert!(root.child("x").is_none(), "an emptied parent goes too");
        root.remove_scope(&[]);
        assert!(root.is_empty());
    }

    /// Whatever ids and values hold — slashes, tabs, line breaks, backslashes, the empty
    /// string, any script — the bytes read back as the same tree.
    #[test]
    fn encoding_round_trips_awkward_text() {
        let mut root = RestorationBucket::new();
        root.insert(&path(&["a/b", "tab\there", "line\nbreak"]), "x\\y/z\r\n\tw");
        root.insert(&path(&["", "empty value"]), "");
        root.insert(&path(&["日本", "مرحبا"]), "héllo 👋");
        root.insert(&path(&["trailing\\"]), "\\");
        let bytes = root.encode();
        assert_eq!(RestorationBucket::decode(&bytes), Some(root.clone()));
        // And an empty bucket is just the header.
        let empty = RestorationBucket::new().encode();
        assert_eq!(empty, HEADER.as_bytes());
        assert_eq!(
            RestorationBucket::decode(&empty),
            Some(RestorationBucket::new())
        );
    }

    #[test]
    fn bytes_that_are_not_a_bucket_read_as_none() {
        for bad in [
            &b""[..],
            b"frus-restoration 2\na\t1",
            b"something else",
            b"frus-restoration 1\nno tab here",
            b"frus-restoration 1\na\tbad escape \\q",
            b"frus-restoration 1\na\\",
            &[0xff, 0xfe, 0x00][..],
        ] {
            assert_eq!(RestorationBucket::decode(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn restorable_values_round_trip() {
        fn round<T: RestorableValue + PartialEq + std::fmt::Debug>(value: T) {
            assert_eq!(T::from_restoration(&value.to_restoration()), Some(value));
        }
        round(42u8);
        round(-7i64);
        round(1.5f32);
        round(f64::MAX);
        round(true);
        round('é');
        round("text with\nlines".to_string());
        round(Some(3u32));
        round(None::<u32>);
        round(Some(Some(String::new())));
        assert_eq!(
            u8::from_restoration("300"),
            None,
            "out of range starts afresh"
        );
        assert_eq!(Option::<u8>::from_restoration("x"), None);
    }

    #[test]
    fn edits_and_offsets_round_trip() {
        let mut edit = Edit {
            cursor: 4,
            anchor: Some(1),
            ..Edit::default()
        };
        edit.composing = Some((1, 2));
        let back = read_edit(&write_edit(&edit)).unwrap();
        assert_eq!(
            (back.cursor, back.anchor, back.composing),
            (4, Some(1), None)
        );
        let caret = read_edit(&write_edit(&Edit {
            cursor: 9,
            ..Edit::default()
        }))
        .unwrap();
        assert_eq!((caret.cursor, caret.anchor), (9, None));
        assert_eq!(read_offset("0,812.5"), Some((0.0, 812.5)));
        assert_eq!(read_offset("NaN,1"), None);
        assert_eq!(read_offset("1"), None);
    }

    /// Nothing is registered, claimed or kept until the application asks for restoration.
    #[test]
    fn off_until_the_application_asks() {
        let restoration = Restoration::default();
        let runtime = Runtime::default();
        assert!(restoration.data(&runtime).is_none());
        assert!(!restoration.restore(b"frus-restoration 1\napp/x\t1"));
        restoration.register(path(&["x"]), || Reading::Value("1".into()));
        assert!(restoration.live.borrow().is_empty());
        assert_eq!(restoration.claim(&path(&["x"])), None);
    }

    /// Registered values are read when the data is taken; a value whose owner has gone
    /// leaves; one restored and never claimed stays for a later build to claim.
    #[test]
    fn data_reads_live_values_and_keeps_unclaimed_ones() {
        let runtime = Runtime::default();
        let restoration = &runtime.restoration;
        restoration.enable("app");
        let mut handed = RestorationBucket::new();
        handed.insert(&path(&["app", "route 1 /b", "draft"]), "unclaimed");
        handed.insert(&path(&["app", "count"]), "3");
        handed.insert(&path(&["other app", "x"]), "not ours");
        assert!(restoration.restore(&handed.encode()));
        assert_eq!(restoration.claim(&path(&["count"])), Some("3".to_string()));

        let count = Rc::new(RefCell::new(4));
        let weak = Rc::downgrade(&count);
        restoration.register(path(&["count"]), move || {
            read_cell(&weak, |n: &i32| Some(n.to_string()))
        });
        restoration.register(path(&["empty"]), || Reading::Nothing);

        let data = restoration.data(&runtime).unwrap();
        let root = RestorationBucket::decode(&data).unwrap();
        let app = root.child("app").unwrap();
        assert_eq!(app.value("count"), Some("4"));
        assert_eq!(app.value("empty"), None);
        assert_eq!(app.get(&path(&["route 1 /b", "draft"])), Some("unclaimed"));
        assert!(root.child("other app").is_none());

        drop(count);
        let bucket = restoration.bucket(&runtime);
        assert_eq!(bucket.value("count"), None, "gone with its owner");
        assert!(!restoration.live.borrow().contains_key(&path(&["count"])));

        restoration.forget_scope(&path(&["route 1 /b"]));
        assert!(restoration.bucket(&runtime).child("route 1 /b").is_none());
    }

    /// A widget's offset is read from the runtime while it is laid out, and its last value
    /// kept once it is not — until its scope is forgotten.
    #[test]
    fn a_scroll_offset_is_restored_and_kept() {
        let mut runtime = Runtime::default();
        runtime.restoration.enable("app");
        let mut handed = RestorationBucket::new();
        handed.insert(&path(&["app", "page", "list"]), "0,300");
        handed.insert(&path(&["app", "page", "field"]), "5,2");
        runtime.restoration.restore(&handed.encode());

        let (list, field) = (WidgetId::from_u64(7), WidgetId::from_u64(8));
        runtime
            .restoration
            .register_scroll(path(&["page", "list"]), list);
        runtime
            .restoration
            .register_edit(path(&["page", "field"]), field);
        assert!(runtime.apply_restored());
        assert_eq!(runtime.scroll.get(&list), Some(&(0.0, 300.0)));
        let edit = runtime.edits[&field];
        assert_eq!((edit.cursor, edit.anchor), (5, Some(2)));
        assert!(!runtime.apply_restored(), "handed over once");

        runtime.scroll.insert(list, (0.0, 450.0));
        assert_eq!(
            runtime
                .restoration
                .bucket(&runtime)
                .get(&path(&["page", "list"])),
            Some("0,450")
        );
        runtime.scroll.remove(&list);
        assert_eq!(
            runtime
                .restoration
                .bucket(&runtime)
                .get(&path(&["page", "list"])),
            Some("0,450"),
            "the last offset, once the list is not laid out"
        );
        runtime.restoration.forget_scope(&path(&["page"]));
        assert!(runtime.restoration.bucket(&runtime).is_empty());
    }
}
