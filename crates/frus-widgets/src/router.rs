//! **A router**: locations, pages, and moving between them.
//!
//! An application with more than one screen names them. A [`GoRoute`] says which pattern of
//! path it answers to and what page it builds; a [`GoRouter`] holds the routes and the stack
//! of pages the person is looking at, and moves that stack when told a *location* — the
//! string a link, a deep link or a button would carry: `/users/42?tab=posts`.
//!
//! ```
//! use frus_widgets::{text, GoRoute, GoRouter};
//!
//! let router = GoRouter::new(vec![
//!     GoRoute::new("/", |_, _| text("home")).routes(vec![
//!         // relative to its parent: `/users/:id`
//!         GoRoute::new("users/:id", |_, state| {
//!             let id = state.param("id").unwrap_or("?").to_string();
//!             text(format!("user {id}"))
//!         })
//!         .name("user"),
//!     ]),
//!     GoRoute::new("/settings", |_, _| text("settings")),
//! ]);
//!
//! router.go("/users/42?tab=posts");
//! assert_eq!(router.location(), "/users/42?tab=posts");
//! assert!(router.can_pop(), "home is underneath");
//!
//! router.pop();
//! assert_eq!(router.location(), "/");
//! ```
//!
//! ## Two ways to move
//!
//! [`GoRouter::go`] says *where*: it makes the stack whatever the location's routes are — a
//! location that lives under `/` has `/` beneath it, so there is somewhere to go back to.
//! [`GoRouter::push`] says *on top of what is here*: it adds one page and leaves the rest,
//! which is how the same page can be open twice.
//!
//! ## Redirects
//!
//! A redirect looks at where a navigation is heading and may send it elsewhere — to a sign-in
//! page while nobody is signed in, away from it once someone is. It runs on every navigation,
//! and again when a [`refresh listenable`](GoRouter::refresh_listenable) changes; a redirect
//! that answers with another location is asked again about that one, up to a limit.
//!
//! ## Pages under the top
//!
//! A page covered by another is kept: what its components hold is still there when the top
//! page is popped. It is built, but not laid out or drawn, until it is on show again.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use frus_core::{AnimationController, SpringDescription};

use crate::component::{request_rebuild, BuildContext};
use crate::navigator::Navigator;
use crate::notifier::{Listenable, Subscription};
use crate::widget::Widget;

// ---------------------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------------------

/// Horizon (s) over which a finger's velocity is projected to decide between going back and
/// staying.
const BACK_PROJECT: f32 = 0.12;
/// Projected position (a fraction) beyond which going back is committed.
const BACK_COMMIT_POS: f32 = 0.5;
/// Stiffness of the page transition's spring.
const SPRING_K: f32 = 220.0;
/// Its damping: close to critical, so a page arrives without overshooting.
const SPRING_C: f32 = 30.0;

fn spring() -> SpringDescription {
    SpringDescription::new(1.0, SPRING_K, SPRING_C)
}

// ---------------------------------------------------------------------------------------
// Locations
// ---------------------------------------------------------------------------------------

fn decode(text: &str, plus_is_space: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// A location taken apart: its path's segments, and its query.
struct Parts {
    segments: Vec<String>,
    query: Vec<(String, String)>,
}

fn split_location(location: &str) -> Parts {
    let without_fragment = location.split('#').next().unwrap_or("");
    let (path, query) = without_fragment
        .split_once('?')
        .unwrap_or((without_fragment, ""));
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect();
    let query = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key, true), decode(value, true))
        })
        .collect();
    Parts { segments, query }
}

// ---------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------

/// Where a page is: what location it was reached at, which route it is, and what the
/// location said — the values of the path's `:parameters`, and the query.
#[derive(Clone)]
pub struct GoRouterState {
    location: String,
    path: String,
    name: Option<String>,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
    extra: Option<Rc<dyn Any>>,
    entering: f32,
}

impl GoRouterState {
    /// How far into view this page is: `1.0` once it has settled, and while a transition
    /// brings it in or takes it out, the number that transition is driven by — `0.0` not yet
    /// arrived. A page that wants to move its own contents as it arrives reads it, and one
    /// that does not ignores it.
    pub fn entering(&self) -> f32 {
        self.entering
    }

    /// The whole location, query included: `/users/42?tab=posts`.
    pub fn location(&self) -> &str {
        &self.location
    }

    /// The pattern of the route that answered: `/users/:id`. Empty when no route did.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The route's name, if it has one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The value of the path parameter `name` — `42` for `:id` in `/users/42`.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    /// Every path parameter of the location.
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// The value of the query parameter `name` — `posts` for `tab` in `?tab=posts`. When it
    /// is given more than once, the last one.
    pub fn query(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(String::as_str)
    }

    /// Every query parameter of the location.
    pub fn query_params(&self) -> &HashMap<String, String> {
        &self.query
    }

    /// The object that came along with the navigation ([`GoRouter::go_with_extra`]), if it
    /// is a `T`.
    pub fn extra<T: 'static>(&self) -> Option<Rc<T>> {
        self.extra.clone().and_then(|extra| extra.downcast().ok())
    }
}

impl fmt::Debug for GoRouterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoRouterState")
            .field("location", &self.location)
            .field("path", &self.path)
            .field("name", &self.name)
            .field("params", &self.params)
            .field("query", &self.query)
            .finish()
    }
}

// ---------------------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------------------

type RouteBuilder = Rc<dyn Fn(&BuildContext, &GoRouterState) -> Box<dyn Widget>>;
type Redirect = Rc<dyn Fn(&GoRouterState) -> Option<String>>;

/// One place the router can go: a pattern of path, and the page it builds.
///
/// A path is made of segments: a literal (`users`), a parameter (`:id`), or a trailing `*`
/// that takes whatever is left. A top-level route's path starts with `/`; a sub-route's is
/// relative to its parent's and does not.
pub struct GoRoute {
    path: String,
    name: Option<String>,
    builder: RouteBuilder,
    routes: Vec<GoRoute>,
    redirect: Option<Redirect>,
}

impl GoRoute {
    /// A route answering to `path`, building its page with `builder`.
    ///
    /// `builder` is told the [`GoRouterState`] — where the location put the person — and may
    /// return any widget: a [`Component`](crate::Component), a `Box<dyn Widget>`, a
    /// scaffold.
    pub fn new<W: Widget + 'static>(
        path: impl Into<String>,
        builder: impl Fn(&BuildContext, &GoRouterState) -> W + 'static,
    ) -> Self {
        Self {
            path: path.into(),
            name: None,
            builder: Rc::new(move |cx, state| Box::new(builder(cx, state))),
            routes: Vec::new(),
            redirect: None,
        }
    }

    /// Names the route, so it can be reached without writing its path:
    /// [`GoRouter::go_named`].
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The routes under this one. A location that one of them answers to has this route's page
    /// beneath its own.
    pub fn routes(mut self, routes: Vec<GoRoute>) -> Self {
        self.routes = routes;
        self
    }

    /// Where to send a navigation heading to this route or below it, if not here — `None` to
    /// let it through.
    pub fn redirect(
        mut self,
        redirect: impl Fn(&GoRouterState) -> Option<String> + 'static,
    ) -> Self {
        self.redirect = Some(Rc::new(redirect));
        self
    }
}

enum Segment {
    Literal(String),
    Param(String),
    Splat,
}

struct Node {
    full_path: String,
    segments: Vec<Segment>,
    name: Option<String>,
    builder: RouteBuilder,
    redirect: Option<Redirect>,
    children: Vec<Rc<Node>>,
}

fn build_node(
    route: GoRoute,
    parent: Option<&str>,
    names: &mut HashMap<String, Rc<Node>>,
) -> Rc<Node> {
    let full_path = match parent {
        None => {
            assert!(
                route.path.starts_with('/'),
                "a top-level route's path starts with `/`: `{}`",
                route.path
            );
            route.path.clone()
        }
        Some(parent) => {
            assert!(
                !route.path.starts_with('/'),
                "a sub-route's path is relative to its parent's, and does not start with `/`: `{}`",
                route.path
            );
            assert!(!route.path.is_empty(), "a sub-route needs a path");
            format!("{}/{}", parent.trim_end_matches('/'), route.path)
        }
    };
    let segments = route
        .path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| match segment {
            "*" => Segment::Splat,
            other => match other.strip_prefix(':') {
                Some(name) => Segment::Param(name.to_string()),
                None => Segment::Literal(other.to_string()),
            },
        })
        .collect();
    let children = route
        .routes
        .into_iter()
        .map(|child| build_node(child, Some(&full_path), names))
        .collect();
    let node = Rc::new(Node {
        full_path,
        segments,
        name: route.name.clone(),
        builder: route.builder,
        redirect: route.redirect,
        children,
    });
    if let Some(name) = route.name {
        let clash = names.insert(name.clone(), node.clone());
        assert!(clash.is_none(), "two routes are named `{name}`");
    }
    node
}

/// How many segments of `input` the pattern takes, filling `params` — or `None` if it does
/// not match the start of it.
fn consume(
    segments: &[Segment],
    input: &[String],
    params: &mut HashMap<String, String>,
) -> Option<usize> {
    let mut at = 0;
    for segment in segments {
        match segment {
            Segment::Literal(literal) => {
                if input.get(at)? != literal {
                    return None;
                }
                at += 1;
            }
            Segment::Param(name) => {
                params.insert(name.clone(), decode(input.get(at)?, false));
                at += 1;
            }
            Segment::Splat => {
                params.insert("*".to_string(), input[at.min(input.len())..].join("/"));
                at = input.len();
            }
        }
    }
    Some(at)
}

/// The chain of routes that answers to `input`, the parent first — the deepest route whose
/// pattern takes all of it.
fn match_nodes(
    nodes: &[Rc<Node>],
    input: &[String],
    params: &mut HashMap<String, String>,
) -> Option<Vec<Rc<Node>>> {
    for node in nodes {
        let mut found = params.clone();
        let Some(used) = consume(&node.segments, input, &mut found) else {
            continue;
        };
        if used == input.len() {
            *params = found;
            return Some(vec![node.clone()]);
        }
        if let Some(mut below) = match_nodes(&node.children, &input[used..], &mut found) {
            *params = found;
            below.insert(0, node.clone());
            return Some(below);
        }
    }
    None
}

// ---------------------------------------------------------------------------------------
// The router
// ---------------------------------------------------------------------------------------

/// One page of the stack.
#[derive(Clone)]
struct Page {
    /// What identifies it: a page reached again by `go` keeps it, and so keeps its state; one
    /// that was pushed has its own.
    key: u64,
    node: Option<Rc<Node>>,
    state: Rc<GoRouterState>,
}

struct Transition {
    from: Page,
    controller: AnimationController,
    forward: bool,
}

struct Back {
    progress: f32,
    settle: Option<AnimationController>,
    commit: bool,
}

#[derive(Default)]
struct Stack {
    pages: Vec<Page>,
    transition: Option<Transition>,
    back: Option<Back>,
}

struct Config {
    nodes: Vec<Rc<Node>>,
    names: HashMap<String, Rc<Node>>,
    initial: String,
    redirect: Option<Redirect>,
    error: Option<RouteBuilder>,
    limit: usize,
    animate: bool,
}

struct Inner {
    config: RefCell<Config>,
    stack: RefCell<Stack>,
    started: Cell<bool>,
    unique: Cell<u64>,
    refresh: RefCell<Option<Subscription>>,
}

/// The routes, and the stack of pages showing which of them the person has been through.
///
/// A cheap handle: clone it into a handler and navigate from there.
#[derive(Clone)]
pub struct GoRouter {
    inner: Rc<Inner>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Go,
    Push,
    Replace,
}

fn hash_of(value: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl GoRouter {
    /// A router over `routes`, starting at `/`.
    pub fn new(routes: Vec<GoRoute>) -> Self {
        let mut names = HashMap::new();
        let nodes = routes
            .into_iter()
            .map(|route| build_node(route, None, &mut names))
            .collect();
        Self {
            inner: Rc::new(Inner {
                config: RefCell::new(Config {
                    nodes,
                    names,
                    initial: "/".to_string(),
                    redirect: None,
                    error: None,
                    limit: 5,
                    animate: true,
                }),
                stack: RefCell::new(Stack::default()),
                started: Cell::new(false),
                unique: Cell::new(0),
                refresh: RefCell::new(None),
            }),
        }
    }

    /// Where the router starts. `/` by default.
    pub fn initial_location(self, location: impl Into<String>) -> Self {
        self.inner.config.borrow_mut().initial = location.into();
        self
    }

    /// A redirect for every navigation: it is told where it is heading and may answer with
    /// another location, or `None` to let it through.
    ///
    /// It closes over whatever it needs to decide — a handle to the signed-in state, say.
    pub fn redirect(self, redirect: impl Fn(&GoRouterState) -> Option<String> + 'static) -> Self {
        self.inner.config.borrow_mut().redirect = Some(Rc::new(redirect));
        self
    }

    /// The page shown when no route answers to a location. A plain "not found" page by
    /// default.
    pub fn error_builder<W: Widget + 'static>(
        self,
        builder: impl Fn(&BuildContext, &GoRouterState) -> W + 'static,
    ) -> Self {
        self.inner.config.borrow_mut().error =
            Some(Rc::new(move |cx, state| Box::new(builder(cx, state))));
        self
    }

    /// How many redirects in a row are followed before the router gives up and shows the
    /// error page. Five by default.
    pub fn redirect_limit(self, limit: usize) -> Self {
        self.inner.config.borrow_mut().limit = limit;
        self
    }

    /// Whether a page slides in and out. On by default.
    pub fn animate(self, animate: bool) -> Self {
        self.inner.config.borrow_mut().animate = animate;
        self
    }

    /// Runs the redirects again whenever `listenable` changes — the way a sign-in or sign-out
    /// moves the person without anyone having navigated.
    pub fn refresh_listenable(self, listenable: &impl Listenable) -> Self {
        let weak: Weak<Inner> = Rc::downgrade(&self.inner);
        let subscription = listenable.add_listener(move || {
            if let Some(inner) = weak.upgrade() {
                GoRouter { inner }.refresh();
            }
        });
        *self.inner.refresh.borrow_mut() = Some(subscription);
        self
    }

    // ---- reading -----------------------------------------------------------------------

    /// The location of the page on top.
    pub fn location(&self) -> String {
        self.start();
        self.inner
            .stack
            .borrow()
            .pages
            .last()
            .map(|page| page.state.location.clone())
            .unwrap_or_default()
    }

    /// The state of the page on top: what its location put in it.
    pub fn state(&self) -> Option<GoRouterState> {
        self.start();
        self.inner
            .stack
            .borrow()
            .pages
            .last()
            .map(|page| (*page.state).clone())
    }

    /// How many pages the stack holds.
    pub fn depth(&self) -> usize {
        self.start();
        self.inner.stack.borrow().pages.len()
    }

    /// Whether there is a page beneath the top to go back to.
    pub fn can_pop(&self) -> bool {
        self.depth() > 1
    }

    /// The location of the route called `name`, with its `:parameters` filled in and `query`
    /// appended. `None` if there is no such route, or a parameter is missing.
    pub fn location_for(
        &self,
        name: &str,
        params: &[(&str, &str)],
        query: &[(&str, &str)],
    ) -> Option<String> {
        let node = self.inner.config.borrow().names.get(name).cloned()?;
        let mut path = String::new();
        for segment in node.full_path.split('/').filter(|s| !s.is_empty()) {
            path.push('/');
            match segment.strip_prefix(':') {
                Some(param) => {
                    let (_, value) = params.iter().find(|(key, _)| *key == param)?;
                    path.push_str(&encode(value));
                }
                None => path.push_str(segment),
            }
        }
        if path.is_empty() {
            path.push('/');
        }
        if !query.is_empty() {
            let pairs: Vec<String> = query
                .iter()
                .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
                .collect();
            path.push('?');
            path.push_str(&pairs.join("&"));
        }
        Some(path)
    }

    // ---- moving ------------------------------------------------------------------------

    /// Goes to `location`: the stack becomes the routes that location is made of.
    pub fn go(&self, location: impl AsRef<str>) {
        self.navigate(location.as_ref(), None, Mode::Go);
    }

    /// [`go`](Self::go), carrying an object the page can read with
    /// [`GoRouterState::extra`].
    pub fn go_with_extra(&self, location: impl AsRef<str>, extra: impl Any) {
        self.navigate(location.as_ref(), Some(Rc::new(extra)), Mode::Go);
    }

    /// Goes to the route called `name`.
    pub fn go_named(&self, name: &str, params: &[(&str, &str)], query: &[(&str, &str)]) {
        match self.location_for(name, params, query) {
            Some(location) => self.go(location),
            None => debug_assert!(false, "no route called `{name}`, or a parameter is missing"),
        }
    }

    /// Pushes the page for `location` on top of what is here.
    pub fn push(&self, location: impl AsRef<str>) {
        self.navigate(location.as_ref(), None, Mode::Push);
    }

    /// [`push`](Self::push), carrying an object the page can read with
    /// [`GoRouterState::extra`].
    pub fn push_with_extra(&self, location: impl AsRef<str>, extra: impl Any) {
        self.navigate(location.as_ref(), Some(Rc::new(extra)), Mode::Push);
    }

    /// Pushes the route called `name`.
    pub fn push_named(&self, name: &str, params: &[(&str, &str)], query: &[(&str, &str)]) {
        match self.location_for(name, params, query) {
            Some(location) => self.push(location),
            None => debug_assert!(false, "no route called `{name}`, or a parameter is missing"),
        }
    }

    /// Replaces the page on top with the one for `location`, so going back skips the one it
    /// replaced.
    pub fn replace(&self, location: impl AsRef<str>) {
        self.navigate(location.as_ref(), None, Mode::Replace);
    }

    /// Removes the page on top. Whether there was one to remove: the last page stays.
    pub fn pop(&self) -> bool {
        self.start();
        let pages = {
            let stack = self.inner.stack.borrow();
            if stack.pages.len() <= 1 {
                return false;
            }
            stack.pages[..stack.pages.len() - 1].to_vec()
        };
        self.commit(pages);
        true
    }

    /// Asks the redirects again about where the top page is. What a
    /// [`refresh listenable`](Self::refresh_listenable) does when it changes.
    pub fn refresh(&self) {
        self.start();
        let target = self
            .inner
            .stack
            .borrow()
            .pages
            .last()
            .map(|page| (page.state.location.clone(), page.state.extra.clone()));
        if let Some((location, extra)) = target {
            self.navigate(&location, extra, Mode::Go);
        }
    }

    // ---- the machinery -----------------------------------------------------------------

    fn start(&self) {
        if self.inner.started.replace(true) {
            return;
        }
        let initial = self.inner.config.borrow().initial.clone();
        let pages = self.pages_for(&initial, None, Mode::Go, &[]);
        self.inner.stack.borrow_mut().pages = pages;
    }

    fn navigate(&self, location: &str, extra: Option<Rc<dyn Any>>, mode: Mode) {
        self.start();
        let current = self.inner.stack.borrow().pages.clone();
        let pages = self.pages_for(location, extra, mode, &current);
        self.commit(pages);
    }

    /// The stack that results from `mode` to `location`, redirects followed.
    fn pages_for(
        &self,
        location: &str,
        extra: Option<Rc<dyn Any>>,
        mode: Mode,
        current: &[Page],
    ) -> Vec<Page> {
        let (nodes, redirect, limit) = {
            let config = self.inner.config.borrow();
            (config.nodes.clone(), config.redirect.clone(), config.limit)
        };
        let mut location = location.to_string();
        let mut seen = 0usize;
        loop {
            let parts = split_location(&location);
            let mut params = HashMap::new();
            let chain = match_nodes(&nodes, &parts.segments, &mut params);
            let query: HashMap<String, String> = parts.query.iter().cloned().collect();
            let leaf = chain.as_ref().and_then(|chain| chain.last());
            let state = GoRouterState {
                location: location.clone(),
                path: leaf.map(|n| n.full_path.clone()).unwrap_or_default(),
                name: leaf.and_then(|n| n.name.clone()),
                params: params.clone(),
                query: query.clone(),
                extra: extra.clone(),
                entering: 1.0,
            };

            // Redirects: the router's own, then each route's, parent first.
            let mut sent = redirect.as_ref().and_then(|r| r(&state));
            if sent.is_none() {
                if let Some(chain) = &chain {
                    sent = chain
                        .iter()
                        .filter_map(|node| node.redirect.as_ref())
                        .find_map(|r| r(&state));
                }
            }
            if let Some(next) = sent.filter(|next| *next != location) {
                seen += 1;
                if seen > limit {
                    let state = GoRouterState {
                        location: location.clone(),
                        path: String::new(),
                        name: None,
                        params: HashMap::new(),
                        query: HashMap::new(),
                        extra: None,
                        entering: 1.0,
                    };
                    return self.error_pages(state, current, mode);
                }
                location = next;
                continue;
            }

            let Some(chain) = chain else {
                return self.error_pages(state, current, mode);
            };
            return match mode {
                Mode::Go => {
                    // One page per route of the chain, each keyed by the part of the location
                    // it answered to, so a page that is still on the way keeps its state.
                    let mut prefix = String::new();
                    let mut used = 0;
                    chain
                        .iter()
                        .enumerate()
                        .map(|(at, node)| {
                            let mut scratch = HashMap::new();
                            let took =
                                consume(&node.segments, &parts.segments[used..], &mut scratch)
                                    .unwrap_or(0);
                            for segment in &parts.segments[used..used + took] {
                                prefix.push('/');
                                prefix.push_str(segment);
                            }
                            used += took;
                            let page_state = if at + 1 == chain.len() {
                                state.clone()
                            } else {
                                // A page underneath stands for the part of the location it
                                // answered — `/` under `/users/42` — so going back to it lands
                                // on *its* location, not on where the person had been heading.
                                GoRouterState {
                                    location: if prefix.is_empty() {
                                        "/".to_string()
                                    } else {
                                        prefix.clone()
                                    },
                                    path: node.full_path.clone(),
                                    name: node.name.clone(),
                                    ..state.clone()
                                }
                            };
                            Page {
                                key: hash_of(("go", prefix.clone(), node.full_path.clone())),
                                node: Some(node.clone()),
                                state: Rc::new(page_state),
                            }
                        })
                        .collect()
                }
                Mode::Push | Mode::Replace => {
                    let mut pages = current.to_vec();
                    if mode == Mode::Replace {
                        pages.pop();
                    }
                    let leaf = chain.last().expect("a match has a route").clone();
                    let unique = self.inner.unique.get() + 1;
                    self.inner.unique.set(unique);
                    pages.push(Page {
                        key: hash_of(("push", unique)),
                        node: Some(leaf),
                        state: Rc::new(state),
                    });
                    pages
                }
            };
        }
    }

    fn error_pages(&self, state: GoRouterState, current: &[Page], mode: Mode) -> Vec<Page> {
        let unique = self.inner.unique.get() + 1;
        self.inner.unique.set(unique);
        let page = Page {
            key: hash_of(("error", unique)),
            node: None,
            state: Rc::new(state),
        };
        match mode {
            Mode::Go => vec![page],
            Mode::Push => {
                let mut pages = current.to_vec();
                pages.push(page);
                pages
            }
            Mode::Replace => {
                let mut pages = current.to_vec();
                pages.pop();
                pages.push(page);
                pages
            }
        }
    }

    /// Makes `pages` the stack, starting the transition from the page that was on top.
    fn commit(&self, pages: Vec<Page>) {
        let animate = self.inner.config.borrow().animate;
        let mut stack = self.inner.stack.borrow_mut();
        let old_top = stack.pages.last().cloned();
        let new_top_key = pages.last().map(|page| page.key);
        if let (true, Some(old), Some(new_key)) = (animate, old_top, new_top_key) {
            if old.key != new_key {
                let forward = !stack.pages.iter().any(|page| page.key == new_key);
                let mut controller = AnimationController::unit();
                controller.set_value(0.0);
                controller.spring_to(1.0, spring(), 0.0);
                stack.transition = Some(Transition {
                    from: old,
                    controller,
                    forward,
                });
            }
        }
        stack.pages = pages;
        stack.back = None;
        drop(stack);
        request_rebuild();
    }

    // ---- driven by the shell -----------------------------------------------------------

    /// Advances the page transition and a back gesture's settling by `dt` seconds. Whether
    /// anything is still moving.
    pub fn tick(&self, dt: f32) -> bool {
        let mut animating = false;
        let mut pop = false;
        {
            let mut stack = self.inner.stack.borrow_mut();
            let arrived = match stack.transition.as_mut() {
                Some(transition) => {
                    if transition.controller.tick(dt) {
                        animating = true;
                        false
                    } else {
                        true
                    }
                }
                None => false,
            };
            if arrived {
                stack.transition = None;
            }
            let mut settled = false;
            if let Some(back) = stack.back.as_mut() {
                if let Some(settle) = back.settle.as_mut() {
                    if settle.tick(dt) {
                        back.progress = settle.value();
                        animating = true;
                    } else {
                        pop = back.commit;
                        settled = true;
                    }
                }
            }
            if settled {
                stack.back = None;
            }
        }
        if pop {
            // The finger has already carried the page away: there is no transition left to
            // play, only the stack to catch up with.
            let pages = {
                let stack = self.inner.stack.borrow();
                stack.pages[..stack.pages.len().saturating_sub(1)].to_vec()
            };
            if !pages.is_empty() {
                self.inner.stack.borrow_mut().pages = pages;
            }
            request_rebuild();
        }
        animating
    }

    /// Whether a back gesture can start: there is a page to go back to.
    pub fn can_go_back(&self) -> bool {
        self.can_pop()
    }

    /// A back gesture is under way, `progress` of the way from `0` to `1`.
    pub fn back_gesture(&self, progress: f32) {
        let mut stack = self.inner.stack.borrow_mut();
        match stack.back.as_mut() {
            Some(back) => back.progress = progress,
            None => {
                stack.back = Some(Back {
                    progress,
                    settle: None,
                    commit: false,
                })
            }
        }
        drop(stack);
        request_rebuild();
    }

    /// The finger lifted, moving at `velocity` (fractions per second): the page goes back or
    /// stays, whichever the momentum and the position say.
    pub fn back_gesture_end(&self, velocity: f32) {
        let mut stack = self.inner.stack.borrow_mut();
        let deep_enough = stack.pages.len() > 1;
        if let Some(back) = stack.back.as_mut() {
            let projected = back.progress + velocity * BACK_PROJECT;
            let commit = projected > BACK_COMMIT_POS && deep_enough;
            back.commit = commit;
            let mut settle = AnimationController::unit();
            settle.set_value(back.progress);
            settle.spring_to(if commit { 1.0 } else { 0.0 }, spring(), velocity);
            back.settle = Some(settle);
        }
        drop(stack);
        request_rebuild();
    }

    // ---- building ----------------------------------------------------------------------

    /// The widget for the router's pages: what an application whose root is a router
    /// builds. It also makes the router reachable from below, with [`BuildContext::router`].
    pub fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        self.start();
        cx.runtime().states.provide(Rc::new(self.clone()));
        let error_builder = self.inner.config.borrow().error.clone();
        // What is on the stack is copied out, so that a page's builder can ask the router
        // things without finding it borrowed.
        let (pages, transition, back) = {
            let stack = self.inner.stack.borrow();
            (
                stack.pages.clone(),
                stack
                    .transition
                    .as_ref()
                    .map(|t| (t.from.clone(), t.controller.value(), t.forward)),
                stack.back.as_ref().map(|b| b.progress),
            )
        };
        let depth = pages.len();
        // A page is built told how far in it is: the transition's own number, seen from
        // this page — the one arriving has it as it is, the one leaving has it the other way.
        let make = |page: &Page, entering: f32| -> Box<dyn Widget> {
            let state = GoRouterState {
                entering,
                ..(*page.state).clone()
            };
            match &page.node {
                Some(node) => (node.builder)(cx, &state),
                None => match &error_builder {
                    Some(builder) => builder(cx, &state),
                    None => Box::new(crate::Text::new(format!(
                        "Page not found: {}",
                        state.location
                    ))),
                },
            }
        };
        let key_of = |page: &Page, depth: usize| (depth, page.key);
        let top = pages.last().expect("a router has at least one page");

        // A back gesture previews the pop, driven by the finger.
        if let Some(progress) = back {
            if depth >= 2 {
                let below = &pages[depth - 2];
                let mut navigator = Navigator::new(key_of(below, depth - 2), make(below, progress))
                    .from(
                        key_of(top, depth - 1),
                        make(top, 1.0 - progress),
                        progress,
                        false,
                    );
                for (at, page) in pages[..depth - 2].iter().enumerate() {
                    navigator = navigator.retain(key_of(page, at), make(page, 1.0));
                }
                return Box::new(navigator);
            }
        }

        match transition {
            Some((from, progress, forward)) => {
                // The page leaving is either still in the stack (a push left it one entry
                // down) or gone from it (a pop took it off the top).
                let from_depth = pages
                    .iter()
                    .position(|page| page.key == from.key)
                    .unwrap_or(depth);
                let mut navigator = Navigator::new(key_of(top, depth - 1), make(top, progress))
                    .from(
                        key_of(&from, from_depth),
                        make(&from, 1.0 - progress),
                        progress,
                        forward,
                    );
                for (at, page) in pages[..depth - 1].iter().enumerate() {
                    if page.key != from.key {
                        navigator = navigator.retain(key_of(page, at), make(page, 1.0));
                    }
                }
                Box::new(navigator)
            }
            None => {
                let mut navigator = Navigator::new(key_of(top, depth - 1), make(top, 1.0));
                for (at, page) in pages[..depth - 1].iter().enumerate() {
                    navigator = navigator.retain(key_of(page, at), make(page, 1.0));
                }
                Box::new(navigator)
            }
        }
    }
}

impl BuildContext<'_> {
    /// The application's router. Handlers capture it and navigate from there:
    ///
    /// ```ignore
    /// let router = cx.router();
    /// button("Open", move || router.go("/settings"))
    /// ```
    ///
    /// Panics if this application has no router — one made with `FrusApp::router`.
    pub fn router(&self) -> GoRouter {
        match self.service::<GoRouter>() {
            Some(router) => (*router).clone(),
            None => {
                panic!("there is no router: this application was not made with `FrusApp::router`")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Container;

    fn page(_: &BuildContext, _: &GoRouterState) -> Container {
        Container::new()
    }

    fn routes() -> Vec<GoRoute> {
        vec![
            GoRoute::new("/", page).routes(vec![
                GoRoute::new("users/:id", page)
                    .name("user")
                    .routes(vec![GoRoute::new("posts/:post", page).name("post")]),
                GoRoute::new("files/*", page).name("files"),
            ]),
            GoRoute::new("/settings", page).name("settings"),
        ]
    }

    fn router() -> GoRouter {
        GoRouter::new(routes()).animate(false)
    }

    fn keys(router: &GoRouter) -> Vec<String> {
        router
            .inner
            .stack
            .borrow()
            .pages
            .iter()
            .map(|p| p.state.path.clone())
            .collect()
    }

    #[test]
    fn it_starts_at_the_initial_location() {
        assert_eq!(router().location(), "/");
        assert_eq!(
            GoRouter::new(routes())
                .initial_location("/settings")
                .location(),
            "/settings"
        );
    }

    #[test]
    fn go_makes_the_stack_the_chain_of_routes_the_location_is_made_of() {
        let r = router();
        r.go("/users/42/posts/7");
        assert_eq!(
            keys(&r),
            ["/", "/users/:id", "/users/:id/posts/:post"],
            "home, the user, then the post: something to go back to at each step"
        );
        let state = r.state().unwrap();
        assert_eq!(state.param("id"), Some("42"));
        assert_eq!(state.param("post"), Some("7"));
        assert_eq!(state.path(), "/users/:id/posts/:post");
        assert_eq!(state.name(), Some("post"));
    }

    #[test]
    fn a_page_underneath_stands_for_its_own_location() {
        let r = router();
        r.go("/users/42/posts/7?x=1");
        let locations: Vec<String> = r
            .inner
            .stack
            .borrow()
            .pages
            .iter()
            .map(|p| p.state.location.clone())
            .collect();
        assert_eq!(
            locations,
            ["/", "/users/42", "/users/42/posts/7?x=1"],
            "each page is the part of the location it answered; the top one, all of it"
        );
        r.pop();
        assert_eq!(
            r.location(),
            "/users/42",
            "so going back lands where that page is"
        );
    }

    #[test]
    fn a_page_underneath_knows_the_parameters_too() {
        let r = router();
        r.go("/users/42/posts/7");
        let below = r.inner.stack.borrow().pages[1].state.clone();
        assert_eq!(below.param("id"), Some("42"));
        assert_eq!(below.path(), "/users/:id");
    }

    #[test]
    fn the_query_is_read_and_decoded() {
        let r = router();
        r.go("/users/1?tab=posts&q=a%20b+c&flag");
        let state = r.state().unwrap();
        assert_eq!(state.query("tab"), Some("posts"));
        assert_eq!(state.query("q"), Some("a b c"));
        assert_eq!(state.query("flag"), Some(""));
        assert_eq!(state.location(), "/users/1?tab=posts&q=a%20b+c&flag");
    }

    #[test]
    fn a_parameter_is_percent_decoded() {
        let r = router();
        r.go("/users/a%2Fb");
        assert_eq!(r.state().unwrap().param("id"), Some("a/b"));
    }

    #[test]
    fn a_wildcard_takes_what_is_left() {
        let r = router();
        r.go("/files/a/b/c.txt");
        assert_eq!(r.state().unwrap().param("*"), Some("a/b/c.txt"));
    }

    #[test]
    fn a_location_no_route_answers_to_is_an_error_page() {
        let r = router();
        r.go("/nowhere");
        assert_eq!(r.depth(), 1);
        assert!(r.inner.stack.borrow().pages[0].node.is_none());
        assert_eq!(r.location(), "/nowhere");
    }

    #[test]
    fn push_adds_a_page_and_pop_removes_it() {
        let r = router();
        r.push("/settings");
        assert_eq!((r.depth(), r.location()), (2, "/settings".to_string()));
        r.push("/settings");
        assert_eq!(r.depth(), 3, "the same page may be open twice");
        assert!(r.pop());
        assert!(r.pop());
        assert!(!r.pop(), "the last page stays");
        assert_eq!(r.location(), "/");
    }

    #[test]
    fn replace_swaps_the_top_page() {
        let r = router();
        r.push("/settings");
        r.replace("/users/3");
        assert_eq!(keys(&r), ["/", "/users/:id"]);
    }

    #[test]
    fn going_back_to_a_page_already_underneath_keeps_it() {
        let r = router();
        r.go("/users/1");
        let home = r.inner.stack.borrow().pages[0].key;
        r.go("/settings");
        r.go("/users/1");
        assert_eq!(
            r.inner.stack.borrow().pages[0].key,
            home,
            "the same place, the same page — and so the same state"
        );
    }

    #[test]
    fn a_named_route_is_reached_by_its_name() {
        let r = router();
        assert_eq!(
            r.location_for("post", &[("id", "4 2"), ("post", "9")], &[("x", "a&b")]),
            Some("/users/4%202/posts/9?x=a%26b".to_string())
        );
        assert_eq!(
            r.location_for("settings", &[], &[]),
            Some("/settings".to_string())
        );
        assert_eq!(
            r.location_for("user", &[], &[]),
            None,
            "a parameter is missing"
        );
        assert_eq!(r.location_for("nope", &[], &[]), None);
        r.go_named("user", &[("id", "5")], &[]);
        assert_eq!(r.location(), "/users/5");
        r.push_named("settings", &[], &[]);
        assert_eq!(r.location(), "/settings");
    }

    #[test]
    fn a_redirect_sends_the_navigation_elsewhere() {
        let signed_in = Rc::new(Cell::new(false));
        let gate = signed_in.clone();
        let r = GoRouter::new(vec![
            GoRoute::new("/", page),
            GoRoute::new("/login", page),
            GoRoute::new("/account", page),
        ])
        .animate(false)
        .redirect(move |state| {
            (state.location() == "/account" && !gate.get()).then(|| "/login".to_string())
        });
        r.go("/account");
        assert_eq!(r.location(), "/login");
        signed_in.set(true);
        r.go("/account");
        assert_eq!(r.location(), "/account");
    }

    #[test]
    fn a_route_can_redirect_for_itself() {
        let r = GoRouter::new(vec![
            GoRoute::new("/", page),
            GoRoute::new("/old", page).redirect(|_| Some("/new".to_string())),
            GoRoute::new("/new", page),
        ])
        .animate(false);
        r.go("/old");
        assert_eq!(r.location(), "/new");
    }

    #[test]
    fn a_redirect_loop_ends_in_the_error_page() {
        let r = GoRouter::new(vec![
            GoRoute::new("/", page),
            GoRoute::new("/a", page),
            GoRoute::new("/b", page),
        ])
        .animate(false)
        .redirect(|state| match state.location() {
            "/a" => Some("/b".to_string()),
            "/b" => Some("/a".to_string()),
            _ => None,
        });
        r.go("/a");
        assert!(r.inner.stack.borrow().pages[0].node.is_none());
    }

    #[test]
    fn a_refresh_listenable_runs_the_redirects_again() {
        use crate::ValueNotifier;
        let auth = ValueNotifier::new(true);
        let watched = auth.clone();
        let r = GoRouter::new(vec![
            GoRoute::new("/", page),
            GoRoute::new("/login", page),
            GoRoute::new("/home", page),
        ])
        .animate(false)
        .redirect(move |state| {
            let signed_in = watched.get();
            match (state.location(), signed_in) {
                ("/home", false) => Some("/login".to_string()),
                ("/login", true) => Some("/home".to_string()),
                _ => None,
            }
        })
        .refresh_listenable(&auth);
        r.go("/home");
        assert_eq!(r.location(), "/home");
        auth.set(false);
        assert_eq!(
            r.location(),
            "/login",
            "signing out moves the page with nobody navigating"
        );
    }

    #[test]
    fn extra_travels_with_the_navigation() {
        let r = router();
        r.go_with_extra("/settings", 41u32);
        assert_eq!(r.state().unwrap().extra::<u32>().as_deref(), Some(&41));
        assert!(r.state().unwrap().extra::<String>().is_none());
    }

    #[test]
    fn a_push_slides_in_and_a_pop_slides_out() {
        let r = GoRouter::new(routes());
        r.go("/settings");
        assert!(
            r.inner.stack.borrow().transition.is_some(),
            "a page arrives"
        );
        assert!(r.inner.stack.borrow().transition.as_ref().unwrap().forward);
        let mut frames = 0;
        while r.tick(1.0 / 60.0) {
            frames += 1;
            assert!(frames < 600, "the spring settles");
        }
        assert!(r.inner.stack.borrow().transition.is_none());

        r.push("/users/1");
        r.pop();
        assert!(
            !r.inner.stack.borrow().transition.as_ref().unwrap().forward,
            "going back"
        );
    }

    #[test]
    fn a_back_gesture_past_halfway_pops_and_a_short_one_does_not() {
        let r = router();
        r.push("/settings");
        assert!(r.can_go_back());
        r.back_gesture(0.2);
        r.back_gesture_end(0.0);
        while r.tick(1.0 / 60.0) {}
        assert_eq!(r.depth(), 2, "let go early: the page comes back");

        r.back_gesture(0.8);
        r.back_gesture_end(0.0);
        while r.tick(1.0 / 60.0) {}
        assert_eq!(r.depth(), 1, "carried far enough: it goes");
    }

    #[test]
    fn a_route_needs_a_slash_at_the_top_and_none_below() {
        let bad_top = std::panic::catch_unwind(|| GoRouter::new(vec![GoRoute::new("nope", page)]));
        assert!(bad_top.is_err());
        let bad_child = std::panic::catch_unwind(|| {
            GoRouter::new(vec![
                GoRoute::new("/", page).routes(vec![GoRoute::new("/x", page)])
            ])
        });
        assert!(bad_child.is_err());
    }
}

#[cfg(test)]
mod animated_tests {
    use super::*;
    use crate::Container;

    fn page(_: &BuildContext, _: &GoRouterState) -> Container {
        Container::new()
    }

    /// The same moves with the slide on — the default — must land where they land with it off.
    #[test]
    fn the_slide_does_not_change_where_the_stack_ends_up() {
        let router = GoRouter::new(vec![
            GoRoute::new("/", page).routes(vec![GoRoute::new("users/:id", page)]),
            GoRoute::new("/settings", page),
        ]);
        router.go("/users/42?tab=posts");
        assert_eq!(router.location(), "/users/42?tab=posts");
        assert!(router.can_pop());
        assert!(router.pop());
        assert_eq!(router.location(), "/");
    }
}
