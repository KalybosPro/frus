//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

pub mod animated_align;
pub mod animated_container;
pub mod animated_cross_fade;
pub mod animated_fractionally_sized_box;
pub mod animated_icon;
pub mod animated_opacity;
pub mod animated_padding;
pub mod animated_positioned;
pub mod animated_size;
pub mod animated_switcher;
pub mod animated_text_style;
pub mod animated_transform;
pub mod curves;
pub mod decorated_transitions;
pub mod hero;
pub mod transitions;

pub fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "animated-container",
            "AnimatedContainer",
            "A box whose colour, size, corners and opacity glide to each new value.",
            include_str!("animated_container.rs"),
            animated_container::example,
        ),
        Demo::new(
            "animated-opacity",
            "Opacity & AnimatedOpacity",
            "See-through by a fixed amount, or fading to each new amount.",
            include_str!("animated_opacity.rs"),
            animated_opacity::example,
        ),
        Demo::new(
            "animated-align",
            "AnimatedAlign",
            "Slides a child to a new anchor inside its box.",
            include_str!("animated_align.rs"),
            animated_align::example,
        ),
        Demo::new(
            "animated-padding",
            "AnimatedPadding",
            "Insets a child by an amount that moves to each new value.",
            include_str!("animated_padding.rs"),
            animated_padding::example,
        ),
        Demo::new(
            "animated-positioned",
            "AnimatedPositioned",
            "A stack layer whose pins move: panels that slide in, cards that travel.",
            include_str!("animated_positioned.rs"),
            animated_positioned::example,
        ),
        Demo::new(
            "animated-scale",
            "AnimatedScale, AnimatedRotation & AnimatedSlide",
            "Grow, turn or shift a child to each new value without moving its neighbours.",
            include_str!("animated_transform.rs"),
            animated_transform::example,
        ),
        Demo::new(
            "animated-default-text-style",
            "AnimatedDefaultTextStyle",
            "The text style handed down to a subtree, moving to each new style.",
            include_str!("animated_text_style.rs"),
            animated_text_style::example,
        ),
        Demo::new(
            "animated-fractionally-sized-box",
            "AnimatedFractionallySizedBox",
            "A fraction of the room, moving to each new fraction.",
            include_str!("animated_fractionally_sized_box.rs"),
            animated_fractionally_sized_box::example,
        ),
        Demo::new(
            "animated-size",
            "AnimatedSize",
            "A box that follows its child's size over time instead of jumping.",
            include_str!("animated_size.rs"),
            animated_size::example,
        ),
        Demo::new(
            "animated-switcher",
            "AnimatedSwitcher",
            "The old child leaves as the new one arrives when the value changes.",
            include_str!("animated_switcher.rs"),
            animated_switcher::example,
        ),
        Demo::new(
            "animated-cross-fade",
            "AnimatedCrossFade",
            "Cross-fades between two known children, the box moving between their sizes.",
            include_str!("animated_cross_fade.rs"),
            animated_cross_fade::example,
        ),
        Demo::new(
            "transitions",
            "Fade, Slide, Scale & Size transitions",
            "Explicit transitions driven by a number the app owns: an AnimationController.",
            include_str!("transitions.rs"),
            transitions::example,
        ),
        Demo::new(
            "decorated-box-transition",
            "DecoratedBoxTransition & DefaultTextStyleTransition",
            "A whole decoration or text style, part of the way from one to another.",
            include_str!("decorated_transitions.rs"),
            decorated_transitions::example,
        ),
        Demo::new(
            "curves",
            "Curve",
            "The shape of a motion, separate from its duration.",
            include_str!("curves.rs"),
            curves::example,
        ),
        Demo::new(
            "animated-icon",
            "Icon::animated & AnimatedIcons",
            "An icon that morphs between two marks: menu and close, play and pause.",
            include_str!("animated_icon.rs"),
            animated_icon::example,
        ),
        Demo::new(
            "hero",
            "Hero & page transitions",
            "A shared element that flies between two pages during a transition.",
            include_str!("hero.rs"),
            hero::example,
        ),
    ]
}
