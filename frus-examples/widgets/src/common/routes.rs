//! The application's routes.

/// The list of categories.
pub(crate) const HOME: &str = "/";
/// One category's widgets: `/category/<id>`.
pub(crate) const CATEGORY: &str = "/category/:id";
/// One widget's demo: `/widget/<id>`.
pub(crate) const DEMO: &str = "/widget/:id";

pub(crate) fn category_location(id: &str) -> String {
    format!("/category/{id}")
}

pub(crate) fn demo_location(id: &str) -> String {
    format!("/widget/{id}")
}
