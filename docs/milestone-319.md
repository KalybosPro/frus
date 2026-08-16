# Milestone 319 — Building from the theme, not merely painting with it

Milestone 318 set out to give `AppBar::center_title` the middle term of
`caller ?? theme ?? framework` and could not, and the reason was worth more than the
change would have been:

> `center_title` is not a painted property. It decides **which children exist and in what
> order**, and by the time a theme is in reach the composition has already been made.

Milestone 309 built the chain and 310 carried it into layout, both by handing a widget the
theme when it is asked how it looks or how big it is. That covers everything a theme
decides *about a widget* and nothing a theme decides *about a composition*. This closes
that.

## The primitive

`ThemeBuilder` takes `theme → widget` and is transparent otherwise: no box, no paint, no
identity of its own, so wrapping something in one does not move it.

```rust
ThemeBuilder::new(|theme| AppBar::new("Inbox").center_title(theme.…).build())
```

Three things had to be true, and each of them shaped it.

**It must build before anything reads its children.** `Widget::children` hands back a
*borrowed slice*, so the children have to exist before the walk arrives. A new hook,
`Widget::build_themed(&self, &Theme)`, is called on the way down by `build_layout` —
**after** the subtree-theme swap, so a builder inside a `Themed` sees that theme and not
the frame's. `hash_node` calls it too: the relayout cache exists to *skip* `build_layout`,
so its walk can be the first one down the tree, and a fingerprint taken of a node with no
children would agree with itself forever.

**It must build once.** The hook takes `&self`, like every other, so the cell is a
`OnceCell` filled through interior mutability. That is safe here for a reason specific to
this framework: a widget tree is rebuilt from `view` rather than mutated, so *once per
instance* and *once per frame* are the same sentence. The closure is `FnOnce`, not `Fn`,
because what it captures is usually a builder holding boxed widgets — a thing that cannot
be produced twice.

**It must keep retained state**, which is the whole difference from `LayoutBuilder`. That
widget also builds late, and its own documentation is blunt about the price: it rebuilds
every frame from a **box**, so it has no persistent focus and no deferred overlays. A theme
is not a box. It is the same frame to frame unless the application changes it, and when it
changes the tree is rebuilt anyway — so the subtree keeps its positional identity, and an
application bar inside one keeps its overflow menu open. There is a test for the identity,
because that claim is the only thing separating the two widgets.

## What it unblocked, immediately

`AppBar::build()` now returns a `ThemeBuilder` and composes inside it. `center_title`
resolves `caller ?? theme ?? platform` — the platform last, because where a title sits is a
system convention before it is a design one — and `AppBarTheme` also carries the title's
type, the background, the foreground, the elevation and the height.

The test counts the springs in the assembled row, since a centred title is *spring, title,
spring* and a flush one is a title after the leading. It asserts first that centring
changes that count at all: an instrument that cannot tell the two apart would let every
other assertion pass while proving nothing.

`Scaffold`, `NavBar` and the pickers are assembled the same way and can follow whenever
someone reaches them.

## The sharp edge, written down

A traversal that reaches a `ThemeBuilder` **without having called `build_themed`** sees a
node with no children. Inside the framework that cannot happen — every traversal is
downstream of `build_layout` or `hash_node` — and `style_themed` builds on demand so a bare
`natural_size` is safe too. But `children()` alone is not self-healing, and the first test
of the app bar's height found exactly that edge by walking children directly. It is called
out at the definition rather than left for the next person to rediscover.

## What the guard caught

Milestone 310 left a test that reads `widget.rs` and `transparent.rs` and insists the
forwarding macro implements **every** hook the trait declares. Adding `build_themed` to
the trait turned it red within the minute, which is exactly the failure it was written for:
without the forward, a `ThemeBuilder` behind a `Keyed` — every list row in the demo — would
never build, and the wrapper would report a node with no children. That test has now caught
two different missing hooks in ten milestones.

## Verification

1070 tests (6 new), clippy silent, goldens unmoved — this changes when a composition is
decided, not what it comes out as.

## Left

- **One consumer.** `Scaffold`, `NavBar`, `Drawer` and the pickers are still assembled
  before a theme is in hand.
- **`children()` is not self-healing**, the above.
- **No scrolled-under elevation** on the bar, carried over from milestone 318: the
  reference raises it to 3 when content passes beneath, and nothing here knows that.
