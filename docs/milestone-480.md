# Milestone 480 — Tabs that are more than a label, and the dots beside them

Closes [#36](https://github.com/KalybosPro/frus/issues/36).

A tab was a label, an icon, or both. That rules out a tab with an unread count beside its
name — which is on half the tab bars ever shipped — a tab with a coloured dot, a tab of
two lines, a tab whose label is rich text.

Three pieces, and the issue asks for a decision on the middle one.

## A tab the caller draws

`tab_widget(child, label, content)`, and `TabItem` behind it with the same shape the last
two milestones settled on: shorthands for the ordinary cases, a value for what they cannot
carry.

**The label is not optional**, even on a tab that draws no word at all. It is what a screen
reader says, and a tab nobody can name is a tab nobody can use. That is not a new demand:
`icon_only_tab` has made it since it was written, for the same reason, and the reference
leaves such a tab nameless and expects the caller to wrap it in something that names it.

**The bar paints nothing of its own on such a tab.** A label drawn under the caller's own
label is the same thing twice. What stays the bar's is everything that makes it a bar: the
press, the ink, the indicator, the row's height, and the announcement.

### The one thing `TabItem` carries that no shorthand does

How wide the tab asks to be, and only when the bar **scrolls**.

A bar that does not scroll shares its width between its tabs in equal parts — the
reference's rule, and the one that keeps a tab still when another is renamed — so it never
asks. When it does scroll, an ordinary tab is measured from its label and a tab the caller
drew cannot be measured at all: a widget cannot be asked how wide it would like to be, which
is [#52](https://github.com/KalybosPro/frus/issues/52) again.

Unsaid, it falls back to its **name**, which is right for a label with a dot beside it and
short for a label with a count in a pill. `TabItem::width` is how a caller says. The test
pins both halves, including that the two come out identical on a bar that does not scroll —
otherwise it would be asserting that a number is used without ever showing it is ignored.

## The swipe: taken, and opt-in

`TabBar::swipeable(true)` makes the panel a [`PageView`] over every tab's content. The
gesture, the fling and the spring to rest are the ones milestone 277 already built, so this
is composition and not machinery.

Off by default, as the issue says it should be: it is a behaviour change for every bar
already written, and a panel that scrolls sideways of its own accord would fight the
gesture rather than answer it.

Two things follow from taking it, and both are the point rather than side effects:

- **Every tab's content is kept**, not only the one showing. The page view has to be able to
  build the tab you are dragging *towards*. The panels were already built by the caller
  either way — they arrive by value — so what changes is that they are held rather than
  dropped.
- **The set takes the height it is offered**, as it already takes the width, and for the
  same reason the existing comment gives for the width: a paged panel sized by whichever
  page happens to be longest would make the control change height as the finger moved
  through it.

The message a swipe produces is the **same one a press on the tab produces**. One rule
reached two ways, rather than a second path into the selection that can drift from the
first.

What the swipe does not do is drag the indicator with the finger. `on_page_changed` reports
where the finger settled, so the indicator slides to the new tab when the page lands rather
than tracking the drag. Tracking it needs the bar to read the panel's live scroll offset,
which is a different piece of plumbing.

## `TabPageSelector`

The row of dots that says which page of several you are on. A module of its own rather than
a corner of the tab bar's, because it pairs with `PageView` as much as with a bar — and
because the tab bar's own widgets are **disableable** and this is not. A read-out has no
disabled state: it says where you are, or it is not there.

That is not a stylistic split. The crate has a tripwire that reads every module holding an
`enabled: bool` and checks that each hook it implements consults the flag; a widget with no
disabled state living in such a module trips it, correctly, and the answer is that it does
not live there.

**The fill crosses rather than jumping.** Each dot's fill is `1 - |index - t|`, clamped,
against the same fractional index the bar's indicator slides along — so two dots either side
of a crossing share exactly one dot's worth of ink between them. The row never reads as two
filled dots or as none. The fill is an **alpha**, not a lerp towards the ring's colour,
because a dot on a dark surface and one on a light surface would need opposite lerps and an
alpha is the same answer on both.

**It takes no press.** A twelve-pixel target is not one, and a row of them is a row of
controls a finger cannot tell apart — the pager beside it in the demo's tour is the control.

**It is announced**, which the reference's is not: *page two of four* is exactly what it is
drawing, and it is the whole of its meaning to anyone who cannot see it.

## Found on the way, and not fixed here

A **scrollable** tab bar pushes whatever is under it two hundred pixels down. The strip goes
inside a horizontal `SingleChildScrollView`, whose height defaults to 200 — a default that
belongs to the axis a scroll actually scrolls. The bar paints its hairline at its real
height, so it looks right until something is placed below it.

Nothing in the suite had ever rendered a scrollable bar with a panel under it; the picture
for this milestone did, which is how it turned up. Giving the non-scrolling axis `Auto`
collapses the strip to nothing instead — a scroll host lays its content out *against* the
viewport, so on the axis that does not scroll the content is given the viewport's size,
which is then `Auto`, and the circle closes at zero. Tried and reverted, with the whole
golden suite green either way, so nothing depends on the 200 today.

It is [#65](https://github.com/KalybosPro/frus/issues/65), and it is a change to the scroll
branch of `ui.rs` rather than to either widget. The golden here uses a bar that does not
scroll.

## Not here

- **`DefaultTabController`**, which the issue says should not exist here and is right: the
  selected index is application state and is already passed in.
- **An indicator that follows the finger** during a swipe, as above.

## In the demo

The tour screen's footer now has the dots above its pager: the same answer twice, once as a
read-out and once as a control, which is the clearest way to show that they are not the same
kind of thing.

## The picture

`tabs_widget_labels`: a count in a pill beside one label, a coloured dot beside another, an
ordinary tab, an icon-only one, and the row of dots underneath.
