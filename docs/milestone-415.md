# Milestone 415 — A tree built two ways, and the traversals that arrive first

Milestone 414 ended by refusing to give `TextField` a theme-resolved type, because the caret
path has no theme. Looking for where to put one turned up something worse, one layer down.

## The shell builds a tree twice, and only one of the two is finished

A `ThemeBuilder` — and everything built on one, an `AppBar` included — **has no children at
all** until something calls `build_themed` on it. `children()` says so in its own doc, and
has since it was written:

```rust
fn children(&self) -> &[Box<dyn Widget<Msg>>] {
    // Unbuilt, this is empty — which is why `build_themed` runs on the way down in
    // the layout pass, before anything reads it. A traversal that arrives first
    // should call `build_themed`, not this.
    self.built.get().map(Vec::as_slice).unwrap_or(&[])
}
```

The frame path obeys that. `view` → `build_ui`, and `build_ui` calls `build_themed` on the
way down before anything reads a child.

**The shell has a second build path, and it does not.** On a burst of keystrokes — a software
keyboard, `adb input text`, auto-repeat — keys arrive faster than a frame, so the next key
must see the current value rather than the retained tree's:

```rust
self.tree = Some(
    self.media_query(width, height)
        .scope(|| self.app.view(&theme)),
);
```

`view` alone. No layout pass. And that tree is read **immediately**, by `reveal_caret` on the
very next key, through `find_widget` — which walks `children()`.

So: type fast into a field that lives under an `AppBar` or any `ThemeBuilder`, and every
traversal into that subtree returns nothing. No field found, no caret revealed, no focus
resolved. Thirty-eight call sites in `app.rs` reach the tree that way.

## The failure mode is silence

Every one of those call sites is an `and_then` chain ending in `?` or `unwrap_or`. Nothing
logs, nothing panics, nothing looks wrong in a screenshot. The caret simply stops following
while you type, and starts again when a frame lands.

That is why this milestone ships a **tripwire** rather than a comment. Reading an unbuilt
`ThemeBuilder`'s children panics in debug and names the fix:

```
a ThemeBuilder's children were read before it was built: a traversal reached this
tree before any layout pass did. Call `build_deferred(tree, &theme)` on it first
```

The rule had been written down for three milestones. Writing it down is not enforcing it.

## `build_deferred`

The half of the walk that prepares a tree, and nothing else:

```rust
pub fn build_deferred<Msg>(root: &dyn Widget<Msg>, theme: &Theme) {
    fn walk<Msg>(widget: &dyn Widget<Msg>, theme: &Theme) {
        let scoped = widget.theme_override(theme);
        let theme = scoped.as_deref().unwrap_or(theme);
        widget.build_themed(theme);
        for child in widget.children() {
            walk(child.as_ref(), theme);
        }
    }
    walk(root, theme);
}
```

**The theme swap is not optional.** A builder inside a `Themed` has to see its own subtree's
theme, and a preparation that is *nearly* the walk is worse than none: what it builds is then
wrong rather than absent, and wrong is the harder of the two to notice. There is a test for
exactly that — a builder under a `Themed` records the radius it was handed, and it is the
subtree's 99 and not the frame's default.

## A mistake in my own fix

The first version called `build_deferred` **after** the `MediaQuery::scope` closure, so the
reader's font setting was already uninstalled while the deferred subtrees were being built. A
builder that measures text would have measured it at scale 1 and the frame would have laid it
out at 1.3.

That is milestone 408's lesson — the frame's own path holds one surface across the build *and*
the layout — reintroduced by hand three milestones later, in the commit fixing the same class
of bug. It is inside the scope now.

## What this does not fix

**No test drives the path that broke.** The tripwire guards the class, and the two new tests
prove `build_deferred` does what the layout pass does — but they build trees by hand. The
shell's burst path has no test at all, because nothing in this repo drives a whole frame with
the shell's state around it. That is a standing 🔴 and this milestone is one more argument
for it: the bug lived in the eleven lines of the codebase that no test reaches.

`TextField`'s type still does not read the theme. This was the blocker under the blocker;
removing it makes that step possible, not done.
