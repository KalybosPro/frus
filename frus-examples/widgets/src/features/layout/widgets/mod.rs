//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

pub mod align;
pub mod aspect_ratio;
pub mod baseline;
pub mod constrained_box;
pub mod container;
pub mod custom_multi_child_layout;
pub mod expanded;
pub mod fitted_box;
pub mod flex;
pub mod fractionally_sized_box;
pub mod indexed_stack;
pub mod layout_builder;
pub mod overflow_bar;
pub mod overflow_box;
pub mod padding;
pub mod placeholder;
pub mod rotated_box;
pub mod row_column;
pub mod safe_area;
pub mod sized_box;
pub mod stack;
pub mod two_pane;
pub mod wrap;

pub fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "container",
            "Container",
            "One box that sizes, pads, colours, rounds, borders and shadows its child.",
            include_str!("container.rs"),
            container::example,
        ),
        Demo::new(
            "padding",
            "Padding",
            "Empty space around a child, on every side or each side on its own.",
            include_str!("padding.rs"),
            padding::example,
        ),
        Demo::new(
            "center",
            "Center & Aligned",
            "Places one child somewhere inside the room it is given.",
            include_str!("align.rs"),
            align::example,
        ),
        Demo::new(
            "row",
            "Row & Column",
            "Children in a line, across or down, with alignment on both axes and spacing.",
            include_str!("row_column.rs"),
            row_column::example,
        ),
        Demo::new(
            "flex",
            "Flex",
            "The flexbox line under Row and Column: shrink-wraps, stretches, wraps, reverses.",
            include_str!("flex.rs"),
            flex::example,
        ),
        Demo::new(
            "expanded",
            "Expanded, Flexible & Spacer",
            "Share the free room of a row or a column between some of its children.",
            include_str!("expanded.rs"),
            expanded::example,
        ),
        Demo::new(
            "sized-box",
            "SizedBox",
            "A box of a given size: to size its child, or, empty, as a fixed gap.",
            include_str!("sized_box.rs"),
            sized_box::example,
        ),
        Demo::new(
            "constrained-box",
            "ConstrainedBox, LimitedBox & Intrinsic",
            "Floors and ceilings on a child's size, or the size its content would like.",
            include_str!("constrained_box.rs"),
            constrained_box::example,
        ),
        Demo::new(
            "overflow-box",
            "UnconstrainedBox & OverflowBox",
            "Let a child be a size its parent did not offer.",
            include_str!("overflow_box.rs"),
            overflow_box::example,
        ),
        Demo::new(
            "aspect-ratio",
            "AspectRatio",
            "A box whose height follows from its width.",
            include_str!("aspect_ratio.rs"),
            aspect_ratio::example,
        ),
        Demo::new(
            "fitted-box",
            "FittedBox",
            "Scales its child to fit its box, following a BoxFit rule.",
            include_str!("fitted_box.rs"),
            fitted_box::example,
        ),
        Demo::new(
            "fractionally-sized-box",
            "FractionallySizedBox",
            "A box sized as a fraction of the room its parent offers.",
            include_str!("fractionally_sized_box.rs"),
            fractionally_sized_box::example,
        ),
        Demo::new(
            "wrap",
            "Wrap & ListBody",
            "A row that flows onto new lines when it runs out of room.",
            include_str!("wrap.rs"),
            wrap::example,
        ),
        Demo::new(
            "stack",
            "Stack & Positioned",
            "Children layered in the same box, pinned to its edges or aligned in it.",
            include_str!("stack.rs"),
            stack::example,
        ),
        Demo::new(
            "indexed-stack",
            "IndexedStack",
            "Several children in one box, only one of them shown.",
            include_str!("indexed_stack.rs"),
            indexed_stack::example,
        ),
        Demo::new(
            "baseline",
            "Baseline",
            "Lines text up on the line its letters sit on.",
            include_str!("baseline.rs"),
            baseline::example,
        ),
        Demo::new(
            "rotated-box",
            "RotatedBox",
            "Turns its child by quarter turns, and the layout turns with it.",
            include_str!("rotated_box.rs"),
            rotated_box::example,
        ),
        Demo::new(
            "safe-area",
            "SafeArea",
            "Keeps its child clear of status bars, notches, the gesture handle and the keyboard.",
            include_str!("safe_area.rs"),
            safe_area::example,
        ),
        Demo::new(
            "layout-builder",
            "LayoutBuilder & OrientationBuilder",
            "Builds from the room actually given, so a component adapts wherever it is placed.",
            include_str!("layout_builder.rs"),
            layout_builder::example,
        ),
        Demo::new(
            "custom-multi-child-layout",
            "CustomMultiChildLayout",
            "Children measured and placed by a function you write.",
            include_str!("custom_multi_child_layout.rs"),
            custom_multi_child_layout::example,
        ),
        Demo::new(
            "overflow-bar",
            "OverflowBar",
            "A row of buttons that becomes a column when they stop fitting.",
            include_str!("overflow_bar.rs"),
            overflow_bar::example,
        ),
        Demo::new(
            "two-pane",
            "TwoPane & responsive",
            "A list and its detail side by side when wide, one at a time when narrow.",
            include_str!("two_pane.rs"),
            two_pane::example,
        ),
        Demo::new(
            "placeholder",
            "Placeholder",
            "A crossed box standing in for a widget not written yet.",
            include_str!("placeholder.rs"),
            placeholder::example,
        ),
    ]
}
