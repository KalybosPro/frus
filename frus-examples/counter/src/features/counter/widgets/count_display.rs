use frus::{Align, BuildContext, Column, FontWeight, StatelessWidget, Text, Widget};

use crate::common::colors::{negative, positive, text_muted};

/// The big number in the middle of the page.
///
/// A `StatelessWidget` configured by its parent: it is made again, with the new count,
/// every time the page rebuilds. Pulling a piece out like this keeps `build` short and
/// the piece reusable.
pub(crate) struct CountDisplay {
    count: i64,
}

impl CountDisplay {
    pub(crate) fn new(count: i64) -> Self {
        Self { count }
    }
}

impl StatelessWidget for CountDisplay {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        // Colour carries meaning: green above zero, red below, the theme's text colour at
        // zero. `cx.theme()` is the theme the application runs with.
        let color = match self.count {
            n if n > 0 => positive(),
            n if n < 0 => negative(),
            _ => cx.theme().on_surface,
        };

        Box::new(
            Column::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Center)
                .child(
                    Text::new(self.count.to_string())
                        .size(72.0)
                        .weight(FontWeight::Bold)
                        .color(color),
                )
                .child(Text::new("the count").size(14.0).color(text_muted())),
        )
    }
}
