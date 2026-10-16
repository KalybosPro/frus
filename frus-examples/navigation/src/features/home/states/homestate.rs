//! What the front page keeps: the category the customer is browsing.

use crate::models::product::{Category, Product, CATALOG};

#[derive(Default)]
pub(crate) struct HomeState {
    /// `None` shows everything.
    pub category: Option<Category>,
}

impl HomeState {
    pub(crate) fn show(&mut self, category: Option<Category>) {
        self.category = category;
    }

    /// The products the page lists, in catalogue order.
    pub(crate) fn visible(&self) -> Vec<&'static Product> {
        CATALOG
            .iter()
            .filter(|product| self.category.is_none_or(|c| product.category == c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_shows_until_a_category_is_picked() {
        let mut state = HomeState::default();
        assert_eq!(state.visible().len(), CATALOG.len());

        state.show(Some(Category::Bags));
        let bags = state.visible();
        assert!(!bags.is_empty());
        assert!(bags.iter().all(|p| p.category == Category::Bags));

        state.show(None);
        assert_eq!(state.visible().len(), CATALOG.len());
    }
}
