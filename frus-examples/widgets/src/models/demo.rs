//! What the gallery is made of: categories, and the demos in each.

use frus::{IconData, Widget};

/// One widget's demo: a live example, and the source file it is written in.
pub(crate) struct Demo {
    /// Where it lives in the address: `/widget/<id>`. Unique across the gallery.
    pub id: &'static str,
    /// The widget's name, as a developer types it.
    pub name: &'static str,
    /// One sentence: what the widget is for.
    pub summary: &'static str,
    /// The demo's own file, shown in the "Code" tab — `include_str!("button.rs")`.
    pub source: &'static str,
    /// Builds the live example.
    pub build: fn() -> Box<dyn Widget>,
}

impl Demo {
    pub(crate) fn new(
        id: &'static str,
        name: &'static str,
        summary: &'static str,
        source: &'static str,
        build: fn() -> Box<dyn Widget>,
    ) -> Self {
        Self {
            id,
            name,
            summary,
            source,
            build,
        }
    }
}

/// A family of widgets: one feature folder under `src/features/`.
pub(crate) struct Category {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub icon: IconData,
    pub demos: fn() -> Vec<Demo>,
}
