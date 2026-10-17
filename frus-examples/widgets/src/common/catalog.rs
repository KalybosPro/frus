//! The gallery's table of contents: every category, in the order the home page lists
//! them. Each category is a feature folder whose `widgets/mod.rs` lists its demos.

use frus::Icons;

use crate::features;
use crate::models::demo::{Category, Demo};

pub fn categories() -> Vec<Category> {
    vec![
        Category {
            id: "layout",
            name: "Layout",
            summary: "Boxes, rows, columns, stacks and constraints",
            icon: Icons::DASHBOARD,
            demos: features::layout::widgets::demos,
        },
        Category {
            id: "text",
            name: "Text",
            summary: "Text, styles, rich and selectable text",
            icon: Icons::TEXT_FIELDS,
            demos: features::text::widgets::demos,
        },
        Category {
            id: "buttons",
            name: "Buttons & menus",
            summary: "Buttons, icon buttons, toggles and menus",
            icon: Icons::SMART_BUTTON,
            demos: features::buttons::widgets::demos,
        },
        Category {
            id: "inputs",
            name: "Inputs & forms",
            summary: "Text fields, choices, sliders, pickers and validation",
            icon: Icons::INPUT,
            demos: features::inputs::widgets::demos,
        },
        Category {
            id: "display",
            name: "Display",
            summary: "Cards, chips, avatars, icons, images, tables and charts",
            icon: Icons::VIEW_QUILT,
            demos: features::display::widgets::demos,
        },
        Category {
            id: "feedback",
            name: "Feedback",
            summary: "Dialogs, snack bars, sheets, banners and progress",
            icon: Icons::FEEDBACK,
            demos: features::feedback::widgets::demos,
        },
        Category {
            id: "navigation",
            name: "Navigation",
            summary: "App bars, tabs, navigation bars, rails and drawers",
            icon: Icons::NAVIGATION,
            demos: features::navigation::widgets::demos,
        },
        Category {
            id: "scrolling",
            name: "Lists & scrolling",
            summary: "Lists, grids, pages, reordering and expansion",
            icon: Icons::VIEW_LIST,
            demos: features::scrolling::widgets::demos,
        },
        Category {
            id: "animation",
            name: "Animation",
            summary: "Implicit animations, transitions and heroes",
            icon: Icons::ANIMATION,
            demos: features::animation::widgets::demos,
        },
        Category {
            id: "painting",
            name: "Painting & effects",
            summary: "Decoration, clipping, transforms, custom painting and filters",
            icon: Icons::BRUSH,
            demos: features::painting::widgets::demos,
        },
        Category {
            id: "interaction",
            name: "Interaction",
            summary: "Gestures, hover, drag and drop, focus and shortcuts",
            icon: Icons::TOUCH_APP,
            demos: features::interaction::widgets::demos,
        },
        Category {
            id: "state",
            name: "State & async",
            summary: "Hooks, notifiers, futures, streams, theme and screen size",
            icon: Icons::SYNC,
            demos: features::state::widgets::demos,
        },
    ]
}

pub fn category(id: &str) -> Option<Category> {
    categories().into_iter().find(|category| category.id == id)
}

/// A demo by its id, with the category it belongs to.
pub fn demo(id: &str) -> Option<(Category, Demo)> {
    categories().into_iter().find_map(|category| {
        let demo = (category.demos)().into_iter().find(|demo| demo.id == id)?;
        Some((category, demo))
    })
}
