# Animation

Most movement in an application is a value that changes — a box that grows, a hint that fades,
a number that counts up — and should get there smoothly rather than at once. In frus that is an
**implicit** animation: you build the widget with the new value, as you would without
animation, and it moves there over the time you give it.

## A box that changes: `AnimatedContainer`

`AnimatedContainer` is a `Container` whose size, colour, corners and padding move to their new
values. The duration is in seconds; the curve says how it accelerates.

```rust
use frus::{component, AnimatedContainer, Button, Column, Curve, Text, Widget};

fn growing_card() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let open = cx.use_state(|| false);
        let on = open.get();
        let scheme = &cx.theme().scheme;
        let toggle = open.clone();
        Box::new(
            Column::new()
                .spacing(16.0)
                .child(
                    Button::new(if on { "Shrink" } else { "Grow" })
                        .on_press(move || toggle.update(|on| *on = !*on)),
                )
                .child(
                    AnimatedContainer::new(0.4, Curve::ease_in_out())
                        .color(if on { scheme.primary } else { scheme.tertiary })
                        .size(if on { 260.0 } else { 140.0 }, if on { 120.0 } else { 72.0 })
                        .radius(if on { 32.0 } else { 8.0 })
                        .padding(16.0)
                        .child(Text::new(if on { "Expanded" } else { "Compact" })),
                ),
        )
    }))
}
```

## Fading, sliding, switching

The same shape works for one property at a time — `AnimatedOpacity`, `AnimatedPadding`,
`AnimatedAlign`, `AnimatedPositioned`, `AnimatedSize` (a box that follows the size of its
child), `AnimatedDefaultTextStyle`:

```rust
use frus::{component, AnimatedOpacity, Button, Column, Curve, Text, Widget};

fn hint() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let shown = cx.use_state(|| true);
        let toggle = shown.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .child(Button::new("Toggle the hint").on_press(move || toggle.update(|s| *s = !*s)))
                .child(AnimatedOpacity::new(
                    if shown.get() { 1.0 } else { 0.0 },
                    0.3,
                    Curve::ease_in_out(),
                    Text::new("Swipe left to archive"),
                )),
        )
    }))
}
```

When what changes is **which** widget is shown — a number that goes up, an icon that becomes
another — `AnimatedSwitcher` fades the old one out and the new one in. It is given a value and
a function that builds the widget for a value; a new value starts the transition.

```rust
use frus::{component, AnimatedSwitcher, Button, Column, Text, Widget};

fn counter() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let count = cx.use_state(|| 0_i32);
        let add = count.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .child(AnimatedSwitcher::new(0.3, count.get(), |n: &i32| {
                    Text::new(n.to_string()).size(40.0)
                }))
                .child(Button::new("+1").on_press(move || add.update(|n| *n += 1))),
        )
    }))
}
```

`AnimatedCrossFade` does the same between exactly two children, and resizes between them.

## Curves

`Curve::Linear`, `Curve::ease_in_out()`, `Curve::ease_out()`, `Curve::ease_in()` and the
emphasised curves of the design system are all there; `Curve::Cubic { x1, y1, x2, y2 }` is any
other, with the control points of CSS's `cubic-bezier()`.
A short duration and an easing curve — 150 to 300 ms, `ease_out` for something arriving — is
what feels responsive.

## What frus animates for you

Several movements need no code: the page transition when the router pushes or pops a screen
(and the back gesture that follows the finger), a button's ripple, a switch's thumb, a text
field's floating label, a scroll's fling and its overscroll. They follow the platform, and the
theme's `page_transitions` changes them.

`Hero` flies a widget from one screen to the next — the photo of a list row that becomes the
header of its page — when both screens wrap it in a `Hero` of the same tag.

## Animations you drive

When the movement is not a value reaching another — a spinner that turns forever, a progress
that follows a gesture — an `AnimationController` holds a progress from 0 to 1 that you start,
reverse, repeat and stop, and the `*Transition` widgets (`FadeTransition`, `SlideTransition`,
`ScaleTransition`, `SizeTransition`, `DecoratedBoxTransition`) draw from it. The widget gallery
(`frus-examples/widgets`, Animation → Transitions) shows one driven by buttons and a slider.

People who prefer the system's reduced motion setting get it: `MediaQuery::of()` reports it,
and frus's own transitions honour it.
