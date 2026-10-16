use frus::{
    Align, Alignment, AppBar, ClipRRect, Container, FontWeight, Insets, Padding, Row,
    SizedBox, StatelessWidget, Text,
};

use crate::common::{
    colors::{grey, light_gray, primary_color},
    shared::notif_icon,
};

pub struct HomeAppBar;

impl StatelessWidget for HomeAppBar {
    fn build(&self, _cx: &frus::BuildContext) -> Box<dyn frus::Widget> {
        AppBar::new()
            .elevation(2.0)
            .title(Padding::new(
                Insets::new(17.5, 0.0, 17.5, 0.0),
                Row::new()
                    .cross_axis_alignment(Align::Center)
                    .child(
                        Text::new("Forge")
                            .color(primary_color())
                            .size(20.0)
                            .weight(FontWeight::Bold),
                    )
                    .child(
                        Text::new("Dashboard")
                            .size(14.0)
                            .color(grey())
                            .weight(FontWeight::Regular),
                    )
                    .spacing(12.0),
            ))
            .action(notif_icon())
            .action(SizedBox::empty().width(16.0))
            .action(
                ClipRRect::new(100.0).child(
                    Container::new()
                        .padding(11.5)
                        .color(light_gray())
                        .height(44.0)
                        .width(44.0)
                        .alignment(Alignment::CENTER)
                        .child(
                            Text::new("K")
                                .color(grey())
                                .size(12.0)
                                .weight(FontWeight::SemiBold),
                        ),
                ),
            )
            .actions_padding(20.0)
            .build()
    }
}
