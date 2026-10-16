//! `Image` — a bitmap fitted into a box: embedded with `asset!`, made from pixels, or
//! fetched over the network. `ImageIcon` puts a picture where an icon goes.

use frus::{
    Align, BoxFit, Color, Column, Container, Expanded, Icon, Icons, Image, ImageData, ImageIcon,
    ImageState, Row, Skeleton, Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let theme = cx.theme();

        // Pixels you make yourself become an image with `ImageData::from_rgba`. The
        // renderer caches a picture by its identity, so make it once (`use_memo`) rather
        // than on every build, which would upload a new texture each frame.
        let gradient = cx.use_memo((), || {
            let (w, h) = (64u32, 64u32);
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    rgba.extend_from_slice(&[(x * 4) as u8, (y * 4) as u8, 200, 255]);
                }
            }
            ImageData::from_rgba(w, h, rgba).into_handle()
        });

        // A network image is in one of three states, and the view says what each looks
        // like. This gallery is built without the `net` feature, so there is no fetcher
        // and the request reports why it failed instead of waiting for ever.
        let remote = Image::network("https://example.com/photo.png")
            .size(120.0, 80.0)
            .placeholder(Skeleton::new().radius(8.0));
        let remote: Box<dyn Widget> = match remote.state() {
            ImageState::Ready => Box::new(remote),
            ImageState::Loading => Box::new(Skeleton::new().width(120.0).height(80.0)),
            ImageState::Failed(why) => Box::new(
                Row::new()
                    .spacing(8.0)
                    .child(Icon::new(Icons::BROKEN_IMAGE).color(theme.error))
                    // The reason can be long: `Expanded` keeps it to the room the icon
                    // leaves, so it wraps rather than overflows.
                    .child(Expanded::new(
                        Text::new(format!("Could not load: {why}"))
                            .wrap()
                            .color(theme.muted),
                    )),
            ),
        };

        sections(vec![
            // `asset!` embeds the file at compile time — no path to get wrong at run time.
            // The path is relative to this source file. Give one side and the other
            // follows the picture's own ratio.
            Box::new(section(
                "From an asset",
                Row::new()
                    .spacing(16.0)
                    .child(
                        frus::asset!("../../../../res/mipmap-xxxhdpi/ic_launcher.png").width(96.0),
                    )
                    .child(
                        frus::asset!("../../../../res/mipmap-xxxhdpi/ic_launcher.png")
                            .width(48.0)
                            .opacity(0.5)
                            .semantic_label("frus logo, faded"),
                    ),
            )),
            // `fit` decides how the picture fills a box of another shape.
            Box::new(section(
                "Fit",
                Wrap::new()
                    .gap(16.0)
                    .run_gap(16.0)
                    .child(fitted("Contain", BoxFit::Contain))
                    .child(fitted("Cover", BoxFit::Cover))
                    .child(fitted("Fill", BoxFit::Fill)),
            )),
            Box::new(section(
                "From pixels",
                Image::new((*gradient).clone()).size(96.0, 96.0),
            )),
            // An image as an icon: same size rules as `Icon`, in its own colours unless
            // tinted (tinting suits white or grey artwork).
            Box::new(section(
                "ImageIcon",
                Row::new()
                    .spacing(16.0)
                    .child(ImageIcon::new(frus::asset!(
                        "../../../../res/mipmap-xxxhdpi/ic_launcher.png"
                    )))
                    .child(
                        ImageIcon::new(frus::asset!(
                            "../../../../res/mipmap-xxxhdpi/ic_launcher.png"
                        ))
                        .size(40.0),
                    )
                    .child(Text::new("next to text, at icon size")),
            )),
            // While a network picture is on its way, `placeholder` fills its box with a
            // colour, a shimmering skeleton or a smaller picture.
            Box::new(section("Network, with a placeholder", remote)),
        ])
    }))
}

/// The logo in a wide 140 × 70 box, fitted one way, with the box outlined so the
/// difference shows.
fn fitted(label: &str, fit: BoxFit) -> Column {
    Column::new()
        .cross_axis_alignment(Align::Start)
        .spacing(4.0)
        .child(
            Container::new().border(1.0, Color::hex("#A1A1AA")).child(
                frus::asset!("../../../../res/mipmap-xxxhdpi/ic_launcher.png")
                    .size(140.0, 70.0)
                    .fit(fit),
            ),
        )
        .child(Text::new(label).size(12.0))
}
