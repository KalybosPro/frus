use frus::{BuildContext, Column, Expanded, Row, StatelessWidget, Widget};

use crate::{
    common::{
        colors::{light_gray, primary_color},
        shared::{active_icon, assigned_icon, completed_icon, soon_icon},
    },
    features::home::pages::widgets::stats_card::StatsCard,
};

pub(crate) struct StatsGrid;

impl StatelessWidget for StatsGrid {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let content = Column::new()
            .spacing(12.0)
            .child(
                Row::new()
                    .spacing(12.0)
                    .child(Expanded::new(
                        StatsCard::new(
                            move || assigned_icon(),
                            12,
                            "Assigned tasks",
                            primary_color().with_alpha(0.1),
                        )
                        .into_widget(),
                    ))
                    .child(Expanded::new(
                        StatsCard::new(move || completed_icon(), 28, "Completed", light_gray())
                            .into_widget(),
                    )),
            )
            .child(
                Row::new()
                    .spacing(12.0)
                    .child(Expanded::new(
                        StatsCard::new(move || soon_icon(), 5, "Due soon", light_gray())
                            .into_widget(),
                    ))
                    .child(Expanded::new(
                        StatsCard::new(move || active_icon(), 3, "Active projects", light_gray())
                            .into_widget(),
                    )),
            );

        Box::new(content)
    }
}
