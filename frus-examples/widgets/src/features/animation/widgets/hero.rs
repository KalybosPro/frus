//! `Hero` — a shared element between two pages: during the transition it flies from where
//! it was to where it is going, instead of one copy fading out while another fades in.
//!
//! Two heroes with the **same tag**, one on each side of a page transition, are one thing
//! in two places. In an app, the transition is a route push (`cx.router().push(..)`); here
//! a small `Navigator` inside the demo plays the same part, so you can also try the
//! three page transitions a theme can choose between (`PageTransitionsTheme`).

use std::time::Duration;

use frus::{
    component, Align, Alignment, AnimationController, Button, Callback, CircleAvatar, Column,
    Container, Curve, FontWeight, Hero, InkWell, Navigator, PageTransitionsBuilder,
    PageTransitionsTheme, Text, Themed, UseRef, UseState, Variant, Widget, Wrap,
};

use crate::common::shared::{section, sections};

const PEOPLE: [&str; 3] = ["Ada Lovelace", "Grace Hopper", "Alan Turing"];
const TRANSITIONS: [(&str, PageTransitionsBuilder); 3] = [
    ("Zoom", PageTransitionsBuilder::Zoom),
    ("Slide (iOS)", PageTransitionsBuilder::Cupertino),
    ("Fade forwards", PageTransitionsBuilder::FadeForwards),
];
const FRAME: Duration = Duration::from_millis(16);
const PAGE_WIDTH: f32 = 300.0;
const PAGE_HEIGHT: f32 = 280.0;

/// Which page of the little stack is showing: the list, or one person.
type Route = Option<usize>;

/// A page's key: its depth in the stack, and what it shows.
fn key(route: Route) -> (usize, usize) {
    match route {
        None => (0, usize::MAX),
        Some(person) => (1, person),
    }
}

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let route = cx.use_state(|| None::<usize>);
        // The page being left and the direction (push or pop), while a transition runs.
        let leaving = cx.use_state(|| None::<(Route, bool)>);
        let controller = cx.use_ref(AnimationController::unit);
        let progress = cx.use_state(|| 1.0_f32);
        let style = cx.use_state(|| 0_usize);
        let (style_name, builder) = TRANSITIONS[style.get()];

        // Advance the transition once a frame while it runs; when it ends, forget the page
        // that was left.
        if leaving.get().is_some() {
            let (driver, shown, left) = (controller.clone(), progress.clone(), leaving.clone());
            cx.use_interval(FRAME, move || {
                let mut running = false;
                driver.update(|c| running = c.tick(FRAME.as_secs_f32()));
                shown.set(driver.with(AnimationController::value));
                if !running {
                    left.set(None);
                }
            });
        }

        let go = Go {
            route: route.clone(),
            leaving: leaving.clone(),
            controller: controller.clone(),
            progress: progress.clone(),
            duration: builder.transition_duration(),
        };
        let scheme = &cx.theme().scheme;
        let surface = scheme.surface_container_low;

        let mut navigator = Navigator::new(key(route.get()), page(route.get(), &go, surface))
            .size(PAGE_WIDTH, PAGE_HEIGHT);
        if let Some((left, forward)) = leaving.get() {
            navigator =
                navigator.from(key(left), page(left, &go, surface), progress.get(), forward);
        }

        // The page transition is a theme decision, per platform; here it is overridden for
        // this subtree only.
        let stage = Themed::tweak(
            move |theme| {
                theme.page_transitions =
                    PageTransitionsTheme::default().with(theme.platform, builder);
            },
            Container::new()
                .radius(16.0)
                .clip()
                .border(1.0, scheme.outline_variant)
                .child(navigator),
        );

        let picker = TRANSITIONS.iter().enumerate().fold(
            Wrap::new().gap(8.0).run_gap(8.0),
            |row, (index, (name, _))| {
                let pick = style.clone();
                row.child(
                    Button::new(*name)
                        .variant(if index == style.get() {
                            Variant::Filled
                        } else {
                            Variant::Outlined
                        })
                        .on_press(move || pick.set(index)),
                )
            },
        );

        sections(vec![
            Box::new(section("Tap a person", stage)),
            Box::new(section(
                format!("Page transition: {style_name}").as_str(),
                picker,
            )),
        ])
    }))
}

/// What moving between the pages needs: the state it changes, and how long it takes.
#[derive(Clone)]
struct Go {
    route: UseState<Route>,
    leaving: UseState<Option<(Route, bool)>>,
    controller: UseRef<AnimationController>,
    progress: UseState<f32>,
    duration: f32,
}

impl Go {
    /// A handler that moves to `to`: forwards for a push, backwards for a pop.
    fn to(&self, to: Route) -> Callback {
        let go = self.clone();
        Callback::new(move || {
            let forward = to.is_some();
            go.leaving.set(Some((go.route.get(), forward)));
            go.route.set(to);
            go.controller.update(|c| {
                c.set_value(0.0);
                c.forward(go.duration, Curve::Linear);
            });
            go.progress.set(0.0);
        })
    }
}

/// The two kinds of page: the list of people, and one person.
fn page(route: Route, go: &Go, surface: frus::Color) -> Box<dyn Widget> {
    let content: Box<dyn Widget> = match route {
        None => {
            let people = PEOPLE.iter().enumerate().fold(
                Wrap::new().gap(16.0).run_gap(16.0),
                |row, (index, name)| {
                    row.child(
                        InkWell::new()
                            .radius(12.0)
                            .on_click(go.to(Some(index)))
                            .child(
                                Container::new().padding(8.0).child(
                                    Column::new()
                                        .spacing(6.0)
                                        .cross_axis_alignment(Align::Center)
                                        // The small end of the flight.
                                        .child(Hero::new(
                                            index,
                                            CircleAvatar::initials(*name).size(56.0),
                                        ))
                                        .child(
                                            Text::new(name.split(' ').next().unwrap_or(name))
                                                .size(12.0),
                                        ),
                                ),
                            ),
                    )
                },
            );
            Box::new(
                Column::new()
                    .spacing(16.0)
                    .cross_axis_alignment(Align::Start)
                    .child(Text::new("People").size(18.0).weight(FontWeight::SemiBold))
                    .child(people),
            )
        }
        Some(index) => Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                // The large end: the same tag, so the avatar flies here from the list.
                .child(Hero::new(
                    index,
                    CircleAvatar::initials(PEOPLE[index]).size(120.0),
                ))
                .child(
                    Text::new(PEOPLE[index])
                        .size(18.0)
                        .weight(FontWeight::SemiBold),
                )
                .child(
                    Button::new("Back")
                        .variant(Variant::Tonal)
                        .on_press(go.to(None)),
                ),
        ),
    };
    Box::new(
        Container::new()
            .width(PAGE_WIDTH)
            .height(PAGE_HEIGHT)
            .padding(16.0)
            .color(surface)
            .alignment(Alignment::TOP_CENTER)
            .child(content),
    )
}
