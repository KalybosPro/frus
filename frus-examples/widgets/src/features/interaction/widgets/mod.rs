//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

mod disabled;
mod drag_and_drop;
mod focus;
mod gesture_detector;
mod ink_well;
mod interactive_viewer;
mod keyboard;
mod listener;
mod mouse_region;
mod pointer_barriers;
mod semantics;
mod shortcuts;
mod visibility;

pub(crate) fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "gesture-detector",
            "GestureDetector",
            "Taps, double taps, long presses, right clicks and drags on any widget.",
            include_str!("gesture_detector.rs"),
            gesture_detector::example,
        ),
        Demo::new(
            "ink-well",
            "InkWell",
            "Makes any content pressable, with an ink ripple, hover and focus feedback.",
            include_str!("ink_well.rs"),
            ink_well::example,
        ),
        Demo::new(
            "mouse-region",
            "MouseRegion",
            "The mouse entering, moving over and leaving a widget, and the cursor there.",
            include_str!("mouse_region.rs"),
            mouse_region::example,
        ),
        Demo::new(
            "listener",
            "Listener",
            "The pointer's raw down, move, up and hover events.",
            include_str!("listener.rs"),
            listener::example,
        ),
        Demo::new(
            "draggable",
            "Draggable & DragTarget",
            "Pick a thing up and drop it on something else, with a payload.",
            include_str!("drag_and_drop.rs"),
            drag_and_drop::example,
        ),
        Demo::new(
            "interactive-viewer",
            "InteractiveViewer",
            "A viewport whose content can be panned and zoomed.",
            include_str!("interactive_viewer.rs"),
            interactive_viewer::example,
        ),
        Demo::new(
            "focus",
            "Focus & traversal",
            "Which widgets the keyboard reaches, in what order, and focusing from code.",
            include_str!("focus.rs"),
            focus::example,
        ),
        Demo::new(
            "shortcuts",
            "Shortcuts & Actions",
            "Keyboard shortcuts: a keystroke means an intent, an intent runs a handler.",
            include_str!("shortcuts.rs"),
            shortcuts::example,
        ),
        Demo::new(
            "focusable-action-detector",
            "FocusableActionDetector & KeyboardListener",
            "A custom control that answers the keyboard for itself.",
            include_str!("keyboard.rs"),
            keyboard::example,
        ),
        Demo::new(
            "ignore-pointer",
            "IgnorePointer & AbsorbPointer",
            "Switch off the pointer for a subtree: let taps through, or stop them.",
            include_str!("pointer_barriers.rs"),
            pointer_barriers::example,
        ),
        Demo::new(
            "visibility",
            "Visibility & Offstage",
            "Hide a child, keeping its box, its input, or nothing at all.",
            include_str!("visibility.rs"),
            visibility::example,
        ),
        Demo::new(
            "disabled",
            "Disabled state",
            "One enabled flag on every control: greyed out, inert, value kept.",
            include_str!("disabled.rs"),
            disabled::example,
        ),
        Demo::new(
            "semantics",
            "Semantics",
            "What a screen reader is told about a widget, said from outside it.",
            include_str!("semantics.rs"),
            semantics::example,
        ),
    ]
}
