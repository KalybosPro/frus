//! `GestureDetector` — taps, double taps, long presses, secondary clicks and drags on any
//! widget, answered with your own handlers.
//!
//! It adds no look of its own (no ink, no hover): it is the widget it wraps. For a
//! pressable surface with feedback, use `InkWell` or a button.

use frus::{
    Align, Alignment, Column, Container, GestureDetector, PanAxis, Point, Positioned, Stack,
    StackFit, State, StateContext, StatefulWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(GestureDemo.into_widget())
}

/// The demo is a small `StatefulWidget`: the state remembers the last gesture and where
/// the dragged chip is, and `cx.handler` turns a reported value into a state change.
struct GestureDemo;

struct GestureDemoState {
    last: String,
    taps: u32,
    /// Where the free chip sits in its box.
    chip: Point,
    /// Where the horizontal-only handle sits along its track.
    handle: f32,
}

impl StatefulWidget for GestureDemo {
    type State = GestureDemoState;

    fn create_state(&self) -> GestureDemoState {
        GestureDemoState {
            last: "Nothing yet".to_string(),
            taps: 0,
            chip: Point::new(16.0, 16.0),
            handle: 0.0,
        }
    }
}

/// The free chip's box and the chip itself, in logical pixels.
const AREA: (f32, f32) = (300.0, 160.0);
const CHIP: (f32, f32) = (72.0, 36.0);
const TRACK: f32 = 260.0;
const HANDLE: f32 = 32.0;

impl State for GestureDemoState {
    type Widget = GestureDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let scheme = &cx.theme().scheme;

        // Four discrete gestures on one pad. With a double tap set, a single tap waits
        // 300 ms to be sure it is not the first half of one.
        let pad = GestureDetector::new(
            Container::new()
                .width(AREA.0)
                .height(110.0)
                .padding(16.0)
                .radius(12.0)
                .color(scheme.secondary_container)
                .alignment(Alignment::CENTER)
                .child(
                    Text::new("Tap, double-tap, long-press or right-click")
                        .color(scheme.on_secondary_container),
                ),
        )
        .on_tap(cx.callback(|s| {
            s.taps += 1;
            s.last = format!("Tap (#{})", s.taps);
        }))
        .on_double_tap(cx.callback(|s| s.last = "Double tap".to_string()))
        .on_long_press(cx.callback(|s| s.last = "Long press".to_string()))
        .on_secondary_tap(cx.callback(|s| s.last = "Secondary tap (right click)".to_string()));

        // A drag: `on_pan_update` reports where the pointer is and how far it moved.
        let moved = cx.handler(|s: &mut Self, delta: Point| {
            s.chip.x = (s.chip.x + delta.x).clamp(0.0, AREA.0 - CHIP.0);
            s.chip.y = (s.chip.y + delta.y).clamp(0.0, AREA.1 - CHIP.1);
            s.last = format!("Pan to ({:.0}, {:.0})", s.chip.x, s.chip.y);
        });
        let ended = cx.handler(|s: &mut Self, velocity: Point| {
            s.last = format!("Pan end, {:.0} px/s across", velocity.x);
        });
        let chip = GestureDetector::new(
            Container::new()
                .width(CHIP.0)
                .height(CHIP.1)
                .radius(18.0)
                .color(scheme.primary)
                .alignment(Alignment::CENTER)
                .child(Text::new("Drag").color(scheme.on_primary)),
        )
        .on_pan_update(move |_local, delta| moved(delta))
        .on_pan_end(ended);
        let free_area = Stack::new()
            .width(AREA.0)
            .height(AREA.1)
            .fit(StackFit::Expand)
            .layer(
                Container::new()
                    .radius(12.0)
                    .color(scheme.surface_container_high),
            )
            .layer(Positioned::new(chip).left(self.chip.x).top(self.chip.y));

        // `pan_axis` keeps only one direction — and inside a vertical scroll, leaves the
        // vertical drags to the scroll.
        let slide = cx.handler(|s: &mut Self, delta: Point| {
            s.handle = (s.handle + delta.x).clamp(0.0, TRACK - HANDLE);
        });
        let handle = GestureDetector::new(
            Container::new()
                .width(HANDLE)
                .height(HANDLE)
                .radius(HANDLE / 2.0)
                .color(scheme.tertiary),
        )
        .pan_axis(PanAxis::Horizontal)
        .on_pan_update(move |_local, delta| slide(delta));
        let track = Stack::new()
            .width(TRACK)
            .height(HANDLE)
            .fit(StackFit::Expand)
            .layer(
                Container::new()
                    .radius(HANDLE / 2.0)
                    .color(scheme.surface_container_highest),
            )
            .layer(Positioned::new(handle).left(self.handle).top(0.0));

        sections(vec![
            Box::new(section(
                "Last gesture",
                Text::new(self.last.clone()).color(scheme.primary),
            )),
            Box::new(section("Taps and presses", pad)),
            Box::new(section("Drag in any direction", free_area)),
            Box::new(section(
                "Drag along one axis",
                Column::new()
                    .spacing(6.0)
                    .cross_axis_alignment(Align::Start)
                    .child(track)
                    .child(Text::new(format!("{:.0} px", self.handle)).size(13.0)),
            )),
        ])
    }
}
