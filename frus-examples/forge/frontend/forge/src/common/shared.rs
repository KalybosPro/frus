use frus::{
    Align, Border, BorderRadius, BoxDecoration, Callback, Colors, Container, FontWeight, Row, Text, TextFieldStyle, Widget, asset,
};

use crate::common::colors::{black, gray};

pub(crate) fn on(work: impl Fn() + 'static) -> Callback {
    Callback::new(work)
}

// Custom widget
pub(crate) fn card(child: Box<dyn Widget>) -> Box<dyn Widget> {
    Box::new(
        Container::new()
            .child(child)
            .decoration(
                BoxDecoration::filled(Colors::TRANSPARENT)
                    .border(Border::new(1.0, gray()))
                    .radius(BorderRadius::uniform(12.0)),
            )
            .padding_each(13.0, 16.0, 13.0, 16.0),
    )
}

// App logo widget
pub(crate) fn app_logo() -> Box<dyn Widget> {
    let logo = asset!("../../assets/logo.png").size(32.0, 32.0);
    Box::new(
        Row::new()
            .child(logo)
            .child(
                Text::new("Forge")
                    .weight(FontWeight::SemiBold)
                    .size(18.0)
                    .color(black()),
            )
            .cross_axis_alignment(Align::Center)
            .spacing(8.0),
    )
}

pub(crate) fn input_title(title: String) -> Text {
    Text::new(title).size(14.0).weight(FontWeight::Regular)
}

pub(crate) fn input_style() -> TextFieldStyle {
    TextFieldStyle {
        text_color: Some(black()),
        radius: Some(12.0),
        icon_size: Some(16.0),
        ..Default::default()
    }
}

pub(crate) fn notif_icon() -> Box<dyn Widget> {
    Box::new(asset!("../../assets/icons/notification.png").size(28.0, 28.0))
}

pub(crate) fn assigned_icon() -> Box<dyn Widget> {
    Box::new(asset!("../../assets/icons/assigned.png").size(20.0, 20.0))
}

pub(crate) fn completed_icon() -> Box<dyn Widget> {
    Box::new(asset!("../../assets/icons/completed.png").size(20.0, 20.0))
}

pub(crate) fn active_icon() -> Box<dyn Widget> {
    Box::new(asset!("../../assets/icons/active.png").size(20.0, 20.0))
}

pub(crate) fn soon_icon() -> Box<dyn Widget> {
    Box::new(asset!("../../assets/icons/soon.png").size(20.0, 20.0))
}
