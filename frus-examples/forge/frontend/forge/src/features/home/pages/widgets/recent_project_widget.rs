use frus::{
    BuildContext, Column, FontWeight, GestureDetector, Insets, ListTile, StatelessWidget, Text,
    TextStyle, Widget,
};

use crate::{
    common::colors::{black, grey, primary_color},
    features::home::pages::widgets::project_tile::ProjectTile,
};

pub(crate) struct RecentProjectWidget;

impl StatelessWidget for RecentProjectWidget {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let content = Column::new()
            .spacing(12.0)
            .child(
                ListTile::new()
                    .padding(Insets::ZERO)
                    .title("Recent projects")
                    .title_style(
                        TextStyle::new(18.0)
                            .color(black())
                            .weight(FontWeight::SemiBold),
                    )
                    .trailing(GestureDetector::new(
                        Text::new("View all")
                            .color(primary_color())
                            .size(12.0)
                            .weight(FontWeight::Regular),
                    )),
            )
            .child(
                ProjectTile::new("Frus Framework".to_string(), 12, 4, 72, primary_color())
                    .into_widget(),
            )
            .child(ProjectTile::new("Forge Mobile".to_string(), 8, 3, 38, grey()).into_widget())
            .child(ProjectTile::new("Rust Docs".to_string(), 24, 6, 100, grey()).into_widget());

        Box::new(content)
    }
}
