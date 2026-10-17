//! Tests for `navigation::features::search::states::searchstate`, moved out of `src/` (milestone 662).

use navigation::common::routes::{QUERY, SORT};
use navigation::features::search::states::searchstate::*;
use navigation::models::product::CATALOG;

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
