//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

mod appbar;
mod bottomappbar;
mod bottombar;
mod collapsingheader;
mod drawer;
mod navbar;
mod navdrawer;
mod navrail;
mod navscaffold;
mod pageselector;
mod pagination;
mod scaffold;
mod steps;
mod tabs;

pub(crate) fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "app-bar",
            "AppBar",
            "The bar at the top of a screen: leading button, title, actions and a bottom.",
            include_str!("appbar.rs"),
            appbar::example,
        ),
        Demo::new(
            "scaffold",
            "Scaffold",
            "The shell of a screen: app bar, body, navigation, drawer, FAB and footer slots.",
            include_str!("scaffold.rs"),
            scaffold::example,
        ),
        Demo::new(
            "tab-bar",
            "TabBar",
            "Tabs and the panel of the selected one: primary, secondary, icons, scrolling, swiping.",
            include_str!("tabs.rs"),
            tabs::example,
        ),
        Demo::new(
            "bottom-bar",
            "BottomBar",
            "The main destinations along the bottom of a phone screen.",
            include_str!("bottombar.rs"),
            bottombar::example,
        ),
        Demo::new(
            "navigation-rail",
            "NavigationRail",
            "The main destinations down the side of a tablet or desktop window.",
            include_str!("navrail.rs"),
            navrail::example,
        ),
        Demo::new(
            "nav-scaffold",
            "NavScaffold",
            "Navigation that becomes a bar, a rail or an extended rail with the size class.",
            include_str!("navscaffold.rs"),
            navscaffold::example,
        ),
        Demo::new(
            "drawer",
            "Drawer & drawer headers",
            "A side panel that slides over the page or sits docked beside it.",
            include_str!("drawer.rs"),
            drawer::example,
        ),
        Demo::new(
            "navigation-drawer",
            "NavigationDrawer",
            "A side panel's list of destinations, with a header, badges and a footer.",
            include_str!("navdrawer.rs"),
            navdrawer::example,
        ),
        Demo::new(
            "bottom-app-bar",
            "BottomAppBar",
            "A bar of actions along the bottom, notched for a docked action button.",
            include_str!("bottomappbar.rs"),
            bottomappbar::example,
        ),
        Demo::new(
            "navigation-bar",
            "NavigationBar",
            "A simple screen header: a centred title and a back button.",
            include_str!("navbar.rs"),
            navbar::example,
        ),
        Demo::new(
            "collapsing-header",
            "CollapsingHeader",
            "A tall header that shrinks to a toolbar as the page under it scrolls.",
            include_str!("collapsingheader.rs"),
            collapsingheader::example,
        ),
        Demo::new(
            "steps",
            "Steps",
            "The numbered step indicator of a wizard or multi-step form.",
            include_str!("steps.rs"),
            steps::example,
        ),
        Demo::new(
            "pagination",
            "Pagination",
            "A page picker: previous, page numbers, next.",
            include_str!("pagination.rs"),
            pagination::example,
        ),
        Demo::new(
            "tab-page-selector",
            "TabPageSelector",
            "Dots that say which of several pages you are on.",
            include_str!("pageselector.rs"),
            pageselector::example,
        ),
    ]
}
