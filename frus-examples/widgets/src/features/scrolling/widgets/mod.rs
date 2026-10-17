//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

pub mod carousel;
pub mod dismissible;
pub mod expansionpanel;
pub mod expansiontile;
pub mod gridview;
pub mod kanban;
pub mod listview;
pub mod listwheel;
pub mod pageview;
pub mod refresh;
pub mod reorderable;
pub mod scrollbehavior;
pub mod scrolloverlay;
pub mod scrollto;
pub mod scrollview;
pub mod sticky;

pub fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "single-child-scroll-view",
            "SingleChildScrollView",
            "Makes one child scrollable: down, across or both ways.",
            include_str!("scrollview.rs"),
            scrollview::example,
        ),
        Demo::new(
            "list-view",
            "ListView",
            "A virtualised list: only the visible rows are built.",
            include_str!("listview.rs"),
            listview::example,
        ),
        Demo::new(
            "grid-view",
            "GridView",
            "Cells in equal columns: a fixed count, as many as fit, or virtualised.",
            include_str!("gridview.rs"),
            gridview::example,
        ),
        Demo::new(
            "page-view",
            "PageView",
            "A scrollable that always comes to rest on a page.",
            include_str!("pageview.rs"),
            pageview::example,
        ),
        Demo::new(
            "carousel-view",
            "CarouselView",
            "One slide at a time between arrows, controlled by the app.",
            include_str!("carousel.rs"),
            carousel::example,
        ),
        Demo::new(
            "reorderable-list",
            "ReorderableList",
            "A list whose rows can be dragged into a new order.",
            include_str!("reorderable.rs"),
            reorderable::example,
        ),
        Demo::new(
            "dismissible",
            "Dismissible",
            "Swipe a row aside to archive or delete it.",
            include_str!("dismissible.rs"),
            dismissible::example,
        ),
        Demo::new(
            "expansion-tile",
            "ExpansionTile",
            "A list row that opens to show more; several make an accordion.",
            include_str!("expansiontile.rs"),
            expansiontile::example,
        ),
        Demo::new(
            "expansion-panel-list",
            "ExpansionPanelList",
            "Panels drawn as one surface that splits apart as one opens.",
            include_str!("expansionpanel.rs"),
            expansionpanel::example,
        ),
        Demo::new(
            "sticky-header",
            "StickyHeader",
            "Section headers that stay at the top while their section is in view.",
            include_str!("sticky.rs"),
            sticky::example,
        ),
        Demo::new(
            "list-wheel",
            "ListWheel",
            "A spinning cylinder of values to pick one from.",
            include_str!("listwheel.rs"),
            listwheel::example,
        ),
        Demo::new(
            "kanban",
            "Kanban",
            "A board of columns and cards, with drag and drop between columns.",
            include_str!("kanban.rs"),
            kanban::example,
        ),
        Demo::new(
            "refresh-indicator",
            "RefreshIndicator",
            "Pull a list down past its top to refresh it.",
            include_str!("refresh.rs"),
            refresh::example,
        ),
        Demo::new(
            "scroll-to",
            "Programmatic scrolling",
            "Read where a list is with on_scroll, and move it with scroll_to.",
            include_str!("scrollto.rs"),
            scrollto::example,
        ),
        Demo::new(
            "scroll-overlay",
            "ScrollOverlay",
            "Something drawn over a scroll region, rebuilt from its offset.",
            include_str!("scrolloverlay.rs"),
            scrolloverlay::example,
        ),
        Demo::new(
            "scroll-behavior",
            "Scroll physics & scrollbars",
            "Edge physics, scrollbars and overscroll, per scrollable or per subtree.",
            include_str!("scrollbehavior.rs"),
            scrollbehavior::example,
        ),
    ]
}
