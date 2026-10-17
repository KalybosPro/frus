//! `GridTile` and `GridTileBar` — a cell of an image grid, with a strip of text laid
//! **over** it rather than under it, so every row of the grid stays the same height.

use frus::{
    Color, Colors, Container, GridTile, GridTileBar, GridView, Icon, IconButton, Icons, Text,
    Widget,
};

use crate::common::shared::{section, sections};

/// Stand-ins for photos: a colour and a caption each.
const PHOTOS: [(&str, &str, &str); 6] = [
    ("#0EA5E9", "Harbour", "Ada"),
    ("#F97316", "Sunset", "Alan"),
    ("#22C55E", "Meadow", "Grace"),
    ("#A855F7", "Dusk", "Linus"),
    ("#EAB308", "Fields", "Edsger"),
    ("#EF4444", "Lanterns", "Barbara"),
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let starred = cx.use_state(|| vec![false; PHOTOS.len()]);

        // A footer bar: a title and a subtitle over the bottom of the picture, and a
        // trailing action that keeps its own tap.
        let footers = PHOTOS.iter().enumerate().fold(
            GridView::new(2).width(320.0).gap(4.0).aspect(1.0),
            |grid, (i, (color, title, author))| {
                let star = starred.clone();
                let on = starred.with(|s| s[i]);
                grid.cell(
                    GridTile::new(photo(color)).footer(
                        GridTileBar::new()
                            .title(Text::new(*title))
                            .subtitle(Text::new(format!("by {author}")))
                            .trailing(
                                IconButton::new(if on { Icons::STAR } else { Icons::STAR_BORDER })
                                    .label("Star")
                                    .on_press(move || star.update(|s| s[i] = !s[i])),
                            ),
                    ),
                )
            },
        );

        sections(vec![
            Box::new(section(
                "Footer with an action",
                Container::new().width(320.0).child(footers),
            )),
            // A header bar sits over the top instead; colours are overridable.
            Box::new(section(
                "Header, custom colours",
                Container::new().width(320.0).child(
                    GridView::new(2)
                        .width(320.0)
                        .gap(4.0)
                        .aspect(1.5)
                        .cell(
                            GridTile::new(photo("#14B8A6")).header(
                                GridTileBar::new()
                                    .leading(Icon::new(Icons::PHOTO_CAMERA).color(Colors::WHITE))
                                    .title(Text::new("Coast")),
                            ),
                        )
                        .cell(
                            GridTile::new(photo("#F43F5E")).header(
                                GridTileBar::new()
                                    .background_color(Color::rgba(1.0, 1.0, 1.0, 0.7))
                                    .foreground_color(Colors::BLACK)
                                    .title(Text::new("Roses")),
                            ),
                        ),
                ),
            )),
        ])
    }))
}

/// A coloured block standing in for a picture. In an app this is an `Image`.
fn photo(hex: &str) -> Container {
    Container::new().color(Color::hex(hex))
}
