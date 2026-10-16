//! `Draggable` and `DragTarget` — pick a thing up and drop it somewhere else.
//!
//! What travels between the two is a **payload**: a `u64` you choose (an id, an index) and
//! get back in the target's `on_drop`. The two ends need not know of each other.
//! Inside a scrollable list, lift with `.long_press()` so the list can still scroll.

use frus::{
    Align, Button, Column, Container, DragTarget, Draggable, State, StateContext, StatefulWidget,
    Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(Sorting.into_widget())
}

const FRUITS: [&str; 5] = ["Apple", "Banana", "Cherry", "Grape", "Lemon"];

/// Where each fruit is: unsorted, or in one of the baskets.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Table,
    Basket(usize),
}

const BASKETS: [&str; 2] = ["Red", "Yellow"];

struct Sorting;

struct SortingState {
    places: [Place; FRUITS.len()],
    message: String,
}

impl StatefulWidget for Sorting {
    type State = SortingState;

    fn create_state(&self) -> SortingState {
        SortingState {
            places: [Place::Table; FRUITS.len()],
            message: "Drag a fruit into a basket".to_string(),
        }
    }
}

impl SortingState {
    fn drop_into(&mut self, fruit: u64, basket: usize) {
        if let Some(place) = self.places.get_mut(fruit as usize) {
            *place = Place::Basket(basket);
            self.message = format!("{} → {} basket", FRUITS[fruit as usize], BASKETS[basket]);
        }
    }
}

impl State for SortingState {
    type Widget = Sorting;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let scheme = &cx.theme().scheme;

        // A fruit chip, draggable, carrying its index as the payload.
        let chip = |index: usize| {
            Draggable::new(
                Container::new()
                    .padding(10.0)
                    .radius(18.0)
                    .color(scheme.secondary_container)
                    .child(Text::new(FRUITS[index]).color(scheme.on_secondary_container)),
            )
            .payload(index as u64)
            // Told whether a target took it: the moment to explain a refusal.
            .on_dropped(cx.handler(|s: &mut Self, accepted: bool| {
                if !accepted {
                    s.message = "Dropped outside a basket: nothing moved".to_string();
                }
            }))
        };
        let in_place = |place: Place| {
            (0..FRUITS.len())
                .filter(|&i| self.places[i] == place)
                .fold(Wrap::new().gap(8.0).run_gap(8.0), |row, i| {
                    row.child(chip(i))
                })
        };

        // A basket: a target that accepts some payloads and says what to do with them.
        let basket = |index: usize| {
            DragTarget::new(
                Container::new()
                    .width(140.0)
                    .height(140.0)
                    .padding(10.0)
                    .radius(12.0)
                    .border(2.0, scheme.outline_variant)
                    .child(
                        Column::new()
                            .spacing(8.0)
                            .cross_axis_alignment(Align::Start)
                            .child(Text::new(format!("{} basket", BASKETS[index])).size(13.0))
                            .child(in_place(Place::Basket(index))),
                    ),
            )
            // The yellow basket refuses cherries and grapes: the highlight only shows
            // over a target that would accept what is being dragged.
            .accepts(move |fruit| index == 0 || !matches!(fruit, 2 | 3))
            .on_drop(cx.handler(move |s: &mut Self, fruit: u64| s.drop_into(fruit, index)))
        };

        let reset = cx.callback(|s| {
            s.places = [Place::Table; FRUITS.len()];
            s.message = "Back on the table".to_string();
        });

        sections(vec![
            Box::new(section(
                "Status",
                Text::new(self.message.clone()).color(scheme.primary),
            )),
            Box::new(section("On the table", in_place(Place::Table))),
            Box::new(section(
                "Baskets",
                Wrap::new()
                    .gap(16.0)
                    .run_gap(16.0)
                    .child(basket(0))
                    .child(basket(1)),
            )),
            Box::new(section("Start again", Button::new("Reset").on_press(reset))),
        ])
    }
}
