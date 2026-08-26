//! [`ScaffoldInfo`]: **what the shell knows and its slots do not.**
//!
//! An [`AppBar`](crate::AppBar) is handed to a [`Scaffold`](crate::Scaffold) already
//! built, so it cannot see the shell it is about to stand in. The reference's does, through
//! the context: `Scaffold.of(context)` tells a bar whether the screen has a drawer, which
//! is how a bar with no `leading` grows a menu button on a screen that has a menu and
//! stays empty on one that does not (`app_bar.dart:1010`).
//!
//! This is that context, in the shape frus already uses twice: an ambient value the walk
//! installs on the way down, like [`MediaQuery`](crate::MediaQuery) and the theme, and a
//! [`Widget`] hook — [`Widget::scaffold_override`] — for the node that imposes one.
//!
//! It carries **messages**, which is the part that needed care: the shell knows what opens
//! its drawer and the bar does not, and the two are generic over different type parameters
//! as far as the ambient is concerned. The message is held as `Rc<dyn Any>` and handed back
//! only to a caller that names the type it was put in with. A bar whose `Msg` is not the
//! shell's asks and is told nothing, which is the honest answer: it could not have sent
//! that message anyway.
//!
//! ```ignore
//! ScaffoldScope::new(ScaffoldInfo::default().with_drawer(Msg::ToggleMenu), shell)
//! ```

use std::any::Any;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::widget::Widget;

/// What the [`Scaffold`](crate::Scaffold) around a widget knows about itself.
///
/// Empty outside a shell — which is not the same as "a shell with nothing in it", but the
/// two want the same answers here, and a bar outside a shell has no drawer to open either
/// way.
#[derive(Clone, Default)]
pub struct ScaffoldInfo {
    /// The message that opens or closes the leading drawer, if the shell has one.
    drawer: Option<Rc<dyn Any>>,
    /// The same for the trailing drawer.
    end_drawer: Option<Rc<dyn Any>>,
}

impl ScaffoldInfo {
    /// The one in force here — empty outside every [`ScaffoldInfo::scope`].
    #[must_use]
    pub fn of() -> Self {
        AMBIENT.with(|a| a.borrow().clone())
    }

    /// Records a leading drawer and the message that toggles it.
    #[must_use]
    pub fn with_drawer<Msg: Clone + 'static>(mut self, toggle: Msg) -> Self {
        self.drawer = Some(Rc::new(toggle));
        self
    }

    /// Records a trailing drawer and the message that toggles it.
    #[must_use]
    pub fn with_end_drawer<Msg: Clone + 'static>(mut self, toggle: Msg) -> Self {
        self.end_drawer = Some(Rc::new(toggle));
        self
    }

    /// Whether the shell has a drawer on the leading edge.
    #[must_use]
    pub fn has_drawer(&self) -> bool {
        self.drawer.is_some()
    }

    /// Whether the shell has a drawer on the trailing edge.
    #[must_use]
    pub fn has_end_drawer(&self) -> bool {
        self.end_drawer.is_some()
    }

    /// The message that toggles the leading drawer, for a widget whose message type it
    /// **is**. `None` for anyone else: a widget cannot send a message of another tree's
    /// type, so being told about it would only invite a cast.
    #[must_use]
    pub fn drawer_toggle<Msg: Clone + 'static>(&self) -> Option<Msg> {
        self.drawer.as_ref()?.downcast_ref::<Msg>().cloned()
    }

    /// The same for the trailing drawer.
    #[must_use]
    pub fn end_drawer_toggle<Msg: Clone + 'static>(&self) -> Option<Msg> {
        self.end_drawer.as_ref()?.downcast_ref::<Msg>().cloned()
    }

    /// Puts this in force until the guard is dropped.
    #[must_use]
    pub fn install(self) -> ScaffoldGuard {
        ScaffoldGuard(AMBIENT.with(|a| a.replace(self)))
    }

    /// Runs `body` with this in force, and puts back what was there.
    pub fn scope<R>(self, body: impl FnOnce() -> R) -> R {
        let _guard = self.install();
        body()
    }

    /// The part of this that changes what a slot **composes**, for the relayout
    /// fingerprint.
    ///
    /// Whether there is a drawer decides whether a bar grows a button, which is a
    /// different row and a different geometry. *Which* message opens it does not: two
    /// shells with different messages and the same drawers lay out identically, and
    /// hashing the message would mean hashing a pointer that moves every frame.
    pub(crate) fn shape_hash<H: Hasher>(&self, hasher: &mut H) {
        self.has_drawer().hash(hasher);
        self.has_end_drawer().hash(hasher);
    }
}

/// Puts back the previous ambient shell when dropped — including while a panic unwinds.
pub struct ScaffoldGuard(ScaffoldInfo);

impl Drop for ScaffoldGuard {
    fn drop(&mut self) {
        AMBIENT.with(|a| a.replace(std::mem::take(&mut self.0)));
    }
}

thread_local! {
    /// The shell in force on this thread. A `RefCell` rather than a `Cell` because the
    /// value is not `Copy`: it holds the messages, and a message is whatever the
    /// application says it is.
    static AMBIENT: RefCell<ScaffoldInfo> = const { RefCell::new(ScaffoldInfo {
        drawer: None,
        end_drawer: None,
    }) };
}

/// A **shell for one subtree**: the counterpart of [`MediaScope`](crate::MediaScope), one
/// milestone later and for the other thing a shell knows.
///
/// A **transparent wrapper**: it lays out, paints and answers exactly
/// as its child does. The only thing it adds is what the subtree below it reads from
/// [`ScaffoldInfo::of`] — including a subtree deferred until
/// [`Widget::build_themed`](crate::Widget::build_themed), which is where an
/// [`AppBar`](crate::AppBar) is composed and therefore the whole point.
pub struct ScaffoldScope<Msg> {
    info: ScaffoldInfo,
    inner: Box<dyn Widget<Msg>>,
}

impl<Msg> ScaffoldScope<Msg> {
    /// Tells `child`'s subtree what shell it is in.
    pub fn new(info: ScaffoldInfo, child: impl Widget<Msg> + 'static) -> Self {
        Self {
            info,
            inner: Box::new(child),
        }
    }

    /// Transparent: it does not change the box its child is.
    fn restyle(&self, base: frus_layout::Style) -> frus_layout::Style {
        base
    }
}

crate::transparent::forward_transparent!(ScaffoldScope {
    fn key(&self) -> Option<u64> {
        self.inner.key()
    }
    fn positioned(&self) -> Option<crate::positioned::Positioning> {
        self.inner.positioned()
    }
    fn theme_override(&self, inherited: &crate::theme::Theme) -> Option<Box<crate::theme::Theme>> {
        self.inner.theme_override(inherited)
    }
    fn media_override(&self, inherited: crate::MediaQuery) -> Option<crate::MediaQuery> {
        self.inner.media_override(inherited)
    }
    /// The shell this scope names — unless the child is itself a scope, in which case the
    /// **inner** one wins, exactly as a nested `Scaffold` wins in the reference.
    fn scaffold_override(&self) -> Option<ScaffoldInfo> {
        Some(
            self.inner
                .scaffold_override()
                .unwrap_or_else(|| self.info.clone()),
        )
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Container, Runtime, Size, Theme};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Menu,
    }

    /// Another tree's message type, which the shell's message is not.
    #[derive(Clone, Debug, PartialEq)]
    enum Other {
        #[allow(dead_code)]
        Menu,
    }

    /// Outside a shell there is no shell, and the answer says so rather than inventing one.
    #[test]
    fn there_is_no_shell_until_one_says_so() {
        let bare = ScaffoldInfo::of();
        assert!(!bare.has_drawer() && !bare.has_end_drawer());
        assert_eq!(bare.drawer_toggle::<Msg>(), None);
    }

    /// The message comes back to the tree it was put in for, and to nobody else.
    #[test]
    fn a_message_is_handed_back_only_to_the_type_it_was_put_in_with() {
        ScaffoldInfo::default().with_drawer(Msg::Menu).scope(|| {
            let info = ScaffoldInfo::of();
            assert!(info.has_drawer());
            assert_eq!(info.drawer_toggle::<Msg>(), Some(Msg::Menu));
            // A tree of another message type is told nothing, because it could not
            // have sent this message anyway.
            assert_eq!(info.drawer_toggle::<Other>(), None);
        });
        // And the scope is closed behind it.
        assert!(!ScaffoldInfo::of().has_drawer());
    }

    /// The scope reaches the subtree through the walk, not only the call that installs it —
    /// which is what an already-built app bar needs.
    #[test]
    fn a_scope_reaches_the_subtree_the_walk_visits() {
        let seen = std::rc::Rc::new(std::cell::Cell::new(false));
        let probe = {
            let seen = seen.clone();
            crate::ThemeBuilder::<Msg>::boxed(move |_| {
                seen.set(ScaffoldInfo::of().has_drawer());
                Box::new(Container::new().color(frus_core::Color::WHITE))
            })
        };
        let tree = ScaffoldScope::new(ScaffoldInfo::default().with_drawer(Msg::Menu), probe);
        build_ui(
            &tree,
            Size::new(100.0, 100.0),
            &Runtime::default(),
            &Theme::default(),
        );
        assert!(seen.get(), "the deferred build did not see the shell");
    }

    /// Two shells that differ in what they hold must not share a layout: a drawer is a
    /// button in the bar, and a button is a different row. The cache keys on a fingerprint
    /// of the walk, so the shell has to be in it — the same trap `theme_override` and
    /// `media_override` each document beside themselves.
    #[test]
    fn the_shell_is_part_of_the_relayout_fingerprint() {
        // One runtime across both builds, so the second really does consult the first's
        // cache rather than starting from nothing.
        let runtime = Runtime::default();
        let height = |drawer: bool| {
            let probe = crate::ThemeBuilder::<Msg>::boxed(move |_| {
                // A shell with a drawer composes a taller box, the way a bar with a menu
                // button composes a different row.
                let tall = if ScaffoldInfo::of().has_drawer() {
                    40.0
                } else {
                    10.0
                };
                Box::new(
                    Container::new()
                        .width(10.0)
                        .height(tall)
                        .color(frus_core::Color::rgb8(255, 0, 128)),
                )
            });
            let info = if drawer {
                ScaffoldInfo::default().with_drawer(Msg::Menu)
            } else {
                ScaffoldInfo::default()
            };
            let tree = ScaffoldScope::new(info, probe);
            build_ui(&tree, Size::new(100.0, 100.0), &runtime, &Theme::default())
                .scene()
                .primitives()
                .iter()
                .find_map(|p| match p {
                    frus_core::Primitive::Rect { rect, .. } => Some(rect.height),
                    _ => None,
                })
        };
        let with = height(true);
        let without = height(false);
        assert_ne!(
            with, without,
            "a shell with a drawer was served from the cache of one without"
        );
        assert_eq!(height(true), with, "and back again");
    }
}
