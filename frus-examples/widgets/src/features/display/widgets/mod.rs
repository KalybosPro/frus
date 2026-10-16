//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

mod avatar;
mod badge;
mod breadcrumb;
mod card;
mod chart;
mod chip;
mod datatable;
mod divider;
mod gridtile;
mod icon;
mod image;
mod listtile;
mod table;
mod timeline;
mod tree;

pub(crate) fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "card",
            "Card",
            "A themed surface that groups one piece of content: elevated, filled or outlined.",
            include_str!("card.rs"),
            card::example,
        ),
        Demo::new(
            "chip",
            "Chip",
            "Compact elements for filters, choices, entries and actions.",
            include_str!("chip.rs"),
            chip::example,
        ),
        Demo::new(
            "circle-avatar",
            "CircleAvatar",
            "A circle that stands for a person: a picture, initials or an icon.",
            include_str!("avatar.rs"),
            avatar::example,
        ),
        Demo::new(
            "badge",
            "Badge",
            "A small count or dot that says there is something new.",
            include_str!("badge.rs"),
            badge::example,
        ),
        Demo::new(
            "icon",
            "Icon",
            "Vector icons from the bundled set, or your own, sized and coloured.",
            include_str!("icon.rs"),
            icon::example,
        ),
        Demo::new(
            "image",
            "Image & ImageIcon",
            "Bitmaps from assets, pixels or the network, fitted into a box.",
            include_str!("image.rs"),
            image::example,
        ),
        Demo::new(
            "divider",
            "Divider & VerticalDivider",
            "Thin rules that separate content across a column or down a row.",
            include_str!("divider.rs"),
            divider::example,
        ),
        Demo::new(
            "list-tile",
            "ListTile",
            "One row of a list: leading, title, subtitle and trailing slots.",
            include_str!("listtile.rs"),
            listtile::example,
        ),
        Demo::new(
            "grid-tile",
            "GridTile & GridTileBar",
            "A cell of an image grid with a caption bar laid over it.",
            include_str!("gridtile.rs"),
            gridtile::example,
        ),
        Demo::new(
            "data-table",
            "DataTable",
            "A table that sorts, searches, pages and selects its rows for you.",
            include_str!("datatable.rs"),
            datatable::example,
        ),
        Demo::new(
            "table",
            "Table",
            "A controlled grid of text or widget cells; the app sorts and selects.",
            include_str!("table.rs"),
            table::example,
        ),
        Demo::new(
            "chart",
            "BarChart & LineChart",
            "Themed charts for comparing amounts and reading trends.",
            include_str!("chart.rs"),
            chart::example,
        ),
        Demo::new(
            "timeline",
            "Timeline",
            "A vertical chronology of events joined by a line.",
            include_str!("timeline.rs"),
            timeline::example,
        ),
        Demo::new(
            "tree",
            "Tree",
            "A hierarchy of rows that expand, collapse and select.",
            include_str!("tree.rs"),
            tree::example,
        ),
        Demo::new(
            "breadcrumb",
            "Breadcrumb",
            "The trail from the top of a hierarchy to the current page.",
            include_str!("breadcrumb.rs"),
            breadcrumb::example,
        ),
    ]
}
