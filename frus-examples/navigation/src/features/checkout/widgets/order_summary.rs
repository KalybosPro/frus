use frus::{
    Align, BuildContext, Card, Column, Divider, FontWeight, Justify, Row, StatelessWidget, Text,
    Widget,
};

use crate::common::shared::format_price;
use crate::models::cart_item::CartItem;

/// The lines of an order and its totals. Used by the checkout and by the confirmation.
pub(crate) struct OrderSummary {
    items: Vec<CartItem>,
    shipping_cents: u32,
    total_cents: u32,
}

impl OrderSummary {
    pub(crate) fn new(items: Vec<CartItem>, shipping_cents: u32, total_cents: u32) -> Self {
        Self {
            items,
            shipping_cents,
            total_cents,
        }
    }
}

fn line(label: String, amount: String, bold: bool) -> Row {
    let weight = if bold {
        FontWeight::Bold
    } else {
        FontWeight::Regular
    };
    Row::new()
        .main_axis_alignment(Justify::SpaceBetween)
        .child(Text::new(label).weight(weight))
        .child(Text::new(amount).weight(weight))
}

impl StatelessWidget for OrderSummary {
    fn build(&self, _cx: &BuildContext) -> Box<dyn Widget> {
        let shipping = if self.shipping_cents == 0 {
            "Free".to_string()
        } else {
            format_price(self.shipping_cents)
        };
        let lines = self.items.iter().fold(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Stretch),
            |column, item| {
                column.child(line(
                    format!("{} × {} ({})", item.quantity, item.name, item.size),
                    format_price(item.total_cents()),
                    false,
                ))
            },
        );
        Box::new(
            Card::new().outlined().padding(16.0).child(
                lines
                    .child(Divider::new())
                    .child(line("Delivery".to_string(), shipping, false))
                    .child(line(
                        "Total".to_string(),
                        format_price(self.total_cents),
                        true,
                    )),
            ),
        )
    }
}
