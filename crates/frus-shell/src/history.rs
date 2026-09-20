//! An application's **location** and the browser's **history**: who owns which.
//!
//! The application owns *where it is* — a router's stack, whatever it keeps — and it is the
//! only one that knows what a location means. The browser owns the *history*: a list of
//! entries, one of them current, moved by the back and forward buttons. Neither can be left
//! to follow the other blindly, because if both push, one screen becomes two entries, and if
//! both pop, one back press takes two steps.
//!
//! This is the agreement, kept here as a small state machine with no browser in it, so that
//! it can be tried against a stand-in for one:
//!
//! - **The application reports where it is** ([`History::reflect`]) and the history follows.
//!   A location the current entry already holds is nothing. One that an earlier entry holds
//!   is that entry: the browser goes back to it ([`Step::Go`]) rather than adding a copy on
//!   top. Any other is a new entry ([`Step::Push`]), which drops whatever lay ahead, as a
//!   browser does.
//! - **The very first report replaces the entry the page opened at**, never adds to it, so a
//!   redirect on the way in — `/` to `/login` — leaves the back button nowhere to be trapped.
//! - **The browser reports where it went** ([`History::popped`]) and the application is told,
//!   unless the browser is only answering a [`Step::Go`] this side asked for. If the
//!   application ends up somewhere else — it redirected — the next report *replaces* the
//!   entry in place: the address is corrected, not added to.
//! - **A `Go` is answered by the browser later**, and a `pushState` made before the answer
//!   would land on the wrong entry, so nothing is sent between the two.
//!
//! Each entry the shell makes carries its own index as the browser's `state`, which is what
//! lets a page that was reloaded, or a jump the browser made, find its place in the list.

/// What the history has to be made to do: the shell turns each into the browser's call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// A new entry for this location (`pushState`).
    Push(String),
    /// This location, in the current entry (`replaceState`).
    Replace(String),
    /// Move this many entries (`history.go`); negative is back. The browser answers with a
    /// `popstate`.
    Go(isize),
}

/// The list of entries the browser holds, as far as this side knows them.
#[derive(Debug)]
pub(crate) struct History {
    /// One per entry; `None` where this side never saw the location — before the entry the
    /// page was reloaded at, say.
    entries: Vec<Option<String>>,
    /// The current entry.
    index: usize,
    /// Whether the application has been reported at least once.
    reported: bool,
    /// A `Go` was sent and its `popstate` has not come: the location it lands on.
    going: Option<String>,
    /// The application was handed a location from outside and has not answered yet.
    handed: bool,
}

impl History {
    /// A history at its first entry, whose location is not known.
    pub(crate) fn new() -> Self {
        Self {
            entries: vec![None],
            index: 0,
            reported: false,
            going: None,
            handed: false,
        }
    }

    /// The page was opened at `location` (`None` if its address carried none), as the entry
    /// numbered `index` (`None` if it is not one this side made, which is the first).
    pub(crate) fn opened(&mut self, location: Option<String>, index: Option<usize>) {
        let index = index.unwrap_or(0);
        self.entries = vec![None; index + 1];
        self.entries[index] = location;
        self.index = index;
    }

    /// The number the current entry carries in the browser's `state`.
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    /// The browser moved to `location`, which is the entry numbered `index` if it carries a
    /// number. Whether the application is to be told: not when this is the answer to a
    /// [`Step::Go`] that came from it.
    pub(crate) fn popped(&mut self, location: String, index: Option<usize>) -> bool {
        match index {
            Some(index) => {
                if self.entries.len() <= index {
                    self.entries.resize(index + 1, None);
                }
                self.index = index;
                self.entries[index] = Some(location.clone());
            }
            None => {
                // An entry this side did not make: the address was edited by hand, which a
                // browser records as a new entry after the current one.
                self.entries.truncate(self.index + 1);
                self.entries.push(Some(location.clone()));
                self.index = self.entries.len() - 1;
            }
        }
        if self.going.take().is_some_and(|goal| goal == location) {
            return false;
        }
        self.handed = true;
        true
    }

    /// The application says it is at `location`. What, if anything, the browser is to be made
    /// to do about it.
    pub(crate) fn reflect(&mut self, location: &str) -> Option<Step> {
        if self.going.is_some() {
            // The browser has a move in flight; the answer will say where it is.
            return None;
        }
        let here = self.entries[self.index].as_deref();
        if !self.reported {
            self.reported = true;
            self.handed = false;
            return if here == Some(location) {
                None
            } else {
                self.entries[self.index] = Some(location.to_string());
                Some(Step::Replace(location.to_string()))
            };
        }
        if here == Some(location) {
            self.handed = false;
            return None;
        }
        if self.handed {
            // Told to go somewhere and it went elsewhere: the address is corrected in place.
            self.handed = false;
            self.entries[self.index] = Some(location.to_string());
            return Some(Step::Replace(location.to_string()));
        }
        for back in 1..=self.index {
            if self.entries[self.index - back].as_deref() == Some(location) {
                self.index -= back;
                self.going = Some(location.to_string());
                return Some(Step::Go(-(back as isize)));
            }
        }
        self.entries.truncate(self.index + 1);
        self.entries.push(Some(location.to_string()));
        self.index += 1;
        Some(Step::Push(location.to_string()))
    }
}

// ---------------------------------------------------------------------------------------
// Addresses
// ---------------------------------------------------------------------------------------

use crate::application::LocationStrategy;

/// The address that shows `location`: what goes in the browser's address bar.
///
/// `base` is the document's base path, and matters only to [`LocationStrategy::Path`].
pub(crate) fn address_of(strategy: LocationStrategy, base: &str, location: &str) -> String {
    match strategy {
        LocationStrategy::Hash => format!("#{location}"),
        LocationStrategy::Path => {
            let base = normalised(base);
            format!("{base}{}", location.trim_start_matches('/'))
        }
    }
}

/// The location an address stands for, or `None` if it names none: a bare page, which keeps
/// the application's own start.
///
/// `pathname`, `search` and `hash` are the address's own parts, as `window.location` gives
/// them. Only the part `strategy` uses is read.
pub(crate) fn location_of(
    strategy: LocationStrategy,
    base: &str,
    pathname: &str,
    search: &str,
    hash: &str,
) -> Option<String> {
    match strategy {
        LocationStrategy::Hash => {
            let written = hash.strip_prefix('#').unwrap_or(hash);
            if written.is_empty() {
                None
            } else if written.starts_with('/') {
                Some(written.to_string())
            } else {
                Some(format!("/{written}"))
            }
        }
        LocationStrategy::Path => {
            let base = normalised(base);
            let rest = pathname
                .strip_prefix(base.trim_end_matches('/'))
                .unwrap_or(pathname);
            let rest = rest.trim_start_matches('/');
            if rest.is_empty() && search.is_empty() {
                None
            } else {
                Some(format!("/{rest}{search}"))
            }
        }
    }
}

/// `base` with a `/` at both ends.
fn normalised(base: &str) -> String {
    let mut base = base.to_string();
    if !base.starts_with('/') {
        base.insert(0, '/');
    }
    if !base.ends_with('/') {
        base.push('/');
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the addresses ----

    #[test]
    fn a_hash_address_carries_the_location_after_the_sign() {
        assert_eq!(
            address_of(LocationStrategy::Hash, "/", "/users/42?tab=posts"),
            "#/users/42?tab=posts"
        );
        assert_eq!(
            location_of(LocationStrategy::Hash, "/", "/", "", "#/users/42?tab=posts"),
            Some("/users/42?tab=posts".to_string())
        );
    }

    #[test]
    fn a_page_opened_with_no_hash_names_no_location() {
        assert_eq!(
            location_of(LocationStrategy::Hash, "/", "/app/", "", ""),
            None
        );
        assert_eq!(
            location_of(LocationStrategy::Hash, "/", "/app/", "", "#"),
            None
        );
        assert_eq!(
            location_of(LocationStrategy::Hash, "/", "/", "?x=1", ""),
            None,
            "the query before the sign is the host's, not the application's"
        );
    }

    #[test]
    fn a_hash_without_a_leading_slash_is_read_as_a_path() {
        assert_eq!(
            location_of(LocationStrategy::Hash, "/", "/", "", "#settings"),
            Some("/settings".to_string())
        );
    }

    #[test]
    fn a_path_address_is_the_location_under_the_base() {
        assert_eq!(
            address_of(LocationStrategy::Path, "/app/", "/users/42?tab=posts"),
            "/app/users/42?tab=posts"
        );
        assert_eq!(address_of(LocationStrategy::Path, "/", "/"), "/");
        assert_eq!(
            address_of(LocationStrategy::Path, "app", "/x"),
            "/app/x",
            "a base is written with a slash at each end whatever it was given with"
        );
        assert_eq!(
            location_of(
                LocationStrategy::Path,
                "/app/",
                "/app/users/42",
                "?tab=posts",
                ""
            ),
            Some("/users/42?tab=posts".to_string())
        );
    }

    #[test]
    fn the_base_itself_names_no_location() {
        assert_eq!(
            location_of(LocationStrategy::Path, "/app/", "/app/", "", ""),
            None
        );
        assert_eq!(
            location_of(LocationStrategy::Path, "/app/", "/app", "", ""),
            None
        );
        assert_eq!(location_of(LocationStrategy::Path, "/", "/", "", ""), None);
    }

    // ---- the history, against a stand-in for a browser ----

    /// What a browser does with the calls the shell makes: a list of entries, one current,
    /// each with its location and its `state`. `go` is answered later, as a real one is.
    struct Browser {
        entries: Vec<(String, Option<usize>)>,
        at: usize,
        /// The answers to come: (location, state) of the entry moved to.
        due: Vec<(String, Option<usize>)>,
    }

    impl Browser {
        /// A tab that has just opened `location`.
        fn opened_at(location: &str) -> Self {
            Self {
                entries: vec![(location.to_string(), None)],
                at: 0,
                due: Vec::new(),
            }
        }

        fn apply(&mut self, step: Step, index: usize) {
            match step {
                Step::Push(l) => {
                    self.entries.truncate(self.at + 1);
                    self.entries.push((l, Some(index)));
                    self.at += 1;
                }
                Step::Replace(l) => self.entries[self.at] = (l, Some(index)),
                Step::Go(delta) => {
                    let to = (self.at as isize + delta).clamp(0, self.entries.len() as isize - 1);
                    self.at = to as usize;
                    self.due.push(self.entries[self.at].clone());
                }
            }
        }

        /// The back button.
        fn back(&mut self) -> (String, Option<usize>) {
            self.at -= 1;
            self.entries[self.at].clone()
        }

        /// The forward button.
        fn forward(&mut self) -> (String, Option<usize>) {
            self.at += 1;
            self.entries[self.at].clone()
        }

        fn locations(&self) -> Vec<&str> {
            self.entries.iter().map(|(l, _)| l.as_str()).collect()
        }
    }

    /// An application whose whole state is a stack of locations, moved as a router moves.
    struct Stack(Vec<String>);

    impl Stack {
        fn at(&self) -> &str {
            self.0.last().unwrap()
        }

        fn push(&mut self, l: &str) {
            self.0.push(l.to_string());
        }

        fn pop(&mut self) {
            self.0.pop();
        }

        /// What `open_location` does: nothing at the place, a pop for the page underneath, a
        /// stack rebuilt otherwise.
        fn open(&mut self, l: &str) {
            if self.at() == l {
                return;
            }
            if self.0.len() > 1 && self.0[self.0.len() - 2] == l {
                self.0.pop();
            } else {
                self.0 = vec!["/".to_string(), l.to_string()];
                self.0.dedup();
            }
        }
    }

    /// A tab, its history and the application, wired as the shell wires them.
    struct Tab {
        browser: Browser,
        history: History,
        app: Stack,
    }

    impl Tab {
        /// Opens the page at `address` (`None`: bare) on an application starting at `/`,
        /// and hands the address to the application — but has not yet reported back.
        fn start(address: Option<&str>) -> Self {
            let mut tab = Self {
                browser: Browser::opened_at(address.unwrap_or("")),
                history: History::new(),
                app: Stack(vec!["/".to_string()]),
            };
            tab.history.opened(address.map(str::to_string), None);
            if let Some(address) = address {
                tab.app.open(address);
            }
            tab
        }

        /// [`start`](Self::start), then the first report.
        fn open(address: Option<&str>) -> Self {
            let mut tab = Self::start(address);
            tab.sync();
            tab
        }

        /// What the shell does after a message and at the top of a frame: the application's
        /// location, reflected.
        fn sync(&mut self) {
            if let Some(step) = self.history.reflect(self.app.at()) {
                let index = self.history.index();
                self.browser.apply(step, index);
            }
        }

        /// The browser's answer to a `Go`, and anything else it says, reaches the shell.
        fn deliver(&mut self, location: String, state: Option<usize>) {
            if self.history.popped(location.clone(), state) {
                self.app.open(&location);
            }
            self.sync();
        }

        fn settle(&mut self) {
            while let Some((location, state)) = self.browser.due.pop() {
                self.deliver(location, state);
            }
        }

        fn press_back(&mut self) {
            let (location, state) = self.browser.back();
            self.deliver(location, state);
        }

        fn press_forward(&mut self) {
            let (location, state) = self.browser.forward();
            self.deliver(location, state);
        }
    }

    #[test]
    fn the_first_report_replaces_the_entry_the_page_opened_at() {
        let tab = Tab::open(None);
        assert_eq!(
            tab.browser.locations(),
            ["/"],
            "one entry, its address filled in"
        );
    }

    #[test]
    fn a_redirect_on_the_way_in_leaves_no_entry_to_be_trapped_by() {
        // Opened at `/private`, which the application answers with `/login`.
        let mut tab = Tab::start(Some("/private"));
        tab.app.0 = vec!["/login".to_string()];
        tab.sync();
        assert_eq!(
            tab.browser.locations(),
            ["/login"],
            "the address was corrected in place; back has nothing to bounce off"
        );
    }

    #[test]
    fn a_page_opened_at_an_address_starts_the_application_there() {
        let tab = Tab::open(Some("/users/42"));
        assert_eq!(tab.app.at(), "/users/42");
        assert_eq!(tab.browser.locations(), ["/users/42"]);
    }

    #[test]
    fn each_move_of_the_application_is_an_entry() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.app.push("/a/b");
        tab.sync();
        assert_eq!(tab.browser.locations(), ["/", "/a", "/a/b"]);
    }

    #[test]
    fn a_report_that_changes_nothing_adds_nothing() {
        let mut tab = Tab::open(None);
        tab.sync();
        tab.sync();
        assert_eq!(tab.browser.locations(), ["/"]);
    }

    #[test]
    fn the_browsers_back_button_pops_the_application_and_adds_no_entry() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.press_back();
        assert_eq!(tab.app.0, ["/"], "the page went");
        assert_eq!(
            tab.browser.locations(),
            ["/", "/a"],
            "and the entry it left is still ahead, for the forward button"
        );
    }

    #[test]
    fn the_forward_button_brings_the_page_back() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.press_back();
        tab.press_forward();
        assert_eq!(tab.app.at(), "/a");
        assert_eq!(tab.browser.locations(), ["/", "/a"]);
    }

    #[test]
    fn the_applications_own_back_goes_back_in_the_browser_instead_of_adding_an_entry() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.app.pop();
        tab.sync();
        assert!(matches!(tab.browser.due.as_slice(), [(l, _)] if l == "/"));
        tab.settle();
        assert_eq!(tab.app.0, ["/"]);
        assert_eq!(
            tab.browser.locations(),
            ["/", "/a"],
            "the browser is at the first entry, the second still ahead — not `/`, `/a`, `/`"
        );
        assert_eq!(tab.browser.at, 0);
    }

    #[test]
    fn a_move_made_while_the_browser_is_answering_waits_for_the_answer() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.app.pop();
        tab.sync(); // sends the `Go`
        tab.app.push("/b");
        tab.sync(); // must not be pushed onto the entry the `Go` is leaving
        assert_eq!(tab.browser.locations(), ["/", "/a"]);
        tab.settle();
        assert_eq!(
            tab.browser.locations(),
            ["/", "/b"],
            "after the answer, the move is made"
        );
    }

    #[test]
    fn going_back_several_pages_at_once_goes_back_that_far_in_the_browser() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.app.push("/a/b");
        tab.sync();
        tab.app.0 = vec!["/".to_string()];
        tab.sync();
        assert_eq!(tab.browser.at, 0, "two entries back in one move");
        tab.settle();
        assert_eq!(tab.app.0, ["/"]);
    }

    #[test]
    fn an_address_the_application_redirects_is_corrected_in_place() {
        let mut tab = Tab::open(None);
        tab.app.push("/a");
        tab.sync();
        tab.press_back();
        // The application, told `/`, answers `/login`.
        tab.history.popped("/".to_string(), Some(0));
        tab.app.0 = vec!["/login".to_string()];
        tab.sync();
        assert_eq!(tab.browser.locations()[0], "/login");
        assert_eq!(tab.browser.entries.len(), 2, "corrected, not added to");
    }

    #[test]
    fn a_page_reloaded_partway_finds_its_place_from_its_state() {
        // The tab's third entry, reloaded: the earlier two are not known to this side.
        let mut history = History::new();
        history.opened(Some("/a/b".to_string()), Some(2));
        assert_eq!(history.index(), 2);
        assert_eq!(history.reflect("/a/b"), None);
        // The application goes back a page it never saw as an entry: a new entry, which is
        // the honest answer when the entry behind is not known to be that page.
        assert_eq!(history.reflect("/a"), Some(Step::Push("/a".to_string())));
        assert_eq!(history.index(), 3);
    }

    #[test]
    fn an_address_edited_by_hand_is_a_new_entry_after_the_current() {
        let mut history = History::new();
        history.opened(Some("/".to_string()), None);
        history.reflect("/");
        assert!(
            history.popped("/typed".to_string(), None),
            "the application is told"
        );
        assert_eq!(history.index(), 1);
    }

    #[test]
    fn a_new_entry_drops_what_lay_ahead_of_it() {
        let mut history = History::new();
        history.opened(Some("/".to_string()), None);
        history.reflect("/");
        history.reflect("/a");
        // Back to `/`, then somewhere else: `/a` is gone from the browser's list.
        assert!(history.popped("/".to_string(), Some(0)));
        assert_eq!(
            history.reflect("/"),
            None,
            "the application went where it was told"
        );
        assert_eq!(history.reflect("/b"), Some(Step::Push("/b".to_string())));
        assert_eq!(history.index(), 1);
        assert_eq!(
            history.reflect("/a"),
            Some(Step::Push("/a".to_string())),
            "a new entry, not a step back to an entry that no longer exists"
        );
    }

    #[test]
    fn the_answer_to_our_own_go_is_not_repeated_to_the_application() {
        let mut history = History::new();
        history.opened(Some("/".to_string()), None);
        history.reflect("/");
        history.reflect("/a");
        assert_eq!(history.reflect("/"), Some(Step::Go(-1)));
        assert!(!history.popped("/".to_string(), Some(0)));
    }

    #[test]
    fn a_different_answer_than_the_go_asked_for_is_told_to_the_application() {
        let mut history = History::new();
        history.opened(Some("/".to_string()), None);
        history.reflect("/");
        history.reflect("/a");
        history.reflect("/b");
        assert_eq!(history.reflect("/"), Some(Step::Go(-2)));
        // A back press lands the browser one entry short of where the `Go` was headed.
        assert!(history.popped("/a".to_string(), Some(1)));
    }
}
