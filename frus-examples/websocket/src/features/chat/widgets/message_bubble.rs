use frus::{
    Align, BorderRadius, BuildContext, Column, Container, Flexible, Justify, Row, StatelessWidget,
    Text, TextAlign, Widget,
};

use crate::models::message::{Kind, Message};

/// The widest a bubble grows, so that a long line reads as a paragraph on a wide window.
const MAX_BUBBLE_WIDTH: f32 = 420.0;

/// One line of the conversation: a bubble on the right for what was sent, on the left for
/// what was received, and a quiet line in the middle for what the app itself says.
pub struct MessageBubble {
    message: Message,
}

impl MessageBubble {
    pub fn new(message: Message) -> Self {
        Self { message }
    }
}

impl StatelessWidget for MessageBubble {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let theme = cx.theme();
        let message = &self.message;

        if message.kind == Kind::Notice {
            return Box::new(
                Text::new(format!("{}  ·  {}", message.text, message.at))
                    .size(12.0)
                    .color(theme.muted)
                    .align(TextAlign::Center),
            );
        }

        // Colours from the theme's scheme: the accent's container for our side, a neutral
        // surface for theirs — both follow light and dark mode by themselves.
        let mine = message.is_mine();
        let scheme = &theme.scheme;
        let (fill, ink) = if mine {
            (scheme.primary_container, scheme.on_primary_container)
        } else {
            (scheme.surface_container_highest, scheme.on_surface)
        };
        // The corner nearest the speaker stays sharp: the bubble points at its side.
        let radius = BorderRadius {
            bottom_right: if mine { 4.0 } else { 16.0 },
            bottom_left: if mine { 16.0 } else { 4.0 },
            ..BorderRadius::uniform(16.0)
        };

        let bubble = Container::new()
            .max_width(MAX_BUBBLE_WIDTH)
            .padding_each(8.0, 12.0, 8.0, 12.0)
            .color(fill)
            .radius(radius)
            .child(
                Column::new()
                    .spacing(2.0)
                    .cross_axis_alignment(if mine { Align::End } else { Align::Start })
                    .child(Text::new(message.text.as_str()).size(15.0).color(ink))
                    .child(
                        Text::new(message.at.to_string())
                            .size(11.0)
                            .color(ink.with_alpha(0.7)),
                    ),
            );

        // `Flexible` lets a short bubble keep its own width and a long one wrap at the
        // row's width instead of running off the side.
        Box::new(
            Row::new()
                .main_axis_alignment(if mine { Justify::End } else { Justify::Start })
                .child(Flexible::new(bubble)),
        )
    }
}
