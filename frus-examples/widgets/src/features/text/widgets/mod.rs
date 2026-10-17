//! The demos of this category, in the order the gallery lists them.

use crate::models::demo::Demo;

pub mod font_family;
pub mod kbd;
pub mod rich_text;
pub mod selectable_text;
pub mod text;
pub mod text_style;
pub mod text_theme;

pub fn demos() -> Vec<Demo> {
    vec![
        Demo::new(
            "text",
            "Text",
            "A run of text: size, weight, colour, decoration, alignment, wrapping, line limits.",
            include_str!("text.rs"),
            text::example,
        ),
        Demo::new(
            "text-style",
            "TextStyle & DefaultTextStyle",
            "A style as a value: shared, merged, or given to a whole subtree.",
            include_str!("text_style.rs"),
            text_style::example,
        ),
        Demo::new(
            "rich-text",
            "RichText & TextSpan",
            "One paragraph in several styles, from a tree of spans.",
            include_str!("rich_text.rs"),
            rich_text::example,
        ),
        Demo::new(
            "text-theme",
            "Text theme",
            "The app's type scale, from display_large to label_small, and headings.",
            include_str!("text_theme.rs"),
            text_theme::example,
        ),
        Demo::new(
            "font-family",
            "FontFamily",
            "Sans-serif, serif, monospace, or a font registered by name.",
            include_str!("font_family.rs"),
            font_family::example,
        ),
        Demo::new(
            "selection-area",
            "Selectable text & SelectionArea",
            "Text the reader can select and copy, alone or across several texts.",
            include_str!("selectable_text.rs"),
            selectable_text::example,
        ),
        Demo::new(
            "kbd",
            "Kbd",
            "A keyboard key cap, for showing shortcuts.",
            include_str!("kbd.rs"),
            kbd::example,
        ),
    ]
}
