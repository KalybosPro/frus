use std::rc::Rc;

use frus::{
    Align, BuildContext, Button, Center, Column, FontWeight, Icon, Icons, Scaffold,
    SingleChildScrollView, StatelessWidget, Text, TextAlign, Widget,
};

use crate::common::{routes::HOME, shared::page_app_bar};
use crate::features::checkout::widgets::order_summary::OrderSummary;
use crate::models::order::Order;

/// The confirmation (`/order-placed`). The order arrives as the navigation's `extra`;
/// opened from an address there is none, and the page still makes sense.
pub(crate) struct OrderPlacedPage {
    order: Option<Rc<Order>>,
}

impl OrderPlacedPage {
    pub(crate) fn new(order: Option<Rc<Order>>) -> Self {
        Self { order }
    }
}

impl StatelessWidget for OrderPlacedPage {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        let router = cx.router();
        let theme = cx.theme();

        let headline = match &self.order {
            Some(order) => format!("Thank you. {} articles are on their way.", order.count()),
            None => "Thank you for your order.".to_string(),
        };
        let mut content = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Center)
            .child(
                Icon::new(Icons::CHECK_CIRCLE)
                    .size(64.0)
                    .color(theme.primary),
            )
            .child(
                Text::new(headline)
                    .size(18.0)
                    .weight(FontWeight::SemiBold)
                    .align(TextAlign::Center),
            );
        if let Some(order) = &self.order {
            content = content.child(
                OrderSummary::new(order.items.clone(), order.shipping_cents, order.total_cents)
                    .into_widget(),
            );
        }
        content = content.child(Button::new("Continue shopping").on_press(move || router.go(HOME)));

        Scaffold::new()
            .background(theme.background)
            .app_bar(page_app_bar(cx, "Order placed").build())
            .body(
                SingleChildScrollView::new()
                    .padding(16.0)
                    .child(Center::new(content)),
            )
            .build()
    }
}
