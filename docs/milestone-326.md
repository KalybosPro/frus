# Milestone 326 — An overlay belongs to its screen

The red item on the roadmap, found on a device during milestone 324's pass: open the home
screen's app-bar overflow menu, choose "Settings →", and the menu is still there — fully
opaque, over the Settings screen.

Two separate things were wrong, one in the framework and one in the application, and the
interesting part is that neither of the two obvious explanations was right.

## What it was not

**It was not retained state leaking.** The menu's open flag lives in the application's
model, and the walk rebuilds from that model every frame. Nothing in the `Runtime` was
holding a stale overlay.

**It was not an overlay whose owner had left the tree.** The roadmap entry guessed that a
`Navigator` push replaced the screen while the overlay registered by the outgoing one was
still being collected. It is not what happens: during a transition the outgoing screen is
genuinely still in the tree — that is what a transition *is* — and once the spring settles
it is gone, menu included. Reproducing it through `Application::view` and reading the scene
primitives, as the entry itself suggested, is what showed this. The menu disappears at the
end of the transition on its own.

## What it was

The reproduction printed the menu item's position on every frame of the transition, and it
never moved. Pinned, while the screen it belongs to slid away underneath it.

That is two mechanisms compounding:

1. `process_overlays` runs **after** both screens are walked and paints above the **whole
   window**. A deferred overlay therefore outranks everything, including the screen drawn
   on top of its owner.
2. The transition is **parallaxed** — the outgoing screen travels only 30 % of the width —
   so the app bar the menu hangs from never actually leaves the window. Nothing was ever
   going to correct this on its own, and the auto-flip that keeps a menu inside the window
   pinned it in place besides.

The rule is that an overlay belongs to a screen, and a screen on its way out takes its
floating layers with it. `Navigator::from` inserts the screen being left at index 0, so
`children[1]` is always the destination — on a push, on a pop, and under a back gesture
alike, which is worth checking rather than assuming, because on a **pop** the screen being
left is the *front* one. Whatever the other screen defers is dropped.

## A second bug the first one hid

While reading `process_overlays`, the auto-flip turned out to have its own version of the
same mistake. It nudges an anchored overlay back inside the window when it overflows an
edge — which is right for a menu opened near the right margin, and assumes the window is
showing the anchor. When the anchor has **left** the window, the nudge does the opposite of
its job: it drags into view a menu belonging to something nobody can see, and leaves a
window-wide dismissal barrier behind it that swallows the next press anywhere on screen.

A portal in a horizontally scrolled row is enough to hit it, no navigator required. An
overlay whose anchor is off screen now goes off screen with it, and takes its barrier along.

This one is not the reported defect — the parallax means the anchor never left, so it was
not what the device saw. It is a bug found by reading the code the reported defect led to,
which is the ordinary reason to read code around a fix.

## The application's half

The device report had a second observation: the menu also **came back** on returning home.
That one is not the framework's. `Msg::Push` already dismissed the drawer and the popup
menu and simply missed the app bar's overflow, so its flag survived the round trip.

Whether choosing a menu item dismisses the menu is genuinely the application's to decide —
half of these actions are toggles that want the menu to stay — so the fix is in `update`,
and it is scoped to the messages that navigate.

## Verification

Three tests, each shown to fail without its fix:

- `a_departing_screens_overlay_is_not_drawn_over_the_incoming_one` covers a push, a pop,
  the destination's *own* overlay (which must be untouched), and the no-transition case.
- `an_overlay_whose_anchor_left_the_window_goes_with_it` covers the auto-flip, checking both
  the drawn panel and `top_dismiss`.
- `navigating_from_the_overflow_menu_closes_it` covers the application half, including that
  a non-navigating action leaves the menu alone.

No golden moved, which is the expected answer: nothing here changes how a settled screen is
drawn.

## Left

- **The suppression is per-screen, not per-depth.** A `Navigator` inside a `Navigator` works
  because the truncation only removes what that subtree pushed, but the rule "index 1 is the
  destination" is a convention of this widget rather than something the type system holds.
- **An overlay is still positioned against the window, not against its screen.** Dropping the
  departing screen's overlays sidesteps that; it does not fix it. A menu on a screen that is
  half off the window would still be laid out as though the screen were whole. The honest
  version anchors an overlay to its screen's slice, which is a larger change to
  `process_overlays`.
- **The two remaining device findings** — a task row's avatar that does not open the task,
  and the missing `surface_container_highest` behind milestone 325's slider rail.
