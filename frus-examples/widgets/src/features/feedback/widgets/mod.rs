//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

mod about_dialog;
mod alert;
mod alert_dialog;
mod bottom_sheet;
mod dialog;
mod draggable_sheet;
mod material_banner;
mod progress_indicator;
mod simple_dialog;
mod skeleton;
mod snack_bar;
mod tooltip;

pub(crate) fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "alert-dialog",
            "AlertDialog",
            "A modal box that asks a question and waits for the answer.",
            include_str!("alert_dialog.rs"),
            alert_dialog::example,
        ),
        Demo::new(
            "dialog",
            "Dialog",
            "The modal surface on its own, with any content in it.",
            include_str!("dialog.rs"),
            dialog::example,
        ),
        Demo::new(
            "simple-dialog",
            "SimpleDialog",
            "A modal list of choices, answered by picking one.",
            include_str!("simple_dialog.rs"),
            simple_dialog::example,
        ),
        Demo::new(
            "about-dialog",
            "AboutDialog, AboutListTile & LicensePage",
            "The \"about this app\" box, and the page listing the licences it ships.",
            include_str!("about_dialog.rs"),
            about_dialog::example,
        ),
        Demo::new(
            "snack-bar",
            "SnackBar & ScaffoldMessenger",
            "A short message at the bottom of the screen that goes away by itself.",
            include_str!("snack_bar.rs"),
            snack_bar::example,
        ),
        Demo::new(
            "bottom-sheet",
            "BottomSheet",
            "A modal panel sliding up from the bottom, for contextual actions.",
            include_str!("bottom_sheet.rs"),
            bottom_sheet::example,
        ),
        Demo::new(
            "draggable-scrollable-sheet",
            "DraggableScrollableSheet",
            "A panel along the bottom that follows the finger between resting heights.",
            include_str!("draggable_sheet.rs"),
            draggable_sheet::example,
        ),
        Demo::new(
            "material-banner",
            "MaterialBanner",
            "A message across the top of the content that stays until it is answered.",
            include_str!("material_banner.rs"),
            material_banner::example,
        ),
        Demo::new(
            "alert",
            "Alert",
            "A message box in the flow of the page: info, success, warning or error.",
            include_str!("alert.rs"),
            alert::example,
        ),
        Demo::new(
            "progress-indicator",
            "LinearProgressIndicator & CircularProgressIndicator",
            "How far along some work is, or that it is under way.",
            include_str!("progress_indicator.rs"),
            progress_indicator::example,
        ),
        Demo::new(
            "skeleton",
            "Skeleton",
            "A pulsing placeholder in the shape of content that is still loading.",
            include_str!("skeleton.rs"),
            skeleton::example,
        ),
        Demo::new(
            "tooltip",
            "Tooltip",
            "A short label that appears while the pointer rests on a widget.",
            include_str!("tooltip.rs"),
            tooltip::example,
        ),
    ]
}
