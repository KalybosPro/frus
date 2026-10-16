//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

mod action_buttons;
mod button;
mod dropdown_button;
mod dropdown_menu;
mod fab;
mod icon_button;
mod menu_anchor;
mod menu_bar;
mod popup_menu_button;
mod segmented_button;
mod toggle_buttons;

pub(crate) fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "button",
            "Button",
            "The labelled button: filled, tonal, elevated, outlined or text, with an optional icon.",
            include_str!("button.rs"),
            button::example,
        ),
        Demo::new(
            "icon-button",
            "IconButton",
            "A square button holding one icon, with four looks and a selected state.",
            include_str!("icon_button.rs"),
            icon_button::example,
        ),
        Demo::new(
            "floating-action-button",
            "FloatingActionButton",
            "The screen's main action, floating above it: small, regular, large or extended.",
            include_str!("fab.rs"),
            fab::example,
        ),
        Demo::new(
            "segmented-button",
            "SegmentedButton",
            "One outlined control with a few segments, exactly one of them chosen.",
            include_str!("segmented_button.rs"),
            segmented_button::example,
        ),
        Demo::new(
            "toggle-buttons",
            "ToggleButtons",
            "A bank of buttons sharing their edges, each switched on or off on its own.",
            include_str!("toggle_buttons.rs"),
            toggle_buttons::example,
        ),
        Demo::new(
            "popup-menu-button",
            "PopupMenuButton & MenuItem",
            "A list of actions floating over a button: icons, ticks, shortcuts and rules.",
            include_str!("popup_menu_button.rs"),
            popup_menu_button::example,
        ),
        Demo::new(
            "menu-anchor",
            "MenuAnchor",
            "A floating panel with any content, anchored to a widget.",
            include_str!("menu_anchor.rs"),
            menu_anchor::example,
        ),
        Demo::new(
            "menu-bar",
            "MenuBar & SubmenuButton",
            "A desktop window's menus along the top, with submenus inside them.",
            include_str!("menu_bar.rs"),
            menu_bar::example,
        ),
        Demo::new(
            "dropdown-button",
            "DropdownButton",
            "A button showing the current choice, whose options float below it.",
            include_str!("dropdown_button.rs"),
            dropdown_button::example,
        ),
        Demo::new(
            "dropdown-menu",
            "DropdownMenu",
            "A dropdown that looks like a text field and filters its choices as you type.",
            include_str!("dropdown_menu.rs"),
            dropdown_menu::example,
        ),
        Demo::new(
            "action-buttons",
            "BackButton, CloseButton & DrawerButton",
            "Icon buttons whose meaning is fixed: back, close, and the two drawers.",
            include_str!("action_buttons.rs"),
            action_buttons::example,
        ),
    ]
}
