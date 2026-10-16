use std::ops::Sub;

use frus::{
    Align, Border, BorderRadius, BoxDecoration, BuildContext, ClipRRect, Color, Colors, Column,
    Container, Expanded, FontWeight, LinearProgressIndicator, MediaQuery, Positioned, Row,
    SizedBox, Stack, StatelessWidget, Text, Widget,
};

use crate::{
    common::colors::{black, gray, grey, light_gray, primary_color},
    features::home::pages::widgets::member_avatar::MemberAvatar,
};

pub(crate) struct ProjectTile {
    title: String,
    tasks: usize,
    members: usize,
    percentage: usize,
    per_color: Color,
}

impl ProjectTile {
    pub fn new(
        title: String,
        tasks: usize,
        members: usize,
        percentage: usize,
        per_color: Color,
    ) -> Self {
        Self {
            title: title,
            tasks: tasks,
            members: members,
            percentage: percentage,
            per_color: per_color,
        }
    }
}

impl StatelessWidget for ProjectTile {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let width = MediaQuery::of().size.width;

        let _title = Column::new()
            .child(
                Row::new()
                    .child(Expanded::new(
                        Text::new(self.title.to_string())
                            .color(black())
                            .size(16.0)
                            .weight(FontWeight::SemiBold),
                    ))
                    .child(
                        Text::new(format!("{}%", self.percentage))
                            .color(self.per_color)
                            .size(14.0)
                            .weight(FontWeight::SemiBold),
                    ),
            )
            .child(
                Row::new()
                    .child(
                        Text::new(format!("{} tasks", self.tasks))
                            .size(12.0)
                            .color(grey()),
                    )
                    .child(Text::new("·").size(12.0).color(grey()))
                    .child(
                        Text::new(format!("{} members", self.members))
                            .size(12.0)
                            .color(grey()),
                    ),
            );

        let progress_bar = LinearProgressIndicator::new(self.percentage as f32 * 0.01)
            .color(primary_color())
            .radius(6.0)
            .track_color(light_gray())
            .min_height(8.0);

        let members = SizedBox::new(
            Stack::new()
                .layer(Positioned::new(
                    MemberAvatar::new("K".to_string(), primary_color()).into_widget(),
                ))
                .layer(
                    Positioned::new(
                        MemberAvatar::new("D".to_string(), primary_color()).into_widget(),
                    )
                    .left(15.0),
                ),
        )
        .height(35.0)
        .width(width.sub(100.0));

        let content = ClipRRect::new(12.0).child(
            Row::new()
                .child(Container::new().width(4.0).height(135.0).color(gray()))
                .child(Expanded::new(
                    Container::new()
                        .height(146.0)
                        .decoration(
                            BoxDecoration::filled(Colors::WHITE)
                                .radius(BorderRadius::uniform(12.0))
                                .border(Border::new(1.0, gray())),
                        )
                        .padding(16.0)
                        .child(
                            Column::new()
                                .spacing(16.0)
                                .cross_axis_alignment(Align::Start)
                                .child(_title)
                                .child(progress_bar)
                                .child(members),
                        ),
                )),
        );

        Box::new(content)
    }
}
