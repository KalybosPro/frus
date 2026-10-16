use frus::{
    Align, BuildContext, Callback, Card, Column, Expanded, FontWeight, IconButton, Icons, Row,
    StatelessWidget, Text, Widget,
};

use crate::common::{colors::price, shared::format_price};
use crate::features::product::widgets::quantity_stepper::QuantityStepper;
use crate::models::cart_item::CartItem;

/// One line of the cart, with its quantity controls. What the buttons do is the page's.
pub(crate) struct CartLine {
    item: CartItem,
    on_less: Callback,
    on_more: Callback,
    on_remove: Callback,
}

impl CartLine {
    pub(crate) fn new(
        item: CartItem,
        on_less: Callback,
        on_more: Callback,
        on_remove: Callback,
    ) -> Self {
        Self {
            item,
            on_less,
            on_more,
            on_remove,
        }
    }
}

impl StatelessWidget for CartLine {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let item = &self.item;
        let about = Column::new()
            .spacing(2.0)
            .cross_axis_alignment(Align::Start)
            .child(Text::new(item.name.clone()).weight(FontWeight::SemiBold))
            .child(
                Text::new(format!(
                    "Size {} · {} each",
                    item.size,
                    format_price(item.unit_price_cents)
                ))
                .size(13.0)
                .color(cx.theme().muted),
            )
            .child(Text::new(format_price(item.total_cents())).color(price()));

        Box::new(
            Card::new().outlined().padding(12.0).child(
                Row::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Center)
                    .child(Expanded::new(about))
                    .child(
                        QuantityStepper::new(
                            item.quantity,
                            self.on_less.clone(),
                            self.on_more.clone(),
                        )
                        .into_widget(),
                    )
                    .child(
                        IconButton::new(Icons::DELETE_OUTLINE)
                            .label("Remove")
                            .on_press(self.on_remove.clone()),
                    ),
            ),
        )
    }
}
