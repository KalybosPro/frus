//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

pub mod backdrop_filter;
pub mod clip;
pub mod color_filtered;
pub mod custom_paint;
pub mod decorated_box;
pub mod elevation;
pub mod gradients;
pub mod image_filtered;
pub mod opacity;
pub mod shader_mask;
pub mod transform;

pub fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "decorated-box",
            "DecoratedBox & ColoredBox",
            "Paints a fill, rounded corners, borders and shadows behind or in front of a child.",
            include_str!("decorated_box.rs"),
            decorated_box::example,
        ),
        Demo::new(
            "gradient",
            "Gradients",
            "Linear, radial and sweep gradients, painted by a BoxDecoration.",
            include_str!("gradients.rs"),
            gradients::example,
        ),
        Demo::new(
            "elevation",
            "Elevation & shadows",
            "The shadows a surface casts at a given height, and shadow blur styles.",
            include_str!("elevation.rs"),
            elevation::example,
        ),
        Demo::new(
            "clip",
            "ClipRect, ClipRRect, ClipOval & ClipPath",
            "Cuts a child's painting to a rectangle, rounded corners, an oval or any path.",
            include_str!("clip.rs"),
            clip::example,
        ),
        Demo::new(
            "transform",
            "Transform & FractionalTranslation",
            "Moves, scales or rotates a child's painting without changing the layout.",
            include_str!("transform.rs"),
            transform::example,
        ),
        Demo::new(
            "opacity",
            "Opacity",
            "Paints a child partly transparent, as one layer.",
            include_str!("opacity.rs"),
            opacity::example,
        ),
        Demo::new(
            "custom-paint",
            "CustomPaint",
            "A canvas you draw yourself, with paths, rectangles and the theme's colours.",
            include_str!("custom_paint.rs"),
            custom_paint::example,
        ),
        Demo::new(
            "shader-mask",
            "ShaderMask",
            "Lays a gradient over a child's pixels: gradient text, fades, spotlights.",
            include_str!("shader_mask.rs"),
            shader_mask::example,
        ),
        Demo::new(
            "color-filtered",
            "ColorFiltered",
            "Changes the colours of everything a child paints.",
            include_str!("color_filtered.rs"),
            color_filtered::example,
        ),
        Demo::new(
            "image-filtered",
            "ImageFiltered",
            "Blurs a child, or grows and shrinks its shapes.",
            include_str!("image_filtered.rs"),
            image_filtered::example,
        ),
        Demo::new(
            "backdrop-filter",
            "BackdropFilter",
            "Blurs what is painted underneath: frosted glass.",
            include_str!("backdrop_filter.rs"),
            backdrop_filter::example,
        ),
    ]
}
