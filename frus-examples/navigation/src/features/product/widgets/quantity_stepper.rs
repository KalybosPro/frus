use frus::{Align, BuildContext, Callback, IconButton, Icons, Row, StatelessWidget, Text, Widget};

/// `−  3  +`. The parent owns the number and says what each button does.
pub(crate) struct QuantityStepper {
    quantity: u32,
    on_less: Callback,
    on_more: Callback,
}

impl QuantityStepper {
    pub(crate) fn new(quantity: u32, on_less: Callback, on_more: Callback) -> Self {
        Self {
            quantity,
            on_less,
            on_more,
        }
    }
}

impl StatelessWidget for QuantityStepper {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        Box::new(
            Row::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Center)
                .child(
                    IconButton::new(Icons::REMOVE)
                        .label("One fewer")
                        .on_press(self.on_less.clone()),
                )
                .child(Text::new(self.quantity.to_string()).size(16.0))
                .child(
                    IconButton::new(Icons::ADD)
                        .label("One more")
                        .on_press(self.on_more.clone()),
                ),
        )
    }
}
