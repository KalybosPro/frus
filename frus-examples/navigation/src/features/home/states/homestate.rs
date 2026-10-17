//! What the front page keeps: the category the customer is browsing.

use crate::models::product::{Category, Product, CATALOG};

#[derive(Default)]
pub struct HomeState {
    /// `None` shows everything.
    pub category: Option<Category>,
}

impl HomeState {
    pub fn show(&mut self, category: Option<Category>) {
        self.category = category;
    }

    /// The products the page lists, in catalogue order.
    pub fn visible(&self) -> Vec<&'static Product> {
        CATALOG
            .iter()
            .filter(|product| self.category.is_none_or(|c| product.category == c))
            .collect()
    }
}
