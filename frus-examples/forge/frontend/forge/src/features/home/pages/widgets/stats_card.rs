use frus::{
    Align, BorderRadius, BoxDecoration, Color, Column, Container, FontWeight, SizedBox,
    StatelessWidget, Text, Widget,
};

use crate::common::colors::{black, grey};

#[allow(dead_code)]
pub struct StatsCard {
    icon: Box<dyn Fn() -> Box<dyn Widget>>,
    count: i64,
    title: String,
    color: Color,
}

impl StatsCard {
    pub fn new<W: Widget + 'static>(
        icon: impl Fn() -> W + 'static,
        count: i64,
        title: impl Into<String>,
        color: Color,
    ) -> Self {
        Self {
            icon: Box::new(move || Box::new(icon())),
            count,
            title: title.into(),
            color,
        }
    }
}

impl StatelessWidget for StatsCard {
    fn build(&self, _cx: &frus::BuildContext) -> Box<dyn Widget> {
        let content = Container::new()
            .height(120.0)
            .width(175.0)
            .decoration(BoxDecoration::filled(self.color).radius(BorderRadius::uniform(12.0)))
            .padding(16.0)
            .child(
                Column::new()
                    .cross_axis_alignment(Align::Center)
                    .cross_axis_alignment(Align::Start)
                    .child((self.icon)())
                    .child(SizedBox::empty().height(16.0))
                    .child(
                        Text::new(self.count.to_string())
                            .color(black())
                            .size(24.0)
                            .weight(FontWeight::Bold),
                    )
                    .child(SizedBox::empty().height(4.0))
                    .child(
                        Text::new(self.title.to_string())
                            .size(12.0)
                            .color(grey())
                            .weight(FontWeight::Regular),
                    ),
            );

        Box::new(content)
    }
}
