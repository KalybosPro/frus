title: `NavBar` collapses around its back button when given no width
labels: good first issue, bug, widgets

`NavBar::paint` centres the title in `bounds`, which only makes sense at full width —
but its `style` asks for `Dimension::Auto`. Given no width by its parent, it hugs the
back button and paints the title *underneath* it.

Every screen in the demo happens to give it a width, which is why this has never shown
in the application. It was found by milestone 296, when the widget got its first
golden.

### Where

`crates/frus-widgets/src/nav_bar.rs`.

### What to do

Decide what a `NavBar` with no width from its parent should be — almost certainly
"fill the space it is offered", the way an app bar does — and make `style` say so.
Then check the title lands where it should at several widths.

### Done when

A pixel test in `crates/frus-test/tests/widgets.rs` renders a `NavBar` whose parent
gives it no width, and the title is centred in the frame rather than sitting under the
button. The existing goldens must not move.

Good first bug: small, real, already diagnosed, and it comes with a way to see it.
