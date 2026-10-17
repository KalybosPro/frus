//! Search: what was typed, how results are ordered, and how both are written into the
//! address. All plain functions, tested in `tests/searchstate_test.rs`.

use crate::common::routes::{QUERY, SORT};
use crate::models::product::{Product, CATALOG};

/// How the results are ordered — `?sort=` in the address.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Relevance,
    Price,
    Name,
}

impl Sort {
    pub const ALL: [Sort; 3] = [Sort::Relevance, Sort::Price, Sort::Name];

    /// Reads `?sort=`. Anything it does not know — a typo, an old link — is the default,
    /// never an error.
    pub fn from_query(value: Option<&str>) -> Sort {
        match value {
            Some("price") => Sort::Price,
            Some("name") => Sort::Name,
            _ => Sort::Relevance,
        }
    }

    pub fn as_query(self) -> &'static str {
        match self {
            Sort::Relevance => "relevance",
            Sort::Price => "price",
            Sort::Name => "name",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Relevance => "Best match",
            Sort::Price => "Price",
            Sort::Name => "Name",
        }
    }

    pub fn index(self) -> usize {
        Sort::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// What the search page keeps: the text in its field.
///
/// It starts from `?q=` and is written back to it on every keystroke, so the address
/// always says what is on screen — a link to it, or a reload on the web, shows the same
/// search.
pub struct SearchState {
    pub text: String,
}

impl SearchState {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

/// The query parameters for a search: an empty text and the default order are left out,
/// so a plain search is just `/search`.
pub fn query_params(text: &str, sort: Sort) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();
    let text = text.trim();
    if !text.is_empty() {
        params.push((QUERY, text.to_string()));
    }
    if sort != Sort::default() {
        params.push((SORT, sort.as_query().to_string()));
    }
    params
}

/// The products matching `text` (name, category or description, ignoring case), in
/// `sort` order. An empty text matches everything.
pub fn results(text: &str, sort: Sort) -> Vec<&'static Product> {
    let needle = text.trim().to_lowercase();
    let mut found: Vec<&'static Product> = CATALOG
        .iter()
        .filter(|product| {
            needle.is_empty()
                || product.name.to_lowercase().contains(&needle)
                || product.category.label().to_lowercase().contains(&needle)
                || product.summary.to_lowercase().contains(&needle)
        })
        .collect();
    match sort {
        // A match in the name ranks above one in the description.
        Sort::Relevance => {
            found.sort_by_key(|product| !product.name.to_lowercase().contains(&needle))
        }
        Sort::Price => found.sort_by_key(|product| product.price_cents),
        Sort::Name => found.sort_by_key(|product| product.name),
    }
    found
}
