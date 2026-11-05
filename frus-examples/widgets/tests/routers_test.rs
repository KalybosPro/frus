//! The gallery's addresses (milestone 751).

use widgets::common::routers::router;
use widgets::common::routes::{category_location, demo_location};

/// A link straight to a category — the address bar, a bookmark — opens it with the list of
/// categories beneath, so its app bar shows the way back and Back lands on that list.
#[test]
fn a_link_to_a_category_has_the_home_page_beneath() {
    let router = router();
    router.go(category_location("inputs"));
    assert_eq!(router.locations(), ["/", "/category/inputs"]);
    assert!(router.can_pop());
    router.pop();
    assert_eq!(router.location(), "/");
}

/// A link straight to a demo has the home page beneath too: the address does not say which
/// category it was reached from.
#[test]
fn a_link_to_a_demo_has_the_home_page_beneath() {
    let router = router();
    router.go(format!("{}?tab=code", demo_location("text_field")));
    assert_eq!(router.locations(), ["/", "/widget/text_field?tab=code"]);
    let state = router.state().unwrap();
    assert_eq!(state.param("id"), Some("text_field"));
    assert_eq!(state.query("tab"), Some("code"));
}
