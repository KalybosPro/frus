//! Search: what was typed, how results are ordered, and how both are written into the
//! address. All plain functions, tested below.

use crate::common::routes::{QUERY, SORT};
use crate::models::product::{Product, CATALOG};

/// How the results are ordered — `?sort=` in the address.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Sort {
    #[default]
    Relevance,
    Price,
    Name,
}

impl Sort {
    pub(crate) const ALL: [Sort; 3] = [Sort::Relevance, Sort::Price, Sort::Name];

    /// Reads `?sort=`. Anything it does not know — a typo, an old link — is the default,
    /// never an error.
    pub(crate) fn from_query(value: Option<&str>) -> Sort {
        match value {
            Some("price") => Sort::Price,
            Some("name") => Sort::Name,
            _ => Sort::Relevance,
        }
    }

    pub(crate) fn as_query(self) -> &'static str {
        match self {
            Sort::Relevance => "relevance",
            Sort::Price => "price",
            Sort::Name => "name",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Sort::Relevance => "Best match",
            Sort::Price => "Price",
            Sort::Name => "Name",
        }
    }

    pub(crate) fn index(self) -> usize {
        Sort::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// What the search page keeps: the text in its field.
///
/// It starts from `?q=` and is written back to it on every keystroke, so the address
/// always says what is on screen — a link to it, or a reload on the web, shows the same
/// search.
pub(crate) struct SearchState {
    pub text: String,
}

impl SearchState {
    pub(crate) fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

/// The query parameters for a search: an empty text and the default order are left out,
/// so a plain search is just `/search`.
pub(crate) fn query_params(text: &str, sort: Sort) -> Vec<(&'static str, String)> {
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
pub(crate) fn results(text: &str, sort: Sort) -> Vec<&'static Product> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sort_reads_back_what_it_writes() {
        for sort in Sort::ALL {
            assert_eq!(Sort::from_query(Some(sort.as_query())), sort);
        }
        assert_eq!(Sort::from_query(Some("cheapest")), Sort::Relevance);
        assert_eq!(Sort::from_query(None), Sort::Relevance);
    }

    #[test]
    fn defaults_are_left_out_of_the_address() {
        assert!(query_params("", Sort::Relevance).is_empty());
        assert!(query_params("   ", Sort::Relevance).is_empty());
        assert_eq!(
            query_params(" shoes ", Sort::Price),
            [(QUERY, "shoes".to_string()), (SORT, "price".to_string())]
        );
    }

    #[test]
    fn a_search_matches_the_category_and_ignores_case() {
        let shoes = results("SHOES", Sort::Relevance);
        assert_eq!(shoes.len(), 3);
        assert!(results("zebra", Sort::Relevance).is_empty());
        assert_eq!(results("", Sort::Relevance).len(), CATALOG.len());
    }

    #[test]
    fn results_follow_the_sort() {
        let prices: Vec<u32> = results("shoes", Sort::Price)
            .iter()
            .map(|p| p.price_cents)
            .collect();
        assert!(prices.windows(2).all(|w| w[0] <= w[1]), "{prices:?}");

        let names: Vec<&str> = results("", Sort::Name).iter().map(|p| p.name).collect();
        assert!(names.windows(2).all(|w| w[0] <= w[1]), "{names:?}");
    }

    #[test]
    fn a_match_in_the_name_comes_first() {
        // "rain" is in "Rain jacket"'s name and in "Trail runner"'s nothing; "light" is
        // in a description only.
        let found = results("ra", Sort::Relevance);
        let first = found.first().map(|p| p.name.to_lowercase());
        assert!(first.is_some_and(|name| name.contains("ra")));
    }
}
