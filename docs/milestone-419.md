# Milestone 419 — A footer alone held nothing off the bottom edge

The `Scaffold` leaves the bottom clearance to whatever is bottom-most. With a bottom bar
there, the bar takes it; with nothing there, the body takes it as a sibling spacer. And with
a **footer** there:

```rust
let body_owns_bottom = !extend_body && footer.is_none() && !bar_below_body;
```

the body steps aside — because the footer is below it — and the footer takes nothing:

```rust
stack = stack.child_boxed(inset_pad(
    Box::new(Container::new().padding(FOOTER_PAD).child(row)),
    0.0,
    insets.right,
    0.0,          // ← the bottom
    insets.left,
));
```

A literal zero, sitting under a comment that says the content "keeps clear of the side
intrusions". It was only ever about the sides. So a screen with persistent footer buttons and
no navigation bar put its **Save** and **Cancel** on the gesture bar.

## What the reference does

```dart
// scaffold.dart:3130
Container(
  decoration: … Border(top: Divider.createBorderSide(context, width: 1.0)) …,
  child: SafeArea(top: false, child: … Padding(padding: EdgeInsets.all(8), …)),
),
_ScaffoldSlot.persistentFooter,
removeLeftPadding: false,
removeTopPadding: true,
removeRightPadding: false,
removeBottomPadding: widget.bottomNavigationBar != null,
maintainBottomViewPadding: !_resizeToAvoidBottomInset,
```

Two halves, as in milestones 417 and 418. The shell hands the slot a **description** — top
removed, sides kept, bottom removed *only when a navigation bar is below it* — and the
footer's own `SafeArea(top: false)` consumes what is left. The decoration stays **outside**
that safe area, which is why the rule and the background still run to the screen's edge.

frus already had the decoration on the right side of the safe area. What it did not have was
a safe area: the padding was worked out inline, and the bottom was never in it.

## What changed

- The footer's inner padding is a real `SafeArea` with the top edge freed, not an
  `inset_pad` with numbers computed at the call site.
- The slot is wrapped in a `MediaScope` carrying the description: top zero, the sides as the
  shell resolved them, and `bottom_clear` unless a bar below already holds the edge off.
- `bottom_clear` and `bar_below_body` moved up the function, above the footer that now needs
  them. Nothing about how either is worked out changed — `bottom_clear` is still the answer
  to "which bottom intrusion, given what this screen said about the keyboard", and the long
  comment explaining that moved with it.

## The test, and both of its halves

`a_footer_holds_the_bottom_edge_off_unless_a_bar_below_it_does` fails in **both** directions,
which is what makes it worth having:

- pinned to the old zero, the buttons land at `y + height = 788` on an 800-pixel screen with
  a 24-pixel gesture bar — sitting on it;
- pinned to always taking the bottom, the second half fails: with a navigation bar below it,
  the footer would pad for an intrusion the bar already holds off, and the gap between the
  footer's own bottom edge and its buttons would exceed a plain footer padding.

## Still open

Two slots left, each its own step:

- **the rail**, which the reference safe-areas on its leading edge, its top and its bottom
  (`navigation_rail.dart:553`) — and whose width the footer's row arithmetic reads, so the
  two move together;
- **the body**, which the reference does not inset at all (`scaffold.dart:3029`): it hands it
  a description with the top removed when there is an app bar and the bottom removed when
  there is a bar below, and a body that wants the notch avoided says `SafeArea` itself.
